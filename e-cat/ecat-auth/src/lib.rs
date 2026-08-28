// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
mod apikey;
mod claims;
mod helpers;
mod jwt;
mod oauth2;

pub use apikey::{ApiKeyLayer, ApiKeyService};
pub use claims::AuthClaims;
pub use helpers::claims_from_request;
pub use jwt::{JwtAuthError, JwtAuthLayer, JwtAuthService};
pub use oauth2::{OAuth2Layer, OAuth2Service};

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;
    use std::collections::HashMap;
    use tower::Layer;

    #[test]
    fn bearer_extraction() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_static("Bearer mytoken123"),
        );
        assert_eq!(
            helpers::extract_bearer(&headers, "Authorization"),
            Some("mytoken123".into())
        );
    }

    #[test]
    fn bearer_extraction_no_header() {
        let headers = http::HeaderMap::new();
        assert_eq!(helpers::extract_bearer(&headers, "Authorization"), None);
    }

    #[test]
    fn bearer_extraction_wrong_prefix() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_static("Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="),
        );
        assert_eq!(helpers::extract_bearer(&headers, "Authorization"), None);
    }

    #[test]
    fn query_param_extraction() {
        assert_eq!(
            helpers::extract_query_param(Some("key=abc123&other=val"), "key"),
            Some("abc123".into())
        );
    }

    #[test]
    fn query_param_not_found() {
        assert_eq!(helpers::extract_query_param(Some("a=1&b=2"), "c"), None);
    }

    #[test]
    fn layer_construction() {
        let layer = JwtAuthLayer::new("secret-key-0123456789abcdefghijklmnopqrstuv")
            .expect("32+ byte secret accepted");
        let _layer = layer
            .require_claims(&["sub", "role"])
            .header_name("X-Auth-Token");
    }

    #[test]
    fn layer_rejects_weak_secret() {
        assert!(matches!(
            JwtAuthLayer::new("too-short"),
            Err(JwtAuthError::WeakKey)
        ));
    }

    #[test]
    fn api_key_layer_construction() {
        let mut keys = HashMap::new();
        keys.insert(
            "key1".into(),
            AuthClaims {
                sub: "user1".into(),
                exp: None,
                iat: None,
                role: Some("admin".into()),
                extra: HashMap::new(),
            },
        );
        let _layer = ApiKeyLayer::new(keys).query_param("api_key");
    }

    #[test]
    fn claims_subject_and_role() {
        let claims = AuthClaims {
            sub: "user42".into(),
            exp: None,
            iat: None,
            role: Some("editor".into()),
            extra: HashMap::new(),
        };
        assert_eq!(claims.subject(), "user42");
        assert_eq!(claims.role(), Some("editor"));
        assert!(claims.has_role("editor"));
        assert!(!claims.has_role("admin"));
    }

    #[test]
    fn claims_deserialize_with_defaults() {
        // 缺失字段全部落到 serde default：sub 空串、exp/iat/role None、extra 空
        let claims: AuthClaims = serde_json::from_str(r#"{"sub":"s1"}"#).unwrap();
        assert_eq!(claims.sub, "s1");
        assert!(claims.exp.is_none());
        assert!(claims.role.is_none());
        assert!(claims.extra.is_empty());

        let empty: AuthClaims = serde_json::from_str("{}").unwrap();
        assert_eq!(empty.sub, "");
    }

    #[test]
    fn claims_serialize_roundtrip_keeps_extra() {
        let mut extra = HashMap::new();
        extra.insert("scope".to_string(), serde_json::json!("read"));
        let claims = AuthClaims {
            sub: "u".into(),
            exp: Some(123),
            iat: Some(100),
            role: Some("admin".into()),
            extra,
        };
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&claims).unwrap()).unwrap();
        assert_eq!(v["sub"], "u");
        assert_eq!(v["role"], "admin");
        assert_eq!(v["extra"].as_object().map(|m| m.len()).unwrap_or(0), 0);
        assert_eq!(v["scope"], "read", "flattened extra at top level");
    }

    #[test]
    fn extract_bearer_trims_whitespace() {
        let mut headers = http::HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_static("Bearer   tok  "));
        assert_eq!(
            helpers::extract_bearer(&headers, "Authorization"),
            Some("tok".into())
        );
    }

    #[test]
    fn extract_bearer_lowercase_prefix_rejected() {
        let mut headers = http::HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_static("bearer tok"));
        assert_eq!(helpers::extract_bearer(&headers, "Authorization"), None);
    }

    #[test]
    fn extract_header_non_utf8_returns_none() {
        let mut headers = http::HeaderMap::new();
        headers.insert("X-K", HeaderValue::from_bytes(b"\xff\xfe").unwrap());
        assert_eq!(helpers::extract_header(&headers, "X-K"), None);
    }

    #[test]
    fn extract_query_param_first_match_wins() {
        assert_eq!(
            helpers::extract_query_param(Some("k=1&k=2"), "k"),
            Some("1".into())
        );
    }

    #[test]
    fn extract_query_param_empty_value() {
        assert_eq!(
            helpers::extract_query_param(Some("k="), "k"),
            Some("".into())
        );
    }

    #[test]
    fn extract_query_param_without_equals() {
        assert_eq!(helpers::extract_query_param(Some("k"), "k"), None);
        assert_eq!(helpers::extract_query_param(None, "k"), None);
    }

    #[tokio::test]
    async fn api_key_layer_header_auth_and_claims_injection() {
        use tower::ServiceExt;

        let mut keys = HashMap::new();
        keys.insert(
            "key1".into(),
            AuthClaims {
                sub: "user1".into(),
                exp: None,
                iat: None,
                role: Some("admin".into()),
                extra: HashMap::new(),
            },
        );
        let layer = ApiKeyLayer::new(keys);
        let svc = layer.layer(axum::routing::get(
            |req: axum::http::Request<axum::body::Body>| async move {
                let claims: &AuthClaims = req.extensions().get().unwrap();
                format!("{}:{}", claims.subject(), claims.role().unwrap_or(""))
            },
        ));

        let svc = svc;
        let resp = svc
            .oneshot(
                axum::http::Request::builder()
                    .header("X-API-Key", "key1")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), http::StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        assert_eq!(String::from_utf8_lossy(&body), "user1:admin");
    }

    #[tokio::test]
    async fn api_key_layer_rejects_unknown_key() {
        use tower::ServiceExt;

        let layer = ApiKeyLayer::new(HashMap::new());
        let svc = layer.layer(axum::routing::get(|| async { "ok" }));
        let resp = svc
            .oneshot(
                axum::http::Request::builder()
                    .header("X-API-Key", "nope")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), http::StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        assert!(
            String::from_utf8_lossy(&body).contains("invalid api key"),
            "got: {}",
            String::from_utf8_lossy(&body)
        );
    }

    #[tokio::test]
    async fn api_key_layer_query_param_fallback() {
        use tower::ServiceExt;

        let mut keys = HashMap::new();
        keys.insert(
            "key1".into(),
            AuthClaims {
                sub: "user1".into(),
                exp: None,
                iat: None,
                role: None,
                extra: HashMap::new(),
            },
        );
        let layer = ApiKeyLayer::new(keys).query_param("api_key");
        let svc = layer.layer(axum::routing::get(
            |req: axum::http::Request<axum::body::Body>| async move {
                let claims: &AuthClaims = req.extensions().get().unwrap();
                claims.subject().to_string()
            },
        ));

        // 无 header，query 兜底命中
        let resp = svc
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/?api_key=key1")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), http::StatusCode::OK);

        // query 里也没有 → 401
        let resp = svc
            .oneshot(
                axum::http::Request::builder()
                    .uri("/")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_key_layer_custom_header_name() {
        use tower::ServiceExt;

        let mut keys = HashMap::new();
        keys.insert(
            "k".into(),
            AuthClaims {
                sub: "s".into(),
                exp: None,
                iat: None,
                role: None,
                extra: HashMap::new(),
            },
        );
        let layer = ApiKeyLayer::new(keys).header_name("X-Custom-Key");
        let svc = layer.layer(axum::routing::get(|| async { "ok" }));

        let resp = svc
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .header("X-API-Key", "k")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            http::StatusCode::UNAUTHORIZED,
            "default header name must not match"
        );

        let resp = svc
            .oneshot(
                axum::http::Request::builder()
                    .header("X-Custom-Key", "k")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), http::StatusCode::OK);
    }
}
