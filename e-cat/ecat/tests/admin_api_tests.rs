// P2-06 集成测试：admin login（离线降级分支 + 真实 DB 路径跳过策略）。
// 离线（db/cache None）：缺字段 400、DB 缺失 503、JWT 签发 role=admin、
// require_admin 守卫 403、版本头缺失 400。
// 真实路径（登录成功/密码错 401/未知邮箱 401）依赖本机 MySQL（localhost:3308，
// 种子管理员 admin@travel.local / Admin@123），连不上时跳过。
#![cfg(test)]

#[path = "../src/business/admin/mod.rs"]
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
use service::line_date_handlers::*;
use service::line_handlers::*;
use service::*;
use ecat::business::shared::jwt_secret;
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



// ===== 线路 itinerary 双向转换（纯函数，无 DB）=====

#[test]
fn itinerary_roundtrip_frontend_array() {
    let front = r#"[
        {"day":1,"title":{"en":"E1","zh":"中1","ja":"日1","ko":"","ru":""},"description":{"en":"d-en","zh":"d-zh","ja":"","ko":"","ru":""}},
        {"day":2,"title":{"en":"E2","zh":"中2","ja":"日2","ko":"韩2","ru":"俄2"},"description":{"ja":"only-ja"}}
    ]"#;
    // 入库：数组 → {"days":[...]}，title 平铺、description 取 zh 优先
    let stored = itinerary_to_storage(front);
    let v: serde_json::Value = serde_json::from_str(&stored).unwrap();
    assert!(v["days"].is_array(), "存储应为 days 数组: {v}");
    assert_eq!(v["days"][0]["title_zh"], "中1");
    assert_eq!(v["days"][0]["title_en"], "E1");
    assert_eq!(v["days"][0]["description"], "d-zh", "zh 优先");
    assert_eq!(v["days"][1]["description"], "only-ja", "无 zh/en 时取非空语言");
    assert_eq!(v["days"][1]["title_ru"], "俄2");
    assert_eq!(v["days"][1]["title_ko"], "韩2");
    // 出参：{"days":[...]} → 前端数组，title 合成对象、description 回填 zh
    let back = itinerary_from_storage(&stored);
    let out: serde_json::Value = serde_json::from_str(&back).unwrap();
    assert!(out.is_array());
    assert_eq!(out[0]["title"]["zh"], "中1");
    assert_eq!(out[0]["title"]["ja"], "日1");
    assert_eq!(out[0]["description"]["zh"], "d-zh");
    assert_eq!(out[1]["title"]["ko"], "韩2");
    assert_eq!(out[1]["description"], serde_json::json!({"zh": "only-ja"}));
    // 前端数组格式直接出参：语义等值（紧凑序列化，键序可能不同）
    let direct: serde_json::Value = serde_json::from_str(&itinerary_from_storage(front)).unwrap();
    let expect: serde_json::Value = serde_json::from_str(front).unwrap();
    assert_eq!(direct, expect);
}

#[test]
fn itinerary_legacy_days_format_compat() {
    let legacy = r#"{"days":[{"day":1,"title_en":"Old","title_zh":"旧","description":"legacy desc"}]}"#;
    // 老数据转前端格式不报错
    let out: serde_json::Value = serde_json::from_str(&itinerary_from_storage(legacy)).unwrap();
    assert_eq!(out[0]["title"]["en"], "Old");
    assert_eq!(out[0]["title"]["ja"], "");
    assert_eq!(out[0]["description"]["zh"], "legacy desc");
    // 老格式再入库：仍为 days 数组结构
    let again: serde_json::Value = serde_json::from_str(&itinerary_to_storage(legacy)).unwrap();
    assert_eq!(again["days"][0]["title_zh"], "旧");
    // 不可解析内容原样返回
    assert_eq!(itinerary_to_storage("not-json"), "not-json");
    assert_eq!(itinerary_from_storage("not-json"), "not-json");
}

