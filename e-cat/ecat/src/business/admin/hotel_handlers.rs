// open-travel admin-service：酒店与房型管理 CRUD（P4-10 后端）
//
// 与 flights 同模式：body 白名单 pick、显式列名、CHAR(3)/DECIMAL/TINYINT
// 列 CAST（sqlx Any 解码限制）。房型全部挂在酒店 id 之下，删除/更新以
// (id, hotel_id) 双条件限定作用域；删酒店前须先删房型（409）。
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use ecat_data::{Row, RdbmsClient};
use ecat_data_sqlx::SqlxClient;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::handlers::{db_unavailable, default_page, default_page_size, not_found, pick, StatusReq};
use super::line_handlers::col_u64;
use super::{err, AdminGuard, ApiResponse, AppState};

const HOTEL_FIELDS: &[&str] = &[
    "name_en", "name_zh", "name_ja", "city_code", "star", "latitude", "longitude",
    "cover_url", "status",
];
// DECIMAL(latitude/longitude)/TINYINT(star/status)/CHAR(3) 列 CAST
const HOTEL_SELECT: &[&str] = &[
    "id", "name_en", "name_zh", "name_ja", "CAST(city_code AS CHAR) AS city_code",
    "CAST(star AS SIGNED) AS star", "CAST(latitude AS DOUBLE) AS latitude",
    "CAST(longitude AS DOUBLE) AS longitude", "cover_url", "CAST(status AS SIGNED) AS status",
];
const ROOM_FIELDS: &[&str] = &[
    "room_type_en", "room_type_zh", "room_type_ja", "price_cents", "breakfast",
    "inventory", "status",
];
const ROOM_SELECT: &[&str] = &[
    "id", "hotel_id", "room_type_en", "room_type_zh", "room_type_ja", "price_cents",
    "CAST(breakfast AS SIGNED) AS breakfast", "CAST(inventory AS SIGNED) AS inventory",
    "CAST(status AS SIGNED) AS status",
];

#[derive(Deserialize)]
pub(crate) struct HotelsQuery {
    #[serde(default = "default_page")]
    pub(crate) page: u64,
    #[serde(default = "default_page_size")]
    pub(crate) page_size: u64,
    pub(crate) keyword: Option<String>,
    pub(crate) city: Option<String>,
}

fn col_str(row: &Row, col: &str) -> String {
    row.get(col).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn col_f64(row: &Row, col: &str) -> f64 {
    row.get(col)
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0.0)
}

fn hotel_from_row(row: &Row) -> Value {
    json!({
        "id": col_u64(row, "id"),
        "id_str": col_u64(row, "id").to_string(),
        "name_en": col_str(row, "name_en"),
        "name_zh": col_str(row, "name_zh"),
        "name_ja": col_str(row, "name_ja"),
        "city_code": col_str(row, "city_code"),
        "star": col_u64(row, "star"),
        "latitude": col_f64(row, "latitude"),
        "longitude": col_f64(row, "longitude"),
        "cover_url": col_str(row, "cover_url"),
        "status": col_u64(row, "status"),
    })
}

fn room_from_row(row: &Row) -> Value {
    json!({
        "id": col_u64(row, "id"),
        "id_str": col_u64(row, "id").to_string(),
        "hotel_id": col_u64(row, "hotel_id"),
        "room_type_en": col_str(row, "room_type_en"),
        "room_type_zh": col_str(row, "room_type_zh"),
        "room_type_ja": col_str(row, "room_type_ja"),
        "price_cents": col_u64(row, "price_cents"),
        "breakfast": col_u64(row, "breakfast"),
        "inventory": col_u64(row, "inventory"),
        "status": col_u64(row, "status"),
    })
}

async fn fetch_hotel(db: &SqlxClient, id: u64) -> Option<Value> {
    let sql = format!("SELECT {} FROM travel_hotels WHERE id = ?", HOTEL_SELECT.join(","));
    let rows = db.query_with(&sql, &[json!(id)]).await.ok()?;
    rows.first().map(hotel_from_row)
}

