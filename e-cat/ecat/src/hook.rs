// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;

#[async_trait]
pub trait LifecycleHook: Send + Sync {
    async fn on_start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
    async fn on_stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

#[async_trait]
impl<F, Fut> LifecycleHook for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send,
{
    async fn on_start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        (self)().await
    }

    async fn on_stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        (self)().await
    }
}
