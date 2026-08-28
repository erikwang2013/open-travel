// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::OnceLock;
use std::task::{Context, Poll};
use std::time::Instant;

use prometheus::{CounterVec, HistogramOpts, HistogramVec, Opts};
use tower::{Layer, Service};

use crate::registry;

static REQUESTS_TOTAL: OnceLock<CounterVec> = OnceLock::new();
static REQUEST_DURATION: OnceLock<HistogramVec> = OnceLock::new();

const LABELS: &[&str] = &["method", "path", "status"];

fn requests_total() -> &'static CounterVec {
    REQUESTS_TOTAL.get_or_init(|| {
        let m = CounterVec::new(
            Opts::new(
                "ecat_http_requests_total",
                "Total HTTP requests handled by the service",
            ),
            LABELS,
        )
        .expect("valid counter vec");
        // 重复注册（AlreadyReg）无害——全局 registry 只注册一次。
        let _ = registry().register(Box::new(m.clone()));
        m
    })
}

fn request_duration() -> &'static HistogramVec {
    REQUEST_DURATION.get_or_init(|| {
        // 5ms ~ 10s 覆盖常规请求时延区间（12 桶指数分布）。
        let buckets =
            prometheus::exponential_buckets(0.005, 2.0, 12).expect("valid histogram buckets");
        let m = HistogramVec::new(
            HistogramOpts::new(
                "ecat_http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .buckets(buckets),
            LABELS,
        )
        .expect("valid histogram vec");
        let _ = registry().register(Box::new(m.clone()));
        m
    })
}

/// Tower Layer：记录 HTTP 请求指标（计数 + 时长直方图）到全局 registry。
///
/// 指标名 `ecat_http_requests_total` / `ecat_http_request_duration_seconds`，
/// 标签 method/path/status；内层服务出错时 status="error"。指标注册于进程级
/// 全局 registry，与 [`crate::metrics_router`] 的 /metrics 端点共享。
///
/// path 标签默认取完整路径——路径含 ID 等高基数场景请用 [`MetricsLayer::with_path_fn`]
/// 归一化/脱敏，避免指标基数爆炸。
#[derive(Clone)]
pub struct MetricsLayer {
    path_fn: Arc<dyn Fn(&axum::http::Uri) -> String + Send + Sync>,
}

impl MetricsLayer {
    pub fn new() -> Self {
        Self {
            path_fn: Arc::new(|uri| uri.path().to_string()),
        }
    }

    /// 自定义 path 标签提取（如 `/users/42` → `/users/:id`、去掉查询参数等）。
    pub fn with_path_fn<F>(self, f: F) -> Self
    where
        F: Fn(&axum::http::Uri) -> String + Send + Sync + 'static,
    {
        Self {
            path_fn: Arc::new(f),
        }
    }
}

impl Default for MetricsLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct MetricsService<S> {
    inner: S,
    path_fn: Arc<dyn Fn(&axum::http::Uri) -> String + Send + Sync>,
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsService {
            inner,
            path_fn: Arc::clone(&self.path_fn),
        }
    }
}

impl<S, B, D> Service<axum::http::Request<B>> for MetricsService<S>
where
    S: Service<axum::http::Request<B>, Response = axum::http::Response<D>> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: Send + 'static,
{
    type Response = axum::http::Response<D>;
    // 透传 inner 错误（tower-http 同款模式）：挂 axum Router 时
    // Error 自动为 Infallible（Router::layer 要求 Error: Into<Infallible>）。
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: axum::http::Request<B>) -> Self::Future {
        let method = req.method().as_str().to_string();
        let path = (self.path_fn)(req.uri());
        let start = Instant::now();
        let fut = self.inner.call(req);
        Box::pin(async move {
            let result = fut.await;
            let status = match &result {
                Ok(resp) => resp.status().as_str().to_string(),
                Err(_) => "error".to_string(),
            };
            requests_total()
                .with_label_values(&[&method, &path, &status])
                .inc();
            request_duration()
                .with_label_values(&[&method, &path, &status])
                .observe(start.elapsed().as_secs_f64());
            result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    #[test]
    fn constructs_with_defaults() {
        let _ = MetricsLayer::new();
        let _ = MetricsLayer::default();
        let _ = MetricsLayer::new().with_path_fn(|_| "all".to_string());
    }

    #[tokio::test]
    async fn records_counter_and_duration() {
        let svc = MetricsLayer::new().layer(tower::service_fn(
            |_req: axum::http::Request<()>| async move {
                Ok::<_, std::convert::Infallible>(axum::http::Response::new(()))
            },
        ));
        svc.oneshot(
            axum::http::Request::builder()
                .uri("/hello")
                .body(())
                .unwrap(),
        )
        .await
        .unwrap();

        let counter = requests_total()
            .get_metric_with_label_values(&["GET", "/hello", "200"])
            .expect("counter recorded");
        assert!(counter.get() >= 1.0);
        let hist = request_duration().get_metric_with_label_values(&["GET", "/hello", "200"]);
        assert!(hist.is_ok(), "duration histogram recorded");
    }

    #[tokio::test]
    async fn records_error_status_when_service_fails() {
        let svc = MetricsLayer::new().layer(tower::service_fn(
            |_req: axum::http::Request<()>| async move {
                Err::<axum::http::Response<()>, _>(std::io::Error::other("boom"))
            },
        ));
        assert!(
            svc.oneshot(
                axum::http::Request::builder()
                    .uri("/fail")
                    .body(())
                    .unwrap(),
            )
            .await
            .is_err()
        );

        let counter = requests_total()
            .get_metric_with_label_values(&["GET", "/fail", "error"])
            .expect("error-status counter recorded");
        assert!(counter.get() >= 1.0);
    }

    #[tokio::test]
    async fn path_fn_redacts_high_cardinality_paths() {
        let layer = MetricsLayer::new().with_path_fn(|uri| {
            let p = uri.path();
            if p.starts_with("/users/") {
                "/users/:id".to_string()
            } else {
                p.to_string()
            }
        });
        let svc = layer.layer(tower::service_fn(
            |_req: axum::http::Request<()>| async move {
                Ok::<_, std::convert::Infallible>(axum::http::Response::new(()))
            },
        ));
        for id in ["42", "43"] {
            svc.clone()
                .oneshot(
                    axum::http::Request::builder()
                        .uri(format!("/users/{id}"))
                        .body(())
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        let redacted = requests_total()
            .get_metric_with_label_values(&["GET", "/users/:id", "200"])
            .expect("redacted path counter recorded");
        assert_eq!(
            redacted.get(),
            2.0,
            "both requests share the redacted label"
        );
        // get_metric_with_label_values 是 get-or-create 语义（查询即创建空指标），
        // 无法用 NotFound 断言；改查 gathered 文本确认原始路径从未作为标签出现。
        let text = crate::metrics_text();
        assert!(
            !text.contains("path=\"/users/42\""),
            "raw high-cardinality path must not be recorded, got: {text}"
        );
    }
}
