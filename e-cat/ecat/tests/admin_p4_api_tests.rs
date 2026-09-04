// P4-08/09/10/14/16 集成测试：订单管理（列表/详情/退款回补库存）、
// 用户管理（列表/禁用）、航班/酒店/房型 CRUD、支付流水/渠道开关 + 409/404 分支。
// 依赖本机 MySQL（localhost:3308），连不上时跳过真实路径。
// 注意：测试共享同一 DB，必须串行运行：cargo test -p admin-service --test p4_api_tests -- --test-threads=1
#![cfg(test)]

#[path = "../src/business/admin/mod.rs"]
mod service;

use axum::extract::{Json, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use ecat_auth::{AuthClaims, JwtAuthLayer};
use ecat_data::RdbmsClient;
use ecat_data_sqlx::SqlxClient;
use serde_json::json;
use service::flight_handlers::*;
use service::handlers::*;
use service::hotel_handlers::*;
use service::line_date_handlers::*;
use service::line_handlers::*;
use service::orders_handlers::*;
use service::payments_handlers::*;
use service::users_handlers::*;
use service::*;
use ecat::business::shared::jwt_secret;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

fn admin_guard() -> AdminGuard {
    AdminGuard(AuthClaims {
        sub: "1".into(),
        exp: None,
        iat: None,
        role: Some("admin".into()),
        extra: HashMap::new(),
    })
}

fn state_with(db: SqlxClient) -> AppState {
    AppState { db: Some(Arc::new(db)), cache: None, mq: None, jwt: JwtAuthLayer::new(jwt_secret()).unwrap() }
}

async fn body_json(resp: impl IntoResponse) -> (StatusCode, serde_json::Value) {
    let (parts, body) = resp.into_response().into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    (parts.status, serde_json::from_slice(&bytes).unwrap())
}

fn unique_name(prefix: &str) -> String {
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    format!("{prefix}-{ms}")
}

/// 本机 MySQL（docker compose 映射 3308）；连不上返回 None，测试跳过。

async fn real_db() -> Option<SqlxClient> {
    let url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:travel_dev@localhost:3308/travel".into());
    match SqlxClient::connect(&url).await {
        Ok(db) => {
            Some(db)
        }
        Err(e) => {
            eprintln!("skip real-db test (mysql unreachable): {e}");
            None
        }
    }
}

/// 直接 SQL 建测试用户，返回 (id, email)。主键已去 AUTO_INCREMENT：显式生成雪花 id。
async fn insert_test_user(db: &SqlxClient) -> (u64, String) {
    let email = format!("p4-user-{}@example.com", unique_name(""));
    let id = ecat::business::shared::snowflake_id().await;
    let _ = db
        .execute_with(
            "INSERT INTO travel_users (id, email, password_hash) VALUES (?, ?, 'x')",
            &[json!(id), json!(email)],
        )
        .await;
    (id, email)
}

// ===== 订单管理 =====

#[tokio::test]
async fn orders_list_filter_and_pagination() {
    let Some(db) = real_db().await else { return };
    let st = state_with(db);
    let (uid, email) = insert_test_user(&st.db.clone().unwrap().as_ref()).await;
    let mut ids = Vec::new();
    for status in [0, 1] {
        let oid = ecat::business::shared::snowflake_id().await;
        let affected = st
            .db
            .as_ref()
            .unwrap()
            .execute_with(
                "INSERT INTO travel_orders (id, user_id, order_type, product_id, product_snapshot, \
                 destination_id, booking_id, status, amount_cents) VALUES (?, ?, 1, 1, ?, 1, 0, ?, ?)",
                &[json!(oid), json!(uid), json!("{\"title\":\"t\",\"quantity\":1}"), json!(status), json!(100)],
            )
            .await;
        assert!(affected.is_ok(), "insert order failed");
        ids.push(oid);
    }

    // keyword 匹配 email：命中 2 条
    let (status, body) = body_json(list_orders(
        State(st.clone()),
        admin_guard(),
        Query(OrdersQuery { page: 1, page_size: 10, status: None, keyword: Some(email.clone()) }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 2, "{body}");
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 2);
    assert_eq!(body["data"]["items"][0]["email"], email);
    // 分页
    let (_, body) = body_json(list_orders(
        State(st.clone()),
        admin_guard(),
        Query(OrdersQuery { page: 2, page_size: 1, status: None, keyword: Some(email.clone()) }),
    ).await).await;
    assert_eq!(body["data"]["total"], 2);
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 1);
    // status 过滤
    let (_, body) = body_json(list_orders(
        State(st.clone()),
        admin_guard(),
        Query(OrdersQuery { page: 1, page_size: 10, status: Some("1".into()), keyword: Some(email.clone()) }),
    ).await).await;
    assert_eq!(body["data"]["total"], 1, "{body}");
    assert_eq!(body["data"]["items"][0]["status"], 1);
    // 非法 status → 400
    let (status, body) = body_json(list_orders(
        State(st.clone()),
        admin_guard(),
        Query(OrdersQuery { page: 1, page_size: 10, status: Some("9".into()), keyword: None }),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "status must be 0-4");
    // keyword 匹配订单 id
    let (_, body) = body_json(list_orders(
        State(st.clone()),
        admin_guard(),
        Query(OrdersQuery { page: 1, page_size: 10, status: None, keyword: Some(ids[0].to_string()) }),
    ).await).await;
    assert_eq!(body["data"]["total"], 1, "订单 id 关键字应命中: {body}");

    let db = st.db.as_ref().unwrap();
    for id in ids {
        let _ = db.execute_with("DELETE FROM travel_orders WHERE id = ?", &[json!(id)]).await;
    }
    let _ = db.execute_with("DELETE FROM travel_users WHERE id = ?", &[json!(uid)]).await;
}

#[tokio::test]
async fn orders_detail_and_refund_restores_stock() {
    let Some(db) = real_db().await else { return };
    let st = state_with(db);
    let (uid, _email) = insert_test_user(&st.db.clone().unwrap().as_ref()).await;

    // 前置：目的地 → 线路 → 班期（余位 8）
    let (status, body) = body_json(create_destination(
        State(st.clone()),
        admin_guard(),
        Json(json!({ "name_en": unique_name("t-od"), "name_zh": "退款测试目的地" })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let dest_id = body["data"]["id"].as_u64().unwrap();
    // 规格 E：id_str 与数值 id 十进制一致（雪花 id >2^53，JS 侧数值解析会舍入，靠 id_str 保真）
    assert_eq!(body["data"]["id_str"], dest_id.to_string(), "{body}");
    assert!(dest_id > 1u64 << 40, "创建响应的 id 应为雪花大 id: {dest_id}");
    let (status, body) = body_json(create_line(
        State(st.clone()),
        admin_guard(),
        Json(json!({ "title_en": "refund line", "title_zh": "退款线路", "destination_id": dest_id })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let line_id = body["data"]["id"].as_u64().unwrap();
    assert_eq!(body["data"]["id_str"], line_id.to_string(), "{body}");
    let (status, body) = body_json(create_line_date(
        State(st.clone()),
        admin_guard(),
        Path(line_id),
        Json(json!({ "depart_date": "2026-12-10", "price_cents": 10000, "seats_left": 8 })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let date_id = body["data"]["id"].as_u64().unwrap();
    assert_eq!(body["data"]["id_str"], date_id.to_string(), "{body}");

    // 模拟下单：余位 8 → 6，建已支付订单 + 支付流水
    let snap = json!({
        "title": "退款线路", "price_cents": 10000, "depart_date": "2026-12-10",
        "quantity": 2, "line_date_id": date_id, "order_type": 1,
    });
    let db = st.db.as_ref().unwrap();
    let _ = db
        .execute_with(
            "UPDATE travel_line_dates SET seats_left = seats_left - 2 WHERE id = ?",
            &[json!(date_id)],
        )
        .await;
    let order_id = ecat::business::shared::snowflake_id().await;
    let _ = db
        .execute_with(
            "INSERT INTO travel_orders (id, user_id, order_type, product_id, product_snapshot, \
             destination_id, booking_id, status, amount_cents) VALUES (?, ?, 1, ?, ?, ?, 0, 1, 20000)",
            &[json!(order_id), json!(uid), json!(line_id), json!(snap.to_string()), json!(dest_id)],
        )
        .await;
    let txn = unique_name("txn");
    let _ = db
        .execute_with(
            "INSERT INTO travel_payments (id, order_id, channel_code, amount_cents, status, txn_no, paid_at) \
             VALUES (?, ?, 'card', 20000, 1, ?, NOW())",
            &[json!(ecat::business::shared::snowflake_id().await), json!(order_id), json!(txn)],
        )
        .await;

    // 详情：含支付流水
    let (status, body) = body_json(order_detail(State(st.clone()), admin_guard(), Path(order_id)).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["id"], order_id);
    assert_eq!(body["data"]["snapshot"]["quantity"], 2);
    assert_eq!(body["data"]["payments"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"]["payments"][0]["status"], 1);
    // 规格 E：详情订单与嵌套支付流水对象的 id_str 均须与数值 id 一致
    assert_eq!(body["data"]["id_str"], order_id.to_string(), "{body}");
    let pay = &body["data"]["payments"][0];
    assert_eq!(pay["id_str"], pay["id"].as_u64().unwrap().to_string(), "{body}");

    // 退款：订单 1→4、支付流水 1→3、余位 6→8
    let (status, body) = body_json(refund_order(State(st.clone()), admin_guard(), Path(order_id)).await).await;
    assert_eq!(status, StatusCode::OK, "refund failed: {body}");
    assert_eq!(body["data"]["status"], 4);
    let rows = db.query_with("SELECT CAST(seats_left AS SIGNED) AS seats_left FROM travel_line_dates WHERE id = ?", &[json!(date_id)]).await.unwrap();
    let seats = rows.first().unwrap().get("seats_left").unwrap().as_i64().unwrap();
    assert_eq!(seats, 8, "退款后余位应回补为 8");
    let rows = db.query_with("SELECT CAST(status AS SIGNED) AS status FROM travel_payments WHERE order_id = ?", &[json!(order_id)]).await.unwrap();
    let pstatus = rows.first().unwrap().get("status").unwrap().as_i64().unwrap();
    assert_eq!(pstatus, 3, "支付流水应置已退款");

    // 重复退款 → 409
    let (status, body) = body_json(refund_order(State(st.clone()), admin_guard(), Path(order_id)).await).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    // 待支付订单退款 → 409
    let pending_id = ecat::business::shared::snowflake_id().await;
    let _ = db
        .execute_with(
            "INSERT INTO travel_orders (id, user_id, order_type, product_id, product_snapshot, \
             destination_id, booking_id, status, amount_cents) VALUES (?, ?, 1, ?, ?, ?, 0, 0, 100)",
            &[json!(pending_id), json!(uid), json!(line_id), json!(snap.to_string()), json!(dest_id)],
        )
        .await;
    let (status, body) = body_json(refund_order(State(st.clone()), admin_guard(), Path(pending_id)).await).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["message"], "only paid or confirmed orders can be refunded");
    // 不存在的订单 → 404
    let (status, body) = body_json(refund_order(State(st.clone()), admin_guard(), Path(99_999_999)).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "order not found");
    let (status, _) = body_json(order_detail(State(st.clone()), admin_guard(), Path(99_999_999)).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // 清理
    for id in [order_id, pending_id] {
        let _ = db.execute_with("DELETE FROM travel_orders WHERE id = ?", &[json!(id)]).await;
    }
    let _ = db.execute_with("DELETE FROM travel_payments WHERE order_id = ?", &[json!(order_id)]).await;
    let _ = db.execute_with("DELETE FROM travel_line_dates WHERE id = ?", &[json!(date_id)]).await;
    let _ = db.execute_with("DELETE FROM travel_lines WHERE id = ?", &[json!(line_id)]).await;
    let _ = db.execute_with("DELETE FROM travel_destinations WHERE id = ?", &[json!(dest_id)]).await;
    let _ = db.execute_with("DELETE FROM travel_users WHERE id = ?", &[json!(uid)]).await;
}

// ===== 用户管理 =====

#[tokio::test]
async fn users_list_and_status_toggle() {
    let Some(db) = real_db().await else { return };
    let st = state_with(db);
    let (uid, email) = insert_test_user(&st.db.clone().unwrap().as_ref()).await;

    // 列表：keyword 命中且绝不返回 password_hash
    let (status, body) = body_json(list_users(
        State(st.clone()),
        admin_guard(),
        Query(UsersQuery { page: 1, page_size: 10, keyword: Some(email.clone()) }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 1, "{body}");
    let item = &body["data"]["items"][0];
    assert_eq!(item["email"], email);
    assert!(item.get("password_hash").is_none(), "绝不返回 password_hash: {body}");
    assert!(item.get("status").is_some());
    // 规格 E：用户行对象 id_str 与数值 id 一致
    let item_id = item["id"].as_u64().unwrap();
    assert_eq!(item["id_str"], item_id.to_string(), "{body}");
    assert_eq!(item_id, uid, "列表返回的 id 应与插入 id 一致: {body}");

    // 禁用 → 状态回读 1
    let (status, body) = body_json(update_user_status(
        State(st.clone()),
        admin_guard(),
        Path(uid),
        Json(StatusReq { status: 1 }),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["status"], 1);
    // 非法状态 → 400
    let (status, body) = body_json(update_user_status(
        State(st.clone()),
        admin_guard(),
        Path(uid),
        Json(StatusReq { status: 5 }),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "status must be 0 or 1");
    // 不存在 → 404
    let (status, body) = body_json(update_user_status(
        State(st.clone()),
        admin_guard(),
        Path(99_999_999),
        Json(StatusReq { status: 1 }),
    ).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "user not found");
    // 恢复并清理
    let _ = st.db.as_ref().unwrap().execute_with("DELETE FROM travel_users WHERE id = ?", &[json!(uid)]).await;
}

// ===== 航班管理 =====

#[tokio::test]
async fn flights_crud_full_flow() {
    let Some(db) = real_db().await else { return };
    let st = state_with(db);
    // 清理历史失败运行残留的测试航班
    let _ = st
        .db
        .as_ref()
        .unwrap()
        .execute_with("DELETE FROM travel_flights WHERE airline LIKE 'Test Air t-flt-%'", &[])
        .await;
    let kw = unique_name("t-flt");

    // 必填校验
    let (status, body) = body_json(create_flight(
        State(st.clone()),
        admin_guard(),
        Json(json!({ "airline": "Test Air", "from_code": "HND", "to_code": "HKG", "depart_at": "2026-12-01 09:00:00", "price_cents": 10000 })),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "flight_no is required");
    // IATA 非 3 字母 → 400
    let (status, body) = body_json(create_flight(
        State(st.clone()),
        admin_guard(),
        Json(json!({ "airline": "Test Air", "flight_no": "TA100", "from_code": "HNDX", "to_code": "HKG", "depart_at": "2026-12-01 09:00:00", "price_cents": 10000 })),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "from_code must be a 3-letter IATA code");
    // 非法时间格式 → 400
    let (status, body) = body_json(create_flight(
        State(st.clone()),
        admin_guard(),
        Json(json!({ "airline": "Test Air", "flight_no": "TA100", "from_code": "HND", "to_code": "HKG", "depart_at": "12/01/2026", "price_cents": 10000 })),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "depart_at must be YYYY-MM-DD HH:MM:SS");

    // 创建
    let (status, body) = body_json(create_flight(
        State(st.clone()),
        admin_guard(),
        Json(json!({
            "airline": format!("Test Air {kw}"), "flight_no": unique_name("F"),
            "from_code": "HND", "to_code": "HKG",
            "depart_at": "2026-12-01 09:00:00", "arrive_at": "2026-12-01 13:15:00",
            "cabin": 1, "price_cents": 158000, "seats_left": 12, "status": 1,
        })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "create flight failed: {body}");
    let flight_id = body["data"]["id"].as_u64().unwrap();
    assert_eq!(body["data"]["id_str"], flight_id.to_string(), "{body}");
    assert_eq!(body["data"]["from_code"], "HND");
    assert_eq!(body["data"]["depart_at"], "2026-12-01 09:00:00");
    assert_eq!(body["data"]["price_cents"], 158000);

    // 列表：keyword + from/to 过滤
    let (status, body) = body_json(list_flights(
        State(st.clone()),
        admin_guard(),
        Query(FlightsQuery { page: 1, page_size: 10, keyword: Some(kw.clone()), from: None, to: None }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 1, "{body}");
    let (_, body) = body_json(list_flights(
        State(st.clone()),
        admin_guard(),
        Query(FlightsQuery { page: 1, page_size: 10, keyword: None, from: Some("HND".into()), to: Some("HKG".into()) }),
    ).await).await;
    assert!(body["data"]["total"].as_u64().unwrap() >= 1, "from/to 过滤应命中: {body}");
    // 无此航线（种子为 NRT→CDG 等）→ 0
    let (_, body) = body_json(list_flights(
        State(st.clone()),
        admin_guard(),
        Query(FlightsQuery { page: 1, page_size: 10, keyword: None, from: Some("NRT".into()), to: Some("HND".into()) }),
    ).await).await;
    assert_eq!(body["data"]["total"], 0, "NRT→HND 不应命中: {body}");

    // 更新 + 上下架
    let (status, body) = body_json(update_flight(
        State(st.clone()),
        admin_guard(),
        Path(flight_id),
        Json(json!({ "price_cents": 168000, "seats_left": 9 })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["price_cents"], 168000);
    let (status, body) = body_json(update_flight_status(
        State(st.clone()),
        admin_guard(),
        Path(flight_id),
        Json(StatusReq { status: 0 }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], 0);
    // 空 body → 400；不存在 → 404
    let (status, body) = body_json(update_flight(
        State(st.clone()),
        admin_guard(),
        Path(flight_id),
        Json(json!({})),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "no fields to update");
    let (status, body) = body_json(update_flight(
        State(st.clone()),
        admin_guard(),
        Path(99_999_999),
        Json(json!({ "price_cents": 1 })),
    ).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "flight not found");
    // 删除
    let (status, _) = body_json(delete_flight(State(st.clone()), admin_guard(), Path(flight_id)).await).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = body_json(delete_flight(State(st.clone()), admin_guard(), Path(99_999_999)).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ===== 酒店/房型管理 =====

#[tokio::test]
async fn hotels_rooms_crud_with_409() {
    let Some(db) = real_db().await else { return };
    let st = state_with(db);
    let kw = unique_name("t-hot");

    // 必填校验
    let (status, body) = body_json(create_hotel(
        State(st.clone()),
        admin_guard(),
        Json(json!({ "name_en": "Only EN" })),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "name_zh is required");

    // 创建酒店
    let (status, body) = body_json(create_hotel(
        State(st.clone()),
        admin_guard(),
        Json(json!({
            "name_en": format!("Hotel {kw}"), "name_zh": format!("测试酒店{kw}"),
            "name_ja": "テスト", "city_code": "TYO", "star": 4,
            "latitude": 35.6909, "longitude": 139.7004,
            "cover_url": "https://erik.xyz/hotel/x.jpg", "status": 1,
        })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "create hotel failed: {body}");
    let hotel_id = body["data"]["id"].as_u64().unwrap();
    assert_eq!(body["data"]["id_str"], hotel_id.to_string(), "{body}");
    assert_eq!(body["data"]["city_code"], "TYO");
    assert_eq!(body["data"]["star"], 4);
    assert_eq!(body["data"]["latitude"], 35.6909);
    // 城市码非 3 字母 → 400
    let (status, body) = body_json(create_hotel(
        State(st.clone()),
        admin_guard(),
        Json(json!({ "name_en": "x", "name_zh": "y", "city_code": "TOKYO" })),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "city_code must be a 3-letter code");

    // 列表：keyword + city
    let (_, body) = body_json(list_hotels(
        State(st.clone()),
        admin_guard(),
        Query(HotelsQuery { page: 1, page_size: 10, keyword: Some(kw.clone()), city: None }),
    ).await).await;
    assert_eq!(body["data"]["total"], 1, "{body}");
    let (_, body) = body_json(list_hotels(
        State(st.clone()),
        admin_guard(),
        Query(HotelsQuery { page: 1, page_size: 10, keyword: None, city: Some("TYO".into()) }),
    ).await).await;
    assert!(body["data"]["total"].as_u64().unwrap() >= 1);

    // 不存在的酒店挂房型 → 404；房型列表 → 404
    let (status, body) = body_json(create_room(
        State(st.clone()),
        admin_guard(),
        Path(99_999_999),
        Json(json!({ "room_type_en": "x", "price_cents": 1 })),
    ).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "hotel not found");
    let (status, _) = body_json(list_rooms(State(st.clone()), admin_guard(), Path(99_999_999)).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // 建房型（缺 price → 400；正常建 → 房型列表裸数组）
    let (status, body) = body_json(create_room(
        State(st.clone()),
        admin_guard(),
        Path(hotel_id),
        Json(json!({ "room_type_en": format!("Standard {kw}") })),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "price_cents is required");
    let (status, body) = body_json(create_room(
        State(st.clone()),
        admin_guard(),
        Path(hotel_id),
        Json(json!({
            "room_type_en": format!("Standard {kw}"), "room_type_zh": "标准房",
            "price_cents": 68000, "breakfast": 1, "inventory": 10, "status": 1,
        })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "create room failed: {body}");
    let room_id = body["data"]["id"].as_u64().unwrap();
    assert_eq!(body["data"]["id_str"], room_id.to_string(), "{body}");
    assert_eq!(body["data"]["breakfast"], 1);
    let (status, body) = body_json(list_rooms(State(st.clone()), admin_guard(), Path(hotel_id)).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 1, "{body}");

    // 房型更新/删除 404
    let (status, body) = body_json(update_room(
        State(st.clone()),
        admin_guard(),
        Path((hotel_id, room_id)),
        Json(json!({ "price_cents": 72000, "inventory": 8 })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["price_cents"], 72000);
    let (status, _) = body_json(update_room(
        State(st.clone()),
        admin_guard(),
        Path((hotel_id, 99_999_999)),
        Json(json!({ "price_cents": 1 })),
    ).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // 有房型时删酒店 → 409
    let (status, body) = body_json(delete_hotel(State(st.clone()), admin_guard(), Path(hotel_id)).await).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["message"], "hotel has related rooms, delete them first");

    // 删房型后酒店可删；更新酒店 + 上下架
    let (status, _) = body_json(delete_room(State(st.clone()), admin_guard(), Path((hotel_id, room_id))).await).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = body_json(update_hotel(
        State(st.clone()),
        admin_guard(),
        Path(hotel_id),
        Json(json!({ "star": 5 })),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["star"], 5);
    let (status, body) = body_json(update_hotel_status(
        State(st.clone()),
        admin_guard(),
        Path(hotel_id),
        Json(StatusReq { status: 0 }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], 0);
    let (status, _) = body_json(delete_hotel(State(st.clone()), admin_guard(), Path(hotel_id)).await).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = body_json(delete_hotel(State(st.clone()), admin_guard(), Path(99_999_999)).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ===== 支付管理（P4-16）=====

#[tokio::test]
async fn payments_list_and_channels_toggle() {
    let Some(db) = real_db().await else { return };
    let st = state_with(db);
    let txn = format!("txn-test-p4-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis());
    // 合成流水（order_id=0 无对应订单，LEFT JOIN 下 email 为空）
    st.db.as_ref().unwrap().execute_with(
        "INSERT INTO travel_payments (id, order_id, channel_code, amount_cents, status, txn_no, created_at) \
         VALUES (?, 0, 'stripe', 100, 1, ?, NOW())",
        &[json!(ecat::business::shared::snowflake_id().await), json!(txn)],
    ).await.unwrap();

    // 列表：channel 过滤命中，字段完整
    let (status, body) = body_json(list_payments(
        State(st.clone()),
        admin_guard(),
        Query(PaymentsQuery { page: 1, page_size: 10, channel: Some("stripe".into()), status: None }),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["data"]["total"].as_u64().unwrap() >= 1, "{body}");
    assert_eq!(body["data"]["items"][0]["txn_no"], txn);
    assert_eq!(body["data"]["items"][0]["channel_code"], "stripe");
    assert_eq!(body["data"]["items"][0]["status"], 1);
    // 非法 status → 400
    let (status, body) = body_json(list_payments(
        State(st.clone()),
        admin_guard(),
        Query(PaymentsQuery { page: 1, page_size: 10, channel: None, status: Some("9".into()) }),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "status must be 0-3");

    // 渠道列表：含 stripe 且 enabled
    let (status, body) = body_json(list_channels(State(st.clone()), admin_guard()).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let stripe = body["data"]["items"].as_array().unwrap().iter()
        .find(|c| c["channel_code"] == "stripe").expect("stripe channel seeded");
    assert_eq!(stripe["enabled"], true);
    let raw = st.db.as_ref().unwrap().query(
        "SELECT name FROM travel_payment_channels WHERE channel_code = 'stripe'",
    ).await.unwrap();
    eprintln!("RAW NAME VALUE: {:?}", raw[0].get("name"));
    assert!(stripe["name"].is_object(), "多语言名称应解析为对象, got: {stripe}");

    // 开关：关闭 → 回读 false；恢复 true；未知渠道 404
    let (status, body) = body_json(update_channel_enabled(
        State(st.clone()),
        admin_guard(),
        Path("stripe".into()),
        Json(EnabledReq { enabled: false }),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["enabled"], false);
    let (status, body) = body_json(update_channel_enabled(
        State(st.clone()),
        admin_guard(),
        Path("stripe".into()),
        Json(EnabledReq { enabled: true }),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["enabled"], true);
    let (status, body) = body_json(update_channel_enabled(
        State(st.clone()),
        admin_guard(),
        Path("no-such-channel".into()),
        Json(EnabledReq { enabled: true }),
    ).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "channel not found");

    // 清理
    let _ = st.db.as_ref().unwrap().execute_with("DELETE FROM travel_payments WHERE txn_no = ?", &[json!(txn)]).await;
}
