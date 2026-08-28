// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use super::claims::AuthClaims;
use super::helpers::{error_response, extract_header, extract_query_param};
use http::{Request, Response, StatusCode};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Clone)]
pub struct ApiKeyLayer {
    keys: Arc<HashMap<String, AuthClaims>>,
    header_name: String,
    query_param: Option<String>,
}

impl ApiKeyLayer {
    pub fn new(keys: HashMap<String, AuthClaims>) -> Self {
        Self {
            keys: Arc::new(keys),
            header_name: "X-API-Key".into(),
            query_param: None,
        }
    }

    pub fn header_name(mut self, name: impl Into<String>) -> Self {
        self.header_name = name.into();
        self
    }

    /// Allow the API key in a query parameter as a fallback when it is not
    /// present in the header.
    ///
    /// # Security risk
    /// Keys passed in query strings can leak through access logs, browser
    /// history, and `Referer` headers, and may be cached by intermediaries.
    /// Prefer the header only; this fallback exists for legacy clients and
    /// logs a warning whenever it is actually used.
    pub fn query_param(mut self, param: impl Into<String>) -> Self {
        self.query_param = Some(param.into());
        self
    }
}

impl<S> Layer<S> for ApiKeyLayer {
    type Service = ApiKeyService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ApiKeyService {
            inner,
            config: Arc::new(self.clone()),
        }
    }
}

#[derive(Clone)]
pub struct ApiKeyService<S> {
    inner: S,
    config: Arc<ApiKeyLayer>,
}

impl<S, B> Service<Request<B>> for ApiKeyService<S>
where
    S: Service<Request<B>, Response = Response<axum::body::Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: Send + 'static,
{
    type Response = Response<axum::body::Body>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(|e| Box::new(e) as _)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let header_key = extract_header(req.headers(), &self.config.header_name);
        let key = match header_key {
            Some(k) => Some(k),
            None => {
                let query_key = self
                    .config
                    .query_param
                    .as_ref()
                    .and_then(|p| extract_query_param(req.uri().query(), p));
                if query_key.is_some() {
                    tracing::warn!(
                        param = ?self.config.query_param,
                        "api key accepted via query parameter; keys in URLs can leak \
                         through logs, history, and Referer headers"
                    );
                }
                query_key
            }
        };

        let claims = key.and_then(|k| self.config.keys.get(&k).cloned());
        let mut inner = self.inner.clone();

        Box::pin(async move {
            match claims {
                Some(c) => {
                    let mut req = req;
                    req.extensions_mut().insert(c);
                    inner.call(req).await.map_err(|e| Box::new(e) as _)
                }
                None => Ok(error_response(
                    StatusCode::UNAUTHORIZED,
                    r#"{"error":"invalid api key"}"#,
                )),
            }
        })
    }
}
