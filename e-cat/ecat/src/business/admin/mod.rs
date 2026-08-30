// open-travel admin-service：管理端登录（JWT role=admin）
//
// 端口 8003。密码方案与 user-service 完全一致（bcrypt + JWT），claims 增加
// role="admin"，sub 为 admin id。中间件链同 user-service：ApiVersion →
// CircuitBreaker → Security → RedisRateLimit；login 公开（限流防暴力）。
// 防枚举：邮箱不存在时仍对固定 hash 执行一次 bcrypt verify，抹平时序差，
// 统一 401。受保护的管理端点用 require_admin 守卫（校验 claims role=admin）。
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use ecat::App;
use ecat_auth::{claims_from_request, AuthClaims, JwtAuthLayer};
use ecat_circuit_breaker::CircuitBreakerLayer;
use ecat_data::RdbmsClient;
use ecat_data_redis::RedisCache;
use ecat_data_sqlx::SqlxClient;
use ecat_middleware::LoggingLayer;
use ecat_mq_kafka::KafkaMq;
use ecat_security::SecurityLayer;
use ecat_tracing::TracingLayer;
use ecat_transport_http::HttpServer;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ecat::business::shared::{
    connect_kafka, connect_primary, jwt_secret, no_error, publish_audit, RedisRateLimitLayer,
};
use std::sync::Arc;
use tower::ServiceBuilder;

#[path = "handlers.rs"]
pub(crate) mod handlers;

#[path = "line_handlers.rs"]
pub(crate) mod line_handlers;

#[path = "line_date_handlers.rs"]
pub(crate) mod line_date_handlers;

#[path = "orders_handlers.rs"]
pub(crate) mod orders_handlers;

#[path = "users_handlers.rs"]
pub(crate) mod users_handlers;

#[path = "payments_handlers.rs"]
pub(crate) mod payments_handlers;

#[path = "flight_handlers.rs"]
pub(crate) mod flight_handlers;

#[path = "hotel_handlers.rs"]
pub(crate) mod hotel_handlers;

#[path = "stats_handlers.rs"]
pub(crate) mod stats_handlers;

#[path = "reports_handlers.rs"]
pub(crate) mod reports_handlers;

#[path = "cdn_handlers.rs"]
pub(crate) mod cdn_handlers;

const PORT: &str = "0.0.0.0:8003";
const TOKEN_TTL_SECS: u64 = 24 * 3600;
// 防枚举：未知邮箱对固定 hash 执行一次 bcrypt verify，与真实校验耗时一致
const DUMMY_HASH: &str = "$2b$12$9hcIip6Kh.hGJ5XqqwVzreqgqvw4tknymE53dKMi1Qb9pkbPQ88N6";
const DUMMY_PASSWORD: &str = "dummy-password-for-timing";

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

#[derive(Deserialize)]
pub(crate) struct LoginReq {
    pub(crate) email: String,
    pub(crate) password: String,
}

#[derive(Serialize)]
pub(crate) struct LoginOut {
    pub(crate) token: String,
}

/// 签发用 claims：sub 为 admin id，role 固定 "admin"；iat/exp 由 sign 自动注入。
#[derive(Serialize)]
pub(crate) struct LoginClaims {
    pub(crate) sub: String,
    pub(crate) role: String,
}

async fn health() -> &'static str {
    "OK"
}

pub(crate) async fn ready(State(state): State<AppState>) -> Json<ApiResponse<bool>> {
    let ready = state.db.is_some() && state.cache.is_some();
    Json(ApiResponse { code: 0, message: "ready".into(), data: Some(ready) })
}

