// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_data::GraphClient;
use ecat_errors::{Error, ErrorCode};
use ecat_tls::TlsClientConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Neo4jConfig {
    pub base_url: String,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub tls: Option<TlsClientConfig>,
}

pub struct Neo4jClient {
    client: reqwest::Client,
    base_url: String,
    username: String,
    password: String,
}

impl Neo4jClient {
    pub fn new(
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            username: username.into(),
            password: password.into(),
        }
    }

    pub fn from_config(cfg: Neo4jConfig) -> Result<Self, Error> {
        let client = ecat_tls::build_reqwest_client(&cfg.tls)
            .map_err(|e| Error::new(ErrorCode::Internal, "neo4j_tls", format!("TLS: {e}")))?;
        Ok(Self {
            client,
            base_url: cfg.base_url,
            username: cfg.username,
            password: cfg.password,
        })
    }
}

#[async_trait]
impl GraphClient for Neo4jClient {
    async fn execute(
        &self,
        cypher: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        let body = serde_json::json!({"statements": [{"statement": cypher, "parameters": params}]});
        let resp = self
            .client
            .post(format!("{}/db/data/transaction/commit", self.base_url))
            .basic_auth(&self.username, Some(&self.password))
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "neo4j", format!("neo4j: {e}")))?;
        if !resp.status().is_success() {
            return Err(Error::new(
                ErrorCode::Internal,
                "neo4j",
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "neo4j", format!("neo4j parse: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::routing::post;
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

    /// mock Neo4j transaction-commit 端点：捕获请求路径/头/体，按给定
    /// 状态码与响应体应答（body 为空时返回成功 JSON），返回 mock base_url。
    async fn spawn_mock(
        captured: Arc<Mutex<Vec<CapturedRequest>>>,
        status: u16,
        body: &'static str,
    ) -> String {
        let app = Router::new()
            .route("/db/data/transaction/commit", post(handle))
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
                Json(serde_json::json!({"results": [], "errors": []})).into_response()
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

    #[tokio::test]
    async fn execute_builds_correct_request() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 200, "").await;
        let client = Neo4jClient::new(base_url, "neo4j", "secret");
        let query = "MATCH (n:User) RETURN n LIMIT $limit";
        let params = serde_json::json!({"limit": 10});
        client.execute(query, &params).await.unwrap();

        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs.len(), 1);
        let r = &reqs[0];
        assert_eq!(r.path, "/db/data/transaction/commit");
        // base64("neo4j:secret")
        assert_eq!(r.header("authorization"), Some("Basic bmVvNGo6c2VjcmV0"));
        let body: serde_json::Value = serde_json::from_slice(&r.body).unwrap();
        assert_eq!(body["statements"][0]["statement"], query);
        assert_eq!(body["statements"][0]["parameters"]["limit"], 10);
    }

    #[tokio::test]
    async fn execute_propagates_server_error() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 500, "boom").await;
        let client = Neo4jClient::new(base_url, "neo4j", "secret");
        let err = client
            .execute("MATCH (n) RETURN n", &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("boom"), "got: {err}");
    }

    #[tokio::test]
    async fn execute_404_returns_body_as_error() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 404, "not found").await;
        let client = Neo4jClient::new(base_url, "neo4j", "secret");
        let err = client
            .execute("MATCH (n) RETURN n", &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("not found"), "got: {err}");
    }

    #[tokio::test]
    async fn execute_non_json_body_returns_parse_error() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 200, "definitely not json").await;
        let client = Neo4jClient::new(base_url, "neo4j", "secret");
        let err = client
            .execute("MATCH (n) RETURN n", &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("neo4j parse"), "got: {err}");
    }
}
