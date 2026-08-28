// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_errors::Error;

/// Document-store client abstraction (e.g. MongoDB).
#[async_trait]
pub trait DocumentClient: Send + Sync {
    /// Insert a document into `collection`, returning its id.
    async fn insert(&self, collection: &str, doc: &serde_json::Value) -> Result<String, Error>;

    /// Find documents matching `filter` (backend-specific JSON query syntax).
    async fn find(
        &self,
        collection: &str,
        filter: &serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, Error>;

    /// Update documents matching `filter`; returns the number of modified documents.
    async fn update(
        &self,
        collection: &str,
        filter: &serde_json::Value,
        update: &serde_json::Value,
    ) -> Result<u64, Error>;

    /// Delete documents matching `filter`; returns the number of deleted documents.
    async fn delete(&self, collection: &str, filter: &serde_json::Value) -> Result<u64, Error>;
}
