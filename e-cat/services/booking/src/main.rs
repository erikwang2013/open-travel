// open-travel booking-service：可预订日期查询（Phase 3 安全加固后）
//
// 端口约定：user 用 8001，本服务用 8002（HTTP 默认 :8000，本地同时启动时避免冲突）。
//
// 查询链路（规划 3.3，最小实现）：
//   1. Redis 查缓存 hot_destinations:{region_id}（TTL 5 分钟），命中直接返回
//   2. 未命中 → MySQL 查 travel_destinations 表
//   3. 回填缓存（失败不影响响应）
//   4. 数据源缺失时返回占位数据（Phase 2 替换为真实链路）
//
// region_id 经 Query<T> 反序列化为 u64，format 进 SQL 无注入风险。
// 注：e-cat 的 RecoveryLayer 与 axum Router::layer 不兼容（Error=Box<dyn Error>
// 不满足 Into<Infallible>），故省略（axum 自带 panic 捕获）；
// 限流为 shared::RedisRateLimitLayer（Redis 分布式固定窗口，fail-open）；
// Security/CircuitBreaker 以 map_err 归一为 Infallible。
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use ecat::App;
use ecat_circuit_breaker::CircuitBreakerLayer;
use ecat_data::Cache;
use ecat_data::RdbmsClient;
use ecat_data_redis::RedisCache;
use ecat_data_sqlx::SqlxClient;
use ecat_middleware::LoggingLayer;
use ecat_security::SecurityLayer;
use ecat_tracing::TracingLayer;
use ecat_transport_http::HttpServer;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::{no_error, RedisRateLimitLayer};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;

const PORT: &str = "0.0.0.0:8002";

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

#[derive(Deserialize)]
struct RegionQuery {
    #[serde(default)]
    region_id: u64,
}

#[derive(Serialize, Deserialize)]
struct DestRow {
    region_id: u64,
    name_en: String,
    name_zh: String,
}

async fn health() -> &'static str {
    "OK"
}

async fn ready(State(state): State<AppState>) -> Json<ApiResponse<bool>> {
    let ready = state.db.is_some() && state.cache.is_some();
    Json(ApiResponse { code: 0, message: "ready".into(), data: Some(ready) })
}

async fn available_dates(
    State(state): State<AppState>,
    Query(q): Query<RegionQuery>,
) -> Json<ApiResponse<Vec<DestRow>>> {
    tracing::info!(event = "booking.dates.viewed", region_id = q.region_id);
    let cache_key = format!("hot_destinations:{}", q.region_id);

    // 1. Redis 缓存优先
    if let Some(cache) = &state.cache {
        if let Ok(Some(raw)) = cache.get(&cache_key).await {
            if let Ok(rows) = serde_json::from_str::<Vec<DestRow>>(&String::from_utf8_lossy(&raw)) {
                return Json(ApiResponse { code: 0, message: "cache hit".into(), data: Some(rows) });
            }
        }
    }

    // 2. 未命中 → MySQL 回源（travel_destinations 表，参数化查询防注入）
    let mut rows: Vec<DestRow> = Vec::new();
    if let Some(db) = &state.db {
        match db
            .query_with(
                "SELECT region_id, name_en, name_zh FROM travel_destinations WHERE region_id = ?",
                &[json!(q.region_id)],
            )
            .await
        {
            Ok(result) => {
                for row in result {
                    let region_id = row.get("region_id").and_then(|v| v.as_u64()).unwrap_or(q.region_id);
                    let name_en = row.get("name_en").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let name_zh = row.get("name_zh").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    rows.push(DestRow { region_id, name_en, name_zh });
                }
            }
            Err(e) => tracing::warn!("db query failed: {e}"),
        }
    }

    // 3. 回填缓存（失败仅告警，不影响响应）
    if !rows.is_empty() {
        if let Some(cache) = &state.cache {
            let raw = serde_json::to_string(&rows).unwrap_or_default();
            if let Err(e) = cache.set(&cache_key, raw.as_bytes(), Duration::from_secs(300)).await {
                tracing::warn!("cache set failed: {e}");
            }
        }
    }

    // 4. 占位数据兜底，保证接口在无数据源环境可响应
    if rows.is_empty() {
        rows.push(DestRow {
            region_id: q.region_id,
            name_en: "placeholder-destination".into(),
            name_zh: "占位目的地".into(),
        });
    }
    Json(ApiResponse { code: 0, message: "ok".into(), data: Some(rows) })
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

    // 业务路由：完整中间件链，执行顺序（外层 → 内层）：
    //   ApiVersion → CircuitBreaker → Security → RateLimit
    // API 版本经 X-Api-Version header 传递（URL 无版本前缀），缺失/非法直接 400。
    // dates 为公开接口（热门目的地展示，无鉴权），限流保留防止滥用。
    // e-cat 中间件的 Error 非 Infallible，需 map_err 归一以满足 axum Router::layer
    // 约束；RateLimit（Redis 分布式）对所有请求计数。
    // 注：tower 先添加的层在外层，且 map_err 内部也是 layer()（新层在内），
    // 故 map_err 声明在目标层之前才能包住它的 error。
    let api = Router::new()
        .route("/api/booking/dates", get(available_dates))
        .layer(
            ServiceBuilder::new()
                .layer(shared::ApiVersionLayer)
                .map_err(no_error)
                .layer(CircuitBreakerLayer::new())
                .map_err(no_error)
                .layer(SecurityLayer::new())
                .map_err(no_error)
                .layer(RedisRateLimitLayer::new(
                    state.cache.clone(),
                    "booking-service",
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
                .layer(TracingLayer::new("booking-service"))
                .layer(LoggingLayer),
        );

    let http_srv = HttpServer::new(PORT).router(router);

    let mut app = App::builder()
        .name("booking-service")
        .version("v0.1.0")
        .server(http_srv)
        .on_start(|| async {
            tracing::info!("booking-service listening on {PORT}");
            Ok(())
        })
        .build()?;

    app.run().await?;
    Ok(())
}
