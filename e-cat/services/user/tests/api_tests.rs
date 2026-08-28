// P1-04 集成测试：直接调用 handler，覆盖 api.md 错误码语义
// （400 邮箱/密码、401 统一防枚举、503 DB 缺失降级、JWT 签发 claims）。
// 离线运行：state 中 db/cache 均为 None，不依赖 MySQL/Redis；
// DB 真实路径（重复检查/插入/查询）无法离线覆盖，此处验证无 DB 时的
// 降级分支，真实路径由部署环境冒烟验证。
//
// main.rs 是 binary crate，tests/ 无法直接访问其私有项，
// 故经 #[path] 以模块方式包含源码（配合 main.rs 中最小 pub(crate) 改动）。
#![cfg(test)]

#[path = "../src/main.rs"]
mod service;

use axum::body::Body;
use axum::extract::{Json, State};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use ecat_auth::{AuthClaims, JwtAuthLayer};
use ecat_data::RdbmsClient;
use ecat_data_sqlx::SqlxClient;
use serde_json::json;
use service::*;
use shared::jwt_secret;
use std::collections::HashMap;
use std::sync::Arc;

fn state() -> AppState {
    AppState { db: None, cache: None, mq: None, jwt: JwtAuthLayer::new(jwt_secret()).unwrap() }
}

async fn body_json(resp: impl IntoResponse) -> (StatusCode, serde_json::Value) {
    let (parts, body) = resp.into_response().into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    (parts.status, serde_json::from_slice(&bytes).unwrap())
}

fn req_with_claims(sub: &str) -> Request<Body> {
    let mut req = Request::new(Body::empty());
    req.extensions_mut().insert(AuthClaims {
        sub: sub.into(),
        exp: None,
        iat: None,
        role: None,
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

#[tokio::test]
async fn register_rejects_invalid_email_400() {
    let (status, body) = body_json(register(
        State(state()),
        Json(RegisterReq { email: "not-an-email".into(), password: "secret123".into(), lang: None }),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], 400);
    assert_eq!(body["message"], "invalid email");
}

#[tokio::test]
async fn register_rejects_short_password_400() {
    let (status, body) = body_json(register(
        State(state()),
        Json(RegisterReq { email: "a@b.com".into(), password: "12345".into(), lang: None }),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], 400);
    assert_eq!(body["message"], "password must be at least 6 characters");
}

#[tokio::test]
async fn register_db_unavailable_503() {
    let (status, body) = body_json(register(
        State(state()),
        Json(RegisterReq { email: "new@example.com".into(), password: "secret123".into(), lang: None }),
    ).await).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], 503);
    assert_eq!(body["message"], "database unavailable");
}

#[tokio::test]
async fn login_empty_email_returns_uniform_401() {
    // 空邮箱/纯空格 trim 后为空，与「密码错误」同样返回 401，防账号枚举
    let (status, body) = body_json(login(
        State(state()),
        Json(LoginReq { email: "   ".into(), password: "whatever".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], 401);
    assert_eq!(body["message"], "invalid credentials");
}

#[tokio::test]
async fn login_db_unavailable_503() {
    let (status, body) = body_json(login(
        State(state()),
        Json(LoginReq { email: "a@b.com".into(), password: "secret123".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], 503);
}

#[tokio::test]
async fn profile_missing_claims_401() {
    let resp = profile(State(state()), Request::new(Body::empty())).await;
    let (status, body) = body_json(resp).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], 401);
    assert_eq!(body["message"], "missing claims");
}

#[tokio::test]
async fn profile_invalid_subject_400() {
    let resp = profile(State(state()), req_with_claims("not-a-u64")).await;
    let (status, body) = body_json(resp).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], 400);
    assert_eq!(body["message"], "invalid subject in token");
}

#[tokio::test]
async fn jwt_sign_injects_iat_exp_and_sub() {
    let jwt = JwtAuthLayer::new(jwt_secret()).unwrap();
    let token = jwt.sign(&LoginClaims { sub: "42".into() }, 86400).unwrap();
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "jwt 应为 header.payload.signature 三段");
    let payload: serde_json::Value = serde_json::from_slice(&b64url_decode(parts[1])).unwrap();
    assert_eq!(payload["sub"], "42");
    let iat = payload["iat"].as_u64().unwrap();
    let exp = payload["exp"].as_u64().unwrap();
    assert_eq!(exp - iat, 86400, "exp = iat + TTL");
}

