// open-travel search-service：OpenSearch 内容索引同步（P3-01）+ 多条件检索（P3-02）
//
// 端口 8004；公开接口（无 JWT）。检索链路：
//   1. Redis 缓存（60s）命中直接返回
//   2. OpenSearch multi_match（q，name_* + description，OR 语义）+ bool 过滤
//      （destination_id term / price_cents range），目的地与景点索引各取一页后合并
//   3. OpenSearch 不可用时回退 MySQL LIKE（status=1，参数化查询，语义与 OS 对齐）
//   4. 检索成功写 travel_searches 日志（热词统计，user_id 留空：公开接口）
// 索引同步：启动 2s 后全量重推 destinations + attractions（幂等 upsert，id 为文档
// id），之后每 60s 重推；OpenSearch 不可用仅告警跳过，不阻塞服务。
// ponytail: 全量重推足够（数据量小），需要增量时按 updated_at 位点推进。
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use ecat::App;
use ecat_circuit_breaker::CircuitBreakerLayer;
use ecat_data::Cache;
use ecat_data::SearchClient;
use ecat_data::RdbmsClient;
use ecat_data_redis::RedisCache;
use ecat_data_sqlx::SqlxClient;
use ecat_data_opensearch::{OpenSearchClient, OpenSearchConfig};
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

const PORT: &str = "0.0.0.0:8004";
const PAGE_SIZE: u32 = 20;
const CACHE_TTL: Duration = Duration::from_secs(60);
const SYNC_INTERVAL: Duration = Duration::from_secs(60);

// 13 语种 name 列（与客户端 ARB 语种一致；travel_destinations 仅有 en/zh/ja 三列）
const NAME_COLS: &str = "name_en, name_zh, name_ja, name_ko, name_ar, name_es, name_fr, \
    name_de, name_pt, name_hi, name_bn, name_id, name_ru";
const NAME_FIELDS: [&str; 13] = [
    "name_en", "name_zh", "name_ja", "name_ko", "name_ar", "name_es", "name_fr",
    "name_de", "name_pt", "name_hi", "name_bn", "name_id", "name_ru",
];

// DECIMAL/TINYINT/TEXT 列必须 CAST：sqlx Any 对 DECIMAL 列 fetch 失败、
// TEXT/JSON 列返回 base64（见 desc_parts）
const DEST_SQL: &str = "SELECT id, name_en, name_zh, name_ja, cover_url, category, region_id, \
    CAST(latitude AS CHAR) AS latitude, CAST(longitude AS CHAR) AS longitude, \
    CAST(sort_order AS CHAR) AS sort_order, CAST(status AS CHAR) AS status, \
    CAST(description AS CHAR) AS description FROM travel_destinations WHERE status = 1 ORDER BY id";
const ATTR_SQL: &str = "SELECT id, destination_id, price_cents, open_hours, \
    CAST(rating_avg AS CHAR) AS rating_avg, cover_url, CAST(status AS CHAR) AS status, \
    CAST(description AS CHAR) AS description, {NAME_COLS} \
    FROM travel_attractions WHERE status = 1 ORDER BY id";

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: Option<Arc<SqlxClient>>,
    pub(crate) replica: Option<Arc<SqlxClient>>,
    pub(crate) cache: Option<Arc<RedisCache>>,
    pub(crate) os: Option<Arc<OpenSearchClient>>,
}

#[derive(Serialize)]
pub(crate) struct ApiResponse<T: Serialize> {
    code: u32,
    message: String,
    data: Option<T>,
}

#[derive(Deserialize)]
pub(crate) struct SearchQuery {
    #[serde(default)]
    pub(crate) q: String,
    #[serde(default)]
    pub(crate) destination_id: Option<u64>,
    #[serde(default)]
    pub(crate) lang: String,
    #[serde(default)]
    pub(crate) price_min: Option<u64>,
    #[serde(default)]
    pub(crate) price_max: Option<u64>,
    #[serde(default = "default_page")]
    pub(crate) page: u32,
}

