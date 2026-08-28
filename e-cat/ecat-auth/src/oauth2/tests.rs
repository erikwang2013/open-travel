use super::*;
use axum::routing::post;
use std::sync::atomic::{AtomicUsize, Ordering};

async fn spawn_introspection_server(count: &'static AtomicUsize) -> String {
    spawn_introspection_server_with(
        count,
        serde_json::json!({
            "active": true,
            "sub": "user-1",
            "role": "admin",
            "exp": 9999999999u64,
        }),
    )
    .await
}

async fn spawn_introspection_server_with(
    count: &'static AtomicUsize,
    claims: serde_json::Value,
) -> String {
    use axum::Json;
    use axum::response::IntoResponse;
    let app = axum::Router::new().route(
        "/introspect",
        post(move || async move {
            count.fetch_add(1, Ordering::SeqCst);
            Json(claims.clone()).into_response()
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}/introspect")
}

#[tokio::test]
async fn introspection_cached_within_ttl() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let url = spawn_introspection_server(&COUNT).await;
    let cfg = OAuth2Layer::new(url, "cid", "csecret")
        .unwrap()
        .cache_ttl(60);

    let claims1 = introspect_token(&cfg, "tok-1").await.unwrap();
    let claims2 = introspect_token(&cfg, "tok-1").await.unwrap();
    assert_eq!(claims1.sub, "user-1");
    assert_eq!(claims2.sub, "user-1");
    assert_eq!(COUNT.load(Ordering::SeqCst), 1, "second call hits cache");

    let claims3 = introspect_token(&cfg, "tok-2").await.unwrap();
    assert_eq!(claims3.role.as_deref(), Some("admin"));
    assert_eq!(COUNT.load(Ordering::SeqCst), 2, "new token re-introspects");
}

/// S2 回归：缓存必须被容量上限约束。容量满后按 FIFO 逐出最旧条目，
/// 海量唯一 token 不会让缓存无限增长。
#[tokio::test]
async fn cache_evicts_oldest_at_capacity() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let url = spawn_introspection_server(&COUNT).await;
    let cfg = OAuth2Layer::new(url, "cid", "csecret")
        .unwrap()
        .cache_ttl(3600)
        .cache_capacity(3);

    for token in ["tok-1", "tok-2", "tok-3", "tok-4"] {
        introspect_token(&cfg, token).await.unwrap();
    }
    assert_eq!(COUNT.load(Ordering::SeqCst), 4);

    // 容量 3：tok-1 最先被逐出，缓存大小不超过上限。
    {
        let cache = cfg.cache.read().await;
        assert_eq!(cache.entries.len(), 3);
        assert!(!cache.entries.contains_key(&cache_key("tok-1")));
        assert!(cache.entries.contains_key(&cache_key("tok-2")));
    }

    // 被逐出的 tok-1 需重新 introspection；随后 tok-2 按 FIFO 被挤出。
    introspect_token(&cfg, "tok-1").await.unwrap();
    assert_eq!(COUNT.load(Ordering::SeqCst), 5);
    assert!(
        !cfg.cache
            .read()
            .await
            .entries
            .contains_key(&cache_key("tok-2"))
    );
}

/// S2 增强：缓存 key 为 token 的 SHA-256 hash，而非明文 token——
/// 内存中不保存凭据明文（转储/取证不泄露 token）；缓存命中
/// 行为不变（由 introspection_cached_within_ttl 覆盖）。
#[tokio::test]
async fn cache_keys_are_token_hashes_not_plaintext() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let url = spawn_introspection_server(&COUNT).await;
    let cfg = OAuth2Layer::new(url, "cid", "csecret")
        .unwrap()
        .cache_ttl(60);

    introspect_token(&cfg, "tok-1").await.unwrap();
    let cache = cfg.cache.read().await;
    assert!(
        !cache.entries.contains_key("tok-1"),
        "raw token must not be used as cache key"
    );
    assert!(
        cache.entries.contains_key(&cache_key("tok-1")),
        "hashed token must be the cache key"
    );
}

