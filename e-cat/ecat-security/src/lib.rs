// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use http::{Request, StatusCode};
use security_rust::Scanner;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskCtx, Poll};

pub use security_rust::{AttackCategory, DetectionResult, ScannerBuilder, Severity};

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("attack blocked: {0}")]
    AttackBlocked(String),
    /// 请求体超过 body_limit 上限（读体阶段即拒绝，不进入扫描）。
    #[error("request body too large")]
    BodyTooLarge,
    #[error("inner error: {0}")]
    Inner(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl SecurityError {
    pub fn to_http_status(&self) -> StatusCode {
        match self {
            Self::AttackBlocked(_) => StatusCode::FORBIDDEN,
            Self::BodyTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::Inner(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// 拦截结果映射为 HTTP 响应：攻击拦截为 403，请求体超限为 413，内部
/// 错误为 500。内部错误的原始信息只进日志，响应体保持通用文案，避免
/// 把内部错误细节（文件路径、SQL、堆栈）泄露给客户端。
impl axum::response::IntoResponse for SecurityError {
    fn into_response(self) -> axum::response::Response {
        let status = self.to_http_status();
        let body = match &self {
            Self::AttackBlocked(types) => {
                format!(r#"{{"error":"attack blocked","types":"{types}"}}"#)
            }
            Self::BodyTooLarge => r#"{"error":"request body too large"}"#.to_string(),
            Self::Inner(e) => {
                tracing::error!(error = %e, "security middleware internal error");
                r#"{"error":"internal server error"}"#.to_string()
            }
        };
        (status, body).into_response()
    }
}

/// Wraps `security_rust::Scanner` with convenient constructors.
pub struct SecurityScanner {
    scanner: Scanner,
}

impl SecurityScanner {
    /// Create scanner with default detector configuration.
    pub fn new() -> Self {
        Self {
            scanner: Scanner::default(),
        }
    }

    /// Scan a single string through all detectors.
    pub fn scan(&self, input: &str) -> Vec<DetectionResult> {
        self.scanner.scan(input)
    }

    /// Scan multiple request parts (path, headers, body, etc.).
    pub fn scan_parts(&self, parts: &[&str]) -> Vec<DetectionResult> {
        let mut results = Vec::with_capacity(parts.len() * 2);
        for part in parts {
            results.extend(self.scanner.scan(part));
        }
        results
    }

    /// Scan request body bytes. Converts to string for analysis.
    pub fn scan_body(&self, body: &[u8]) -> Vec<DetectionResult> {
        if let Ok(s) = std::str::from_utf8(body) {
            self.scanner.scan(s)
        } else {
            Vec::new()
        }
    }
}

impl Default for SecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Logs detections and returns a blocking error when a High/Critical attack
/// was found. Shared by the header-scanning and body-scanning middlewares.
fn evaluate(results: &[DetectionResult]) -> Option<SecurityError> {
    let mut blocked = false;
    for r in results {
        tracing::warn!(
            attack_type = %r.attack_type,
            category = ?r.category,
            severity = ?r.severity,
            matched = %r.matched_pattern,
            "attack detected"
        );
        // jwt_attack 的宽正则（ey..ey.. 匹配一切标准 JWT）会误伤合法 token：
        // 服务端鉴权由 JwtAuthLayer 验签把关（alg:none/伪造签名在验签层拒绝），
        // 此处仅记日志不拦截。
        if r.attack_type != "jwt_attack" && matches!(r.severity, Severity::High | Severity::Critical)
        {
            blocked = true;
        }
    }
    if blocked {
        let attack_types: Vec<String> = results.iter().map(|r| r.attack_type.to_string()).collect();
        return Some(SecurityError::AttackBlocked(attack_types.join(", ")));
    }
    None
}

/// 百分号解码，仅用于扫描检测；原始 URI 在响应/日志/转发中保持不变。
/// 无效的 % 序列原样保留，非 UTF-8 解码结果按 replacement 字符处理。
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Builds the scan list from URI and headers (shared by both middlewares).
/// URI 先做百分号解码再扫描：`?q=SELECT%20*%20FROM%20users` 若不解码会绕过
/// 要求字面空白的 SQLi 正则。解码仅用于检测，URI 本身不变。
/// 代理拓扑头（X-Forwarded-For/X-Real-IP/Forwarded 等）由网关重写，携带
/// 内网 IP（如 docker 网关 172.x），SSRF 检测会误伤内网部署，跳过不扫。
/// Authorization 同理：JWT 是本站正常鉴权流量，jwt_attack 规则会误报，
/// 扫描跳过（token 的校验由 JwtAuthLayer 负责）。
fn is_proxy_header(name: &http::header::HeaderName) -> bool {
    matches!(
        name.as_str(),
        "x-forwarded-for"
            | "x-real-ip"
            | "forwarded"
            | "x-forwarded-host"
            | "x-forwarded-proto"
            | "x-forwarded-port"
            | "authorization"
    )
}

fn request_parts<B>(req: &Request<B>) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    parts.push(percent_decode(&req.uri().to_string()));
    for (name, value) in req.headers() {
        if is_proxy_header(name) {
            continue;
        }
        if let Ok(v) = value.to_str() {
            parts.push(v.to_string());
        }
    }
    parts
}

// ── Tower Layer ──

#[derive(Clone)]
pub struct SecurityLayer {
    scanner: Arc<SecurityScanner>,
}

impl SecurityLayer {
    pub fn new() -> Self {
        Self {
            scanner: Arc::new(SecurityScanner::new()),
        }
    }
}

impl Default for SecurityLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> tower::Layer<S> for SecurityLayer {
    type Service = SecurityService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityService {
            inner,
            scanner: Arc::clone(&self.scanner),
        }
    }
}

#[derive(Clone)]
pub struct SecurityService<S> {
    inner: S,
    scanner: Arc<SecurityScanner>,
}

impl<S, B> tower::Service<Request<B>> for SecurityService<S>
where
    S: tower::Service<Request<B>> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    // 拦截时直接返回 403 响应而非 Err：Err 经服务端 no_error 归一到
    // Infallible 时触发 unreachable panic，会打崩 worker 线程
    S::Response: From<axum::response::Response> + Send,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = SecurityError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut TaskCtx<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner
            .poll_ready(cx)
            .map_err(|e| SecurityError::Inner(e.into()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let scanner = Arc::clone(&self.scanner);
        let parts = request_parts(&req);
        let strings: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
        let results = scanner.scan_parts(&strings);

        if let Some(err) = evaluate(&results) {
            use axum::response::IntoResponse;
            let resp: S::Response = err.into_response().into();
            return Box::pin(async move { Ok(resp) });
        }

        let fut = self.inner.call(req);
        Box::pin(async move { fut.await.map_err(|e| SecurityError::Inner(e.into())) })
    }
}

// ── Body-scanning variant ──

/// Tower layer that scans URI, headers, **and** the request body.
///
/// The body is read exactly once (up to [`body_limit`](Self::body_limit)
/// bytes) and passed through to the inner service, so handlers still receive
/// the full payload. Use this instead of [`SecurityLayer`] when request
/// bodies must also be checked for SQLi/XSS payloads.
#[derive(Clone)]
pub struct SecurityBodyLayer {
    scanner: Arc<SecurityScanner>,
    body_limit: usize,
}

impl SecurityBodyLayer {
    pub fn new() -> Self {
        Self {
            scanner: Arc::new(SecurityScanner::new()),
            body_limit: 10 * 1024 * 1024,
        }
    }

    /// Maximum body size (in bytes) that will be buffered and scanned.
    /// Larger bodies are rejected with a 413 (Payload Too Large) rather
    /// than buffered.
    pub fn body_limit(mut self, limit: usize) -> Self {
        self.body_limit = limit;
        self
    }
}

impl Default for SecurityBodyLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> tower::Layer<S> for SecurityBodyLayer {
    type Service = SecurityBodyService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityBodyService {
            inner,
            scanner: Arc::clone(&self.scanner),
            body_limit: self.body_limit,
        }
    }
}

#[derive(Clone)]
pub struct SecurityBodyService<S> {
    inner: S,
    scanner: Arc<SecurityScanner>,
    body_limit: usize,
}

impl<S> tower::Service<Request<axum::body::Body>> for SecurityBodyService<S>
where
    S: tower::Service<Request<axum::body::Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = S::Response;
    type Error = SecurityError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut TaskCtx<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner
            .poll_ready(cx)
            .map_err(|e| SecurityError::Inner(e.into()))
    }

    fn call(&mut self, req: Request<axum::body::Body>) -> Self::Future {
        let scanner = Arc::clone(&self.scanner);
        let body_limit = self.body_limit;
        let mut inner = self.inner.clone();
        let parts = request_parts(&req);
        let strings: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
        let header_results = scanner.scan_parts(&strings);

        Box::pin(async move {
            let (parts, body) = req.into_parts();
            // Read the body exactly once; the collected bytes become the new
            // body so downstream handlers can still access the payload.
            let bytes = match axum::body::to_bytes(body, body_limit).await {
                Ok(bytes) => bytes,
                Err(e) => {
                    // LengthLimitError = 超限：413；其余读体错误为 500
                    let inner: Box<dyn std::error::Error + Send + Sync> = e.into_inner();
                    if inner
                        .downcast_ref::<http_body_util::LengthLimitError>()
                        .is_some()
                    {
                        return Err(SecurityError::BodyTooLarge);
                    }
                    return Err(SecurityError::Inner(inner));
                }
            };

            let mut results = header_results;
            results.extend(scanner.scan_body(&bytes));

            if let Some(err) = evaluate(&results) {
                return Err(err);
            }

            let req = Request::from_parts(parts, axum::body::Body::from(bytes));
            inner
                .call(req)
                .await
                .map_err(|e| SecurityError::Inner(e.into()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_detects_sql_injection() {
        let s = SecurityScanner::new();
        let results = s.scan("SELECT * FROM users; DROP TABLE users;");
        assert!(!results.is_empty());
    }

    #[test]
    fn scanner_detects_xss() {
        let s = SecurityScanner::new();
        let results = s.scan("<script>alert('xss')</script>");
        assert!(!results.is_empty());
    }

    #[test]
    fn scanner_clean_input_no_detection() {
        let s = SecurityScanner::new();
        let results = s.scan("hello world");
        assert!(results.is_empty());
    }

    #[test]
    fn scanner_scan_parts_aggregates() {
        let s = SecurityScanner::new();
        let results = s.scan_parts(&["clean", "<script>x</script>"]);
        assert!(!results.is_empty());
    }

    #[test]
    fn attack_blocked_maps_to_403() {
        use axum::response::IntoResponse;
        let resp = SecurityError::AttackBlocked("sqli".into()).into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn inner_error_maps_to_500() {
        use axum::response::IntoResponse;
        let resp = SecurityError::Inner(Box::new(std::io::Error::other("boom"))).into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn layer_constructs() {
        let _layer = SecurityLayer::new();
    }

    #[test]
    fn layer_default_constructs() {
        let _layer: SecurityLayer = Default::default();
    }

    #[test]
    fn body_layer_constructs() {
        let _layer = SecurityBodyLayer::new().body_limit(1024);
    }

    #[test]
    fn body_layer_default_constructs() {
        let _layer: SecurityBodyLayer = Default::default();
    }

    #[tokio::test]
    async fn body_layer_blocks_attack_in_body() {
        use tower::Layer as _;
        use tower::ServiceExt;

        let layer = SecurityBodyLayer::new();
        let svc = layer.layer(tower::service_fn(|_: Request<axum::body::Body>| async {
            Ok::<_, std::convert::Infallible>(http::Response::new(axum::body::Body::empty()))
        }));

        let req = http::Request::builder()
            .method("POST")
            .uri("/submit")
            .body(axum::body::Body::from("<script>alert('xss')</script>"))
            .unwrap();
        let result = svc.oneshot(req).await;
        assert!(matches!(result, Err(SecurityError::AttackBlocked(_))));
    }

    #[tokio::test]
    async fn body_over_limit_maps_to_413() {
        use tower::Layer as _;
        use tower::ServiceExt;

        let layer = SecurityBodyLayer::new().body_limit(8);
        let svc = layer.layer(tower::service_fn(|_: Request<axum::body::Body>| async {
            Ok::<_, std::convert::Infallible>(http::Response::new(axum::body::Body::empty()))
        }));

        let req = http::Request::builder()
            .method("POST")
            .uri("/submit")
            .body(axum::body::Body::from("x".repeat(64)))
            .unwrap();
        let result = svc.oneshot(req).await;
        assert!(matches!(result, Err(SecurityError::BodyTooLarge)));
        assert_eq!(
            SecurityError::BodyTooLarge.to_http_status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[tokio::test]
    async fn inner_error_response_body_does_not_leak_detail() {
        use axum::response::IntoResponse;
        let resp = SecurityError::Inner(Box::new(std::io::Error::other(
            "secret-db-dsn=s3://user:pass",
        )))
        .into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let (_, body) = resp.into_parts();
        let bytes = axum::body::to_bytes(body, 4096).await.unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains("secret-db-dsn"), "leaked detail: {text}");
        assert!(text.contains("internal server error"), "got: {text}");
    }

    #[tokio::test]
    async fn attack_blocked_response_body_shape() {
        use axum::response::IntoResponse;
        let resp = SecurityError::AttackBlocked("sqli, xss".into()).into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let (_, body) = resp.into_parts();
        let bytes = axum::body::to_bytes(body, 1024).await.unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert_eq!(
            text,
            r#"{"error":"attack blocked","types":"sqli, xss"}"#.to_string()
        );
    }

    #[tokio::test]
    async fn body_too_large_response_body_shape() {
        use axum::response::IntoResponse;
        let resp = SecurityError::BodyTooLarge.into_response();
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let (_, body) = resp.into_parts();
        let bytes = axum::body::to_bytes(body, 1024).await.unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert_eq!(text, r#"{"error":"request body too large"}"#.to_string());
    }

    #[test]
    fn request_parts_include_uri_and_headers() {
        let req = http::Request::builder()
            .uri("/search?q=abc")
            .header("X-Custom", "val-1")
            .body(())
            .unwrap();
        let parts = request_parts(&req);
        assert_eq!(
            parts,
            vec!["/search?q=abc".to_string(), "val-1".to_string()]
        );
    }

    #[test]
    fn percent_decode_handles_encoded_sql_chars() {
        assert_eq!(
            percent_decode("/q?x=SELECT%20*%20FROM%20users"),
            "/q?x=SELECT * FROM users"
        );
        assert_eq!(percent_decode("1%27%20OR%20%271%27%3D%271"), "1' OR '1'='1");
        assert_eq!(
            percent_decode("/clean?q=hello%20world"),
            "/clean?q=hello world"
        );
        assert_eq!(percent_decode("%3cscript%3e"), "<script>");
        // 无效 % 序列原样保留
        assert_eq!(percent_decode("/%zz%"), "/%zz%");
        assert_eq!(percent_decode("/100%25"), "/100%");
    }

    #[test]
    fn request_parts_percent_decode_uri_only() {
        let req = http::Request::builder()
            .uri("/search?q=SELECT%20*%20FROM%20users")
            .header("X-Custom", "val%201")
            .body(())
            .unwrap();
        let parts = request_parts(&req);
        // URI 解码后进入扫描列表；header 原样保留（header 无编码层）
        assert_eq!(parts[0], "/search?q=SELECT * FROM users");
        assert_eq!(parts[1], "val%201");
    }

    #[tokio::test]
    async fn header_layer_blocks_attack_in_uri() {
        use tower::Layer as _;
        use tower::ServiceExt;

        let layer = SecurityLayer::new();
        let svc = layer.layer(tower::service_fn(|_: Request<()>| async {
            Ok::<_, std::convert::Infallible>(http::Response::new(axum::body::Body::empty()))
        }));

        // URI 不允许空格，SQLi 正则需字面空白 → 用 URI 合法字符即可命中的
        // javascript: XSS 载荷
        let req = http::Request::builder()
            .uri("/redirect?url=javascript:alert(1)")
            .body(())
            .unwrap();
        let resp = svc.oneshot(req).await.expect("blocked as response");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn header_layer_blocks_attack_in_header() {
        use tower::Layer as _;
        use tower::ServiceExt;

        let layer = SecurityLayer::new();
        let svc = layer.layer(tower::service_fn(|_: Request<()>| async {
            Ok::<_, std::convert::Infallible>(http::Response::new(axum::body::Body::empty()))
        }));

        let req = http::Request::builder()
            .uri("/clean")
            .header("X-Trace", "<script>alert(1)</script>")
            .body(())
            .unwrap();
        let resp = svc.oneshot(req).await.expect("blocked as response");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn header_layer_blocks_encoded_sqli_in_uri() {
        use tower::Layer as _;
        use tower::ServiceExt;

        let layer = SecurityLayer::new();
        let svc = layer.layer(tower::service_fn(|_: Request<()>| async {
            Ok::<_, std::convert::Infallible>(http::Response::new(axum::body::Body::empty()))
        }));

        // %20 编码空格：解码后 `SELECT * FROM users` 命中 SQLi 正则
        let req = http::Request::builder()
            .uri("/search?q=SELECT%20*%20FROM%20users")
            .body(())
            .unwrap();
        let resp = svc.oneshot(req).await.expect("blocked as response");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn header_layer_blocks_encoded_single_quote_sqli_in_uri() {
        use tower::Layer as _;
        use tower::ServiceExt;

        let layer = SecurityLayer::new();
        let svc = layer.layer(tower::service_fn(|_: Request<()>| async {
            Ok::<_, std::convert::Infallible>(http::Response::new(axum::body::Body::empty()))
        }));

        // %27 编码单引号 + %20 编码空格：解码后 `1' OR '1'='1` 命中 SQLi 正则
        let req = http::Request::builder()
            .uri("/login?u=1%27%20OR%20%271%27%3D%271")
            .body(())
            .unwrap();
        let resp = svc.oneshot(req).await.expect("blocked as response");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn header_layer_passes_clean_encoded_query_through() {
        use tower::Layer as _;
        use tower::ServiceExt;

        let layer = SecurityLayer::new();
        let svc = layer.layer(tower::service_fn(|req: Request<()>| async move {
            Ok::<_, std::convert::Infallible>(http::Response::new(axum::body::Body::from(
                req.uri().path().to_string(),
            )))
        }));

        // 正常请求含 %20 编码空格不应误拦截
        let req = http::Request::builder()
            .uri("/search?q=hello%20world")
            .body(())
            .unwrap();
        let resp = svc.oneshot(req).await.expect("clean request passes");
        let (_, body) = resp.into_parts();
        let bytes = axum::body::to_bytes(body, 1024).await.unwrap();
        assert_eq!(String::from_utf8_lossy(&bytes), "/search");
    }

    #[tokio::test]
    async fn header_layer_passes_proxy_headers_with_internal_ip() {
        use tower::Layer as _;
        use tower::ServiceExt;

        let layer = SecurityLayer::new();
        let svc = layer.layer(tower::service_fn(|req: Request<()>| async move {
            Ok::<_, std::convert::Infallible>(http::Response::new(axum::body::Body::from(
                req.uri().path().to_string(),
            )))
        }));

        // nginx 网关注入的代理头携带 docker 内网 IP，不应触发 SSRF 拦截
        let req = http::Request::builder()
            .uri("/api/v1/booking/dates?region_id=1")
            .header("X-Real-IP", "172.19.0.1")
            .header("X-Forwarded-For", "172.19.0.1")
            .body(())
            .unwrap();
        let resp = svc.oneshot(req).await.expect("proxy headers pass");
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn header_layer_passes_clean_request_through() {
        use tower::Layer as _;
        use tower::ServiceExt;

        let layer = SecurityLayer::new();
        let svc = layer.layer(tower::service_fn(|req: Request<()>| async move {
            Ok::<_, std::convert::Infallible>(http::Response::new(axum::body::Body::from(
                req.uri().path().to_string(),
            )))
        }));

        let req = http::Request::builder().uri("/clean").body(()).unwrap();
        let resp = svc.oneshot(req).await.expect("clean request passes");
        let (_, body) = resp.into_parts();
        let bytes = axum::body::to_bytes(body, 1024).await.unwrap();
        assert_eq!(String::from_utf8_lossy(&bytes), "/clean");
    }

    #[tokio::test]
    async fn body_layer_passes_clean_body_through() {
        use tower::Layer as _;
        use tower::ServiceExt;

        let layer = SecurityBodyLayer::new();
        let svc = layer.layer(tower::service_fn(
            |req: Request<axum::body::Body>| async move {
                let (_, body) = req.into_parts();
                let bytes = axum::body::to_bytes(body, 1024).await.unwrap();
                Ok::<_, std::convert::Infallible>(http::Response::new(axum::body::Body::from(
                    bytes,
                )))
            },
        ));

        let req = http::Request::builder()
            .method("POST")
            .uri("/submit")
            .body(axum::body::Body::from("hello world"))
            .unwrap();
        let resp = svc.oneshot(req).await.expect("clean body passes through");
        let (_, body) = resp.into_parts();
        let bytes = axum::body::to_bytes(body, 1024).await.unwrap();
        assert_eq!(&bytes[..], b"hello world");
    }
}