fn default_page() -> u32 {
    1
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct SearchItem {
    pub(crate) id: u64,
    #[serde(rename = "type")]
    pub(crate) item_type: String,
    pub(crate) name: String,
    pub(crate) price_cents: u64,
    pub(crate) cover_url: String,
    pub(crate) description: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct SearchResult {
    pub(crate) total: u64,
    pub(crate) page: u32,
    pub(crate) page_size: u32,
    pub(crate) items: Vec<SearchItem>,
}

#[derive(Clone, Copy, PartialEq)]
enum SyncKind {
    Destination,
    Attraction,
}

async fn health() -> &'static str {
    "OK"
}

pub(crate) async fn ready(State(state): State<AppState>) -> Json<ApiResponse<bool>> {
    let ready = state.db.is_some() && state.replica.is_some() && state.cache.is_some() && state.os.is_some();
    Json(ApiResponse { code: 0, message: "ready".into(), data: Some(ready) })
}

pub(crate) fn norm_lang(lang: &str) -> String {
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

/// 只读路径：从库优先，失败/为空回退主库。
fn read_db(state: &AppState) -> Option<&SqlxClient> {
    state.replica.as_ref().map(|d| d.as_ref()).or_else(|| state.db.as_ref().map(|d| d.as_ref()))
}

/// name 取 name_{lang} 列，该语种为空/无该列时回退 name_en（目的地表仅 en/zh/ja）。
fn pick_name(row: &ecat_data::Row, lang: &str) -> String {
    let v = row.get(&format!("name_{lang}")).and_then(|v| v.as_str()).unwrap_or("");
    if v.is_empty() { col_str(row, "name_en") } else { v.to_string() }
}

/// description 为 JSON 文本（键为语言代码）。sqlx Any 将 TEXT/JSON 列按 Blob
/// base64 编码返回，先尝试 base64 解码再解析；非 base64 直接解析。
/// 返回（全语种拼接检索文本, 原始 JSON 对象按语种展示）。
fn desc_parts(raw: &str) -> (String, serde_json::Value) {
    use base64::Engine as _;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_else(|| raw.to_string());
    let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str::<serde_json::Value>(&decoded)
        .ok()
        .and_then(|j| j.as_object().cloned())
        .unwrap_or_default();
    let flat = map.values().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" ");
    (flat, serde_json::Value::Object(map))
}

/// description 按 lang 取键，缺失/为空回退 en（MySQL 回退路径展示用）。
fn pick_desc(row: &ecat_data::Row, lang: &str) -> String {
    use base64::Engine as _;
    let raw = match row.get("description") {
        Some(v) if v.is_string() => v.as_str().unwrap_or("").to_string(),
        _ => return String::new(),
    };
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_else(|| raw.clone());
    serde_json::from_str::<serde_json::Value>(&decoded)
        .ok()
        .and_then(|j| j.as_object().map(|m| {
            m.get(lang).or_else(|| m.get("en")).and_then(|v| v.as_str()).unwrap_or("").to_string()
        }))
        .unwrap_or_default()
}

fn item_from_row(row: &ecat_data::Row, item_type: &str, lang: &str) -> SearchItem {
    SearchItem {
        id: col_u64(row, "id"),
        item_type: item_type.into(),
        name: pick_name(row, lang),
        price_cents: col_u64(row, "price_cents"),
        cover_url: col_str(row, "cover_url"),
        description: pick_desc(row, lang),
    }
}

// ===== P3-01：内容索引同步 =====

async fn sync_table(db: &SqlxClient, os: &OpenSearchClient, index: &str, sql: &str, kind: SyncKind) -> (u64, u64) {
    let rows = match db.query_with(sql, &[]).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(index, "sync query failed: {e}");
            return (0, 0);
        }
    };
    let mut ok = 0;
    let mut fail = 0;
    for row in rows {
        let id = col_u64(&row, "id");
        let mut doc = serde_json::Map::new();
        doc.insert("id".into(), json!(id));
        doc.insert("status".into(), json!(col_str(&row, "status")));
        doc.insert("cover_url".into(), json!(col_str(&row, "cover_url")));
        for f in NAME_FIELDS {
            doc.insert(f.to_string(), json!(col_str(&row, f)));
        }
        let (flat, map) = desc_parts(&col_str(&row, "description"));
        doc.insert("description".into(), json!(flat));
        doc.insert("desc_map".into(), map);
        match kind {
            SyncKind::Destination => {
                doc.insert("category".into(), json!(col_str(&row, "category")));
                doc.insert("region_id".into(), json!(col_u64(&row, "region_id")));
                doc.insert("latitude".into(), json!(col_str(&row, "latitude")));
                doc.insert("longitude".into(), json!(col_str(&row, "longitude")));
                doc.insert("sort_order".into(), json!(col_str(&row, "sort_order")));
            }
            SyncKind::Attraction => {
                doc.insert("destination_id".into(), json!(col_u64(&row, "destination_id")));
                doc.insert("price_cents".into(), json!(col_u64(&row, "price_cents")));
                doc.insert("open_hours".into(), json!(col_str(&row, "open_hours")));
                doc.insert("rating_avg".into(), json!(col_str(&row, "rating_avg")));
            }
        }
        match os.index(index, &id.to_string(), &serde_json::Value::Object(doc)).await {
            Ok(()) => ok += 1,
            Err(e) => {
                tracing::warn!(index, id, "index upsert failed: {e}");
                fail += 1;
            }
        }
    }
    (ok, fail)
}