pub(crate) async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginReq>,
) -> (StatusCode, Json<ApiResponse<LoginOut>>) {
    let email = body.email.trim().to_string();
    if email.is_empty() || body.password.is_empty() {
        return err(StatusCode::BAD_REQUEST, 400, "email and password required");
    }
    let Some(db) = state.db.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    };

    let rows = match db
        .query_with(
            "SELECT id, password_hash FROM travel_admins WHERE email = ? AND status = 1",
            &[json!(email)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "admin login query failed");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    // 统一 401 防枚举：邮箱不存在也走一次 bcrypt verify，抹平时序差
    let Some(row) = rows.first() else {
        let _ = tokio::task::spawn_blocking(move || {
            bcrypt::verify(DUMMY_PASSWORD, DUMMY_HASH)
        })
        .await;
        return err(StatusCode::UNAUTHORIZED, 401, "invalid credentials");
    };
    let Some(hash) = row.get("password_hash").and_then(|v| v.as_str()) else {
        return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
    };
    let Some(admin_id) = row.get("id").and_then(|v| v.as_u64()) else {
        return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
    };

    let hash = hash.to_string();
    let password = body.password;
    let ok = match tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash)).await {
        Ok(Ok(v)) => v,
        _ => return err(StatusCode::UNAUTHORIZED, 401, "invalid credentials"),
    };
    if !ok {
        return err(StatusCode::UNAUTHORIZED, 401, "invalid credentials");
    }

    let token = match state
        .jwt
        .sign(&LoginClaims { sub: admin_id.to_string(), role: "admin".into() }, TOKEN_TTL_SECS)
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "jwt sign failed");
            return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
        }
    };
    if let Some(mq) = &state.mq {
        publish_audit(mq, "admin.login", admin_id, json!({})).await;
    }
    (StatusCode::OK, ApiResponse::ok(LoginOut { token }))
}

/// 校验 claims 是否具备 admin 角色（无 claims 401 / 非 admin 403）。
fn admin_check(claims: Option<&AuthClaims>) -> Result<AuthClaims, Response> {
    let Some(claims) = claims else {
        return Err(err::<serde_json::Value>(
            StatusCode::UNAUTHORIZED,
            401,
            "missing claims",
        )
        .into_response());
    };
    if !claims.has_role("admin") {
        return Err(err::<serde_json::Value>(
            StatusCode::FORBIDDEN,
            403,
            "admin role required",
        )
        .into_response());
    }
    Ok(claims.clone())
}

/// 受保护管理端点守卫（handler 内直接调用）。仅集成测试使用，主程序无引用。
#[allow(dead_code)]
pub(crate) fn require_admin(req: &Request) -> Result<AuthClaims, Response> {
    admin_check(claims_from_request(req))
}

