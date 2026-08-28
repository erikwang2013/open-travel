// open-travel admin-service：班期（line dates）CRUD
//
// 全部挂在线路 id 之下，删除/更新以 (id, line_id) 双条件限定作用域。
// depart_date 冲突由 uk_line_date 唯一键兜底 → 409（前端直接展示 message）。
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use ecat_data::RdbmsClient;
use serde_json::{json, Value};

use super::handlers::{db_unavailable, not_found, pick};
use super::line_handlers::{date_from_row, fetch_date, valid_date, DATE_FIELDS, DATE_SELECT};
use super::{err, AdminGuard, ApiResponse, AppState};

/// 班期列表：返回裸数组（前端 LineDatesPage 直接按 List 消费）。
pub(crate) async fn list_line_dates(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(line_id): Path<u64>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let rows = match db
        .query_with("SELECT id FROM travel_lines WHERE id = ?", &[json!(line_id)])
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "line existence check failed");
            return db_unavailable();
        }
    };
    if rows.is_empty() {
        return not_found("line");
    }
    let sql = format!(
        "SELECT {} FROM travel_line_dates WHERE line_id = ? ORDER BY depart_date ASC, id ASC",
        DATE_SELECT.join(",")
    );
    let rows = match db.query_with(&sql, &[json!(line_id)]).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "line dates list query failed");
            return db_unavailable();
        }
    };
    let list: Vec<Value> = rows.iter().map(date_from_row).collect();
    ApiResponse::ok(list).into_response()
}

pub(crate) async fn create_line_date(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(line_id): Path<u64>,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    let Some(depart_date) = obj.get("depart_date").and_then(Value::as_str) else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "depart_date is required").into_response();
    };
    if !valid_date(depart_date) {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "depart_date must be YYYY-MM-DD").into_response();
    }
    if let Some(s) = obj.get("status").and_then(Value::as_i64) {
        if s != 0 && s != 1 {
            return err::<Value>(StatusCode::BAD_REQUEST, 400, "status must be 0 or 1").into_response();
        }
    }
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let rows = match db
        .query_with("SELECT id FROM travel_lines WHERE id = ?", &[json!(line_id)])
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "line existence check failed");
            return db_unavailable();
        }
    };
    if rows.is_empty() {
        return not_found("line");
    }
    let (mut cols, mut vals) = pick(obj, DATE_FIELDS);
    for (c, v) in [("price_cents", json!(0)), ("seats_left", json!(0)), ("status", json!(1))] {
        if !cols.iter().any(|x| x == c) {
            cols.push(c.into());
            vals.push(v);
        }
    }
    let mut insert_cols = vec!["line_id".to_string()];
    insert_cols.extend(cols);
    let mut insert_vals = vec![json!(line_id)];
    insert_vals.extend(vals);
    let sql = format!(
        "INSERT INTO travel_line_dates ({}) VALUES ({})",
        insert_cols.join(","),
        vec!["?"; insert_cols.len()].join(",")
    );
    // uk_line_date(line_id, depart_date) 唯一键兜底：重复日期 → 409
    if let Err(e) = db.execute_with(&sql, &insert_vals).await {
        if format!("{e}").contains("Duplicate") {
            return err::<Value>(StatusCode::CONFLICT, 409, "depart date already exists").into_response();
        }
        tracing::warn!(error = %e, "line date insert failed");
        return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
    }
    let rows = db
        .query_with(
            "SELECT id FROM travel_line_dates WHERE line_id = ? AND depart_date = ?",
            &[json!(line_id), json!(depart_date)],
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
    match fetch_date(&db, line_id, id).await {
        Some(d) => (StatusCode::OK, ApiResponse::ok(d)).into_response(),
        None => db_unavailable(),
    }
}

pub(crate) async fn update_line_date(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path((line_id, date_id)): Path<(u64, u64)>,
    Json(body): Json<Value>,
) -> Response {
    let Some(obj) = body.as_object() else {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "request body must be a JSON object").into_response();
    };
    if let Some(s) = obj.get("depart_date").and_then(Value::as_str) {
        if !valid_date(s) {
            return err::<Value>(StatusCode::BAD_REQUEST, 400, "depart_date must be YYYY-MM-DD").into_response();
        }
    }
    if let Some(s) = obj.get("status").and_then(Value::as_i64) {
        if s != 0 && s != 1 {
            return err::<Value>(StatusCode::BAD_REQUEST, 400, "status must be 0 or 1").into_response();
        }
    }
    let (cols, vals) = pick(obj, DATE_FIELDS);
    if cols.is_empty() {
        return err::<Value>(StatusCode::BAD_REQUEST, 400, "no fields to update").into_response();
    }
    let Some(db) = state.db.clone() else { return db_unavailable() };
    // 改日期时排除自身查重（uk_line_date 兜底）
    if let Some(d) = obj.get("depart_date").and_then(Value::as_str) {
        let rows = match db
            .query_with(
                "SELECT id FROM travel_line_dates WHERE line_id = ? AND depart_date = ? AND id <> ?",
                &[json!(line_id), json!(d), json!(date_id)],
            )
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!(error = %e, "line date conflict check failed");
                return db_unavailable();
            }
        };
        if !rows.is_empty() {
            return err::<Value>(StatusCode::CONFLICT, 409, "depart date already exists").into_response();
        }
    }
    let sets: Vec<String> = cols.iter().map(|c| format!("{c} = ?")).collect();
    let mut params = vals;
    params.push(json!(date_id));
    params.push(json!(line_id));
    let sql = format!("UPDATE travel_line_dates SET {} WHERE id = ? AND line_id = ?", sets.join(","));
    match db.execute_with(&sql, &params).await {
        Ok(0) => return not_found("line date"),
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, "line date update failed");
            return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
        }
    }
    match fetch_date(&db, line_id, date_id).await {
        Some(d) => (StatusCode::OK, ApiResponse::ok(d)).into_response(),
        None => db_unavailable(),
    }
}

pub(crate) async fn delete_line_date(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path((line_id, date_id)): Path<(u64, u64)>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    match db
        .execute_with(
            "DELETE FROM travel_line_dates WHERE id = ? AND line_id = ?",
            &[json!(date_id), json!(line_id)],
        )
        .await
    {
        Ok(0) => not_found("line date"),
        Ok(_) => (StatusCode::OK, ApiResponse::ok(Value::Null)).into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "line date delete failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}
