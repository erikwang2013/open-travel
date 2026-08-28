// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use axum::http::HeaderMap;
use std::collections::HashMap;
use std::sync::Arc;

pub enum VersionStrategy {
    PathPrefix,
    Header,
}

pub struct VersionedRouter {
    versions: HashMap<String, axum::Router>,
    default_version: Option<String>,
    strategy: VersionStrategy,
}

impl VersionedRouter {
    pub fn new(strategy: VersionStrategy) -> Self {
        Self {
            versions: HashMap::new(),
            default_version: None,
            strategy,
        }
    }

    pub fn add_version(mut self, version: impl Into<String>, router: axum::Router) -> Self {
        self.versions.insert(version.into(), router);
        self
    }

    pub fn default_version(mut self, version: impl Into<String>) -> Self {
        self.default_version = Some(version.into());
        self
    }

    pub fn build(self) -> axum::Router {
        match self.strategy {
            VersionStrategy::PathPrefix => self.build_path_router(),
            VersionStrategy::Header => self.build_header_router(),
        }
    }

    fn build_path_router(self) -> axum::Router {
        let default = self
            .default_version
            .and_then(|v| self.versions.get(&v).cloned());
        let mut router = axum::Router::new();
        for (version, version_router) in self.versions {
            router = router.nest(&format!("/{version}"), version_router);
        }
        if let Some(default_router) = default {
            router = router.merge(default_router);
        }
        router
    }

    fn build_header_router(self) -> axum::Router {
        use axum::extract::State;

        #[derive(Clone)]
        struct VersionState {
            names: Arc<Vec<String>>,
        }

        let version_names: Arc<Vec<String>> = Arc::new(self.versions.keys().cloned().collect());

        let mut router = axum::Router::new();
        for (_ver, vr) in self.versions {
            router = router.merge(vr);
        }

        let state = VersionState {
            names: Arc::clone(&version_names),
        };
        router = router.layer(axum::middleware::from_fn_with_state(
            state,
            |State(s): State<VersionState>,
             req: axum::http::Request<axum::body::Body>,
             next: axum::middleware::Next| async move {
                if let Some(ver) = extract_version(req.headers())
                    && !s.names.contains(&ver)
                {
                    // 直接构造响应，避免 builder+unwrap（http::Error 无意义的生产 panic 面）
                    let mut resp =
                        axum::http::Response::new(axum::body::Body::from("unknown version"));
                    *resp.status_mut() = axum::http::StatusCode::NOT_FOUND;
                    return resp;
                }
                next.run(req).await
            },
        ));
        router
    }
}

pub fn extract_version(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Accept")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .find(|p| p.trim().starts_with("version="))
                .map(|p| {
                    p.trim()
                        .trim_start_matches("version=")
                        .trim_matches('"')
                        .to_string()
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;

    async fn health() -> &'static str {
        "ok"
    }

    #[test]
    fn path_versioned_router_builds() {
        let v1 = axum::Router::new().route("/health", get(health));
        let v2 = axum::Router::new().route("/health", get(health));
        let router = VersionedRouter::new(VersionStrategy::PathPrefix)
            .add_version("v1", v1)
            .add_version("v2", v2)
            .default_version("v1")
            .build();
        assert!(router.has_routes());
    }

    #[test]
    fn header_versioned_router_builds() {
        let v1 = axum::Router::new().route("/health", get(health));
        let v2 = axum::Router::new().route("/users", get(health));
        let router = VersionedRouter::new(VersionStrategy::Header)
            .add_version("v1", v1)
            .add_version("v2", v2)
            .default_version("v1")
            .build();
        assert!(router.has_routes());
    }

    #[test]
    fn extract_version_from_accept() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Accept",
            "application/json; version=\"v2\"".parse().unwrap(),
        );
        assert_eq!(extract_version(&headers), Some("v2".into()));
    }

    /// N6：header 策略下未知版本必须 404（覆盖原 builder+unwrap 的生产路径），
    /// 已知版本与无版本头正常放行。
    #[tokio::test]
    async fn header_strategy_rejects_unknown_version() {
        use tower::ServiceExt;

        let router = VersionedRouter::new(VersionStrategy::Header)
            .add_version("v1", axum::Router::new().route("/health", get(health)))
            .build();
        let req = axum::http::Request::builder()
            .uri("/health")
            .header("Accept", "application/json; version=\"v9\"")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);

        let ok_req = axum::http::Request::builder()
            .uri("/health")
            .header("Accept", "application/json; version=\"v1\"")
            .body(axum::body::Body::empty())
            .unwrap();
        let ok_resp = router.oneshot(ok_req).await.unwrap();
        assert_eq!(ok_resp.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn path_strategy_routes_versioned_and_default() {
        use tower::ServiceExt;

        let router = VersionedRouter::new(VersionStrategy::PathPrefix)
            .add_version("v1", axum::Router::new().route("/health", get(health)))
            .default_version("v1")
            .build();

        let resp = router
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/v1/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        // 默认版本合并到根路径
        let resp = router
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        // 未注册版本 → 404（nest 无匹配回落 404）
        let resp = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/v9/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn extract_version_missing_or_malformed() {
        assert_eq!(extract_version(&HeaderMap::new()), None);

        let mut headers = HeaderMap::new();
        headers.insert("Accept", "application/json".parse().unwrap());
        assert_eq!(extract_version(&headers), None);

        let mut headers = HeaderMap::new();
        headers.insert("Accept", "version=".parse().unwrap());
        assert_eq!(extract_version(&headers), Some("".into()));
    }

    #[test]
    fn extract_version_without_quotes() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", "version=v2".parse().unwrap());
        assert_eq!(extract_version(&headers), Some("v2".into()));
    }
}
