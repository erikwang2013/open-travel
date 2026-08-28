// order-service 业务 handlers：下单 / 列表 / 详情 / 取消 + 库存预占与回补。
use super::*;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use ecat_data::{Cache, RdbmsClient};
use serde_json::json;

// DATETIME/DATE/TINYINT 列 CAST AS CHAR：sqlx Any 对 MySQL 时间类型
// 及 TINYINT（order_type/status）解码失败，CAST 后 col_u64 解析字符串
const ORDER_COLS: &str = "id, user_id, CAST(order_type AS CHAR) AS order_type, product_id, \
    CAST(status AS CHAR) AS status, amount_cents, \
    product_snapshot, CAST(expire_at AS CHAR) AS expire_at, \
    CAST(created_at AS CHAR) AS created_at";

fn col_u64(row: &ecat_data::Row, col: &str) -> u64 {
    row.get(col)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0)
}

fn col_str(row: &ecat_data::Row, col: &str) -> String {
    row.get(col).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

/// 商品快照：sqlx Any 对 TEXT 列可能按 base64 返回，先尝试 base64 解码再解析
/// JSON，非 base64 直接解析（同 booking pick_desc 模式）。
fn snapshot_value(row: &ecat_data::Row) -> serde_json::Value {
    use base64::Engine as _;
    let raw = col_str(row, "product_snapshot");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_else(|| raw.clone());
    serde_json::from_str(&decoded).unwrap_or(serde_json::Value::Null)
}

fn order_from_row(row: &ecat_data::Row) -> OrderOut {
    OrderOut {
        id: col_u64(row, "id"),
        order_type: col_u64(row, "order_type") as u8,
        product_id: col_u64(row, "product_id"),
        status: col_u64(row, "status") as u8,
        amount_cents: col_u64(row, "amount_cents"),
        snapshot: snapshot_value(row),
        expire_at: {
            let s = col_str(row, "expire_at");
            if s.is_empty() { None } else { Some(s) }
        },
        created_at: col_str(row, "created_at"),
    }
}

fn stock_key(line_date_id: u64) -> String {
    format!("travel:stock:{line_date_id}")
}

/// Redis 预占：key 不存在时用 DB 当前余位初始化，再 DECR 原子扣减；
/// 结果为负回滚（INCR 补回）返回 false 由调用方转 409。
/// ponytail: 初始化用 SET 非 SETNX，并发首次预占可能覆盖一次扣减导致 Redis
/// 计数偏大；DB 原子扣减（WHERE seats_left >= ?）是最终兜底，不会超卖，
/// 且 key 过期（1 天）后会按 DB 余位自愈。
async fn pre_reserve(cache: &RedisCache, line_date_id: u64, qty: u64, db_seats: u64) -> bool {
    let key = stock_key(line_date_id);
    let empty = cache.get(&key).await.map(|v| v.is_none()).unwrap_or(true);
    if empty {
        if let Err(e) = cache
            .set(&key, db_seats.to_string().as_bytes(), Duration::from_secs(STOCK_TTL_SECS))
            .await
        {
            tracing::warn!(error = %e, "stock key init failed, rely on db gate");
            return true;
        }
    }
    match cache.increment(&key, -(qty as i64)).await {
        Ok(n) if n >= 0 => true,
        Ok(_) => {
            let _ = cache.increment(&key, qty as i64).await;
            false
        }
        Err(e) => {
            // fail-open：Redis 故障时 DB 原子扣减兜底
            tracing::warn!(error = %e, "stock decrement failed, rely on db gate");
            true
        }
    }
}

/// 回补余位（取消/超时）：DB +quantity，Redis INCR +quantity。
async fn restore_stock(state: &AppState, line_date_id: u64, qty: u64) {
    if line_date_id == 0 || qty == 0 {
        return;
    }
    if let Some(db) = &state.db {
        if let Err(e) = db
            .execute_with(
                "UPDATE travel_line_dates SET seats_left = seats_left + ? WHERE id = ?",
                &[json!(qty), json!(line_date_id)],
            )
            .await
        {
            tracing::warn!(error = %e, "stock restore failed");
        }
    }
    if let Some(cache) = &state.cache {
        if let Err(e) = cache.increment(&stock_key(line_date_id), qty as i64).await {
            tracing::warn!(error = %e, "stock redis restore failed");
        }
    }
}

/// 过期订单惰性清理：expire_at 已过且仍待支付 → 置 4 并回补余位。
/// 列表/详情前调用 + 启动后台任务每 60s 一轮。
pub(crate) async fn expire_pending_orders(state: &AppState) {
    let Some(db) = state.db.clone() else { return };
    let rows = match db
        .query_with(
            "SELECT id, product_snapshot FROM travel_orders \
             WHERE status = 0 AND expire_at < NOW() LIMIT 100",
            &[],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "expiry sweep query failed");
            return;
        }
    };
    for row in rows {
        let id = col_u64(&row, "id");
        let snap = snapshot_value(&row);
        restore_stock(
            state,
            snap.get("line_date_id").and_then(|v| v.as_u64()).unwrap_or(0),
            snap.get("quantity").and_then(|v| v.as_u64()).unwrap_or(0),
        )
        .await;
        if let Err(e) = db
            .execute_with(
                "UPDATE travel_orders SET status = 4 WHERE id = ? AND status = 0",
                &[json!(id)],
            )
            .await
        {
            tracing::warn!(error = %e, "expiry sweep update failed");
        }
    }
}

