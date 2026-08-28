// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_errors::{Error, ErrorCode};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum FieldValue {
    Float(f64),
    Int(i64),
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct DataPoint {
    pub measurement: String,
    pub tags: HashMap<String, String>,
    pub fields: HashMap<String, FieldValue>,
    pub timestamp: Option<i64>,
}

impl DataPoint {
    pub fn new(measurement: impl Into<String>) -> Self {
        Self {
            measurement: measurement.into(),
            tags: HashMap::new(),
            fields: HashMap::new(),
            timestamp: None,
        }
    }

    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: FieldValue) -> Self {
        self.fields.insert(key.into(), value);
        self
    }

    pub fn with_timestamp(mut self, ts: i64) -> Self {
        self.timestamp = Some(ts);
        self
    }
}

#[async_trait]
pub trait TsdbClient: Send + Sync {
    async fn write(&self, points: &[DataPoint]) -> Result<(), Error>;
    async fn query(&self, query: &str) -> Result<serde_json::Value, Error>;

    /// Delete data using a backend-specific query (e.g. `DELETE FROM ...`).
    /// Backends that cannot delete return an error.
    async fn delete(&self, _query: &str) -> Result<(), Error> {
        Err(Error::new(
            ErrorCode::Internal,
            "tsdb",
            "delete not supported by this backend",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datapoint_builder_accumulates_metadata() {
        let dp = DataPoint::new("cpu")
            .with_tag("host", "h1")
            .with_tag("env", "prod")
            .with_field("usage", FieldValue::Float(0.85))
            .with_field("count", FieldValue::Int(3))
            .with_field("active", FieldValue::Bool(true))
            .with_field("name", FieldValue::String("web".into()))
            .with_timestamp(1_700_000_000_000);
        assert_eq!(dp.measurement, "cpu");
        assert_eq!(dp.tags.len(), 2);
        assert_eq!(dp.tags.get("host").map(String::as_str), Some("h1"));
        assert_eq!(dp.fields.len(), 4);
        assert!(
            matches!(dp.fields.get("usage"), Some(FieldValue::Float(v)) if (v - 0.85).abs() < 1e-9)
        );
        assert!(matches!(dp.fields.get("count"), Some(FieldValue::Int(3))));
        assert!(matches!(
            dp.fields.get("active"),
            Some(FieldValue::Bool(true))
        ));
        assert!(matches!(
            dp.fields.get("name"),
            Some(FieldValue::String(s)) if s == "web"
        ));
        assert_eq!(dp.timestamp, Some(1_700_000_000_000));
    }

    #[test]
    fn datapoint_defaults_are_empty() {
        let dp = DataPoint::new("mem");
        assert!(dp.tags.is_empty());
        assert!(dp.fields.is_empty());
        assert_eq!(dp.timestamp, None);
    }

    #[test]
    fn datapoint_overwrites_existing_tag() {
        let dp = DataPoint::new("cpu")
            .with_tag("host", "a")
            .with_tag("host", "b");
        assert_eq!(dp.tags.get("host").map(String::as_str), Some("b"));
        assert_eq!(dp.tags.len(), 1);
    }

    struct NoDeleteClient;

    #[async_trait]
    impl TsdbClient for NoDeleteClient {
        async fn write(&self, _points: &[DataPoint]) -> Result<(), Error> {
            Ok(())
        }
        async fn query(&self, _query: &str) -> Result<serde_json::Value, Error> {
            Ok(serde_json::Value::Null)
        }
    }

    #[tokio::test]
    async fn delete_defaults_to_not_supported_error() {
        let client = NoDeleteClient;
        let err = client.delete("DROP TABLE t").await.unwrap_err();
        assert!(
            err.to_string().contains("delete not supported"),
            "unexpected error: {err}"
        );
    }
}
