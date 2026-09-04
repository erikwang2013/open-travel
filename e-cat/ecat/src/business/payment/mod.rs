// open-travel payment-service：支付发起 / 模拟收银台 / 渠道回调 / 流水列表 / 渠道列表（P4-06/P4-15）
//
// 端口 8009（网关 /api/v1/payments/ → ecat-payment:8009）。
// travel_payments.status：0待支付 → 1成功 → 2失败（3已退款，本期不使用）。
// 幂等：同订单发起支付已存在流水直接返回；回调按 txn_no 更新且仅接受 status=0，
//      重复回调（已终态 1/2/3）直接返回成功不重复入账。
// 验签：开发环境模拟——X-Signature = HMAC-SHA256(原始请求体, "sandbox-secret")，
//      验签失败 401 且不入账。
// 渠道抽象（P4-15）：本期所有渠道统一走模拟实现（sandbox 收银台 + 模拟验签）；
//      真实渠道接入点：handlers::create_payment 内按 channel_code 分发替换。
//
// 中间件链（外层 → 内层）：ApiVersion → CircuitBreaker → Security → RateLimit
// → [仅 /api/v1/payments] JWT。JWT 与 user-service 同一密钥，claims.sub 为 user_id。
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use ecat::App;
use ecat_auth::{AuthClaims, JwtAuthLayer};
use ecat_circuit_breaker::CircuitBreakerLayer;
use ecat_data_sqlx::SqlxClient;
use ecat_middleware::LoggingLayer;
use ecat_security::SecurityLayer;
use ecat_tracing::TracingLayer;
use ecat_transport_http::HttpServer;
use serde::Serialize;
use ecat::business::shared::{connect_primary, jwt_secret, no_error, RedisRateLimitLayer};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tower::ServiceBuilder;

#[path = "handlers.rs"]
pub(crate) mod handlers;

const PORT: &str = "0.0.0.0:8009";

/// 内部确认回调（order-service pay-success）的返回类型。
pub(crate) type ConfirmFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;

/// 支付确认抽象：真实实现 HTTP 调 order-service，集成测试注入 mock。
pub(crate) trait OrderConfirm: Send + Sync {
    fn confirm(&self, order_id: u64, txn_no: &str) -> ConfirmFuture;
}

/// 真实实现：POST {ORDER_SERVICE_URL}/api/v1/orders/{id}/pay-success，带 X-Internal-Token。
pub(crate) struct HttpOrderConfirm {
    client: reqwest::Client,
    url: String,
}

impl OrderConfirm for HttpOrderConfirm {
    fn confirm(&self, order_id: u64, txn_no: &str) -> ConfirmFuture {
        let client = self.client.clone();
        let url = format!("{}/api/v1/orders/{}/pay-success", self.url, order_id);
        let txn_no = txn_no.to_string();
        Box::pin(async move {
            let resp = client
                .post(url)
                .header("x-internal-token", internal_token())
                .json(&serde_json::json!({ "txn_no": txn_no }))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if resp.status().is_success() {
                Ok(())
            } else {
                Err(format!("order service returned {}", resp.status()))
            }
        })
    }
}

/// 内部接口共享密钥（order-service 同源，环境变量可覆盖）。
pub(crate) fn internal_token() -> String {
    std::env::var("INTERNAL_TOKEN").unwrap_or_else(|_| "dev-internal-secret".into())
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: Option<Arc<SqlxClient>>,
    pub(crate) confirm: Arc<dyn OrderConfirm>,
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

/// 登录态守卫（与 order-service 同款）：JwtAuthLayer 注入的 claims.sub 即 user_id。
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

async fn health() -> &'static str {
    "OK"
}

pub(crate) async fn ready(State(state): State<AppState>) -> Json<ApiResponse<bool>> {
    Json(ApiResponse { code: 0, message: "ready".into(), data: Some(state.db.is_some()) })
}

/// 业务路由 + 中间件链；独立成函数便于集成测试直接构造。
/// JWT 只挂在 /api/v1/payments（用户接口）；callback/sandbox/channels 公开。
pub(crate) fn api_router(state: AppState) -> Router {
    let authed = Router::new()
        .route(
            "/api/v1/payments",
            get(handlers::payment_list).post(handlers::create_payment),
        )
        .layer(ServiceBuilder::new().map_err(no_error).layer(state.jwt.clone()));
    let public = Router::new()
        .route(
            "/api/v1/payments/callback/{channel_code}",
            axum::routing::post(handlers::payment_callback),
        )
        .route("/api/v1/payments/sandbox/{txn_no}", get(handlers::sandbox_page))
        .route("/api/v1/payments/channels", get(handlers::channel_list));

    Router::new()
        .merge(public)
        .merge(authed)
        .layer(
            ServiceBuilder::new()
                .map_err(no_error)
                .layer(CircuitBreakerLayer::new())
                .map_err(no_error)
                .layer(SecurityLayer::new())
                .map_err(no_error)
                .layer(RedisRateLimitLayer::new(None, "payment-service", 100, 60)),
        )
        .with_state(state)
}

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let jwt = JwtAuthLayer::new(jwt_secret()).expect("valid jwt secret");
    let order_url =
        std::env::var("ORDER_SERVICE_URL").unwrap_or_else(|_| "http://ecat-order:8006".into());
    let state = AppState {
        db: connect_primary().await,
        confirm: Arc::new(HttpOrderConfirm { client: reqwest::Client::new(), url: order_url }),
        jwt: jwt.clone(),
    };

    let api = api_router(state.clone());
    let router = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .with_state(state)
        .merge(api)
        .layer(
            ServiceBuilder::new()
                .layer(TracingLayer::new("payment-service"))
                .layer(LoggingLayer),
        );

    let http_srv = HttpServer::new(PORT).router(router);

    let mut app = App::builder()
        .name("payment-service")
        .version("v0.1.0")
        .server(http_srv)
        .on_start(|| async {
            tracing::info!("payment-service listening on {PORT}");
            Ok(())
        })
        .build()?;

    app.run().await?;
    Ok(())
}