/// P3-08 下单：Redis 预占 → DB 原子扣减 → 插入订单（补偿顺序，见 main.rs 头注释）。
pub(crate) async fn create_order(
    State(state): State<AppState>,
    UserGuard(user_id): UserGuard,
    Json(body): Json<CreateOrderReq>,
) -> (StatusCode, Json<ApiResponse<OrderOut>>) {
    if body.order_type != 1 {
        let status = if body.order_type == 2 || body.order_type == 3 {
            StatusCode::NOT_IMPLEMENTED
        } else {
            StatusCode::BAD_REQUEST
        };
        return err(status, status.as_u16().into(), "only order_type=1 (line) supported yet");
    }
    if body.quantity == 0 || body.quantity > 99 {
        return err(StatusCode::BAD_REQUEST, 400, "quantity must be 1-99");
    }
    let Some(db) = state.db.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    };

    // 1. 班期校验（可售 + 未出发），取价格与余位
    let rows = match db
        .query_with(
            "SELECT ld.id, ld.line_id, ld.price_cents, ld.seats_left, \
             CAST(ld.depart_date AS CHAR) AS depart_date \
             FROM travel_line_dates ld \
             WHERE ld.id = ? AND ld.status = 1 AND ld.depart_date >= CURDATE()",
            &[json!(body.line_date_id)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "line date query failed");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    let Some(ld_row) = rows.first() else {
        return err(StatusCode::NOT_FOUND, 404, "line date not available");
    };
    let ld_line_id = col_u64(ld_row, "line_id");
    let price_cents = col_u64(ld_row, "price_cents");
    let db_seats = col_u64(ld_row, "seats_left");
    let depart_date = col_str(ld_row, "depart_date");
    if ld_line_id != body.product_id {
        return err(StatusCode::BAD_REQUEST, 400, "line_date_id does not match product_id");
    }

    // 2. 线路标题（快照用，zh 为空回退 en）
    let lines = match db
        .query_with(
            "SELECT destination_id, title_zh, title_en FROM travel_lines \
             WHERE id = ? AND status = 1",
            &[json!(body.product_id)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "line query failed");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    let Some(line_row) = lines.first() else {
        return err(StatusCode::NOT_FOUND, 404, "line not found");
    };
    let destination_id = col_u64(line_row, "destination_id");
    let title_zh = col_str(line_row, "title_zh");
    let title_en = col_str(line_row, "title_en");
    let title = if title_zh.is_empty() { title_en } else { title_zh };

    // 3. Redis 预占（cache 缺失/故障时 fail-open，DB 兜底）
    if let Some(cache) = &state.cache {
        if !pre_reserve(cache, body.line_date_id, body.quantity, db_seats).await {
            return err(StatusCode::CONFLICT, 409, "insufficient stock");
        }
    }

    // 4. DB 原子扣减（真实防线）；失败回滚 Redis 预占
    let affected = match db
        .execute_with(
            "UPDATE travel_line_dates SET seats_left = seats_left - ? \
             WHERE id = ? AND seats_left >= ? AND status = 1",
            &[json!(body.quantity), json!(body.line_date_id), json!(body.quantity)],
        )
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(error = %e, "stock decrement failed");
            if let Some(cache) = &state.cache {
                let _ = cache.increment(&stock_key(body.line_date_id), body.quantity as i64).await;
            }
            return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
        }
    };
    if affected == 0 {
        if let Some(cache) = &state.cache {
            let _ = cache.increment(&stock_key(body.line_date_id), body.quantity as i64).await;
        }
        return err(StatusCode::CONFLICT, 409, "insufficient stock");
    }

    // 5. 插入订单：amount = price × qty，快照含 quantity/line_date_id 供取消/超时回补
    let snapshot = json!({
        "title": title,
        "price_cents": price_cents,
        "depart_date": depart_date,
        "quantity": body.quantity,
        "line_date_id": body.line_date_id,
        "order_type": 1,
    });
    let insert = db
        .execute_with(
            "INSERT INTO travel_orders \
             (user_id, order_type, product_id, product_snapshot, destination_id, \
              booking_id, status, amount_cents, expire_at) \
             VALUES (?, ?, ?, ?, ?, 0, 0, ?, DATE_ADD(NOW(), INTERVAL 15 MINUTE))",
            &[
                json!(user_id),
                json!(body.order_type),
                json!(body.product_id),
                json!(snapshot.to_string()),
                json!(destination_id),
                json!(price_cents * body.quantity),
            ],
        )
        .await;
    if let Err(e) = insert {
        // 补偿：扣减已发生但订单未落库 → 回补 DB 与 Redis，避免少卖
        tracing::warn!(error = %e, "order insert failed, compensating stock");
        restore_stock(&state, body.line_date_id, body.quantity).await;
        return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
    }

    // 6. 读回订单（last insert 行）
    let fetched = match db
        .query_with(
            &format!("SELECT {ORDER_COLS} FROM travel_orders WHERE user_id = ? ORDER BY id DESC LIMIT 1"),
            &[json!(user_id)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "order readback failed");
            return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
        }
    };
    let Some(row) = fetched.first() else {
        return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
    };
    let out = order_from_row(row);
    if let Some(mq) = &state.mq {
        publish_audit(
            mq,
            "order.created",
            user_id,
            json!({ "order_id": out.id, "product_id": body.product_id, "quantity": body.quantity }),
        )
        .await;
    }
    (StatusCode::OK, ApiResponse::ok(out))
}

