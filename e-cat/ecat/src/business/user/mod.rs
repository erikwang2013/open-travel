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
// 限流为 ecat::business::shared::RedisRateLimitLayer（Redis 分布式固定窗口，fail-open）；
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
use serde_json::{json, Value};
use ecat_mq_kafka::KafkaMq;
use ecat::business::shared::{connect_kafka, connect_primary, jwt_secret, no_error, publish_audit, RedisRateLimitLayer};
use std::sync::Arc;
use tower::ServiceBuilder;

const PORT: &str = "0.0.0.0:8001";
const TOKEN_TTL_SECS: u64 = 24 * 3600;

// 支持的语言列表（与 docs/travel-project-planning-v2.md 的 12 语种一致）
const SUPPORTED_LANGS: &[&str] =
    &["en", "zh", "ja", "ko", "ru", "de", "fr", "es", "pt", "hi", "ar", "bn", "id"];

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
pub(crate) struct RegisterReq {
    pub(crate) email: String,
    pub(crate) password: String,
    pub(crate) lang: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct LoginReq {
    pub(crate) email: String,
    pub(crate) password: String,
}

#[derive(Serialize)]
pub(crate) struct UserOut {
    user_id: u64,
    email: String,
}

#[derive(Serialize)]
pub(crate) struct LoginOut {
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

#[derive(Deserialize)]
pub(crate) struct ProfileUpdateReq {
    pub(crate) nickname: Option<String>,
    pub(crate) lang: Option<String>,
}

#[derive(Serialize)]
struct ProfileUpdateOut {
    id: u64,
    email: String,
    nickname: String,
    lang: String,
}

/// 签发用 claims：sub 为 user_id，iat/exp 由 JwtAuthLayer::sign 自动注入。
#[derive(Serialize)]
pub(crate) struct LoginClaims {
    pub(crate) sub: String,
}

async fn health() -> &'static str {
    "OK"
}

pub(crate) async fn ready(State(state): State<AppState>) -> Json<ApiResponse<bool>> {
    // 数据源缺失时 /ready 报告降级状态，但不阻塞服务启动
    let ready = state.db.is_some() && state.cache.is_some();
    Json(ApiResponse { code: 0, message: "ready".into(), data: Some(ready) })
}

pub(crate) async fn register(
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
    // 主键去 AUTO_INCREMENT 后显式生成雪花 id；预查已排除重复，
    // INSERT 仍失败按重复处理（唯一索引兜底，竞态安全）
    let user_id = ecat::business::shared::snowflake_id().await;
    match db
        .execute_with(
            "INSERT INTO travel_users (id, email, password_hash, lang) VALUES (?, ?, ?, ?)",
            &[json!(user_id), json!(email), json!(hash), json!(lang)],
        )
        .await
    {
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, "register insert failed");
            return err(StatusCode::CONFLICT, 409, "email already registered");
        }
    }
    if let Some(mq) = &state.mq {
        publish_audit(mq, "user.register", user_id, json!({ "lang": lang })).await;
    }
    (StatusCode::OK, ApiResponse::ok(UserOut { user_id, email: email.to_string() }))
}

pub(crate) async fn login(
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
    if let Some(mq) = &state.mq {
        publish_audit(mq, "user.login", user_id, json!({})).await;
    }
    (StatusCode::OK, ApiResponse::ok(LoginOut { token, user_id, email }))
}

