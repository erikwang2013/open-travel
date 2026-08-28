// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use super::claims::AuthClaims;
use super::helpers::{error_response, extract_bearer};
use http::{Request, Response, StatusCode};
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// HTTP timeout for token introspection requests.
const INTROSPECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// 内省响应体大小上限（1 MiB）：有界读取，防止恶意提供方无界响应耗尽内存。
const INTROSPECT_BODY_LIMIT: usize = 1024 * 1024;

/// 内省缓存容量上限：达到后按 FIFO 逐出最旧条目，防止海量唯一 token
/// 无限占用内存（S2 DoS）。
const CACHE_CAPACITY: usize = 10_000;

/// 默认缓存 claims 白名单：除结构化字段（sub/exp/iat/role，始终保留）外，
/// extra 仅以下标准非敏感字段落缓存；email/phone 等 PII 与自定义敏感字段
/// 默认不缓存（任务 #36）。
const DEFAULT_CLAIMS_WHITELIST: &[&str] = &["iss", "aud", "scope", "roles"];

/// 内省结果缓存：token -> (claims, 缓存时间)。TTL 内命中直接返回 claims
/// （避免每请求反序列化 JSON），过期后重新 introspection。
/// FIFO 有界：达到容量上限时逐出最旧条目；order 与 entries 一一对应，
/// 每个 key 只入队一次。过期条目由 purge_expired 在写入路径上时间淘汰，
/// 不残留内存。
struct IntrospectCache {
    entries: HashMap<String, (AuthClaims, std::time::Instant)>,
    order: VecDeque<String>,
}

impl IntrospectCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    /// TTL 时间淘汰：清除所有过期条目（调用方需持有写锁）。
    fn purge_expired(&mut self, ttl: std::time::Duration) {
        self.entries
            .retain(|_, (_, cached_at)| cached_at.elapsed() < ttl);
        self.order.retain(|k| self.entries.contains_key(k));
    }
}

/// 按白名单过滤缓存 claims：结构化字段（sub/exp/iat/role）始终保留；
/// extra 仅保留白名单内字段；白名单含 "*" 时保留全部 extra。
/// 仅作用于缓存值，introspection 直接返回的 claims 不受影响。
fn filter_claims(claims: AuthClaims, whitelist: &[String]) -> AuthClaims {
    if whitelist.iter().any(|k| k == "*") {
        return claims;
    }
    let extra = claims
        .extra
        .into_iter()
        .filter(|(k, _)| whitelist.iter().any(|w| w == k))
        .collect();
    AuthClaims { extra, ..claims }
}

/// 内省缓存 key：token 的 SHA-256 hex。缓存中不保存明文 token，
/// 内存转储/取证不泄露凭据（S2 增强）；命中语义不变。
fn cache_key(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Clone)]
pub struct OAuth2Layer {
    introspection_url: String,
    client_id: String,
    client_secret: String,
    cache_ttl_secs: u64,
    cache_capacity: usize,
    /// 缓存 claims 白名单：extra 中仅白名单字段落缓存（任务 #36）。
    claims_whitelist: Vec<String>,
    /// Shared HTTP client: connections are pooled and reused across requests
    /// instead of being created (and torn down) per request.
    client: reqwest::Client,
    cache: Arc<tokio::sync::RwLock<IntrospectCache>>,
}

impl OAuth2Layer {
    /// The introspection URL must use `https`; plain `http` is rejected
    /// (skipped in `cfg(test)` so unit tests may point at a local server).
    pub fn new(
        introspection_url: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Result<Self, String> {
        let introspection_url = introspection_url.into();
        #[cfg(not(test))]
        {
            if !introspection_url.starts_with("https://") {
                return Err(format!(
                    "introspection URL must use https, got: {introspection_url}"
                ));
            }
        }
        let client = reqwest::Client::builder()
            .timeout(INTROSPECT_TIMEOUT)
            .build()
            .map_err(|e| format!("failed to build http client: {e}"))?;
        Ok(Self {
            introspection_url,
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            cache_ttl_secs: 300,
            cache_capacity: CACHE_CAPACITY,
            claims_whitelist: DEFAULT_CLAIMS_WHITELIST
                .iter()
                .map(|s| s.to_string())
                .collect(),
            client,
            cache: Arc::new(tokio::sync::RwLock::new(IntrospectCache::new())),
        })
    }

    pub fn cache_ttl(mut self, secs: u64) -> Self {
        self.cache_ttl_secs = secs;
        self
    }

    pub fn cache_capacity(mut self, n: usize) -> Self {
        self.cache_capacity = n.max(1);
        self
    }

    /// 配置缓存 claims 白名单：extra 中仅列出的字段落缓存（默认
    /// iss/aud/scope/roles）。传入 "*" 表示缓存全部 extra 字段（逃生门）。
    /// 结构化字段 sub/exp/iat/role 不受白名单影响，始终保留。
    pub fn cache_claims_whitelist(
        mut self,
        keys: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.claims_whitelist = keys.into_iter().map(Into::into).collect();
        self
    }
}

impl<S> Layer<S> for OAuth2Layer {
    type Service = OAuth2Service<S>;

