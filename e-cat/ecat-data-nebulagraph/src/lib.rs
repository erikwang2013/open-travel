// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_data::GraphClient;
use ecat_errors::{Error, ErrorCode};
use ecat_tls::TlsClientConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct NebulaGraphConfig {
    pub base_url: String,
    pub space: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub tls: Option<TlsClientConfig>,
}

pub struct NebulaGraphClient {
    client: reqwest::Client,
    base_url: String,
    space: String,
    username: Option<String>,
    password: Option<String>,
}

impl NebulaGraphClient {
    pub fn new(base_url: impl Into<String>, space: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            space: space.into(),
            username: None,
            password: None,
        }
    }

    pub fn with_auth(
        base_url: impl Into<String>,
        space: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            space: space.into(),
            username: Some(username.into()),
            password: Some(password.into()),
        }
    }

    pub fn from_config(cfg: NebulaGraphConfig) -> Result<Self, Error> {
        let client = ecat_tls::build_reqwest_client(&cfg.tls)
            .map_err(|e| Error::new(ErrorCode::Internal, "nebula_tls", format!("TLS: {e}")))?;
        Ok(Self {
            client,
            base_url: cfg.base_url,
            space: cfg.space,
            username: cfg.username,
            password: cfg.password,
        })
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        ecat_tls::apply_basic_auth(req, &self.username, &self.password)
    }
}

#[async_trait]
impl GraphClient for NebulaGraphClient {
    async fn execute(
        &self,
        ngql: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        if !params.is_null() {
            return Err(Error::new(
                ErrorCode::Internal,
                "nebula",
                "params not supported",
            ));
        }
        let req = self
            .client
            .post(format!("{}/api/ngql/execute", self.base_url))
            .json(&serde_json::json!({"gql": ngql, "space": self.space}));
        let resp = self
            .apply_auth(req)
            .send()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "nebula", format!("nebula: {e}")))?;
        if !resp.status().is_success() {
            return Err(Error::new(
                ErrorCode::Internal,
                "nebula",
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "nebula", format!("nebula parse: {e}")))
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

    #[test]
    fn client_constructs() {
        let _client = NebulaGraphClient::new("http://localhost:19669", "test_space");
    }

    #[test]
    fn config_with_optional_auth() {
        let cfg: NebulaGraphConfig = serde_json::from_str(
            r#"{"base_url":"http://localhost:19669","space":"test","username":"root","password":"nebula"}"#
        ).unwrap();
        let client = NebulaGraphClient::from_config(cfg).unwrap();
        assert!(client.username.is_some());
    }

    #[test]
    fn config_missing_space_is_error() {
        let result: Result<NebulaGraphConfig, _> =
            serde_json::from_str(r#"{"base_url":"http://localhost:19669"}"#);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_rejects_params() {
        let client = NebulaGraphClient::new("http://localhost:19669", "test_space");
        let err = client
            .execute("SHOW SPACES", &serde_json::json!({"limit": 5}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("params not supported"));
    }

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

    /// mock NebulaGraph ngql 端点：捕获请求路径/头/体，按给定状态码与
    /// 响应体应答（body 为空时返回成功 JSON），返回 mock base_url。
    async fn spawn_mock(
        captured: Arc<Mutex<Vec<CapturedRequest>>>,
        status: u16,
        body: &'static str,
    ) -> String {
        let app = Router::new()
            .route("/api/ngql/execute", post(handle))
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

    #[tokio::test]
    async fn execute_builds_correct_request() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 200, "").await;
        let client = NebulaGraphClient::with_auth(base_url, "test_space", "root", "nebula");
        let ngql = "SHOW SPACES";
        client
            .execute(ngql, &serde_json::Value::Null)
            .await
            .unwrap();

        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs.len(), 1);
        let r = &reqs[0];
        assert_eq!(r.path, "/api/ngql/execute");
        // base64("root:nebula")
        assert_eq!(r.header("authorization"), Some("Basic cm9vdDpuZWJ1bGE="));
        let body: serde_json::Value = serde_json::from_slice(&r.body).unwrap();
        assert_eq!(body["gql"], ngql);
        assert_eq!(body["space"], "test_space");
    }

    #[tokio::test]
    async fn execute_propagates_server_error() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 500, "boom").await;
        let client = NebulaGraphClient::new(base_url, "test_space");
        let err = client
            .execute("SHOW SPACES", &serde_json::Value::Null)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("boom"), "got: {err}");
    }

    #[tokio::test]
    async fn execute_non_json_body_returns_parse_error() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 200, "not json at all").await;
        let client = NebulaGraphClient::new(base_url, "test_space");
        let err = client
            .execute("SHOW SPACES", &serde_json::Value::Null)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("nebula parse"), "got: {err}");
    }
}
