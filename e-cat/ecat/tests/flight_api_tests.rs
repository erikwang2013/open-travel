// P4-02 集成测试：直接调用 handler，覆盖 search 参数校验/价格排序/日期与舱位过滤/
// 分页/空结果，详情与 404，Redis 缓存。离线运行（state 全 None）不依赖 MySQL/Redis；
// 真实数据用例连不上时跳过（同 line 测试模式）。
//
// main.rs 是 binary crate，tests/ 无法直接访问其私有项，
// 故经 #[path] 以模块方式包含源码（配合 main.rs 中最小 pub(crate) 改动）。
#![cfg(test)]

#[path = "../src/business/flight/mod.rs"]
mod service;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use ecat_data::{Cache, RdbmsClient};
use ecat_data_redis::RedisCache;
use service::*;
use ecat::business::shared::connect_primary;
use std::sync::Arc;

fn state() -> AppState {
    AppState { db: None, replica: None, cache: None }
}

fn q(from: &str, to: &str, date: Option<&str>, cabin: Option<&str>) -> SearchQuery {
    SearchQuery {
        from: Some(from.into()),
        to: Some(to.into()),
        date: date.map(String::from),
        cabin: cabin.map(String::from),
        page: None,
        page_size: None,
    }
}

async fn body_json(resp: impl IntoResponse) -> (StatusCode, serde_json::Value) {
    let (parts, body) = resp.into_response().into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    (parts.status, serde_json::from_slice(&bytes).unwrap())
}

// ---------- 离线用例（无 DB/Redis）----------

