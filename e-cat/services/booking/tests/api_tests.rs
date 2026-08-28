// P1-04 集成测试：直接调用 handler，覆盖 api.md 的 dates 接口语义
// （缓存未命中/DB 缺失时的占位兜底、region_id 缺省、ready 降级报告）。
// 离线运行：state 中 db/cache 均为 None，不依赖 MySQL/Redis。
//
// main.rs 是 binary crate，tests/ 无法直接访问其私有项，
// 故经 #[path] 以模块方式包含源码（配合 main.rs 中最小 pub(crate) 改动）。
#![cfg(test)]

#[path = "../src/main.rs"]
mod service;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use ecat_data::{Cache, RdbmsClient};
use ecat_data_redis::RedisCache;
use service::*;
use shared::connect_primary;
use std::sync::Arc;

fn state() -> AppState {
    AppState { db: None, replica: None, cache: None }
}

async fn body_json(resp: impl IntoResponse) -> (StatusCode, serde_json::Value) {
    let (parts, body) = resp.into_response().into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    (parts.status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn available_dates_placeholder_fallback_without_datasource() {
    // 无 DB/Redis 时返回占位目的地，保证接口可响应（Phase 2 前的兜底语义）
    let (status, body) = body_json(available_dates(
        State(state()),
        Query(RegionQuery { region_id: 7 }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["code"], 0);
    assert_eq!(body["message"], "ok");
    let rows = body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["region_id"], 7);
    assert_eq!(rows[0]["name_en"], "placeholder-destination");
    assert_eq!(rows[0]["name_zh"], "占位目的地");
}

#[tokio::test]
async fn available_dates_missing_region_param_defaults_zero() {
    // Query 缺省反序列化：无 region_id 参数时 serde default 为 0
    let q: RegionQuery = serde_json::from_str("{}").unwrap();
    assert_eq!(q.region_id, 0);
    let (status, body) = body_json(available_dates(State(state()), Query(q)).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"][0]["region_id"], 0);
}

#[tokio::test]
async fn ready_reports_degraded_without_datasources() {
    let (status, body) = body_json(ready(State(state())).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], false);
}

#[tokio::test]
async fn attractions_list_requires_destination_id() {
    let (status, body) = body_json(attractions_list(
        State(state()),
        Query(AttractionsQuery { destination_id: None, lang: "en".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], 400);
}

#[tokio::test]
async fn attraction_detail_returns_404_when_not_found() {
    let (status, body) = body_json(attraction_detail(
        State(state()),
        Path(999_999_999u64),
        Query(LangQuery { lang: "en".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "attraction not found");
}

// 以下用例依赖真实 MySQL（种子数据），连接失败时跳过（离线环境测试仍可跑）。

/// 列表按 zh 返回中文名（与表 name_zh 列一致）。
#[tokio::test]
async fn attractions_list_returns_zh_names_with_real_db() {
    let Some(db) = connect_primary().await else { return; };
    let (status, body) = body_json(attractions_list(
        State(AppState { db: Some(db.clone()), replica: None, cache: None }),
        Query(AttractionsQuery { destination_id: Some(2), lang: "zh".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["data"].as_array().unwrap();
    if rows.is_empty() { return; } // 无种子数据环境跳过
    let raw = db.query_with(
        "SELECT name_zh FROM travel_attractions WHERE destination_id = 2 AND status = 1 ORDER BY id LIMIT 1",
        &[],
    ).await.unwrap();
    let expected_zh = raw.first().and_then(|r| r.get("name_zh")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    assert!(!expected_zh.is_empty());
    assert_eq!(rows[0]["name"], expected_zh);
}

/// 未知语种（无对应 name_* 列）回退 name_en；lang 大小写不敏感。
#[tokio::test]
async fn attractions_list_falls_back_to_english_with_real_db() {
    let Some(db) = connect_primary().await else { return; };
    let (status, body) = body_json(attractions_list(
        State(AppState { db: Some(db.clone()), replica: None, cache: None }),
        Query(AttractionsQuery { destination_id: Some(2), lang: "XX".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["data"].as_array().unwrap();
    if rows.is_empty() { return; }
    let raw = db.query_with(
        "SELECT name_en FROM travel_attractions WHERE destination_id = 2 AND status = 1 ORDER BY id LIMIT 1",
        &[],
    ).await.unwrap();
    let expected_en = raw.first().and_then(|r| r.get("name_en")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    assert_eq!(rows[0]["name"], expected_en);
}

/// 无缓存时列表写入 Redis（TTL 5min），二次请求命中缓存。
#[tokio::test]
async fn attractions_list_caches_in_redis_on_miss() {
    let Ok(cache) = RedisCache::connect(&std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6381".into())).await else { return; };
    let Some(db) = connect_primary().await else { return; };
    let cache = Arc::new(cache);
    let key = "travel:attractions:2:en";
    let _ = cache.delete(key).await;
    let (status, _) = body_json(attractions_list(
        State(AppState { db: Some(db), replica: None, cache: Some(cache.clone()) }),
        Query(AttractionsQuery { destination_id: Some(2), lang: "en".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    let cached = cache.get(key).await.unwrap();
    assert!(cached.is_some(), "list response should be cached in redis");

    // 缓存命中路径：无 DB 也返回数据
    let (status2, body2) = body_json(attractions_list(
        State(AppState { db: None, replica: None, cache: Some(cache) }),
        Query(AttractionsQuery { destination_id: Some(2), lang: "en".into() }),
    ).await).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(body2["message"], "cache hit");
}

/// 详情返回完整字段：zh 取中文名与中文描述，reviews 预留空数组；
/// 未知语种 description 回退 en。
#[tokio::test]
async fn attraction_detail_returns_localized_description_with_real_db() {
    let Some(db) = connect_primary().await else { return; };
    let (status, body) = body_json(attraction_detail(
        State(AppState { db: Some(db.clone()), replica: None, cache: None }),
        Path(100000u64),
        Query(LangQuery { lang: "zh".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    if body["data"].is_null() { return; } // 无种子数据环境跳过
    assert_eq!(body["data"]["name"], "东京晴空塔");
    let desc_zh = body["data"]["description"].as_str().unwrap_or("");
    assert!(desc_zh.contains("634米"), "zh description should contain Chinese text, got: {desc_zh}");
    assert_eq!(body["data"]["reviews"], serde_json::json!([]));

    let (_, body_en) = body_json(attraction_detail(
        State(AppState { db: Some(db), replica: None, cache: None }),
        Path(100000u64),
        Query(LangQuery { lang: "xx".into() }),
    ).await).await;
    let desc_en = body_en["data"]["description"].as_str().unwrap_or("");
    assert!(desc_en.contains("634m"), "fallback description should be English, got: {desc_en}");
}
