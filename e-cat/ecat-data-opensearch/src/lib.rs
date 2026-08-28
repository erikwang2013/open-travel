// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
//! OpenSearch client.
//!
//! All writes/reads/errors are validated against the HTTP status code; index
//! names and document ids are percent-encoded before being placed in the URL
//! path so that reserved characters cannot break the request.

use async_trait::async_trait;
use ecat_data::SearchClient;
use ecat_errors::{Error, ErrorCode};
use ecat_tls::TlsClientConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct OpenSearchConfig {
    pub base_url: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub tls: Option<TlsClientConfig>,
}

pub struct OpenSearchClient {
    client: reqwest::Client,
    base_url: String,
    username: Option<String>,
    password: Option<String>,
}

impl OpenSearchClient {
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

    pub fn from_config(cfg: OpenSearchConfig) -> Result<Self, Error> {
        let client = ecat_tls::build_reqwest_client(&cfg.tls)
            .map_err(|e| Error::new(ErrorCode::Internal, "opensearch", format!("TLS: {e}")))?;
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

/// Build the non-2xx error message, including the HTTP status code.
async fn status_error(prefix: &str, resp: reqwest::Response) -> Error {
    let status = resp.status().as_u16();
    Error::new(
        ErrorCode::Internal,
        "opensearch",
        format!(
            "{prefix} failed: status {status}, body: {}",
            resp.text().await.unwrap_or_default()
        ),
    )
}

#[async_trait]
impl SearchClient for OpenSearchClient {
    async fn index(&self, index: &str, id: &str, doc: &serde_json::Value) -> Result<(), Error> {
        let req = self
            .client
            .put(format!(
                "{}/{}/_doc/{}",
                self.base_url,
                percent_encode_segment(index),
                percent_encode_segment(id)
            ))
            .json(doc);
        let resp =
            self.apply_auth(req).send().await.map_err(|e| {
                Error::new(ErrorCode::Internal, "opensearch", format!("index: {e}"))
            })?;
        if !resp.status().is_success() {
            return Err(status_error("index", resp).await);
        }
        Ok(())
    }

    async fn search(
        &self,
        index: &str,
        query: &serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        let req = self
            .client
            .post(format!(
                "{}/{}/_search",
                self.base_url,
                percent_encode_segment(index)
            ))
            .json(query);
        let resp =
            self.apply_auth(req).send().await.map_err(|e| {
                Error::new(ErrorCode::Internal, "opensearch", format!("search: {e}"))
            })?;
        if !resp.status().is_success() {
            return Err(status_error("search", resp).await);
        }
        resp.json()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "opensearch", format!("parse: {e}")))
    }