async fn fetch_room(db: &SqlxClient, hotel_id: u64, id: u64) -> Option<Value> {
    let sql = format!(
        "SELECT {} FROM travel_hotel_rooms WHERE id = ? AND hotel_id = ?",
        ROOM_SELECT.join(",")
    );
    let rows = db.query_with(&sql, &[json!(id), json!(hotel_id)]).await.ok()?;
    rows.first().map(room_from_row)
}

/// 城市码校验：恰好 3 个字母（与 IATA 一致）
fn valid_city(s: &str) -> bool {
    s.len() == 3 && s.bytes().all(|b| b.is_ascii_alphabetic())
}

fn check_hotel_fields(body: &Map<String, Value>) -> Result<(), String> {
    if let Some(s) = body.get("city_code").and_then(Value::as_str) {
        if !valid_city(s) {
            return Err("city_code must be a 3-letter code".into());
        }
    }
    if let Some(s) = body.get("status").and_then(Value::as_i64) {
        if s != 0 && s != 1 {
            return Err("status must be 0 or 1".into());
        }
    }
    Ok(())
}

fn check_room_fields(body: &Map<String, Value>) -> Result<(), String> {
    if let Some(s) = body.get("status").and_then(Value::as_i64) {
        if s != 0 && s != 1 {
            return Err("status must be 0 or 1".into());
        }
    }
    Ok(())
}

/// 酒店列表：keyword 匹配 name_en/name_zh；city 为城市码精确过滤。
pub(crate) async fn list_hotels(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Query(q): Query<HotelsQuery>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let page = q.page.max(1);
    let page_size = q.page_size.clamp(1, 100);
    let mut cond = String::from(" WHERE 1=1");
    let mut params: Vec<Value> = Vec::new();
    if let Some(kw) = q.keyword.as_deref().filter(|k| !k.is_empty()) {
        cond.push_str(" AND (name_en LIKE ? OR name_zh LIKE ?)");
        params.push(json!(format!("%{kw}%")));
        params.push(json!(format!("%{kw}%")));
    }
    if let Some(c) = q.city.as_deref().filter(|c| !c.is_empty()) {
        cond.push_str(" AND city_code = ?");
        params.push(json!(c));
    }
    let total = match db
        .query_with(&format!("SELECT COUNT(*) AS total FROM travel_hotels{cond}"), &params)
        .await
    {
        Ok(rows) => rows
            .first()
            .and_then(|r| r.get("total"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as u64,
        Err(e) => {
            tracing::warn!(error = %e, "hotel count query failed");
            return db_unavailable();
        }
    };
    let mut list_params = params;
    list_params.push(json!(page_size));
    list_params.push(json!((page - 1) * page_size));
    let sql = format!(
        "SELECT {} FROM travel_hotels{cond} ORDER BY id ASC LIMIT ? OFFSET ?",
        HOTEL_SELECT.join(",")
    );
    let rows = match db.query_with(&sql, &list_params).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "hotel list query failed");
            return db_unavailable();
        }
    };
    let list: Vec<Value> = rows.iter().map(hotel_from_row).collect();
    ApiResponse::ok(json!({ "items": list, "total": total, "page": page, "page_size": page_size }))
        .into_response()
}

