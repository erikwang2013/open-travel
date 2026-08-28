// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_data::Cache;
use ecat_errors::Error;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 内嵌 JSON 配置：示例用 JSON 声明数据访问层参数，不实际连接外部数据库。
const CONFIG_JSON: &str = r#"{
  "backend": "cache",
  "host": "127.0.0.1",
  "port": 6379,
  "ttl_seconds": 60
}"#;

#[derive(Debug, Deserialize)]
struct DataConfig {
    backend: String,
    host: String,
    port: u16,
    ttl_seconds: u64,
}

/// 最小内存 Cache 实现：演示 ecat-data 的 Cache trait 用法。
/// 生产场景按配置选择对应后端（如 ecat-data-redis 的 RedisCache）。
#[derive(Clone, Default)]
struct InMemoryCache {
    store: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

#[async_trait]
impl Cache for InMemoryCache {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        Ok(self.store.lock().unwrap().get(key).cloned())
    }

    async fn set(&self, key: &str, value: &[u8], _ttl: Duration) -> Result<(), Error> {
        self.store
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_vec());
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), Error> {
        self.store.lock().unwrap().remove(key);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cfg: DataConfig = serde_json::from_str(CONFIG_JSON)?;
    println!(
        "databases example: backend={} at {}:{} (ttl {}s)",
        cfg.backend, cfg.host, cfg.port, cfg.ttl_seconds
    );

    let cache = InMemoryCache::default();
    cache
        .set(
            "greeting",
            b"hello from e-cat",
            Duration::from_secs(cfg.ttl_seconds),
        )
        .await?;
    let got = cache.get("greeting").await?;
    println!(
        "cache.get -> {:?}",
        got.map(|v| String::from_utf8_lossy(&v).into_owned())
    );
    cache.delete("greeting").await?;
    println!("after delete -> {:?}", cache.get("greeting").await?);
    Ok(())
}
