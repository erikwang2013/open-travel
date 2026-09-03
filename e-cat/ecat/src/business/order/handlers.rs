// order-service 业务 handlers：下单 / 列表 / 详情 / 取消 + 库存预占与回补。
use super::*;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
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

// 库存预占 key 按商品类型隔离（1 线路班期 / 2 航班 / 3 酒店房型），
// 避免不同类型商品 id 相同导致 Redis 预占互相覆盖
fn stock_key(order_type: u8, product_id: u64) -> String {
    format!("travel:stock:{order_type}:{product_id}")
}

/// Redis 预占：key 不存在时用 DB 当前余位初始化，再 DECR 原子扣减；
/// 结果为负回滚（INCR 补回）返回 false 由调用方转 409。
/// ponytail: 初始化用 SET 非 SETNX，并发首次预占可能覆盖一次扣减导致 Redis
/// 计数偏大；DB 原子扣减（WHERE seats_left >= ?）是最终兜底，不会超卖，
/// 且 key 过期（1 天）后会按 DB 余位自愈。
async fn pre_reserve(cache: &RedisCache, order_type: u8, product_id: u64, qty: u64, db_seats: u64) -> bool {
    let key = stock_key(order_type, product_id);
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

/// 回补库存（取消/超时）：按商品类型回补对应表 + Redis 预占。
async fn restore_stock(state: &AppState, order_type: u8, product_id: u64, qty: u64) {
    if product_id == 0 || qty == 0 {
        return;
    }
    let sql = match order_type {
        2 => "UPDATE travel_flights SET seats_left = seats_left + ? WHERE id = ?",
        3 => "UPDATE travel_hotel_rooms SET inventory = inventory + ? WHERE id = ?",
        _ => "UPDATE travel_line_dates SET seats_left = seats_left + ? WHERE id = ?",
    };
    if let Some(db) = &state.db {
        if let Err(e) = db.execute_with(sql, &[json!(qty), json!(product_id)]).await {
            tracing::warn!(error = %e, "stock restore failed");
        }
    }
    if let Some(cache) = &state.cache {
        if let Err(e) = cache.increment(&stock_key(order_type, product_id), qty as i64).await {
            tracing::warn!(error = %e, "stock redis restore failed");
        }
    }
}

/// 快照中的商品类型（缺省按 1 线路处理，兼容旧快照）。
fn snapshot_order_type(snap: &serde_json::Value) -> u8 {
    snap.get("order_type").and_then(|v| v.as_u64()).unwrap_or(1) as u8
}

/// 过期订单惰性清理：expire_at 已过且仍待支付 → 置 4 并回补余位。
/// 列表/详情前调用 + 启动后台任务每 60s 一轮。
pub(crate) async fn expire_pending_orders(state: &AppState) {
    let Some(db) = state.db.clone() else { return };
    let rows = match db
        .query_with(
            "SELECT id, product_id, product_snapshot FROM travel_orders \
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
        let ot = snapshot_order_type(&snap);
        let pid = if ot == 1 {
            snap.get("line_date_id").and_then(|v| v.as_u64()).unwrap_or(0)
        } else {
            col_u64(&row, "product_id")
        };
        restore_stock(state, ot, pid, snap.get("quantity").and_then(|v| v.as_u64()).unwrap_or(0)).await;
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

/// P3-08/P4-12 下单：Redis 预占 → DB 原子扣减 → 插入订单（补偿顺序，见 main.rs 头注释）。
/// order_type 1 线路（line_date_id 必填）/ 2 航班（product_id=航班 id）/ 3 酒店（product_id=房型 id）。
pub(crate) async fn create_order(
    State(state): State<AppState>,
    UserGuard(user_id): UserGuard,
    Json(body): Json<CreateOrderReq>,
) -> (StatusCode, Json<ApiResponse<OrderOut>>) {
    if !(1..=3).contains(&body.order_type) {
        return err(StatusCode::BAD_REQUEST, 400, "order_type must be 1-3");
    }
    if body.quantity == 0 || body.quantity > 99 {
        return err(StatusCode::BAD_REQUEST, 400, "quantity must be 1-99");
    }
    let Some(db) = state.db.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    };
    // 库存行 id：线路按班期（line_date_id）扣减，航班/酒店按商品行（product_id）
    let stock_id = if body.order_type == 1 { body.line_date_id } else { body.product_id };

    // 1. 商品校验（可售 + 未出发），取价格与库存；快照按类型组装
    let (price_cents, db_stock, destination_id, snapshot) = match body.order_type {
        // 线路：line_date_id 与 product_id（线路 id）双校验
        1 => {
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
            if col_u64(ld_row, "line_id") != body.product_id {
                return err(StatusCode::BAD_REQUEST, 400, "line_date_id does not match product_id");
            }
            let price_cents = col_u64(ld_row, "price_cents");
            let db_stock = col_u64(ld_row, "seats_left");
            let depart_date = col_str(ld_row, "depart_date");
            let lines = match db
                .query_with(
                    "SELECT title_zh, title_en, destination_id FROM travel_lines \
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
            let title_zh = col_str(line_row, "title_zh");
            let title = if title_zh.is_empty() { col_str(line_row, "title_en") } else { title_zh };
            (
                price_cents,
                db_stock,
                col_u64(line_row, "destination_id"),
                json!({
                    "title": title,
                    "price_cents": price_cents,
                    "depart_date": depart_date,
                    "quantity": body.quantity,
                    "line_date_id": body.line_date_id,
                    "order_type": 1,
                }),
            )
        }
        // 航班：product_id 即航班行 id，余票不足/已起飞拒售
        2 => {
            let rows = match db
                .query_with(
                    "SELECT airline, flight_no, from_code, to_code, price_cents, seats_left, \
                     CAST(depart_at AS CHAR) AS depart_at \
                     FROM travel_flights \
                     WHERE id = ? AND status = 1 AND depart_at > NOW()",
                    &[json!(body.product_id)],
                )
                .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::warn!(error = %e, "flight query failed");
                    return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
                }
            };
            let Some(f_row) = rows.first() else {
                return err(StatusCode::NOT_FOUND, 404, "flight not available");
            };
            let price_cents = col_u64(f_row, "price_cents");
            let db_stock = col_u64(f_row, "seats_left");
            let title = format!(
                "{} {} {}→{}",
                col_str(f_row, "airline"),
                col_str(f_row, "flight_no"),
                col_str(f_row, "from_code"),
                col_str(f_row, "to_code")
            );
            (
                price_cents,
                db_stock,
                0,
                json!({
                    "title": title,
                    "price_cents": price_cents,
                    "depart_at": col_str(f_row, "depart_at"),
                    "quantity": body.quantity,
                    "order_type": 2,
                }),
            )
        }
        // 酒店：product_id 即房型 id，快照含酒店名 + 房型 + 入住/离店
        3 => {
            let rows = match db
                .query_with(
                    "SELECT r.room_type_zh, r.room_type_en, r.price_cents, r.inventory, \
                     h.name_zh, h.name_en \
                     FROM travel_hotel_rooms r \
                     JOIN travel_hotels h ON h.id = r.hotel_id \
                     WHERE r.id = ? AND r.status = 1 AND h.status = 1",
                    &[json!(body.product_id)],
                )
                .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::warn!(error = %e, "room query failed");
                    return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
                }
            };
            let Some(r_row) = rows.first() else {
                return err(StatusCode::NOT_FOUND, 404, "room not available");
            };
            let price_cents = col_u64(r_row, "price_cents");
            let db_stock = col_u64(r_row, "inventory");
            let hotel_zh = col_str(r_row, "name_zh");
            let hotel = if hotel_zh.is_empty() { col_str(r_row, "name_en") } else { hotel_zh };
            let room_zh = col_str(r_row, "room_type_zh");
            let room = if room_zh.is_empty() { col_str(r_row, "room_type_en") } else { room_zh };
            (
                price_cents,
                db_stock,
                0,
                json!({
                    "title": format!("{hotel} - {room}"),
                    "price_cents": price_cents,
                    "quantity": body.quantity,
                    "check_in": body.check_in,
                    "check_out": body.check_out,
                    "order_type": 3,
                }),
            )
        }
        _ => unreachable!(),
    };

    // 2. Redis 预占（cache 缺失/故障时 fail-open，DB 兜底）
    if let Some(cache) = &state.cache {
        if !pre_reserve(cache, body.order_type, stock_id, body.quantity, db_stock).await {
            return err(StatusCode::CONFLICT, 409, "insufficient stock");
        }
    }

    // 3. DB 原子扣减（真实防线）；失败回滚 Redis 预占
    let stock_sql = match body.order_type {
        2 => "UPDATE travel_flights SET seats_left = seats_left - ? \
              WHERE id = ? AND seats_left >= ? AND status = 1",
        3 => "UPDATE travel_hotel_rooms SET inventory = inventory - ? \
              WHERE id = ? AND inventory >= ? AND status = 1",
        _ => "UPDATE travel_line_dates SET seats_left = seats_left - ? \
              WHERE id = ? AND seats_left >= ? AND status = 1",
    };
    let affected = match db
        .execute_with(
            stock_sql,
            &[json!(body.quantity), json!(stock_id), json!(body.quantity)],
        )
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(error = %e, "stock decrement failed");
            if let Some(cache) = &state.cache {
                let _ = cache.increment(&stock_key(body.order_type, stock_id), body.quantity as i64).await;
            }
            return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
        }
    };
    if affected == 0 {
        if let Some(cache) = &state.cache {
            let _ = cache.increment(&stock_key(body.order_type, stock_id), body.quantity as i64).await;
        }
        return err(StatusCode::CONFLICT, 409, "insufficient stock");
    }

    // 4. 插入订单：amount = price × qty，destination_id 仅线路有意义（航班/酒店记 0）。
    //    主键去 AUTO_INCREMENT 后显式生成雪花 id
    let order_id = idgen_rs::id_helper::next_id();
    let insert = db
        .execute_with(
            "INSERT INTO travel_orders \
             (id, user_id, order_type, product_id, product_snapshot, destination_id, \
              booking_id, status, amount_cents, expire_at) \
             VALUES (?, ?, ?, ?, ?, ?, 0, 0, ?, DATE_ADD(NOW(), INTERVAL 15 MINUTE))",
            &[
                json!(order_id),
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
        restore_stock(&state, body.order_type, stock_id, body.quantity).await;
        return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
    }

    // 6. 读回订单（按生成的 id 精确回查，修复原并发下可能取回他人订单的竞态）
    let fetched = match db
        .query_with(
            &format!("SELECT {ORDER_COLS} FROM travel_orders WHERE id = ?"),
            &[json!(order_id)],
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

/// P4-07 支付确认（内部接口，X-Internal-Token 防护）：status 0→1。
/// 幂等：已终态（1/2/3）直接返回当前订单；仅接受 status=0（取消/超时后不可支付）。
/// 置 1 后发布 order.paid 审计事件（含 txn_no/amount）。
pub(crate) async fn pay_success(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    headers: HeaderMap,
    Json(body): Json<PaySuccessReq>,
) -> (StatusCode, Json<ApiResponse<OrderOut>>) {
    let token = headers.get("x-internal-token").and_then(|v| v.to_str().ok()).unwrap_or("");
    let expected = std::env::var("INTERNAL_TOKEN").unwrap_or_else(|_| "dev-internal-secret".into());
    if token != expected {
        return err(StatusCode::UNAUTHORIZED, 401, "invalid internal token");
    }
    let Some(db) = state.db.clone() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
    };
    let rows = match db
        .query_with(&format!("SELECT {ORDER_COLS} FROM travel_orders WHERE id = ?"), &[json!(id)])
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "order query failed");
            return err(StatusCode::SERVICE_UNAVAILABLE, 503, "database unavailable");
        }
    };
    let Some(row) = rows.first() else {
        return err(StatusCode::NOT_FOUND, 404, "order not found");
    };
    let cur = col_u64(row, "status");
    if cur == 1 || cur == 2 || cur == 3 {
        // 幂等：已支付/已确认/已完成直接返回当前订单
        return (StatusCode::OK, ApiResponse::ok(order_from_row(row)));
    }
    if cur == 4 {
        return err(StatusCode::CONFLICT, 409, "order cancelled, cannot be paid");
    }
    let amount_cents = col_u64(row, "amount_cents");
    let affected = match db
        .execute_with(
            "UPDATE travel_orders SET status = 1 WHERE id = ? AND status = 0",
            &[json!(id)],
        )
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(error = %e, "pay success update failed");
            return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
        }
    };
    if affected == 0 {
        // 并发下已被其他确认请求置 1，幂等放行
        return err(StatusCode::CONFLICT, 409, "order not payable");
    }
    if let Some(mq) = &state.mq {
        publish_audit(
            mq,
            "order.paid",
            0,
            json!({ "order_id": id, "txn_no": body.txn_no, "amount_cents": amount_cents }),
        )
        .await;
    }
    let rows = match db
        .query_with(&format!("SELECT {ORDER_COLS} FROM travel_orders WHERE id = ?"), &[json!(id)])
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "order readback failed");
            return err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error");
        }
    };
    match rows.first() {
        Some(row) => (StatusCode::OK, ApiResponse::ok(order_from_row(row))),
        None => err(StatusCode::INTERNAL_SERVER_ERROR, 500, "internal error"),
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
    let ot = snapshot_order_type(&snap);
    let pid = if ot == 1 {
        snap.get("line_date_id").and_then(|v| v.as_u64()).unwrap_or(0)
    } else {
        col_u64(&row, "product_id")
    };
    let qty = snap.get("quantity").and_then(|v| v.as_u64()).unwrap_or(0);

    restore_stock(&state, ot, pid, qty).await;
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
