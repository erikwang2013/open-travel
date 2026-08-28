// open-travel user-service：用户注册 / 登录 / 资料（JWT 登录闭环）
//
// 端口约定：HTTP 默认 :8000；user 服务用 8001、booking 用 8002，
// 便于本地同时启动调试，避免端口冲突。
//
// 中间件链（外层 → 内层）：Tracing → CircuitBreaker → Security → RateLimit
// 仅 /api/v1/user/profile 再挂 Auth(JWT)；/register、/login 公开（限流仍生效，
// 防止暴力破解）。/health、/ready 不走鉴权。
// 注：e-cat 的 RecoveryLayer 与 axum Router::layer 不兼容（Error=Box<dyn Error>
// 不满足 Into<Infallible>），故省略（axum 自带 panic 捕获）；
// 限流为 shared::RedisRateLimitLayer（Redis 分布式固定窗口，fail-open）；
// Auth/Security/CircuitBreaker 以 map_err 归一为 Infallible。
//
// 数据源连接失败仅告警不退出（无 MySQL/Redis 环境时服务仍可启动）。
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use ecat::App;
use ecat_auth::claims_from_request;
use ecat_auth::JwtAuthLayer;
use ecat_circuit_breaker::CircuitBreakerLayer;
use ecat_data::RdbmsClient;
use ecat_data_redis::RedisCache;
use ecat_data_sqlx::SqlxClient;
use ecat_middleware::LoggingLayer;
use ecat_security::SecurityLayer;
use ecat_tracing::TracingLayer;
use ecat_transport_http::HttpServer;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::{jwt_secret, no_error, RedisRateLimitLayer};
use std::sync::Arc;
use tower::ServiceBuilder;

const PORT: &str = "0.0.0.0:8001";
const TOKEN_TTL_SECS: u64 = 24 * 3600;

#[derive(Clone)]
struct AppState {
    db: Option<Arc<SqlxClient>>,
    cache: Option<Arc<RedisCache>>,
    jwt: JwtAuthLayer,
}

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
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
struct RegisterReq {
    email: String,
    password: String,
    lang: Option<String>,
}

#[derive(Deserialize)]
struct LoginReq {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct UserOut {
    user_id: u64,
    email: String,
}

#[derive(Serialize)]
struct LoginOut {
    token: String,
    user_id: u64,
    email: String,
}

#[derive(Serialize)]
struct ProfileOut {
    user_id: u64,
    email: String,
    lang: String,
}

/// 签发用 claims：sub 为 user_id，iat/exp 由 JwtAuthLayer::sign 自动注入。
#[derive(Serialize)]
struct LoginClaims {
    sub: String,
}

async fn health() -> &'static str {
    "OK"
}

async fn ready(State(state): State<AppState>) -> Json<ApiResponse<bool>> {
    // 数据源缺失时 /ready 报告降级状态，但不阻塞服务启动
    let ready = state.db.is_some() && state.cache.is_some();
    Json(ApiResponse { code: 0, message: "ready".into(), data: Some(ready) })
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterReq>,
) -> (StatusCode, Json<ApiResponse<UserOut>>) {
    let email = body.email.trim();
    if email.is_empty() || !email.contains('@') {
        return err(StatusCode::BAD_REQUEST, 400, "invalid email");
    }
    if body.password.len() < 6 {
        return err(StatusCode::BAD_REQUEST, 400, "password must be at least 6 characters");
    }
    let Some(db) = state.db.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    };

    let dup = match db
        .query_with("SELECT id FROM travel_users WHERE email = ?", &[json!(email)])
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "register duplicate check failed");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    if !dup.is_empty() {
        return err(StatusCode::CONFLICT, 409, "email already registered");
    }

    // bcrypt cost 12 约百毫秒级，spawn_blocking 避免阻塞异步线程
    let password = body.password;
    let hash = match tokio::task::spawn_blocking(move || bcrypt::hash(password, bcrypt::DEFAULT_COST))
        .await
    {
        Ok(Ok(h)) => h,
        _ => {
            tracing::error!("bcrypt hash failed");
            return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
        }
    };

    let lang = body.lang.as_deref().unwrap_or("en");
    // 预查已排除重复；INSERT 仍失败按重复处理（唯一索引兜底，竞态安全）
    match db
        .execute_with(
            "INSERT INTO travel_users (email, password_hash, lang) VALUES (?, ?, ?)",
            &[json!(email), json!(hash), json!(lang)],
        )
        .await
    {
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, "register insert failed");
            return err(StatusCode::CONFLICT, 409, "email already registered");
        }
    }

    let rows = match db
        .query_with("SELECT id FROM travel_users WHERE email = ?", &[json!(email)])
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "register fetch id failed");
            return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
        }
    };
    let Some(user_id) = rows.first().and_then(|r| r.get("id")).and_then(|v| v.as_u64()) else {
        return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
    };
    (StatusCode::OK, ApiResponse::ok(UserOut { user_id, email: email.to_string() }))
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginReq>,
) -> (StatusCode, Json<ApiResponse<LoginOut>>) {
    let email = body.email.trim().to_string();
    if email.is_empty() {
        return err(StatusCode::UNAUTHORIZED, 401, "invalid credentials");
    }
    let Some(db) = state.db.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    };

    let rows = match db
        .query_with(
            "SELECT id, email, password_hash FROM travel_users WHERE email = ?",
            &[json!(email)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "login query failed");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    // 统一 401：不区分"邮箱不存在"与"密码错误"，避免枚举账号
    let Some(row) = rows.first() else {
        return err(StatusCode::UNAUTHORIZED, 401, "invalid credentials");
    };
    let Some(hash) = row.get("password_hash").and_then(|v| v.as_str()) else {
        return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
    };
    let Some(user_id) = row.get("id").and_then(|v| v.as_u64()) else {
        return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
    };

    // 转 owned：spawn_blocking 要求 'static 闭包，不能借用 rows
    let hash = hash.to_string();
    let password = body.password;
    let ok = match tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash)).await {
        Ok(Ok(v)) => v,
        _ => return err(StatusCode::UNAUTHORIZED, 401, "invalid credentials"),
    };
    if !ok {
        return err(StatusCode::UNAUTHORIZED, 401, "invalid credentials");
    }

    let token = match state.jwt.sign(&LoginClaims { sub: user_id.to_string() }, TOKEN_TTL_SECS) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "jwt sign failed");
            return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
        }
    };
    (StatusCode::OK, ApiResponse::ok(LoginOut { token, user_id, email }))
}

