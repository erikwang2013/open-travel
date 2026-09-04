// open-travel flight-service：航班查询/比价（P4-02）
//
// 端口约定：user 8001 / booking 8002 / admin 8003 / search 8004 / line 8005 / order 8006 / 本服务 8007。
//
// 本服务只做查询（下单在 order-service P4-07）。查询链路（与 line 一致）：
//   1. search：Redis 缓存 travel:flights:{from}:{to}:{date}:{cabin}（TTL 60s），未命中回源
//   2. 详情：不缓存——余票随 order-service 预占实时扣减，必须读实时值
//   3. 从库优先、失败/为空回退主库
//
// sqlx Any 坑（同 line/booking）：DATETIME/TINYINT/CHAR(3) 列一律 CAST AS CHAR 保证字符串，
// 数值解析用 col_u64（字符串 parse 兜底）；显式列名不 SELECT *。
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
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;

const PORT: &str = "0.0.0.0:8007";
const SEARCH_CACHE_TTL: u64 = 60; // 余票不实时进缓存，比价结果 60s 新鲜度足够

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
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
    pub(crate) date: Option<String>,
    pub(crate) cabin: Option<String>,
    pub(crate) page: Option<u64>,
    pub(crate) page_size: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct FlightRow {
    pub(crate) id: u64,
    pub(crate) airline: String,
    pub(crate) flight_no: String,
    pub(crate) from_code: String,
    pub(crate) to_code: String,
    pub(crate) depart_at: String,
    pub(crate) arrive_at: String,
    pub(crate) cabin: u64,
    pub(crate) price_cents: u64,
    pub(crate) seats_left: u64,
    pub(crate) sold_out: bool,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SearchData {
    pub(crate) total: u64,
    pub(crate) page: u64,
    pub(crate) page_size: u64,
    pub(crate) items: Vec<FlightRow>,
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

/// 显式列名；DATETIME/TINYINT/CHAR(3) 列 CAST AS CHAR（sqlx Any 驱动限制）。
const FLIGHT_COLS: &str = "id, airline, flight_no, \
    CAST(from_code AS CHAR) AS from_code, CAST(to_code AS CHAR) AS to_code, \
    CAST(depart_at AS CHAR) AS depart_at, CAST(arrive_at AS CHAR) AS arrive_at, \
    CAST(cabin AS CHAR) AS cabin, price_cents, seats_left";

fn row_to_flight(row: &ecat_data::Row) -> FlightRow {
    let seats = col_u64(row, "seats_left");
    FlightRow {
        id: col_u64(row, "id"),
        airline: col_str(row, "airline"),
        flight_no: col_str(row, "flight_no"),
        from_code: col_str(row, "from_code"),
        to_code: col_str(row, "to_code"),
        depart_at: col_str(row, "depart_at"),
        arrive_at: col_str(row, "arrive_at"),
        cabin: col_u64(row, "cabin"),
        price_cents: col_u64(row, "price_cents"),
        seats_left: seats,
        sold_out: seats == 0,
    }
}

/// 校验 IATA 三字母码：3 位 ASCII 字母。
fn valid_code(s: &str) -> bool {
    s.len() == 3 && s.bytes().all(|b| b.is_ascii_alphabetic())
}

/// 校验 YYYY-MM-DD。
fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[7] == b'-'
        && b[..4].iter().all(|c| c.is_ascii_digit())
        && b[5..7].iter().all(|c| c.is_ascii_digit())
        && b[8..].iter().all(|c| c.is_ascii_digit())
}

/// 动态拼 WHERE 的公共过滤子句（count 与分页查询共用）。
fn filter_sql(parts: &mut Vec<String>, params: &mut Vec<serde_json::Value>, from: &str, to: &str, date: &Option<String>, cabin: &Option<u64>) {
    parts.push("status = 1".into());
    parts.push("from_code = ?".into());
    params.push(json!(from));
    parts.push("to_code = ?".into());
    params.push(json!(to));
    if let Some(d) = date {
        parts.push("CAST(depart_at AS DATE) = ?".into());
        params.push(json!(d));
    }
    if let Some(c) = cabin {
        parts.push("cabin = ?".into());
        params.push(json!(c));
    }
}

/// 回源查询：返回 (total, items)，查询失败时返回 (0, 空)。
async fn fetch_search(
    db: &SqlxClient,
    from: &str,
    to: &str,
    date: &Option<String>,
    cabin: &Option<u64>,
    page: u64,
    page_size: u64,
) -> (u64, Vec<FlightRow>) {
    let mut where_parts: Vec<String> = Vec::new();
    let mut params: Vec<serde_json::Value> = Vec::new();
    filter_sql(&mut where_parts, &mut params, from, to, date, cabin);
    let where_sql = where_parts.join(" AND ");

    // 1. 总数
    let total = match db
        .query_with(&format!("SELECT COUNT(*) AS c FROM travel_flights WHERE {where_sql}"), &params)
        .await
    {
        Ok(result) => result.first().map(|r| col_u64(r, "c")).unwrap_or(0),
        Err(e) => {
            tracing::warn!("flights count query failed: {e}");
            return (0, Vec::new());
        }
    };

    // 2. 分页数据（价格升序，同价按 id 稳定排序）
    let mut page_params = params.clone();
    page_params.push(json!(page_size));
    page_params.push(json!((page - 1) * page_size));
    let sql = format!(
        "SELECT {FLIGHT_COLS} FROM travel_flights WHERE {where_sql} \
         ORDER BY price_cents ASC, id ASC LIMIT ? OFFSET ?"
    );
    let items = match db.query_with(&sql, &page_params).await {
        Ok(result) => result.iter().map(row_to_flight).collect(),
        Err(e) => {
            tracing::warn!("flights query failed: {e}");
            Vec::new()
        }
    };
    (total, items)
}

async fn fetch_flight(db: &SqlxClient, id: u64) -> Option<FlightRow> {
    let result = match db
        .query_with(
            &format!("SELECT {FLIGHT_COLS} FROM travel_flights WHERE id = ? AND status = 1"),
            &[json!(id)],
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("flight query failed: {e}");
            return None;
        }
    };
    result.first().map(row_to_flight)
}

