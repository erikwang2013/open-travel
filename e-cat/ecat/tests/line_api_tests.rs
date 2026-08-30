// P3-05/P3-06 集成测试：直接调用 handler，覆盖列表过滤/空结果、详情/404、
// 日历余位与排序、语言回退。离线运行（state 全 None）不依赖 MySQL/Redis；
// 真实数据用例连不上时跳过（同 booking 测试模式）。
//
// main.rs 是 binary crate，tests/ 无法直接访问其私有项，
// 故经 #[path] 以模块方式包含源码（配合 main.rs 中最小 pub(crate) 改动）。
#![cfg(test)]

#[path = "../src/business/line/mod.rs"]
mod service;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use ecat_data::Cache;
use ecat_data_redis::RedisCache;
use service::*;
use ecat::business::shared::connect_primary;
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
async fn lines_list_returns_empty_without_datasource() {
    // 无 DB/Redis 时返回空数组，不报错
    let (status, body) = body_json(lines_list(
        State(state()),
        Query(LinesQuery { destination_id: None, lang: "en".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn lines_list_destination_id_is_optional() {
    // destination_id 缺省即全部（不要求必填，区别于 booking 的 attractions）
    let q: LinesQuery = serde_json::from_str("{}").unwrap();
    assert_eq!(q.destination_id, None);
    assert_eq!(q.lang, "");
    let (status, _) = body_json(lines_list(State(state()), Query(q)).await).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn line_detail_returns_404_when_not_found() {
    let (status, body) = body_json(line_detail(
        State(state()),
        Path(999_999_999u64),
        Query(LangQuery { lang: "en".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "line not found");
}

#[tokio::test]
async fn line_dates_returns_empty_without_datasource() {
    let (status, body) = body_json(line_dates(
        State(state()),
        Path(10020001u64),
        Query(LangQuery { lang: "en".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
}

// 以下用例依赖真实 MySQL（种子数据），连接失败/无数据时跳过（离线环境测试仍可跑）。

/// 列表按 destination_id 过滤 + zh 标题与种子一致（title_zh 列）。
#[tokio::test]
async fn lines_list_filters_by_destination_and_zh_title_with_real_db() {
    let Some(db) = connect_primary().await else { return; };
    let (status, body) = body_json(lines_list(
        State(AppState { db: Some(db.clone()), replica: None, cache: None }),
        Query(LinesQuery { destination_id: Some(2), lang: "zh".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["data"].as_array().unwrap();
    if rows.is_empty() { return; } // 无种子数据环境跳过
    // destination_id=2 的种子：10020001 东京经典 3 日游 / 10020002 东京箱根 2 日快线
    assert!(rows.iter().all(|r| r["destination_id"] == 2), "all rows must match filter");
    assert_eq!(rows[0]["title"], "东京经典 3 日游");
    assert_eq!(rows[0]["days"], 3);
    assert_eq!(rows[0]["price_cents"], 128000);
    assert_eq!(rows[0]["max_pax"], 20);
}

/// 未知语种回退链 lang→zh→en：无 title_{lang} 列时取 title_zh。
#[tokio::test]
async fn lines_list_unknown_lang_falls_back_to_zh_with_real_db() {
    let Some(db) = connect_primary().await else { return; };
    let (_, zh) = body_json(lines_list(
        State(AppState { db: Some(db.clone()), replica: None, cache: None }),
        Query(LinesQuery { destination_id: Some(2), lang: "zh".into() }),
    ).await).await;
    let (_, xx) = body_json(lines_list(
        State(AppState { db: Some(db), replica: None, cache: None }),
        Query(LinesQuery { destination_id: Some(2), lang: "xx".into() }),
    ).await).await;
    let zh_rows = zh["data"].as_array().unwrap();
    let xx_rows = xx["data"].as_array().unwrap();
    if zh_rows.is_empty() || xx_rows.is_empty() { return; }
    assert_eq!(xx_rows[0]["title"], zh_rows[0]["title"]);
}

/// 详情返回完整字段：itinerary 数组按 lang 取 day 标题（zh），404 用真实 DB 再验证一次。
#[tokio::test]
async fn line_detail_returns_localized_itinerary_with_real_db() {
    let Some(db) = connect_primary().await else { return; };
    let (status, body) = body_json(line_detail(
        State(AppState { db: Some(db.clone()), replica: None, cache: None }),
        Path(10020001u64),
        Query(LangQuery { lang: "zh".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    if body["data"].is_null() { return; } // 无种子数据环境跳过
    assert_eq!(body["data"]["title"], "东京经典 3 日游");
    assert_eq!(body["data"]["days"], 3);
    assert_eq!(body["data"]["itinerary"].as_array().unwrap().len(), 3);
    assert_eq!(body["data"]["itinerary"][0]["day"], 1);
    assert_eq!(body["data"]["itinerary"][0]["title"], "涩谷与新宿");
    assert_eq!(body["data"]["itinerary"][2]["title"], "富士山一日游");

    // 未知语种 day title 回退 zh
    let (_, body_xx) = body_json(line_detail(
        State(AppState { db: Some(db.clone()), replica: None, cache: None }),
        Path(10020001u64),
        Query(LangQuery { lang: "xx".into() }),
    ).await).await;
    assert_eq!(body_xx["data"]["itinerary"][0]["title"], "涩谷与新宿");

    // 不存在线路 404
    let (status404, body404) = body_json(line_detail(
        State(AppState { db: Some(db), replica: None, cache: None }),
        Path(999_999_999u64),
        Query(LangQuery { lang: "en".into() }),
    ).await).await;
    assert_eq!(status404, StatusCode::NOT_FOUND);
    assert_eq!(body404["code"], 404);
}

/// 日历：返回未来班期，按日期升序，余位实时值正确，sold_out 标注。
#[tokio::test]
async fn line_dates_returns_sorted_future_dates_with_seats_with_real_db() {
    let Some(db) = connect_primary().await else { return; };
    let (status, body) = body_json(line_dates(
        State(AppState { db: Some(db), replica: None, cache: None }),
        Path(10020001u64),
        Query(LangQuery { lang: "en".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["data"].as_array().unwrap();
    if rows.is_empty() { return; } // 种子日期过期或缺失时跳过
    // 种子：10020001 → 09-10(12) / 09-17(8) / 09-24(15) / 10-01(20)
    assert_eq!(rows[0]["date"], "2026-09-10");
    assert_eq!(rows[0]["seats_left"], 12);
    assert_eq!(rows[0]["price_cents"], 128000);
    assert_eq!(rows[0]["sold_out"], false);
    let dates: Vec<&str> = rows.iter().map(|r| r["date"].as_str().unwrap()).collect();
    let mut sorted = dates.clone();
    sorted.sort();
    assert_eq!(dates, sorted, "dates must be sorted ascending");
}

/// 列表写 Redis 缓存（TTL 5min），二次请求命中缓存（无 DB 也返回数据）。
#[tokio::test]
async fn lines_list_caches_in_redis_on_miss() {
    let Ok(cache) = RedisCache::connect(&std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6381".into())).await else { return; };
    let Some(db) = connect_primary().await else { return; };
    let cache = Arc::new(cache);
    let key = "travel:lines:2:en";
    let _ = cache.delete(key).await;
    let (status, _) = body_json(lines_list(
        State(AppState { db: Some(db), replica: None, cache: Some(cache.clone()) }),
        Query(LinesQuery { destination_id: Some(2), lang: "en".into() }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    let cached = cache.get(key).await.unwrap();
    assert!(cached.is_some(), "list response should be cached in redis");

    // 缓存命中路径：无 DB 也返回数据
    let (status2, body2) = body_json(lines_list(
        State(AppState { db: None, replica: None, cache: Some(cache) }),
        Query(LinesQuery { destination_id: Some(2), lang: "en".into() }),
    ).await).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(body2["message"], "cache hit");
}
