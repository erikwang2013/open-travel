// open-travel admin-service：航班管理 CRUD（P4-10 后端）
//
// 与 lines 同模式：body 白名单 pick 防注入、显式列名、CHAR(3)/DATETIME/
// TINYINT 列 CAST（sqlx Any 解码限制）、IATA 码 3 字母校验。
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

const FLIGHT_FIELDS: &[&str] = &[
    "airline", "flight_no", "from_code", "to_code", "depart_at", "arrive_at",
    "cabin", "price_cents", "seats_left", "status",
];
const FLIGHT_SELECT: &[&str] = &[
    "id", "airline", "flight_no",
    "CAST(from_code AS CHAR) AS from_code", "CAST(to_code AS CHAR) AS to_code",
    "CAST(depart_at AS CHAR) AS depart_at", "CAST(arrive_at AS CHAR) AS arrive_at",
    "CAST(cabin AS SIGNED) AS cabin", "price_cents",
    "CAST(seats_left AS SIGNED) AS seats_left", "CAST(status AS SIGNED) AS status",
];

#[derive(Deserialize)]
pub(crate) struct FlightsQuery {
    #[serde(default = "default_page")]
    pub(crate) page: u64,
    #[serde(default = "default_page_size")]
    pub(crate) page_size: u64,
    pub(crate) keyword: Option<String>,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
}

fn col_str(row: &Row, col: &str) -> String {
    row.get(col).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn flight_from_row(row: &Row) -> Value {
    json!({
        "id": col_u64(row, "id"),
        "airline": col_str(row, "airline"),
        "flight_no": col_str(row, "flight_no"),
        "from_code": col_str(row, "from_code"),
        "to_code": col_str(row, "to_code"),
        "depart_at": col_str(row, "depart_at"),
        "arrive_at": col_str(row, "arrive_at"),
        "cabin": col_u64(row, "cabin"),
        "price_cents": col_u64(row, "price_cents"),
        "seats_left": col_u64(row, "seats_left"),
        "status": col_u64(row, "status"),
    })
}

async fn fetch_flight(db: &SqlxClient, id: u64) -> Option<Value> {
    let sql = format!("SELECT {} FROM travel_flights WHERE id = ?", FLIGHT_SELECT.join(","));
    let rows = db.query_with(&sql, &[json!(id)]).await.ok()?;
    rows.first().map(flight_from_row)
}

/// IATA 机场码：恰好 3 个字母
fn valid_iata(s: &str) -> bool {
    s.len() == 3 && s.bytes().all(|b| b.is_ascii_alphabetic())
}

/// DATETIME 格式校验：YYYY-MM-DD HH:MM:SS（DB 拒绝前先 400，避免 500）
fn valid_dt(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 19 && b[4] == b'-' && b[7] == b'-' && b[10] == b' ' && b[13] == b':'
        && b[16] == b':'
        && (0..19).all(|i| matches!(i, 4 | 7 | 10 | 13 | 16) || b[i].is_ascii_digit())
}

fn check_fields(body: &Map<String, Value>) -> Result<(), String> {
    if let Some(s) = body.get("from_code").and_then(Value::as_str) {
        if !valid_iata(s) {
            return Err("from_code must be a 3-letter IATA code".into());
        }
    }
    if let Some(s) = body.get("to_code").and_then(Value::as_str) {
        if !valid_iata(s) {
            return Err("to_code must be a 3-letter IATA code".into());
        }
    }
    if let Some(s) = body.get("depart_at").and_then(Value::as_str) {
        if !valid_dt(s) {
            return Err("depart_at must be YYYY-MM-DD HH:MM:SS".into());
        }
    }
    if let Some(s) = body.get("arrive_at").and_then(Value::as_str) {
        if !valid_dt(s) {
            return Err("arrive_at must be YYYY-MM-DD HH:MM:SS".into());
        }
    }
    if let Some(s) = body.get("status").and_then(Value::as_i64) {
        if s != 0 && s != 1 {
            return Err("status must be 0 or 1".into());
        }
    }
    Ok(())
}

/// 航班列表：keyword 匹配 airline/flight_no；from/to 为 IATA 码精确过滤。
pub(crate) async fn list_flights(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Query(q): Query<FlightsQuery>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let page = q.page.max(1);
    let page_size = q.page_size.clamp(1, 100);
    let mut cond = String::from(" WHERE 1=1");
    let mut params: Vec<Value> = Vec::new();
    if let Some(kw) = q.keyword.as_deref().filter(|k| !k.is_empty()) {
        cond.push_str(" AND (airline LIKE ? OR flight_no LIKE ?)");
        params.push(json!(format!("%{kw}%")));
        params.push(json!(format!("%{kw}%")));
    }
    if let Some(f) = q.from.as_deref().filter(|f| !f.is_empty()) {
        cond.push_str(" AND from_code = ?");
        params.push(json!(f));
    }
    if let Some(t) = q.to.as_deref().filter(|t| !t.is_empty()) {
        cond.push_str(" AND to_code = ?");
        params.push(json!(t));
    }
    let total = match db
        .query_with(&format!("SELECT COUNT(*) AS total FROM travel_flights{cond}"), &params)
        .await
    {
        Ok(rows) => rows
            .first()
            .and_then(|r| r.get("total"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as u64,
        Err(e) => {
            tracing::warn!(error = %e, "flight count query failed");
            return db_unavailable();
        }
    };
    let mut list_params = params;
    list_params.push(json!(page_size));
    list_params.push(json!((page - 1) * page_size));
    let sql = format!(
        "SELECT {} FROM travel_flights{cond} ORDER BY id ASC LIMIT ? OFFSET ?",
        FLIGHT_SELECT.join(",")
    );
    let rows = match db.query_with(&sql, &list_params).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "flight list query failed");
            return db_unavailable();
        }
    };
    let list: Vec<Value> = rows.iter().map(flight_from_row).collect();
    ApiResponse::ok(json!({ "items": list, "total": total, "page": page, "page_size": page_size }))
        .into_response()
}