/// P3-09 订单列表：当前用户订单倒序；先跑惰性过期扫描。
pub(crate) async fn list_orders(
    State(state): State<AppState>,
    UserGuard(user_id): UserGuard,
    Query(q): Query<ListQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<OrderOut>>>) {
    expire_pending_orders(&state).await;
    let Some(db) = state.db.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    };
    let page = q.page.max(1);
    let page_size = q.page_size.clamp(1, 50);
    let offset = (page - 1) * page_size;
    let rows = match db
        .query_with(
            &format!(
                "SELECT {ORDER_COLS} FROM travel_orders WHERE user_id = ? \
                 ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?"
            ),
            &[json!(user_id), json!(page_size), json!(offset)],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "order list query failed");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    (StatusCode::OK, ApiResponse::ok(rows.iter().map(order_from_row).collect()))
}

/// 查本人订单（详情/取消共用）：非本人或不存在 → None。
async fn fetch_own_order(db: &SqlxClient, user_id: u64, id: u64) -> Option<ecat_data::Row> {
    let rows = db
        .query_with(
            &format!("SELECT {ORDER_COLS} FROM travel_orders WHERE id = ? AND user_id = ?"),
            &[json!(id), json!(user_id)],
        )
        .await
        .ok()?;
    rows.into_iter().next()
}

/// P3-09 订单详情（仅本人）。
pub(crate) async fn order_detail(
    State(state): State<AppState>,
    UserGuard(user_id): UserGuard,
    Path(id): Path<u64>,
) -> (StatusCode, Json<ApiResponse<OrderOut>>) {
    expire_pending_orders(&state).await;
    let Some(db) = state.db.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    };
    match fetch_own_order(&db, user_id, id).await {
        Some(row) => (StatusCode::OK, ApiResponse::ok(order_from_row(&row))),
        None => err(StatusCode::NOT_FOUND, 404, "order not found"),
    }
}

/// P3-09 取消订单：仅 status=0 可取消（0→4），回补余位 + Redis 预占。
pub(crate) async fn cancel_order(
    State(state): State<AppState>,
    UserGuard(user_id): UserGuard,
    Path(id): Path<u64>,
) -> (StatusCode, Json<ApiResponse<OrderOut>>) {
    let Some(db) = state.db.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    };
    let Some(row) = fetch_own_order(&db, user_id, id).await else {
        return err(StatusCode::NOT_FOUND, 404, "order not found");
    };
    if col_u64(&row, "status") != 0 {
        return err(StatusCode::BAD_REQUEST, 400, "only pending orders can be cancelled");
    }
    let snap = snapshot_value(&row);
    let line_date_id = snap.get("line_date_id").and_then(|v| v.as_u64()).unwrap_or(0);
    let qty = snap.get("quantity").and_then(|v| v.as_u64()).unwrap_or(0);

    restore_stock(&state, line_date_id, qty).await;
    let affected = match db
        .execute_with(
            "UPDATE travel_orders SET status = 4 WHERE id = ? AND status = 0",
            &[json!(id)],
        )
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(error = %e, "order cancel update failed");
            return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
        }
    };
    if affected == 0 {
        // 并发下已被其他请求取消（回补已重复执行，UPDATE + 幂等无害）
        return err(StatusCode::BAD_REQUEST, 400, "order already cancelled");
    }
    let mut out = order_from_row(&row);
    out.status = 4;
    if let Some(mq) = &state.mq {
        publish_audit(mq, "order.cancelled", user_id, json!({ "order_id": out.id })).await;
    }
    (StatusCode::OK, ApiResponse::ok(out))
}
