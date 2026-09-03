// open-travel admin-service：用户管理（列表 / 禁用）
//
// 列表绝不返回 password_hash（SELECT 显式列名排除）；禁用（status=1）后
// user-service 的 JWT 请求返回 403（见 user-service main.rs ensure_user_enabled）。
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use ecat_data::{Row, RdbmsClient};
use ecat_data_sqlx::SqlxClient;
use serde::Deserialize;
use serde_json::{json, Value};

use super::handlers::{db_unavailable, default_page, default_page_size, not_found, StatusReq};
use super::line_handlers::col_u64;
use super::{err, AdminGuard, ApiResponse, AppState};

// created_at DATETIME、status TINYINT CAST（sqlx Any 解码限制）
const USER_SELECT: &str = "id, email, lang, CAST(status AS SIGNED) AS status, \
    CAST(created_at AS CHAR) AS created_at";

#[derive(Deserialize)]
pub(crate) struct UsersQuery {
    #[serde(default = "default_page")]
    pub(crate) page: u64,
    #[serde(default = "default_page_size")]
    pub(crate) page_size: u64,
    pub(crate) keyword: Option<String>,
}

fn col_str(row: &Row, col: &str) -> String {
    row.get(col).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn user_from_row(row: &Row) -> Value {
    json!({
        "id": col_u64(row, "id"),
        "id_str": col_u64(row, "id").to_string(),
        "email": col_str(row, "email"),
        "lang": col_str(row, "lang"),
        "status": col_u64(row, "status"),
        "created_at": col_str(row, "created_at"),
    })
}

async fn fetch_user(db: &SqlxClient, id: u64) -> Option<Value> {
    let sql = format!("SELECT {USER_SELECT} FROM travel_users WHERE id = ?");
    let rows = db.query_with(&sql, &[json!(id)]).await.ok()?;
    rows.first().map(user_from_row)
}

/// 用户列表：keyword 匹配 email；绝不返回 password_hash。
pub(crate) async fn list_users(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Query(q): Query<UsersQuery>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let page = q.page.max(1);
    let page_size = q.page_size.clamp(1, 100);
    let mut cond = String::from(" WHERE 1=1");
    let mut params: Vec<Value> = Vec::new();
    if let Some(kw) = q.keyword.as_deref().filter(|k| !k.is_empty()) {
        cond.push_str(" AND email LIKE ?");
        params.push(json!(format!("%{kw}%")));
    }
    let total = match db
        .query_with(&format!("SELECT COUNT(*) AS total FROM travel_users{cond}"), &params)
        .await
    {
        Ok(rows) => rows
            .first()
            .and_then(|r| r.get("total"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as u64,
        Err(e) => {
            tracing::warn!(error = %e, "user count query failed");
            return db_unavailable();
        }
    };
    let mut list_params = params;
    list_params.push(json!(page_size));
    list_params.push(json!((page - 1) * page_size));
    let sql = format!(
        "SELECT {USER_SELECT} FROM travel_users{cond} ORDER BY id ASC LIMIT ? OFFSET ?"
    );
    let rows = match db.query_with(&sql, &list_params).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "user list query failed");
            return db_unavailable();
        }
    };
    let list: Vec<Value> = rows.iter().map(user_from_row).collect();
    ApiResponse::ok(json!({ "items": list, "total": total, "page": page, "page_size": page_size }))
        .into_response()
}

/// 禁用/启用用户：status 0 正常 / 1 禁用。
pub(crate) async fn update_user_status(
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
        .execute_with("UPDATE travel_users SET status = ? WHERE id = ?", &[json!(body.status), json!(id)])
        .await
    {
        Ok(0) => not_found("user"),
        Ok(_) => match fetch_user(&db, id).await {
            Some(u) => (StatusCode::OK, ApiResponse::ok(u)).into_response(),
            None => db_unavailable(),
        },
        Err(e) => {
            tracing::warn!(error = %e, "user status update failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}