#[tokio::test]
async fn search_requires_from_and_to() {
    let (s1, b1) = body_json(flights_search(State(state()), Query(q("", "HKG", None, None))).await).await;
    assert_eq!(s1, StatusCode::BAD_REQUEST);
    assert_eq!(b1["code"], 400);

    let (s2, _) = body_json(flights_search(State(state()), Query(q("HND", "", None, None))).await).await;
    assert_eq!(s2, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn search_rejects_bad_iata_and_bad_cabin() {
    // 非 3 字母码
    let (s1, _) = body_json(flights_search(State(state()), Query(q("TOKYO", "HKG", None, None))).await).await;
    assert_eq!(s1, StatusCode::BAD_REQUEST);
    let (s2, _) = body_json(flights_search(State(state()), Query(q("H1D", "HKG", None, None))).await).await;
    assert_eq!(s2, StatusCode::BAD_REQUEST);
    // cabin 越界/非数字
    let (s3, _) = body_json(flights_search(State(state()), Query(q("HND", "HKG", None, Some("3")))).await).await;
    assert_eq!(s3, StatusCode::BAD_REQUEST);
    let (s4, _) = body_json(flights_search(State(state()), Query(q("HND", "HKG", None, Some("abc")))).await).await;
    assert_eq!(s4, StatusCode::BAD_REQUEST);
    // date 格式错误
    let (s5, _) = body_json(flights_search(State(state()), Query(q("HND", "HKG", Some("2026/09/10"), None))).await).await;
    assert_eq!(s5, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn search_returns_empty_without_datasource() {
    let (status, body) = body_json(flights_search(State(state()), Query(q("HND", "HKG", None, None))).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"]["total"], 0);
    assert_eq!(body["data"]["page"], 1);
    assert_eq!(body["data"]["page_size"], 20);
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn detail_returns_404_without_datasource() {
    let (status, body) = body_json(flight_detail(State(state()), Path(40001001u64)).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "flight not found");
}

// ---------- 真实 DB 用例（无种子数据/连接失败时跳过）----------

/// search 价格升序 + 字段完整：HND→HKG 种子经济 158000 / 商务 458000。
#[tokio::test]
async fn search_sorts_by_price_asc_with_real_db() {
    let Some(db) = connect_primary().await else { return; };
    // 自复位断言行余位（历史遗留订单会消耗，sold_out 断言依赖它）
    db.execute_with("UPDATE travel_flights SET seats_left = 12 WHERE id = 40001001", &[]).await.unwrap();
    let (status, body) = body_json(flights_search(
        State(AppState { db: Some(db), replica: None, cache: None }),
        Query(q("HND", "HKG", None, None)),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    let data = &body["data"];
    let items = data["items"].as_array().unwrap();
    if items.is_empty() { return; } // 无种子数据环境跳过
    assert_eq!(data["total"], 2);
    assert_eq!(items[0]["cabin"], 0);
    assert_eq!(items[0]["price_cents"], 158000);
    assert_eq!(items[0]["airline"], "Cathay Pacific");
    assert_eq!(items[0]["flight_no"], "CX501");
    assert_eq!(items[0]["from_code"], "HND");
    assert_eq!(items[0]["to_code"], "HKG");
    assert_eq!(items[0]["depart_at"], "2026-09-10 09:00:00");
    assert_eq!(items[0]["sold_out"], false);
    assert_eq!(items[1]["cabin"], 1);
    assert_eq!(items[1]["price_cents"], 458000);
    let prices: Vec<u64> = items.iter().map(|r| r["price_cents"].as_u64().unwrap()).collect();
    let mut sorted = prices.clone();
    sorted.sort();
    assert_eq!(prices, sorted, "prices must be sorted ascending");
}

/// 日期过滤：10 号 2 班，11 号无班。
#[tokio::test]
async fn search_filters_by_date_with_real_db() {
    let Some(db) = connect_primary().await else { return; };
    let (_, hit) = body_json(flights_search(
        State(AppState { db: Some(db.clone()), replica: None, cache: None }),
        Query(q("HND", "HKG", Some("2026-09-10"), None)),
    ).await).await;
    let (_, miss) = body_json(flights_search(
        State(AppState { db: Some(db), replica: None, cache: None }),
        Query(q("HND", "HKG", Some("2026-09-11"), None)),
    ).await).await;
    if hit["data"]["items"].as_array().unwrap().is_empty() { return; }
    assert_eq!(hit["data"]["total"], 2);
    assert!(hit["data"]["items"].as_array().unwrap().iter().all(|r| r["depart_at"].as_str().unwrap().starts_with("2026-09-10")));
    assert_eq!(miss["data"]["total"], 0);
    assert_eq!(miss["data"]["items"].as_array().unwrap().len(), 0);
}

/// 舱位过滤：经济 1 班，头等 0 班。
#[tokio::test]
async fn search_filters_by_cabin_with_real_db() {
    let Some(db) = connect_primary().await else { return; };
    let (_, eco) = body_json(flights_search(
        State(AppState { db: Some(db.clone()), replica: None, cache: None }),
        Query(q("HND", "HKG", None, Some("0"))),
    ).await).await;
    let (_, first) = body_json(flights_search(
        State(AppState { db: Some(db), replica: None, cache: None }),
        Query(q("HND", "HKG", None, Some("2"))),
    ).await).await;
    if eco["data"]["items"].as_array().unwrap().is_empty() { return; }
    assert_eq!(eco["data"]["total"], 1);
    assert_eq!(eco["data"]["items"][0]["cabin"], 0);
    assert_eq!(first["data"]["total"], 0);
}

/// 无结果航线（HKG→NRT 无种子）返回空 items，不报错。
#[tokio::test]
async fn search_no_result_returns_empty_items_with_real_db() {
    let Some(db) = connect_primary().await else { return; };
    let (status, body) = body_json(flights_search(
        State(AppState { db: Some(db), replica: None, cache: None }),
        Query(q("HKG", "NRT", None, None)),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 0);
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 0);
}

/// 分页：page_size=1 时两页各 1 条。
#[tokio::test]
async fn search_paginates_with_real_db() {
    let Some(db) = connect_primary().await else { return; };
    let mut q1 = q("HND", "HKG", None, None);
    q1.page = Some(1);
    q1.page_size = Some(1);
    let (_, p1) = body_json(flights_search(
        State(AppState { db: Some(db.clone()), replica: None, cache: None }),
        Query(q1),
    ).await).await;
    let mut q2 = q("HND", "HKG", None, None);
    q2.page = Some(2);
    q2.page_size = Some(1);
    let (_, p2) = body_json(flights_search(
        State(AppState { db: Some(db), replica: None, cache: None }),
        Query(q2),
    ).await).await;
    if p1["data"]["items"].as_array().unwrap().is_empty() { return; }
    assert_eq!(p1["data"]["total"], 2);
    assert_eq!(p1["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(p2["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(p1["data"]["items"][0]["price_cents"], 158000);
    assert_eq!(p2["data"]["items"][0]["price_cents"], 458000);
}

/// 详情：存在返回完整字段，不存在 404。
#[tokio::test]
async fn detail_returns_flight_and_404_with_real_db() {
    let Some(db) = connect_primary().await else { return; };
    // 自复位断言行余位（历史遗留 order_type=2 订单会消耗）
    db.execute_with("UPDATE travel_flights SET seats_left = 12 WHERE id = 40001001", &[]).await.unwrap();
    let (status, body) = body_json(flight_detail(
        State(AppState { db: Some(db.clone()), replica: None, cache: None }),
        Path(40001001u64),
    ).await).await;
    if body["data"].is_null() { return; } // 无种子数据环境跳过
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], 40001001);
    assert_eq!(body["data"]["airline"], "Cathay Pacific");
    assert_eq!(body["data"]["flight_no"], "CX501");
    assert_eq!(body["data"]["seats_left"], 12);
    assert_eq!(body["data"]["sold_out"], false);

    let (s404, b404) = body_json(flight_detail(
        State(AppState { db: Some(db), replica: None, cache: None }),
        Path(999_999_999u64),
    ).await).await;
    assert_eq!(s404, StatusCode::NOT_FOUND);
    assert_eq!(b404["code"], 404);
}

/// search 写 Redis 缓存（TTL 60s），二次请求命中缓存（无 DB 也返回数据）。
#[tokio::test]
async fn search_caches_in_redis_on_miss() {
    let Ok(cache) = RedisCache::connect(&std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6381".into())).await else { return; };
    let Some(db) = connect_primary().await else { return; };
    let cache = Arc::new(cache);
    let key = "travel:flights:HND:HKG:all:all";
    let _ = cache.delete(key).await;
    let (status, _) = body_json(flights_search(
        State(AppState { db: Some(db), replica: None, cache: Some(cache.clone()) }),
        Query(q("HND", "HKG", None, None)),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    let cached = cache.get(key).await.unwrap();
    assert!(cached.is_some(), "search response should be cached in redis");

    // 缓存命中路径：无 DB 也返回数据
    let (status2, body2) = body_json(flights_search(
        State(AppState { db: None, replica: None, cache: Some(cache) }),
        Query(q("HND", "HKG", None, None)),
    ).await).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(body2["message"], "cache hit");
    assert_eq!(body2["data"]["items"][0]["price_cents"], 158000);
}
