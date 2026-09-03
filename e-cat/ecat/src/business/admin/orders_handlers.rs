// open-travel admin-service：订单管理（列表 / 详情 / 退款回补库存）
//
// 退款（改退操作）：仅 status=1（已支付）/2（已确认）可退，其余 409。
// sqlx Any 事务无查询能力，采用「状态门闩」模式：先 UPDATE 订单状态
// （WHERE status IN (1,2)，并发退款仅一个 affected=1），再回补库存与
// 支付流水；回补失败仅告警（订单已置取消，不会重复退款）。
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use base64::Engine as _;
use ecat_data::{Row, RdbmsClient};
use ecat_data_sqlx::SqlxClient;
use serde::Deserialize;
use serde_json::{json, Value};

use super::handlers::{
    db_unavailable, default_page, default_page_size, not_found,
};
use super::line_handlers::col_u64;
use super::{err, AdminGuard, ApiResponse, AppState};

// DATETIME/TINYINT/TEXT 列 CAST：sqlx Any 驱动对 MySQL 时间/微小整数解码失败，
// 字符串化后 col_u64 解析（同 order-service）；TEXT 快照可能 base64 返回需兜底
const ORDER_SELECT: &str = "o.id, o.user_id, CAST(o.order_type AS CHAR) AS order_type, \
    o.product_id, CAST(o.status AS CHAR) AS status, o.amount_cents, o.product_snapshot, \
    CAST(o.created_at AS CHAR) AS created_at, CAST(o.expire_at AS CHAR) AS expire_at, u.email";
const PAYMENT_SELECT: &str = "id, channel_code, amount_cents, CAST(status AS SIGNED) AS status, \
    txn_no, CAST(paid_at AS CHAR) AS paid_at";

#[derive(Deserialize)]
pub(crate) struct OrdersQuery {
    #[serde(default = "default_page")]
    pub(crate) page: u64,
    #[serde(default = "default_page_size")]
    pub(crate) page_size: u64,
    pub(crate) status: Option<String>,
    pub(crate) keyword: Option<String>,
}

fn col_str(row: &Row, col: &str) -> String {
    row.get(col).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

/// 商品快照：sqlx Any 对 TEXT 列可能按 base64 返回，先尝试 base64 解码再解析 JSON
fn snapshot_value(row: &Row) -> Value {
    let raw = col_str(row, "product_snapshot");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_else(|| raw.clone());
    serde_json::from_str(&decoded).unwrap_or(Value::Null)
}

fn order_from_row(row: &Row) -> Value {
    let expire = col_str(row, "expire_at");
    json!({
        "id": col_u64(row, "id"),
        "id_str": col_u64(row, "id").to_string(),
        "user_id": col_u64(row, "user_id"),
        "email": col_str(row, "email"),
        "order_type": col_u64(row, "order_type"),
        "product_id": col_u64(row, "product_id"),
        "status": col_u64(row, "status"),
        "amount_cents": col_u64(row, "amount_cents"),
        "snapshot": snapshot_value(row),
        "created_at": col_str(row, "created_at"),
        "expire_at": if expire.is_empty() { Value::Null } else { json!(expire) },
    })
}

fn payment_from_row(row: &Row) -> Value {
    let paid_at = col_str(row, "paid_at");
    json!({
        "id": col_u64(row, "id"),
        "id_str": col_u64(row, "id").to_string(),
        "channel_code": col_str(row, "channel_code"),
        "amount_cents": col_u64(row, "amount_cents"),
        "status": col_u64(row, "status"),
        "txn_no": col_str(row, "txn_no"),
        "paid_at": if paid_at.is_empty() { Value::Null } else { json!(paid_at) },
    })
}

async fn fetch_order(db: &SqlxClient, id: u64) -> Option<Value> {
    let sql = format!(
        "SELECT {ORDER_SELECT} FROM travel_orders o \
         LEFT JOIN travel_users u ON u.id = o.user_id WHERE o.id = ?"
    );
    let rows = db.query_with(&sql, &[json!(id)]).await.ok()?;
    rows.first().map(order_from_row)
}

async fn fetch_payments(db: &SqlxClient, order_id: u64) -> Vec<Value> {
    let sql = format!("SELECT {PAYMENT_SELECT} FROM travel_payments WHERE order_id = ? ORDER BY id ASC");
    match db.query_with(&sql, &[json!(order_id)]).await {
        Ok(rows) => rows.iter().map(payment_from_row).collect(),
        Err(e) => {
            tracing::warn!(error = %e, "payments query failed");
            Vec::new()
        }
    }
}

/// 订单列表：keyword 匹配订单 id 或用户 email，status 可选过滤（0-4）。
pub(crate) async fn list_orders(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Query(q): Query<OrdersQuery>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let page = q.page.max(1);
    let page_size = q.page_size.clamp(1, 100);
    let mut cond = String::from(" WHERE 1=1");
    let mut params: Vec<Value> = Vec::new();
    if let Some(s) = q.status.as_deref().filter(|s| !s.is_empty()) {
        let Ok(status) = s.parse::<i64>() else {
            return err::<Value>(StatusCode::BAD_REQUEST, 400, "status must be 0-4").into_response();
        };
        if !(0..=4).contains(&status) {
            return err::<Value>(StatusCode::BAD_REQUEST, 400, "status must be 0-4").into_response();
        }
        cond.push_str(" AND o.status = ?");
        params.push(json!(status));
    }
    if let Some(kw) = q.keyword.as_deref().filter(|k| !k.is_empty()) {
        cond.push_str(" AND (CAST(o.id AS CHAR) LIKE ? OR u.email LIKE ?)");
        params.push(json!(format!("%{kw}%")));
        params.push(json!(format!("%{kw}%")));
    }
    let join = " FROM travel_orders o LEFT JOIN travel_users u ON u.id = o.user_id";
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
            tracing::warn!(error = %e, "order count query failed");
            return db_unavailable();
        }
    };
    let mut list_params = params;
    list_params.push(json!(page_size));
    list_params.push(json!((page - 1) * page_size));
    let sql = format!(
        "SELECT {ORDER_SELECT}{join}{cond} ORDER BY o.id DESC LIMIT ? OFFSET ?"
    );
    let rows = match db.query_with(&sql, &list_params).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "order list query failed");
            return db_unavailable();
        }
    };
    let list: Vec<Value> = rows.iter().map(order_from_row).collect();
    ApiResponse::ok(json!({ "items": list, "total": total, "page": page, "page_size": page_size }))
        .into_response()
}

