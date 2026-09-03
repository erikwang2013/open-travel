// open-travel admin-service：管理端目的地/景区 CRUD
//
// admin 鉴权：CRUD 路由挂在 JwtAuthLayer 上（api_router），handler 内
// require_admin 校验 claims role=admin（缺失 401 / 非 admin 403）。
// body 以 serde_json::Value 接收，经静态白名单 pick 出列名，杜绝 SQL 注入。
// cover_url 直接收 URL 字符串；文件上传随 Phase 4 CDN 落地，本轮不做。
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::Engine as _;
use ecat_data::{RdbmsClient, Row};
use ecat_data_sqlx::SqlxClient;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::{err, AdminGuard, ApiResponse, AppState};

// 创建/更新白名单（与表列名一一对应）
const DEST_FIELDS: &[&str] = &[
    "name_en", "name_zh", "name_ja", "description", "cover_url", "status",
    "sort_order", "latitude", "longitude", "region_id", "category",
];
// sqlx Any 驱动无法解码 MySQL DECIMAL/TINYINT 列（fetch 直接报错），统一 CAST
const DEST_SELECT: &[&str] = &[
    "id", "name_en", "name_zh", "name_ja", "CAST(description AS CHAR) AS description",
    "CAST(latitude AS DOUBLE) AS latitude", "CAST(longitude AS DOUBLE) AS longitude",
    "category", "region_id", "cover_url", "CAST(status AS SIGNED) AS status", "sort_order",
];
const ATTR_FIELDS: &[&str] = &[
    "destination_id", "name_en", "name_zh", "name_ja", "name_ko", "name_ar",
    "name_es", "name_fr", "name_de", "name_pt", "name_hi", "name_bn",
    "name_id", "name_ru", "description", "price_cents", "status", "open_hours",
    "rating_avg", "cover_url",
];
// 更新不允许改 destination_id，避免悬空引用
const ATTR_UPDATE_FIELDS: &[&str] = &[
    "name_en", "name_zh", "name_ja", "name_ko", "name_ar", "name_es",
    "name_fr", "name_de", "name_pt", "name_hi", "name_bn", "name_id",
    "name_ru", "description", "price_cents", "status", "open_hours",
    "rating_avg", "cover_url",
];
const ATTR_SELECT: &[&str] = &[
    "id", "destination_id", "name_en", "name_zh", "name_ja", "name_ko",
    "name_ar", "name_es", "name_fr", "name_de", "name_pt", "name_hi",
    "name_bn", "name_id", "name_ru", "CAST(description AS CHAR) AS description",
    "price_cents", "CAST(status AS SIGNED) AS status", "open_hours",
    "CAST(rating_avg AS DOUBLE) AS rating_avg", "cover_url",
];

/// 从 body 中挑出白名单内的字段（跳过 null），返回列名与参数。
/// description 契约上是 JSON 对象，落库统一为 JSON 字符串（text/json 列）。
pub(crate) fn pick(body: &Map<String, Value>, fields: &[&str]) -> (Vec<String>, Vec<Value>) {
    let mut cols = Vec::new();
    let mut vals = Vec::new();
    for f in fields {
        if let Some(v) = body.get(*f) {
            if !v.is_null() {
                cols.push((*f).to_string());
                vals.push(if v.is_object() || v.is_array() {
                    Value::String(serde_json::to_string(v).unwrap_or_default())
                } else {
                    v.clone()
                });
            }
        }
    }
    (cols, vals)
}

pub(crate) fn col_alias(c: &str) -> &str {
    c.rsplit(" AS ").next().unwrap_or(c)
}

