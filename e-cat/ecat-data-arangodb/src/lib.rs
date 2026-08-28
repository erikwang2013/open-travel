// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_data::GraphClient;
use ecat_errors::{Error, ErrorCode};
use ecat_tls::TlsClientConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ArangoConfig {
    pub base_url: String,
    pub db: String,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub tls: Option<TlsClientConfig>,
}

pub struct ArangoClient {
    client: reqwest::Client,
    base_url: String,
    db: String,
    username: String,
    password: String,
}

impl ArangoClient {
    pub fn new(
        base_url: impl Into<String>,
        db: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            db: db.into(),
            username: username.into(),
            password: password.into(),
        }
    }

    pub fn from_config(cfg: ArangoConfig) -> Result<Self, Error> {
        let client = ecat_tls::build_reqwest_client(&cfg.tls)
            .map_err(|e| Error::new(ErrorCode::Internal, "arango_tls", format!("TLS: {e}")))?;
        Ok(Self {
            client,
            base_url: cfg.base_url,
            db: cfg.db,
            username: cfg.username,
            password: cfg.password,
        })
    }
}

/// Percent-encode a single URL path segment (RFC 3986): every byte except
/// unreserved characters (`A-Z a-z 0-9 - _ . ~`) becomes `%XX`.
fn percent_encode_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[async_trait]
impl GraphClient for ArangoClient {
    async fn execute(
        &self,
        aql: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        let body = serde_json::json!({"query": aql, "bindVars": params});
        let resp = self
            .client
            .post(format!(
                "{}/_db/{}/_api/cursor",
                self.base_url,
                percent_encode_segment(&self.db)
            ))
            .basic_auth(&self.username, Some(&self.password))
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "arango", format!("arango: {e}")))?;
        if !resp.status().is_success() {
            return Err(Error::new(
                ErrorCode::Internal,
                "arango",
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "arango", format!("arango parse: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::{Json, Router};
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct CapturedRequest {
        path: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }

    impl CapturedRequest {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.as_str())
        }
    }

    type MockState = (Arc<Mutex<Vec<CapturedRequest>>>, u16, &'static str);

    /// mock ArangoDB cursor 端点（路径含编码后的 db 名，用 fallback 捕获
    /// 任意路径）：捕获请求路径/头/体，按给定状态码与响应体应答
    /// （body 为空时返回成功 JSON），返回 mock base_url。
    async fn spawn_mock(
        captured: Arc<Mutex<Vec<CapturedRequest>>>,
        status: u16,
        body: &'static str,
    ) -> String {
        let app = Router::new()
            .fallback(handle)
            .with_state((captured, status, body));

        async fn handle(
            State((captured, status, body)): State<MockState>,
            req: axum::http::Request<Body>,
        ) -> axum::response::Response {
            let path = req.uri().path().to_string();
            let (parts, req_body) = req.into_parts();
            let headers = parts
                .headers
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().to_string(),
                        v.to_str().unwrap_or_default().to_string(),
                    )
                })
                .collect();
            let req_body = to_bytes(req_body, usize::MAX).await.unwrap_or_default();
            captured
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(CapturedRequest {
                    path,
                    headers,
                    body: req_body.to_vec(),
                });
            if body.is_empty() {
                Json(serde_json::json!({"result": []})).into_response()
            } else {
                (StatusCode::from_u16(status).unwrap(), Body::from(body)).into_response()
            }
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    #[test]
    fn percent_encode_segment_encodes_reserved_chars() {
        assert_eq!(percent_encode_segment("mydb"), "mydb");
        assert_eq!(percent_encode_segment("my db/1"), "my%20db%2F1");
        assert_eq!(percent_encode_segment("你好"), "%E4%BD%A0%E5%A5%BD");
    }

    #[test]
    fn config_deserializes() {
        let cfg: ArangoConfig = serde_json::from_value(serde_json::json!({
            "base_url": "http://localhost:8529",
            "db": "mydb",
            "username": "root",
            "password": "secret",
        }))
        .unwrap();
        assert_eq!(cfg.db, "mydb");
        assert!(cfg.tls.is_none());
    }

    #[test]
    fn config_missing_db_is_error() {
        let result: Result<ArangoConfig, _> = serde_json::from_str(
            r#"{"base_url":"http://localhost:8529","username":"root","password":"s"}"#,
        );
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_builds_correct_request() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 200, "").await;
        let client = ArangoClient::new(base_url, "mydb", "root", "secret");
        let aql = "FOR u IN users FILTER u.age > @min RETURN u";
        let params = serde_json::json!({"min": 18});
        client.execute(aql, &params).await.unwrap();

        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs.len(), 1);
        let r = &reqs[0];
        assert_eq!(r.path, "/_db/mydb/_api/cursor");
        // base64("root:secret")
        assert_eq!(r.header("authorization"), Some("Basic cm9vdDpzZWNyZXQ="));
        let body: serde_json::Value = serde_json::from_slice(&r.body).unwrap();
        assert_eq!(body["query"], aql);
        assert_eq!(body["bindVars"]["min"], 18);
    }

    #[tokio::test]
    async fn execute_percent_encodes_db_name() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 200, "").await;
        let client = ArangoClient::new(base_url, "my db/1", "root", "");
        client
            .execute("RETURN 1", &serde_json::json!({}))
            .await
            .unwrap();

        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].path, "/_db/my%20db%2F1/_api/cursor");
    }

    #[tokio::test]
    async fn execute_propagates_server_error() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 500, "boom").await;
        let client = ArangoClient::new(base_url, "mydb", "root", "");
        let err = client
            .execute("RETURN 1", &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("boom"), "got: {err}");
    }

    #[tokio::test]
    async fn execute_non_json_body_returns_parse_error() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 200, "not json").await;
        let client = ArangoClient::new(base_url, "mydb", "root", "");
        let err = client
            .execute("RETURN 1", &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("arango parse"), "got: {err}");
    }
}
