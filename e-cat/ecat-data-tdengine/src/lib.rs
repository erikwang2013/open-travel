// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_data::{DataPoint, FieldValue, TsdbClient};
use ecat_errors::{Error, ErrorCode};
use ecat_tls::TlsClientConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TdengineConfig {
    pub base_url: String,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub database: Option<String>,
    #[serde(default)]
    pub tls: Option<TlsClientConfig>,
}

pub struct TdengineClient {
    client: reqwest::Client,
    base_url: String,
    username: String,
    password: String,
    database: Option<String>,
}

impl TdengineClient {
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
            database: None,
        }
    }

    pub fn from_config(cfg: TdengineConfig) -> Result<Self, Error> {
        let client = ecat_tls::build_reqwest_client(&cfg.tls)
            .map_err(|e| Error::new(ErrorCode::Internal, "tdengine_tls", format!("TLS: {e}")))?;
        Ok(Self {
            client,
            base_url: cfg.base_url,
            username: cfg.username,
            password: cfg.password,
            database: cfg.database,
        })
    }

    fn sql_url(&self) -> String {
        match &self.database {
            Some(db) => format!("{}/rest/sql/{}", self.base_url, percent_encode_segment(db)),
            None => format!("{}/rest/sql", self.base_url),
        }
    }

    async fn exec(&self, sql: &str) -> Result<serde_json::Value, Error> {
        let resp = self
            .client
            .post(self.sql_url())
            .basic_auth(&self.username, Some(&self.password))
            .json(&serde_json::json!({ "sql": sql }))
            .send()
            .await
            .map_err(|e| {
                Error::new(
                    ErrorCode::Internal,
                    "tdengine",
                    format!("tdengine exec: {e}"),
                )
            })?;
        if !resp.status().is_success() {
            return Err(Error::new(
                ErrorCode::Internal,
                "tdengine",
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json().await.map_err(|e| {
            Error::new(
                ErrorCode::Internal,
                "tdengine",
                format!("tdengine parse: {e}"),
            )
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

/// 转义双引号字符串字面量：先转义反斜杠再转义双引号，防止注入逃逸
fn escape_sql_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// 转义双引号包裹的标识符（measurement/列名）
fn escape_ident(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// 单条 DataPoint 生成一条 INSERT 语句
fn point_to_insert(p: &DataPoint) -> String {
    // Tags are flattened as columns; measurement is the table name.
    let mut cols = Vec::new();
    let mut vals = Vec::new();
    cols.push("ts".to_string());
    vals.push(
        p.timestamp
            .map(|ts| ts.to_string())
            .unwrap_or_else(|| "now".to_string()),
    );
    for (k, v) in &p.tags {
        cols.push(format!("\"{}\"", escape_ident(k)));
        vals.push(format!("\"{}\"", escape_sql_string(v)));
    }
    for (k, v) in &p.fields {
        cols.push(format!("\"{}\"", escape_ident(k)));
        vals.push(match v {
            FieldValue::Float(f) => f.to_string(),
            FieldValue::Int(i) => i.to_string(),
            FieldValue::String(s) => format!("\"{}\"", escape_sql_string(s)),
            FieldValue::Bool(b) => {
                if *b {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
        });
    }
    format!(
        "INSERT INTO \"{}\" ({}) VALUES ({})",
        escape_ident(&p.measurement),
        cols.join(", "),
        vals.join(", ")
    )
}

/// 每批最多写入的语句数，TDengine REST 支持换行分隔的多语句
const BATCH_SIZE: usize = 100;

#[async_trait]
impl TsdbClient for TdengineClient {
    async fn write(&self, points: &[DataPoint]) -> Result<(), Error> {
        for chunk in points.chunks(BATCH_SIZE) {
            let sql = chunk
                .iter()
                .map(point_to_insert)
                .collect::<Vec<_>>()
                .join("\n");
            self.exec(&sql).await?;
        }
        Ok(())
    }

    async fn query(&self, sql: &str) -> Result<serde_json::Value, Error> {
        self.exec(sql).await
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

    /// mock TDengine REST 端点（fallback 捕获任意路径）：捕获请求路径/头/体，
    /// 按给定状态码与响应体应答（body 为空时返回成功 JSON），返回 mock base_url。
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
                Json(serde_json::json!({"code": 0})).into_response()
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
    fn config_deserializes() {
        let cfg: TdengineConfig = serde_json::from_value(serde_json::json!({
            "base_url": "http://localhost:6041",
            "username": "root",
            "password": "taosdata",
            "database": "demo",
        }))
        .unwrap();
        assert_eq!(cfg.database.as_deref(), Some("demo"));
        assert!(cfg.tls.is_none());
    }

    #[test]
    fn client_constructs() {
        let _client = TdengineClient::new("http://localhost:6041", "root", "taosdata");
    }

    #[test]
    fn sql_url_encodes_database_segment() {
        let mut client = TdengineClient::new("http://localhost:6041", "root", "taosdata");
        client.database = Some("my db/1".into());
        assert_eq!(
            client.sql_url(),
            "http://localhost:6041/rest/sql/my%20db%2F1"
        );
    }

    #[tokio::test]
    async fn query_sends_sql_and_parses_result() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 200, "").await;
        let mut client = TdengineClient::new(base_url, "root", "taosdata");
        client.database = Some("demo".into());
        let result = client.query("SELECT * FROM meters").await.unwrap();
        assert_eq!(result["code"], 0);

        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs.len(), 1);
        let r = &reqs[0];
        assert_eq!(r.path, "/rest/sql/demo");
        // base64("root:taosdata")
        assert_eq!(
            r.header("authorization"),
            Some("Basic cm9vdDp0YW9zZGF0YQ==")
        );
        let body: serde_json::Value = serde_json::from_slice(&r.body).unwrap();
        assert_eq!(body["sql"], "SELECT * FROM meters");
    }

    #[tokio::test]
    async fn query_propagates_server_error() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 500, "boom").await;
        let client = TdengineClient::new(base_url, "root", "taosdata");
        let err = client.query("SELECT 1").await.unwrap_err();
        assert!(err.to_string().contains("boom"), "got: {err}");
    }

    #[tokio::test]
    async fn query_non_json_body_returns_parse_error() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 200, "not json").await;
        let client = TdengineClient::new(base_url, "root", "taosdata");
        let err = client.query("SELECT 1").await.unwrap_err();
        assert!(err.to_string().contains("tdengine parse"), "got: {err}");
    }

    #[test]
    fn point_to_insert_flattens_tags_and_fields() {
        let p = DataPoint::new("meters")
            .with_tag("location", "beijing")
            .with_field("voltage", FieldValue::Float(220.5))
            .with_field("current", FieldValue::Int(3))
            .with_field("online", FieldValue::Bool(true))
            .with_field("note", FieldValue::String("ok".into()))
            .with_timestamp(1_700_000_000_000);
        let sql = point_to_insert(&p);
        // tags 为列；timestamp 为数值；字符串列带引号；Float 无后缀；Int 无后缀
        assert!(sql.starts_with("INSERT INTO \"meters\" ("), "got: {sql}");
        assert!(sql.contains("\"location\""), "tag 列缺失: {sql}");
        assert!(sql.contains("\"beijing\""), "tag 值缺失: {sql}");
        assert!(sql.contains("voltage"), "field 列缺失: {sql}");
        assert!(sql.contains("220.5"), "float 值缺失: {sql}");
        assert!(sql.contains("\"ok\""), "字符串值缺失: {sql}");
        assert!(sql.contains("1700000000000"), "时间戳缺失: {sql}");
        assert!(sql.ends_with(")"));

        let online = DataPoint::new("m")
            .with_field("flag", FieldValue::Bool(false))
            .with_field("ratio", FieldValue::Float(0.1))
            .with_field("delta", FieldValue::Int(-5));
        let sql2 = point_to_insert(&online);
        assert!(sql2.contains("false"), "bool false 缺失: {sql2}");
        assert!(sql2.contains("0.1"), "float 缺失: {sql2}");
        assert!(sql2.contains("-5"), "负 int 缺失: {sql2}");
    }

    #[test]
    fn point_to_insert_escapes_quotes_and_backslashes() {
        let p = DataPoint::new("a\"b\\c")
            .with_tag("k\"1", "v\\1")
            .with_field("s", FieldValue::String("say \"hi\" \\ done".into()));
        let sql = point_to_insert(&p);
        // 双引号转义为 \"，反斜杠转义为 \\；注入载荷不得逃出字面量
        assert!(sql.contains("\"a\\\"b\\\\c\""), "表名转义失败: {sql}");
        assert!(sql.contains("\"k\\\"1\""), "tag 键转义失败: {sql}");
        assert!(
            sql.contains("say \\\"hi\\\" \\\\ done"),
            "字符串值转义失败: {sql}"
        );
    }

    #[test]
    fn point_to_insert_missing_timestamp_uses_now() {
        let p = DataPoint::new("m").with_field("v", FieldValue::Int(1));
        let sql = point_to_insert(&p);
        assert!(sql.contains("VALUES (now"), "无时间戳应落 now: {sql}");
    }

    #[test]
    fn point_to_insert_no_tags_or_fields_keeps_ts_column() {
        let p = DataPoint::new("m").with_timestamp(5);
        let sql = point_to_insert(&p);
        assert_eq!(sql, "INSERT INTO \"m\" (ts) VALUES (5)");
    }

    #[tokio::test]
    async fn write_chunks_into_batches_of_100() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock(Arc::clone(&captured), 200, "").await;
        let client = TdengineClient::new(base_url, "root", "taosdata");
        let points: Vec<DataPoint> = (0..250)
            .map(|i| {
                DataPoint::new("m")
                    .with_tag("i", i.to_string())
                    .with_field("v", FieldValue::Int(i))
            })
            .collect();
        client.write(&points).await.unwrap();

        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs.len(), 3, "250 点应分 3 批（100+100+50）");
        for (idx, r) in reqs.iter().enumerate() {
            let body: serde_json::Value = serde_json::from_slice(&r.body).unwrap();
            let sql = body["sql"].as_str().unwrap();
            let lines = sql.lines().count();
            let expected = if idx < 2 { 100 } else { 50 };
            assert_eq!(lines, expected, "第 {} 批行数", idx + 1);
        }
    }
}
