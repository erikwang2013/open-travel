// open-travel line-service：线路列表/详情（P3-05）+ 出发日历与余位（P3-06）
//
// 端口约定：user 8001 / booking 8002 / admin 8003 / search 8004 / 本服务 8005 / order 8006。
// 网关已配置 /api/lines/ → ecat-line:8005，接口需 X-Api-Version: v1（ApiVersionLayer）。
//
// 查询链路（与 booking 一致）：
//   1. 列表：Redis 缓存 travel:lines:{destination_id}:{lang}（TTL 300s），未命中回源
//   2. 详情：不缓存（含 itinerary JSON），从库优先、失败/为空回退主库
//   3. 日历：不缓存——余位随 order-service 预占实时扣减（travel_line_dates.seats_left），
//      必须读实时值；按 depart_date 升序，余位 ≤0 标记 sold_out
//
// sqlx Any 坑（同 booking）：TEXT/JSON 列（itinerary）按 Blob 解码后 base64 返回，
// 先 base64 解码再解析 JSON；DATE 列 CAST 为 CHAR 保证字符串；显式列名不 SELECT *。
// title 回退链：title_{lang} → title_zh → title_en（in-memory 取列，缺列不报错）。
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
use shared::{connect_primary, connect_replica, no_error, RedisRateLimitLayer};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;

const PORT: &str = "0.0.0.0:8005";

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
pub(crate) struct LinesQuery {
    // 目的地过滤可选：缺省返回全部上架线路
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
pub(crate) struct LineRow {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) destination_id: u64,
    pub(crate) days: u64,
    pub(crate) price_cents: u64,
    pub(crate) max_pax: u64,
    pub(crate) cover_url: String,
}

#[derive(Serialize)]
pub(crate) struct ItineraryDay {
    pub(crate) day: u64,
    pub(crate) title: String,
    pub(crate) description: String,
}

#[derive(Serialize)]
pub(crate) struct LineDetail {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) destination_id: u64,
    pub(crate) days: u64,
    pub(crate) price_cents: u64,
    pub(crate) max_pax: u64,
    pub(crate) itinerary: Vec<ItineraryDay>,
    pub(crate) cover_url: String,
}

#[derive(Serialize)]
pub(crate) struct DepartureDate {
    pub(crate) id: u64,
    pub(crate) date: String,
    pub(crate) price_cents: u64,
    pub(crate) seats_left: u64,
    pub(crate) sold_out: bool,
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

fn norm_lang(lang: &str) -> String {
    let l = lang.trim().to_lowercase();
    if l.is_empty() { "en".into() } else { l }
}

fn col_str(row: &ecat_data::Row, col: &str) -> String {
    row.get(col).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn col_u64(row: &ecat_data::Row, col: &str) -> u64 {
    row.get(col)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0)
}

/// title 取 title_{lang} 列，为空/缺列回退 title_zh，再回退 title_en。
fn pick_title(row: &ecat_data::Row, lang: &str) -> String {
    let v = row.get(&format!("title_{lang}")).and_then(|v| v.as_str()).unwrap_or("");
    if v.is_empty() {
        let zh = col_str(row, "title_zh");
        if zh.is_empty() { col_str(row, "title_en") } else { zh }
    } else {
        v.to_string()
    }
}

/// itinerary 为 TEXT（JSON {"days":[{day,title_en,title_zh,title_ja,description}]}）。
/// sqlx Any 将 MySQL TEXT/JSON 列按 Blob 解码并 base64 返回，故先 base64 解码再解析；
/// 非 base64 文本直接解析（与 booking 的 pick_desc 同一防御策略）。
fn parse_itinerary(row: &ecat_data::Row, lang: &str) -> Vec<ItineraryDay> {
    use base64::Engine as _;
    let raw = match row.get("itinerary") {
        Some(v) if v.is_string() => v.as_str().unwrap_or("").to_string(),
        _ => return Vec::new(),
    };
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_else(|| raw.clone());
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&decoded) else {
        return Vec::new();
    };
    let Some(days) = v.get("days").and_then(|d| d.as_array()) else {
        return Vec::new();
    };
    let take = |map: &serde_json::Map<String, serde_json::Value>, key: &str| {
        map.get(key).and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or("").to_string()
    };
    days.iter()
        .map(|d| {
            let mut day_val = ItineraryDay { day: 0, title: String::new(), description: String::new() };
            let Some(map) = d.as_object() else { return day_val };
            day_val.day = map.get("day").and_then(|v| v.as_u64()).unwrap_or(0);
            let t = take(map, &format!("title_{lang}"));
            day_val.title = if t.is_empty() {
                let zh = take(map, "title_zh");
                if zh.is_empty() { take(map, "title_en") } else { zh }
            } else {
                t
            };
            day_val.description = take(map, "description");
            day_val
        })
        .collect()
}