fn row_to_json(row: &Row, cols: &[&str]) -> Value {
    let vals = row.values();
    let mut m = Map::new();
    for (i, c) in cols.iter().enumerate() {
        let key = col_alias(c).to_string();
        let v = vals.get(i).cloned().unwrap_or(Value::Null);
        m.insert(key.clone(), v);
        // id_str：雪花 id > JS 2^53，Flutter web 数值往返会舍入，须附字符串形式
        if key == "id" {
            if let Some(n) = m.get("id").and_then(|x| x.as_u64()) {
                m.insert("id_str".into(), json!(n.to_string()));
            }
        }
    }
    // description 契约上是 JSON 对象：sqlx Any 驱动把 TEXT 列按 Blob 返回
    // base64（CAST 无效），此处兜底解码并还原为 JSON
    if let Some(desc) = m.remove("description") {
        m.insert("description".into(), desc_to_json(desc));
    }
    Value::Object(m)
}

fn desc_to_json(v: Value) -> Value {
    let Some(s) = v.as_str() else { return v };
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(s)
        .ok()
        .and_then(|b| String::from_utf8(b).ok());
    let candidate = decoded.unwrap_or_else(|| s.to_string());
    serde_json::from_str(&candidate).unwrap_or_else(|_| Value::String(candidate))
}

pub(crate) fn db_unavailable() -> Response {
    err::<Value>(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable").into_response()
}

pub(crate) fn not_found(entity: &str) -> Response {
    err::<Value>(StatusCode::NOT_FOUND, 404, &format!("{entity} not found")).into_response()
}

async fn fetch_dest(db: &SqlxClient, id: u64) -> Option<Value> {
    let sql = format!("SELECT {} FROM travel_destinations WHERE id = ?", DEST_SELECT.join(","));
    let rows = db.query_with(&sql, &[json!(id)]).await.ok()?;
    rows.first().map(|r| row_to_json(r, DEST_SELECT))
}

async fn fetch_attr(db: &SqlxClient, id: u64) -> Option<Value> {
    let sql = format!("SELECT {} FROM travel_attractions WHERE id = ?", ATTR_SELECT.join(","));
    let rows = db.query_with(&sql, &[json!(id)]).await.ok()?;
    rows.first().map(|r| row_to_json(r, ATTR_SELECT))
}

#[derive(Deserialize)]
pub(crate) struct PageQuery {
    #[serde(default = "default_page")]
    pub(crate) page: u64,
    #[serde(default = "default_page_size")]
    pub(crate) page_size: u64,
    pub(crate) status: Option<String>,
    pub(crate) keyword: Option<String>,
    pub(crate) destination_id: Option<u64>,
}

pub(crate) fn default_page() -> u64 {
    1
}

pub(crate) fn default_page_size() -> u64 {
    10
}

pub(crate) fn clamp_page(q: &PageQuery) -> (u64, u64) {
    (q.page.max(1), q.page_size.clamp(1, 100))
}

pub(crate) async fn list_destinations(
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
        cond.push_str(" AND (name_en LIKE ? OR name_zh LIKE ?)");
        params.push(json!(format!("%{kw}%")));
        params.push(json!(format!("%{kw}%")));
    }
    let total = match db
        .query_with(&format!("SELECT COUNT(*) AS total FROM travel_destinations{cond}"), &params)
        .await
    {
        Ok(rows) => rows
            .first()
            .and_then(|r| r.get("total"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as u64,
        Err(e) => {
            tracing::warn!(error = %e, "dest count query failed");
            return db_unavailable();
        }
    };
    let mut list_params = params.clone();
    list_params.push(json!(page_size));
    list_params.push(json!((page - 1) * page_size));
    let sql = format!(
        "SELECT {} FROM travel_destinations{cond} ORDER BY sort_order ASC, id ASC LIMIT ? OFFSET ?",
        DEST_SELECT.join(",")
    );
    let rows = match db.query_with(&sql, &list_params).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "dest list query failed");
            return db_unavailable();
        }
    };
    let list: Vec<Value> = rows.iter().map(|r| row_to_json(r, DEST_SELECT)).collect();
    ApiResponse::ok(json!({ "list": list, "total": total, "page": page, "page_size": page_size }))
        .into_response()
}

