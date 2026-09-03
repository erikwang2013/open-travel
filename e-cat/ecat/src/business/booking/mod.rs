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
// 限流为 ecat::business::shared::RedisRateLimitLayer（Redis 分布式固定窗口，fail-open）；
// Security/CircuitBreaker 以 map_err 归一为 Infallible。
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ecat::App;
use ecat_auth::JwtAuthLayer;
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
use ecat::business::shared::{connect_primary, connect_replica, init_id_gen, jwt_secret, no_error, RedisRateLimitLayer};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;

mod reviews;

const PORT: &str = "0.0.0.0:8002";

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: Option<Arc<SqlxClient>>,
    // 从库连接池（只读路径优先，失败/为空回退主库）
    pub(crate) replica: Option<Arc<SqlxClient>>,
    pub(crate) cache: Option<Arc<RedisCache>>,
}

#[derive(Serialize)]
pub(crate) struct ApiResponse<T: Serialize> {
    pub(crate) code: u32,
    pub(crate) message: String,
    pub(crate) data: Option<T>,
}

#[derive(Deserialize)]
pub(crate) struct RegionQuery {
    #[serde(default)]
    pub(crate) region_id: u64,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct DestRow {
    id: u64,
    region_id: u64,
    name_en: String,
    name_zh: String,
}

#[derive(Deserialize)]
pub(crate) struct AttractionsQuery {
    pub(crate) destination_id: Option<u64>,
    #[serde(default)]
    pub(crate) lang: String,
}

#[derive(Deserialize)]
pub(crate) struct LangQuery {
    #[serde(default)]
    pub(crate) lang: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct AttractionRow {
    pub(crate) id: u64,
    pub(crate) destination_id: u64,
    pub(crate) name: String,
    pub(crate) price_cents: u64,
    pub(crate) open_hours: String,
    pub(crate) rating_avg: f64,
    pub(crate) cover_url: String,
}

#[derive(Serialize)]
pub(crate) struct AttractionDetail {
    pub(crate) id: u64,
    pub(crate) destination_id: u64,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) price_cents: u64,
    pub(crate) open_hours: String,
    pub(crate) rating_avg: f64,
    pub(crate) cover_url: String,
    pub(crate) reviews: Vec<serde_json::Value>,
}

async fn health() -> &'static str {
    "OK"
}

pub(crate) async fn ready(State(state): State<AppState>) -> Json<ApiResponse<bool>> {
    // 双源就绪判定：主库 + 从库 + 缓存均连上才 ready；
    // 任一缺失不阻塞服务启动（仅报告降级状态）
    let ready = state.db.is_some() && state.replica.is_some() && state.cache.is_some();
    Json(ApiResponse { code: 0, message: "ready".into(), data: Some(ready) })
}

/// 从指定连接池查询目的地；失败返回空（由调用方决定回退）。
async fn fetch_destinations(db: &SqlxClient, region_id: u64) -> Vec<DestRow> {
    match db
        .query_with(
            "SELECT id, region_id, name_en, name_zh FROM travel_destinations WHERE region_id = ? AND status = 1 ORDER BY sort_order ASC, id ASC",
            &[json!(region_id)],
        )
        .await
    {
        Ok(result) => {
            let mut rows = Vec::with_capacity(result.len());
            for row in result {
                let id = row.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                let rid = row.get("region_id").and_then(|v| v.as_u64()).unwrap_or(region_id);
                let name_en = row.get("name_en").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let name_zh = row.get("name_zh").and_then(|v| v.as_str()).unwrap_or("").to_string();
                rows.push(DestRow { id, region_id: rid, name_en, name_zh });
            }
            rows
        }
        Err(e) => {
            tracing::warn!("db query failed: {e}");
            Vec::new()
        }
    }
}

pub(crate) fn err<T: Serialize>(
    status: StatusCode,
    code: u32,
    message: &str,
) -> (StatusCode, Json<ApiResponse<T>>) {
    (status, Json(ApiResponse { code, message: message.into(), data: None }))
}

fn norm_lang(lang: &str) -> String {
    let l = lang.trim().to_lowercase();
    if l.is_empty() { "en".into() } else { l }
}

pub(crate) fn col_str(row: &ecat_data::Row, col: &str) -> String {
    row.get(col).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

pub(crate) fn col_u64(row: &ecat_data::Row, col: &str) -> u64 {
    row.get(col)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0)
}

