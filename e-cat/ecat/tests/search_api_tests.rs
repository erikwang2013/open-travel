// search-service 集成测试：直接调用 handler，覆盖 P3-02 检索语义。
// 离线运行：state 全 None，不依赖任何数据源；真实路径依赖本机
// MySQL（3308）/ Redis（6381），连不上自动跳过（照抄 admin/user 测试约定）。
#![cfg(test)]

#[path = "../src/business/search/mod.rs"]
mod service;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use ecat_data::Cache;
use ecat_data::RdbmsClient;
use ecat_data_redis::RedisCache;
use ecat_data_sqlx::SqlxClient;
use service::*;
use std::sync::Arc;

fn state() -> AppState {
    AppState { db: None, replica: None, cache: None, os: None }
}

async fn body_json(resp: impl IntoResponse) -> (StatusCode, serde_json::Value) {
    let (parts, body) = resp.into_response().into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    (parts.status, serde_json::from_slice(&bytes).unwrap())
}

/// 本机 MySQL（docker compose 映射 3308）；连不上返回 None，测试跳过。

async fn test_db() -> Option<Arc<SqlxClient>> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:travel_dev@localhost:3308/travel".into());
    match SqlxClient::connect(&url).await {
        Ok(db) => {
            Some(Arc::new(db))
        }
        Err(e) => {
            eprintln!("db unavailable, skipping real-db test: {e}");
            None
        }
    }
}

// ===== 离线用例（无数据源）=====

