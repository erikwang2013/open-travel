// open-travel hotel-service：酒店搜索/详情（P4-04）
//
// 端口约定：user 8001 / booking 8002 / admin 8003 / search 8004 / line 8005 /
// order 8006 / payment 8007 / 本服务 8008。网关已配置 /api/hotels/ → ecat-hotel:8008。
//
// 只做查询（下单在 order-service P4-07）；房价本期固定，check_in/check_out
// 仅展示用、不参与计价。search 结果 Redis 缓存 travel:hotels:{city}:{page}
// （TTL 60s），详情不缓存（库存实时）；无 Redis 降级直查。
//
// sqlx Any 坑（同 line/booking）：TINYINT 列（star/breakfast/status）与
// DECIMAL 列（latitude/longitude）必须 CAST AS CHAR，否则解码失败；
// 数值统一字符串 parse 兜底（col_u64/col_f64）。
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
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
use ecat::business::shared::{connect_primary, connect_replica, no_error, RedisRateLimitLayer};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;

const PORT: &str = "0.0.0.0:8008";

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: Option<Arc<SqlxClient>>,
    // 从库连接池（只读路径优先，失败/为空回退主库）
    pub(crate) replica: Option<Arc<SqlxClient>>,
    pub(crate) cache: Option<Arc<RedisCache>>,
}

#[derive(Serialize)]
pub(crate) struct ApiResponse<T: Serialize> {
    code: u32,
    message: String,
    data: Option<T>,
}