pub(crate) async fn create_destination(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    let name_en = obj.get("name_en").and_then(Value::as_str).unwrap_or("").to_string();
    let name_zh = obj.get("name_zh").and_then(Value::as_str).unwrap_or("").to_string();
    if name_en.trim().is_empty() || name_zh.trim().is_empty() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "name_en and name_zh are required").into_response();
    }
    let (mut cols, mut vals) = pick(obj, DEST_FIELDS);
    // NOT NULL 无默认值列，缺省补默认值，保证最小 body（仅名称）也能落库
    for (c, v) in [
        ("name_ja", json!("")),
        ("latitude", json!(0.0)),
        ("longitude", json!(0.0)),
        ("region_id", json!(0)),
    ] {
        if !cols.iter().any(|x| x == c) {
            cols.push(c.into());
            vals.push(v);
        }
    }
    // 主键去 AUTO_INCREMENT 后显式生成雪花 id（pick 白名单不含 id，body 无法覆盖）
    let new_id = idgen_rs::id_helper::next_id();
    cols.insert(0, "id".into());
    vals.insert(0, json!(new_id));
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let sql = format!(
        "INSERT INTO travel_destinations ({}) VALUES ({})",
        cols.join(","),
        vec!["?"; cols.len()].join(",")
    );
    if let Err(e) = db.execute_with(&sql, &vals).await {
        tracing::warn!(error = %e, "dest insert failed");
        return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
    }
    match fetch_dest(&db, new_id).await {
        Some(dest) => (StatusCode::OK, ApiResponse::ok(dest)).into_response(),
        None => db_unavailable(),
    }
}

pub(crate) async fn update_destination(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(id): Path<u64>,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    let (cols, vals) = pick(obj, DEST_FIELDS);
    if cols.is_empty() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "no fields to update").into_response();
    }
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let sets: Vec<String> = cols.iter().map(|c| format!("{c} = ?")).collect();
    let mut params = vals;
    params.push(json!(id));
    let sql = format!("UPDATE travel_destinations SET {} WHERE id = ?", sets.join(","));
    match db.execute_with(&sql, &params).await {
        Ok(0) => return not_found("destination"),
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, "dest update failed");
            return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
        }
    }
    match fetch_dest(&db, id).await {
        Some(dest) => (StatusCode::OK, ApiResponse::ok(dest)).into_response(),
        None => db_unavailable(),
    }
}

#[derive(Deserialize)]
pub(crate) struct StatusReq {
    pub(crate) status: i64,
}

