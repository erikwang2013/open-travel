// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
// 测试模块：lib.rs 的私有项通过 super::* 引用
use super::*;

#[test]
fn client_constructs() {
    let _client = ClickhouseClient::new("http://localhost:8123", "default");
}

#[test]
fn config_with_optional_auth() {
    let cfg: ClickhouseConfig = serde_json::from_str(
        r#"{"base_url":"http://localhost:8123","username":"default","password":"secret"}"#,
    )
    .unwrap();
    let client = ClickhouseClient::from_config(cfg).unwrap();
    assert!(client.username.is_some());
}

#[test]
fn quote_ident_escapes_backticks() {
    assert_eq!(quote_ident("cpu"), "`cpu`");
    assert_eq!(quote_ident("a`b"), "`a``b`");
}

#[test]
fn field_type_maps_variants() {
    assert_eq!(field_type(&FieldValue::Float(1.0)), "Float64");
    assert_eq!(field_type(&FieldValue::Int(1)), "Int64");
    assert_eq!(field_type(&FieldValue::String("s".into())), "String");
    assert_eq!(field_type(&FieldValue::Bool(true)), "UInt8");
}

#[test]
fn build_create_table_sql() {
    let sql = build_create_table(
        "cpu",
        &["host".to_string()],
        &[("usage".to_string(), "Float64")],
    );
    assert_eq!(
        sql,
        "CREATE TABLE IF NOT EXISTS `cpu` (`host` String, `usage` Float64, `timestamp` Int64 DEFAULT 0) ENGINE = MergeTree ORDER BY timestamp"
    );
}

#[test]
fn build_insert_body_serializes_and_escapes() {
    let points = [
        DataPoint::new("cpu")
            .with_tag("host", "a`b,c")
            .with_field("usage", FieldValue::Float(0.5))
            .with_timestamp(100),
        DataPoint::new("cpu")
            .with_field("usage", FieldValue::Int(7))
            .with_timestamp(200),
    ];
    let refs: Vec<&DataPoint> = points.iter().collect();
    let body = build_insert_body(&refs, &["host".to_string()], &["usage".to_string()]);
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], r#"{"host":"a`b,c","usage":0.5,"timestamp":100}"#);
    assert_eq!(lines[1], r#"{"usage":7,"timestamp":200}"#);
}

#[test]
fn field_to_json_non_finite_floats_fall_back_to_zero() {
    assert_eq!(
        field_to_json(&FieldValue::Float(f64::NAN)),
        serde_json::json!(0)
    );
    assert_eq!(
        field_to_json(&FieldValue::Float(f64::INFINITY)),
        serde_json::json!(0)
    );
    assert_eq!(
        field_to_json(&FieldValue::Float(f64::NEG_INFINITY)),
        serde_json::json!(0)
    );
    assert_eq!(
        field_to_json(&FieldValue::Float(0.5)),
        serde_json::json!(0.5)
    );
}

#[test]
fn build_insert_body_omits_timestamp_key_when_missing() {
    let points = [DataPoint::new("cpu")
        .with_tag("host", "h1")
        .with_field("usage", FieldValue::Float(0.5))];
    let refs: Vec<&DataPoint> = points.iter().collect();
    let body = build_insert_body(&refs, &["host".to_string()], &["usage".to_string()]);
    assert_eq!(body, "{\"host\":\"h1\",\"usage\":0.5}\n");
}

#[test]
fn build_insert_body_empty_points_is_empty_string() {
    let refs: Vec<&DataPoint> = Vec::new();
    let body = build_insert_body(&refs, &["host".to_string()], &["usage".to_string()]);
    assert_eq!(body, "");
}

#[test]
fn build_insert_body_filters_keys_per_point() {
    // 两个不同 measurement 的点只含各自拥有的键（键过滤按点进行）
    let points = [
        DataPoint::new("cpu")
            .with_tag("host", "a")
            .with_timestamp(1),
        DataPoint::new("mem")
            .with_tag("dc", "x")
            .with_field("used", FieldValue::Int(5)),
    ];
    let refs: Vec<&DataPoint> = points.iter().collect();
    let body = build_insert_body(&refs, &["host".to_string()], &["used".to_string()]);
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], r#"{"host":"a","timestamp":1}"#);
    assert_eq!(lines[1], r#"{"used":5}"#);
}