#[derive(Deserialize)]
pub(crate) struct SearchQuery {
    // 城市代码过滤（CHAR(3)），可选；空返回全部上架酒店
    pub(crate) city: Option<String>,
    // 入住/离店日期，本期不参与计价，仅记录展示
    pub(crate) check_in: Option<String>,
    pub(crate) check_out: Option<String>,
    pub(crate) page: Option<u64>,
    pub(crate) page_size: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct RoomRow {
    pub(crate) id: u64,
    pub(crate) room_type_en: String,
    pub(crate) room_type_zh: String,
    pub(crate) room_type_ja: String,
    pub(crate) price_cents: u64,
    pub(crate) breakfast: u64,
    pub(crate) inventory: u64,
    pub(crate) status: u64,
    // 仅内部用于按酒店分组，不进响应与缓存
    #[serde(skip)]
    pub(crate) hotel_id: u64,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct HotelRow {
    pub(crate) id: u64,
    pub(crate) name_en: String,
    pub(crate) name_zh: String,
    pub(crate) name_ja: String,
    pub(crate) city_code: String,
    pub(crate) star: u64,
    pub(crate) latitude: f64,
    pub(crate) longitude: f64,
    pub(crate) cover_url: String,
    pub(crate) rooms: Vec<RoomRow>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct HotelList {
    pub(crate) total: u64,
    pub(crate) page: u64,
    pub(crate) page_size: u64,
    pub(crate) items: Vec<HotelRow>,
}

async fn health() -> &'static str {
    "OK"
}

pub(crate) async fn ready(State(state): State<AppState>) -> Json<ApiResponse<bool>> {
    let ready = state.db.is_some() && state.replica.is_some() && state.cache.is_some();
    Json(ApiResponse { code: 0, message: "ready".into(), data: Some(ready) })
}

fn err<T: Serialize>(
    status: StatusCode,
    code: u32,
    message: &str,
) -> (StatusCode, Json<ApiResponse<T>>) {
    (status, Json(ApiResponse { code, message: message.into(), data: None }))
}

fn col_str(row: &ecat_data::Row, col: &str) -> String {
    row.get(col).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn col_u64(row: &ecat_data::Row, col: &str) -> u64 {
    row.get(col)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0)
}

/// DECIMAL 列已 CAST AS CHAR，走字符串 parse 兜底。
fn col_f64(row: &ecat_data::Row, col: &str) -> f64 {
    row.get(col)
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0.0)
}

fn row_to_hotel(row: &ecat_data::Row) -> HotelRow {
    HotelRow {
        id: col_u64(row, "id"),
        name_en: col_str(row, "name_en"),
        name_zh: col_str(row, "name_zh"),
        name_ja: col_str(row, "name_ja"),
        city_code: col_str(row, "city_code"),
        star: col_u64(row, "star"),
        latitude: col_f64(row, "latitude"),
        longitude: col_f64(row, "longitude"),
        cover_url: col_str(row, "cover_url"),
        rooms: Vec::new(),
    }
}

/// 显式列名；TINYINT/DECIMAL 列 CAST AS CHAR（sqlx Any 解码要求）。
const HOTEL_COLS: &str = "id, name_en, name_zh, name_ja, city_code, \
    CAST(star AS CHAR) AS star, CAST(latitude AS CHAR) AS latitude, \
    CAST(longitude AS CHAR) AS longitude, cover_url";

const ROOM_COLS: &str = "id, hotel_id, room_type_en, room_type_zh, room_type_ja, \
    price_cents, CAST(breakfast AS CHAR) AS breakfast, inventory, \
    CAST(status AS CHAR) AS status";

/// 房型单查询（WHERE hotel_id IN (...)）避免 N+1；仅上架房型（status = 1）。
async fn fetch_rooms(db: &SqlxClient, hotel_ids: &[u64]) -> Vec<RoomRow> {
    if hotel_ids.is_empty() {
        return Vec::new();
    }
    let placeholders = vec!["?"; hotel_ids.len()].join(",");
    let sql = format!(
        "SELECT {ROOM_COLS} FROM travel_hotel_rooms WHERE status = 1 AND hotel_id IN ({placeholders})"
    );
    let params: Vec<serde_json::Value> = hotel_ids.iter().map(|&id| json!(id)).collect();
    match db.query_with(&sql, &params).await {
        Ok(result) => result
            .iter()
            .map(|row| RoomRow {
                id: col_u64(row, "id"),
                hotel_id: col_u64(row, "hotel_id"),
                room_type_en: col_str(row, "room_type_en"),
                room_type_zh: col_str(row, "room_type_zh"),
                room_type_ja: col_str(row, "room_type_ja"),
                price_cents: col_u64(row, "price_cents"),
                breakfast: col_u64(row, "breakfast"),
                inventory: col_u64(row, "inventory"),
                status: col_u64(row, "status"),
            })
            .collect(),
        Err(e) => {
            tracing::warn!("hotel rooms query failed: {e}");
            Vec::new()
        }
    }
}

/// 搜索：总数 + 分页酒店 + 房型（同一从库/主库），star DESC, id ASC。
async fn fetch_search(db: &SqlxClient, city: &str, page: u64, page_size: u64) -> Option<HotelList> {
    let mut where_sql = String::from("WHERE status = 1");
    let mut params: Vec<serde_json::Value> = Vec::new();
    if !city.is_empty() {
        where_sql.push_str(" AND city_code = ?");
        params.push(json!(city));
    }
    let offset = (page - 1) * page_size;
    let mut limit_params = params.clone();
    limit_params.push(json!(page_size));
    limit_params.push(json!(offset));

    let result = db
        .query_with(
            &format!(
                "SELECT {HOTEL_COLS} FROM travel_hotels {where_sql} ORDER BY star DESC, id ASC LIMIT ? OFFSET ?"
            ),
            &limit_params,
        )
        .await
        .ok()?;
    let hotels: Vec<HotelRow> = result.iter().map(row_to_hotel).collect();
    let total = db
        .query_with(
            &format!("SELECT CAST(COUNT(*) AS CHAR) AS total FROM travel_hotels {where_sql}"),
            &params,
        )
        .await
        .ok()
        .and_then(|r| r.first().map(|row| col_u64(row, "total")))
        .unwrap_or(0);

    let ids: Vec<u64> = hotels.iter().map(|h| h.id).collect();
    let mut by_hotel: HashMap<u64, Vec<RoomRow>> = HashMap::new();
    for room in fetch_rooms(db, &ids).await {
        by_hotel.entry(room.hotel_id).or_default().push(room);
    }
    let items: Vec<HotelRow> = hotels
        .into_iter()
        .map(|mut h| {
            h.rooms = by_hotel.remove(&h.id).unwrap_or_default();
            h
        })
        .collect();
    Some(HotelList { total, page, page_size, items })
}

async fn fetch_hotel(db: &SqlxClient, id: u64) -> Option<HotelRow> {
    let result = db
        .query_with(
            &format!("SELECT {HOTEL_COLS} FROM travel_hotels WHERE id = ? AND status = 1"),
            &[json!(id)],
        )
        .await
        .ok()?;
    let row = result.first()?;
    let mut hotel = row_to_hotel(row);
    hotel.rooms = fetch_rooms(db, &[id]).await;
    Some(hotel)
}

pub(crate) async fn hotels_search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> (StatusCode, Json<ApiResponse<HotelList>>) {
    let city = q.city.as_deref().unwrap_or("").trim().to_uppercase();
    if city.len() > 3 {
        return err(StatusCode::BAD_REQUEST, 400, "city code must be at most 3 characters");
    }
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).max(1);
    tracing::info!(event = "hotel.search", city = %city, page, page_size,
        check_in = ?q.check_in, check_out = ?q.check_out);
    let cache_key = format!("travel:hotels:{}:{page}", if city.is_empty() { "all" } else { &city });