#[tokio::test]
async fn update_profile_missing_claims_401() {
    let resp = update_profile(State(state()), Request::new(Body::empty())).await;
    let (status, body) = body_json(resp).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], 401);
    assert_eq!(body["message"], "missing claims");
}

#[tokio::test]
async fn update_profile_rejects_unsupported_lang_400() {
    let mut req = Request::builder()
        .method("PUT")
        .uri("/api/user/profile")
        .body(Body::from(r#"{"lang":"xx"}"#))
        .unwrap();
    req.extensions_mut().insert(AuthClaims {
        sub: "1".into(),
        exp: None,
        iat: None,
        role: None,
        extra: HashMap::new(),
    });
    let resp = update_profile(State(state()), req).await;
    let (status, body) = body_json(resp).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], 400);
    assert_eq!(body["message"], "unsupported lang");
}

#[tokio::test]
async fn update_profile_db_unavailable_503() {
    let mut req = Request::builder()
        .method("PUT")
        .uri("/api/user/profile")
        .body(Body::from(r#"{"nickname":"A","lang":"zh"}"#))
        .unwrap();
    req.extensions_mut().insert(AuthClaims {
        sub: "1".into(),
        exp: None,
        iat: None,
        role: None,
        extra: HashMap::new(),
    });
    let resp = update_profile(State(state()), req).await;
    let (status, body) = body_json(resp).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], 503);
}

/// 本机 MySQL（docker compose 映射 3308）；连不上返回 None，测试跳过。
async fn user_real_db() -> Option<SqlxClient> {
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
async fn update_profile_updates_nickname_and_lang() {
    let Some(db) = user_real_db().await else { return };
    let email = format!("profile-test-{}@example.com", std::process::id());
    let hash = bcrypt::hash("secret123", bcrypt::DEFAULT_COST).unwrap();
    let _ = db
        .execute_with(
            "INSERT INTO travel_users (email, password_hash) VALUES (?, ?)",
            &[json!(email), json!(hash)],
        )
        .await;
    let rows = db
        .query_with("SELECT id FROM travel_users WHERE email = ?", &[json!(email)])
        .await
        .unwrap();
    let Some(id) = rows.first().and_then(|r| r.get("id")).and_then(|v| v.as_u64()) else {
        return;
    };
    let mut req = Request::builder()
        .method("PUT")
        .uri("/api/user/profile")
        .body(Body::from(r#"{"nickname":"Alice","lang":"zh"}"#))
        .unwrap();
    req.extensions_mut().insert(AuthClaims {
        sub: id.to_string(),
        exp: None,
        iat: None,
        role: None,
        extra: HashMap::new(),
    });
    let db_arc = Arc::new(db);
    let state = AppState {
        db: Some(db_arc.clone()),
        cache: None,
        mq: None,
        jwt: JwtAuthLayer::new(jwt_secret()).unwrap(),
    };
    let resp = update_profile(State(state), req).await;
    let (status, body) = body_json(resp).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], id);
    assert_eq!(body["data"]["email"], email);
    assert_eq!(body["data"]["nickname"], "Alice");
    assert_eq!(body["data"]["lang"], "zh");
    let _ = db_arc.execute_with("DELETE FROM travel_users WHERE id = ?", &[json!(id)]).await;
}

#[tokio::test]
async fn ready_reports_degraded_without_datasources() {
    let (status, body) = body_json(ready(State(state())).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], false);
}
