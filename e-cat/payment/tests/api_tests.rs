// P4-06/P4-15 集成测试：支付发起 / 幂等 / 渠道列表排序 / 回调验签与幂等 / 不重复入账。
// 依赖本机 3308 容器库（mysql://root:travel_dev@localhost:3308/travel），连接失败跳过。
// 订单确认用注入 mock（MockConfirm），不依赖真实 order-service。
#![cfg(test)]

#[path = "../src/main.rs"]
mod service;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use ecat_auth::JwtAuthLayer;
use ecat_data::RdbmsClient;
use ecat_data_sqlx::SqlxClient;
use serde::Serialize;
use serde_json::{json, Value};
use service::{handlers::hmac_hex, *};
use shared::jwt_secret;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

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
    let mut builder = Request::builder().method(method).uri(uri).header("x-api-version", "v1");
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

/// 回调请求：自动计算 X-Signature（传 None 表示不带签名头，模拟验签失败）。
async fn callback(
    router: Router,
    channel: &str,
    txn_no: &str,
    status: u8,
    with_sig: bool,
) -> (StatusCode, Value) {
    let body = json!({ "txn_no": txn_no, "status": status }).to_string();
    let mut builder = Request::builder()
        .method("POST")
        .uri(format!("/api/payments/callback/{channel}"))
        .header("content-type", "application/json")
        .header("x-api-version", "v1");
    if with_sig {
        builder = builder.header("x-signature", hmac_hex(body.as_bytes()));
    }
    let resp = router
        .clone()
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

/// 依赖 3308 容器库；连接失败跳过（离线仍可跑）。
async fn connect_test_db() -> Option<Arc<SqlxClient>> {
    match SqlxClient::connect("mysql://root:travel_dev@localhost:3308/travel?charset=utf8mb4").await {
        Ok(db) => Some(Arc::new(db)),
        Err(e) => {
            eprintln!("db connect skipped: {e}");
            None
        }
    }
}

/// 直接插入一条待支付订单（支付路径只读 travel_orders 的 status/amount_cents）。
async fn insert_order(db: &SqlxClient, user_id: u64, amount_cents: u64) -> u64 {
    db.execute_with(
        "INSERT INTO travel_orders (user_id, order_type, product_id, product_snapshot, \
         destination_id, booking_id, status, amount_cents, expire_at) \
         VALUES (?, 1, 1, '{}', 0, 0, 0, ?, DATE_ADD(NOW(), INTERVAL 15 MINUTE))",
        &[json!(user_id), json!(amount_cents)],
    )
    .await
    .unwrap();
    let rows = db
        .query_with("SELECT MAX(id) AS id FROM travel_orders WHERE user_id = ?", &[json!(user_id)])
        .await
        .unwrap();
    rows[0].get("id").and_then(|v| v.as_u64()).unwrap()
}

async fn cleanup(db: &SqlxClient, order_id: u64) {
    let _ = db
        .execute_with("DELETE FROM travel_payments WHERE order_id = ?", &[json!(order_id)])
        .await;
    let _ = db
        .execute_with("DELETE FROM travel_orders WHERE id = ?", &[json!(order_id)])
        .await;
}

/// 可注入结果的 mock：记录确认调用次数（幂等测试用）。
#[derive(Clone)]
struct MockConfirm {
    calls: Arc<AtomicUsize>,
    result: Arc<Mutex<Result<(), String>>>,
}

impl MockConfirm {
    fn new(result: Result<(), String>) -> Self {
        Self { calls: Arc::new(AtomicUsize::new(0)), result: Arc::new(Mutex::new(result)) }
    }
    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl OrderConfirm for MockConfirm {
    fn confirm(&self, _order_id: u64, _txn_no: &str) -> ConfirmFuture {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let result = self.result.lock().unwrap().clone();
        Box::pin(async move { result })
    }
}

fn state_with(db: Arc<SqlxClient>, confirm: MockConfirm) -> AppState {
    AppState {
        db: Some(db),
        confirm: Arc::new(confirm),
        jwt: JwtAuthLayer::new(jwt_secret()).unwrap(),
    }
}

fn pay_body(order_id: u64, channel: &str) -> Value {
    json!({ "order_id": order_id, "channel_code": channel })
}

fn router(state: AppState) -> Router {
    api_router(state)
}

#[tokio::test]
async fn create_payment_success() {
    let Some(db) = connect_test_db().await else { return };
    let order_id = insert_order(&db, 900101, 19900).await;
    let r = router(state_with(db.clone(), MockConfirm::new(Ok(()))));
    let (status, body) = call(r, "POST", "/api/payments", Some(&sign("900101")), Some(pay_body(order_id, "stripe"))).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["code"], 0);
    let d = &body["data"];
    assert_eq!(d["order_id"], json!(order_id));
    assert_eq!(d["channel_code"], "stripe");
    assert_eq!(d["amount_cents"], 19900);
    assert_eq!(d["status"], 0);
    assert!(d["txn_no"].as_str().unwrap().starts_with("stripe_"));
    assert!(d["checkout_url"]
        .as_str()
        .unwrap()
        .contains(&format!("/api/payments/sandbox/{}", d["txn_no"].as_str().unwrap())));
    let rows = db
        .query_with("SELECT CAST(status AS CHAR) AS status FROM travel_payments WHERE order_id = ?", &[json!(order_id)])
        .await
        .unwrap();
    assert_eq!(rows[0].get("status").and_then(|v| v.as_str()).unwrap(), "0");
    cleanup(&db, order_id).await;
}

#[tokio::test]
async fn create_payment_idempotent_same_txn() {
    let Some(db) = connect_test_db().await else { return };
    let order_id = insert_order(&db, 900102, 5000).await;
    let r = router(state_with(db.clone(), MockConfirm::new(Ok(()))));
    let (_, b1) = call(r.clone(), "POST", "/api/payments", Some(&sign("900102")), Some(pay_body(order_id, "stripe"))).await;
    let (status, b2) = call(r, "POST", "/api/payments", Some(&sign("900102")), Some(pay_body(order_id, "stripe"))).await;
    assert_eq!(status, StatusCode::OK, "body: {b2}");
    assert_eq!(b1["data"]["txn_no"], b2["data"]["txn_no"], "重复发起应返回同一条流水");
    let rows = db
        .query_with("SELECT COUNT(*) AS c FROM travel_payments WHERE order_id = ?", &[json!(order_id)])
        .await
        .unwrap();
    assert_eq!(rows[0].get("c").and_then(|v| v.as_u64()).unwrap(), 1);
    cleanup(&db, order_id).await;
}

#[tokio::test]
async fn create_payment_requires_jwt() {
    let (status, _) = call(router(state_with_db_or_none()), "POST", "/api/payments", None, Some(pay_body(1, "stripe"))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn channel_list_sorted_zh_first() {
    let Some(db) = connect_test_db().await else { return };
    let r = router(state_with(db, MockConfirm::new(Ok(()))));
    let (status, body) = call(r, "GET", "/api/payments/channels?lang=zh", None, None).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    let arr = body["data"].as_array().unwrap();
    assert!(!arr.is_empty());
    // zh 本国渠道（alipay/wechat）排最前，且按 priority DESC
    assert_eq!(arr[0]["channel_code"], "alipay");
    assert_eq!(arr[1]["channel_code"], "wechat");
    let idx = |code: &str| arr.iter().position(|c| c["channel_code"] == code);
    if let Some(usdt) = idx("usdt") {
        assert!(usdt > 1, "全语言渠道 usdt 应排本国渠道之后");
    }
    // name 已按 lang 解析为字符串（非 JSON）
    assert!(arr[0]["name"].as_str().unwrap().contains("支付宝"));
}

#[tokio::test]
async fn create_payment_disabled_channel_400() {
    let Some(db) = connect_test_db().await else { return };
    let order_id = insert_order(&db, 900103, 100).await;
    let r = router(state_with(db.clone(), MockConfirm::new(Ok(()))));
    // 停用 usdt
    db.execute_with("UPDATE travel_payment_channels SET enabled = 0 WHERE channel_code = 'usdt'", &[]).await.unwrap();
    let (status, body) = call(r.clone(), "POST", "/api/payments", Some(&sign("900103")), Some(pay_body(order_id, "usdt"))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["code"], 400);
    // 未知渠道 404
    let (status, body) = call(r, "POST", "/api/payments", Some(&sign("900103")), Some(pay_body(order_id, "nope"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    db.execute_with("UPDATE travel_payment_channels SET enabled = 1 WHERE channel_code = 'usdt'", &[]).await.unwrap();
    cleanup(&db, order_id).await;
}

#[tokio::test]
async fn callback_bad_signature_401_not_booked() {
    let Some(db) = connect_test_db().await else { return };
    let order_id = insert_order(&db, 900104, 8888).await;
    let confirm = MockConfirm::new(Ok(()));
    let r = router(state_with(db.clone(), confirm.clone()));
    let (_, body) = call(r.clone(), "POST", "/api/payments", Some(&sign("900104")), Some(pay_body(order_id, "stripe"))).await;
    let txn = body["data"]["txn_no"].as_str().unwrap().to_string();
    // 无签名 / 错误签名 → 401
    let (status, _) = callback(r.clone(), "stripe", &txn, 1, false).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let mut wrong = Request::builder()
        .method("POST")
        .uri("/api/payments/callback/stripe")
        .header("content-type", "application/json")
        .header("x-api-version", "v1")
        .header("x-signature", "deadbeef")
        .body(Body::from(json!({"txn_no": txn, "status": 1}).to_string()))
        .unwrap();
    let resp = r.clone().oneshot(wrong).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    // 未入账
    let rows = db
        .query_with("SELECT CAST(status AS CHAR) AS status FROM travel_payments WHERE order_id = ?", &[json!(order_id)])
        .await
        .unwrap();
    assert_eq!(rows[0].get("status").and_then(|v| v.as_str()).unwrap(), "0");
    assert_eq!(confirm.call_count(), 0);
    cleanup(&db, order_id).await;
}

#[tokio::test]
async fn callback_success_books_and_confirms() {
    let Some(db) = connect_test_db().await else { return };
    let order_id = insert_order(&db, 900105, 6600).await;
    let confirm = MockConfirm::new(Ok(()));
    let r = router(state_with(db.clone(), confirm.clone()));
    let (_, body) = call(r.clone(), "POST", "/api/payments", Some(&sign("900105")), Some(pay_body(order_id, "stripe"))).await;
    let txn = body["data"]["txn_no"].as_str().unwrap().to_string();
    let (status, body) = callback(r.clone(), "stripe", &txn, 1, true).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["code"], 0);
    let rows = db
        .query_with(
            "SELECT CAST(status AS CHAR) AS status, CAST(paid_at AS CHAR) AS paid_at \
             FROM travel_payments WHERE order_id = ?",
            &[json!(order_id)],
        )
        .await
        .unwrap();
    assert_eq!(rows[0].get("status").and_then(|v| v.as_str()).unwrap(), "1");
    assert!(!rows[0].get("paid_at").and_then(|v| v.as_str()).unwrap_or("").is_empty());
    assert_eq!(confirm.call_count(), 1, "成功回调应调 order-service 确认一次");
    cleanup(&db, order_id).await;
}

#[tokio::test]
async fn callback_idempotent_no_double_confirm() {
    let Some(db) = connect_test_db().await else { return };
    let order_id = insert_order(&db, 900106, 6600).await;
    let confirm = MockConfirm::new(Ok(()));
    let r = router(state_with(db.clone(), confirm.clone()));
    let (_, body) = call(r.clone(), "POST", "/api/payments", Some(&sign("900106")), Some(pay_body(order_id, "stripe"))).await;
    let txn = body["data"]["txn_no"].as_str().unwrap().to_string();
    let (s1, b1) = callback(r.clone(), "stripe", &txn, 1, true).await;
    let (s2, b2) = callback(r.clone(), "stripe", &txn, 1, true).await;
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(s2, StatusCode::OK, "重复回调应返回成功: {b1} {b2}");
    let rows = db
        .query_with("SELECT CAST(status AS CHAR) AS status FROM travel_payments WHERE order_id = ?", &[json!(order_id)])
        .await
        .unwrap();
    assert_eq!(rows[0].get("status").and_then(|v| v.as_str()).unwrap(), "1");
    assert_eq!(confirm.call_count(), 1, "重复回调不重复入账、不重复确认");
    cleanup(&db, order_id).await;
}

#[tokio::test]
async fn callback_failure_marks_failed() {
    let Some(db) = connect_test_db().await else { return };
    let order_id = insert_order(&db, 900107, 3300).await;
    let confirm = MockConfirm::new(Ok(()));
    let r = router(state_with(db.clone(), confirm.clone()));
    let (_, body) = call(r.clone(), "POST", "/api/payments", Some(&sign("900107")), Some(pay_body(order_id, "stripe"))).await;
    let txn = body["data"]["txn_no"].as_str().unwrap().to_string();
    let (status, body) = callback(r, "stripe", &txn, 2, true).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    let rows = db
        .query_with("SELECT CAST(status AS CHAR) AS status FROM travel_payments WHERE order_id = ?", &[json!(order_id)])
        .await
        .unwrap();
    assert_eq!(rows[0].get("status").and_then(|v| v.as_str()).unwrap(), "2");
    assert_eq!(confirm.call_count(), 0, "失败回调不触发订单确认");
    cleanup(&db, order_id).await;
}

// 无 DB 时的 401 用例：构造无数据源 state（仅 jwt 就位）
fn state_with_db_or_none() -> AppState {
    AppState {
        db: None,
        confirm: Arc::new(MockConfirm::new(Ok(()))),
        jwt: JwtAuthLayer::new(jwt_secret()).unwrap(),
    }
}
