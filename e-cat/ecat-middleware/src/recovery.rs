// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use std::future::Future;
use std::pin::Pin;
use tower::{Layer, Service};
use tracing::Instrument;

#[derive(Clone)]
pub struct RecoveryLayer;

impl<S> Layer<S> for RecoveryLayer {
    type Service = RecoveryService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        RecoveryService { inner }
    }
}

#[derive(Clone)]
pub struct RecoveryService<S> {
    inner: S,
}

impl<S, Req> Service<Req> for RecoveryService<S>
where
    S: Service<Req> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    Req: Send + 'static,
{
    type Response = S::Response;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(|e| Box::new(e) as _)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let span = tracing::Span::current();
        let fut = self.inner.call(req);
        Box::pin(async move {
            match tokio::task::spawn(fut.instrument(span)).await {
                Ok(Ok(response)) => Ok(response),
                Ok(Err(e)) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
                Err(_) => Err(Box::new(std::io::Error::other("task panicked"))
                    as Box<dyn std::error::Error + Send + Sync>),
            }
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
        let _layer = RecoveryLayer;
    }

    #[test]
    fn layer_wraps_service() {
        let layer = RecoveryLayer;
        let _svc = layer.layer(EchoService);
    }

    #[tokio::test]
    async fn calls_inner_service() {
        let layer = RecoveryLayer;
        let mut svc = layer.layer(EchoService);
        let result = svc.call("hello".into()).await.unwrap();
        assert_eq!(result, "hello");
    }

    /// 内层任务 panic 必须转换为 Err("task panicked")，不能向调用方传播 panic。
    #[tokio::test]
    async fn inner_panic_becomes_error() {
        struct PanicService;
        impl Service<()> for PanicService {
            type Response = ();
            type Error = std::io::Error;
            type Future = Pin<Box<dyn Future<Output = Result<(), std::io::Error>> + Send>>;
            fn poll_ready(
                &mut self,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<(), Self::Error>> {
                std::task::Poll::Ready(Ok(()))
            }
            fn call(&mut self, _req: ()) -> Self::Future {
                Box::pin(async { panic!("boom") })
            }
        }

        let layer = RecoveryLayer;
        let mut svc = layer.layer(PanicService);
        let err = svc.call(()).await.unwrap_err();
        assert!(err.to_string().contains("task panicked"), "got: {err}");
    }
}
