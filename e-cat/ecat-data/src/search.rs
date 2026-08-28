// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_errors::{Error, ErrorCode};

#[async_trait]
pub trait SearchClient: Send + Sync {
    async fn index(&self, index: &str, id: &str, doc: &serde_json::Value) -> Result<(), Error>;
    async fn search(
        &self,
        index: &str,
        query: &serde_json::Value,
    ) -> Result<serde_json::Value, Error>;
    async fn delete(&self, index: &str, id: &str) -> Result<(), Error>;

    /// Bulk index documents as `(id, doc)` pairs in one round trip.
    /// Backends that cannot bulk-index return an error.
    async fn bulk_index(
        &self,
        _index: &str,
        _docs: &[(String, serde_json::Value)],
    ) -> Result<(), Error> {
        Err(Error::new(
            ErrorCode::Internal,
            "search",
            "bulk_index not supported by this backend",
        ))
    }

    /// Update an existing document, replacing it with `doc`.
    async fn update(&self, _index: &str, _id: &str, _doc: &serde_json::Value) -> Result<(), Error> {
        Err(Error::new(
            ErrorCode::Internal,
            "search",
            "update not supported by this backend",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 只实现核心三操作的后端：可选操作必须走默认实现的报错路径。
    struct MinimalSearch;

    #[async_trait]
    impl SearchClient for MinimalSearch {
        async fn index(
            &self,
            _index: &str,
            _id: &str,
            _doc: &serde_json::Value,
        ) -> Result<(), Error> {
            Ok(())
        }
        async fn search(
            &self,
            _index: &str,
            _query: &serde_json::Value,
        ) -> Result<serde_json::Value, Error> {
            Ok(serde_json::Value::Null)
        }
        async fn delete(&self, _index: &str, _id: &str) -> Result<(), Error> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn optional_ops_default_to_not_supported_error() {
        let client = MinimalSearch;
        let err = client.bulk_index("idx", &[]).await.unwrap_err();
        assert!(
            err.to_string().contains("bulk_index not supported"),
            "got: {err}"
        );
        let err = client
            .update("idx", "1", &serde_json::Value::Null)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("update not supported"),
            "got: {err}"
        );
    }
}
