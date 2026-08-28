// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
//! InfluxDB 2.x client (line protocol writer + Flux query).
//!
//! Measurement、tag key/value、field key 按 line protocol 转义（`,`、` `、
//! `=`、`\`）；字符串 field 值只需转义 `"` 和 `\`（引号内的逗号与空格
//! 合法）。tag/field 输出按 key 排序，保证行协议确定性。

use async_trait::async_trait;
use ecat_data::{DataPoint, FieldValue, TsdbClient};
use ecat_errors::{Error, ErrorCode};
use ecat_tls::TlsClientConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct InfluxConfig {
    pub base_url: String,
    pub org: String,
    pub bucket: String,
    pub token: String,
    #[serde(default)]
    pub tls: Option<TlsClientConfig>,
}

pub struct InfluxClient {
    client: reqwest::Client,
    write_url: String,
    query_url: String,
    org: String,
    bucket: String,
    token: String,
}

impl InfluxClient {
    pub fn new(
        base_url: impl Into<String>,
        org: impl Into<String>,
        bucket: impl Into<String>,
        token: impl Into<String>,
    ) -> Self {
        let base = base_url.into();
        Self {
            write_url: format!("{base}/api/v2/write"),
            query_url: format!("{base}/api/v2/query"),
            org: org.into(),
            bucket: bucket.into(),
            token: token.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn from_config(cfg: InfluxConfig) -> Result<Self, Error> {
        let base = cfg.base_url.clone();
        let client = ecat_tls::build_reqwest_client(&cfg.tls)
            .map_err(|e| Error::new(ErrorCode::Internal, "influx", format!("TLS: {e}")))?;
        Ok(Self {
            write_url: format!("{base}/api/v2/write"),
            query_url: format!("{base}/api/v2/query"),
            org: cfg.org,
            bucket: cfg.bucket,
            token: cfg.token,
            client,
        })
    }
}

/// Escape a measurement, tag key/value or field key per InfluxDB line
/// protocol: backslash, comma, space and `=` must be escaped in these
/// unquoted parts of a line.
fn escape_line_part(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | ',' | ' ' | '=' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Escape a string field value per InfluxDB line protocol: only backslash
/// and double quote are required inside the quoted value; comma and space
/// are legal verbatim there.
fn escape_field_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | '"' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

#[async_trait]
impl TsdbClient for InfluxClient {
    async fn write(&self, points: &[DataPoint]) -> Result<(), Error> {
        let mut lines = String::new();
        for p in points {
            // 经 BTreeMap 排序后输出：tag/field 顺序确定，行协议可复现
            let tags: String = if p.tags.is_empty() {
                String::new()
            } else {
                p.tags
                    .iter()
                    .collect::<std::collections::BTreeMap<_, _>>()
                    .iter()
                    .map(|(k, v)| format!(",{}={}", escape_line_part(k), escape_line_part(v)))
                    .collect::<Vec<_>>()
                    .join("")
            };
            let fields: String = p
                .fields
                .iter()
                .collect::<std::collections::BTreeMap<_, _>>()
                .iter()
                .map(|(k, v)| {
                    let k = escape_line_part(k);
                    match v {
                        FieldValue::Float(f) => format!("{k}={f}"),
                        FieldValue::Int(i) => format!("{k}={i}i"),
                        FieldValue::String(s) => format!("{k}=\"{}\"", escape_field_string(s)),
                        FieldValue::Bool(b) => format!("{k}={b}"),
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
            lines.push_str(&format!(
                "{}{tags} {fields}",
                escape_line_part(&p.measurement)
            ));
            if let Some(ts) = p.timestamp {
                lines.push_str(&format!(" {ts}"));
            }
            lines.push('\n');
        }

        let resp = self
            .client
            .post(&self.write_url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Content-Type", "text/plain; charset=utf-8")
            .query(&[
                ("org", &self.org),
                ("bucket", &self.bucket),
                ("precision", &"ns".to_string()),
            ])
            .body(lines)
            .send()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "influx", format!("write: {e}")))?;

        if !resp.status().is_success() {
            return Err(Error::new(
                ErrorCode::Internal,
                "influx",
                format!("write failed: {}", resp.text().await.unwrap_or_default()),
            ));
        }
        Ok(())
    }

    async fn query(&self, query: &str) -> Result<serde_json::Value, Error> {
        let resp = self
            .client
            .post(&self.query_url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Content-Type", "application/vnd.flux")
            .query(&[("org", &self.org)])
            .body(query.to_string())
            .send()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "influx", format!("query: {e}")))?;
        if !resp.status().is_success() {
            return Err(Error::new(
                ErrorCode::Internal,
                "influx",
                format!("query failed: {}", resp.text().await.unwrap_or_default()),
            ));
        }
        resp.json()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "influx", format!("parse: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_constructs() {
        let _client = InfluxClient::new("http://localhost:8086", "myorg", "mybucket", "mytoken");
    }

    #[test]
    fn data_point_builder() {
        let p = DataPoint::new("cpu")
            .with_tag("host", "server01")
            .with_field("usage", FieldValue::Float(0.85))
            .with_timestamp(1625097600000000000);
        assert_eq!(p.measurement, "cpu");
        assert_eq!(p.tags.get("host").unwrap(), "server01");
    }

    #[test]
    fn escapes_line_parts() {
        assert_eq!(escape_line_part("a,b c=d\\e"), "a\\,b\\ c\\=d\\\\e");
        assert_eq!(escape_line_part("plain"), "plain");
    }

    #[test]
    fn escapes_field_strings() {
        // 引号与反斜杠转义；空格、逗号在引号值内原样保留（line protocol 规范）
        assert_eq!(escape_field_string("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_field_string("a\\b"), "a\\\\b");
        assert_eq!(escape_field_string("x y,z"), "x y,z");
    }

    /// mock InfluxDB 的 /api/v2/query 端点，返回给定状态码与错误体。
    async fn spawn_mock_query(status: u16, body: &'static str) -> String {
        let app = axum::Router::new().route(
            "/api/v2/query",
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
        let base_url = spawn_mock_query(400, r#"{"error":"invalid flux"}"#).await;
        let client = InfluxClient::new(base_url, "org", "bucket", "token");
        let err = client.query("from(bucket: \"x\")").await.unwrap_err();
        assert!(err.to_string().contains("invalid flux"));
    }

    #[derive(Clone)]
    struct WriteCapture {
        path: String,
        headers: Vec<(String, String)>,
        query: Vec<(String, String)>,
        body: String,
    }

    impl WriteCapture {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.as_str())
        }
    }

    /// mock InfluxDB 的 /api/v2/write 端点：捕获请求路径/头/查询参数/体，
    /// 按给定状态码与错误体应答。
    async fn spawn_mock_write(
        captured: std::sync::Arc<std::sync::Mutex<Vec<WriteCapture>>>,
        status: u16,
        body: &'static str,
    ) -> String {
        let app = axum::Router::new().route(
            "/api/v2/write",
            axum::routing::post(
                move |req: axum::http::Request<axum::body::Body>| async move {
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
                    let query: Vec<(String, String)> = parts
                        .uri
                        .query()
                        .map(|q| {
                            q.split('&')
                                .filter_map(|kv| {
                                    let (k, v) = kv.split_once('=')?;
                                    Some((k.to_string(), v.to_string()))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    let req_body = axum::body::to_bytes(req_body, usize::MAX)
                        .await
                        .unwrap_or_default();
                    captured
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .push(WriteCapture {
                            path,
                            headers,
                            query,
                            body: String::from_utf8_lossy(&req_body).into_owned(),
                        });
                    if status == 200 {
                        axum::response::Response::new(axum::body::Body::from(""))
                    } else {
                        use axum::response::IntoResponse;
                        (
                            axum::http::StatusCode::from_u16(status).unwrap(),
                            axum::response::Response::new(axum::body::Body::from(body)),
                        )
                            .into_response()
                    }
                },
            ),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn write_builds_line_protocol_with_escaping() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let base_url = spawn_mock_write(captured.clone(), 200, "").await;
        let client = InfluxClient::new(base_url, "myorg", "mybucket", "mytoken");

        let point = DataPoint::new("cpu load")
            .with_tag("host name", "srv,1")
            .with_tag("env", "prod")
            .with_field("usage", FieldValue::Float(0.85))
            .with_field("count", FieldValue::Int(3))
            .with_field("note", FieldValue::String("say \"hi\"".into()))
            .with_field("up", FieldValue::Bool(true))
            .with_timestamp(1_700_000_000_000);
        client.write(&[point]).await.unwrap();

        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].path, "/api/v2/write");
        assert_eq!(
            reqs[0].header("authorization"),
            Some("Token mytoken"),
            "Token 认证头缺失"
        );
        // 查询参数：org/bucket/precision=ns
        let qs = &reqs[0].query;
        assert!(qs.contains(&("org".into(), "myorg".into())), "{qs:?}");
        assert!(qs.contains(&("bucket".into(), "mybucket".into())), "{qs:?}");
        assert!(qs.contains(&("precision".into(), "ns".into())), "{qs:?}");

        // 行协议：measurement 转义空格，tag 转义逗号/空格，字符串值只转义引号
        // （引号内空格合法）；tag/field 按 key 排序（BTreeMap），整行输出确定。
        assert_eq!(
            reqs[0].body,
            "cpu\\ load,env=prod,host\\ name=srv\\,1 count=3i,note=\"say \\\"hi\\\"\",up=true,usage=0.85 1700000000000\n"
        );
    }

    #[tokio::test]
    async fn write_sends_multiple_points_as_multiple_lines() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let base_url = spawn_mock_write(captured.clone(), 200, "").await;
        let client = InfluxClient::new(base_url, "org", "bucket", "t");

        let p1 = DataPoint::new("cpu").with_field("u", FieldValue::Float(0.1));
        let p2 = DataPoint::new("mem").with_field("u", FieldValue::Float(0.2));
        client.write(&[p1, p2]).await.unwrap();

        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs.len(), 1);
        let lines: Vec<&str> = reqs[0].body.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("cpu u=0.1"));
        assert!(lines[1].starts_with("mem u=0.2"));
    }

    #[tokio::test]
    async fn write_propagates_server_error() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let base_url = spawn_mock_write(captured.clone(), 400, "line parse error").await;
        let client = InfluxClient::new(base_url, "org", "bucket", "t");
        let err = client
            .write(&[DataPoint::new("cpu").with_field("u", FieldValue::Float(1.0))])
            .await
            .unwrap_err();
        assert!(err.to_string().contains("line parse error"), "got: {err}");
    }

    #[tokio::test]
    async fn query_parses_successful_json_response() {
        let body = r#"{"results":[{"series":[{"name":"cpu"}]}]}"#;
        let base_url = spawn_mock_query(200, body).await;
        let client = InfluxClient::new(base_url, "org", "bucket", "token");
        let v = client.query("from(bucket: \"x\")").await.unwrap();
        assert_eq!(v["results"][0]["series"][0]["name"], "cpu");
    }

    #[tokio::test]
    async fn write_without_timestamp_omits_ts_suffix() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let base_url = spawn_mock_write(captured.clone(), 200, "").await;
        let client = InfluxClient::new(base_url, "org", "bucket", "t");
        client
            .write(&[DataPoint::new("cpu").with_field("u", FieldValue::Int(1))])
            .await
            .unwrap();
        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs[0].body, "cpu u=1i\n");
    }
}
