// open-travel admin-service：线路（lines）CRUD + itinerary 双向转换
//
// 与 destinations 同模式：body 白名单 pick 防注入、显式列名、DATE/TINYINT
// 列 CAST（sqlx Any 驱动解码限制）、TEXT 列 base64 兜底。
// itinerary 双向转换：前端数组格式 ⇄ 存储 {"days":[...]} 格式（见函数注释）。
// 班期 CRUD 见 line_date_handlers.rs。
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::Engine as _;
use ecat_data::{Row, RdbmsClient};
use ecat_data_sqlx::SqlxClient;
use serde_json::{json, Map, Value};

use super::handlers::{clamp_page, db_unavailable, not_found, pick, PageQuery, StatusReq};
use super::{err, AdminGuard, ApiResponse, AppState};

const LINE_FIELDS: &[&str] = &[
    "title_en", "title_zh", "title_ja", "title_ko", "title_ru", "destination_id",
    "days", "departure_date", "price_cents", "max_pax", "status", "cover_url",
];
// DATE/TINYINT/SMALLINT 列 CAST：sqlx Any 对 MySQL 时间/微小整数类型解码失败
const LINE_SELECT: &[&str] = &[
    "id", "title_en", "title_zh", "title_ja", "title_ko", "title_ru", "destination_id",
    "CAST(days AS SIGNED) AS days", "CAST(departure_date AS CHAR) AS departure_date",
    "price_cents", "CAST(max_pax AS SIGNED) AS max_pax", "itinerary",
    "CAST(status AS SIGNED) AS status", "cover_url",
];
pub(crate) const DATE_SELECT: &[&str] = &[
    "id", "line_id", "CAST(depart_date AS CHAR) AS depart_date", "price_cents",
    "seats_left", "CAST(status AS SIGNED) AS status",
];
pub(crate) const DATE_FIELDS: &[&str] = &["depart_date", "price_cents", "seats_left", "status"];
const LINE_LANGS: [&str; 5] = ["en", "zh", "ja", "ko", "ru"];

pub(crate) fn col_u64(row: &Row, col: &str) -> u64 {
    row.get(col)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0)
}

pub(crate) fn col_str(row: &Row, col: &str) -> String {
    row.get(col).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

// ===== itinerary 双向转换（纯函数，可单测）=====

/// 标题对象 {en,zh,ja,ko,ru} → 平铺列 title_en/zh/...，缺失键补空串
fn flat_title(title: &Value) -> Map<String, Value> {
    let mut m = Map::new();
    for l in LINE_LANGS {
        let v = title.get(l).and_then(|v| v.as_str()).unwrap_or("").to_string();
        m.insert(format!("title_{l}"), Value::String(v));
    }
    m
}

/// description 对象 → 字符串：zh 优先、en 回退、再任取非空语言
fn desc_to_str(desc: &Value) -> String {
    if let Some(s) = desc.get("zh").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
        return s.to_string();
    }
    if let Some(s) = desc.get("en").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
        return s.to_string();
    }
    desc.as_object()
        .and_then(|m| m.values().find_map(|v| v.as_str().filter(|s| !s.is_empty())))
        .unwrap_or("")
        .to_string()
}

/// 入库：前端数组格式 [{"day":1,"title":{...},"description":{...}}]
/// → 存储格式 {"days":[{"day":1,"title_en":...,"description":"..."}]}
/// 已是 {"days":[...]}（种子/老数据）或不可解析时原样返回。
pub(crate) fn itinerary_to_storage(raw: &str) -> String {
    let Ok(v) = serde_json::from_str::<Value>(raw) else { return raw.to_string() };
    if let Some(arr) = v.as_array() {
        let days: Vec<Value> = arr
            .iter()
            .filter_map(|d| d.as_object())
            .map(|d| {
                let mut day = Map::new();
                day.insert("day".into(), d.get("day").cloned().unwrap_or(json!(0)));
                for (k, v) in flat_title(d.get("title").unwrap_or(&Value::Null)) {
                    day.insert(k, v);
                }
                let desc = match d.get("description") {
                    Some(Value::String(s)) => s.clone(),
                    Some(other) => desc_to_str(other),
                    None => String::new(),
                };
                day.insert("description".into(), json!(desc));
                Value::Object(day)
            })
            .collect();
        return serde_json::to_string(&json!({ "days": days })).unwrap_or_else(|_| raw.to_string());
    }
    if v.is_object() {
        // 已是存储格式：规范化后原样落库（兼容种子数据）
        return serde_json::to_string(&v).unwrap_or_else(|_| raw.to_string());
    }
    raw.to_string()
}

