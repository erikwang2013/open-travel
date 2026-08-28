// P1-04 集成测试：直接调用 handler，覆盖 api.md 的 dates 接口语义
// （缓存未命中/DB 缺失时的占位兜底、region_id 缺省、ready 降级报告）。
// 离线运行：state 中 db/cache 均为 None，不依赖 MySQL/Redis。
//
// main.rs 是 binary crate，tests/ 无法直接访问其私有项，
// 故经 #[path] 以模块方式包含源码（配合 main.rs 中最小 pub(crate) 改动）。
#![cfg(test)]

#[path = "../src/main.rs"]
mod service;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use service::*;

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
