// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use base64::Engine as _;
use ecat_data::{RdbmsClient, RdbmsError, Row, TransactionInner};
use ecat_tls::TlsClientConfig;
use serde::Deserialize;
use sqlx::any::AnyRow;
use sqlx::{AnyPool, Column as SqlxColumn, Row as SqlxRow};

#[derive(Debug, Clone, Deserialize)]
pub struct SqlxConfig {
    pub url: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    /// TLS — SQLx TLS is configured via URL params (e.g. ?sslmode=require).
    /// This field is reserved for future programmatic TLS support.
    #[serde(default)]
    pub tls: Option<TlsClientConfig>,
}

fn percent_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ':' | '/' | '@' | '#' | '?' | '&' | '=' | '%' | '+' | ' ' => {
                format!("%{:02X}", c as u8)
            }
            _ => c.to_string(),
        })
        .collect()
}

/// AnyPool 首次连接前必须先安装驱动，否则 sqlx 直接 panic
/// "No drivers installed"。install_default_drivers 内部对每个驱动
/// 也是 Once 保护，这里再用 Once 显式保证只装一次。
static DRIVERS_INSTALLED: std::sync::Once = std::sync::Once::new();

fn ensure_drivers() {
    DRIVERS_INSTALLED.call_once(sqlx::any::install_default_drivers);
}

pub struct SqlxClient {
    pool: AnyPool,
}

impl SqlxClient {
    pub async fn connect(url: &str) -> Result<Self, sqlx::Error> {
        ensure_drivers();
        let pool = AnyPool::connect(url).await?;
        Ok(Self { pool })
    }

    pub async fn connect_with_auth(
        url: &str,
        username: &str,
        password: &str,
    ) -> Result<Self, sqlx::Error> {
        let url = if url.contains('@') {
            url.to_string()
        } else {
            let encoded_user = percent_encode(username);
            let encoded_pass = percent_encode(password);
            url.replacen("://", &format!("://{encoded_user}:{encoded_pass}@"), 1)
        };
        Self::connect(&url).await
    }

    pub async fn from_config(cfg: SqlxConfig) -> Result<Self, sqlx::Error> {
        match (&cfg.username, &cfg.password) {
            (Some(u), Some(p)) if !u.is_empty() || !p.is_empty() => {
                Self::connect_with_auth(&cfg.url, u, p).await
            }
            _ => Self::connect(&cfg.url).await,
        }
    }

    pub fn from_pool(pool: AnyPool) -> Self {
        Self { pool }
    }
}

/// 单格类型转换链：bool → i64 → i32 → f64（NaN/Inf 转字符串）→ String →
/// Blob（base64）。此前 Blob/BYTEA 列会静默落到 Null。
/// 注意：sqlx Any 驱动不支持时间类型，timestamp 列会在 fetch 时直接报错
/// （AnyDriverError），不会静默——调用方可自行 CAST 成文本。
fn cell_to_json(row: &AnyRow, col: &str) -> serde_json::Value {
    row.try_get::<bool, _>(col)
        .map(serde_json::Value::Bool)
        .or_else(|_| {
            row.try_get::<i64, _>(col)
                .map(|n| serde_json::Value::Number(n.into()))
        })
        .or_else(|_| {
            row.try_get::<i32, _>(col)
                .map(|n| serde_json::Value::Number((n as i64).into()))
        })
        .or_else(|_| {
            row.try_get::<f64, _>(col)
                .ok()
                .and_then(|n| {
                    if n.is_finite() {
                        serde_json::Number::from_f64(n).map(serde_json::Value::Number)
                    } else if n.is_nan() {
                        Some(serde_json::Value::String("NaN".into()))
                    } else if n > 0.0 {
                        Some(serde_json::Value::String("Infinity".into()))
                    } else {
                        Some(serde_json::Value::String("-Infinity".into()))
                    }
                })
                .ok_or(())
        })
        .or_else(|_| row.try_get::<String, _>(col).map(serde_json::Value::String))
        .or_else(|_| {
            row.try_get::<Vec<u8>, _>(col).map(|b| {
                serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(b))
            })
        })
        .unwrap_or(serde_json::Value::Null)
}

