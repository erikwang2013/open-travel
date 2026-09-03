//! open-travel 业务服务共享代码：JWT 密钥解析、Redis 分布式限流、API 版本校验、axum Error 归一。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use ecat_data::Cache;
use ecat_data_redis::RedisCache;
use ecat_data_sqlx::SqlxClient;
use ecat_mq::MessageQueue;
use ecat_mq_kafka::KafkaMq;
use serde_json::json;
use std::convert::Infallible;
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEV_JWT_SECRET: &str = "dev-only-change-me-32-bytes-minimum-secret";

/// 连接 MySQL 主库（DATABASE_URL，写路径）。失败返回 None 并告警，不阻塞服务启动。
pub async fn connect_primary() -> Option<Arc<SqlxClient>> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://travel:pass@localhost:3306/travel".into());
    match SqlxClient::connect(&url).await {
        Ok(db) => {
            tracing::info!("mysql connected");
            Some(Arc::new(db))
        }
        Err(e) => {
            tracing::warn!("mysql connect failed, continuing without db: {e}");
            None
        }
    }
}

/// 连接 MySQL 从库（REPLICA_DATABASE_URL，只读路径）。失败返回 None 并告警，
/// 不阻塞服务启动；调用方对只读失败回退主库。
/// 首次 compose 启动时从库需 dump 主库数据，短重试避免启动竞态。
pub async fn connect_replica() -> Option<Arc<SqlxClient>> {
    let url = std::env::var("REPLICA_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://travel:pass@localhost:3306/travel".into());
    for attempt in 1..=15 {
        match SqlxClient::connect(&url).await {
            Ok(db) => {
                tracing::info!("mysql replica connected");
                return Some(Arc::new(db));
            }
            Err(e) => {
                tracing::warn!(
                    "mysql replica connect attempt {attempt}/15 failed: {e}"
                );
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
    None
}

/// 连接 Kafka（KAFKA_BROKERS，默认 localhost:9092）。rdkafka 生产者为惰性连接，
/// 此处失败仅告警不阻塞服务启动（审计为旁路，不阻断业务）。
pub async fn connect_kafka() -> Option<Arc<KafkaMq>> {
    let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".into());
    match KafkaMq::connect(&brokers).await {
        Ok(mq) => {
            tracing::info!("kafka connected: {brokers}");
            Some(Arc::new(mq))
        }
        Err(e) => {
            tracing::warn!("kafka connect failed, continuing without audit: {e}");
            None
        }
    }
}

/// 发布审计事件到 travel-audit（fail-open：Kafka 不可用时仅告警，不阻断业务）。
pub async fn publish_audit(mq: &KafkaMq, event: &str, actor_user_id: u64, extra: serde_json::Value) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let payload = json!({
        "event": event,
        "actor_user_id": actor_user_id,
        "ts": ts,
        // ponytail: 未接入 Request/ConnectInfo，IP 暂记 "-"，接入后补齐
        "ip": "-",
        "extra": extra,
    });
    match serde_json::to_vec(&payload) {
        Ok(bytes) => {
            if let Err(e) = mq.publish("travel-audit", &bytes).await {
                tracing::warn!(error = %e, "audit publish failed");
            }
        }
        Err(e) => tracing::warn!(error = %e, "audit serialize failed"),
    }
}

/// JWT 密钥：优先读 JWT_SECRET 环境变量；未配置或长度不足时退回开发占位密钥并告警。
/// 生产部署必须通过环境变量/配置中心下发，切勿使用占位密钥。
pub fn jwt_secret() -> String {
    match std::env::var("JWT_SECRET") {
        Ok(secret) if secret.len() >= 32 => secret,
        Ok(_) => {
            tracing::warn!("JWT_SECRET 长度不足 32 字节，退回开发占位密钥");
            DEV_JWT_SECRET.to_string()
        }
        Err(_) => {
            tracing::warn!("JWT_SECRET 未设置，使用开发占位密钥（生产必须配置）");
            DEV_JWT_SECRET.to_string()
        }
    }
}

/// 雪花 ID 生成器初始化：worker id 取 ECAT_WORKER_ID（缺省 0）。
/// 单机多服务每个进程须配不同 id（config/docker-compose.yml）。idgen_rs 全局单例且
/// init 幂等（首调生效），重复调用无害；未 init 时 next_id() panic，各 run() 必须首行调用。
pub fn init_id_gen() {
    let worker_id = std::env::var("ECAT_WORKER_ID")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(0);
    assert!(
        worker_id < 64,
        "ECAT_WORKER_ID 超出 6-bit worker 域（0..63），当前 {worker_id}；超界会静默别名导致跨进程 PK 碰撞"
    );
    idgen_rs::id_helper::init_with_capacity(worker_id, 64, 10_000);
}

/// e-cat 中间件的 Error 非 Infallible，无法满足 axum Router::layer 的 Into<Infallible>
/// 约束；map_err 到不可构造的 NoError（From 实现 unreachable，实际永不执行）。
#[derive(Debug)]
pub struct NoError;

impl fmt::Display for NoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "infallible")
    }
}