    fn layer(&self, inner: S) -> Self::Service {
        OAuth2Service {
            inner,
            config: Arc::new(self.clone()),
        }
    }
}

#[derive(Clone)]
pub struct OAuth2Service<S> {
    inner: S,
    config: Arc<OAuth2Layer>,
}

impl<S, B> Service<Request<B>> for OAuth2Service<S>
where
    S: Service<Request<B>, Response = Response<axum::body::Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: Send + 'static,
{
    type Response = Response<axum::body::Body>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(|e| Box::new(e) as _)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let token = extract_bearer(req.headers(), "Authorization");
        let config = Arc::clone(&self.config);
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let token = match token {
                Some(t) => t,
                None => {
                    return Ok(error_response(
                        StatusCode::UNAUTHORIZED,
                        r#"{"error":"missing bearer token"}"#,
                    ));
                }
            };

            match introspect_token(&config, &token).await {
                Ok(c) => {
                    let mut req = req;
                    req.extensions_mut().insert(c);
                    inner.call(req).await.map_err(|e| Box::new(e) as _)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "oauth2 introspection failed");
                    Ok(error_response(
                        StatusCode::UNAUTHORIZED,
                        r#"{"error":"invalid token"}"#,
                    ))
                }
            }
        })
    }
}

async fn introspect_token(config: &OAuth2Layer, token: &str) -> Result<AuthClaims, String> {
    let key = cache_key(token);
    // TTL 内命中缓存，避免每个请求都打 introspection 端点。
    if config.cache_ttl_secs > 0 {
        let cache = config.cache.read().await;
        if let Some((claims, cached_at)) = cache.entries.get(&key)
            && cached_at.elapsed() < std::time::Duration::from_secs(config.cache_ttl_secs)
        {
            return Ok(claims.clone());
        }
    }

    let params = [
        ("token", token),
        ("client_id", &config.client_id),
        ("client_secret", &config.client_secret),
    ];

    let mut resp = config
        .client
        .post(&config.introspection_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("introspection request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("introspection returned {}", resp.status()));
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("introspection read: {e}"))?
    {
        if bytes.len() + chunk.len() > INTROSPECT_BODY_LIMIT {
            return Err(format!(
                "introspection response exceeds {INTROSPECT_BODY_LIMIT} bytes"
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    let body: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("introspection parse: {e}"))?;

    let active = body
        .get("active")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !active {
        return Err("token is not active".into());
    }

    let sub = match body.get("sub").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return Err("introspection response missing sub".into()),
    };

    let role = body
        .get("role")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut extra = HashMap::new();
    if let Some(obj) = body.as_object() {
        for (k, v) in obj {
            if !matches!(
                k.as_str(),
                "active" | "sub" | "role" | "client_id" | "exp" | "iat"
            ) {
                extra.insert(k.clone(), v.clone());
            }
        }
    }

    let claims = AuthClaims {
        sub,
        exp: body.get("exp").and_then(|v| v.as_u64()),
        iat: body.get("iat").and_then(|v| v.as_u64()),
        role,
        extra,
    };

    if config.cache_ttl_secs > 0 {
        let mut cache = config.cache.write().await;
        // TTL 时间淘汰：仅在缓存接近容量上限时全量扫描（O(n)），避免每次
        // miss 写都扫全部条目；过期条目读取时被惰性跳过、命中写入时覆盖，
        // 内存仍受容量上限约束（P2）。
        if cache.entries.len() >= (config.cache_capacity * 9 / 10).max(1) {
            cache.purge_expired(std::time::Duration::from_secs(config.cache_ttl_secs));
        }
        // 新 key 且容量已满：FIFO 逐出最旧条目（order 与 entries 一一对应，
        // 每个 key 只入队一次，不产生重复条目）。
        if !cache.entries.contains_key(&key) {
            if cache.entries.len() >= config.cache_capacity
                && let Some(oldest) = cache.order.pop_front()
            {
                cache.entries.remove(&oldest);
            }
            cache.order.push_back(key.clone());
        }
        // 只缓存白名单 claims：敏感/非常规字段不落缓存（任务 #36）。
        cache.entries.insert(
            key,
            (
                filter_claims(claims.clone(), &config.claims_whitelist),
                std::time::Instant::now(),
            ),
        );
    }

    Ok(claims)
}

#[cfg(test)]
mod tests;
