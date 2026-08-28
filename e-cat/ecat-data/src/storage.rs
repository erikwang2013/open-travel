// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_errors::Error;

/// Object-storage client abstraction (e.g. S3, MinIO).
#[async_trait]
pub trait StorageClient: Send + Sync {
    /// Upload `data` under `key` in `bucket`.
    async fn put(&self, bucket: &str, key: &str, data: &[u8]) -> Result<(), Error>;

    /// Download the object at `key` in `bucket`.
    async fn get(&self, bucket: &str, key: &str) -> Result<Vec<u8>, Error>;

    /// Delete the object at `key` in `bucket`.
    async fn delete(&self, bucket: &str, key: &str) -> Result<(), Error>;

    /// List object keys in `bucket` under `prefix`.
    async fn list(&self, bucket: &str, prefix: &str) -> Result<Vec<String>, Error>;
}