async fn sync_all(state: &AppState) {
    let Some(db) = read_db(state) else { return };
    let Some(os) = &state.os else { return };
    let started = std::time::Instant::now();
    let (d_ok, d_fail) = sync_table(db, os, "travel_destinations", DEST_SQL, SyncKind::Destination).await;
    let (a_ok, a_fail) = sync_table(db, os, "travel_attractions", &ATTR_SQL.replace("{NAME_COLS}", NAME_COLS), SyncKind::Attraction).await;
    tracing::info!(
        destinations_ok = d_ok, destinations_fail = d_fail,
        attractions_ok = a_ok, attractions_fail = a_fail,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "opensearch sync done"
    );
}

async fn sync_loop(state: AppState) {
    // 等 MySQL/OpenSearch 就绪后首次全量同步，之后按间隔重推
    tokio::time::sleep(Duration::from_secs(2)).await;
    let mut ticker = tokio::time::interval(SYNC_INTERVAL);
    loop {
        sync_all(&state).await;
        ticker.tick().await;
    }
}

// ===== P3-02：多条件检索 =====

pub(crate) fn build_os_query(q: &str, dest: Option<u64>, pmin: Option<u64>, pmax: Option<u64>, page: u32) -> serde_json::Value {
    let mut must = Vec::new();
    if q.is_empty() {
        must.push(json!({"match_all": {}}));
    } else {
        // multi_match 默认 best_fields + OR：命中任一 name_* / description 即可
        must.push(json!({
            "multi_match": {
                "query": q,
                "fields": ["name_en", "name_zh", "name_ja", "name_ko", "name_ar", "name_es",
                           "name_fr", "name_de", "name_pt", "name_hi", "name_bn", "name_id",
                           "name_ru", "description"]
            }
        }));
    }
    if let Some(d) = dest {
        must.push(json!({"term": {"destination_id": d}}));
    }
    if pmin.is_some() || pmax.is_some() {
        let mut range = serde_json::Map::new();
        if let Some(v) = pmin { range.insert("gte".into(), json!(v)); }
        if let Some(v) = pmax { range.insert("lte".into(), json!(v)); }
        // 目的地文档无 price_cents，range 不命中 → 价格过滤时自动排除目的地
        must.push(json!({"range": {"price_cents": range}}));
    }
    json!({
        "query": {"bool": {"must": must}},
        "from": (page - 1) * PAGE_SIZE,
        "size": PAGE_SIZE
    })
}

