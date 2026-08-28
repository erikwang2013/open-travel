// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_data::{DataPoint, FieldValue, TsdbClient};
use ecat_errors::{Error, ErrorCode};
use ecat_tls::TlsClientConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct IotdbConfig {
    pub base_url: String,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub tls: Option<TlsClientConfig>,
}

pub struct IotdbClient {
    client: reqwest::Client,
    base_url: String,
    username: String,
    password: String,
}

impl IotdbClient {
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

    pub fn from_config(cfg: IotdbConfig) -> Result<Self, Error> {
        let client = ecat_tls::build_reqwest_client(&cfg.tls)
            .map_err(|e| Error::new(ErrorCode::Internal, "iotdb", format!("TLS: {e}")))?;
        Ok(Self {
            client,
            base_url: cfg.base_url,
            username: cfg.username,
            password: cfg.password,
        })
    }
}

#[async_trait]
impl TsdbClient for IotdbClient {
    async fn write(&self, points: &[DataPoint]) -> Result<(), Error> {
        for p in points {
            // Apache IoTDB REST v2 insertTablet body:
            // {"device": "...", "is_aligned": false, "timestamps": [...],
            //  "measurements": [...], "data_types": [...], "values": [[...]]}
            // `device` = measurement; tags are not representable in this API.
            let mut measurements = Vec::with_capacity(p.fields.len());
            let mut data_types = Vec::with_capacity(p.fields.len());
            let mut values: Vec<serde_json::Value> = Vec::with_capacity(p.fields.len());
            for (k, v) in &p.fields {
                measurements.push(k.clone());
                let (dt, val) = match v {
                    FieldValue::Float(f) => (
                        "DOUBLE",
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(*f).unwrap_or(0.into()),
                        ),
                    ),
                    FieldValue::Int(i) => ("INT64", serde_json::Value::Number((*i).into())),
                    FieldValue::String(s) => ("TEXT", serde_json::Value::String(s.clone())),
                    FieldValue::Bool(b) => ("BOOLEAN", serde_json::Value::Bool(*b)),
                };
                data_types.push(dt);
                values.push(val);
            }
            let body = serde_json::json!({
                "device": p.measurement,
                "is_aligned": false,
                "timestamps": [p.timestamp.unwrap_or(0)],
                "measurements": measurements,
                "data_types": data_types,
                "values": [values],
            });
            let resp = self
                .client
                .post(format!("{}/rest/v2/insertTablet", self.base_url))
                .basic_auth(&self.username, Some(&self.password))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| {
                    Error::new(ErrorCode::Internal, "iotdb", format!("iotdb write: {e}"))
                })?;
            if !resp.status().is_success() {
                return Err(Error::new(
                    ErrorCode::Internal,
                    "iotdb",
                    resp.text().await.unwrap_or_default(),
                ));
            }
            // IoTDB REST v2 may return HTTP 200 with a body `code` != 200 on
            // some failures; surface those too.
            if let Ok(v) = resp.json::<serde_json::Value>().await
                && let Some(code) = v.get("code").and_then(|c| c.as_i64())
                && code != 200
            {
                return Err(Error::new(
                    ErrorCode::Internal,
                    "iotdb",
                    format!(
                        "iotdb write failed: code {code}: {}",
                        v.get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("no message")
                    ),
                ));
            }
        }
        Ok(())
    }

    async fn query(&self, sql: &str) -> Result<serde_json::Value, Error> {
        let resp = self
            .client
            .post(format!("{}/rest/v2/query", self.base_url))
            .basic_auth(&self.username, Some(&self.password))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(sql.to_string())
            .send()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "iotdb", format!("iotdb query: {e}")))?;
        if !resp.status().is_success() {
            return Err(Error::new(
                ErrorCode::Internal,
                "iotdb",
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "iotdb", format!("iotdb parse: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Request, State};
    use axum::response::{IntoResponse, Response};
    use std::sync::{Arc, Mutex};

    #[test]
    fn client_constructs() {
        let _client = IotdbClient::new("http://localhost:18080", "root", "root");
    }

    #[derive(Debug)]
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

    /// mock IoTDB /rest/v2/insertTablet 端点：捕获请求路径/头/体，按给定
    /// 状态码与响应体应答，返回 mock 的 base_url。
    async fn spawn_mock_insert(
        captured: Arc<Mutex<Vec<CapturedRequest>>>,
        status: u16,
        body: &'static str,
    ) -> String {
        let config = Arc::new(MockConfig {
            captured,
            status,
            body,
        });
        let app = axum::Router::new()
            .route("/rest/v2/insertTablet", axum::routing::post(handle_insert))
            .with_state(config);

        async fn handle_insert(State(config): State<Arc<MockConfig>>, req: Request) -> Response {
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
            let req_body = axum::body::to_bytes(req_body, usize::MAX)
                .await
                .unwrap_or_default();
            config
                .captured
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(CapturedRequest {
                    path,
                    headers,
                    body: req_body.to_vec(),
                });
            if config.body.is_empty() {
                axum::Json(serde_json::json!({"code": 200})).into_response()
            } else {
                (
                    axum::http::StatusCode::from_u16(config.status).unwrap(),
                    axum::response::Response::new(axum::body::Body::from(config.body)),
                )
                    .into_response()
            }
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    struct MockConfig {
        captured: Arc<Mutex<Vec<CapturedRequest>>>,
        status: u16,
        body: &'static str,
    }

    /// 按 measurement 索引对齐断言 fields 构造的三元组
    /// （measurements/data_types/values[0] 来自同一循环，顺序一致但
    /// HashMap 迭代顺序不定，故按名称索引断言）。
    fn assert_field(
        body: &serde_json::Value,
        field: &str,
        expected_type: &str,
        expected_value: serde_json::Value,
    ) {
        let measurements = body["measurements"].as_array().unwrap();
        let idx = measurements
            .iter()
            .position(|m| m.as_str() == Some(field))
            .unwrap_or_else(|| panic!("field {field} missing from {measurements:?}"));
        assert_eq!(
            body["data_types"][idx].as_str(),
            Some(expected_type),
            "type for {field}"
        );
        assert_eq!(body["values"][0][idx], expected_value, "value for {field}");
    }

    #[tokio::test]
    async fn insert_tablet_sends_full_protocol_body() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock_insert(captured.clone(), 200, "").await;
        let client = IotdbClient::new(base_url, "root", "root");

        let point = DataPoint::new("cpu")
            .with_field("usage", FieldValue::Float(0.85))
            .with_field("count", FieldValue::Int(3))
            .with_field("active", FieldValue::Bool(true))
            .with_field("name", FieldValue::String("web".into()))
            .with_timestamp(1_700_000_000_000);
        client.write(&[point]).await.unwrap();

        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].path, "/rest/v2/insertTablet");
        assert_eq!(reqs[0].header("content-type"), Some("application/json"));
        // reqwest basic_auth("root", "root") → base64("root:root")
        assert_eq!(reqs[0].header("authorization"), Some("Basic cm9vdDpyb290"));

        let body: serde_json::Value = serde_json::from_slice(&reqs[0].body).unwrap();
        assert_eq!(body["device"], "cpu");
        assert_eq!(body["is_aligned"], false);
        assert_eq!(
            body["timestamps"],
            serde_json::json!([1_700_000_000_000_i64])
        );
        // values 为 [时间戳] × [字段] 的二维数组，单点单时间戳
        assert_eq!(body["values"].as_array().unwrap().len(), 1);
        // 字段类型编码与取值逐一对齐（HashMap 顺序不定，按名断言）
        assert_field(&body, "usage", "DOUBLE", serde_json::json!(0.85));
        assert_field(&body, "count", "INT64", serde_json::json!(3));
        assert_field(&body, "active", "BOOLEAN", serde_json::json!(true));
        assert_field(&body, "name", "TEXT", serde_json::json!("web"));
    }

    #[tokio::test]
    async fn insert_tablet_defaults_timestamp_to_zero() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock_insert(captured.clone(), 200, "").await;
        let client = IotdbClient::new(base_url, "root", "root");

        client
            .write(&[DataPoint::new("mem").with_field("used", FieldValue::Int(7))])
            .await
            .unwrap();

        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        let body: serde_json::Value = serde_json::from_slice(&reqs[0].body).unwrap();
        assert_eq!(body["timestamps"], serde_json::json!([0]));
    }

    #[tokio::test]
    async fn insert_tablet_sends_one_request_per_point() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock_insert(captured.clone(), 200, "").await;
        let client = IotdbClient::new(base_url, "root", "root");

        let p1 = DataPoint::new("cpu").with_field("usage", FieldValue::Float(0.5));
        let p2 = DataPoint::new("mem").with_field("used", FieldValue::Int(1));
        client.write(&[p1, p2]).await.unwrap();

        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs.len(), 2, "每点独立一次 insertTablet 请求");
        let devices: Vec<String> = reqs
            .iter()
            .map(|r| {
                let body: serde_json::Value = serde_json::from_slice(&r.body).unwrap();
                body["device"].as_str().unwrap().to_string()
            })
            .collect();
        assert!(devices.iter().any(|d| d == "cpu"));
        assert!(devices.iter().any(|d| d == "mem"));
    }

    #[tokio::test]
    async fn write_returns_err_on_http_error() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock_insert(captured.clone(), 500, "boom").await;
        let client = IotdbClient::new(base_url, "root", "root");
        let err = client
            .write(&[DataPoint::new("cpu").with_field("x", FieldValue::Int(1))])
            .await
            .unwrap_err();
        assert!(err.to_string().contains("boom"), "got: {err}");
    }

    #[tokio::test]
    async fn write_returns_err_on_2xx_with_failure_code() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        // IoTDB REST v2 部分失败返回 HTTP 200 + body code != 200
        let base_url = spawn_mock_insert(
            captured.clone(),
            200,
            r#"{"code":501,"message":"table not exists"}"#,
        )
        .await;
        let client = IotdbClient::new(base_url, "root", "root");
        let err = client
            .write(&[DataPoint::new("cpu").with_field("x", FieldValue::Int(1))])
            .await
            .unwrap_err();
        assert!(err.to_string().contains("table not exists"), "got: {err}");
    }

    #[tokio::test]
    async fn write_converts_non_finite_float_to_zero() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_mock_insert(captured.clone(), 200, "").await;
        let client = IotdbClient::new(base_url, "root", "root");
        client
            .write(&[DataPoint::new("cpu").with_field("x", FieldValue::Float(f64::NAN))])
            .await
            .unwrap();
        let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
        let body: serde_json::Value = serde_json::from_slice(&reqs[0].body).unwrap();
        assert_field(&body, "x", "DOUBLE", serde_json::json!(0));
    }

    /// mock IoTDB /rest/v2/query 端点：按给定状态码与响应体应答。
    async fn spawn_mock_query(status: u16, body: &'static str) -> String {
        let app = axum::Router::new().route(
            "/rest/v2/query",
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
    async fn query_parses_successful_json_response() {
        let body = r#"{"code":200,"expression":[{"alias":"x"}],"timestamp":[],"values":[]}"#;
        let base_url = spawn_mock_query(200, body).await;
        let client = IotdbClient::new(base_url, "root", "root");
        let v = client.query("select x from root.s").await.unwrap();
        assert_eq!(v["code"], 200);
        assert_eq!(v["expression"][0]["alias"], "x");
    }

    #[tokio::test]
    async fn query_returns_err_on_http_error() {
        let base_url = spawn_mock_query(500, "query failed").await;
        let client = IotdbClient::new(base_url, "root", "root");
        let err = client.query("select 1").await.unwrap_err();
        assert!(err.to_string().contains("query failed"), "got: {err}");
    }

    #[tokio::test]
    async fn query_non_json_body_returns_parse_error() {
        let base_url = spawn_mock_query(200, "not json").await;
        let client = IotdbClient::new(base_url, "root", "root");
        let err = client.query("select 1").await.unwrap_err();
        assert!(err.to_string().contains("iotdb parse"), "got: {err}");
    }
}