pub(crate) async fn create_flight(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    for f in ["airline", "flight_no", "from_code", "to_code", "depart_at"] {
        if obj.get(f).and_then(Value::as_str).unwrap_or("").trim().is_empty() {
            return err::<Value>(StatusCode::BAD_REQUEST, 400, &format!("{f} is required")).into_response();
        }
    }
    if obj.get("price_cents").and_then(Value::as_u64).is_none() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "price_cents is required").into_response();
    }
    if let Err(msg) = check_fields(obj) {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, &msg).into_response();
    }
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let (mut cols, mut vals) = pick(obj, FLIGHT_FIELDS);
    let depart_at = obj.get("depart_at").and_then(Value::as_str).unwrap_or("").to_string();
    // NOT NULL 无默认值/必填列缺省补默认值，保证最小 body 也能落库
    for (c, v) in [
        ("arrive_at", json!(depart_at)),
        ("cabin", json!(0)),
        ("seats_left", json!(0)),
        ("status", json!(1)),
    ] {
        if !cols.iter().any(|x| x == c) {
            cols.push(c.into());
            vals.push(v);
        }
    }
    let sql = format!(
        "INSERT INTO travel_flights ({}) VALUES ({})",
        cols.join(","),
        vec!["?"; cols.len()].join(",")
    );
    if let Err(e) = db.execute_with(&sql, &vals).await {
        tracing::warn!(error = %e, "flight insert failed");
        return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
    }
    // 取回新行 id：航班号+出发时间配对查询（管理端低频写，重复时刻取最新一条）
    let rows = db
        .query_with(
            "SELECT id FROM travel_flights WHERE flight_no = ? AND depart_at = ? ORDER BY id DESC LIMIT 1",
            &[json!(obj.get("flight_no").and_then(Value::as_str).unwrap_or("")), json!(depart_at)],
        )
        .await
        .ok()
        .and_then(|r| r.into_iter().next());
    let Some(row) = rows else {
        return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
    };
    let Some(id) = row.get("id").and_then(|v| v.as_u64()) else {
        return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
    };
    match fetch_flight(&db, id).await {
        Some(f) => (StatusCode::OK, ApiResponse::ok(f)).into_response(),
        None => db_unavailable(),
    }
}

pub(crate) async fn update_flight(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(id): Path<u64>,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    if let Err(msg) = check_fields(obj) {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, &msg).into_response();
    }
    let (cols, vals) = pick(obj, FLIGHT_FIELDS);
    if cols.is_empty() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "no fields to update").into_response();
    }
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let sets: Vec<String> = cols.iter().map(|c| format!("{c} = ?")).collect();
    let mut params = vals;
    params.push(json!(id));
    let sql = format!("UPDATE travel_flights SET {} WHERE id = ?", sets.join(","));
    match db.execute_with(&sql, &params).await {
        Ok(0) => return not_found("flight"),
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, "flight update failed");
            return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
        }
    }
    match fetch_flight(&db, id).await {
        Some(f) => (StatusCode::OK, ApiResponse::ok(f)).into_response(),
        None => db_unavailable(),
    }
}

pub(crate) async fn update_flight_status(
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
        .execute_with("UPDATE travel_flights SET status = ? WHERE id = ?", &[json!(body.status), json!(id)])
        .await
    {
        Ok(0) => not_found("flight"),
        Ok(_) => match fetch_flight(&db, id).await {
            Some(f) => (StatusCode::OK, ApiResponse::ok(f)).into_response(),
            None => db_unavailable(),
        },
        Err(e) => {
            tracing::warn!(error = %e, "flight status update failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}

/// 物理删除。
pub(crate) async fn delete_flight(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(id): Path<u64>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    match db.execute_with("DELETE FROM travel_flights WHERE id = ?", &[json!(id)]).await {
        Ok(0) => not_found("flight"),
        Ok(_) => (StatusCode::OK, ApiResponse::ok(Value::Null)).into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "flight delete failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}
