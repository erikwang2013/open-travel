// P2-06 集成测试：admin login（离线降级分支 + 真实 DB 路径跳过策略）。
// 离线（db/cache None）：缺字段 400、DB 缺失 503、JWT 签发 role=admin、
// require_admin 守卫 403、版本头缺失 400。
// 真实路径（登录成功/密码错 401/未知邮箱 401）依赖本机 MySQL（localhost:3308，
// 种子管理员 admin@travel.local / Admin@123），连不上时跳过。
#![cfg(test)]

#[path = "../src/main.rs"]
mod service;

use axum::body::Body;
use axum::extract::{Json, Path, Query, State};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use ecat_auth::{AuthClaims, JwtAuthLayer};
use ecat_data::RdbmsClient;
use ecat_data_sqlx::SqlxClient;
use serde_json::json;
use service::handlers::*;
use service::*;
use shared::jwt_secret;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

fn state() -> AppState {
    AppState { db: None, cache: None, mq: None, jwt: JwtAuthLayer::new(jwt_secret()).unwrap() }
}

fn state_with(db: SqlxClient) -> AppState {
    AppState { db: Some(Arc::new(db)), cache: None, mq: None, jwt: JwtAuthLayer::new(jwt_secret()).unwrap() }
}

async fn body_json(resp: impl IntoResponse) -> (StatusCode, serde_json::Value) {
    let (parts, body) = resp.into_response().into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    (parts.status, serde_json::from_slice(&bytes).unwrap())
}

fn req_with_claims(sub: &str, role: Option<&str>) -> Request<Body> {
    let mut req = Request::new(Body::empty());
    req.extensions_mut().insert(AuthClaims {
        sub: sub.into(),
        exp: None,
        iat: None,
        role: role.map(String::from),
        extra: HashMap::new(),
    });
    req
}

fn b64url_decode(s: &str) -> Vec<u8> {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let s = s.replace('-', "+").replace('_', "/");
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0u32;
    for &b in s.as_bytes() {
        if let Some(v) = T.iter().position(|&t| t == b) {
            acc = (acc << 6) | v as u32;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
                acc &= (1 << bits) - 1;
            }
        }
    }
    out
}

/// 本机 MySQL（docker compose 映射 3308）；连不上返回 None，测试跳过真实路径。
async fn real_db() -> Option<SqlxClient> {
    let url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:travel_dev@localhost:3308/travel".into());
    match SqlxClient::connect(&url).await {
        Ok(db) => Some(db),
        Err(e) => {
            eprintln!("skip real-db test (mysql unreachable): {e}");
            None
        }
    }
}

#[tokio::test]
async fn login_missing_fields_400() {
    let (status, body) = body_json(login(
        State(state()),
        Json(LoginReq { email: "".into(), password: "".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], 400);
    assert_eq!(body["message"], "email and password required");
}

#[tokio::test]
async fn login_db_unavailable_503() {
    let (status, body) = body_json(login(
        State(state()),
        Json(LoginReq { email: "admin@travel.local".into(), password: "Admin@123".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], 503);
}

#[tokio::test]
async fn jwt_sign_issues_admin_role_claim() {
    let jwt = JwtAuthLayer::new(jwt_secret()).unwrap();
    let token = jwt.sign(&LoginClaims { sub: "1".into(), role: "admin".into() }, 86400).unwrap();
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "jwt 应为 header.payload.signature 三段");
    let payload: serde_json::Value = serde_json::from_slice(&b64url_decode(parts[1])).unwrap();
    assert_eq!(payload["sub"], "1");
    assert_eq!(payload["role"], "admin");
    let iat = payload["iat"].as_u64().unwrap();
    let exp = payload["exp"].as_u64().unwrap();
    assert_eq!(exp - iat, 86400, "exp = iat + TTL");
}

#[tokio::test]
async fn require_admin_rejects_missing_claims_401() {
    let resp = require_admin(&Request::new(Body::empty())).unwrap_err();
    let (status, body) = body_json(resp).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], 401);
    assert_eq!(body["message"], "missing claims");
}

#[tokio::test]
async fn require_admin_rejects_non_admin_role_403() {
    let req = req_with_claims("1", Some("editor"));
    let resp = require_admin(&req).unwrap_err();
    let (status, body) = body_json(resp).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], "admin role required");
}