pub(crate) async fn profile(State(state): State<AppState>, req: Request) -> Response {
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
    // P4-14：禁用用户（status=1）的 JWT 请求一律 403
    if let Err(resp) = ensure_user_enabled(&state, user_id).await {
        return resp;
    }
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

pub(crate) async fn update_profile(State(state): State<AppState>, req: Request) -> Response {
    let Some(claims) = claims_from_request(&req) else {
        return err::<ProfileUpdateOut>(StatusCode::UNAUTHORIZED, 401, "missing claims")
            .into_response();
    };
    let Ok(user_id) = claims.sub.parse::<u64>() else {
        return err::<ProfileUpdateOut>(StatusCode::BAD_REQUEST, 400, "invalid subject in token")
            .into_response();
    };
    // claims_from_request 需要整 Request，body 手动解析（不再用 Json extractor）
    let bytes = match axum::body::to_bytes(req.into_body(), 1 << 16).await {
        Ok(b) => b,
        Err(_) => {
            return err::<ProfileUpdateOut>(StatusCode::BAD_REQUEST, 400, "invalid json body")
                .into_response()
        }
    };
    let body: ProfileUpdateReq = match serde_json::from_slice(&bytes) {
        Ok(b) => b,
        Err(_) => {
            return err::<ProfileUpdateOut>(StatusCode::BAD_REQUEST, 400, "invalid json body")
                .into_response()
        }
    };
    let nickname = body.nickname.as_deref().unwrap_or("").trim();
    if nickname.len() > 100 {
        return err::<ProfileUpdateOut>(StatusCode::BAD_REQUEST, 400, "nickname too long")
            .into_response();
    }
    if let Some(lang) = body.lang.as_deref() {
        if !SUPPORTED_LANGS.contains(&lang) {
            return err::<ProfileUpdateOut>(StatusCode::BAD_REQUEST, 400, "unsupported lang")
                .into_response();
        }
    }
    let Some(db) = state.db.clone() else {
        return err::<ProfileUpdateOut>(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable")
            .into_response();
    };
    // P4-14：禁用用户（status=1）的 JWT 请求一律 403
    if let Err(resp) = ensure_user_enabled(&state, user_id).await {
        return resp;
    }
    let rows = match db
        .query_with(
            "SELECT id, email, nickname, lang FROM travel_users WHERE id = ?",
            &[json!(user_id)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "profile update select failed");
            return err::<ProfileUpdateOut>(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable")
                .into_response();
        }
    };
    let Some(row) = rows.first() else {
        return err::<ProfileUpdateOut>(StatusCode::NOT_FOUND, 404, "user not found").into_response();
    };
    let email = row.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let cur_nickname = row.get("nickname").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let cur_lang = row.get("lang").and_then(|v| v.as_str()).unwrap_or("en").to_string();
    // 未提供字段保持原值（幂等 UPDATE 两列）
    let new_nickname = body.nickname.map(|s| s.trim().to_string()).unwrap_or(cur_nickname);
    let new_lang = body.lang.unwrap_or(cur_lang);
    if let Err(e) = db
        .execute_with(
            "UPDATE travel_users SET nickname = ?, lang = ? WHERE id = ?",
            &[json!(new_nickname), json!(new_lang), json!(user_id)],
        )
        .await
    {
        tracing::warn!(error = %e, "profile update failed");
        return err::<ProfileUpdateOut>(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable")
            .into_response();
    }
    (StatusCode::OK, ApiResponse::ok(ProfileUpdateOut {
        id: user_id,
        email,
        nickname: new_nickname,
        lang: new_lang,
    }))
    .into_response()
}

/// P4-14 禁用检查：token 有效但 travel_users.status=1（禁用）→ 403。
/// 每次请求查一次库（profile 低频，简单优先；压测需要时再上 Redis 缓存）。
/// db 缺失或用户不存在时放行，交给 handler 的 503/404 兜底。
async fn ensure_user_enabled(state: &AppState, user_id: u64) -> Result<(), Response> {
    let Some(db) = state.db.clone() else { return Ok(()) };
    let rows = match db
        .query_with(
            "SELECT CAST(status AS SIGNED) AS status FROM travel_users WHERE id = ?",
            &[json!(user_id)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "user status query failed");
            return Err(err::<Value>(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable")
                .into_response());
        }
    };
    let Some(row) = rows.first() else { return Ok(()) };
    let status = row
        .get("status")
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0);
    if status == 1 {
        return Err(err::<Value>(StatusCode::FORBIDDEN, 403, "account disabled").into_response());
    }
    Ok(())
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

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let jwt = JwtAuthLayer::new(jwt_secret()).expect("valid jwt secret");
    let state = AppState {
        db: connect_primary().await,
        cache: connect_cache().await,
        mq: connect_kafka().await,
        jwt: jwt.clone(),
    };

    // 业务路由：注册/登录公开；profile 挂 JWT（Auth 层内）。
    // 执行顺序（外层 → 内层）：ApiVersion → CircuitBreaker → Security → RateLimit
    //   → [profile 仅] Auth(JWT)
    // e-cat 中间件的 Error 非 Infallible，需 map_err 归一以满足 axum Router::layer
    // 约束；RateLimit（Redis 分布式）覆盖全部业务路由：未认证请求也计入限流，
    // 防止暴力请求耗尽资源。
    let api = Router::new()
        .route("/api/v1/user/register", post(register))
        .route("/api/v1/user/login", post(login))
        .merge(
            Router::new()
                .route("/api/v1/user/profile", get(profile).put(update_profile))
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