/// 出参：存储格式 {"days":[...]} → 前端数组格式（JSON 字符串）
/// title_* 平铺列合成 title 对象；description 字符串回填 zh（编辑所见即所存）。
/// 兼容种子老数据（days 内无 title 对象）与已是数组格式的数据。
pub(crate) fn itinerary_from_storage(raw: &str) -> String {
    // TEXT 列 sqlx Any 可能按 base64 返回，先解码再解析（同 order snapshot 模式）
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_else(|| raw.to_string());
    let Ok(v) = serde_json::from_str::<Value>(&decoded) else { return decoded };
    if v.as_array().is_some() {
        return serde_json::to_string(&v).unwrap_or(decoded);
    }
    let Some(days) = v.get("days").and_then(|d| d.as_array()) else { return decoded };
    let out: Vec<Value> = days
        .iter()
        .filter_map(|d| d.as_object())
        .map(|d| {
            let mut day = Map::new();
            day.insert("day".into(), d.get("day").cloned().unwrap_or(json!(0)));
            let mut title = Map::new();
            for l in LINE_LANGS {
                let t = d.get(&format!("title_{l}")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                title.insert(l.into(), Value::String(t));
            }
            day.insert("title".into(), Value::Object(title));
            let desc = d.get("description").and_then(|v| v.as_str()).unwrap_or("");
            day.insert("description".into(), if desc.is_empty() { json!({}) } else { json!({ "zh": desc }) });
            Value::Object(day)
        })
        .collect();
    serde_json::to_string(&json!(out)).unwrap_or(decoded)
}

pub(crate) fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[7] == b'-'
        && (0..10).all(|i| i == 4 || i == 7 || b[i].is_ascii_digit())
}

// ===== 行 → 前端 JSON =====

fn line_from_row(row: &Row) -> Value {
    json!({
        "id": col_u64(row, "id"),
        "id_str": col_u64(row, "id").to_string(),
        "title_en": col_str(row, "title_en"),
        "title_zh": col_str(row, "title_zh"),
        "title_ja": col_str(row, "title_ja"),
        "title_ko": col_str(row, "title_ko"),
        "title_ru": col_str(row, "title_ru"),
        "destination_id": col_u64(row, "destination_id"),
        "days": col_u64(row, "days"),
        "departure_date": col_str(row, "departure_date"),
        "price_cents": col_u64(row, "price_cents"),
        "max_pax": col_u64(row, "max_pax"),
        "itinerary": itinerary_from_storage(&col_str(row, "itinerary")),
        "status": col_u64(row, "status"),
        "cover_url": col_str(row, "cover_url"),
    })
}

pub(crate) fn date_from_row(row: &Row) -> Value {
    json!({
        "id": col_u64(row, "id"),
        "id_str": col_u64(row, "id").to_string(),
        "line_id": col_u64(row, "line_id"),
        "depart_date": col_str(row, "depart_date"),
        "price_cents": col_u64(row, "price_cents"),
        "seats_left": col_u64(row, "seats_left"),
        "status": col_u64(row, "status"),
    })
}

