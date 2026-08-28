// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::response::IntoResponse;
use tower::{Layer, Service};

/// 请求校验错误：携带 HTTP 状态码与错误消息。
#[derive(Debug, Clone)]
pub struct ValidateError {
    status: http::StatusCode,
    message: String,
}

impl ValidateError {
    /// 默认 400 Bad Request。
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            status: http::StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub fn with_status(status: http::StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn status(&self) -> http::StatusCode {
        self.status
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// 请求校验规则：对请求头/方法/路径等元数据做校验（**不消费 body**）。
///
/// body 级校验（JSON schema、字段约束等）需要读取 body 流，属于 handler 或
/// axum extractor 的职责——本中间件仅校验请求元数据，保持 body 原样透传。
/// 示例：`axum::extract::Json<CreateUser>` extractor 的 `Rejection` 即可完成
/// body 结构校验。
pub trait RequestValidator<B> {
    fn validate(&self, req: &http::Request<B>) -> Result<(), ValidateError>;
}

/// 闭包入口包装：`ValidateLayer::from_fn` 创建的 validator。
pub struct FnValidator<B, F> {
    f: F,
    _marker: PhantomData<fn(&B)>,
}

// 手动 impl：derive 会隐含 B: Clone（挂 axum Router 时 B = Body 不可 Clone），
// 而 PhantomData<fn(&B)> 的 Clone 与 B 无关。
impl<B, F: Clone> Clone for FnValidator<B, F> {
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            _marker: PhantomData,
        }
    }
}

impl<B, F> RequestValidator<B> for FnValidator<B, F>
where
    F: Fn(&http::Request<B>) -> Result<(), ValidateError>,
{
    fn validate(&self, req: &http::Request<B>) -> Result<(), ValidateError> {
        (self.f)(req)
    }
}

/// Validate Layer：请求校验，失败短路返回 `{"error": message}` JSON 响应。
#[derive(Clone)]
pub struct ValidateLayer<V> {
    validator: V,
}

impl<V> ValidateLayer<V> {
    pub fn new(validator: V) -> Self {
        Self { validator }
    }
}

impl<B, F> ValidateLayer<FnValidator<B, F>> {
    /// 以闭包方式构造校验规则（免自定义 struct 实现 trait）。
    pub fn from_fn(f: F) -> Self
    where
        F: Fn(&http::Request<B>) -> Result<(), ValidateError> + Clone,
    {
        Self::new(FnValidator {
            f,
            _marker: PhantomData,
        })
    }
}

#[derive(Clone)]
pub struct ValidateService<V, S> {
    inner: S,
    validator: V,
}

impl<S, V> Layer<S> for ValidateLayer<V>
where
    V: Clone,
{
    type Service = ValidateService<V, S>;

    fn layer(&self, inner: S) -> Self::Service {
        ValidateService {
            inner,
            validator: self.validator.clone(),
        }
    }
}

impl<S, B, V> Service<http::Request<B>> for ValidateService<V, S>
where
    V: RequestValidator<B>,
    S: Service<http::Request<B>> + Send + 'static,
    S::Response: Into<axum::response::Response>,
    S::Future: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: Send + 'static,
{
    type Response = axum::response::Response;
    // 透传 inner 错误（M1 同款）：挂 axum Router 时 Error 自动为
    // Infallible（Router::layer 要求 Error: Into<Infallible>）。
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<B>) -> Self::Future {
        match self.validator.validate(&req) {
            Ok(()) => {
                let fut = self.inner.call(req);
                Box::pin(async move {
                    let resp = fut.await?;
                    Ok(resp.into())
                })
            }
            Err(e) => {
                let status = e.status;
                let message = e.message;
                Box::pin(async move {
                    Ok(
                        (status, axum::Json(serde_json::json!({ "error": message })))
                            .into_response(),
                    )
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tower::ServiceExt;

    #[tokio::test]
    async fn passes_through_valid_requests() {
        let calls = Arc::new(AtomicU32::new(0));
        let svc =
            ValidateLayer::from_fn(|_req: &http::Request<()>| Ok(())).layer(tower::service_fn({
                let calls = Arc::clone(&calls);
                move |_req: http::Request<()>| {
                    let calls = Arc::clone(&calls);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Ok::<_, std::convert::Infallible>(axum::response::Response::new(
                            axum::body::Body::empty(),
                        ))
                    }
                }
            }));
        let resp = svc
            .oneshot(http::Request::builder().body(()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(calls.load(Ordering::SeqCst), 1, "inner must be called once");
    }

    #[tokio::test]
    async fn rejects_with_default_400_and_json_body() {
        let svc = ValidateLayer::from_fn(|req: &http::Request<()>| {
            if req.method() == http::Method::GET {
                Ok(())
            } else {
                Err(ValidateError::new("method not allowed"))
            }
        })
        .layer(tower::service_fn(|_req: http::Request<()>| async move {
            Ok::<_, std::convert::Infallible>(axum::response::Response::new(
                axum::body::Body::empty(),
            ))
        }));
        let resp = svc
            .oneshot(
                http::Request::builder()
                    .method(http::Method::POST)
                    .body(())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        assert!(
            String::from_utf8_lossy(&body).contains("method not allowed"),
            "json body must carry the error message, got: {}",
            String::from_utf8_lossy(&body)
        );
    }

    #[tokio::test]
    async fn rejects_with_custom_status() {
        let svc = ValidateLayer::from_fn(|_req: &http::Request<()>| {
            Err(ValidateError::with_status(
                http::StatusCode::UNPROCESSABLE_ENTITY,
                "unprocessable",
            ))
        })
        .layer(tower::service_fn(|_req: http::Request<()>| async move {
            Ok::<_, std::convert::Infallible>(axum::response::Response::new(
                axum::body::Body::empty(),
            ))
        }));
        let resp = svc
            .oneshot(http::Request::builder().body(()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), http::StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn from_fn_closure_entry_works() {
        let svc = ValidateLayer::from_fn(|req: &http::Request<()>| {
            if req.uri().path() == "/admin" {
                Err(ValidateError::new("admin area forbidden"))
            } else {
                Ok(())
            }
        })
        .layer(tower::service_fn(|_req: http::Request<()>| async move {
            Ok::<_, std::convert::Infallible>(axum::response::Response::new(
                axum::body::Body::empty(),
            ))
        }));
        let resp = svc
            .oneshot(http::Request::builder().uri("/admin").body(()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn does_not_call_inner_on_failure() {
        let calls = Arc::new(AtomicU32::new(0));
        let svc = ValidateLayer::from_fn(|_req: &http::Request<()>| {
            Err(ValidateError::new("always rejected"))
        })
        .layer(tower::service_fn({
            let calls = Arc::clone(&calls);
            move |_req: http::Request<()>| {
                let calls = Arc::clone(&calls);
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, std::convert::Infallible>(axum::response::Response::new(
                        axum::body::Body::empty(),
                    ))
                }
            }
        }));
        let resp = svc
            .oneshot(http::Request::builder().body(()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "inner must not run when validation fails"
        );
    }

    #[tokio::test]
    async fn mounts_on_axum_router() {
        // 覆盖两个 Router 集成约束：B = axum::body::Body（不可 Clone，
        // 验证 FnValidator 手动 Clone impl）与 Error: Into<Infallible>。
        let router = axum::Router::new()
            .route("/", axum::routing::get(|| async { "ok" }))
            .layer(ValidateLayer::from_fn(
                |req: &http::Request<axum::body::Body>| {
                    if req.uri().path() == "/admin" {
                        Err(ValidateError::new("admin area forbidden"))
                    } else {
                        Ok(())
                    }
                },
            ));

        let ok = router
            .clone()
            .oneshot(
                http::Request::builder()
                    .uri("/")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok.status(), http::StatusCode::OK);

        let rejected = router
            .oneshot(
                http::Request::builder()
                    .uri("/admin")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(rejected.status(), http::StatusCode::BAD_REQUEST);
    }
}