/// P1 优化：缓存直接保存 AuthClaims 结构体，命中路径不再每请求
/// serde_json 反序列化。
#[tokio::test]
async fn cache_stores_claims_without_json_roundtrip() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let url = spawn_introspection_server(&COUNT).await;
    let cfg = OAuth2Layer::new(url, "cid", "csecret")
        .unwrap()
        .cache_ttl(60);

    introspect_token(&cfg, "tok-1").await.unwrap();
    let cache = cfg.cache.read().await;
    let (claims, _) = cache
        .entries
        .get(&cache_key("tok-1"))
        .expect("token cached");
    assert_eq!(claims.sub, "user-1");
    assert_eq!(claims.role.as_deref(), Some("admin"));
}

#[tokio::test]
async fn introspection_ttl_zero_disables_cache() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let url = spawn_introspection_server(&COUNT).await;
    let cfg = OAuth2Layer::new(url, "cid", "csecret")
        .unwrap()
        .cache_ttl(0);

    let _ = introspect_token(&cfg, "tok-1").await.unwrap();
    let _ = introspect_token(&cfg, "tok-1").await.unwrap();
    assert_eq!(COUNT.load(Ordering::SeqCst), 2, "ttl=0 must never cache");
}

/// 任务 #36：缓存只保留白名单 extra claims（默认 iss/aud/scope/roles），
/// email/phone 等敏感字段不落缓存；miss 路径返回完整 claims，
/// 缓存命中仅返回白名单字段。
#[tokio::test]
async fn cache_stores_only_whitelisted_extra_claims() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let url = spawn_introspection_server_with(
        &COUNT,
        serde_json::json!({"active": true, "sub": "user-1", "role": "admin",
                "iss": "https://issuer.example", "aud": "api", "scope": "read write",
                "email": "user@example.com", "phone": "+86", "custom_pii": "sensitive"}),
    )
    .await;
    let cfg = OAuth2Layer::new(url, "cid", "csecret")
        .unwrap()
        .cache_ttl(60);

    let full = introspect_token(&cfg, "tok-1").await.unwrap();
    assert!(
        full.extra.contains_key("email"),
        "miss path keeps full claims"
    );
    let cache = cfg.cache.read().await;
    let (cached, _) = cache
        .entries
        .get(&cache_key("tok-1"))
        .expect("token cached");
    for kept in ["iss", "aud", "scope"] {
        assert!(
            cached.extra.contains_key(kept),
            "whitelisted {kept} must be cached"
        );
    }
    for dropped in ["email", "phone", "custom_pii"] {
        assert!(
            !cached.extra.contains_key(dropped),
            "{dropped} must not be cached"
        );
    }
    drop(cache);
    let hit = introspect_token(&cfg, "tok-1").await.unwrap();
    assert!(
        !hit.extra.contains_key("email"),
        "hit returns whitelisted claims only"
    );
}

/// 任务 #36：白名单可配置；"*" 表示缓存全部 extra 字段（逃生门）。
#[tokio::test]
async fn cache_claims_whitelist_is_configurable() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let url = spawn_introspection_server_with(
        &COUNT,
        serde_json::json!({"active": true, "sub": "user-1", "email": "e@x.com",
                "groups": ["a", "b"], "phone": "+86"}),
    )
    .await;
    let cfg = OAuth2Layer::new(url, "cid", "csecret")
        .unwrap()
        .cache_ttl(60)
        .cache_claims_whitelist(["email", "groups"]);

    introspect_token(&cfg, "tok-1").await.unwrap();
    let cache = cfg.cache.read().await;
    let (cached, _) = cache
        .entries
        .get(&cache_key("tok-1"))
        .expect("token cached");
    assert!(cached.extra.contains_key("email"));
    assert!(cached.extra.contains_key("groups"));
    assert!(!cached.extra.contains_key("phone"));
    drop(cache);

    let url2 = spawn_introspection_server_with(
        &COUNT,
        serde_json::json!({"active": true, "sub": "user-1", "anything": {"nested": 1}}),
    )
    .await;
    let cfg2 = OAuth2Layer::new(url2, "cid", "csecret")
        .unwrap()
        .cache_ttl(60)
        .cache_claims_whitelist(["*"]);
    introspect_token(&cfg2, "tok-1").await.unwrap();
    let cache2 = cfg2.cache.read().await;
    let (cached2, _) = cache2
        .entries
        .get(&cache_key("tok-1"))
        .expect("token cached");
    assert!(
        cached2.extra.contains_key("anything"),
        "\"*\" keeps all extra"
    );
}

