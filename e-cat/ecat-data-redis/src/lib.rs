// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_data::Cache;
use ecat_errors::{Error, ErrorCode};
use ecat_lock::{DistributedLock, LockError};
use ecat_tls::TlsClientConfig;
use redis::AsyncCommands;
use redis::ConnectionInfo;
use redis::aio::MultiplexedConnection;
use serde::Deserialize;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    #[serde(default)]
    pub password: Option<String>,
    /// TLS configuration. When enabled, uses `rediss://` scheme.
    /// Cert paths are for future TLS connection parameter support.
    #[serde(default)]
    pub tls: Option<TlsClientConfig>,
}

fn build_url(cfg: &RedisConfig) -> String {
    if cfg.tls.as_ref().is_some_and(|t| t.is_enabled()) {
        cfg.url.replacen("redis://", "rediss://", 1)
    } else {
        cfg.url.clone()
    }
}

pub struct RedisCache {
    conn: MultiplexedConnection,
}

impl RedisCache {
    pub async fn connect(url: &str) -> Result<Self, Error> {
        let client = redis::Client::open(url)
            .map_err(|e| Error::new(ErrorCode::Internal, "redis", format!("redis open: {e}")))?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "redis", format!("redis connect: {e}")))?;
        Ok(Self { conn })
    }

    pub async fn connect_with_password(url: &str, password: &str) -> Result<Self, Error> {
        // 通过 ConnectionInfo 单独传密码，避免密码嵌入 URL（否则错误消息会泄露口令）
        let mut info: ConnectionInfo = url
            .parse()
            .map_err(|e| Error::new(ErrorCode::Internal, "redis", format!("redis url: {e}")))?;
        info.redis.password = Some(password.to_string());
        let client = redis::Client::open(info)
            .map_err(|e| Error::new(ErrorCode::Internal, "redis", format!("redis open: {e}")))?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "redis", format!("redis connect: {e}")))?;
        Ok(Self { conn })
    }

    // Reconnection behavior: there is no explicit reconnect logic here.
    // The underlying redis::aio::MultiplexedConnection reconnects
    // internally on transient failures; a dropped connection is detected
    // on the next command, which will return an error.
    pub async fn from_config(cfg: RedisConfig) -> Result<Self, Error> {
        let url = build_url(&cfg);
        match &cfg.password {
            Some(pw) if !pw.is_empty() => Self::connect_with_password(&url, pw).await,
            _ => Self::connect(&url).await,
        }
    }

    pub fn from_connection(conn: MultiplexedConnection) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl Cache for RedisCache {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        let mut conn = self.conn.clone();
        conn.get(key)
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "redis", format!("redis get: {e}")))
    }

    async fn set(&self, key: &str, value: &[u8], ttl: Duration) -> Result<(), Error> {
        let mut conn = self.conn.clone();
        let millis = ttl.as_millis();
        if millis > 0 {
            let ms = if millis > u64::MAX as u128 {
                u64::MAX
            } else {
                millis as u64
            };
            let (): () = conn.pset_ex(key, value, ms).await.map_err(|e| {
                Error::new(ErrorCode::Internal, "redis", format!("redis psetex: {e}"))
            })?;
        } else {
            let (): () = conn
                .set(key, value)
                .await
                .map_err(|e| Error::new(ErrorCode::Internal, "redis", format!("redis set: {e}")))?;
        }
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), Error> {
        let mut conn = self.conn.clone();
        conn.del(key)
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "redis", format!("redis del: {e}")))
    }

    async fn increment(&self, key: &str, delta: i64) -> Result<i64, Error> {
        let mut conn = self.conn.clone();
        conn.incr(key, delta)
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "redis", format!("redis incr: {e}")))
    }

    async fn ttl(&self, key: &str) -> Result<Option<Duration>, Error> {
        let mut conn = self.conn.clone();
        let ttl: i64 = conn
            .ttl(key)
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "redis", format!("redis ttl: {e}")))?;
        Ok(ttl_to_duration(ttl))
    }

    async fn multi_get(&self, keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>, Error> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let mut conn = self.conn.clone();
        conn.mget(keys)
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "redis", format!("redis mget: {e}")))
    }
}