pub(crate) async fn create_hotel(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    for f in ["name_en", "name_zh", "city_code"] {
        if obj.get(f).and_then(Value::as_str).unwrap_or("").trim().is_empty() {
            return err::<Value>(StatusCode::BAD_REQUEST, 400, &format!("{f} is required")).into_response();
        }
    }
    if let Err(msg) = check_hotel_fields(obj) {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, &msg).into_response();
    }
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let (mut cols, mut vals) = pick(obj, HOTEL_FIELDS);
    // NOT NULL 无默认值/必填列缺省补默认值，保证最小 body 也能落库
    for (c, v) in [
        ("name_ja", json!("")),
        ("star", json!(3)),
        ("latitude", json!(0.0)),
        ("longitude", json!(0.0)),
        ("cover_url", json!("")),
        ("status", json!(1)),
    ] {
        if !cols.iter().any(|x| x == c) {
            cols.push(c.into());
            vals.push(v);
        }
    }
    // 主键去 AUTO_INCREMENT 后显式生成雪花 id（pick 白名单不含 id，body 无法覆盖）
    let new_id = ecat::business::shared::snowflake_id().await;
    cols.insert(0, "id".into());
    vals.insert(0, json!(new_id));
    let sql = format!(
        "INSERT INTO travel_hotels ({}) VALUES ({})",
        cols.join(","),
        vec!["?"; cols.len()].join(",")
    );
    if let Err(e) = db.execute_with(&sql, &vals).await {
        tracing::warn!(error = %e, "hotel insert failed");
        return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
    }
    match fetch_hotel(&db, new_id).await {
        Some(h) => (StatusCode::OK, ApiResponse::ok(h)).into_response(),
        None => db_unavailable(),
    }
}

pub(crate) async fn update_hotel(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(id): Path<u64>,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    if let Err(msg) = check_hotel_fields(obj) {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, &msg).into_response();
    }
    let (cols, vals) = pick(obj, HOTEL_FIELDS);
    if cols.is_empty() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "no fields to update").into_response();
    }
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let sets: Vec<String> = cols.iter().map(|c| format!("{c} = ?")).collect();
    let mut params = vals;
    params.push(json!(id));
    let sql = format!("UPDATE travel_hotels SET {} WHERE id = ?", sets.join(","));
    match db.execute_with(&sql, &params).await {
        Ok(0) => return not_found("hotel"),
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, "hotel update failed");
            return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
        }
    }
    match fetch_hotel(&db, id).await {
        Some(h) => (StatusCode::OK, ApiResponse::ok(h)).into_response(),
        None => db_unavailable(),
    }
}

pub(crate) async fn update_hotel_status(
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
        .execute_with("UPDATE travel_hotels SET status = ? WHERE id = ?", &[json!(body.status), json!(id)])
        .await
    {
        Ok(0) => not_found("hotel"),
        Ok(_) => match fetch_hotel(&db, id).await {
            Some(h) => (StatusCode::OK, ApiResponse::ok(h)).into_response(),
            None => db_unavailable(),
        },
        Err(e) => {
            tracing::warn!(error = %e, "hotel status update failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}

/// 物理删除：有关联房型时 409 提示先删房型。
pub(crate) async fn delete_hotel(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(id): Path<u64>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let rows = match db
        .query_with("SELECT COUNT(*) AS cnt FROM travel_hotel_rooms WHERE hotel_id = ?", &[json!(id)])
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "room count query failed");
            return db_unavailable();
        }
    };
    let cnt = rows.first().and_then(|r| r.get("cnt")).and_then(|v| v.as_i64()).unwrap_or(0);
    if cnt > 0 {
        return err::<Value>(
            StatusCode::CONFLICT,
            409,
            "hotel has related rooms, delete them first",
        )
        .into_response();
    }
    match db.execute_with("DELETE FROM travel_hotels WHERE id = ?", &[json!(id)]).await {
        Ok(0) => not_found("hotel"),
        Ok(_) => (StatusCode::OK, ApiResponse::ok(Value::Null)).into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "hotel delete failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}

/// 房型列表：返回裸数组（前端直接按 List 消费）。
pub(crate) async fn list_rooms(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(hotel_id): Path<u64>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let rows = match db
        .query_with("SELECT id FROM travel_hotels WHERE id = ?", &[json!(hotel_id)])
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "hotel existence check failed");
            return db_unavailable();
        }
    };
    if rows.is_empty() {
        return not_found("hotel");
    }
    let sql = format!(
        "SELECT {} FROM travel_hotel_rooms WHERE hotel_id = ? ORDER BY id ASC",
        ROOM_SELECT.join(",")
    );
    let rows = match db.query_with(&sql, &[json!(hotel_id)]).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "rooms list query failed");
            return db_unavailable();
        }
    };
    let list: Vec<Value> = rows.iter().map(room_from_row).collect();
    ApiResponse::ok(list).into_response()
}

