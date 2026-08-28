// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
mod metrics_layer;

use axum::Router;
use axum::response::IntoResponse;
use axum::routing::get;
use prometheus::{Encoder, Registry, TextEncoder};
use std::sync::OnceLock;

pub use metrics_layer::{MetricsLayer, MetricsService};

static REGISTRY: OnceLock<Registry> = OnceLock::new();

pub fn registry() -> &'static Registry {
    REGISTRY.get_or_init(Registry::new)
}

pub fn metrics_text() -> String {
    metrics_text_for(registry())
}

fn metrics_text_for(reg: &Registry) -> String {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    if encoder.encode(&reg.gather(), &mut buffer).is_err() {
        return String::from("# metrics encoding failed\n");
    }
    if buffer.is_empty() {
        // 空 registry（无指标注册）：输出提示注释行而非空 body——
        // 监控可区分"健康但无数据"与"异常"（纯空响应无法区分）。
        // '#' 开头为 Prometheus 注释行，scrape 仍合法（0 个指标）。
        return String::from("# no metrics registered\n");
    }
    String::from_utf8(buffer).unwrap_or_else(|_| String::from("# metrics: invalid utf-8\n"))
}

pub fn metrics_router() -> Router {
    async fn handler() -> impl IntoResponse {
        (
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; version=0.0.4; charset=utf-8",
            )],
            metrics_text(),
        )
    }
    Router::new().route("/metrics", get(handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_singleton() {
        let r1 = registry() as *const Registry;
        let r2 = registry() as *const Registry;
        assert_eq!(r1, r2);
    }

    #[test]
    fn metrics_text_does_not_panic() {
        let text = metrics_text();
        // empty registry produces empty or minimal output — just check it's valid UTF-8
        let _ = text;
    }

    /// 第三轮：空 registry（无指标注册）时输出提示注释行而非空 body——
    /// 监控可区分"健康但无数据"与"异常"（纯空响应无法区分）。
    /// 用独立 Registry 断言（MetricsLayer 测试会向全局 registry 注册指标，
    /// 与并行测试的精确匹配存在竞态）。
    #[test]
    fn metrics_text_marks_empty_registry() {
        let text = metrics_text_for(&Registry::new());
        assert_eq!(text, "# no metrics registered\n", "got: {text}");
    }

    #[tokio::test]
    async fn metrics_router_serves_prometheus_text() {
        use tower::ServiceExt;
        let router = metrics_router();
        let resp = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let ct = resp
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(
            ct, "text/plain; version=0.0.4; charset=utf-8",
            "got content-type: {ct}"
        );
    }

    #[test]
    fn metrics_text_includes_registered_metrics() {
        let reg = Registry::new();
        let counter =
            prometheus::Counter::with_opts(prometheus::Opts::new("ecat_test_total", "test"))
                .unwrap();
        reg.register(Box::new(counter.clone())).unwrap();
        counter.inc();
        let text = metrics_text_for(&reg);
        assert!(text.contains("ecat_test_total 1"), "got: {text}");
    }

    #[tokio::test]
    async fn metrics_router_404_on_other_paths() {
        use tower::ServiceExt;
        let router = metrics_router();
        let resp = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/other")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn metrics_router_rejects_post() {
        use tower::ServiceExt;
        let router = metrics_router();
        let resp = router
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/metrics")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::METHOD_NOT_ALLOWED);
    }
}