async fn fetch_line(db: &SqlxClient, id: u64) -> Option<Value> {
    let sql = format!("SELECT {} FROM travel_lines WHERE id = ?", LINE_SELECT.join(","));
    let rows = db.query_with(&sql, &[json!(id)]).await.ok()?;
    rows.first().map(line_from_row)
}

pub(crate) async fn fetch_date(db: &SqlxClient, line_id: u64, id: u64) -> Option<Value> {
    let sql = format!(
        "SELECT {} FROM travel_line_dates WHERE id = ? AND line_id = ?",
        DATE_SELECT.join(",")
    );
    let rows = db.query_with(&sql, &[json!(id), json!(line_id)]).await.ok()?;
    rows.first().map(date_from_row)
}

/// body 中 itinerary 单独处理：前端数组字符串 → 存储 {"days":[...]} 字符串。
/// 返回 (cols, vals) 追加式字段，调用方按需拼接。
fn itinerary_col(body: &Map<String, Value>) -> Option<(String, Value)> {
    let v = body.get("itinerary")?;
    if v.is_null() {
        return None;
    }
    let raw = match v {
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    };
    Some(("itinerary".into(), json!(itinerary_to_storage(&raw))))
}

// ===== 线路 CRUD =====

/// 列表：keyword 匹配 title_zh/title_en，status 过滤；响应键 items（前端契约）。
pub(crate) async fn list_lines(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Query(q): Query<PageQuery>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let (page, page_size) = clamp_page(&q);
    let mut cond = String::from(" WHERE 1=1");
    let mut params: Vec<Value> = Vec::new();
    if let Some(s) = q.status.as_deref().filter(|s| !s.is_empty()) {
        let Ok(status) = s.parse::<i64>() else {
            return err::<Value>(StatusCode::BAD_REQUEST, 400, "status must be 0 or 1").into_response();
        };
        if status != 0 && status != 1 {
            return err::<Value>(StatusCode::BAD_REQUEST, 400, "status must be 0 or 1").into_response();
        }
        cond.push_str(" AND status = ?");
        params.push(json!(status));
    }
    if let Some(kw) = q.keyword.as_deref().filter(|k| !k.is_empty()) {
        cond.push_str(" AND (title_zh LIKE ? OR title_en LIKE ?)");
        params.push(json!(format!("%{kw}%")));
        params.push(json!(format!("%{kw}%")));
    }
    let total = match db
        .query_with(&format!("SELECT COUNT(*) AS total FROM travel_lines{cond}"), &params)
        .await
    {
        Ok(rows) => rows
            .first()
            .and_then(|r| r.get("total"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as u64,
        Err(e) => {
            tracing::warn!(error = %e, "line count query failed");
            return db_unavailable();
        }
    };
    let mut list_params = params;
    list_params.push(json!(page_size));
    list_params.push(json!((page - 1) * page_size));
    let sql = format!(
        "SELECT {} FROM travel_lines{cond} ORDER BY id ASC LIMIT ? OFFSET ?",
        LINE_SELECT.join(",")
    );
    let rows = match db.query_with(&sql, &list_params).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "line list query failed");
            return db_unavailable();
        }
    };
    let list: Vec<Value> = rows.iter().map(line_from_row).collect();
    ApiResponse::ok(json!({ "items": list, "total": total, "page": page, "page_size": page_size }))
        .into_response()
}

