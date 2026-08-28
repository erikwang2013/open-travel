// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Initialize structured logging with env filter.
///
/// NOTE: only one subscriber can be installed per process. Do not call this
/// together with `ecat_tracing_otlp::init` (or any other subscriber init);
/// the second `init` would panic with "a global default trace dispatcher
/// has already been set".
pub fn init(service_name: &str) {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!(service = service_name, "tracing initialized");
}

/// Tower Layer that creates a request span with trace_id injection.
#[derive(Clone)]
pub struct TracingLayer {
    service_name: String,
}

impl TracingLayer {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }
}

impl<S> Layer<S> for TracingLayer {
    type Service = TracingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TracingService {
            inner,
            service_name: self.service_name.clone(),
        }
    }
}

#[derive(Clone)]
pub struct TracingService<S> {
    inner: S,
    service_name: String,
}

impl<S, B> Service<http::Request<B>> for TracingService<S>
where
    S: Service<http::Request<B>> + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<B>) -> Self::Future {
        // 从请求头提取 trace_id（canonical x-ecat-trace-id 优先，traceparent
        // 兜底），记录到 span 字段；请求无 trace id 时留空字段。
        let trace_id = extract_trace_id(req.headers());
        let span = tracing::info_span!(
            "request",
            service = %self.service_name,
            trace_id = tracing::field::Empty,
        );
        if let Some(id) = trace_id {
            span.record("trace_id", id);
        }
        let fut = self.inner.call(req);
        Box::pin(async move {
            let _guard = span.enter();
            fut.await
        })
    }
}