#[test]
fn config_defaults_database_to_default() {
    let cfg: ClickhouseConfig =
        serde_json::from_str(r#"{"base_url":"http://localhost:8123"}"#).unwrap();
    assert_eq!(cfg.database, "default");
}

#[test]
fn config_missing_base_url_is_error() {
    let result: Result<ClickhouseConfig, _> = serde_json::from_str(r#"{"database":"d"}"#);
    assert!(result.is_err());
}

#[derive(Clone)]
struct ClickCapture {
    path: String,
    query: Vec<(String, String)>,
    body: String,
}

/// mock ClickHouse HTTP 端点（fallback 捕获任意路径）：捕获路径/查询参数/体，
/// 按给定状态码、响应体与响应头应答（body 为空时返回空体 200）。
async fn spawn_mock(
    captured: std::sync::Arc<std::sync::Mutex<Vec<ClickCapture>>>,
    status: u16,
    body: &'static str,
    summary_header: Option<&'static str>,
) -> String {
    let app = axum::Router::new().fallback(
        move |req: axum::http::Request<axum::body::Body>| async move {
            let path = req.uri().path().to_string();
            let (parts, req_body) = req.into_parts();
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
                .push(ClickCapture {
                    path,
                    query,
                    body: String::from_utf8_lossy(&req_body).into_owned(),
                });
            let mut resp = axum::response::Response::new(axum::body::Body::from(body));
            if let Some(summary) = summary_header {
                resp.headers_mut()
                    .insert("x-clickhouse-summary", summary.parse().unwrap());
            }
            *resp.status_mut() = axum::http::StatusCode::from_u16(status).unwrap();
            resp
        },
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn execute_parses_written_rows_from_summary_header() {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let base_url = spawn_mock(
        captured.clone(),
        200,
        "",
        Some(r#"{"written_rows":"42","written_bytes":"123"}"#),
    )
    .await;
    let client = ClickhouseClient::new(base_url, "default");
    let affected = client.execute("INSERT INTO t VALUES (1)").await.unwrap();
    assert_eq!(affected, 42);

    let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
    assert_eq!(reqs[0].path, "/");
    assert!(
        reqs[0]
            .query
            .contains(&("database".into(), "default".into()))
    );
    assert!(
        reqs[0]
            .query
            .contains(&("send_progress_in_http_headers".into(), "1".into()))
    );
    assert_eq!(reqs[0].body, "INSERT INTO t VALUES (1)");
}

#[tokio::test]
async fn execute_falls_back_to_zero_without_summary_header() {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let base_url = spawn_mock(captured.clone(), 200, "", None).await;
    let client = ClickhouseClient::new(base_url, "default");
    assert_eq!(client.execute("SELECT 1").await.unwrap(), 0);
}

#[tokio::test]
async fn execute_propagates_http_error() {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let base_url = spawn_mock(captured.clone(), 500, "table missing", None).await;
    let client = ClickhouseClient::new(base_url, "default");
    let err = client.execute("SELECT 1").await.unwrap_err();
    assert!(err.to_string().contains("table missing"), "got: {err}");
}

#[tokio::test]
async fn query_parses_jsoneachrow_lines_into_rows() {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let body = "{\"id\":1,\"name\":\"alice\"}\n{\"id\":2,\"name\":\"bob\"}\n";
    let base_url = spawn_mock(captured.clone(), 200, body, None).await;
    let client = ClickhouseClient::new(base_url, "default");
    let rows = ecat_data::RdbmsClient::query(&client, "SELECT * FROM t")
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get("id"), Some(&serde_json::json!(1)));
    assert_eq!(rows[0].get("name"), Some(&serde_json::json!("alice")));
    assert_eq!(rows[1].get("id"), Some(&serde_json::json!(2)));

    let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
    assert!(
        reqs[0]
            .query
            .contains(&("default_format".into(), "JSONEachRow".into()))
    );
}

#[tokio::test]
async fn query_empty_body_returns_no_rows() {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let base_url = spawn_mock(captured.clone(), 200, "", None).await;
    let client = ClickhouseClient::new(base_url, "default");
    let rows = ecat_data::RdbmsClient::query(&client, "SELECT * FROM t")
        .await
        .unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn query_unparseable_line_returns_error_with_snippet() {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let base_url = spawn_mock(captured.clone(), 200, "{\"id\":1}\nnot json\n", None).await;
    let client = ClickhouseClient::new(base_url, "default");
    let err = ecat_data::RdbmsClient::query(&client, "SELECT * FROM t")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("unparseable row"), "got: {err}");
}

#[tokio::test]
async fn write_creates_table_once_then_inserts() {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let base_url = spawn_mock(captured.clone(), 200, "", None).await;
    let client = ClickhouseClient::new(base_url, "default");

    let point = DataPoint::new("cpu")
        .with_tag("host", "h1")
        .with_field("usage", FieldValue::Float(0.5))
        .with_timestamp(100);
    client.write(std::slice::from_ref(&point)).await.unwrap();
    // 第二批同 measurement：建表缓存命中，不再发 CREATE
    client.write(std::slice::from_ref(&point)).await.unwrap();

    let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
    assert_eq!(reqs.len(), 3, "第一批 CREATE+INSERT，第二批仅 INSERT");
    assert!(reqs[0].body.starts_with("CREATE TABLE IF NOT EXISTS `cpu`"));
    assert!(reqs[1].body.starts_with("INSERT INTO `cpu`"));
    assert!(reqs[2].body.starts_with("INSERT INTO `cpu`"));
}

#[tokio::test]
async fn write_recreates_table_after_create_ttl_expiry() {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let base_url = spawn_mock(captured.clone(), 200, "", None).await;
    let mut client = ClickhouseClient::new(base_url, "default");
    // 缩短 TTL：默认 60s 无法在测试里等待
    client.create_ttl = std::time::Duration::from_millis(5);

    let point = DataPoint::new("cpu").with_field("u", FieldValue::Float(0.5));
    client.write(std::slice::from_ref(&point)).await.unwrap();
    // TTL 未过期：缓存命中，不再发 CREATE
    client.write(std::slice::from_ref(&point)).await.unwrap();
    // 等 TTL 过期后再写：重新 CREATE + INSERT
    std::thread::sleep(std::time::Duration::from_millis(30));
    client.write(std::slice::from_ref(&point)).await.unwrap();

    let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
    assert_eq!(reqs.len(), 5, "CREATE+INSERT, INSERT, CREATE+INSERT");
    assert!(reqs[0].body.starts_with("CREATE TABLE IF NOT EXISTS `cpu`"));
    assert!(reqs[1].body.starts_with("INSERT INTO `cpu`"));
    assert!(reqs[2].body.starts_with("INSERT INTO `cpu`"));
    assert!(reqs[3].body.starts_with("CREATE TABLE IF NOT EXISTS `cpu`"));
    assert!(reqs[4].body.starts_with("INSERT INTO `cpu`"));
}

#[tokio::test]
async fn write_recreates_table_when_dropped_externally() {
    // 有状态 mock：第一次 INSERT 报 "表不存在"，之后全部成功
    let bodies = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let first_insert = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let mock_bodies = std::sync::Arc::clone(&bodies);
    let mock_first_insert = std::sync::Arc::clone(&first_insert);
    let app = axum::Router::new().fallback(
        move |req: axum::http::Request<axum::body::Body>| async move {
            let (_, req_body) = req.into_parts();
            let req_body = axum::body::to_bytes(req_body, usize::MAX)
                .await
                .unwrap_or_default();
            let body = String::from_utf8_lossy(&req_body).into_owned();
            mock_bodies
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(body.clone());
            if body.starts_with("INSERT INTO")
                && mock_first_insert.swap(false, std::sync::atomic::Ordering::SeqCst)
            {
                use axum::response::IntoResponse;
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Table default.cpu doesn't exist",
                )
                    .into_response()
            } else {
                axum::response::Response::new(axum::body::Body::from(""))
            }
        },
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = ClickhouseClient::new(format!("http://{addr}"), "default");
    let point = DataPoint::new("cpu").with_field("u", FieldValue::Float(0.5));
    client.write(std::slice::from_ref(&point)).await.unwrap();

    let bs = bodies.lock().unwrap_or_else(|e| e.into_inner());
    assert_eq!(
        bs.len(),
        4,
        "CREATE + INSERT(缺表) + CREATE + INSERT(重试): {bs:?}"
    );
    assert!(bs[0].starts_with("CREATE TABLE IF NOT EXISTS `cpu`"));
    assert!(bs[1].starts_with("INSERT INTO `cpu`"));
    assert!(bs[2].starts_with("CREATE TABLE IF NOT EXISTS `cpu`"));
    assert!(bs[3].starts_with("INSERT INTO `cpu`"));
}

#[tokio::test]
async fn write_mixed_measurements_group_into_separate_tables() {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let base_url = spawn_mock(captured.clone(), 200, "", None).await;
    let client = ClickhouseClient::new(base_url, "default");

    let points = [
        DataPoint::new("cpu").with_field("u", FieldValue::Float(0.1)),
        DataPoint::new("mem").with_field("u", FieldValue::Int(1)),
        DataPoint::new("cpu").with_field("u", FieldValue::Float(0.2)),
    ];
    client.write(&points).await.unwrap();

    let reqs = captured.lock().unwrap_or_else(|e| e.into_inner());
    assert_eq!(reqs.len(), 4, "cpu CREATE+INSERT, mem CREATE+INSERT");
    assert!(reqs[0].body.contains("`cpu`"));
    assert!(reqs[1].body.contains("`cpu`"));
    assert!(reqs[2].body.contains("`mem`"));
    assert!(reqs[3].body.contains("`mem`"));
}

#[tokio::test]
async fn write_propagates_create_error() {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let base_url = spawn_mock(captured.clone(), 500, "create failed", None).await;
    let client = ClickhouseClient::new(base_url, "default");
    let err = client
        .write(&[DataPoint::new("cpu").with_field("u", FieldValue::Float(0.1))])
        .await
        .unwrap_err();
    assert!(err.to_string().contains("create failed"), "got: {err}");
}

#[tokio::test]
async fn tsdb_query_returns_json_rows_array() {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let body = "{\"u\":1}\n{\"u\":2}\n";
    let base_url = spawn_mock(captured.clone(), 200, body, None).await;
    let client = ClickhouseClient::new(base_url, "default");
    let v = ecat_data::TsdbClient::query(&client, "SELECT u FROM t")
        .await
        .unwrap();
    assert_eq!(v, serde_json::json!([{"u": 1}, {"u": 2}]));
}
