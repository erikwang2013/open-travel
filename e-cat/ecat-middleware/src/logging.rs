// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use tower::{Layer, Service};

#[derive(Clone)]
pub struct LoggingLayer;

impl<S> Layer<S> for LoggingLayer {
    type Service = LoggingService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        LoggingService { inner }
    }
}

#[derive(Clone)]
pub struct LoggingService<S> {
    inner: S,
}

impl<S, Req> Service<Req> for LoggingService<S>
where
    S: Service<Req> + Send + 'static,
    S::Future: Send + 'static,
    Req: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let start = Instant::now();
        let fut = self.inner.call(req);
        Box::pin(async move {
            let result = fut.await;
            let elapsed = start.elapsed();
            tracing::info!(
                duration_ms = elapsed.as_millis() as u64,
                "request completed"
            );
            result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::Service;

    #[derive(Clone)]
    struct EchoService;

    impl Service<String> for EchoService {
        type Response = String;
        type Error = std::io::Error;
        type Future = Pin<Box<dyn Future<Output = Result<String, std::io::Error>> + Send>>;

        fn poll_ready(
            &mut self,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            std::task::Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: String) -> Self::Future {
            Box::pin(async move { Ok(req) })
        }
    }

    #[test]
    fn layer_constructs() {
        let _layer = LoggingLayer;
    }

    #[test]
    fn layer_wraps_service() {
        let layer = LoggingLayer;
        let _svc = layer.layer(EchoService);
    }

    #[tokio::test]
    async fn calls_inner_service() {
        let layer = LoggingLayer;
        let mut svc = layer.layer(EchoService);
        let result = svc.call("hello".into()).await.unwrap();
        assert_eq!(result, "hello");
    }
}
