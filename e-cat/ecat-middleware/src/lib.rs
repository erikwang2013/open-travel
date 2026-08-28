// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
mod logging;
mod ratelimit;
#[cfg(feature = "redis")]
mod ratelimit_redis;
mod recovery;
mod retry;
mod timeout;
mod tracing;
mod validate;

#[cfg(feature = "cors")]
pub use tower_http::cors::{AllowOrigin, Any, CorsLayer};

pub use logging::LoggingLayer;
pub use ratelimit::{MemoryStore, RateLimitLayer, RateLimitStore};
#[cfg(feature = "redis")]
pub use ratelimit_redis::RedisRateLimitStore;
pub use recovery::RecoveryLayer;
pub use retry::{DefaultRule, RetryLayer, RetryRule, RetryService, exponential_backoff};
pub use timeout::TimeoutLayer;
pub use tracing::TracingLayer;
pub use validate::{FnValidator, RequestValidator, ValidateError, ValidateLayer, ValidateService};

#[cfg(all(test, feature = "cors"))]
mod cors_tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;

    /// 回归（6e5d15a M1 挂载事故）：CorsLayer 必须可挂载到 axum Router
    /// 且能应答 OPTIONS 预检请求。
    #[tokio::test]
    async fn cors_layer_mounts_and_answers_preflight() {
        use tower::ServiceExt;
        let app = axum::Router::new()
            .route("/", axum::routing::get(|| async { "ok" }))
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods([axum::http::Method::GET]),
            );
        let req = Request::builder()
            .method("OPTIONS")
            .uri("/")
            .header("origin", "https://example.com")
            .header("access-control-request-method", "GET")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get("access-control-allow-origin")
                .and_then(|v| v.to_str().ok()),
            Some("*"),
            "preflight must carry access-control-allow-origin"
        );
    }

    #[tokio::test]
    async fn cors_actual_request_gets_allow_origin() {
        use tower::ServiceExt;
        let app = axum::Router::new()
            .route("/", axum::routing::get(|| async { "ok" }))
            .layer(CorsLayer::new().allow_origin(Any));
        let req = Request::builder()
            .method("GET")
            .uri("/")
            .header("origin", "https://example.com")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        assert!(resp.headers().contains_key("access-control-allow-origin"));
    }
}
