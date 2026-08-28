// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use super::claims::AuthClaims;
use super::helpers::{error_response, extract_bearer};
use http::{Request, Response, StatusCode};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Errors while constructing or operating the JWT auth layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JwtAuthError {
    /// The shared secret is shorter than 32 bytes, which is too weak for HS256.
    WeakKey,
}

impl std::fmt::Display for JwtAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WeakKey => write!(f, "JWT secret must be at least 32 bytes for HS256"),
        }
    }
}

impl std::error::Error for JwtAuthError {}

#[derive(Clone)]
pub struct JwtAuthLayer {
    required_claims: Vec<String>,
    header_name: String,
    /// 配置后强制校验 JWT 的 iss（签发者）声明，不匹配即拒绝。
    required_issuer: Option<String>,
    /// 配置后强制校验 JWT 的 aud（受众）声明，不匹配即拒绝。
    required_audience: Option<String>,
    /// 构建一次复用：DecodingKey 持有密钥副本，逐请求重建是纯浪费（P1）。
    decoding_key: Arc<jsonwebtoken::DecodingKey>,
    /// 基准校验配置：每个请求 clone（Validation: Clone），避免重建。
    validation: jsonwebtoken::Validation,
}

impl JwtAuthLayer {
    /// Create a layer for HS256-signed tokens.
    ///
    /// The secret must be at least 32 bytes (the minimum key size HS256
    /// accepts per RFC 7518); shorter keys are rejected with
    /// [`JwtAuthError::WeakKey`].
    pub fn new(secret: impl Into<String>) -> Result<Self, JwtAuthError> {
        let secret = secret.into();
        if secret.len() < 32 {
            return Err(JwtAuthError::WeakKey);
        }
        let secret_bytes = secret.into_bytes();
        Ok(Self {
            decoding_key: Arc::new(jsonwebtoken::DecodingKey::from_secret(&secret_bytes)),
            validation: jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256),
            required_claims: vec!["sub".into()],
            header_name: "Authorization".into(),
            required_issuer: None,
            required_audience: None,
        })
    }

    pub fn require_claims(mut self, claims: &[&str]) -> Self {
        self.required_claims = claims.iter().map(|c| c.to_string()).collect();
        self
    }

    /// 强制校验 iss（签发者）声明：缺失或不匹配即拒绝。
    /// 默认不校验 iss，保持向后兼容。
    pub fn required_issuer(mut self, issuer: impl Into<String>) -> Self {
        let issuer = issuer.into();
        self.validation.set_issuer(&[issuer.as_str()]);
        self.required_issuer = Some(issuer);
        self
    }

    /// 强制校验 aud（受众）声明：缺失或不匹配即拒绝。
    /// 默认不校验 aud，保持向后兼容。
    pub fn required_audience(mut self, audience: impl Into<String>) -> Self {
        let audience = audience.into();
        self.validation.set_audience(&[audience.as_str()]);
        self.required_audience = Some(audience);
        self
    }

    pub fn header_name(mut self, name: impl Into<String>) -> Self {
        self.header_name = name.into();
        self
    }
}

impl<S> Layer<S> for JwtAuthLayer {
    type Service = JwtAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        JwtAuthService {
            inner,
            config: Arc::new(self.clone()),
        }
    }
}

#[derive(Clone)]
pub struct JwtAuthService<S> {
    inner: S,
    config: Arc<JwtAuthLayer>,
}