/// 受保护管理端点守卫（提取器形态，可与 Json/Query 组合使用）：
/// 从 JwtAuthLayer 注入的 extensions 取 claims，校验 role=admin。
pub(crate) struct AdminGuard(#[allow(dead_code)] pub(crate) AuthClaims);

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for AdminGuard {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        admin_check(parts.extensions.get::<AuthClaims>()).map(AdminGuard)
    }
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

/// 业务路由（login 公开 + CRUD 全挂 JWT）+ 中间件链；独立成函数便于集成测试直接构造。
/// CRUD 路由经 JwtAuthLayer 校验 token 并把 claims 注入请求，handler 内
/// require_admin 再校验 role=admin。
pub(crate) fn api_router(state: AppState) -> Router {
    let admin = Router::new()
        .route(
            "/api/admin/destinations",
            get(handlers::list_destinations).post(handlers::create_destination),
        )
        .route("/api/admin/destinations/{id}", put(handlers::update_destination))
        .route(
            "/api/admin/destinations/{id}/status",
            put(handlers::update_destination_status),
        )
        .route("/api/admin/destinations/{id}", delete(handlers::delete_destination))
        .route(
            "/api/admin/attractions",
            get(handlers::list_attractions).post(handlers::create_attraction),
        )
        .route("/api/admin/attractions/{id}", put(handlers::update_attraction))
        .route("/api/admin/attractions/{id}", delete(handlers::delete_attraction))
        .route(
            "/api/admin/lines",
            get(line_handlers::list_lines).post(line_handlers::create_line),
        )
        .route("/api/admin/lines/{id}", put(line_handlers::update_line))
        .route("/api/admin/lines/{id}", delete(line_handlers::delete_line))
        .route(
            "/api/admin/lines/{id}/status",
            put(line_handlers::update_line_status),
        )
        .route(
            "/api/admin/lines/{id}/dates",
            get(line_date_handlers::list_line_dates).post(line_date_handlers::create_line_date),
        )
        .route(
            "/api/admin/lines/{id}/dates/{date_id}",
            put(line_date_handlers::update_line_date),
        )
        .route(
            "/api/admin/lines/{id}/dates/{date_id}",
            delete(line_date_handlers::delete_line_date),
        )
        .route("/api/admin/stats/overview", get(stats_handlers::overview))
        .route("/api/admin/stats/top", get(stats_handlers::top))
        .route("/api/admin/stats/trend", get(stats_handlers::trend))
        .route("/api/admin/reports/sales", get(reports_handlers::sales_report))
        .route("/api/admin/reports/payments", get(reports_handlers::payments_report))
        .route("/api/admin/orders", get(orders_handlers::list_orders))
        .route("/api/admin/orders/{id}", get(orders_handlers::order_detail))
        .route("/api/admin/orders/{id}/refund", post(orders_handlers::refund_order))
        .route("/api/admin/users", get(users_handlers::list_users))
        .route("/api/admin/users/{id}/status", patch(users_handlers::update_user_status))
        .route("/api/admin/payments", get(payments_handlers::list_payments))
        .route("/api/admin/payments/channels", get(payments_handlers::list_channels))
        .route(
            "/api/admin/payments/channels/{code}/enabled",
            patch(payments_handlers::update_channel_enabled),
        )
        .route(
            "/api/admin/flights",
            get(flight_handlers::list_flights).post(flight_handlers::create_flight),
        )
        .route("/api/admin/flights/{id}", put(flight_handlers::update_flight))
        .route("/api/admin/flights/{id}", delete(flight_handlers::delete_flight))
        .route(
            "/api/admin/flights/{id}/status",
            put(flight_handlers::update_flight_status),
        )
        .route(
            "/api/admin/hotels",
            get(hotel_handlers::list_hotels).post(hotel_handlers::create_hotel),
        )
        .route("/api/admin/hotels/{id}", put(hotel_handlers::update_hotel))
        .route("/api/admin/hotels/{id}", delete(hotel_handlers::delete_hotel))
        .route(
            "/api/admin/hotels/{id}/status",
            put(hotel_handlers::update_hotel_status),
        )
        .route(
            "/api/admin/hotels/{id}/rooms",
            get(hotel_handlers::list_rooms).post(hotel_handlers::create_room),
        )
        .route(
            "/api/admin/hotels/{id}/rooms/{room_id}",
            put(hotel_handlers::update_room),
        )
        .route(
            "/api/admin/hotels/{id}/rooms/{room_id}",
            delete(hotel_handlers::delete_room),
        )
        .route("/api/admin/cdn/providers", get(cdn_handlers::list_providers))
        .route("/api/admin/cdn/providers/{code}", put(cdn_handlers::update_provider))
        .route(
            "/api/admin/cdn/providers/{code}/status",
            patch(cdn_handlers::update_provider_status),
        )
        .route("/api/admin/cdn/providers/{code}/plan", post(cdn_handlers::provider_plan))
        .layer(ServiceBuilder::new().map_err(no_error).layer(state.jwt.clone()));

    Router::new()
        .route("/api/admin/login", post(login))
        .merge(admin)
        .layer(
            ServiceBuilder::new()
                .layer(ecat::business::shared::ApiVersionLayer)
                .map_err(no_error)
                .layer(CircuitBreakerLayer::new())
                .map_err(no_error)
                .layer(SecurityLayer::new())
                .map_err(no_error)
                .layer(RedisRateLimitLayer::new(state.cache.clone(), "admin-service", 100, 60)),
        )
        .with_state(state)
}

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let jwt = JwtAuthLayer::new(jwt_secret()).expect("valid jwt secret");
    let state = AppState {
        db: connect_primary().await,
        cache: connect_cache().await,
        mq: connect_kafka().await,
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
                .layer(TracingLayer::new("admin-service"))
                .layer(LoggingLayer),
        );

    let http_srv = HttpServer::new(PORT).router(router);

    let mut app = App::builder()
        .name("admin-service")
        .version("v0.1.0")
        .server(http_srv)
        .on_start(|| async {
            tracing::info!("admin-service listening on {PORT}");
            Ok(())
        })
        .build()?;

    app.run().await?;
    Ok(())
}
