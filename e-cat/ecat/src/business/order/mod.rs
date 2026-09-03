// open-travel order-service：下单 / 订单列表 / 详情 / 取消（P3-08/P3-09）
//
// 端口 8006（网关 /api/v1/orders/ → ecat-order:8006）。
// 状态机：0待支付 → 1已支付 → 2已确认 → 3已完成（P4-07 支付闭环回调推进）；
//         0 → 4（超时 / 取消）。本期仅实现 order_type=1（线路）。
//
// 防超卖核心：Redis 预占（travel:stock:{line_date_id}，DECR 原子扣减，负值回滚
// INCR 补回）→ DB 原子扣减（UPDATE ... WHERE seats_left >= ?，受影响行数 0
// 回滚 Redis）→ 插入订单。两条防线都保证不超卖，Redis 是快路径，DB 是兜底。
//
// 事务：sqlx Any 的 Transaction 不提供查询执行方法（仅 commit/rollback），
// 故采用「先扣后插，插入失败回补」补偿顺序替代事务：
//   ponytail: 补偿非原子，崩溃窗口（扣减成功、订单未插入）会少卖一个位，
//   后台扫描修复不了（无订单可查），可接受；后续接入事务/出库对账再收紧。
//
// 超时释放：惰性（列表/详情前扫 expire_at < NOW() 且 status=0 → 置 4 并回补
// 余位与 Redis）+ 启动后台任务每 60s 扫一轮。
//
// 中间件链（外层 → 内层）：ApiVersion → CircuitBreaker → Security → RateLimit
// → [仅 /api/v1/orders/*] JWT。JWT 与 user-service 同一密钥，claims.sub 为 user_id。
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use ecat::App;
use ecat_auth::{AuthClaims, JwtAuthLayer};
use ecat_circuit_breaker::CircuitBreakerLayer;
use ecat_data_redis::RedisCache;
use ecat_data_sqlx::SqlxClient;
use ecat_middleware::LoggingLayer;
use ecat_mq_kafka::KafkaMq;
use ecat_security::SecurityLayer;
use ecat_tracing::TracingLayer;
use ecat_transport_http::HttpServer;
use serde::{Deserialize, Serialize};
use ecat::business::shared::{
    connect_kafka, connect_primary, init_id_gen, jwt_secret, no_error, publish_audit, RedisRateLimitLayer,
};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;

#[path = "handlers.rs"]
pub(crate) mod handlers;

const PORT: &str = "0.0.0.0:8006";
const STOCK_TTL_SECS: u64 = 86400;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: Option<Arc<SqlxClient>>,
    pub(crate) cache: Option<Arc<RedisCache>>,
    // Kafka 审计生产者（fail-open：不可用时仅告警，不阻断业务）
    pub(crate) mq: Option<Arc<KafkaMq>>,
    pub(crate) jwt: JwtAuthLayer,
}

#[derive(Serialize)]
pub(crate) struct ApiResponse<T: Serialize> {
    code: u32,
    message: String,
    data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Json<Self> {
        Json(Self { code: 0, message: "ok".into(), data: Some(data) })
    }
}

fn err<T: Serialize>(
    status: StatusCode,
    code: u32,
    message: &str,
) -> (StatusCode, Json<ApiResponse<T>>) {
    (status, Json(ApiResponse { code, message: message.into(), data: None }))
}

/// 登录态守卫（提取器形态，可与 Json/Query/Path 组合）：JwtAuthLayer 注入的
/// claims.sub 即 user_id；无 claims 401，sub 非法 400。普通用户即可下单，
/// 无需 role。
pub(crate) struct UserGuard(pub(crate) u64);

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for UserGuard {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let claims: &AuthClaims = parts.extensions.get::<AuthClaims>().ok_or_else(|| {
            err::<serde_json::Value>(StatusCode::UNAUTHORIZED, 401, "missing claims")
                .into_response()
        })?;
        let user_id = claims.subject().parse::<u64>().map_err(|_| {
            err::<serde_json::Value>(StatusCode::BAD_REQUEST, 400, "invalid subject in token")
                .into_response()
        })?;
        Ok(UserGuard(user_id))
    }
}