fn rows_to_result(rows: Vec<AnyRow>) -> Vec<Row> {
    if rows.is_empty() {
        return Vec::new();
    }
    let columns: Vec<String> = rows[0]
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();
    rows.iter()
        .map(|row| {
            let values: Vec<serde_json::Value> =
                columns.iter().map(|col| cell_to_json(row, col)).collect();
            Row::new(columns.clone(), values)
        })
        .collect()
}

#[async_trait]
impl RdbmsClient for SqlxClient {
    async fn execute(&self, sql: &str) -> Result<u64, RdbmsError> {
        sqlx::query(sql)
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected())
            .map_err(|e| RdbmsError::Database(e.to_string()))
    }

    async fn query(&self, sql: &str) -> Result<Vec<Row>, RdbmsError> {
        let rows: Vec<AnyRow> = sqlx::query(sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| RdbmsError::Database(e.to_string()))?;
        Ok(rows_to_result(rows))
    }

    async fn execute_with(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> Result<u64, RdbmsError> {
        let mut q = sqlx::query(sql);
        for p in params {
            q = match p {
                serde_json::Value::String(s) => q.bind(s.as_str()),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        q.bind(i)
                    } else if let Some(f) = n.as_f64() {
                        q.bind(f)
                    } else {
                        q.bind(n.to_string())
                    }
                }
                serde_json::Value::Bool(b) => q.bind(*b),
                serde_json::Value::Null => q.bind(None::<String>),
                _ => q.bind(p.to_string()),
            };
        }
        q.execute(&self.pool)
            .await
            .map(|r| r.rows_affected())
            .map_err(|e| RdbmsError::Database(e.to_string()))
    }

    async fn query_with(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> Result<Vec<Row>, RdbmsError> {
        let mut q = sqlx::query(sql);
        for p in params {
            q = match p {
                serde_json::Value::String(s) => q.bind(s.as_str()),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        q.bind(i)
                    } else if let Some(f) = n.as_f64() {
                        q.bind(f)
                    } else {
                        q.bind(n.to_string())
                    }
                }
                serde_json::Value::Bool(b) => q.bind(*b),
                serde_json::Value::Null => q.bind(None::<String>),
                _ => q.bind(p.to_string()),
            };
        }
        let rows: Vec<sqlx::any::AnyRow> = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| RdbmsError::Database(e.to_string()))?;
        Ok(rows_to_result(rows))
    }

    async fn transaction(&self) -> Result<ecat_data::Transaction, RdbmsError> {
        let tx = self
            .pool
            .begin()
            .await
            .map_err(|e| RdbmsError::Database(e.to_string()))?;
        Ok(ecat_data::Transaction::with_inner(Box::new(
            SqlxTransactionWrapper { inner: Some(tx) },
        )))
    }
}

struct SqlxTransactionWrapper {
    inner: Option<sqlx::Transaction<'static, sqlx::Any>>,
}

#[async_trait]
impl TransactionInner for SqlxTransactionWrapper {
    async fn commit(&mut self) -> Result<(), RdbmsError> {
        if let Some(tx) = self.inner.take() {
            tx.commit()
                .await
                .map_err(|e| RdbmsError::Database(e.to_string()))?;
        }
        Ok(())
    }

