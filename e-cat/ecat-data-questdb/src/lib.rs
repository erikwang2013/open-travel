// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
//! QuestDB client (HTTP `/exec` endpoint).
//!
//! Error responses pass through the server's raw body text: credentials are
//! sent via the Authorization header (never in the URL), so error messages
//! cannot leak secrets; outer layers handle the generic error text.

use async_trait::async_trait;
use ecat_data::{RdbmsClient, RdbmsError, Row};
use ecat_tls::TlsClientConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct QuestdbConfig {
    pub base_url: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub tls: Option<TlsClientConfig>,
}

pub struct QuestdbClient {
    client: reqwest::Client,
    base_url: String,
    username: Option<String>,
    password: Option<String>,
}

impl QuestdbClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            username: None,
            password: None,
        }
    }

    pub fn with_auth(
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            username: Some(username.into()),
            password: Some(password.into()),
        }
    }

    pub fn from_config(cfg: QuestdbConfig) -> Result<Self, RdbmsError> {
        let client = ecat_tls::build_reqwest_client(&cfg.tls)
            .map_err(|e| RdbmsError::Config(format!("TLS: {e}")))?;
        Ok(Self {
            client,
            base_url: cfg.base_url,
            username: cfg.username,
            password: cfg.password,
        })
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        ecat_tls::apply_basic_auth(req, &self.username, &self.password)
    }
}

#[async_trait]
impl RdbmsClient for QuestdbClient {
    async fn execute(&self, sql: &str) -> Result<u64, RdbmsError> {
        let req = self
            .client
            .post(format!("{}/exec", self.base_url))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(sql.to_string());
        let resp = self
            .apply_auth(req)
            .send()
            .await
            .map_err(|e| RdbmsError::Database(format!("questdb: {e}")))?;
        if !resp.status().is_success() {
            return Err(RdbmsError::Database(
                resp.text()
                    .await
                    .unwrap_or_else(|e| format!("questdb: {e}")),
            ));
        }
        Ok(0)
    }

    async fn query(&self, sql: &str) -> Result<Vec<Row>, RdbmsError> {
        let req = self
            .client
            .post(format!("{}/exec?count=true", self.base_url))
            .header("Content-Type", "text/plain; charset=utf-8")
            .header("Accept", "application/json")
            .body(sql.to_string());
        let resp = self
            .apply_auth(req)
            .send()
            .await
            .map_err(|e| RdbmsError::Database(format!("questdb: {e}")))?;
        if !resp.status().is_success() {
            return Err(RdbmsError::Database(
                resp.text()
                    .await
                    .unwrap_or_else(|e| format!("questdb: {e}")),
            ));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| RdbmsError::Database(format!("questdb parse: {e}")))?;
        // 2xx 响应也可能携带 error 字段（无 columns/dataset 时）
        if let Some(err) = body
            .get("error")
            .and_then(|e| e.as_str())
            .filter(|e| !e.is_empty())
        {
            return Err(RdbmsError::Database(err.to_string()));
        }
        let mut rows = Vec::new();
        if let Some(columns) = body.get("columns").and_then(|c| c.as_array()) {
            let cols: Vec<String> = columns
                .iter()
                .filter_map(|c| {
                    c.get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                })
                .collect();
            if let Some(dataset) = body.get("dataset").and_then(|d| d.as_array()) {
                for row in dataset {
                    if let Some(vals) = row.as_array() {
                        rows.push(Row::new(cols.clone(), vals.clone()));
                    }
                }
            }
        }
        Ok(rows)
    }

    async fn transaction(&self) -> Result<ecat_data::Transaction, RdbmsError> {
        Err(RdbmsError::Database(
            "QuestDB does not support transactions".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_constructs() {
        let _client = QuestdbClient::new("http://localhost:9000");
    }

    #[test]
    fn config_with_optional_auth() {
        let cfg: QuestdbConfig = serde_json::from_str(
            r#"{"base_url":"http://localhost:9000","username":"admin","password":"quest"}"#,
        )
        .unwrap();
        let client = QuestdbClient::from_config(cfg).unwrap();
        assert!(client.username.is_some());
    }

    /// mock QuestDB 的 /exec 端点，返回给定状态码与响应体。
    async fn spawn_mock_exec(status: u16, body: &'static str) -> String {
        let app = axum::Router::new().route(
            "/exec",
            axum::routing::post(move || async move {
                (
                    axum::http::StatusCode::from_u16(status).unwrap(),
                    axum::response::Response::new(axum::body::Body::from(body)),
                )
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn query_returns_err_on_http_400() {
        let base_url =
            spawn_mock_exec(400, r#"{"code":"invalid","error":"table not found"}"#).await;
        let client = QuestdbClient::new(base_url);
        let err = client.query("select * from nope").await.unwrap_err();
        assert!(err.to_string().contains("table not found"));
    }

    #[tokio::test]
    async fn query_returns_err_on_2xx_with_error_field() {
        let base_url = spawn_mock_exec(200, r#"{"error":"no columns"}"#).await;
        let client = QuestdbClient::new(base_url);
        let err = client.query("select 1").await.unwrap_err();
        assert!(err.to_string().contains("no columns"));
    }

    #[tokio::test]
    async fn query_parses_dataset_into_rows() {
        let body = r#"{
            "columns": [{"name": "id", "type": "INT"}, {"name": "name", "type": "STRING"}],
            "dataset": [[1, "alice"], [2, "bob"]],
            "count": 2
        }"#;
        let base_url = spawn_mock_exec(200, body).await;
        let client = QuestdbClient::new(base_url);
        let rows = client.query("select * from t").await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("id"), Some(&serde_json::json!(1)));
        assert_eq!(rows[0].get("name"), Some(&serde_json::json!("alice")));
        assert_eq!(rows[1].get("id"), Some(&serde_json::json!(2)));
    }

    #[tokio::test]
    async fn query_handles_empty_dataset() {
        let body = r#"{"columns": [{"name": "id", "type": "INT"}], "dataset": [], "count": 0}"#;
        let base_url = spawn_mock_exec(200, body).await;
        let client = QuestdbClient::new(base_url);
        let rows = client.query("select * from t").await.unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn query_skips_malformed_dataset_rows() {
        // 行不是数组时跳过，不 panic
        let body = r#"{"columns": [{"name": "id", "type": "INT"}], "dataset": [[1], "oops", [3]]}"#;
        let base_url = spawn_mock_exec(200, body).await;
        let client = QuestdbClient::new(base_url);
        let rows = client.query("select * from t").await.unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn execute_returns_zero_on_success() {
        let base_url = spawn_mock_exec(200, r#"{"ddl":"OK"}"#).await;
        let client = QuestdbClient::new(base_url);
        assert_eq!(client.execute("create table t (i int)").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn transaction_returns_not_supported_error() {
        let client = QuestdbClient::new("http://localhost:9000");
        let err = match client.transaction().await {
            Err(e) => e,
            Ok(_) => panic!("expected unsupported error"),
        };
        assert!(
            err.to_string().contains("does not support transactions"),
            "got: {err}"
        );
    }
}
