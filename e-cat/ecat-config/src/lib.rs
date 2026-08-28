// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
mod encrypted;
mod env;
mod file;

pub use encrypted::ObfuscatedSource;
pub use env::EnvSource;
pub use file::FileSource;

use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;

#[async_trait]
pub trait ConfigSource: Send + Sync {
    async fn load(&self) -> Result<HashMap<String, serde_json::Value>, ConfigError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config error: {0}")]
    Other(String),
}

#[derive(Default)]
pub struct Config {
    data: HashMap<String, serde_json::Value>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn load(&mut self, source: &dyn ConfigSource) -> Result<(), ConfigError> {
        let values = source.load().await?;
        self.data.extend(values);
        Ok(())
    }

    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.data.get(key).and_then(|v| T::deserialize(v).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_new_is_empty() {
        let c = Config::new();
        assert!(c.get::<serde_json::Value>("any").is_none());
    }

    #[test]
    fn config_default_is_empty() {
        let c = Config::default();
        assert!(c.get::<serde_json::Value>("any").is_none());
    }

    #[tokio::test]
    async fn config_load_from_source() {
        struct TestSource;
        #[async_trait::async_trait]
        impl ConfigSource for TestSource {
            async fn load(&self) -> Result<HashMap<String, serde_json::Value>, ConfigError> {
                let mut m = HashMap::new();
                m.insert("key".into(), serde_json::Value::String("val".into()));
                Ok(m)
            }
        }

        let mut c = Config::new();
        c.load(&TestSource).await.unwrap();
        assert_eq!(c.get::<String>("key"), Some("val".into()));
    }

    #[test]
    fn config_get_typed() {
        let mut c = Config::new();
        // Manually insert to test typed retrieval
        c.data
            .insert("num".into(), serde_json::Value::Number(42.into()));
        c.data
            .insert("s".into(), serde_json::Value::String("hello".into()));

        assert_eq!(c.get::<i32>("num"), Some(42));
        assert_eq!(c.get::<String>("s"), Some("hello".into()));
        assert!(c.get::<i32>("s").is_none()); // type mismatch
    }

    #[tokio::test]
    async fn config_load_merges_and_later_source_overrides() {
        struct Src(serde_json::Value);
        #[async_trait::async_trait]
        impl ConfigSource for Src {
            async fn load(&self) -> Result<HashMap<String, serde_json::Value>, ConfigError> {
                Ok(self
                    .0
                    .as_object()
                    .expect("object")
                    .clone()
                    .into_iter()
                    .collect())
            }
        }

        let mut c = Config::new();
        c.load(&Src(serde_json::json!({"a": 1, "b": 1})))
            .await
            .unwrap();
        c.load(&Src(serde_json::json!({"b": 2, "c": 3})))
            .await
            .unwrap();
        assert_eq!(c.get::<i32>("a"), Some(1));
        assert_eq!(c.get::<i32>("b"), Some(2), "later source overrides");
        assert_eq!(c.get::<i32>("c"), Some(3));
    }
}
