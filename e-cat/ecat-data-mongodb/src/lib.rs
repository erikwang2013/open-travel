// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_data::DocumentClient;
use ecat_errors::{Error, ErrorCode};
use ecat_tls::TlsClientConfig;
use futures_util::TryStreamExt;
use mongodb::bson;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct MongoConfig {
    pub url: String,
    pub database: String,
    #[serde(default)]
    pub tls: Option<TlsClientConfig>,
}

pub struct MongoClient {
    client: mongodb::Client,
    database: String,
}

impl MongoClient {
    pub async fn from_config(cfg: MongoConfig) -> Result<Self, Error> {
        let client = mongodb::Client::with_uri_str(&cfg.url).await.map_err(|e| {
            Error::new(
                ErrorCode::Internal,
                "mongodb",
                format!("mongodb connect: {e}"),
            )
        })?;
        Ok(Self {
            client,
            database: cfg.database,
        })
    }
}

#[async_trait]
impl DocumentClient for MongoClient {
    async fn insert(&self, collection: &str, doc: &Value) -> Result<String, Error> {
        let doc = bson::to_document(doc).map_err(|e| {
            Error::new(ErrorCode::Internal, "mongodb", format!("mongodb bson: {e}"))
        })?;
        let result = self
            .client
            .database(&self.database)
            .collection::<bson::Document>(collection)
            .insert_one(doc)
            .await
            .map_err(|e| {
                Error::new(
                    ErrorCode::Internal,
                    "mongodb",
                    format!("mongodb insert: {e}"),
                )
            })?;
        Ok(result.inserted_id.to_string())
    }

    async fn find(&self, collection: &str, filter: &Value) -> Result<Vec<Value>, Error> {
        let filter = bson::to_document(filter).map_err(|e| {
            Error::new(ErrorCode::Internal, "mongodb", format!("mongodb bson: {e}"))
        })?;
        let cursor = self
            .client
            .database(&self.database)
            .collection::<bson::Document>(collection)
            .find(filter)
            .await
            .map_err(|e| {
                Error::new(ErrorCode::Internal, "mongodb", format!("mongodb find: {e}"))
            })?;
        let docs: Vec<bson::Document> = cursor.try_collect().await.map_err(|e| {
            Error::new(ErrorCode::Internal, "mongodb", format!("mongodb find: {e}"))
        })?;
        docs.iter()
            .map(|d| {
                serde_json::to_value(d).map_err(|e| {
                    Error::new(ErrorCode::Internal, "mongodb", format!("mongodb json: {e}"))
                })
            })
            .collect()
    }

    async fn update(&self, collection: &str, filter: &Value, update: &Value) -> Result<u64, Error> {
        let filter = bson::to_document(filter).map_err(|e| {
            Error::new(ErrorCode::Internal, "mongodb", format!("mongodb bson: {e}"))
        })?;
        let update = bson::to_document(update).map_err(|e| {
            Error::new(ErrorCode::Internal, "mongodb", format!("mongodb bson: {e}"))
        })?;
        let result = self
            .client
            .database(&self.database)
            .collection::<bson::Document>(collection)
            .update_many(filter, update)
            .await
            .map_err(|e| {
                Error::new(
                    ErrorCode::Internal,
                    "mongodb",
                    format!("mongodb update: {e}"),
                )
            })?;
        Ok(result.modified_count)
    }

    async fn delete(&self, collection: &str, filter: &Value) -> Result<u64, Error> {
        let filter = bson::to_document(filter).map_err(|e| {
            Error::new(ErrorCode::Internal, "mongodb", format!("mongodb bson: {e}"))
        })?;
        let result = self
            .client
            .database(&self.database)
            .collection::<bson::Document>(collection)
            .delete_many(filter)
            .await
            .map_err(|e| {
                Error::new(
                    ErrorCode::Internal,
                    "mongodb",
                    format!("mongodb delete: {e}"),
                )
            })?;
        Ok(result.deleted_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_deserializes() {
        let cfg: MongoConfig = serde_json::from_value(serde_json::json!({
            "url": "mongodb://localhost:27017",
            "database": "app",
        }))
        .unwrap();
        assert_eq!(cfg.database, "app");
    }

    #[tokio::test]
    async fn from_config_rejects_bad_uri() {
        let result = MongoClient::from_config(MongoConfig {
            url: "not-a-valid-uri".into(),
            database: "app".into(),
            tls: None,
        })
        .await;
        assert!(result.is_err());
    }

    #[test]
    fn config_missing_url_or_database_is_error() {
        assert!(serde_json::from_str::<MongoConfig>(r#"{"database":"app"}"#).is_err());
        assert!(
            serde_json::from_str::<MongoConfig>(r#"{"url":"mongodb://localhost:27017"}"#).is_err()
        );
    }

    /// serde_json i64 → bson i64 → serde_json 的全程保真：超过 f64 精确表示
    /// 范围的整数不得被 float 转换截断（如 ObjectId 式大 id / 时间戳）。
    #[test]
    fn bson_roundtrip_preserves_large_i64_precision() {
        let v: Value = serde_json::json!({"big": 9_007_199_254_740_993_i64});
        let doc = bson::to_document(&v).unwrap();
        let back: Value = serde_json::to_value(doc).unwrap();
        assert_eq!(back["big"], serde_json::json!(9_007_199_254_740_993_i64));
    }

    #[test]
    fn bson_roundtrip_preserves_nested_and_negative() {
        let v: Value = serde_json::json!({
            "neg": -42,
            "f": -0.25,
            "arr": [1, "two", null],
            "deep": {"a": {"b": {"c": true}}},
        });
        let doc = bson::to_document(&v).unwrap();
        let back: Value = serde_json::to_value(doc).unwrap();
        assert_eq!(v, back);
    }

    /// insert/find/update/delete 的公共输入路径：serde_json::Value →
    /// bson::Document 往返保真（嵌套、数组、各标量类型、null）。
    #[test]
    fn bson_roundtrip_preserves_json_object() {
        let v: Value = serde_json::json!({
            "name": "alice",
            "age": 30,
            "active": true,
            "score": 1.5,
            "tags": ["a", "b", "c"],
            "nested": {"level": 2, "nil": null},
            "none": null,
        });
        let doc = bson::to_document(&v).unwrap();
        let back: Value = serde_json::to_value(doc).unwrap();
        assert_eq!(v, back);
    }

    /// bson::to_document 要求顶层为文档：null / 数组等非文档值必须报错
    /// （否则 insert 会拿非法文档直达网络）。
    #[test]
    fn bson_to_document_rejects_non_document_top_level() {
        assert!(bson::to_document(&Value::Null).is_err());
        assert!(bson::to_document(&serde_json::json!([1, 2, 3])).is_err());
        assert!(bson::to_document(&Value::from("str")).is_err());
    }

    /// 错误路径先于网络访问：insert 的 bson 转换失败返回 Error，
    /// 不发起任何连接（url 指向不可达端口也无妨）。
    #[tokio::test]
    async fn insert_rejects_non_document_before_network() {
        let client = MongoClient::from_config(MongoConfig {
            url: "mongodb://127.0.0.1:1".into(),
            database: "app".into(),
            tls: None,
        })
        .await
        .unwrap();
        let err = client.insert("col", &Value::Null).await.unwrap_err();
        assert!(err.to_string().contains("mongodb bson:"), "got: {err}");
    }
}