/// 任务 #36：缓存接近容量上限（≥90%）时，TTL 过期条目在后续写入时被
/// 真正清除（时间淘汰），而非残留内存直到容量逐出。
#[tokio::test]
async fn cache_purges_expired_entries_on_write() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let url = spawn_introspection_server(&COUNT).await;
    // 容量 3 → 淘汰阈值 90% = 2：写入第三条时 len≥2 触发全量清除。
    let cfg = OAuth2Layer::new(url, "cid", "csecret")
        .unwrap()
        .cache_ttl(1)
        .cache_capacity(3);

    introspect_token(&cfg, "tok-1").await.unwrap();
    introspect_token(&cfg, "tok-2").await.unwrap();
    assert_eq!(cfg.cache.read().await.entries.len(), 2);

    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    introspect_token(&cfg, "tok-3").await.unwrap();
    let cache = cfg.cache.read().await;
    assert_eq!(cache.entries.len(), 1, "expired entries must be purged");
    assert!(cache.entries.contains_key(&cache_key("tok-3")));
    assert!(!cache.entries.contains_key(&cache_key("tok-1")));
    assert!(!cache.entries.contains_key(&cache_key("tok-2")));
    assert_eq!(COUNT.load(Ordering::SeqCst), 3);
}

/// P2 优化：缓存远未接近容量上限时跳过全量时间淘汰（O(n) 扫描），
/// 过期条目保留；读取路径惰性跳过并重新 introspection，内存仍受容量约束。
#[tokio::test]
async fn cache_skips_purge_below_threshold() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let url = spawn_introspection_server(&COUNT).await;
    // 容量 100 → 淘汰阈值 90% = 90：仅 3 条时全量清除不触发。
    let cfg = OAuth2Layer::new(url, "cid", "csecret")
        .unwrap()
        .cache_ttl(1)
        .cache_capacity(100);

    introspect_token(&cfg, "tok-1").await.unwrap();
    introspect_token(&cfg, "tok-2").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    introspect_token(&cfg, "tok-3").await.unwrap();
    let cache = cfg.cache.read().await;
    assert_eq!(cache.entries.len(), 3, "sweep skipped below 90% threshold");
    assert!(cache.entries.contains_key(&cache_key("tok-1")));
    assert!(cache.entries.contains_key(&cache_key("tok-2")));
    drop(cache);

    // 过期条目读取时惰性重新 introspection，不依赖写入路径的全量清除。
    let claims = introspect_token(&cfg, "tok-1").await.unwrap();
    assert_eq!(claims.sub, "user-1");
    assert_eq!(COUNT.load(Ordering::SeqCst), 4);
}

/// v3.0.1 修复：内省响应体有界读取，超过 1 MiB 直接报错，
/// 防止恶意提供方无界响应耗尽内存。
#[tokio::test]
async fn introspection_rejects_oversized_body() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let pad = "x".repeat(1024 * 1024 + 4096);
    let url = spawn_introspection_server_with(
        &COUNT,
        serde_json::json!({"active": true, "sub": "user-1", "pad": pad}),
    )
    .await;
    let cfg = OAuth2Layer::new(url, "cid", "csecret").unwrap();
    let err = introspect_token(&cfg, "tok-1").await.unwrap_err();
    assert!(err.contains("exceeds"), "unexpected error: {err}");
}