#[tokio::test]
async fn search_returns_empty_without_datasource() {
    let (status, body) = body_json(search(
        State(state()),
        Query(SearchQuery { q: "tokyo".into(), destination_id: None, lang: "en".into(), price_min: None, price_max: None, page: 1 }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"]["total"], 0);
    assert_eq!(body["data"]["page"], 1);
    assert_eq!(body["data"]["page_size"], 20);
    assert_eq!(body["data"]["items"], serde_json::json!([]));
}

#[tokio::test]
async fn search_defaults_page_to_one_and_clamps_zero() {
    let q: SearchQuery = serde_json::from_str(r#"{"q":"tokyo","lang":"zh"}"#).unwrap();
    assert_eq!(q.page, 1);
    assert_eq!(q.destination_id, None);
    let (_, body) = body_json(search(State(state()), Query(SearchQuery { page: 0, ..q })).await).await;
    assert_eq!(body["data"]["page"], 1);
}

#[tokio::test]
async fn norm_lang_normalizes_case_and_defaults() {
    assert_eq!(norm_lang("ZH"), "zh");
    assert_eq!(norm_lang("  En  "), "en");
    assert_eq!(norm_lang(""), "en");
}

#[test]
fn build_os_query_combines_filters() {
    let v = build_os_query("tokyo sky", Some(2), Some(100), Some(5000), 3);
    let must = v["query"]["bool"]["must"].as_array().unwrap();
    assert_eq!(must.len(), 3);
    assert_eq!(must[0]["multi_match"]["query"], "tokyo sky");
    assert!(must[0]["multi_match"]["fields"].as_array().unwrap().contains(&serde_json::json!("description")));
    assert_eq!(must[1]["term"]["destination_id"], 2);
    assert_eq!(must[2]["range"]["price_cents"]["gte"], 100);
    assert_eq!(must[2]["range"]["price_cents"]["lte"], 5000);
    // 单索引从 0 取前 page*PAGE_SIZE 条，合并后在应用层切页
    assert_eq!(v["from"], 0);
    assert_eq!(v["size"], 60);
}

#[test]
fn build_os_query_empty_q_uses_match_all() {
    let v = build_os_query("", None, None, None, 1);
    let must = v["query"]["bool"]["must"].as_array().unwrap();
    assert_eq!(must.len(), 1);
    assert!(must[0].get("match_all").is_some());
}

#[test]
fn hits_total_handles_object_and_number_forms() {
    assert_eq!(hits_total(&serde_json::json!({"hits": {"total": {"value": 7}}})), 7);
    assert_eq!(hits_total(&serde_json::json!({"hits": {"total": 3}})), 3);
    assert_eq!(hits_total(&serde_json::json!({})), 0);
}

// ===== 真实 MySQL 路径（3308 不可用时跳过）=====

/// q 检索正常：结果统一列表含 type 字段，total 与 DB 命中一致。
#[tokio::test]
async fn fallback_search_returns_merged_items_with_real_db() {
    let Some(db) = test_db().await else { return };
    let (status, body) = body_json(search(
        State(AppState { db: Some(db.clone()), replica: None, cache: None, os: None }),
        Query(SearchQuery { q: "tokyo".into(), destination_id: None, lang: "zh".into(), price_min: None, price_max: None, page: 1 }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    let total = body["data"]["total"].as_u64().unwrap();
    let items = body["data"]["items"].as_array().unwrap();
    assert!(total >= items.len() as u64);
    if items.is_empty() { return; }
    let t = items[0]["type"].as_str().unwrap();
    assert!(t == "destination" || t == "attraction", "unexpected type: {t}");
    assert!(items[0]["name"].as_str().unwrap_or("").len() > 0);
}

/// destination_id 过滤：命中项全为景点且属于该目的地。
#[tokio::test]
async fn fallback_search_filters_destination_with_real_db() {
    let Some(db) = test_db().await else { return };
    let (_, body) = body_json(search(
        State(AppState { db: Some(db.clone()), replica: None, cache: None, os: None }),
        Query(SearchQuery { q: String::new(), destination_id: Some(2), lang: "en".into(), price_min: None, price_max: None, page: 1 }),
    ).await).await;
    let items = body["data"]["items"].as_array().unwrap();
    if items.is_empty() { return; }
    for item in items {
        assert_eq!(item["type"], "attraction");
        let raw = db.query_with(
            "SELECT destination_id FROM travel_attractions WHERE id = ?",
            &[serde_json::json!(item["id"].as_u64().unwrap())],
        ).await.unwrap();
        assert_eq!(raw[0].get("destination_id").and_then(|v| v.as_u64()), Some(2));
    }
}

/// 分页：两索引合并后统一切页，page 1/2 不重叠且每页不超过 PAGE_SIZE。
/// 两索引各造 20 条（唯一关键词），旧实现 page 1 会拼出 40 条、page 2 为空。
#[tokio::test]
async fn fallback_search_paginates_without_overlap_with_real_db() {
    let Some(db) = test_db().await else { return };
    let tag = format!("pagetest{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    for i in 0..20u64 {
        let dest_id = ecat::business::shared::snowflake_id().await;
        db.query_with(
            "INSERT INTO travel_destinations (id, name_en, name_zh, name_ja, latitude, longitude, region_id, sort_order) \
             VALUES (?, ?, ?, ?, 0, 0, 0, ?)",
            &[serde_json::json!(dest_id), serde_json::json!(format!("{tag} d{i}")),
              serde_json::json!(format!("{tag} d{i}")), serde_json::json!(format!("{tag} d{i}")),
              serde_json::json!(i)],
        ).await.unwrap();
        db.query_with(
            "INSERT INTO travel_attractions (id, destination_id, name_en, name_zh, name_ja) VALUES (?, 1, ?, ?, ?)",
            &[serde_json::json!(ecat::business::shared::snowflake_id().await),
              serde_json::json!(format!("{tag} a{i}")), serde_json::json!(format!("{tag} a{i}")),
              serde_json::json!(format!("{tag} a{i}"))],
        ).await.unwrap();
    }
    let mk = |page| SearchQuery { q: tag.clone(), destination_id: None, lang: "en".into(), price_min: None, price_max: None, page };
    let (_, p1) = body_json(search(
        State(AppState { db: Some(db.clone()), replica: None, cache: None, os: None }),
        Query(mk(1)),
    ).await).await;
    let (_, p2) = body_json(search(
        State(AppState { db: Some(db.clone()), replica: None, cache: None, os: None }),
        Query(mk(2)),
    ).await).await;
    let _ = db.query_with("DELETE FROM travel_destinations WHERE name_en LIKE ?", &[serde_json::json!(format!("%{tag}%"))]).await;
    let _ = db.query_with("DELETE FROM travel_attractions WHERE name_en LIKE ?", &[serde_json::json!(format!("%{tag}%"))]).await;

    let a1 = p1["data"]["items"].as_array().unwrap();
    let a2 = p2["data"]["items"].as_array().unwrap();
    assert_eq!(p1["data"]["total"], 40);
    assert_eq!(a1.len(), 20, "page 1 must hold PAGE_SIZE items, not 2x");
    assert_eq!(a2.len(), 20, "page 2 must hold the next PAGE_SIZE items");
    assert!(a1.iter().all(|i| i["type"] == "destination"), "page 1 must be destinations first");
    assert!(a2.iter().all(|i| i["type"] == "attraction"), "page 2 must continue with attractions");
    let key = |i: &serde_json::Value| (i["type"].as_str().unwrap().to_string(), i["id"].as_u64().unwrap());
    let page1: Vec<_> = a1.iter().map(key).collect();
    assert!(a2.iter().map(key).all(|k| !page1.contains(&k)), "page 2 must not repeat page 1");
}

/// 价格过滤：命中项价格在区间内（目的地无价格字段被排除）。
#[tokio::test]
async fn fallback_search_price_range_with_real_db() {
    let Some(db) = test_db().await else { return };
    let (_, body) = body_json(search(
        State(AppState { db: Some(db.clone()), replica: None, cache: None, os: None }),
        Query(SearchQuery { q: String::new(), destination_id: None, lang: "en".into(), price_min: Some(1000), price_max: Some(9000), page: 1 }),
    ).await).await;
    let items = body["data"]["items"].as_array().unwrap();
    if items.is_empty() { return; }
    for item in items {
        assert_eq!(item["type"], "attraction");
        let p = item["price_cents"].as_u64().unwrap();
        assert!((1000..=9000).contains(&p), "price out of range: {p}");
    }
}

/// 检索日志落库：同关键词命中 travel_searches 记录数增长。
async fn log_count(db: &SqlxClient, keyword: &str) -> u64 {
    db.query_with(
        "SELECT COUNT(*) AS n FROM travel_searches WHERE keyword = ?",
        &[serde_json::json!(keyword)],
    ).await.unwrap()[0].get("n").and_then(|v| v.as_u64()).unwrap_or(0)
}

#[tokio::test]
async fn search_logs_to_travel_searches_with_real_db() {
    let Some(db) = test_db().await else { return };
    let before = log_count(&db, "tokyo_log_test").await;
    let (_, body) = body_json(search(
        State(AppState { db: Some(db.clone()), replica: None, cache: None, os: None }),
        Query(SearchQuery { q: "tokyo_log_test".into(), destination_id: None, lang: "en".into(), price_min: None, price_max: None, page: 1 }),
    ).await).await;
    assert_eq!(body["code"], 0);
    let after = log_count(&db, "tokyo_log_test").await;
    assert_eq!(after, before + 1, "search log row should be inserted");
}

/// 缓存命中不写检索日志（未命中才写，避免缓存命中放大 MySQL 写入）。
#[tokio::test]
async fn cache_hit_does_not_log_search_with_real_db() {
    let Ok(cache) = RedisCache::connect(&std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6381".into())).await else { return };
    let Some(db) = test_db().await else { return };
    let kw = format!("cache_log_test_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    let state = AppState { db: Some(db.clone()), replica: None, cache: Some(Arc::new(cache)), os: None };
    let query = || SearchQuery { q: kw.clone(), destination_id: None, lang: "en".into(), price_min: None, price_max: None, page: 1 };

    let (_, _) = body_json(search(State(state.clone()), Query(query())).await).await;
    let after_miss = log_count(&db, &kw).await;
    assert_eq!(after_miss, 1, "cache miss should log once");

    let (status, body) = body_json(search(State(state), Query(query())).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "cache hit");
    assert_eq!(log_count(&db, &kw).await, after_miss, "cache hit must not log");
    let _ = db.query_with("DELETE FROM travel_searches WHERE keyword = ?", &[serde_json::json!(kw)]).await;
}

/// 热词聚合（P5-03）：自造 keyword 写入 travel_searches，all 周期应包含该词。
#[tokio::test]
async fn hotwords_aggregates_recent_keywords_with_real_db() {
    let Some(db) = test_db().await else { return };
    let kw = format!("hotwords_test_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    db.query_with(
        "INSERT INTO travel_searches (id, keyword, lang, result_count) VALUES (?, ?, 'en', 0)",
        &[serde_json::json!(ecat::business::shared::snowflake_id().await), serde_json::json!(kw)],
    ).await.unwrap();
    let (status, body) = body_json(hotwords(
        State(AppState { db: Some(db.clone()), replica: None, cache: None, os: None }),
        Query(HotwordsQuery { period: "all".into(), limit: 50 }),
    ).await).await;
    let _ = db.query_with("DELETE FROM travel_searches WHERE keyword = ?", &[serde_json::json!(kw)]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["code"], 0);
    assert!(body["data"].as_array().unwrap().iter().any(|h| h["keyword"] == serde_json::json!(kw)),
        "inserted keyword should appear in hotwords");
}

#[tokio::test]
async fn hotwords_rejects_invalid_period_and_limit() {
    let cases = [
        HotwordsQuery { period: "hour".into(), limit: 10 },
        HotwordsQuery { period: "day".into(), limit: 0 },
        HotwordsQuery { period: "day".into(), limit: 51 },
    ];
    for q in cases {
        let (status, body) = body_json(hotwords(State(state()), Query(q)).await).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "should reject invalid params");
        assert_eq!(body["code"], 400);
    }
}

/// 缓存：结果写入 Redis（60s），二次请求命中缓存（无 DB 也返回）。
#[tokio::test]
async fn search_caches_result_in_redis() {
    let Ok(cache) = RedisCache::connect(&std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6381".into())).await else { return };
    let cache = Arc::new(cache);
    let key = "travel:search:en:tokyo:0:0:0:1";
    let _ = cache.delete(key).await;
    let (status, _) = body_json(search(
        State(AppState { db: None, replica: None, cache: Some(cache.clone()), os: None }),
        Query(SearchQuery { q: "tokyo".into(), destination_id: None, lang: "en".into(), price_min: None, price_max: None, page: 1 }),
    ).await).await;
    assert_eq!(status, StatusCode::OK);
    let cached = cache.get(key).await.unwrap();
    assert!(cached.is_some(), "search response should be cached in redis");

    let (_, body) = body_json(search(
        State(AppState { db: None, replica: None, cache: Some(cache), os: None }),
        Query(SearchQuery { q: "tokyo".into(), destination_id: None, lang: "en".into(), price_min: None, price_max: None, page: 1 }),
    ).await).await;
    assert_eq!(body["message"], "cache hit");
}
