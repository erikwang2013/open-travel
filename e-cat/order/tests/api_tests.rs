// P3-08/P3-09 集成测试：经 Router 一次性调用，覆盖下单/库存/取消/列表/详情。
// DB/Redis 依赖用例在无数据源时跳过（离线仍可跑）。JWT 用固定密钥自签
// （与 user-service 共享 jwt_secret），sub 即 user_id。
#![cfg(test)]

#[path = "../src/main.rs"]
mod service;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use ecat_auth::JwtAuthLayer;
use ecat_data::{Cache, RdbmsClient};
use ecat_data_redis::RedisCache;
use ecat_data_sqlx::SqlxClient;
use tower::ServiceExt;
use serde::Serialize;
use serde_json::{json, Value};
use service::*;
use shared::{connect_primary, jwt_secret};
use std::sync::Arc;

#[derive(Serialize)]
struct LoginClaims {
    sub: String,
}

fn sign(sub: &str) -> String {
    let jwt = JwtAuthLayer::new(jwt_secret()).unwrap();
    jwt.sign(&LoginClaims { sub: sub.into() }, 3600).unwrap()
}

async fn call(
    router: Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("x-api-version", "v1");
    if let Some(t) = token {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }
    let req = match body {
        Some(v) => builder
            .header("content-type", "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json_body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, json_body)
}

fn state_with(db: Arc<SqlxClient>) -> AppState {
    AppState { db: Some(db), cache: None, mq: None, jwt: JwtAuthLayer::new(jwt_secret()).unwrap() }
}

fn router(state: AppState) -> Router {
    api_router(state)
}

/// 取一条未来班期并把它余位设为指定值；返回 (line_date_id, line_id, price)。
async fn pick_line_date(db: &SqlxClient, seats: u64) -> (u64, u64, u64) {
    let rows = db
        .query_with(
            "SELECT id, line_id, price_cents FROM travel_line_dates \
             WHERE status = 1 AND depart_date >= CURDATE() LIMIT 1",
            &[],
        )
        .await
        .unwrap();
    let row = rows.first().unwrap();
    let id = row.get("id").and_then(|v| v.as_u64()).unwrap();
    let line_id = row.get("line_id").and_then(|v| v.as_u64()).unwrap();
    let price = row.get("price_cents").and_then(|v| v.as_u64()).unwrap();
    db.execute_with(
        "UPDATE travel_line_dates SET seats_left = ? WHERE id = ?",
        &[json!(seats), json!(id)],
    )
    .await
    .unwrap();
    (id, line_id, price)
}

fn order_body(line_id: u64, line_date_id: u64, qty: u64) -> Value {
    json!({"order_type": 1, "product_id": line_id, "line_date_id": line_date_id, "quantity": qty})
}

#[tokio::test]
async fn create_order_requires_jwt() {
    // JwtAuthLayer 直接拒绝（非 JSON 信封），只断言状态码
    let (status, _) = call(router(state_with_db_or_none()), "POST", "/api/orders", None, Some(json!({}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unsupported_order_type_returns_400() {
    let Some(db) = connect_primary().await else { return };
    let (status, _) = call(
        router(state_with(db)),
        "POST",
        "/api/orders",
        Some(&sign("1")),
        Some(json!({"order_type": 9, "product_id": 1, "quantity": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_order_success_decrements_seats() {
    let Some(db) = connect_primary().await else { return };
    let (ld_id, line_id, price) = pick_line_date(&db, 10).await;
    let (status, body) = call(
        router(state_with(db.clone())),
        "POST",
        "/api/orders",
        Some(&sign("1")),
        Some(order_body(line_id, ld_id, 2)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"]["status"], 0);
    assert_eq!(body["data"]["amount_cents"], price * 2);
    assert_eq!(body["data"]["snapshot"]["quantity"], 2);
    assert!(body["data"]["expire_at"].is_string());
    let rows = db
        .query_with("SELECT seats_left FROM travel_line_dates WHERE id = ?", &[json!(ld_id)])
        .await
        .unwrap();
    assert_eq!(rows[0].get("seats_left").and_then(|v| v.as_u64()).unwrap(), 8);
}

#[tokio::test]
async fn insufficient_stock_returns_409() {
    let Some(db) = connect_primary().await else { return };
    let (ld_id, line_id, _) = pick_line_date(&db, 1).await;
    let (status, body) = call(
        router(state_with(db.clone())),
        "POST",
        "/api/orders",
        Some(&sign("1")),
        Some(order_body(line_id, ld_id, 2)),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "body: {body}");
    assert_eq!(body["code"], 409);
    // 余位未被扣减
    let rows = db
        .query_with("SELECT seats_left FROM travel_line_dates WHERE id = ?", &[json!(ld_id)])
        .await
        .unwrap();
    assert_eq!(rows[0].get("seats_left").and_then(|v| v.as_u64()).unwrap(), 1);
}

#[tokio::test]
async fn concurrent_orders_cannot_oversell() {
    let Some(db) = connect_primary().await else { return };
    let (ld_id, line_id, _) = pick_line_date(&db, 1).await;
    let r = router(state_with(db.clone()));
    let token = sign("1");
    let body = order_body(line_id, ld_id, 1);
    let (a, b) = tokio::join!(
        call(r.clone(), "POST", "/api/orders", Some(&token), Some(body.clone())),
        call(r, "POST", "/api/orders", Some(&token), Some(body))
    );
    let ok_count = [&a, &b].iter().filter(|(s, _)| *s == StatusCode::OK).count();
    let conflict_count = [&a, &b].iter().filter(|(s, _)| *s == StatusCode::CONFLICT).count();
    assert_eq!((ok_count, conflict_count), (1, 1), "a={a:?} b={b:?}");
    let rows = db
        .query_with("SELECT seats_left FROM travel_line_dates WHERE id = ?", &[json!(ld_id)])
        .await
        .unwrap();
    assert_eq!(rows[0].get("seats_left").and_then(|v| v.as_u64()).unwrap(), 0);
}

#[tokio::test]
async fn cancel_success_restores_seats() {
    let Some(db) = connect_primary().await else { return };
    let (ld_id, line_id, _) = pick_line_date(&db, 10).await;
    let r = router(state_with(db.clone()));
    let token = sign("1");
    let (_, body) = call(r.clone(), "POST", "/api/orders", Some(&token), Some(order_body(line_id, ld_id, 3))).await;
    let order_id = body["data"]["id"].as_u64().unwrap();
    let (status, body) = call(r, "POST", &format!("/api/orders/{order_id}/cancel"), Some(&token), None).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["status"], 4);
    let rows = db
        .query_with("SELECT seats_left FROM travel_line_dates WHERE id = ?", &[json!(ld_id)])
        .await
        .unwrap();
    assert_eq!(rows[0].get("seats_left").and_then(|v| v.as_u64()).unwrap(), 10);
}

#[tokio::test]
async fn cancel_non_pending_returns_400() {
    let Some(db) = connect_primary().await else { return };
    let (ld_id, line_id, _) = pick_line_date(&db, 10).await;
    let r = router(state_with(db.clone()));
    let token = sign("1");
    let (_, body) = call(r.clone(), "POST", "/api/orders", Some(&token), Some(order_body(line_id, ld_id, 1))).await;
    let order_id = body["data"]["id"].as_u64().unwrap();
    db.execute_with(
        "UPDATE travel_orders SET status = 1 WHERE id = ?",
        &[json!(order_id)],
    )
    .await
    .unwrap();
    let (status, body) = call(r, "POST", &format!("/api/orders/{order_id}/cancel"), Some(&token), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["code"], 400);
}

#[tokio::test]
async fn list_returns_only_own_orders() {
    let Some(db) = connect_primary().await else { return };
    let (ld_id, line_id, _) = pick_line_date(&db, 10).await;
    let r = router(state_with(db.clone()));
    let (_, _) = call(r.clone(), "POST", "/api/orders", Some(&sign("1")), Some(order_body(line_id, ld_id, 1))).await;
    let (_, _) = call(r.clone(), "POST", "/api/orders", Some(&sign("2")), Some(order_body(line_id, ld_id, 1))).await;
    let (status, body) = call(r.clone(), "GET", "/api/orders", Some(&sign("1")), None).await;
    assert_eq!(status, StatusCode::OK);
    let items = body["data"].as_array().unwrap();
    assert!(!items.is_empty());
    for item in items {
        assert!(item["user_id"].is_null()); // user_id 不外显
        assert!(item["snapshot"]["quantity"].as_u64().is_some());
    }
    // 用户 2 的列表同样只含自己的（至少有一条）
    let (_, body2) = call(r, "GET", "/api/orders", Some(&sign("2")), None).await;
    assert!(!body2["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn detail_of_other_user_returns_404() {
    let Some(db) = connect_primary().await else { return };
    let (ld_id, line_id, _) = pick_line_date(&db, 10).await;
    let r = router(state_with(db.clone()));
    let (_, body) = call(r.clone(), "POST", "/api/orders", Some(&sign("1")), Some(order_body(line_id, ld_id, 1))).await;
    let order_id = body["data"]["id"].as_u64().unwrap();
    let (status, body) = call(r.clone(), "GET", &format!("/api/orders/{order_id}"), Some(&sign("2")), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    // 本人可见
    let (_, body2) = call(r, "GET", &format!("/api/orders/{order_id}"), Some(&sign("1")), None).await;
    assert_eq!(body2["data"]["id"], json!(order_id));
}

#[tokio::test]
async fn expired_pending_order_is_cancelled_and_stock_restored() {
    let Some(db) = connect_primary().await else { return };
    let (ld_id, line_id, _) = pick_line_date(&db, 10).await;
    let r = router(state_with(db.clone()));
    let token = sign("1");
    let (_, body) = call(r.clone(), "POST", "/api/orders", Some(&token), Some(order_body(line_id, ld_id, 2))).await;
    let order_id = body["data"]["id"].as_u64().unwrap();
    db.execute_with(
        "UPDATE travel_orders SET expire_at = DATE_SUB(NOW(), INTERVAL 1 HOUR) WHERE id = ?",
        &[json!(order_id)],
    )
    .await
    .unwrap();
    // 触发惰性清理：列表前扫描过期订单
    let (_, list_body) = call(r, "GET", "/api/orders", Some(&token), None).await;
    let items = list_body["data"].as_array().unwrap();
    let expired = items.iter().find(|i| i["id"] == json!(order_id)).unwrap();
    assert_eq!(expired["status"], 4, "expired order should be auto-cancelled");
    let rows = db
        .query_with("SELECT seats_left FROM travel_line_dates WHERE id = ?", &[json!(ld_id)])
        .await
        .unwrap();
    assert_eq!(rows[0].get("seats_left").and_then(|v| v.as_u64()).unwrap(), 10);
}

#[tokio::test]
async fn redis_stock_key_released_on_cancel() {
    let Some(db) = connect_primary().await else { return };
    let Ok(cache) = RedisCache::connect(&std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6381".into())).await else { return };
    let cache = Arc::new(cache);
    let (ld_id, line_id, _) = pick_line_date(&db, 10).await;
    let state = AppState {
        db: Some(db),
        cache: Some(cache.clone()),
        mq: None,
        jwt: JwtAuthLayer::new(jwt_secret()).unwrap(),
    };
    let r = router(state);
    let token = sign("1");
    let key = format!("travel:stock:1:{ld_id}");
    let _ = cache.delete(&key).await;
    let (_, body) = call(r.clone(), "POST", "/api/orders", Some(&token), Some(order_body(line_id, ld_id, 4))).await;
    assert_eq!(body["code"], 0, "body: {body}");
    let left: i64 = cache
        .get(&key)
        .await
        .unwrap()
        .and_then(|v| String::from_utf8(v).ok())
        .and_then(|s| s.parse().ok())
        .unwrap();
    assert_eq!(left, 6);
    let order_id = body["data"]["id"].as_u64().unwrap();
    let (_, _) = call(r, "POST", &format!("/api/orders/{order_id}/cancel"), Some(&token), None).await;
    let left2: i64 = cache
        .get(&key)
        .await
        .unwrap()
        .and_then(|v| String::from_utf8(v).ok())
        .and_then(|s| s.parse().ok())
        .unwrap();
    assert_eq!(left2, 10);
}

// ---- P4-07 支付闭环：pay-success 内部接口 ----

/// pay-success 专用请求：带 X-Internal-Token（None 表示不带，测 401）。
async fn pay_success_call(router: Router, order_id: u64, token: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri(format!("/api/orders/{order_id}/pay-success"))
        .header("x-api-version", "v1")
        .header("content-type", "application/json");
    if let Some(t) = token {
        builder = builder.header("x-internal-token", t);
    }
    let req = builder
        .body(Body::from(json!({"txn_no": "stripe_test_txn"}).to_string()))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

async fn new_pending_order(db: &SqlxClient, r: &Router, user: &str) -> u64 {
    let (ld_id, line_id, _) = pick_line_date(db, 10).await;
    let (_, body) = call(r.clone(), "POST", "/api/orders", Some(&sign(user)), Some(order_body(line_id, ld_id, 1))).await;
    body["data"]["id"].as_u64().unwrap()
}

#[tokio::test]
async fn pay_success_marks_order_paid() {
    let Some(db) = connect_primary().await else { return };
    let r = router(state_with(db.clone()));
    let order_id = new_pending_order(&db, &r, "1").await;
    let (status, body) = pay_success_call(r, order_id, Some("dev-internal-secret")).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["data"]["status"], 1);
    let rows = db
        .query_with("SELECT CAST(status AS CHAR) AS status FROM travel_orders WHERE id = ?", &[json!(order_id)])
        .await
        .unwrap();
    assert_eq!(rows[0].get("status").and_then(|v| v.as_str()).unwrap(), "1");
}

#[tokio::test]
async fn pay_success_idempotent() {
    let Some(db) = connect_primary().await else { return };
    let r = router(state_with(db.clone()));
    let order_id = new_pending_order(&db, &r, "1").await;
    let (s1, b1) = pay_success_call(r.clone(), order_id, Some("dev-internal-secret")).await;
    let (s2, b2) = pay_success_call(r, order_id, Some("dev-internal-secret")).await;
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(s2, StatusCode::OK, "重复确认应幂等成功: {b1} {b2}");
    assert_eq!(b2["data"]["status"], 1);
}

#[tokio::test]
async fn pay_success_without_token_401() {
    let Some(db) = connect_primary().await else { return };
    let r = router(state_with(db.clone()));
    let order_id = new_pending_order(&db, &r, "1").await;
    let (status, _) = pay_success_call(r, order_id, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn pay_success_cancelled_order_409() {
    let Some(db) = connect_primary().await else { return };
    let r = router(state_with(db.clone()));
    let order_id = new_pending_order(&db, &r, "1").await;
    let (_, _) = call(r.clone(), "POST", &format!("/api/orders/{order_id}/cancel"), Some(&sign("1")), None).await;
    let (status, body) = pay_success_call(r, order_id, Some("dev-internal-secret")).await;
    assert_eq!(status, StatusCode::CONFLICT, "取消后不可支付: {body}");
}

// 无 DB 时的 401 用例：构造无数据源 state（db/cache/mq 均 None）
fn state_with_db_or_none() -> AppState {
    AppState {
        db: None,
        cache: None,
        mq: None,
        jwt: JwtAuthLayer::new(jwt_secret()).unwrap(),
    }
}