/// v3.0.1 修复：内省响应必须携带非空 sub，缺失或空串一律拒绝。
#[tokio::test]
async fn introspection_rejects_missing_or_empty_sub() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let missing =
        spawn_introspection_server_with(&COUNT, serde_json::json!({"active": true})).await;
    let cfg = OAuth2Layer::new(missing, "cid", "csecret").unwrap();
    let err = introspect_token(&cfg, "tok-1").await.unwrap_err();
    assert!(err.contains("missing sub"), "unexpected error: {err}");

    let empty =
        spawn_introspection_server_with(&COUNT, serde_json::json!({"active": true, "sub": ""}))
            .await;
    let cfg = OAuth2Layer::new(empty, "cid", "csecret").unwrap();
    let err = introspect_token(&cfg, "tok-2").await.unwrap_err();
    assert!(err.contains("missing sub"), "unexpected error: {err}");
}

/// 内省端点返回非 2xx（如 500）时必须报错，不得把错误页当 claims 解析。
#[tokio::test]
async fn introspection_non_success_status_is_error() {
    let app = axum::Router::new().route(
        "/introspect",
        post(async || (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "boom")),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let cfg = OAuth2Layer::new(format!("http://{addr}/introspect"), "cid", "csecret").unwrap();
    let err = introspect_token(&cfg, "tok-1").await.unwrap_err();
    assert!(err.contains("returned 500"), "unexpected error: {err}");
}

/// active=false 的 token 必须拒绝，即使响应带 sub。
#[tokio::test]
async fn introspection_inactive_token_is_rejected() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let url = spawn_introspection_server_with(
        &COUNT,
        serde_json::json!({"active": false, "sub": "user-1", "role": "admin"}),
    )
    .await;
    let cfg = OAuth2Layer::new(url, "cid", "csecret").unwrap();
    let err = introspect_token(&cfg, "tok-1").await.unwrap_err();
    assert!(err.contains("not active"), "unexpected error: {err}");
}

/// 内省响应缺少 active 字段（或非布尔）时按不活跃拒绝。
#[tokio::test]
async fn introspection_missing_active_field_is_rejected() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let url = spawn_introspection_server_with(
        &COUNT,
        serde_json::json!({"sub": "user-1", "scope": "read"}),
    )
    .await;
    let cfg = OAuth2Layer::new(url, "cid", "csecret").unwrap();
    let err = introspect_token(&cfg, "tok-1").await.unwrap_err();
    assert!(err.contains("not active"), "unexpected error: {err}");
}

/// 内省响应体不是 JSON 时按解析错误拒绝。
#[tokio::test]
async fn introspection_non_json_body_is_rejected() {
    let app = axum::Router::new().route("/introspect", post(async || "this is not json"));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let cfg = OAuth2Layer::new(format!("http://{addr}/introspect"), "cid", "csecret").unwrap();
    let err = introspect_token(&cfg, "tok-1").await.unwrap_err();
    assert!(err.contains("parse"), "unexpected error: {err}");
}

/// filter_claims：白名单不含 "*" 时丢弃未列出的 extra 字段。
#[test]
fn filter_claims_drops_non_whitelisted_extra() {
    let mut extra = HashMap::new();
    extra.insert("iss".to_string(), serde_json::json!("https://i"));
    extra.insert("email".to_string(), serde_json::json!("e@x.com"));
    let claims = AuthClaims {
        sub: "s".into(),
        exp: None,
        iat: None,
        role: Some("admin".into()),
        extra,
    };
    let whitelist: Vec<String> = ["iss"].iter().map(|s| s.to_string()).collect();
    let filtered = filter_claims(claims, &whitelist);
    assert!(filtered.extra.contains_key("iss"));
    assert!(
        !filtered.extra.contains_key("email"),
        "email must be dropped"
    );
    assert_eq!(
        filtered.role.as_deref(),
        Some("admin"),
        "structured fields kept"
    );
}

/// filter_claims：白名单含 "*" 时保留全部 extra。
#[test]
fn filter_claims_star_keeps_everything() {
    let mut extra = HashMap::new();
    extra.insert("anything".to_string(), serde_json::json!({"n": 1}));
    let claims = AuthClaims {
        sub: "s".into(),
        exp: None,
        iat: None,
        role: None,
        extra,
    };
    let whitelist: Vec<String> = ["*"].iter().map(|s| s.to_string()).collect();
    let filtered = filter_claims(claims, &whitelist);
    assert!(filtered.extra.contains_key("anything"));
}