    // 1. Redis 缓存优先（TTL 60s）
    if let Some(cache) = &state.cache {
        if let Ok(Some(raw)) = cache.get(&cache_key).await {
            if let Ok(list) = serde_json::from_str::<HotelList>(&String::from_utf8_lossy(&raw)) {
                return (StatusCode::OK, Json(ApiResponse { code: 0, message: "cache hit".into(), data: Some(list) }));
            }
        }
    }

    // 2. 未命中 → MySQL 回源（从库优先，失败/为空回退主库）
    let mut list: Option<HotelList> = None;
    for db in [state.replica.as_ref(), state.db.as_ref()].into_iter().flatten() {
        list = fetch_search(db, &city, page, page_size).await;
        if list.as_ref().is_some_and(|l| !l.items.is_empty()) {
            break;
        }
    }

    // 3. 回填缓存（失败仅告警，不影响响应）
    if let Some(l) = &list {
        if !l.items.is_empty() {
            if let Some(cache) = &state.cache {
                let raw = serde_json::to_string(l).unwrap_or_default();
                if let Err(e) = cache.set(&cache_key, raw.as_bytes(), Duration::from_secs(60)).await {
                    tracing::warn!("hotel cache set failed: {e}");
                }
            }
        }
    }

    // 4. 无结果返回空 items（total 0）
    let data = list.unwrap_or(HotelList { total: 0, page, page_size, items: Vec::new() });
    (StatusCode::OK, Json(ApiResponse { code: 0, message: "ok".into(), data: Some(data) }))
}

pub(crate) async fn hotel_detail(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> (StatusCode, Json<ApiResponse<HotelRow>>) {
    tracing::info!(event = "hotel.viewed", id);
    let mut found: Option<HotelRow> = None;
    for db in [state.replica.as_ref(), state.db.as_ref()].into_iter().flatten() {
        found = fetch_hotel(db, id).await;
        if found.is_some() {
            break;
        }
    }
    match found {
        Some(h) => (StatusCode::OK, Json(ApiResponse { code: 0, message: "ok".into(), data: Some(h) })),
        None => err(StatusCode::NOT_FOUND, 404, "hotel not found"),
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

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = AppState {
        db: connect_primary().await,
        replica: connect_replica().await,
        cache: connect_cache().await,
    };

    // 业务路由：完整中间件链，执行顺序（外层 → 内层）：
    //   ApiVersion → CircuitBreaker → Security → RateLimit
    // 公开接口无 JWT；限流保留防止滥用。
    let api = Router::new()
        .route("/api/hotels/search", get(hotels_search))
        .route("/api/hotels/{id}", get(hotel_detail))
        .layer(
            ServiceBuilder::new()
                .layer(ecat::business::shared::ApiVersionLayer)
                .map_err(no_error)
                .layer(CircuitBreakerLayer::new())
                .map_err(no_error)
                .layer(SecurityLayer::new())
                .map_err(no_error)
                .layer(RedisRateLimitLayer::new(
                    state.cache.clone(),
                    "hotel-service",
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
                .layer(TracingLayer::new("hotel-service"))
                .layer(LoggingLayer),
        );

    let http_srv = HttpServer::new(PORT).router(router);

    let mut app = App::builder()
        .name("hotel-service")
        .version("v0.1.0")
        .server(http_srv)
        .on_start(|| async {
            tracing::info!("hotel-service listening on {PORT}");
            Ok(())
        })
        .build()?;

    app.run().await?;
    Ok(())
}
