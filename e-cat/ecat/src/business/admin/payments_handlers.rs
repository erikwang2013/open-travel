// open-travel admin-service：支付管理（流水账单 / 渠道开关）
//
// P4-16：流水列表支持 channel/status 筛选 + 分页；渠道列表全量返回
// （含禁用项，供管理端展示开关）；enabled 切换即时生效（payment-service
// 路由渠道时实时读表）。
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::Engine as _;
use ecat_data::{Row, RdbmsClient};
use serde::Deserialize;
use serde_json::{json, Value};

use super::handlers::{db_unavailable, default_page, default_page_size, not_found};
use super::line_handlers::col_u64;
use super::{err, AdminGuard, ApiResponse, AppState};

// DATETIME/TINYINT/TEXT 列 CAST（sqlx Any 解码限制，同 orders_handlers）
const PAYMENT_SELECT: &str = "p.id, p.order_id, p.channel_code, p.amount_cents, \
    CAST(p.status AS SIGNED) AS status, p.txn_no, CAST(p.created_at AS CHAR) AS created_at, \
    CAST(p.paid_at AS CHAR) AS paid_at, u.email";
const CHANNEL_SELECT: &str = "channel_code, name, CAST(type AS SIGNED) AS type, \
    CAST(enabled AS SIGNED) AS enabled, CAST(priority AS SIGNED) AS priority, \
    languages, countries";

#[derive(Deserialize)]
pub(crate) struct PaymentsQuery {
    #[serde(default = "default_page")]
    pub(crate) page: u64,
    #[serde(default = "default_page_size")]
    pub(crate) page_size: u64,
    pub(crate) channel: Option<String>,
    pub(crate) status: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct EnabledReq {
    pub(crate) enabled: bool,
}

fn col_str(row: &Row, col: &str) -> String {
    row.get(col).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

/// 渠道名多语言 JSON：sqlx Any 可能按 base64 返回 TEXT，解码兜底后解析
fn channel_name(row: &Row) -> Value {
    let raw = col_str(row, "name");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_else(|| raw.clone());
    serde_json::from_str(&decoded).unwrap_or(Value::Null)
}

fn payment_from_row(row: &Row) -> Value {
    let paid_at = col_str(row, "paid_at");
    json!({
        "id": col_u64(row, "id"),
        "id_str": col_u64(row, "id").to_string(),
        "order_id": col_u64(row, "order_id"),
        "email": col_str(row, "email"),
        "channel_code": col_str(row, "channel_code"),
        "amount_cents": col_u64(row, "amount_cents"),
        "status": col_u64(row, "status"),
        "txn_no": col_str(row, "txn_no"),
        "created_at": col_str(row, "created_at"),
        "paid_at": if paid_at.is_empty() { Value::Null } else { json!(paid_at) },
    })
}

/// 流水列表：channel_code 精确匹配 + status 0-3 过滤，按 id 倒序分页。
pub(crate) async fn list_payments(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Query(q): Query<PaymentsQuery>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let page = q.page.max(1);
    let page_size = q.page_size.clamp(1, 100);
    let mut cond = String::from(" WHERE 1=1");
    let mut params: Vec<Value> = Vec::new();
    if let Some(c) = q.channel.as_deref().filter(|c| !c.is_empty()) {
        cond.push_str(" AND p.channel_code = ?");
        params.push(json!(c));
    }
    if let Some(s) = q.status.as_deref().filter(|s| !s.is_empty()) {
        let Ok(status) = s.parse::<i64>() else {
            return err::<Value>(StatusCode::BAD_REQUEST, 400, "status must be 0-3").into_response();
        };
        if !(0..=3).contains(&status) {
            return err::<Value>(StatusCode::BAD_REQUEST, 400, "status must be 0-3").into_response();
        }
        cond.push_str(" AND p.status = ?");
        params.push(json!(status));
    }
    let join = " FROM travel_payments p LEFT JOIN travel_orders o ON o.id = p.order_id \
                LEFT JOIN travel_users u ON u.id = o.user_id";
    let total = match db
        .query_with(&format!("SELECT COUNT(*) AS total{join}{cond}"), &params)
        .await
    {
        Ok(rows) => rows
            .first()
            .and_then(|r| r.get("total"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as u64,
        Err(e) => {
            tracing::warn!(error = %e, "payment count query failed");
            return db_unavailable();
        }
    };
    let mut list_params = params;
    list_params.push(json!(page_size));
    list_params.push(json!((page - 1) * page_size));
    let sql = format!(
        "SELECT {PAYMENT_SELECT}{join}{cond} ORDER BY p.id DESC LIMIT ? OFFSET ?"
    );
    let rows = match db.query_with(&sql, &list_params).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "payment list query failed");
            return db_unavailable();
        }
    };
    let list: Vec<Value> = rows.iter().map(payment_from_row).collect();
    ApiResponse::ok(json!({ "items": list, "total": total, "page": page, "page_size": page_size }))
        .into_response()
}

fn channel_from_row(row: &Row) -> Value {
    json!({
        "channel_code": col_str(row, "channel_code"),
        "name": channel_name(row),
        "type": col_u64(row, "type"),
        "enabled": col_u64(row, "enabled") == 1,
        "priority": col_u64(row, "priority"),
        "languages": col_str(row, "languages"),
        "countries": col_str(row, "countries"),
    })
}

/// 渠道列表：全量返回（含禁用项），按 priority 升序。
pub(crate) async fn list_channels(
    State(state): State<AppState>,
    _guard: AdminGuard,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let rows = match db.query(&format!("SELECT {CHANNEL_SELECT} FROM travel_payment_channels ORDER BY priority ASC")).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "channel list query failed");
            return db_unavailable();
        }
    };
    let list: Vec<Value> = rows.iter().map(channel_from_row).collect();
    ApiResponse::ok(json!({ "items": list })).into_response()
}

/// 渠道开关：enabled 0 关闭 / 1 开启，即时生效。
pub(crate) async fn update_channel_enabled(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(code): Path<String>,
    Json(body): Json<EnabledReq>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    match db
        .execute_with(
            "UPDATE travel_payment_channels SET enabled = ? WHERE channel_code = ?",
            &[json!(if body.enabled { 1 } else { 0 }), json!(code)],
        )
        .await
    {
        Ok(0) => not_found("channel"),
        Ok(_) => {
            let rows = db
                .query_with(
                    &format!("SELECT {CHANNEL_SELECT} FROM travel_payment_channels WHERE channel_code = ?"),
                    &[json!(code)],
                )
                .await
                .unwrap_or_default();
            match rows.first() {
                Some(row) => (StatusCode::OK, ApiResponse::ok(channel_from_row(row))).into_response(),
                None => db_unavailable(),
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "channel update failed");
            err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response()
        }
    }
}