impl std::error::Error for NoError {}

impl From<NoError> for Infallible {
    fn from(_: NoError) -> Infallible {
        unreachable!()
    }
}

pub fn no_error<E>(_: E) -> NoError {
    NoError
}

/// Redis 固定窗口分布式限流（替换 Phase 1 的进程内限流，多实例共享计数）。
///
/// key = `rl:{service}:{window_start}`，窗口内第一个请求建 key 并设 TTL，
/// 后续请求 INCR；计数超过 max 返回 429。
/// fail-open：Redis 不可用时放行并告警，避免限流组件拖垮业务。
/// ponytail: 全局窗口按服务维度计数（未区分客户端 IP），需要按 IP 限流时
/// 在 key 中拼接入 IP 即可。
#[derive(Clone)]
pub struct RedisRateLimitLayer {
    cache: Option<Arc<RedisCache>>,
    service: &'static str,
    max: u64,
    window_secs: u64,
}

impl RedisRateLimitLayer {
    pub fn new(
        cache: Option<Arc<RedisCache>>,
        service: &'static str,
        max: u64,
        window_secs: u64,
    ) -> Self {
        Self { cache, service, max, window_secs }
    }
}

impl<S> tower::Layer<S> for RedisRateLimitLayer {
    type Service = RedisRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RedisRateLimitService {
            inner,
            cache: self.cache.clone(),
            service: self.service,
            max: self.max,
            window_secs: self.window_secs,
        }
    }
}

#[derive(Clone)]
pub struct RedisRateLimitService<S> {
    inner: S,
    cache: Option<Arc<RedisCache>>,
    service: &'static str,
    max: u64,
    window_secs: u64,
}

impl<S> tower::Service<Request<Body>> for RedisRateLimitService<S>
where
    S: tower::Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let fut = self.inner.call(req);
        let Some(cache) = self.cache.clone() else {
            return Box::pin(async move { fut.await });
        };
        let key = rate_key(self.service, self.window_secs);
        let max = self.max;
        let ttl = self.window_secs + 1;
        Box::pin(async move {
            match cache.increment(&key, 1).await {
                Ok(n) if n > max as i64 => {
                    return Ok(Response::builder()
                        .status(StatusCode::TOO_MANY_REQUESTS)
                        .body(Body::from("too many requests"))
                        .expect("valid response"));
                }
                Ok(1) => {
                    // 窗口内第一个请求：建 key 并设置 TTL（并发下重复 set 幂等无害）
                    let _ = cache.set(&key, b"1", Duration::from_secs(ttl)).await;
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("rate limit check failed, allowing request: {e}");
                }
            }
            fut.await
        })
    }
}

fn rate_key(service: &str, window_secs: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let window_start = now / window_secs * window_secs;
    format!("rl:{service}:{window_start}")
}

/// API 版本校验层：版本经 `X-Api-Version` header 传递（URL 不含版本前缀）。
/// 强制要求：缺失或非支持版本直接 400，不进入业务处理。
/// 手写 tower::Layer（error 透传），因为 axum from_fn 要求 inner error
/// 精确为 Infallible，与本项目 map_err(no_error) 归一后的 NoError 不兼容。
#[derive(Clone)]
pub struct ApiVersionLayer;

impl<S> tower::Layer<S> for ApiVersionLayer {
    type Service = ApiVersionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ApiVersionService { inner }
    }
}

#[derive(Clone)]
pub struct ApiVersionService<S> {
    inner: S,
}

impl<S> tower::Service<Request<Body>> for ApiVersionService<S>
where
    S: tower::Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let version = req
            .headers()
            .get("x-api-version")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if version != "v1" {
            return Box::pin(async move {
                Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"code":400,"message":"missing or unsupported api version (X-Api-Version: v1 required)","data":null}"#,
                    ))
                    .expect("valid response"))
            });
        }
        let fut = self.inner.call(req);
        Box::pin(async move { fut.await })
    }
}
