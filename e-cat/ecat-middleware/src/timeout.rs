// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tower::{Layer, Service};

#[derive(Clone)]
pub struct TimeoutLayer {
    timeout: Duration,
}

impl TimeoutLayer {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl<S> Layer<S> for TimeoutLayer {
    type Service = TimeoutService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        TimeoutService {
            inner,
            timeout: self.timeout,
        }
    }
}

#[derive(Clone)]
pub struct TimeoutService<S> {
    inner: S,
    timeout: Duration,
}

impl<S, Req> Service<Req> for TimeoutService<S>
where
    S: Service<Req> + Send + 'static,
    S::Future: Send + 'static,
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
        let fut = self.inner.call(req);
        let timeout = self.timeout;
        Box::pin(async move {
            tokio::time::timeout(timeout, fut)
                .await
                .map_err(|_| {
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "request timed out",
                    )) as Box<dyn std::error::Error + Send + Sync>
                })?
                .map_err(|e| Box::new(e) as _)
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
    fn layer_constructs_with_duration() {
        let layer = TimeoutLayer::new(Duration::from_secs(30));
        let _svc = layer.layer(EchoService);
    }

    #[test]
    fn layer_clones() {
        let layer = TimeoutLayer::new(Duration::from_millis(500));
        let _layer2 = layer.clone();
    }

    #[tokio::test]
    async fn calls_inner_service_within_timeout() {
        let layer = TimeoutLayer::new(Duration::from_secs(5));
        let mut svc = layer.layer(EchoService);
        let result = svc.call("hello".into()).await.unwrap();
        assert_eq!(result, "hello");
    }

    #[tokio::test]
    async fn times_out_slow_service() {
        #[derive(Clone)]
        struct SlowService;

        impl Service<String> for SlowService {
            type Response = String;
            type Error = std::io::Error;
            type Future = Pin<Box<dyn Future<Output = Result<String, std::io::Error>> + Send>>;

            fn poll_ready(
                &mut self,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<(), Self::Error>> {
                std::task::Poll::Ready(Ok(()))
            }

            fn call(&mut self, _req: String) -> Self::Future {
                Box::pin(async {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    Ok("done".into())
                })
            }
        }

        let layer = TimeoutLayer::new(Duration::from_millis(10));
        let mut svc = layer.layer(SlowService);
        let result = svc.call("hello".into()).await;
        assert!(result.is_err());
    }
}