/// 显式列名（title_* 全部列出供 in-memory 回退），不用 SELECT *。
const LINE_COLS: &str = "id, destination_id, days, price_cents, max_pax, cover_url, \
    title_en, title_zh, title_ja, title_ko, title_ru";

async fn fetch_lines(db: &SqlxClient, destination_id: Option<u64>, lang: &str) -> Vec<LineRow> {
    let (sql, params): (String, Vec<serde_json::Value>) = match destination_id {
        Some(did) => (
            format!("SELECT {LINE_COLS} FROM travel_lines WHERE status = 1 AND destination_id = ? ORDER BY id"),
            vec![json!(did)],
        ),
        None => (
            format!("SELECT {LINE_COLS} FROM travel_lines WHERE status = 1 ORDER BY id"),
            Vec::new(),
        ),
    };
    match db.query_with(&sql, &params).await {
        Ok(result) => result
            .iter()
            .map(|row| LineRow {
                id: col_u64(row, "id"),
                title: pick_title(row, lang),
                destination_id: col_u64(row, "destination_id"),
                days: col_u64(row, "days"),
                price_cents: col_u64(row, "price_cents"),
                max_pax: col_u64(row, "max_pax"),
                cover_url: col_str(row, "cover_url"),
            })
            .collect(),
        Err(e) => {
            tracing::warn!("lines query failed: {e}");
            Vec::new()
        }
    }
}

async fn fetch_line(db: &SqlxClient, id: u64, lang: &str) -> Option<LineDetail> {
    let result = match db
        .query_with(
            &format!("SELECT {LINE_COLS}, CAST(itinerary AS CHAR) AS itinerary FROM travel_lines WHERE id = ? AND status = 1"),
            &[json!(id)],
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("line query failed: {e}");
            return None;
        }
    };
    let row = result.first()?;
    Some(LineDetail {
        id: col_u64(row, "id"),
        title: pick_title(row, lang),
        destination_id: col_u64(row, "destination_id"),
        days: col_u64(row, "days"),
        price_cents: col_u64(row, "price_cents"),
        max_pax: col_u64(row, "max_pax"),
        itinerary: parse_itinerary(row, lang),
        cover_url: col_str(row, "cover_url"),
    })
}

/// 出发日历：未来班期（depart_date >= 今天）按日期升序，余位实时读取。
async fn fetch_dates(db: &SqlxClient, line_id: u64) -> Vec<DepartureDate> {
    match db
        .query_with(
            "SELECT id, CAST(depart_date AS CHAR) AS depart_date, price_cents, seats_left \
             FROM travel_line_dates WHERE line_id = ? AND status = 1 \
             AND depart_date >= CURDATE() ORDER BY depart_date",
            &[json!(line_id)],
        )
        .await
    {
        Ok(result) => result
            .iter()
            .map(|row| {
                let seats = col_u64(row, "seats_left");
                DepartureDate {
                    id: col_u64(row, "id"),
                    date: col_str(row, "depart_date"),
                    price_cents: col_u64(row, "price_cents"),
                    seats_left: seats,
                    sold_out: seats == 0,
                }
            })
            .collect(),
        Err(e) => {
            tracing::warn!("line dates query failed: {e}");
            Vec::new()
        }
    }
}

