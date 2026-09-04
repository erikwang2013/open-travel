//! open-travel 业务服务共享代码：JWT 密钥解析、Redis 分布式限流、雪花 ID 初始化、axum Error 归一。

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
use std::sync::{Arc, LazyLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEV_JWT_SECRET: &str = "dev-only-change-me-32-bytes-minimum-secret";

/// 雪花 ID 6-bit worker 域上限（0..63）。`idgen_rs` 不校验 worker id 是否超出位宽，
/// 超界会静默溢出进时间戳位、产生跨进程 PK 碰撞，故由本层强制断言。
const WORKER_ID_MAX: u64 = 64;
/// worker 领号计数器 key。刻意不与既有业务 key（`rl:*`、`hotwords:*`）共用命名空间；
/// 计数器值必须是整数，被其他组件写入非整数会让 `INCR` 失败，`claim_worker_id` 有
/// 自清理兜底。
/// worker id 会随重启漂移（user 首次领 0、重启后领 9），**不代表服务身份**：已发 id
/// 嵌的是旧 worker id、新 id 嵌新 id，位段不同不冲突（`init_with_capacity` 幂等）。
/// 64 个号位是**一次性预算**，不回收——反复重启/崩溃会耗尽，耗尽后按位宽断言 fail-fast。
const WORKER_CLAIM_KEY: &str = "ex:idgen:worker-idx";

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

/// 雪花 ID 唯一入口：首次取号时在异步运行时内领 worker id 并初始化生成器，随后取号。
/// 取代散落在各 handler 的 `idgen_rs::id_helper::next_id()`——后者在生成器未初始化时
/// panic 且堆栈离根因很远；此函数把初始化与取号收敛到一处，漏初始化当场炸在入口。
///
/// 取号路径必须异步：领号是一次 Redis `INCR`，同步入口只能 `block_on`，而
/// `block_on` 不能在正在驱动任务的线程上执行（单线程 tokio 运行时、`#[tokio::test]`
/// 直接 panic）。所有调用点本就在请求/测试的异步上下文中，故直接 `.await`。
pub async fn snowflake_id() -> u64 {
    if !idgen_rs::id_helper::is_initialized() {
        // tokio::sync::OnceCell 而非 std OnceLock：后者在闭包 panic 时永久置为已
        // 初始化，后续取号静默拿 None；此处失败即进程中止，不留脏状态。
        static INIT: LazyLock<tokio::sync::OnceCell<()>> =
            LazyLock::new(|| tokio::sync::OnceCell::new());
        INIT.get_or_init(
            || async {
                let _ = claim_worker_id().await.expect(
                    "worker id 领号失败（不可降级，详见 claim_worker_id）"
                );
            },
        )
        .await;
    }
    // try_next_id 而非 next_id：后者失败时消息是 idgen_rs 自带的
    // "ID generator not initialized."，不带本项目的诊断上下文。
    idgen_rs::id_helper::try_next_id().unwrap_or_else(|msg| {
        panic!(
            "雪花 ID 生成器未初始化: {msg}。snowflake_id() 首次调用会经 Redis 领号，\
             反复失败说明领号通道不通（见 connect_worker_claim_redis）"
        )
    })
}

/// 从 Redis 原子领一个全局唯一的雪花 worker id 并初始化生成器。
/// `INCR` 而非 `GET`+`SET`——后者两个进程会同时读到 None 然后都写成功，等于没修。
/// 失败即 panic 拒绝启动：worker id 唯一性只能由 Redis 原子操作保证，降级到 0 会让
/// 多进程静默 PK 碰撞（不可逆数据损坏）；「雪花不可用导致服务起不来」可观测可恢复。
/// 与限流层的 fail-open 语义相反，故不复用 `connect_cache()`。
pub async fn claim_worker_id() -> Result<u16, String> {
    // pub 仅为集成测试：无法重置 idgen_rs 全局单例，fail-closed 行为直测本函数。
    let cache = connect_worker_claim_redis().await?;
    // 先清污染值：key 上若残留非整数（其他组件复用同名 key），`INCR` 报
    // "value is not an integer" 而失败——那会让 64 个号位全部领不到号。
    // 计数器是纯单调计数器，清掉后重新计数只影响 worker id 分配顺序，不影响唯一性
    // （唯一性来自 INCR 原子性 + 6-bit 位宽断言），故失败即清理后继续。
    let idx = match cache.increment(WORKER_CLAIM_KEY, 1).await {
        Ok(n) => n,
        Err(e) => {
            if let Err(c) = cache.delete(WORKER_CLAIM_KEY).await {
                return Err(format!("worker id 计数器非整数，清理失败（{e}；清理错误 {c}）"));
            }
            cache.increment(WORKER_CLAIM_KEY, 1)
                .await
                .map_err(|e| e.to_string())?
        }
    };
    assert!(
        idx < WORKER_ID_MAX as i64,
        "可用 worker id 已耗尽（领到 {idx}，上限 {WORKER_ID_MAX}）；扩容需改大 max_nodes，\
         禁止 % 取模——回绕后与已领号进程同毫秒序列碰撞"
    );
    tracing::info!(worker_id = idx, "雪花 worker id 已领");
    idgen_rs::id_helper::init_with_capacity(idx as u16, WORKER_ID_MAX as u32, 10_000);
    Ok(idx as u16)
}

/// 领号专用 Redis 连接。不重试——失败发生在首次取号（即第一次 INSERT 时），进程
/// panic 可观测，重试无意义；与限流的 fail-open 连接互不影响。
/// pub 仅为集成测试断言 fail-closed：测试无法重置 idgen_rs 全局单例，故直测本函数
/// 的 Err 路径而非 `#[should_panic]`。
pub async fn connect_worker_claim_redis() -> Result<Arc<RedisCache>, String> {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into());
    RedisCache::connect(&url)
        .await
        .map(Arc::new)
        .map_err(|e| e.to_string())
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