impl<S, B> Service<Request<B>> for JwtAuthService<S>
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
        let token = extract_bearer(req.headers(), &self.config.header_name);
        let config = Arc::clone(&self.config);
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let token = match token {
                Some(t) => t,
                None => {
                    return Ok(error_response(
                        StatusCode::UNAUTHORIZED,
                        r#"{"error":"missing authorization token"}"#,
                    ));
                }
            };

            let validation = config.validation.clone();
            let token_data = match jsonwebtoken::decode::<AuthClaims>(
                &token,
                config.decoding_key.as_ref(),
                &validation,
            ) {
                Ok(data) => data,
                Err(e) => {
                    // Distinguish expiry in the logs without leaking
                    // jsonwebtoken internals to clients.
                    let expired =
                        matches!(e.kind(), jsonwebtoken::errors::ErrorKind::ExpiredSignature);
                    tracing::warn!(
                        error = %e,
                        expired,
                        "jwt validation failed"
                    );
                    return Ok(error_response(
                        StatusCode::UNAUTHORIZED,
                        r#"{"error":"invalid token"}"#,
                    ));
                }
            };

            // jsonwebtoken 的 set_issuer/set_audience 只在声明存在时校验匹配，
            // 声明缺失会被静默跳过；required_issuer/audience 语义是"必须存在且匹配"，
            // 因此这里补存在性检查。
            if (config.required_issuer.is_some() && !token_data.claims.extra.contains_key("iss"))
                || (config.required_audience.is_some()
                    && !token_data.claims.extra.contains_key("aud"))
            {
                return Ok(error_response(
                    StatusCode::UNAUTHORIZED,
                    r#"{"error":"invalid token"}"#,
                ));
            }

            for claim in &config.required_claims {
                let satisfied = match claim.as_str() {
                    "sub" => !token_data.claims.sub.is_empty(),
                    "role" => token_data.claims.role.is_some(),
                    _ => token_data.claims.extra.contains_key(claim),
                };
                if !satisfied {
                    return Ok(error_response(
                        StatusCode::FORBIDDEN,
                        format!(r#"{{"error":"missing required claim: {claim}"}}"#),
                    ));
                }
            }

            let claims = token_data.claims;
            let mut req = req;
            req.extensions_mut().insert(claims);
            inner.call(req).await.map_err(|e| Box::new(e) as _)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    const SECRET: &str = "0123456789abcdef0123456789abcdef";
    const ISSUER: &str = "https://issuer.example";
    const AUDIENCE: &str = "api.example";

    fn make_token(sub: &str, iss: Option<&str>, aud: Option<&str>) -> String {
        let mut claims = serde_json::json!({ "sub": sub, "exp": 4_102_444_800u64 });
        if let Some(iss) = iss {
            claims["iss"] = serde_json::Value::String(iss.into());
        }
        if let Some(aud) = aud {
            claims["aud"] = serde_json::Value::String(aud.into());
        }
        jsonwebtoken::encode(
            &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap()
    }

    async fn call_layer(layer: JwtAuthLayer, token: &str) -> axum::http::StatusCode {
        // JwtAuthService 的错误类型是 Box<dyn Error>，axum Router（Infallible）
        // 无法直接 layer，此处用 Layer::layer 包一个 MethodRouter 做端到端测试。
        let svc = layer.layer(axum::routing::get(|| async { "ok" }));
        svc.oneshot(
            axum::http::Request::builder()
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
    }

    /// S4 向后兼容：未配置 required_issuer/audience 时，
    /// 不带 iss/aud 的 token 必须照常通过。
    #[tokio::test]
    async fn default_layer_accepts_token_without_iss_aud() {
        let layer = JwtAuthLayer::new(SECRET).unwrap();
        let token = make_token("user-1", None, None);
        assert_eq!(call_layer(layer, &token).await, StatusCode::OK);
    }

    /// S4：配置 required_issuer 后，iss 缺失或不匹配即拒绝。
    #[tokio::test]
    async fn required_issuer_rejects_missing_and_wrong() {
        let layer = JwtAuthLayer::new(SECRET).unwrap().required_issuer(ISSUER);

        let ok = make_token("user-1", Some(ISSUER), None);
        assert_eq!(call_layer(layer.clone(), &ok).await, StatusCode::OK);

        let wrong = make_token("user-1", Some("https://other.example"), None);
        assert_eq!(
            call_layer(layer.clone(), &wrong).await,
            StatusCode::UNAUTHORIZED
        );

        let missing = make_token("user-1", None, None);
        assert_eq!(call_layer(layer, &missing).await, StatusCode::UNAUTHORIZED);
    }

    /// S4：配置 required_audience 后，aud 缺失或不匹配即拒绝。
    #[tokio::test]
    async fn required_audience_rejects_missing_and_wrong() {
        let layer = JwtAuthLayer::new(SECRET)
            .unwrap()
            .required_audience(AUDIENCE);

        let ok = make_token("user-1", None, Some(AUDIENCE));
        assert_eq!(call_layer(layer.clone(), &ok).await, StatusCode::OK);

        let wrong = make_token("user-1", None, Some("other.example"));
        assert_eq!(
            call_layer(layer.clone(), &wrong).await,
            StatusCode::UNAUTHORIZED
        );

        let missing = make_token("user-1", None, None);
        assert_eq!(call_layer(layer, &missing).await, StatusCode::UNAUTHORIZED);
    }

    fn make_token_with_exp(exp: u64) -> String {
        jsonwebtoken::encode(
            &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
            &serde_json::json!({ "sub": "user-1", "exp": exp }),
            &jsonwebtoken::EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn expired_token_is_rejected() {
        let layer = JwtAuthLayer::new(SECRET).unwrap();
        let token = make_token_with_exp(1_000_000_000u64); // 2001 年已过期
        assert_eq!(call_layer(layer, &token).await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn token_with_wrong_signature_is_rejected() {
        let layer = JwtAuthLayer::new(SECRET).unwrap();
        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
            &serde_json::json!({ "sub": "user-1", "exp": 4_102_444_800u64 }),
            &jsonwebtoken::EncodingKey::from_secret(b"another-secret-0123456789abcdef"),
        )
        .unwrap();
        assert_eq!(call_layer(layer, &token).await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn missing_required_role_claim_is_forbidden() {
        let layer = JwtAuthLayer::new(SECRET)
            .unwrap()
            .require_claims(&["sub", "role"]);
        let token = make_token("user-1", None, None); // 无 role 声明
        assert_eq!(call_layer(layer, &token).await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn custom_header_name_is_used() {
        let layer = JwtAuthLayer::new(SECRET)
            .unwrap()
            .header_name("X-Auth-Token");
        let token = make_token("user-1", None, None);
        let svc = layer.layer(axum::routing::get(|| async { "ok" }));
        let resp = svc
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .header("X-Auth-Token", format!("Bearer {token}"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // 默认 Authorization 头不带 token → 401
        let resp = svc
            .oneshot(
                axum::http::Request::builder()
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn missing_bearer_token_is_unauthorized() {
        let layer = JwtAuthLayer::new(SECRET).unwrap();
        let svc = layer.layer(axum::routing::get(|| async { "ok" }));
        let resp = svc
            .oneshot(
                axum::http::Request::builder()
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