pub(crate) async fn create_line(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    let title_zh = obj.get("title_zh").and_then(Value::as_str).unwrap_or("").trim().to_string();
    if title_zh.is_empty() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "title_zh is required").into_response();
    }
    let Some(dest_id) = obj.get("destination_id").and_then(Value::as_u64) else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "destination_id is required").into_response();
    };
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let exists = match db
        .query_with("SELECT id FROM travel_destinations WHERE id = ?", &[json!(dest_id)])
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "dest existence check failed");
            return db_unavailable();
        }
    };
    if exists.is_empty() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "destination not found").into_response();
    }
    let (mut cols, mut vals) = pick(obj, LINE_FIELDS);
    // NOT NULL 无默认值/必填列缺省补默认值，保证最小 body 也能落库
    for (c, v) in [
        ("title_en", json!("")),
        ("title_ja", json!("")),
        ("title_ko", json!("")),
        ("title_ru", json!("")),
        ("days", json!(1)),
        ("price_cents", json!(0)),
        ("max_pax", json!(20)),
    ] {
        if !cols.iter().any(|x| x == c) {
            cols.push(c.into());
            vals.push(v);
        }
    }
    if let Some((c, v)) = itinerary_col(obj) {
        cols.push(c);
        vals.push(v);
    }
    // 主键去 AUTO_INCREMENT 后显式生成雪花 id（pick 白名单不含 id，body 无法覆盖）
    let new_id = idgen_rs::id_helper::next_id();
    cols.insert(0, "id".into());
    vals.insert(0, json!(new_id));
    let sql = format!(
        "INSERT INTO travel_lines ({}) VALUES ({})",
        cols.join(","),
        vec!["?"; cols.len()].join(",")
    );
    if let Err(e) = db.execute_with(&sql, &vals).await {
        tracing::warn!(error = %e, "line insert failed");
        return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
    }
    match fetch_line(&db, new_id).await {
        Some(line) => (StatusCode::OK, ApiResponse::ok(line)).into_response(),
        None => db_unavailable(),
    }
}

pub(crate) async fn update_line(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(id): Path<u64>,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    let (mut cols, mut vals) = pick(obj, LINE_FIELDS);
    if let Some((c, v)) = itinerary_col(obj) {
        cols.push(c);
        vals.push(v);
    }
    if cols.is_empty() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "no fields to update").into_response();
    }
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let sets: Vec<String> = cols.iter().map(|c| format!("{c} = ?")).collect();
    let mut params = vals;
    params.push(json!(id));
    let sql = format!("UPDATE travel_lines SET {} WHERE id = ?", sets.join(","));
    match db.execute_with(&sql, &params).await {
        Ok(0) => return not_found("line"),
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, "line update failed");
            return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
        }
    }
    match fetch_line(&db, id).await {
        Some(line) => (StatusCode::OK, ApiResponse::ok(line)).into_response(),
        None => db_unavailable(),
    }
}

pub(crate) async fn update_line_status(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(id): Path<u64>,
    Json(body): Json<StatusReq>,
) -> Response {
    if body.status != 0 && body.status != 1 {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "status must be 0 or 1").into_response();
    }
    let Some(db) = state.db.clone() else { return db_unavailable() };
    match db
        .execute_with("UPDATE travel_lines SET status = ? WHERE id = ?", &[json!(body.status), json!(id)])
        .await
    {
        Ok(0) => not_found("line"),
        Ok(_) => match fetch_line(&db, id).await {
            Some(line) => (StatusCode::OK, ApiResponse::ok(line)).into_response(),
            None => db_unavailable(),
        },
        Err(e) => {
            tracing::warn!(error = %e, "line status update failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}

/// 物理删除（同 destinations 模式）：有关联班期时 409 提示先删班期。
pub(crate) async fn delete_line(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(id): Path<u64>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let rows = match db
        .query_with("SELECT COUNT(*) AS cnt FROM travel_line_dates WHERE line_id = ?", &[json!(id)])
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "line date count query failed");
            return db_unavailable();
        }
    };
    let cnt = rows.first().and_then(|r| r.get("cnt")).and_then(|v| v.as_i64()).unwrap_or(0);
    if cnt > 0 {
        return err::<Value>(StatusCode::CONFLICT, 409, "line has related dates, delete them first")
            .into_response();
    }
    match db.execute_with("DELETE FROM travel_lines WHERE id = ?", &[json!(id)]).await {
        Ok(0) => not_found("line"),
        Ok(_) => (StatusCode::OK, ApiResponse::ok(Value::Null)).into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "line delete failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}