/// GET /api/v1/flights/search：航班查询/比价。
/// from/to 必填（IATA 三字母码），date/cabin 可选；结果按价格升序，分页默认 page=1 page_size=20。
pub(crate) async fn flights_search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> (StatusCode, Json<ApiResponse<SearchData>>) {
    // 1. 参数校验（边界输入）
    let Some(from_raw) = &q.from else {
        return err(StatusCode::BAD_REQUEST, 400, "from is required (IATA code)");
    };
    let Some(to_raw) = &q.to else {
        return err(StatusCode::BAD_REQUEST, 400, "to is required (IATA code)");
    };
    let from = from_raw.trim().to_uppercase();
    let to = to_raw.trim().to_uppercase();
    if !valid_code(&from) || !valid_code(&to) {
        return err(StatusCode::BAD_REQUEST, 400, "from/to must be 3-letter IATA codes");
    }
    if let Some(d) = &q.date {
        if !valid_date(d.trim()) {
            return err(StatusCode::BAD_REQUEST, 400, "date must be YYYY-MM-DD");
        }
    }
    let cabin = match &q.cabin {
        None => None,
        Some(c) => match c.trim().parse::<u64>() {
            Ok(cv) if cv <= 2 => Some(cv),
            _ => return err(StatusCode::BAD_REQUEST, 400, "cabin must be 0, 1 or 2"),
        },
    };
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    if page < 1 || page_size < 1 || page_size > 100 {
        return err(StatusCode::BAD_REQUEST, 400, "page must be >= 1, page_size must be 1..=100");
    }

    let date = q.date.as_ref().map(|d| d.trim().to_string());
    tracing::info!(event = "flight.flights.searched", from = %from, to = %to, date = ?date, cabin, page, page_size);

    // 2. Redis 缓存优先
    let cache_key = format!("travel:flights:{from}:{to}:{}:{}", date.as_deref().unwrap_or("all"), cabin.map(|c| c.to_string()).unwrap_or_else(|| "all".into()));
    if let Some(cache) = &state.cache {
        if let Ok(Some(raw)) = cache.get(&cache_key).await {
            if let Ok(data) = serde_json::from_str::<SearchData>(&String::from_utf8_lossy(&raw)) {
                return (StatusCode::OK, Json(ApiResponse { code: 0, message: "cache hit".into(), data: Some(data) }));
            }
        }
    }

    // 3. 未命中 → MySQL 回源（从库优先，失败/为空回退主库）
    let mut total = 0u64;
    let mut items: Vec<FlightRow> = Vec::new();
    for db in [state.replica.as_ref(), state.db.as_ref()].into_iter().flatten() {
        let (t, rows) = fetch_search(db, &from, &to, &date, &cabin, page, page_size).await;
        if !rows.is_empty() || t > 0 {
            total = t;
            items = rows;
            break;
        }
    }

    // 4. 回填缓存（失败仅告警），TTL 60s；空结果不缓存（余票实时变化）
    if !items.is_empty() {
        if let Some(cache) = &state.cache {
            let data = SearchData { total, page, page_size, items: items.clone() };
            let raw = serde_json::to_string(&data).unwrap_or_default();
            if let Err(e) = cache.set(&cache_key, raw.as_bytes(), Duration::from_secs(SEARCH_CACHE_TTL)).await {
                tracing::warn!("flights cache set failed: {e}");
            }
        }
    }

    // 5. 无结果返回空 items
    (StatusCode::OK, Json(ApiResponse { code: 0, message: "ok".into(), data: Some(SearchData { total, page, page_size, items }) }))
}

/// GET /api/v1/flights/{id}：详情，不缓存（余票实时）；不存在或下架返回 404。
pub(crate) async fn flight_detail(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> (StatusCode, Json<ApiResponse<FlightRow>>) {
    tracing::info!(event = "flight.flight.viewed", id);
    let mut found: Option<FlightRow> = None;
    for db in [state.replica.as_ref(), state.db.as_ref()].into_iter().flatten() {
        found = fetch_flight(db, id).await;
        if found.is_some() {
            break;
        }
    }
    match found {
        Some(f) => (StatusCode::OK, Json(ApiResponse { code: 0, message: "ok".into(), data: Some(f) })),
        None => err(StatusCode::NOT_FOUND, 404, "flight not found"),
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
        .route("/api/v1/flights/search", get(flights_search))
        .route("/api/v1/flights/{id}", get(flight_detail))
        .layer(
            ServiceBuilder::new()
                .map_err(no_error)
                .layer(CircuitBreakerLayer::new())
                .map_err(no_error)
                .layer(SecurityLayer::new())
                .map_err(no_error)
                .layer(RedisRateLimitLayer::new(
                    state.cache.clone(),
                    "flight-service",
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
                .layer(TracingLayer::new("flight-service"))
                .layer(LoggingLayer),
        );

    let http_srv = HttpServer::new(PORT).router(router);

    let mut app = App::builder()
        .name("flight-service")
        .version("v0.1.0")
        .server(http_srv)
        .on_start(|| async {
            tracing::info!("flight-service listening on {PORT}");
            Ok(())
        })
        .build()?;

    app.run().await?;
    Ok(())
}