    async fn delete(&self, index: &str, id: &str) -> Result<(), Error> {
        let req = self.client.delete(format!(
            "{}/{}/_doc/{}",
            self.base_url,
            percent_encode_segment(index),
            percent_encode_segment(id)
        ));
        let resp =
            self.apply_auth(req).send().await.map_err(|e| {
                Error::new(ErrorCode::Internal, "opensearch", format!("delete: {e}"))
            })?;
        if !resp.status().is_success() {
            return Err(status_error("delete", resp).await);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_constructs() {
        let _client = OpenSearchClient::new("http://localhost:9200");
    }

    #[test]
    fn client_with_auth() {
        let _client = OpenSearchClient::with_auth("http://localhost:9200", "admin", "secret");
    }

    #[test]
    fn config_with_optional_auth() {
        let cfg: OpenSearchConfig = serde_json::from_str(
            r#"{"base_url":"http://localhost:9200","username":"admin","password":"secret"}"#,
        )
        .unwrap();
        let client = OpenSearchClient::from_config(cfg).unwrap();
        assert!(client.username.is_some());
    }

    #[test]
    fn percent_encode_segment_encodes_reserved_chars() {
        assert_eq!(percent_encode_segment("logs-2026"), "logs-2026");
        assert_eq!(
            percent_encode_segment("a/b c#d?e%f"),
            "a%2Fb%20c%23d%3Fe%25f"
        );
    }

    #[test]
    fn config_missing_base_url_is_error() {
        let result: Result<OpenSearchConfig, _> = serde_json::from_str(r#"{}"#);
        assert!(result.is_err());
    }

    type Captured =
        std::sync::Arc<std::sync::Mutex<Vec<(String, String, Vec<(String, String)>, Vec<u8>)>>>;

    /// mock OpenSearch：捕获请求方法与路径，按给定状态码与响应体应答
    /// （body 为空时返回成功 JSON），返回 mock base_url。
    async fn spawn_mock(captured: Captured, status: u16, body: &'static str) -> String {
        let app = axum::Router::new().fallback(
            move |req: axum::http::Request<axum::body::Body>| async move {
                let method = req.method().to_string();
                let path = req.uri().path().to_string();
                let (parts, req_body) = req.into_parts();
                let headers: Vec<(String, String)> = parts
                    .headers
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.as_str().to_string(),
                            v.to_str().unwrap_or_default().to_string(),
                        )
                    })
                    .collect();
                let req_body = axum::body::to_bytes(req_body, usize::MAX)
                    .await
                    .unwrap_or_default();
                captured.lock().unwrap_or_else(|e| e.into_inner()).push((
                    method,
                    path,
                    headers,
                    req_body.to_vec(),
                ));
                use axum::response::IntoResponse;
                if body.is_empty() {
                    axum::Json(serde_json::json!({"hits": {"total": 0}})).into_response()
                } else {
                    (
                        axum::http::StatusCode::from_u16(status).unwrap(),
                        axum::response::Response::new(axum::body::Body::from(body)),
                    )
                        .into_response()
                }
            },
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    #[tokio::test]
    async fn index_puts_encoded_path_with_doc_and_auth() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let base_url = spawn_mock(captured.clone(), 200, "").await;
        let client = OpenSearchClient::with_auth(base_url, "admin", "secret");
        client
            .index("logs 2026", "doc/1", &serde_json::json!({"msg": "hi"}))
            .await
            .unwrap();

        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].0, "PUT");
        // 路径段 percent-encode：空格与斜杠被编码
        assert_eq!(reqs[0].1, "/logs%202026/_doc/doc%2F1");
        assert_eq!(
            header(&reqs[0].2, "authorization"),
            Some("Basic YWRtaW46c2VjcmV0"),
            "basic auth 头缺失"
        );
        let body: serde_json::Value = serde_json::from_slice(&reqs[0].3).unwrap();
        assert_eq!(body["msg"], "hi");
    }

    #[tokio::test]
    async fn search_posts_query_and_parses_json_response() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let base_url = spawn_mock(captured.clone(), 200, r#"{"hits":{"total":1}}"#).await;
        let client = OpenSearchClient::new(base_url);
        let query = serde_json::json!({"query": {"match_all": {}}});
        let v = client.search("idx", &query).await.unwrap();
        assert_eq!(v["hits"]["total"], 1);

        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs[0].0, "POST");
        assert_eq!(reqs[0].1, "/idx/_search");
        let body: serde_json::Value = serde_json::from_slice(&reqs[0].3).unwrap();
        assert_eq!(body, query);
    }

    #[tokio::test]
    async fn delete_sends_delete_request() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let base_url = spawn_mock(captured.clone(), 200, "").await;
        let client = OpenSearchClient::new(base_url);
        client.delete("idx", "1").await.unwrap();
        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs[0].0, "DELETE");
        assert_eq!(reqs[0].1, "/idx/_doc/1");
    }

    #[tokio::test]
    async fn index_propagates_http_error_with_status_and_body() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let base_url = spawn_mock(captured.clone(), 400, "mapper parse error").await;
        let client = OpenSearchClient::new(base_url);
        let err = client
            .index("idx", "1", &serde_json::json!({}))
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("status 400"), "got: {msg}");
        assert!(msg.contains("mapper parse error"), "got: {msg}");
    }

    #[tokio::test]
    async fn search_non_json_body_returns_parse_error() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let base_url = spawn_mock(captured.clone(), 200, "not json").await;
        let client = OpenSearchClient::new(base_url);
        let err = client
            .search("idx", &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("parse"), "got: {err}");
    }
}