pub(crate) async fn create_room(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(hotel_id): Path<u64>,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    let room_type_en = obj.get("room_type_en").and_then(Value::as_str).unwrap_or("").trim().to_string();
    if room_type_en.is_empty() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "room_type_en is required").into_response();
    }
    if obj.get("price_cents").and_then(Value::as_u64).is_none() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "price_cents is required").into_response();
    }
    if let Err(msg) = check_room_fields(obj) {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, &msg).into_response();
    }
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let rows = match db
        .query_with("SELECT id FROM travel_hotels WHERE id = ?", &[json!(hotel_id)])
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "hotel existence check failed");
            return db_unavailable();
        }
    };
    if rows.is_empty() {
        return not_found("hotel");
    }
    let (mut cols, mut vals) = pick(obj, ROOM_FIELDS);
    for (c, v) in [
        ("room_type_zh", json!("")),
        ("room_type_ja", json!("")),
        ("breakfast", json!(0)),
        ("inventory", json!(0)),
        ("status", json!(1)),
    ] {
        if !cols.iter().any(|x| x == c) {
            cols.push(c.into());
            vals.push(v);
        }
    }
    // 主键去 AUTO_INCREMENT 后显式生成雪花 id（pick 白名单不含 id，body 无法覆盖）
    let new_id = ecat::business::shared::snowflake_id().await;
    let mut insert_cols = vec!["id".to_string(), "hotel_id".to_string()];
    insert_cols.extend(cols);
    let mut insert_vals = vec![json!(new_id), json!(hotel_id)];
    insert_vals.extend(vals);
    let sql = format!(
        "INSERT INTO travel_hotel_rooms ({}) VALUES ({})",
        insert_cols.join(","),
        vec!["?"; insert_cols.len()].join(",")
    );
    if let Err(e) = db.execute_with(&sql, &insert_vals).await {
        tracing::warn!(error = %e, "room insert failed");
        return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
    }
    match fetch_room(&db, hotel_id, new_id).await {
        Some(r) => (StatusCode::OK, ApiResponse::ok(r)).into_response(),
        None => db_unavailable(),
    }
}

pub(crate) async fn update_room(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path((hotel_id, room_id)): Path<(u64, u64)>,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    if let Err(msg) = check_room_fields(obj) {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, &msg).into_response();
    }
    let (cols, vals) = pick(obj, ROOM_FIELDS);
    if cols.is_empty() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "no fields to update").into_response();
    }
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let sets: Vec<String> = cols.iter().map(|c| format!("{c} = ?")).collect();
    let mut params = vals;
    params.push(json!(room_id));
    params.push(json!(hotel_id));
    let sql = format!("UPDATE travel_hotel_rooms SET {} WHERE id = ? AND hotel_id = ?", sets.join(","));
    match db.execute_with(&sql, &params).await {
        Ok(0) => return not_found("room"),
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, "room update failed");
            return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
        }
    }
    match fetch_room(&db, hotel_id, room_id).await {
        Some(r) => (StatusCode::OK, ApiResponse::ok(r)).into_response(),
        None => db_unavailable(),
    }
}

pub(crate) async fn delete_room(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path((hotel_id, room_id)): Path<(u64, u64)>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    match db
        .execute_with(
            "DELETE FROM travel_hotel_rooms WHERE id = ? AND hotel_id = ?",
            &[json!(room_id), json!(hotel_id)],
        )
        .await
    {
        Ok(0) => not_found("room"),
        Ok(_) => (StatusCode::OK, ApiResponse::ok(Value::Null)).into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "room delete failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}
