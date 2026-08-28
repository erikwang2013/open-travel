// open-travel user-service：用户资料与注册（Phase 3 安全加固后）
//
// 端口约定：HTTP 默认 :8000；user 服务用 8001、booking 用 8002，
// 便于本地同时启动调试，避免端口冲突。
//
// 中间件链（外层 → 内层）：Tracing → Auth(JWT) → RateLimit → Security → CircuitBreaker
// /health、/ready 不走鉴权；业务路由挂在独立子 Router 上。
// 注：e-cat 的 RecoveryLayer 与 axum Router::layer 不兼容（Error=Box<dyn Error>
// 不满足 Into<Infallible>），故省略（axum 自带 panic 捕获）；
// 限流为 shared::RedisRateLimitLayer（Redis 分布式固定窗口，fail-open）；
// Auth/Security/CircuitBreaker 以 map_err 归一为 Infallible。
//
// 数据源连接失败仅告警不退出（无 MySQL/Redis 环境时服务仍可启动）。
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use ecat::App;
use ecat_auth::JwtAuthLayer;
use ecat_circuit_breaker::CircuitBreakerLayer;
use ecat_data_redis::RedisCache;
use ecat_data_sqlx::SqlxClient;
use ecat_middleware::LoggingLayer;
use ecat_security::SecurityLayer;
use ecat_tracing::TracingLayer;
use ecat_transport_http::HttpServer;
use serde::Serialize;
use shared::{jwt_secret, no_error, RedisRateLimitLayer};
use std::sync::Arc;
use tower::ServiceBuilder;

const PORT: &str = "0.0.0.0:8001";

#[derive(Clone)]
struct AppState {
    db: Option<Arc<SqlxClient>>,
    cache: Option<Arc<RedisCache>>,
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

#[derive(Serialize)]
struct Profile {
    user_id: u64,
    nickname: String,
}

async fn health() -> &'static str {
    "OK"
}

async fn ready(State(state): State<AppState>) -> Json<ApiResponse<bool>> {
    // 数据源缺失时 /ready 报告降级状态，但不阻塞服务启动
    let ready = state.db.is_some() && state.cache.is_some();
    Json(ApiResponse { code: 0, message: "ready".into(), data: Some(ready) })
}

// TODO(Phase 2)：接入真实用户表；当前返回占位数据
async fn profile(State(_state): State<AppState>) -> Json<ApiResponse<Profile>> {
    tracing::info!(event = "user.profile.viewed", user_id = 1);
    ApiResponse::ok(Profile { user_id: 1, nickname: "traveler".into() })
}

async fn register(State(_state): State<AppState>) -> Json<ApiResponse<Profile>> {
    tracing::info!(event = "user.register", user_id = 2, method = "placeholder");
    ApiResponse::ok(Profile { user_id: 2, nickname: "new-user".into() })
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
    let state = AppState { db: connect_db().await, cache: connect_cache().await };

    let jwt = JwtAuthLayer::new(jwt_secret()).expect("valid jwt secret");

    // 业务路由：完整中间件链，执行顺序（外层 → 内层）：
    //   CircuitBreaker → Security → Auth(JWT) → RateLimit
    // e-cat 中间件的 Error 非 Infallible，需 map_err 归一以满足 axum Router::layer
    // 约束；RateLimit（Redis 分布式）置于最内层（认证通过后计数）。
    // 注：tower 先添加的层在外层，且 map_err 内部也是 layer()（新层在内），
    // 故 map_err 声明在目标层之前才能包住它的 error。
    let api = Router::new()
        .route("/api/v1/user/profile", get(profile))
        .route("/api/v1/user/register", post(register))
        .layer(
            ServiceBuilder::new()
                .map_err(no_error)
                .layer(CircuitBreakerLayer::new())
                .map_err(no_error)
                .layer(SecurityLayer::new())
                .map_err(no_error)
                .layer(jwt)
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