pub(crate) async fn lines_list(
    State(state): State<AppState>,
    Query(q): Query<LinesQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<LineRow>>>) {
    let lang = norm_lang(&q.lang);
    let did = q.destination_id;
    tracing::info!(event = "line.lines.listed", destination_id = did, lang = %lang);
    let cache_key = format!("travel:lines:{}:{lang}", did.map(|d| d.to_string()).unwrap_or_else(|| "all".into()));

    // 1. Redis 缓存优先
    if let Some(cache) = &state.cache {
        if let Ok(Some(raw)) = cache.get(&cache_key).await {
            if let Ok(rows) = serde_json::from_str::<Vec<LineRow>>(&String::from_utf8_lossy(&raw)) {
                return (StatusCode::OK, Json(ApiResponse { code: 0, message: "cache hit".into(), data: Some(rows) }));
            }
        }
    }

    // 2. 未命中 → MySQL 回源（从库优先，失败/为空回退主库）
    let mut rows: Vec<LineRow> = Vec::new();
    for db in [state.replica.as_ref(), state.db.as_ref()].into_iter().flatten() {
        rows = fetch_lines(db, did, &lang).await;
        if !rows.is_empty() {
            break;
        }
    }

    // 3. 回填缓存（失败仅告警，不影响响应），TTL 5 分钟
    if !rows.is_empty() {
        if let Some(cache) = &state.cache {
            let raw = serde_json::to_string(&rows).unwrap_or_default();
            if let Err(e) = cache.set(&cache_key, raw.as_bytes(), Duration::from_secs(300)).await {
                tracing::warn!("lines cache set failed: {e}");
            }
        }
    }

    // 4. 无数据返回空数组
    (StatusCode::OK, Json(ApiResponse { code: 0, message: "ok".into(), data: Some(rows) }))
}

pub(crate) async fn line_detail(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Query(q): Query<LangQuery>,
) -> (StatusCode, Json<ApiResponse<LineDetail>>) {
    let lang = norm_lang(&q.lang);
    tracing::info!(event = "line.line.viewed", id, lang = %lang);
    let mut found: Option<LineDetail> = None;
    for db in [state.replica.as_ref(), state.db.as_ref()].into_iter().flatten() {
        found = fetch_line(db, id, &lang).await;
        if found.is_some() {
            break;
        }
    }
    match found {
        Some(l) => (StatusCode::OK, Json(ApiResponse { code: 0, message: "ok".into(), data: Some(l) })),
        None => err(StatusCode::NOT_FOUND, 404, "line not found"),
    }
}

/// 出发日历：余位实时（order-service 原子扣减 travel_line_dates.seats_left），不缓存。
pub(crate) async fn line_dates(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Query(_q): Query<LangQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<DepartureDate>>>) {
    tracing::info!(event = "line.dates.viewed", id);
    let mut rows: Vec<DepartureDate> = Vec::new();
    for db in [state.replica.as_ref(), state.db.as_ref()].into_iter().flatten() {
        rows = fetch_dates(db, id).await;
        if !rows.is_empty() {
            break;
        }
    }
    (StatusCode::OK, Json(ApiResponse { code: 0, message: "ok".into(), data: Some(rows) }))
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
    let state = AppState {
        db: connect_primary().await,
        replica: connect_replica().await,
        cache: connect_cache().await,
    };

    // 业务路由：完整中间件链，执行顺序（外层 → 内层）：
    //   ApiVersion → CircuitBreaker → Security → RateLimit
    // 公开接口无 JWT；限流保留防止滥用。
    let api = Router::new()
        .route("/api/lines", get(lines_list))
        .route("/api/lines/{id}", get(line_detail))
        .route("/api/lines/{id}/dates", get(line_dates))
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
                    "line-service",
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
                .layer(TracingLayer::new("line-service"))
                .layer(LoggingLayer),
        );

    let http_srv = HttpServer::new(PORT).router(router);

    let mut app = App::builder()
        .name("line-service")
        .version("v0.1.0")
        .server(http_srv)
        .on_start(|| async {
            tracing::info!("line-service listening on {PORT}");
            Ok(())
        })
        .build()?;

    app.run().await?;
    Ok(())
}