// ===== 线路/班期 CRUD：真实 DB 全流程（MySQL 3308 不可用时跳过）=====

#[tokio::test]
async fn lines_crud_full_flow() {
    let Some(db) = real_db().await else { return };
    let st = state_with(db);
    let kw = unique_name("t-line");
    let itin = serde_json::json!([
        {"day": 1, "title": {"en": "Day1", "zh": "第一天", "ja": "", "ko": "", "ru": ""},
         "description": {"en": "desc-en", "zh": "描述", "ja": "", "ko": "", "ru": ""}}
    ]);
    let itin_str = serde_json::to_string(&itin).unwrap();

    // 前置：目的地
    let (status, body) = body_json(create_destination(
        State(st.clone()),
        admin_guard(),
        Json(json!({ "name_en": format!("{kw}-dest"), "name_zh": "线路测试目的地" })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "create dest failed: {body}");
    let dest_id = body["data"]["id"].as_u64().unwrap();

    // 缺 title_zh → 400
    let (status, body) = body_json(create_line(
        State(st.clone()),
        admin_guard(),
        Json(json!({ "destination_id": dest_id, "title_en": "x" })),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "title_zh is required");

    // 创建线路（含 itinerary 数组格式）
    let (status, body) = body_json(create_line(
        State(st.clone()),
        admin_guard(),
        Json(json!({
            "title_en": format!("{kw} line"),
            "title_zh": format!("{kw} 线路"),
            "title_ja": "テスト",
            "destination_id": dest_id,
            "days": 3,
            "departure_date": "2026-10-01",
            "price_cents": 88800,
            "max_pax": 15,
            "itinerary": itin_str,
            "status": 1,
            "cover_url": "https://img.example/line.jpg",
        })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "create line failed: {body}");
    let line_id = body["data"]["id"].as_u64().unwrap();
    assert_eq!(body["data"]["price_cents"], 88800);
    assert_eq!(body["data"]["departure_date"], "2026-10-01");
    // itinerary 出参须为前端数组格式字符串
    let out_itin: serde_json::Value = serde_json::from_str(body["data"]["itinerary"].as_str().unwrap()).unwrap();
    assert_eq!(out_itin[0]["title"]["zh"], "第一天");
    assert_eq!(out_itin[0]["description"]["zh"], "描述");

    // 列表：items 键 + keyword 命中
    let (status, body) = body_json(list_lines(
        State(st.clone()),
        admin_guard(),
        Query(PageQuery { page: 1, page_size: 10, status: None, keyword: Some(kw.clone()), destination_id: None }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 1, "keyword 应命中 1 条: {body}");
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 1);

    // 更新：改价格 + 换 itinerary
    let itin2 = serde_json::json!([
        {"day": 1, "title": {"en": "N1", "zh": "新一天", "ja": "", "ko": "", "ru": ""},
         "description": {"en": "", "zh": "新描述", "ja": "", "ko": "", "ru": ""}}
    ]);
    let (status, body) = body_json(update_line(
        State(st.clone()),
        admin_guard(),
        Path(line_id),
        Json(json!({ "price_cents": 99900, "itinerary": serde_json::to_string(&itin2).unwrap() })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "update line failed: {body}");
    assert_eq!(body["data"]["price_cents"], 99900);
    let out_itin: serde_json::Value = serde_json::from_str(body["data"]["itinerary"].as_str().unwrap()).unwrap();
    assert_eq!(out_itin[0]["title"]["zh"], "新一天");

    // 上下架
    let (status, body) = body_json(update_line_status(
        State(st.clone()),
        admin_guard(),
        Path(line_id),
        Json(StatusReq { status: 0 }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], 0);

    // 班期：创建 / 重复 409 / 列表裸数组 / 更新 / 删除
    let (status, body) = body_json(create_line_date(
        State(st.clone()),
        admin_guard(),
        Path(line_id),
        Json(json!({ "depart_date": "2026-11-05", "price_cents": 90000, "seats_left": 8 })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "create date failed: {body}");
    let date_id = body["data"]["id"].as_u64().unwrap();
    assert_eq!(body["data"]["depart_date"], "2026-11-05");
    assert_eq!(body["data"]["status"], 1, "缺省上架");

    let (status, body) = body_json(create_line_date(
        State(st.clone()),
        admin_guard(),
        Path(line_id),
        Json(json!({ "depart_date": "2026-11-05", "price_cents": 1 })),
    ).await).await;
    assert_eq!(status, StatusCode::CONFLICT, "重复日期应 409: {body}");
    assert_eq!(body["message"], "depart date already exists");

    let (status, body) = body_json(create_line_date(
        State(st.clone()),
        admin_guard(),
        Path(line_id),
        Json(json!({ "depart_date": "2026-11-06" })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "create date2 failed: {body}");
    let date2_id = body["data"]["id"].as_u64().unwrap();

    // 列表：data 为裸数组
    let (status, body) = body_json(list_line_dates(
        State(st.clone()),
        admin_guard(),
        Path(line_id),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 2);

    // 更新班期（价格 + 停售）
    let (status, body) = body_json(update_line_date(
        State(st.clone()),
        admin_guard(),
        Path((line_id, date_id)),
        Json(json!({ "price_cents": 95000, "seats_left": 5, "status": 0 })),
    ).await).await;
    assert_eq!(status, StatusCode::OK, "update date failed: {body}");
    assert_eq!(body["data"]["price_cents"], 95000);
    assert_eq!(body["data"]["status"], 0);

    // 改到已存在日期 → 409
    let (status, _) = body_json(update_line_date(
        State(st.clone()),
        admin_guard(),
        Path((line_id, date_id)),
        Json(json!({ "depart_date": "2026-11-06" })),
    ).await).await;
    assert_eq!(status, StatusCode::CONFLICT);

    // 有关联班期时删线路 → 409；删完班期后可删
    let (status, body) = body_json(delete_line(State(st.clone()), admin_guard(), Path(line_id)).await).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], 409);
    for d in [date_id, date2_id] {
        let (status, _) = body_json(delete_line_date(State(st.clone()), admin_guard(), Path((line_id, d))).await).await;
        assert_eq!(status, StatusCode::OK);
    }
    let (status, _) = body_json(delete_line(State(st.clone()), admin_guard(), Path(line_id)).await).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = body_json(delete_destination(State(st.clone()), admin_guard(), Path(dest_id)).await).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn line_and_date_not_found_404() {
    let Some(db) = real_db().await else { return };
    let st = state_with(db);
    let (status, body) = body_json(update_line(
        State(st.clone()),
        admin_guard(),
        Path(99_999_999),
        Json(json!({ "title_zh": "x" })),
    ).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "line not found");
    let (status, _) = body_json(delete_line(State(st.clone()), admin_guard(), Path(99_999_999)).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // 不存在的线路挂班期 → 404
    let (status, body) = body_json(create_line_date(
        State(st.clone()),
        admin_guard(),
        Path(99_999_999),
        Json(json!({ "depart_date": "2026-12-01", "price_cents": 1 })),
    ).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "line not found");
    let (status, _) = body_json(update_line_date(
        State(st.clone()),
        admin_guard(),
        Path((99_999_999, 99_999_999)),
        Json(json!({ "price_cents": 1 })),
    ).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = body_json(delete_line_date(
        State(st.clone()),
        admin_guard(),
        Path((99_999_999, 99_999_999)),
    ).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // 非法日期格式 → 400
    let (status, body) = body_json(create_line_date(
        State(st.clone()),
        admin_guard(),
        Path(99_999_999),
        Json(json!({ "depart_date": "11/05/2026", "price_cents": 1 })),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "depart_date must be YYYY-MM-DD");
}