/// redis TTL 语义映射：-2 表示键不存在、-1 表示无过期时间，均映射为 None；
/// 其余为剩余秒数。
fn ttl_to_duration(ttl: i64) -> Option<Duration> {
    if ttl < 0 {
        None
    } else {
        Some(Duration::from_secs(ttl as u64))
    }
}

/// Distributed lock backed by Redis `SET NX PX`.
pub struct RedisLock {
    conn: MultiplexedConnection,
}

impl RedisLock {
    pub async fn connect(url: &str) -> Result<Self, LockError> {
        let client =
            redis::Client::open(url).map_err(|e| LockError::Other(format!("redis open: {e}")))?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| LockError::Other(format!("redis connect: {e}")))?;
        Ok(Self { conn })
    }

    pub async fn from_config(cfg: RedisConfig) -> Result<Self, LockError> {
        let url = build_url(&cfg);
        if let Some(pw) = cfg.password.as_ref().filter(|p| !p.is_empty()) {
            // 通过 ConnectionInfo 单独传密码，避免密码嵌入 URL 后泄露在错误消息中
            let mut info: ConnectionInfo = url
                .parse()
                .map_err(|e| LockError::Other(format!("redis url: {e}")))?;
            info.redis.password = Some(pw.clone());
            let client = redis::Client::open(info)
                .map_err(|e| LockError::Other(format!("redis open: {e}")))?;
            let conn = client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| LockError::Other(format!("redis connect: {e}")))?;
            Ok(Self { conn })
        } else {
            Self::connect(&url).await
        }
    }

    pub fn from_connection(conn: MultiplexedConnection) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl DistributedLock for RedisLock {
    async fn acquire(&self, key: &str, ttl: Duration) -> Result<Option<String>, LockError> {
        let mut conn = self.conn.clone();
        let token = Uuid::new_v4().to_string();
        let millis = ttl.as_millis();
        // 与 Cache::set 保持一致：ttl 溢出时钳制为 u64::MAX
        let px = if millis > u64::MAX as u128 {
            u64::MAX
        } else {
            millis as u64
        };
        let acquired: Option<()> = conn
            .set_options(
                key,
                token.as_str(),
                redis::SetOptions::default()
                    .conditional_set(redis::ExistenceCheck::NX)
                    .with_expiration(redis::SetExpiry::PX(px)),
            )
            .await
            .map_err(|e| LockError::Other(format!("redis acquire: {e}")))?;
        if acquired.is_some() {
            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    async fn release(&self, key: &str, token: &str) -> Result<(), LockError> {
        let mut conn = self.conn.clone();
        // Compare-and-delete: only release when the token still matches the holder.
        let script = r#"
            if redis.call("get", KEYS[1]) == ARGV[1] then
                return redis.call("del", KEYS[1])
            else
                return 0
            end
        "#;
        let (): () = redis::Script::new(script)
            .key(key)
            .arg(token)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| LockError::Other(format!("redis release: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_fails_bad_url() {
        let result = RedisCache::connect("redis://nonexistent:9999").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn lock_connect_fails_bad_url() {
        let result = RedisLock::connect("redis://nonexistent:9999").await;
        assert!(result.is_err());
    }

    #[test]
    fn ttl_to_duration_maps_redis_semantics() {
        assert_eq!(ttl_to_duration(-2), None, "missing key");
        assert_eq!(ttl_to_duration(-1), None, "no expiry");
        assert_eq!(ttl_to_duration(0), Some(Duration::ZERO));
        assert_eq!(ttl_to_duration(120), Some(Duration::from_secs(120)));
    }

    fn arg_bytes(a: redis::Arg<&[u8]>) -> Vec<u8> {
        match a {
            redis::Arg::Simple(bytes) => bytes.to_vec(),
            redis::Arg::Cursor => b"*".to_vec(),
        }
    }

    #[test]
    fn incrby_cmd_targets_key_and_delta() {
        let mut cmd = redis::cmd("INCRBY");
        cmd.arg("rl:key").arg(3i64);
        let args: Vec<Vec<u8>> = cmd.args_iter().map(arg_bytes).collect();
        assert_eq!(
            args,
            vec![b"INCRBY".to_vec(), b"rl:key".to_vec(), b"3".to_vec()]
        );
    }

    #[test]
    fn mget_cmd_targets_all_keys() {
        let mut cmd = redis::cmd("MGET");
        cmd.arg("k1").arg("k2").arg("k3");
        let args: Vec<Vec<u8>> = cmd.args_iter().map(arg_bytes).collect();
        assert_eq!(
            args,
            vec![
                b"MGET".to_vec(),
                b"k1".to_vec(),
                b"k2".to_vec(),
                b"k3".to_vec()
            ]
        );
    }

    #[test]
    fn config_deserializes_with_password() {
        let cfg: RedisConfig =
            serde_json::from_str(r#"{"url": "redis://localhost:6379", "password": "secret"}"#)
                .unwrap();
        assert_eq!(cfg.url, "redis://localhost:6379");
        assert_eq!(cfg.password.as_deref(), Some("secret"));
        assert!(cfg.tls.is_none());
    }

    #[test]
    fn config_missing_url_is_error() {
        let result: Result<RedisConfig, _> = serde_json::from_str(r#"{"password": "x"}"#);
        assert!(result.is_err());
    }

    fn tls_enabled() -> TlsClientConfig {
        TlsClientConfig {
            ca_cert: None,
            client_cert: None,
            client_key: None,
            skip_verify: Some(true),
        }
    }

    fn tls_disabled() -> TlsClientConfig {
        TlsClientConfig {
            ca_cert: None,
            client_cert: None,
            client_key: None,
            skip_verify: None,
        }
    }

    #[test]
    fn build_url_swaps_to_rediss_when_tls_enabled() {
        let cfg = RedisConfig {
            url: "redis://localhost:6379".into(),
            password: None,
            tls: Some(tls_enabled()),
        };
        assert_eq!(build_url(&cfg), "rediss://localhost:6379");
    }

    #[test]
    fn build_url_keeps_redis_when_tls_disabled() {
        let cfg = RedisConfig {
            url: "redis://localhost:6379".into(),
            password: None,
            tls: Some(tls_disabled()),
        };
        assert_eq!(build_url(&cfg), "redis://localhost:6379");
    }

    #[test]
    fn build_url_keeps_non_redis_scheme_unchanged() {
        // TLS 只换 redis:// 前缀；非标准 scheme 原样保留
        let cfg = RedisConfig {
            url: "unix:///tmp/redis.sock".into(),
            password: None,
            tls: Some(tls_enabled()),
        };
        assert_eq!(build_url(&cfg), "unix:///tmp/redis.sock");
    }

    #[tokio::test]
    async fn from_config_with_password_path_fails_on_unreachable() {
        // 走 connect_with_password 分支：密码经 ConnectionInfo 传递而非嵌入 URL
        let cfg = RedisConfig {
            url: "redis://127.0.0.1:59999".into(),
            password: Some("pw".into()),
            tls: None,
        };
        // RedisCache 无 Debug，用 match 拿错误文本
        match RedisCache::from_config(cfg).await {
            Err(e) => assert!(!e.to_string().contains("pw"), "password leaked: {e}"),
            Ok(_) => panic!("unreachable redis should fail"),
        }
    }

    #[tokio::test]
    async fn lock_from_config_with_password_path_fails_on_unreachable() {
        let cfg = RedisConfig {
            url: "redis://127.0.0.1:59999".into(),
            password: Some("pw".into()),
            tls: None,
        };
        // RedisLock 无 Debug，用 match 拿错误文本
        match RedisLock::from_config(cfg).await {
            Err(e) => assert!(!e.to_string().contains("pw"), "password leaked: {e}"),
            Ok(_) => panic!("unreachable redis should fail"),
        }
    }
}