/// 订单详情：订单对象 + 支付流水数组。
pub(crate) async fn order_detail(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(id): Path<u64>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let Some(order) = fetch_order(&db, id).await else {
        return not_found("order");
    };
    let payments = fetch_payments(&db, id).await;
    let mut m = order.as_object().cloned().unwrap_or_default();
    m.insert("payments".into(), json!(payments));
    ApiResponse::ok(Value::Object(m)).into_response()
}

/// 退款：状态门闩（UPDATE status IN (1,2)）→ 回补库存 → 支付流水置已退款。
pub(crate) async fn refund_order(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(id): Path<u64>,
) -> Response {
    let Some(db) = state.db.clone() else { return db_unavailable() };
    let sql = format!(
        "SELECT {ORDER_SELECT} FROM travel_orders o \
         LEFT JOIN travel_users u ON u.id = o.user_id WHERE o.id = ?"
    );
    let rows = match db.query_with(&sql, &[json!(id)]).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "refund order query failed");
            return db_unavailable();
        }
    };
    let Some(row) = rows.first() else {
        return not_found("order");
    };
    let status = col_u64(row, "status");
    if status != 1 && status != 2 {
        return err::<Value>(
            StatusCode::CONFLICT,
            409,
            "only paid or confirmed orders can be refunded",
        )
        .into_response();
    }
    // 状态门闩：并发退款仅一个 affected=1；0 行说明已被并发请求抢先
    let affected = match db
        .execute_with(
            "UPDATE travel_orders SET status = 4 WHERE id = ? AND status IN (1, 2)",
            &[json!(id)],
        )
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(error = %e, "refund status update failed");
            return err::<Value>(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error").into_response();
        }
    };
    if affected == 0 {
        return err::<Value>(
            StatusCode::CONFLICT,
            409,
            "only paid or confirmed orders can be refunded",
        )
        .into_response();
    }
    // 回补库存（失败仅告警：订单已置取消，不会重复退款）
    restore_inventory(&db, row).await;
    // 支付流水置已退款（幂等：仅 status=1 的记录）
    if let Err(e) = db
        .execute_with(
            "UPDATE travel_payments SET status = 3 WHERE order_id = ? AND status = 1",
            &[json!(id)],
        )
        .await
    {
        tracing::warn!(error = %e, "refund payment update failed");
    }
    match fetch_order(&db, id).await {
        Some(o) => (StatusCode::OK, ApiResponse::ok(o)).into_response(),
        None => db_unavailable(),
    }
}

/// 按 order_type 回补对应库存表：1 线路（snapshot.line_date_id）→ 余位；
/// 2 机票（product_id 即航班行 id）→ 余票；3 酒店（product_id 即房型 id）→ 库存。
async fn restore_inventory(db: &SqlxClient, row: &Row) {
    let order_type = col_u64(row, "order_type");
    let snap = snapshot_value(row);
    let qty = snap.get("quantity").and_then(|v| v.as_u64()).unwrap_or(0);
    if qty == 0 {
        return;
    }
    let product_id = col_u64(row, "product_id");
    // ponytail: 酒店房型 id 直接取 product_id（快照无 room_id 字段），
    // 若订单快照后续改存 room_id 需在此加回退读取
    let (sql, stock_id) = match order_type {
        1 => (
            "UPDATE travel_line_dates SET seats_left = seats_left + ? WHERE id = ?",
            snap.get("line_date_id").and_then(|v| v.as_u64()).unwrap_or(0),
        ),
        2 => ("UPDATE travel_flights SET seats_left = seats_left + ? WHERE id = ?", product_id),
        3 => ("UPDATE travel_hotel_rooms SET inventory = inventory + ? WHERE id = ?", product_id),
        _ => return,
    };
    if stock_id == 0 {
        tracing::warn!(order_type, "refund stock restore skipped: stock id missing");
        return;
    }
    if let Err(e) = db.execute_with(sql, &[json!(qty), json!(stock_id)]).await {
        tracing::warn!(error = %e, order_type, "refund stock restore failed");
    }
}