/// Extract trace_id from request headers for propagation.
///
/// Reads the canonical [`ecat_metadata::TRACE_ID`] header first, then
/// falls back to the W3C `traceparent` header.
pub fn extract_trace_id(headers: &http::HeaderMap) -> Option<String> {
    headers
        .get(ecat_metadata::TRACE_ID)
        .or_else(|| headers.get("traceparent"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Inject trace_id into a header map for downstream calls.
///
/// Carries forward an existing upstream trace id (canonical header first,
/// then W3C `traceparent`) so the trace chain is preserved across services;
/// only when no upstream trace id is present does it generate a random
/// 32-hex-char trace id (UUID v4) under the canonical
/// [`ecat_metadata::TRACE_ID`] header.
pub fn inject_trace_id(headers: &mut http::HeaderMap) {
    if extract_trace_id(headers).is_some() {
        return;
    }
    let trace_id = uuid::Uuid::new_v4().simple().to_string();
    if let Ok(v) = http::HeaderValue::from_str(&trace_id) {
        headers.insert(ecat_metadata::TRACE_ID, v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracing_layer_constructs() {
        let _layer = TracingLayer::new("test-service");
    }

    #[test]
    fn extract_empty_headers() {
        let headers = http::HeaderMap::new();
        assert_eq!(extract_trace_id(&headers), None);
    }

    #[test]
    fn extract_prefers_canonical_header_over_traceparent() {
        let mut headers = http::HeaderMap::new();
        headers.insert(ecat_metadata::TRACE_ID, "abc123".parse().unwrap());
        headers.insert("traceparent", "tp-000".parse().unwrap());
        assert_eq!(extract_trace_id(&headers).as_deref(), Some("abc123"));
    }

    #[test]
    fn extract_falls_back_to_traceparent() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            "traceparent",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
                .parse()
                .unwrap(),
        );
        assert_eq!(
            extract_trace_id(&headers).unwrap(),
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
        );
    }

    #[test]
    fn inject_trace_id_adds_header() {
        let mut headers = http::HeaderMap::new();
        inject_trace_id(&mut headers);
        let value = headers
            .get(ecat_metadata::TRACE_ID)
            .expect("canonical header set");
        assert_eq!(value.len(), 32, "32-hex-char trace id");
        assert!(
            value
                .to_str()
                .unwrap()
                .chars()
                .all(|c| c.is_ascii_hexdigit()),
            "trace id is hex"
        );
    }

    /// N4：上游 canonical trace_id 已存在时沿用，不覆盖生成新 UUID。
    #[test]
    fn inject_preserves_existing_trace_id() {
        let mut headers = http::HeaderMap::new();
        headers.insert(ecat_metadata::TRACE_ID, "abc123".parse().unwrap());
        inject_trace_id(&mut headers);
        assert_eq!(
            headers
                .get(ecat_metadata::TRACE_ID)
                .expect("canonical header set")
                .to_str()
                .unwrap(),
            "abc123"
        );
    }

    /// N4：上游只有 traceparent 时沿用链路（extract 的兜底语义），
    /// 不覆盖 canonical header，也不打断链路。
    #[test]
    fn inject_preserves_upstream_traceparent() {
        let mut headers = http::HeaderMap::new();
        headers.insert("traceparent", "tp-000".parse().unwrap());
        inject_trace_id(&mut headers);
        assert!(
            headers.get(ecat_metadata::TRACE_ID).is_none(),
            "must not generate a fresh trace id when upstream traceparent exists"
        );
        assert_eq!(
            headers.get("traceparent").unwrap().to_str().unwrap(),
            "tp-000"
        );
    }

    #[test]
    fn extract_skips_non_utf8_header() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            ecat_metadata::TRACE_ID,
            http::HeaderValue::from_bytes(b"\xff\xfe").unwrap(),
        );
        assert_eq!(extract_trace_id(&headers), None);
    }

    #[test]
    fn extract_canonical_header_alone() {
        let mut headers = http::HeaderMap::new();
        headers.insert(ecat_metadata::TRACE_ID, "t1".parse().unwrap());
        assert_eq!(extract_trace_id(&headers).as_deref(), Some("t1"));
    }

    #[tokio::test]
    async fn service_preserves_response() {
        use tower::ServiceExt;

        let svc = TracingLayer::new("svc").layer(tower::service_fn(
            |_req: http::Request<()>| async move {
                Ok::<_, std::convert::Infallible>(
                    http::Response::builder()
                        .status(http::StatusCode::CREATED)
                        .body("ok".to_string())
                        .unwrap(),
                )
            },
        ));
        let resp = svc
            .oneshot(http::Request::builder().body(()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), http::StatusCode::CREATED);
        assert_eq!(resp.into_body(), "ok");
    }

    #[derive(Clone)]
    struct CaptureWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for CaptureWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// N4：request span 必须记录 trace_id 字段（与 CHANGELOG 2.3.3 声明一致，
    /// 头名 x-ecat-trace-id 与 ecat-metadata 一致）。
    #[tokio::test]
    async fn span_records_trace_id() {
        use tower::ServiceExt;

        let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let writer = CaptureWriter(std::sync::Arc::clone(&buf));
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let svc = TracingLayer::new("svc").layer(tower::service_fn(
            |_req: http::Request<()>| async move {
                Ok::<_, std::convert::Infallible>(http::Response::new(()))
            },
        ));
        svc.oneshot(
            http::Request::builder()
                .header(ecat_metadata::TRACE_ID, "abc123")
                .body(())
                .unwrap(),
        )
        .await
        .unwrap();

        let out = String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap();
        assert!(
            out.contains("trace_id=\"abc123\""),
            "span must record trace_id, got: {out}"
        );
    }

    /// N4：无 trace id 请求头时 span 正常创建（trace_id 为空字段）。
    #[tokio::test]
    async fn span_works_without_trace_id() {
        use tower::ServiceExt;

        let svc = TracingLayer::new("svc").layer(tower::service_fn(
            |_req: http::Request<()>| async move {
                Ok::<_, std::convert::Infallible>(http::Response::new(()))
            },
        ));
        let resp = svc
            .oneshot(http::Request::builder().body(()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), http::StatusCode::OK);
    }
}