#[derive(Deserialize)]
pub(crate) struct CreateOrderReq {
    pub(crate) order_type: u8,
    pub(crate) product_id: u64,
    #[serde(default)]
    pub(crate) line_date_id: u64,
    pub(crate) quantity: u64,
    #[serde(default)]
    pub(crate) check_in: Option<String>,
    #[serde(default)]
    pub(crate) check_out: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct OrderOut {
    id: u64,
    order_type: u8,
    product_id: u64,
    status: u8,
    amount_cents: u64,
    snapshot: serde_json::Value,
    expire_at: Option<String>,
    created_at: String,
}

/// P4-07 支付确认请求（内部接口，仅 txn_no）。
#[derive(Deserialize)]
pub(crate) struct PaySuccessReq {
    pub(crate) txn_no: String,
}

#[derive(Deserialize)]
pub(crate) struct ListQuery {
    #[serde(default)]
    pub(crate) page: u64,
    #[serde(default)]
    pub(crate) page_size: u64,
}

async fn health() -> &'static str {
    "OK"
}

pub(crate) async fn ready(State(state): State<AppState>) -> Json<ApiResponse<bool>> {
    let ready = state.db.is_some() && state.cache.is_some();
    Json(ApiResponse { code: 0, message: "ready".into(), data: Some(ready) })
}

async fn connect_cache() -> Option<Arc<RedisCache>> {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into());
    match RedisCache::connect(&url).await {
        Ok(cache) => {
            tracing::info!("redis connected");
            Some(Arc::new(cache))
        }
        Err(e) => {
            tracing::warn!("redis connect failed, continuing without cache: {e}");
            None
        }
    }
}

/// 业务路由 + 中间件链；独立成函数便于集成测试直接构造。
pub(crate) fn api_router(state: AppState) -> Router {
    let orders = Router::new()
        .route("/api/v1/orders", get(handlers::list_orders).post(handlers::create_order))
        .route("/api/v1/orders/{id}", get(handlers::order_detail))
        .route("/api/v1/orders/{id}/cancel", axum::routing::post(handlers::cancel_order))
        .layer(ServiceBuilder::new().map_err(no_error).layer(state.jwt.clone()));
    // 内部接口（P4-07）：payment-service 回调确认，不挂 JWT，用 X-Internal-Token 防护
    let internal = Router::new().route(
        "/api/v1/orders/{id}/pay-success",
        axum::routing::post(handlers::pay_success),
    );

    Router::new()
        .merge(orders)
        .merge(internal)
        .layer(
            ServiceBuilder::new()
                .map_err(no_error)
                .layer(CircuitBreakerLayer::new())
                .map_err(no_error)
                .layer(SecurityLayer::new())
                .map_err(no_error)
                .layer(RedisRateLimitLayer::new(state.cache.clone(), "order-service", 100, 60)),
        )
        .with_state(state)
}

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_id_gen();
    let jwt = JwtAuthLayer::new(jwt_secret()).expect("valid jwt secret");
    let state = AppState {
        db: connect_primary().await,
        cache: connect_cache().await,
        mq: connect_kafka().await,
        jwt: jwt.clone(),
    };

    // 启动即跑一轮过期扫描，之后每 60s 一轮（fail-tolerant，仅告警）
    let sweep_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            handlers::expire_pending_orders(&sweep_state).await;
        }
    });

    let api = api_router(state.clone());
    let router = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .with_state(state)
        .merge(api)
        .layer(
            ServiceBuilder::new()
                .layer(TracingLayer::new("order-service"))
                .layer(LoggingLayer),
        );

    let http_srv = HttpServer::new(PORT).router(router);

    let mut app = App::builder()
        .name("order-service")
        .version("v0.1.0")
        .server(http_srv)
        .on_start(|| async {
            tracing::info!("order-service listening on {PORT}");
            Ok(())
        })
        .build()?;

    app.run().await?;
    Ok(())
}