    async fn rollback(&mut self) -> Result<(), RdbmsError> {
        if let Some(tx) = self.inner.take() {
            tx.rollback()
                .await
                .map_err(|e| RdbmsError::Database(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encode_special_chars() {
        assert_eq!(percent_encode("user:pass"), "user%3Apass");
        assert_eq!(percent_encode("a/b@c"), "a%2Fb%40c");
        assert_eq!(percent_encode("a#b?c&d=e"), "a%23b%3Fc%26d%3De");
        assert_eq!(percent_encode("100%"), "100%25");
        assert_eq!(percent_encode("a+b"), "a%2Bb");
        assert_eq!(percent_encode("hello world"), "hello%20world");
    }

    #[test]
    fn percent_encode_no_special_chars() {
        assert_eq!(percent_encode("simple"), "simple");
        assert_eq!(percent_encode("user123"), "user123");
        assert_eq!(percent_encode(""), "");
    }

    #[test]
    fn config_deserialize_basic() {
        let cfg: SqlxConfig =
            serde_json::from_str(r#"{"url": "postgres://localhost/db"}"#).unwrap();
        assert_eq!(cfg.url, "postgres://localhost/db");
        assert!(cfg.username.is_none());
        assert!(cfg.password.is_none());
        assert!(cfg.tls.is_none());
    }

    #[test]
    fn config_deserialize_with_auth() {
        let cfg: SqlxConfig = serde_json::from_str(
            r#"{"url": "mysql://localhost/db", "username": "root", "password": "secret"}"#,
        )
        .unwrap();
        assert_eq!(cfg.url, "mysql://localhost/db");
        assert_eq!(cfg.username.as_deref(), Some("root"));
        assert_eq!(cfg.password.as_deref(), Some("secret"));
    }

    #[test]
    fn config_deserialize_with_tls() {
        let cfg: SqlxConfig = serde_json::from_str(
            r#"{"url": "postgres://localhost/db", "tls": {"skip_verify": true}}"#,
        )
        .unwrap();
        assert!(cfg.tls.is_some());
        let tls = cfg.tls.unwrap();
        assert_eq!(tls.skip_verify, Some(true));
    }

    #[test]
    fn config_missing_url_is_error() {
        let result: Result<SqlxConfig, _> = serde_json::from_str(r#"{}"#);
        assert!(result.is_err());
    }

    #[test]
    fn from_pool_is_constructible() {
        // Compile-time check: SqlxClient::from_pool exists with correct signature.
        fn _check_sig(pool: sqlx::AnyPool) -> SqlxClient {
            SqlxClient::from_pool(pool)
        }
    }

    /// 内存 SQLite：无外部服务即可做端到端往返。
    /// 每次调用名唯一，避免并行测试互相干扰。
    fn mem_sqlite(name: &str) -> String {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("sqlite:ecat-test-{name}{n}?mode=memory&cache=shared")
    }

    /// sqlx 0.8 要求先安装 Any driver（Once 保护，重复调用安全）。
    fn init_drivers() {
        sqlx::any::install_default_drivers();
    }

    /// 单连接池客户端：内存库随连接销毁，多连接池建的表会被后续
    /// 连接丢失，单连接池保证同测试内所有语句命中同一库。
    async fn single_conn_client(name: &str) -> SqlxClient {
        init_drivers();
        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(1)
            .connect(&mem_sqlite(name))
            .await
            .unwrap();
        SqlxClient::from_pool(pool)
    }

    #[tokio::test]
    async fn connect_and_execute_query_round_trip() {
        let client = single_conn_client("t1").await;
        client
            .execute("CREATE TABLE t (id INTEGER, name TEXT)")
            .await
            .unwrap();
        client
            .execute("INSERT INTO t VALUES (1, 'alice'), (2, 'bob')")
            .await
            .unwrap();
        let rows = client
            .query("SELECT id, name FROM t ORDER BY id")
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("id"), Some(&serde_json::json!(1)));
        assert_eq!(rows[0].get("name"), Some(&serde_json::json!("alice")));
        assert_eq!(rows[1].get("id"), Some(&serde_json::json!(2)));
        assert_eq!(rows[1].get("missing"), None);
    }

    #[tokio::test]
    async fn execute_with_binds_all_json_value_types() {
        let client = single_conn_client("t1").await;
        client
            .execute("CREATE TABLE t (s TEXT, i INTEGER, f REAL, b INTEGER, n TEXT)")
            .await
            .unwrap();
        let affected = client
            .execute_with(
                "INSERT INTO t VALUES (?, ?, ?, ?, ?)",
                &[
                    serde_json::json!("str"),
                    serde_json::json!(42),
                    serde_json::json!(1.5),
                    serde_json::json!(true),
                    serde_json::Value::Null,
                ],
            )
            .await
            .unwrap();
        assert_eq!(affected, 1);
        let rows = client.query("SELECT * FROM t").await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("s"), Some(&serde_json::json!("str")));
        assert_eq!(rows[0].get("i"), Some(&serde_json::json!(42)));
        assert_eq!(rows[0].get("f"), Some(&serde_json::json!(1.5)));
        // SQLite 无布尔类型：true 绑定后回读为整数 1
        assert_eq!(rows[0].get("b"), Some(&serde_json::json!(1)));
        assert_eq!(rows[0].get("n"), Some(&serde_json::Value::Null));
    }

    #[tokio::test]
    async fn query_with_parameterized_sql() {
        let client = single_conn_client("t1").await;
        client.execute("CREATE TABLE t (name TEXT)").await.unwrap();
        client
            .execute_with("INSERT INTO t VALUES (?)", &[serde_json::json!("x")])
            .await
            .unwrap();
        let rows = client
            .query_with(
                "SELECT name FROM t WHERE name = ?",
                &[serde_json::json!("x")],
            )
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("name"), Some(&serde_json::json!("x")));
    }

    #[tokio::test]
    async fn cell_to_json_encodes_blob_as_base64() {
        let client = single_conn_client("t1").await;
        client.execute("CREATE TABLE t (data BLOB)").await.unwrap();
        // 真实 BLOB 字节（0x01 0x02 0x03），绕过 bind（bind 会把 JSON 值绑成文本）
        client
            .execute("INSERT INTO t VALUES (x'010203')")
            .await
            .unwrap();
        let rows = client.query("SELECT data FROM t").await.unwrap();
        assert_eq!(rows[0].get("data"), Some(&serde_json::json!("AQID")));
    }

    #[tokio::test]
    async fn connect_with_auth_does_not_inject_into_non_url_scheme() {
        // sqlite URL 不含 "://"，凭据注入分支不生效，直接连接成功
        init_drivers();
        let client = SqlxClient::connect_with_auth(&mem_sqlite("t2"), "user", "pass")
            .await
            .unwrap();
        client.execute("SELECT 1").await.unwrap();
    }

    #[tokio::test]
    async fn from_config_without_auth_connects_and_queries() {
        // 不手动 init_drivers：验证 from_config 自身完成驱动安装后
        // 能在内存 sqlite 上实际执行查询
        // （注意：多连接池下 mode=memory 库随连接关闭而销毁，故只做单条查询）
        let cfg = SqlxConfig {
            url: mem_sqlite("t3"),
            username: None,
            password: None,
            tls: None,
        };
        let client = SqlxClient::from_config(cfg).await.unwrap();
        let rows = client.query("SELECT 1 AS one").await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("one"), Some(&serde_json::json!(1)));
    }

    #[tokio::test]
    async fn from_config_with_empty_credentials_connects_plain() {
        // (Some(""), Some("")) 或 (Some(""), None) 都走无认证分支；
        // from_config 内部会装驱动，无需手动 init_drivers
        for (u, p) in [
            (Some("".to_string()), Some("".to_string())),
            (Some("".to_string()), None),
        ] {
            let cfg = SqlxConfig {
                url: mem_sqlite("t3"),
                username: u,
                password: p,
                tls: None,
            };
            let client = SqlxClient::from_config(cfg).await.unwrap();
            client.execute("SELECT 1").await.unwrap();
        }
    }
}
