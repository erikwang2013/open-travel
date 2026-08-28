// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_errors::Error;

#[async_trait]
pub trait GraphClient: Send + Sync {
    async fn execute(
        &self,
        query: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, Error>;
}