async fn profile(State(state): State<AppState>, req: Request) -> Response {
    let Some(claims) = claims_from_request(&req) else {
        return err::<ProfileOut>(StatusCode::UNAUTHORIZED, 401, "missing claims").into_response();
    };
    let Ok(user_id) = claims.sub.parse::<u64>() else {
        return err::<ProfileOut>(StatusCode::BAD_REQUEST, 400, "invalid subject in token")
            .into_response();
    };
    let Some(db) = state.db.clone() else {
        return err::<ProfileOut>(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable")
            .into_response();
    };
    let rows = match db
        .query_with(
            "SELECT id, email, lang FROM travel_users WHERE id = ?",
            &[json!(user_id)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "profile query failed");
            return err::<ProfileOut>(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable")
                .into_response();
        }
    };
    let Some(row) = rows.first() else {
        return err::<ProfileOut>(StatusCode::NOT_FOUND, 404, "user not found").into_response();
    };
    let email = row.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let lang = row.get("lang").and_then(|v| v.as_str()).unwrap_or("en").to_string();
    (StatusCode::OK, ApiResponse::ok(ProfileOut { user_id, email, lang })).into_response()
}

async fn connect_db() -> Option<Arc<SqlxClient>> {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let jwt = JwtAuthLayer::new(jwt_secret()).expect("valid jwt secret");
    let state = AppState { db: connect_db().await, cache: connect_cache().await, jwt: jwt.clone() };

    // 业务路由：注册/登录公开；profile 挂 JWT（Auth 层内）。
    // 执行顺序（外层 → 内层）：CircuitBreaker → Security → RateLimit
    //   → [profile 仅] Auth(JWT)
    // e-cat 中间件的 Error 非 Infallible，需 map_err 归一以满足 axum Router::layer
    // 约束；RateLimit（Redis 分布式）覆盖全部业务路由：未认证请求也计入限流，
    // 防止暴力请求耗尽资源。
    let api = Router::new()
        .route("/api/v1/user/register", post(register))
        .route("/api/v1/user/login", post(login))
        .merge(
            Router::new()
                .route("/api/v1/user/profile", get(profile))
                .layer(ServiceBuilder::new().map_err(no_error).layer(jwt)),
        )
        .layer(
            ServiceBuilder::new()
                .map_err(no_error)
                .layer(CircuitBreakerLayer::new())
                .map_err(no_error)
                .layer(SecurityLayer::new())
                .map_err(no_error)
                .layer(RedisRateLimitLayer::new(
                    state.cache.clone(),
                    "user-service",
                    100,
                    60,
                )),
        );

    // 全局：Tracing（注入 trace_id）→ Logging
    let router = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .merge(api)
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TracingLayer::new("user-service"))
                .layer(LoggingLayer),
        );

    let http_srv = HttpServer::new(PORT).router(router);

    let mut app = App::builder()
        .name("user-service")
        .version("v0.1.0")
        .server(http_srv)
        .on_start(|| async {
            tracing::info!("user-service listening on {PORT}");
            Ok(())
        })
        .build()?;

    app.run().await?;
    Ok(())
}
