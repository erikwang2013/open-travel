// P4-04 集成测试：直接调用 handler，覆盖搜索全部/城市过滤/排序/房型随带/
// 无结果/详情 404。离线运行（state 全 None）不依赖 MySQL/Redis；
// 真实数据用例直连本机 3308 容器库（mysql://root:travel_dev@localhost:3308/travel），
// 连接失败时跳过（同 line/booking 测试模式）。
//
// main.rs 是 binary crate，tests/ 无法直接访问其私有项，
// 故经 #[path] 以模块方式包含源码（配合 main.rs 中最小 pub(crate) 改动）。
#![cfg(test)]

#[path = "../src/business/hotel/mod.rs"]
mod service;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use ecat_data_sqlx::SqlxClient;
use service::*;
use std::sync::Arc;

fn state() -> AppState {
    AppState { db: None, replica: None, cache: None }
}

/// 连本机 3308 容器库；失败返回 None（测试跳过）。
async fn test_db() -> Option<Arc<SqlxClient>> {
    SqlxClient::connect("mysql://root:travel_dev@localhost:3308/travel?charset=utf8mb4")
        .await
        .ok()
        .map(Arc::new)
}

async fn body_json(resp: impl IntoResponse) -> (StatusCode, serde_json::Value) {
    let (parts, body) = resp.into_response().into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    (parts.status, serde_json::from_slice(&bytes).unwrap())
}

fn search_query(city: &str, page: Option<u64>, page_size: Option<u64>) -> SearchQuery {
    SearchQuery {
        city: if city.is_empty() { None } else { Some(city.into()) },
        check_in: None,
        check_out: None,
        page,
        page_size,
    }
}

#[tokio::test]
async fn hotels_search_returns_empty_without_datasource() {
    // 无 DB/Redis 时返回空 items，不报错
    let (status, body) = body_json(hotels_search(State(state()), Query(search_query("", None, None))).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"]["total"], 0);
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn hotels_search_defaults_pagination() {
    // page/page_size 缺省为 1/20
    let q: SearchQuery = serde_json::from_str("{}").unwrap();
    assert_eq!(q.city, None);
    let (status, body) = body_json(hotels_search(State(state()), Query(q)).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["page"], 1);
    assert_eq!(body["data"]["page_size"], 20);
}

#[tokio::test]
async fn hotels_search_city_too_long_returns_400() {
    // 城市代码 >3 字符 → 400（无效输入）
    let (status, body) = body_json(hotels_search(State(state()), Query(search_query("ABCD", None, None))).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], 400);
}

#[tokio::test]
async fn hotel_detail_returns_404_without_datasource() {
    let (status, body) = body_json(hotel_detail(State(state()), Path(9_999_999_999u64)).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "hotel not found");
}

// 以下用例依赖真实 MySQL（3308 容器库，DDL+种子已就位），连接失败时跳过。

/// 搜索全部：返回全部上架酒店（种子 TYO×2/HKG/PAR/LON），每家带可用房型。
#[tokio::test]
async fn hotels_search_returns_all_hotels_with_rooms_with_real_db() {
    let Some(db) = test_db().await else { return; };
    let (status, body) = body_json(hotels_search(
        State(AppState { db: Some(db), replica: None, cache: None }),
        Query(search_query("", None, None)),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    let items = body["data"]["items"].as_array().unwrap();
    if items.is_empty() { return; } // 无种子数据环境跳过
    assert_eq!(items.len() as u64, body["data"]["total"].as_u64().unwrap());
    // 每家有房型数组，房型含完整字段且均上架
    for hotel in items {
        assert!(hotel["id"].as_u64().unwrap() > 0);
        assert_eq!(hotel["city_code"].as_str().unwrap().len(), 3);
        assert!(hotel["latitude"].as_f64().unwrap() > 0.0);
        assert!(hotel["longitude"].as_f64().unwrap() != 0.0);
        let rooms = hotel["rooms"].as_array().unwrap();
        assert!(!rooms.is_empty(), "hotel {} must carry rooms", hotel["id"]);
        for room in rooms {
            assert_eq!(room["status"], 1);
            assert!(room["price_cents"].as_u64().unwrap() > 0);
            assert_eq!(room["id"].as_u64().unwrap() > 0, true);
        }
    }
}

/// 城市过滤 + 排序：city=TYO 只返回 TYO 酒店，star 降序（id 升序）。
#[tokio::test]
async fn hotels_search_filters_by_city_and_sorts_by_star_with_real_db() {
    let Some(db) = test_db().await else { return; };
    let (status, body) = body_json(hotels_search(
        State(AppState { db: Some(db), replica: None, cache: None }),
        Query(search_query("TYO", None, None)),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    let items = body["data"]["items"].as_array().unwrap();
    if items.is_empty() { return; }
    assert!(items.iter().all(|h| h["city_code"] == "TYO"), "all rows must match city filter");
    let stars: Vec<u64> = items.iter().map(|h| h["star"].as_u64().unwrap()).collect();
    let mut sorted = stars.clone();
    sorted.sort_by(|a, b| b.cmp(a));
    assert_eq!(stars, sorted, "star must be sorted descending");
    // 语言列齐全（三语名称非空）
    assert!(!items[0]["name_en"].as_str().unwrap().is_empty());
    assert!(!items[0]["name_zh"].as_str().unwrap().is_empty());
    assert!(!items[0]["name_ja"].as_str().unwrap().is_empty());
}

/// 无结果：未知城市返回 200 + 空 items + total 0。
#[tokio::test]
async fn hotels_search_no_result_returns_empty_items_with_real_db() {
    let Some(db) = test_db().await else { return; };
    let (status, body) = body_json(hotels_search(
        State(AppState { db: Some(db), replica: None, cache: None }),
        Query(search_query("ZZZ", None, None)),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 0);
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 0);
}

/// 详情：返回与列表一致的结构（含全部可用房型）；不存在/下架 → 404。
#[tokio::test]
async fn hotel_detail_returns_rooms_and_404_with_real_db() {
    let Some(db) = test_db().await else { return; };
    // 先取一个真实酒店 id
    let (_, list) = body_json(hotels_search(
        State(AppState { db: Some(db.clone()), replica: None, cache: None }),
        Query(search_query("", None, None)),
    ).await).await;
    let Some(first) = list["data"]["items"].as_array().and_then(|a| a.first()) else { return; };
    let id = first["id"].as_u64().unwrap();

    let (status, body) = body_json(hotel_detail(
        State(AppState { db: Some(db.clone()), replica: None, cache: None }),
        Path(id),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], id);
    assert_eq!(body["data"]["city_code"], first["city_code"]);
    let rooms = body["data"]["rooms"].as_array().unwrap();
    assert!(!rooms.is_empty(), "detail must carry rooms");
    assert!(rooms.iter().all(|r| r["status"] == 1));
    // 详情房型集合与列表随带一致
    let list_rooms = first["rooms"].as_array().unwrap();
    assert_eq!(rooms.len(), list_rooms.len());
    assert_eq!(rooms[0]["id"], list_rooms[0]["id"]);

    // 不存在酒店 404
    let (status404, body404) = body_json(hotel_detail(
        State(AppState { db: Some(db), replica: None, cache: None }),
        Path(9_999_999_999u64),
    ).await).await;
    assert_eq!(status404, StatusCode::NOT_FOUND);
    assert_eq!(body404["code"], 404);
}