pub(crate) fn hits_total(resp: &serde_json::Value) -> u64 {
    match resp.pointer("/hits/total") {
        Some(t) if t.is_object() => t.get("value").and_then(|v| v.as_u64()).unwrap_or(0),
        Some(t) => t.as_u64().unwrap_or(0),
        None => 0,
    }
}

/// _source 转统一列表项：name_{lang} 回退 en；desc_map 按 lang 取展示文本。
fn os_item(src: &serde_json::Value, lang: &str) -> Option<SearchItem> {
    let id = src.get("id").and_then(|v| v.as_u64())?;
    let name = src.get(&format!("name_{lang}")).and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .or_else(|| src.get("name_en").and_then(|v| v.as_str())).unwrap_or("").to_string();
    let description = src.pointer("/desc_map").map(|m| m.get(lang).or_else(|| m.get("en")))
        .flatten().and_then(|v| v.as_str()).unwrap_or("").to_string();
    Some(SearchItem {
        id,
        item_type: if src.get("destination_id").is_some() { "attraction" } else { "destination" }.into(),
        name,
        price_cents: src.get("price_cents").and_then(|v| v.as_u64()).unwrap_or(0),
        cover_url: src.get("cover_url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        description,
    })
}

/// 单索引检索；OpenSearch 异常返回 None（调用方回退 MySQL）。
async fn search_os_index(
    os: &OpenSearchClient,
    index: &str,
    q: &str,
    dest: Option<u64>,
    pmin: Option<u64>,
    pmax: Option<u64>,
    page: u32,
    lang: &str,
) -> Option<(Vec<SearchItem>, u64)> {
    let query = build_os_query(q, dest, pmin, pmax, page);
    match os.search(index, &query).await {
        Ok(resp) => {
            let total = hits_total(&resp);
            let items = resp.pointer("/hits/hits").and_then(|h| h.as_array())
                .map(|arr| arr.iter().filter_map(|h| h.pointer("/_source")).filter_map(|s| os_item(s, lang)).collect())
                .unwrap_or_default();
            Some((items, total))
        }
        Err(e) => {
            tracing::warn!(index, "opensearch search failed: {e}");
            None
        }
    }
}

fn like_pattern(q: &str) -> String {
    format!("%{q}%")
}

/// 构造 name_*/description 的 OR LIKE 条件；q 为空返回空条件（全量）。
fn like_cond(q: &str, cols: &[&str]) -> (String, Vec<serde_json::Value>) {
    if q.is_empty() {
        return (String::new(), Vec::new());
    }
    let ors: Vec<String> = cols.iter().map(|c| format!("{c} LIKE ?")).collect();
    let params = cols.iter().map(|_| json!(like_pattern(q))).collect();
    (format!("AND ({})", ors.join(" OR ")), params)
}

async fn count_rows(db: &SqlxClient, sql: &str, params: &[serde_json::Value]) -> u64 {
    match db.query_with(sql, params).await {
        Ok(rows) => rows.first().and_then(|r| r.get("n")).and_then(|v| v.as_u64()).unwrap_or(0),
        Err(e) => {
            tracing::warn!("count query failed: {e}");
            0
        }
    }
}

/// MySQL 回退：目的地（仅 en/zh/ja 三语 name）。价格过滤时目的地不参与检索
/// （无价格字段，与 OpenSearch range 行为一致）。
async fn fallback_dest(db: &SqlxClient, q: &str, page: u32, lang: &str) -> (Vec<SearchItem>, u64) {
    let (cond, params) = like_cond(q, &["name_en", "name_zh", "name_ja", "description"]);
    let total = count_rows(db, &format!("SELECT COUNT(*) AS n FROM travel_destinations WHERE status = 1 {cond}"), &params).await;
    let sql = format!(
        "SELECT id, name_en, name_zh, name_ja, cover_url, CAST(description AS CHAR) AS description \
         FROM travel_destinations WHERE status = 1 {cond} ORDER BY sort_order ASC, id ASC \
         LIMIT {PAGE_SIZE} OFFSET {}",
        (page - 1) * PAGE_SIZE
    );
    match db.query_with(&sql, &params).await {
        Ok(rows) => {
            let items = rows.iter().map(|r| item_from_row(r, "destination", lang)).collect();
            (items, total)
        }
        Err(e) => {
            tracing::warn!("destination fallback query failed: {e}");
            (Vec::new(), 0)
        }
    }
}

async fn fallback_attr(
    db: &SqlxClient,
    q: &str,
    dest: Option<u64>,
    pmin: Option<u64>,
    pmax: Option<u64>,
    page: u32,
    lang: &str,
) -> (Vec<SearchItem>, u64) {
    let mut conds = Vec::new();
    let mut params = Vec::new();
    let (like, like_params) = like_cond(q, &["name_en", "name_zh", "name_ja", "name_ko", "name_ar",
        "name_es", "name_fr", "name_de", "name_pt", "name_hi", "name_bn", "name_id", "name_ru", "description"]);
    if !like.is_empty() {
        conds.push(like);
        params.extend(like_params);
    }
    if let Some(d) = dest {
        conds.push("destination_id = ?".into());
        params.push(json!(d));
    }
    if let Some(v) = pmin {
        conds.push("price_cents >= ?".into());
        params.push(json!(v));
    }
    if let Some(v) = pmax {
        conds.push("price_cents <= ?".into());
        params.push(json!(v));
    }
    let where_clause = if conds.is_empty() { String::new() } else { format!("AND {}", conds.join(" AND ")) };
    let total = count_rows(db, &format!("SELECT COUNT(*) AS n FROM travel_attractions WHERE status = 1 {where_clause}"), &params).await;
    let sql = format!(
        "SELECT id, destination_id, price_cents, cover_url, CAST(description AS CHAR) AS description, {NAME_COLS} \
         FROM travel_attractions WHERE status = 1 {where_clause} ORDER BY id ASC \
         LIMIT {PAGE_SIZE} OFFSET {}",
        (page - 1) * PAGE_SIZE
    );
    match db.query_with(&sql, &params).await {
        Ok(rows) => {
            let items = rows.iter().map(|r| item_from_row(r, "attraction", lang)).collect();
            (items, total)
        }
        Err(e) => {
            tracing::warn!("attraction fallback query failed: {e}");
            (Vec::new(), 0)
        }
    }
}

async fn fallback_search(
    db: &SqlxClient,
    q: &str,
    dest: Option<u64>,
    pmin: Option<u64>,
    pmax: Option<u64>,
    page: u32,
    lang: &str,
) -> SearchResult {
    let mut items = Vec::new();
    let mut total = 0u64;
    // 目的地表无 destination_id/price_cents 列：两种过滤下都不参与检索
    // （与 OpenSearch 行为一致：term/range 不命中缺失字段）
    if dest.is_none() && pmin.is_none() && pmax.is_none() {
        let (d_items, d_total) = fallback_dest(db, q, page, lang).await;
        items.extend(d_items);
        total += d_total;
    }
    let (a_items, a_total) = fallback_attr(db, q, dest, pmin, pmax, page, lang).await;
    items.extend(a_items);
    total += a_total;
    SearchResult { total, page, page_size: PAGE_SIZE, items }
}

/// 检索日志落库（主库写路径；失败仅告警）。user_id 留空：公开接口无鉴权。
async fn log_search(state: &AppState, keyword: &str, lang: &str, count: u64) {
    let Some(db) = &state.db else { return };
    match db.query_with(
        "INSERT INTO travel_searches (keyword, lang, result_count) VALUES (?, ?, ?)",
        &[json!(keyword), json!(lang), json!(count)],
    ).await {
        Ok(_) => {}
        Err(e) => tracing::warn!(error = %e, "search log insert failed"),
    }
}

pub(crate) async fn search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> (StatusCode, Json<ApiResponse<SearchResult>>) {
    let lang = norm_lang(&q.lang);
    let page = q.page.max(1);
    let keyword = q.q.trim().to_string();
    tracing::info!(event = "search.query", keyword = %keyword, lang = %lang);

    let cache_key = format!(
        "travel:search:{lang}:{keyword}:{}:{}:{}:{page}",
        q.destination_id.unwrap_or(0),
        q.price_min.unwrap_or(0),
        q.price_max.unwrap_or(0)
    );
    if let Some(cache) = &state.cache {
        if let Ok(Some(raw)) = cache.get(&cache_key).await {
            if let Ok(result) = serde_json::from_str::<SearchResult>(&String::from_utf8_lossy(&raw)) {
                log_search(&state, &keyword, &lang, result.total).await;
                return (StatusCode::OK, Json(ApiResponse { code: 0, message: "cache hit".into(), data: Some(result) }));
            }
        }
    }

    let result = if let Some(os) = &state.os {
        let d = search_os_index(os, "travel_destinations", &keyword, q.destination_id, q.price_min, q.price_max, page, &lang).await;
        let a = search_os_index(os, "travel_attractions", &keyword, q.destination_id, q.price_min, q.price_max, page, &lang).await;
        match (d, a) {
            (Some((d_items, d_total)), Some((a_items, a_total))) => {
                // 目的地在前 + 各索引页切片合并（与 MySQL 回退语义一致）
                let mut items = d_items;
                items.extend(a_items);
                SearchResult { total: d_total + a_total, page, page_size: PAGE_SIZE, items }
            }
            // OpenSearch 故障 → 回退 MySQL
            _ => match read_db(&state) {
                Some(db) => fallback_search(db, &keyword, q.destination_id, q.price_min, q.price_max, page, &lang).await,
                None => SearchResult { total: 0, page, page_size: PAGE_SIZE, items: Vec::new() },
            },
        }
    } else if let Some(db) = read_db(&state) {
        fallback_search(db, &keyword, q.destination_id, q.price_min, q.price_max, page, &lang).await
    } else {
        SearchResult { total: 0, page, page_size: PAGE_SIZE, items: Vec::new() }
    };

    if let Some(cache) = &state.cache {
        if let Ok(raw) = serde_json::to_string(&result) {
            if let Err(e) = cache.set(&cache_key, raw.as_bytes(), CACHE_TTL).await {
                tracing::warn!("search cache set failed: {e}");
            }
        }
    }
    log_search(&state, &keyword, &lang, result.total).await;
    (StatusCode::OK, Json(ApiResponse { code: 0, message: "ok".into(), data: Some(result) }))
}

// ===== 启动 =====

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

fn connect_os() -> Option<Arc<OpenSearchClient>> {
    let url = std::env::var("OPENSEARCH_URL").unwrap_or_else(|_| "http://localhost:9201".into());
    match OpenSearchClient::from_config(OpenSearchConfig { base_url: url, username: None, password: None, tls: None }) {
        Ok(os) => {
            tracing::info!("opensearch connected");
            Some(Arc::new(os))
        }
        Err(e) => {
            tracing::warn!("opensearch connect failed, continuing without it: {e}");
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
        os: connect_os(),
    };

    // 索引同步后台任务：不阻塞 HTTP 监听
    let sync_state = state.clone();
    tokio::spawn(async move { sync_loop(sync_state).await });

    // 中间件链（外层 → 内层）：ApiVersion → CircuitBreaker → Security → RateLimit
    let api = Router::new()
        .route("/api/search", get(search))
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
                    "search-service",
                    100,
                    60,
                )),
        );

    let router = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .merge(api)
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TracingLayer::new("search-service"))
                .layer(LoggingLayer),
        );

    let http_srv = HttpServer::new(PORT).router(router);

    let mut app = App::builder()
        .name("search-service")
        .version("v0.1.0")
        .server(http_srv)
        .on_start(|| async {
            tracing::info!("search-service listening on {PORT}");
            Ok(())
        })
        .build()?;

    app.run().await?;
    Ok(())
}