pub(crate) fn col_f64(row: &ecat_data::Row, col: &str) -> f64 {
    row.get(col)
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_u64().map(|n| n as f64))
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(0.0)
}

/// name 取 name_{lang} 列，该语种为空/无该列时回退 name_en。
fn pick_name(row: &ecat_data::Row, lang: &str) -> String {
    let v = row.get(&format!("name_{lang}")).and_then(|v| v.as_str()).unwrap_or("");
    if v.is_empty() {
        col_str(row, "name_en")
    } else {
        v.to_string()
    }
}

/// description 为 JSON（键为语言代码），按 lang 取键，缺失/为空回退 en。
/// 注：sqlx Any 将 MySQL JSON 列（BINARY 标志）按 Blob 解码并 base64 编码返回，
/// 故先尝试 base64 解码再解析 JSON；非 base64 文本直接解析。
fn pick_desc(row: &ecat_data::Row, lang: &str) -> String {
    use base64::Engine as _;
    let raw = match row.get("description") {
        Some(v) if v.is_string() => v.as_str().unwrap_or("").to_string(),
        _ => return String::new(),
    };
    let take = |map: &serde_json::Map<String, serde_json::Value>| {
        map.get(lang)
            .or_else(|| map.get("en"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("")
            .to_string()
    };
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_else(|| raw.clone());
    serde_json::from_str::<serde_json::Value>(&decoded)
        .ok()
        .and_then(|j| j.as_object().map(take))
        .unwrap_or_default()
}

/// 按目的地查上架景区；查询失败返回空（由调用方决定回退）。
/// 注意：不能用 SELECT * —— sqlx Any 驱动不支持 MySQL JSON/NewDecimal 类型列
/// （description/rating_avg），故显式列名 + CAST 规避。
const ATTR_COLS: &str = "id, destination_id, price_cents, open_hours, \
    CAST(rating_avg AS CHAR) AS rating_avg, cover_url, \
    name_en, name_zh, name_ja, name_ko, name_ru, name_de, name_fr, name_es, \
    name_pt, name_hi, name_ar, name_bn, name_id";

async fn fetch_attractions(db: &SqlxClient, destination_id: u64, lang: &str) -> Vec<AttractionRow> {
    match db
        .query_with(
            &format!("SELECT {ATTR_COLS} FROM travel_attractions WHERE destination_id = ? AND status = 1 ORDER BY id"),
            &[json!(destination_id)],
        )
        .await
    {
        Ok(result) => result
            .iter()
            .map(|row| AttractionRow {
                id: col_u64(row, "id"),
                destination_id: col_u64(row, "destination_id"),
                name: pick_name(row, lang),
                price_cents: col_u64(row, "price_cents"),
                open_hours: col_str(row, "open_hours"),
                rating_avg: col_f64(row, "rating_avg"),
                cover_url: col_str(row, "cover_url"),
            })
            .collect(),
        Err(e) => {
            tracing::warn!("attractions query failed: {e}");
            Vec::new()
        }
    }
}

async fn fetch_attraction(db: &SqlxClient, id: u64, lang: &str) -> Option<AttractionDetail> {
    let result = match db
        .query_with(
            &format!("SELECT {ATTR_COLS}, CAST(description AS CHAR) AS description FROM travel_attractions WHERE id = ? AND status = 1"),
            &[json!(id)],
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("attraction query failed: {e}");
            return None;
        }
    };
    let row = result.first()?;
    let mut detail = AttractionDetail {
        id: col_u64(row, "id"),
        destination_id: col_u64(row, "destination_id"),
        name: pick_name(row, lang),
        description: pick_desc(row, lang),
        price_cents: col_u64(row, "price_cents"),
        open_hours: col_str(row, "open_hours"),
        rating_avg: col_f64(row, "rating_avg"),
        cover_url: col_str(row, "cover_url"),
        reviews: Vec::new(),
    };
    // P5-01：详情返回真实评价（最近 20 条），均分读时 AVG 聚合覆盖表内缓存值
    detail.reviews = reviews::fetch_reviews(db, id, reviews::DETAIL_REVIEW_LIMIT, 0)
        .await
        .into_iter()
        .map(|r| serde_json::to_value(r).unwrap_or_default())
        .collect();
    if let Some(avg) = reviews::fetch_rating_avg(db, id).await {
        detail.rating_avg = avg;
    }
    Some(detail)
}

pub(crate) async fn attractions_list(
    State(state): State<AppState>,
    Query(q): Query<AttractionsQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<AttractionRow>>>) {
    let Some(destination_id) = q.destination_id else {
        return err(StatusCode::BAD_REQUEST, 400, "destination_id is required");
    };
    let lang = norm_lang(&q.lang);
    tracing::info!(event = "booking.attractions.listed", destination_id, lang = %lang);
    let cache_key = format!("travel:attractions:{destination_id}:{lang}");

    // 1. Redis 缓存优先
    if let Some(cache) = &state.cache {
        if let Ok(Some(raw)) = cache.get(&cache_key).await {
            if let Ok(rows) = serde_json::from_str::<Vec<AttractionRow>>(&String::from_utf8_lossy(&raw)) {
                return (StatusCode::OK, Json(ApiResponse { code: 0, message: "cache hit".into(), data: Some(rows) }));
            }
        }
    }

    // 2. 未命中 → MySQL 回源（从库优先，失败/为空回退主库）
    let mut rows: Vec<AttractionRow> = Vec::new();
    for db in [state.replica.as_ref(), state.db.as_ref()].into_iter().flatten() {
        rows = fetch_attractions(db, destination_id, &lang).await;
        if !rows.is_empty() {
            break;
        }
    }

    // 3. 回填缓存（失败仅告警，不影响响应），TTL 5 分钟
    if !rows.is_empty() {
        if let Some(cache) = &state.cache {
            let raw = serde_json::to_string(&rows).unwrap_or_default();
            if let Err(e) = cache.set(&cache_key, raw.as_bytes(), Duration::from_secs(300)).await {
                tracing::warn!("attractions cache set failed: {e}");
            }
        }
    }

    // 4. 无数据返回空数组
    (StatusCode::OK, Json(ApiResponse { code: 0, message: "ok".into(), data: Some(rows) }))
}

pub(crate) async fn attraction_detail(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Query(q): Query<LangQuery>,
) -> (StatusCode, Json<ApiResponse<AttractionDetail>>) {
    let lang = norm_lang(&q.lang);
    tracing::info!(event = "booking.attraction.viewed", id, lang = %lang);
    let mut found: Option<AttractionDetail> = None;
    for db in [state.replica.as_ref(), state.db.as_ref()].into_iter().flatten() {
        found = fetch_attraction(db, id, &lang).await;
        if found.is_some() {
            break;
        }
    }
    match found {
        Some(a) => (StatusCode::OK, Json(ApiResponse { code: 0, message: "ok".into(), data: Some(a) })),
        None => err(StatusCode::NOT_FOUND, 404, "attraction not found"),
    }
}

pub(crate) async fn available_dates(
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
    //    读写分离：从库优先，查询失败或为空时回退主库
    let mut rows: Vec<DestRow> = Vec::new();
    for db in [state.replica.as_ref(), state.db.as_ref()].into_iter().flatten() {
        rows = fetch_destinations(db, q.region_id).await;
        if !rows.is_empty() {
            break;
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
            id: q.region_id,
            region_id: q.region_id,
            name_en: "placeholder-destination".into(),
            name_zh: "占位目的地".into(),
        });
    }
    Json(ApiResponse { code: 0, message: "ok".into(), data: Some(rows) })
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
    init_id_gen();
    let state = AppState {
        db: connect_primary().await,
        replica: connect_replica().await,
        cache: connect_cache().await,
    };

    // 业务路由：完整中间件链，执行顺序（外层 → 内层）：
    //   ApiVersion → CircuitBreaker → Security → RateLimit
    // dates/attractions 为公开接口（无鉴权），限流保留防止滥用；
    // POST /api/v1/reviews 挂 JWT（Auth 层内），GET /api/v1/reviews 公开。
    // e-cat 中间件的 Error 非 Infallible，需 map_err 归一以满足 axum Router::layer
    // 约束；RateLimit（Redis 分布式）对所有请求计数。
    // 注：tower 先添加的层在外层，且 map_err 内部也是 layer()（新层在内），
    // 故 map_err 声明在目标层之前才能包住它的 error。
    let jwt = JwtAuthLayer::new(jwt_secret()).expect("valid jwt secret");
    let api = Router::new()
        .route("/api/v1/booking/dates", get(available_dates))
        .route("/api/v1/booking/attractions", get(attractions_list))
        .route("/api/v1/booking/attractions/{id}", get(attraction_detail))
        .route("/api/v1/reviews", get(reviews::list_reviews))
        .merge(
            Router::new()
                .route("/api/v1/reviews", post(reviews::create_review))
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