pub(crate) async fn update_destination_status(
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
        .execute_with("UPDATE travel_destinations SET status = ? WHERE id = ?", &[json!(body.status), json!(id)])
        .await
    {
        Ok(0) => not_found("destination"),
        Ok(_) => match fetch_dest(&db, id).await {
            Some(dest) => (StatusCode::OK, ApiResponse::ok(dest)).into_response(),
            None => db_unavailable(),
        },
        Err(e) => {
            tracing::warn!(error = %e, "dest status update failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}

pub(crate) async fn delete_destination(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(id): Path<u64>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let rows = match db
        .query_with("SELECT COUNT(*) AS cnt FROM travel_attractions WHERE destination_id = ?", &[json!(id)])
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "attraction count query failed");
            return db_unavailable();
        }
    };
    let cnt = rows.first().and_then(|r| r.get("cnt")).and_then(|v| v.as_i64()).unwrap_or(0);
    if cnt > 0 {
        return err::<Value>(
            StatusCode::CONFLICT,
            409,
            "destination has related attractions, delete them first",
        )
        .into_response();
    }
    match db.execute_with("DELETE FROM travel_destinations WHERE id = ?", &[json!(id)]).await {
        Ok(0) => not_found("destination"),
        Ok(_) => (StatusCode::OK, ApiResponse::ok(Value::Null)).into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "dest delete failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}

pub(crate) async fn list_attractions(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Query(q): Query<PageQuery>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let (page, page_size) = clamp_page(&q);
    let mut cond = String::from(" WHERE 1=1");
    let mut params: Vec<Value> = Vec::new();
    if let Some(did) = q.destination_id {
        cond.push_str(" AND destination_id = ?");
        params.push(json!(did));
    }
    let total = match db
        .query_with(&format!("SELECT COUNT(*) AS total FROM travel_attractions{cond}"), &params)
        .await
    {
        Ok(rows) => rows
            .first()
            .and_then(|r| r.get("total"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as u64,
        Err(e) => {
            tracing::warn!(error = %e, "attr count query failed");
            return db_unavailable();
        }
    };
    let mut list_params = params.clone();
    list_params.push(json!(page_size));
    list_params.push(json!((page - 1) * page_size));
    let sql = format!(
        "SELECT {} FROM travel_attractions{cond} ORDER BY id ASC LIMIT ? OFFSET ?",
        ATTR_SELECT.join(",")
    );
    let rows = match db.query_with(&sql, &list_params).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "attr list query failed");
            return db_unavailable();
        }
    };
    let list: Vec<Value> = rows.iter().map(|r| row_to_json(r, ATTR_SELECT)).collect();
    ApiResponse::ok(json!({ "list": list, "total": total, "page": page, "page_size": page_size }))
        .into_response()
}

pub(crate) async fn create_attraction(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    let name_en = obj.get("name_en").and_then(Value::as_str).unwrap_or("").to_string();
    if name_en.trim().is_empty() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "name_en is required").into_response();
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
    let (mut cols, mut vals) = pick(obj, ATTR_FIELDS);
    // 13 个 name_* 与 open_hours 均 NOT NULL 无默认值，缺省补空串（契约：仅 name_en 必填）
    for c in ATTR_FIELDS
        .iter()
        .filter(|c| c.starts_with("name_") || **c == "open_hours")
    {
        if !cols.iter().any(|x| x == c) {
            cols.push((*c).into());
            vals.push(Value::String(String::new()));
        }
    }
    // 主键去 AUTO_INCREMENT 后显式生成雪花 id（pick 白名单不含 id，body 无法覆盖）
    let new_id = idgen_rs::id_helper::next_id();
    cols.insert(0, "id".into());
    vals.insert(0, json!(new_id));
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let sql = format!(
        "INSERT INTO travel_attractions ({}) VALUES ({})",
        cols.join(","),
        vec!["?"; cols.len()].join(",")
    );
    if let Err(e) = db.execute_with(&sql, &vals).await {
        tracing::warn!(error = %e, "attr insert failed");
        return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
    }
    match fetch_attr(&db, new_id).await {
        Some(attr) => (StatusCode::OK, ApiResponse::ok(attr)).into_response(),
        None => db_unavailable(),
    }
}

pub(crate) async fn update_attraction(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(id): Path<u64>,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    let (cols, vals) = pick(obj, ATTR_UPDATE_FIELDS);
    if cols.is_empty() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "no fields to update").into_response();
    }
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let sets: Vec<String> = cols.iter().map(|c| format!("{c} = ?")).collect();
    let mut params = vals;
    params.push(json!(id));
    let sql = format!("UPDATE travel_attractions SET {} WHERE id = ?", sets.join(","));
    match db.execute_with(&sql, &params).await {
        Ok(0) => return not_found("attraction"),
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, "attr update failed");
            return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
        }
    }
    match fetch_attr(&db, id).await {
        Some(attr) => (StatusCode::OK, ApiResponse::ok(attr)).into_response(),
        None => db_unavailable(),
    }
}

pub(crate) async fn delete_attraction(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(id): Path<u64>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    match db.execute_with("DELETE FROM travel_attractions WHERE id = ?", &[json!(id)]).await {
        Ok(0) => not_found("attraction"),
        Ok(_) => (StatusCode::OK, ApiResponse::ok(Value::Null)).into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "attr delete failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}