#[tokio::test]
async fn require_admin_accepts_admin_role() {
    let req = req_with_claims("1", Some("admin"));
    let claims = require_admin(&req).unwrap();
    assert_eq!(claims.subject(), "1");
    assert!(claims.has_role("admin"));
}

#[tokio::test]
async fn missing_api_version_returns_400() {
    use tower::ServiceExt;
    let router = api_router(state());
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email":"a@b.c","password":"x"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    // 带上版本头则通过版本层，落到 handler → db None → 503
    let resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/login")
                .header("x-api-version", "v1")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email":"a@b.c","password":"x"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn login_success_issues_admin_token() {
    let Some(db) = real_db().await else { return };
    let (status, body) = body_json(login(
        State(state_with(db)),
        Json(LoginReq { email: "admin@travel.local".into(), password: "Admin@123".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["code"], 0);
    let token = body["data"]["token"].as_str().expect("token present");
    let payload: serde_json::Value =
        serde_json::from_slice(&b64url_decode(token.split('.').nth(1).unwrap())).unwrap();
    assert_eq!(payload["role"], "admin");
    assert!(payload["sub"].as_str().is_some_and(|s| !s.is_empty()));
}

#[tokio::test]
async fn login_wrong_password_401() {
    let Some(db) = real_db().await else { return };
    let (status, body) = body_json(login(
        State(state_with(db)),
        Json(LoginReq { email: "admin@travel.local".into(), password: "WrongPass1".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], 401);
    assert_eq!(body["message"], "invalid credentials");
}

#[tokio::test]
async fn login_unknown_email_uniform_401() {
    let Some(db) = real_db().await else { return };
    let (status, body) = body_json(login(
        State(state_with(db)),
        Json(LoginReq { email: "ghost@travel.local".into(), password: "Admin@123".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], 401);
    assert_eq!(body["message"], "invalid credentials");
}

/// 临时：生成种子管理员与防枚举 dummy hash（运行后移除此测试）
#[test]
fn print_seed_hashes() {
    let admin = bcrypt::hash("Admin@123", bcrypt::DEFAULT_COST).unwrap();
    let dummy = bcrypt::hash("dummy-password-for-timing", bcrypt::DEFAULT_COST).unwrap();
    println!("SEED_HASH={admin}");
    println!("DUMMY_HASH={dummy}");
}

#[tokio::test]
async fn ready_reports_degraded_without_datasources() {
    let (status, body) = body_json(ready(State(state())).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], false);
}

// ===== 管理端 CRUD：离线分支（db None）=====

fn admin_guard() -> AdminGuard {
    AdminGuard(AuthClaims {
        sub: "1".into(),
        exp: None,
        iat: None,
        role: Some("admin".into()),
        extra: HashMap::new(),
    })
}

fn signed_token(role: &str) -> String {
    let jwt = JwtAuthLayer::new(jwt_secret()).unwrap();
    jwt.sign(&LoginClaims { sub: "1".into(), role: role.into() }, 3600).unwrap()
}

fn page_q(page: u64, page_size: u64) -> Query<PageQuery> {
    Query(PageQuery { page, page_size, status: None, keyword: None, destination_id: None })
}

/// 走完整路由链（JWT 层 → AdminGuard）：无 token 401、非 admin 403、db 缺失 503。
async fn router_call(method: &str, uri: &str, token: Option<&str>) -> (StatusCode, serde_json::Value) {
    use tower::ServiceExt;
    let router = api_router(state());
    let mut req = Request::builder().method(method).uri(uri).header("x-api-version", "v1");
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let resp = router.clone().oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, body)
}

#[tokio::test]
async fn crud_without_token_401() {
    let (status, body) = router_call("GET", "/api/admin/destinations", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // JwtAuthLayer 直接拒绝，body 为 {"error":...} 非标准信封
    assert_eq!(body["error"], "missing authorization token");
}

#[tokio::test]
async fn crud_non_admin_token_403() {
    let (status, body) = router_call("GET", "/api/admin/destinations", Some(&signed_token("user"))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], 403);
    assert_eq!(body["message"], "admin role required");
}

#[tokio::test]
async fn crud_db_unavailable_503() {
    let (status, body) = router_call("GET", "/api/admin/destinations", Some(&signed_token("admin"))).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], 503);
}

#[tokio::test]
async fn create_destination_missing_names_400() {
    let (status, body) = body_json(create_destination(
        State(state()),
        admin_guard(),
        Json(json!({ "name_en": "x" })),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], 400);
    assert_eq!(body["message"], "name_en and name_zh are required");
}

#[tokio::test]
async fn create_attraction_missing_name_or_destination_400() {
    let (status, body) = body_json(create_attraction(
        State(state()),
        admin_guard(),
        Json(json!({ "destination_id": 1 })),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "name_en is required");
    let (status, body) = body_json(create_attraction(
        State(state()),
        admin_guard(),
        Json(json!({ "name_en": "x" })),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "destination_id is required");
}

#[tokio::test]
async fn update_destination_empty_body_400() {
    let (status, body) = body_json(update_destination(
        State(state()),
        admin_guard(),
        Path(1),
        Json(json!({})),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], 400);
    assert_eq!(body["message"], "no fields to update");
}

#[tokio::test]
async fn status_must_be_0_or_1_400() {
    let (status, body) = body_json(update_destination_status(
        State(state()),
        admin_guard(),
        Path(1),
        Json(StatusReq { status: 5 }),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], 400);
}

// ===== 管理端 CRUD：真实 DB 全流程（MySQL 3308 不可用时跳过）=====

fn unique_name(prefix: &str) -> String {
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    format!("{prefix}-{ms}")
}

#[tokio::test]
async fn destinations_crud_full_flow() {
    let Some(db) = real_db().await else { return };
    let st = state_with(db);
    let kw = unique_name("t-dest");

    for i in 0..2 {
        let (status, body) = body_json(create_destination(
            State(st.clone()),
            admin_guard(),
            Json(json!({
                "name_en": format!("{kw}-{i}"),
                "name_zh": format!("测试{i}"),
                "name_ja": "テスト",
                "description": { "en": "desc" },
                "cover_url": "https://img.example/x.jpg",
                "status": 1,
                "sort_order": i,
                "latitude": 1.23,
                "longitude": 4.56,
                "region_id": 1,
                "category": "city",
            })),
        ).await).await;
        assert_eq!(status, StatusCode::OK, "create {i} failed: {body}");
        assert_eq!(body["code"], 0);
        assert!(body["data"]["id"].as_u64().is_some(), "created row id missing: {body}");
        // description 需还原为 JSON 对象（驱动返回 base64，handler 兜底解码）
        assert_eq!(body["data"]["description"], json!({"en": "desc"}), "desc: {body}");
    }

    // 分页 + keyword 过滤 + status 过滤
    let (status, body) = body_json(list_destinations(
        State(st.clone()),
        admin_guard(),
        Query(PageQuery {
            page: 1,
            page_size: 1,
            status: Some("1".into()),
            keyword: Some(kw.clone()),
            destination_id: None,
        }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 2, "keyword 应命中刚创建的 2 条");
    assert_eq!(body["data"]["page"], 1);
    assert_eq!(body["data"]["page_size"], 1);
    assert_eq!(body["data"]["list"].as_array().unwrap().len(), 1);
    let created_id = body["data"]["list"][0]["id"].as_u64().unwrap();

    // 部分更新
    let (status, body) = body_json(update_destination(
        State(st.clone()),
        admin_guard(),
        Path(created_id),
        Json(json!({ "name_zh": "改名" })),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["name_zh"], "改名");
    assert_eq!(body["data"]["name_en"], format!("{kw}-0"));

    // 上下架
    let (status, body) = body_json(update_destination_status(
        State(st.clone()),
        admin_guard(),
        Path(created_id),
        Json(StatusReq { status: 0 }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], 0, "toggle resp: {body}");
    let (_, body) = body_json(list_destinations(
        State(st.clone()),
        admin_guard(),
        Query(PageQuery {
            page: 1,
            page_size: 10,
            status: Some("0".into()),
            keyword: Some(kw.clone()),
            destination_id: None,
        }),
    ).await).await;
    assert_eq!(body["data"]["total"], 1, "下架后 status=0 过滤应命中 1 条");

    // 清理
    for row in body["data"]["list"].as_array().unwrap() {
        let id = row["id"].as_u64().unwrap();
        let (status, body) = body_json(delete_destination(State(st.clone()), admin_guard(), Path(id)).await).await;
        assert_eq!(status, StatusCode::OK, "delete failed: {body}");
    }
}

#[tokio::test]
async fn attractions_crud_flow_with_409() {
    let Some(db) = real_db().await else { return };
    let st = state_with(db);
    let kw = unique_name("t-attr");

    let (status, body) = body_json(create_destination(
        State(st.clone()),
        admin_guard(),
        Json(json!({ "name_en": kw.clone(), "name_zh": "景区目的地" })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "create dest failed: {body}");
    let dest_id = body["data"]["id"].as_u64().unwrap();

    // 外键校验：不存在的 destination_id → 400
    let (status, body) = body_json(create_attraction(
        State(st.clone()),
        admin_guard(),
        Json(json!({ "destination_id": 99_999_999, "name_en": "x" })),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "destination not found");

    let (status, body) = body_json(create_attraction(
        State(st.clone()),
        admin_guard(),
        Json(json!({
            "destination_id": dest_id,
            "name_en": format!("{kw}-spot"),
            "name_zh": "景区",
            "name_ja": "スポット",
            "price_cents": 8800,
            "open_hours": "09:00-17:00",
            "rating_avg": 4.5,
            "cover_url": "https://img.example/spot.jpg",
            "description": { "en": "spot desc" },
        })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "create attr failed: {body}");
    let attr_id = body["data"]["id"].as_u64().unwrap();
    assert_eq!(body["data"]["price_cents"], 8800);
    assert_eq!(body["data"]["rating_avg"], 4.5);

    // destination_id 过滤
    let (status, body) = body_json(list_attractions(
        State(st.clone()),
        admin_guard(),
        Query(PageQuery { page: 1, page_size: 10, status: None, keyword: None, destination_id: Some(dest_id) }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 1);

    // 部分更新景区
    let (status, body) = body_json(update_attraction(
        State(st.clone()),
        admin_guard(),
        Path(attr_id),
        Json(json!({ "price_cents": 9900, "name_zh": "景区改" })),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["price_cents"], 9900);
    assert_eq!(body["data"]["name_zh"], "景区改");

    // 有关联景区时删目的地 → 409
    let (status, body) = body_json(delete_destination(State(st.clone()), admin_guard(), Path(dest_id)).await).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], 409);

    // 删景区后目的地可删
    let (status, _) = body_json(delete_attraction(State(st.clone()), admin_guard(), Path(attr_id)).await).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = body_json(delete_destination(State(st.clone()), admin_guard(), Path(dest_id)).await).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn update_and_delete_not_found_404() {
    let Some(db) = real_db().await else { return };
    let st = state_with(db);
    let (status, body) = body_json(update_destination(
        State(st.clone()),
        admin_guard(),
        Path(99_999_999),
        Json(json!({ "name_zh": "x" })),
    ).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], 404);
    assert_eq!(body["message"], "destination not found");
    let (status, body) = body_json(delete_destination(State(st.clone()), admin_guard(), Path(99_999_999)).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], 404);
    let (status, body) = body_json(delete_attraction(State(st.clone()), admin_guard(), Path(99_999_999)).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "attraction not found");
}


