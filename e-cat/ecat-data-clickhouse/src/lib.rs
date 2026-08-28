// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_data::{DataPoint, FieldValue, RdbmsClient, RdbmsError, Row, TsdbClient};
use ecat_errors::{Error, ErrorCode};
use ecat_tls::TlsClientConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ClickhouseConfig {
    pub base_url: String,
    #[serde(default = "default_database")]
    pub database: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub tls: Option<TlsClientConfig>,
}

fn default_database() -> String {
    "default".into()
}

/// 建表缓存 TTL：外部 drop/改表后，超过 TTL 的下一次写入会重新 CREATE。
const CREATE_TTL: std::time::Duration = std::time::Duration::from_secs(60);

pub struct ClickhouseClient {
    client: reqwest::Client,
    base_url: String,
    database: String,
    username: Option<String>,
    password: Option<String>,
    // 建表缓存：每 client 一份，避免跨 client/database 误跳过建表；
    // 记录建表时间，超过 create_ttl 后重新 CREATE（CREATE IF NOT EXISTS 幂等）。
    created: std::sync::Mutex<std::collections::HashMap<String, std::time::Instant>>,
    create_ttl: std::time::Duration,
}

impl ClickhouseClient {
    pub fn new(base_url: impl Into<String>, database: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            database: database.into(),
            username: None,
            password: None,
            created: std::sync::Mutex::new(std::collections::HashMap::new()),
            create_ttl: CREATE_TTL,
        }
    }

    pub fn with_auth(
        base_url: impl Into<String>,
        database: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            database: database.into(),
            username: Some(username.into()),
            password: Some(password.into()),
            created: std::sync::Mutex::new(std::collections::HashMap::new()),
            create_ttl: CREATE_TTL,
        }
    }

    pub fn from_config(cfg: ClickhouseConfig) -> Result<Self, RdbmsError> {
        let client = ecat_tls::build_reqwest_client(&cfg.tls)
            .map_err(|e| RdbmsError::Config(format!("TLS: {e}")))?;
        Ok(Self {
            client,
            base_url: cfg.base_url,
            database: cfg.database,
            username: cfg.username,
            password: cfg.password,
            created: std::sync::Mutex::new(std::collections::HashMap::new()),
            create_ttl: CREATE_TTL,
        })
    }

    /// 建表缓存缺失或已过期（需要重新 CREATE）。
    fn table_needs_create(&self, measurement: &str) -> bool {
        let created = self.created.lock().unwrap_or_else(|e| e.into_inner());
        match created.get(measurement) {
            Some(at) => at.elapsed() >= self.create_ttl,
            None => true,
        }
    }

    /// 执行 CREATE TABLE IF NOT EXISTS 并刷新缓存；失败不缓存，下次调用重试。
    async fn create_table(
        &self,
        measurement: &str,
        tag_keys: &[String],
        field_cols: &[(String, &'static str)],
    ) -> Result<(), Error> {
        let create = build_create_table(measurement, tag_keys, field_cols);
        let resp = self.post(&create, &[]).send().await.map_err(|e| {
            Error::new(ErrorCode::Internal, "clickhouse", format!("ch create: {e}"))
        })?;
        if !resp.status().is_success() {
            return Err(Error::new(
                ErrorCode::Internal,
                "clickhouse",
                format!(
                    "ch create failed: {}",
                    resp.text().await.unwrap_or_default()
                ),
            ));
        }
        self.created
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(measurement.to_string(), std::time::Instant::now());
        Ok(())
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        ecat_tls::apply_basic_auth(req, &self.username, &self.password)
    }

    fn post(&self, sql: &str, params: &[(&str, String)]) -> reqwest::RequestBuilder {
        let mut rb = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "text/plain; charset=utf-8")
            .query(&[("database", self.database.clone())])
            .body(sql.to_string());
        for (k, v) in params {
            rb = rb.query(&[(*k, v.clone())]);
        }
        self.apply_auth(rb)
    }
}

fn quote_ident(s: &str) -> String {
    format!("`{}`", s.replace('`', "``"))
}

fn field_type(v: &FieldValue) -> &'static str {
    match v {
        FieldValue::Float(_) => "Float64",
        FieldValue::Int(_) => "Int64",
        FieldValue::String(_) => "String",
        FieldValue::Bool(_) => "UInt8",
    }
}

fn field_to_json(v: &FieldValue) -> serde_json::Value {
    match v {
        // 非有限浮点（NaN/±Inf）无法用 JSON number 表示：serde_json 的 from_f64
        // 对非有限值返回 None，且 ClickHouse 的 JSONEachRow 也没有 NaN/Inf 字面量。
        // 因此回退为 0 保证序列化不失败；调用方应在写入前自行清洗 NaN/Inf。
        FieldValue::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Number(0.into())),
        FieldValue::Int(i) => serde_json::Value::Number((*i).into()),
        FieldValue::String(s) => serde_json::Value::String(s.clone()),
        FieldValue::Bool(b) => serde_json::Value::Bool(*b),
    }
}

fn build_create_table(
    measurement: &str,
    tag_keys: &[String],
    field_cols: &[(String, &'static str)],
) -> String {
    let mut cols: Vec<String> = tag_keys
        .iter()
        .map(|k| format!("{} String", quote_ident(k)))
        .collect();
    cols.extend(
        field_cols
            .iter()
            .map(|(k, ty)| format!("{} {ty}", quote_ident(k))),
    );
    cols.push("`timestamp` Int64 DEFAULT 0".into());
    format!(
        "CREATE TABLE IF NOT EXISTS {} ({}) ENGINE = MergeTree ORDER BY timestamp",
        quote_ident(measurement),
        cols.join(", ")
    )
}

fn build_insert_body(points: &[&DataPoint], tag_keys: &[String], field_keys: &[String]) -> String {
    // 逐点按 tags → fields → timestamp 顺序手写 JSON 对象，保证输出列序与
    // INSERT 语句列序一致（serde_json 的 Map 默认按 key 排序，不依赖其 preserve_order 特性）。
    // 键的引号序列化与列序无关且全批一致，预计算一次跨点复用。
    let tag_quoted: Vec<(String, String)> = tag_keys
        .iter()
        .map(|k| (k.clone(), serde_json::to_string(k).unwrap()))
        .collect();
    let field_quoted: Vec<(String, String)> = field_keys
        .iter()
        .map(|k| (k.clone(), serde_json::to_string(k).unwrap()))
        .collect();
    let ts_quoted = serde_json::to_string("timestamp").unwrap();
    let mut out = String::new();
    for p in points {
        let mut parts: Vec<String> = Vec::new();
        for (k, qk) in &tag_quoted {
            if let Some(v) = p.tags.get(k) {
                parts.push(format!("{qk}:{}", serde_json::to_string(v).unwrap()));
            }
        }
        for (k, qk) in &field_quoted {
            if let Some(v) = p.fields.get(k) {
                parts.push(format!(
                    "{qk}:{}",
                    serde_json::to_string(&field_to_json(v)).unwrap()
                ));
            }
        }
        if let Some(ts) = p.timestamp {
            parts.push(format!("{ts_quoted}:{ts}"));
        }
        out.push('{');
        out.push_str(&parts.join(","));
        out.push_str("}\n");
    }
    out
}

#[async_trait]
impl RdbmsClient for ClickhouseClient {
    async fn execute(&self, sql: &str) -> Result<u64, RdbmsError> {
        let resp = self
            .post(sql, &[("send_progress_in_http_headers", "1".to_string())])
            .send()
            .await
            .map_err(|e| RdbmsError::Database(format!("ch: {e}")))?;
        if !resp.status().is_success() {
            return Err(RdbmsError::Database(resp.text().await.unwrap_or_default()));
        }
        // ClickHouse reports written/result rows in the X-ClickHouse-Summary
        // response header (enabled via send_progress_in_http_headers=1).
        // Falls back to 0 when the server does not send the header.
        let affected = resp
            .headers()
            .get("x-clickhouse-summary")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| {
                v.get("written_rows")
                    .and_then(|n| n.as_str())
                    .map(String::from)
            })
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        Ok(affected)
    }

    async fn query(&self, sql: &str) -> Result<Vec<Row>, RdbmsError> {
        let resp = self
            .post(sql, &[("default_format", "JSONEachRow".to_string())])
            .send()
            .await
            .map_err(|e| RdbmsError::Database(format!("ch query: {e}")))?;
        let text = resp
            .text()
            .await
            .map_err(|e| RdbmsError::Database(format!("ch read: {e}")))?;
        let mut rows = Vec::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let v = serde_json::from_str::<serde_json::Value>(line).map_err(|e| {
                RdbmsError::Database(format!(
                    "ch query: unparseable row (first 200 bytes shown): {e}: {}",
                    line.chars().take(200).collect::<String>()
                ))
            })?;
            if let Some(obj) = v.as_object() {
                let cols: Vec<String> = obj.keys().cloned().collect();
                let vals: Vec<serde_json::Value> = obj.values().cloned().collect();
                rows.push(Row::new(cols, vals));
            }
        }
        Ok(rows)
    }

    async fn transaction(&self) -> Result<ecat_data::Transaction, RdbmsError> {
        Err(RdbmsError::Database(
            "ClickHouse does not support transactions".into(),
        ))
    }
}

#[async_trait]
impl TsdbClient for ClickhouseClient {
    async fn write(&self, points: &[DataPoint]) -> Result<(), Error> {
        // 按 measurement 分组，保持首见顺序；分组只存引用，避免克隆整点
        let mut order: Vec<&str> = Vec::new();
        let mut groups: std::collections::HashMap<&str, Vec<&DataPoint>> =
            std::collections::HashMap::new();
        for p in points {
            if !groups.contains_key(p.measurement.as_str()) {
                order.push(&p.measurement);
            }
            groups.entry(&p.measurement).or_default().push(p);
        }

        for measurement in order {
            let pts = &groups[&measurement];
            // 列集合取本批全部点；同名 field 类型不一致时先见者胜（文档注明）
            let tag_keys: Vec<String> = pts
                .iter()
                .flat_map(|p| p.tags.keys().cloned())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            let field_cols: Vec<(String, &'static str)> = {
                let mut m: std::collections::BTreeMap<String, &'static str> =
                    std::collections::BTreeMap::new();
                for p in pts {
                    for (k, v) in &p.fields {
                        m.entry(k.clone()).or_insert_with(|| field_type(v));
                    }
                }
                m.into_iter().collect()
            };
            let field_keys: Vec<String> = field_cols.iter().map(|(k, _)| k.clone()).collect();

            // 建表（按 client 缓存 + TTL；CREATE IF NOT EXISTS 幂等）。
            // 列类型由首批点的字段类型决定并固定；后续批次若出现同名不同型的字段，
            // ClickHouse 不会自动 ALTER 列，写入会以服务端错误失败（调用方需保证类型一致）。
            if self.table_needs_create(measurement) {
                self.create_table(measurement, &tag_keys, &field_cols)
                    .await?;
            }

            let body = build_insert_body(pts, &tag_keys, &field_keys);
            let cols: Vec<String> = tag_keys
                .iter()
                .chain(field_keys.iter())
                .chain(std::iter::once(&"timestamp".to_string()))
                .map(|c| quote_ident(c))
                .collect();
            // JSONEachRow 数据随请求体放在语句之后（ClickHouse HTTP 接口标准用法）
            let insert = format!(
                "INSERT INTO {} ({}) FORMAT JSONEachRow\n{}",
                quote_ident(measurement),
                cols.join(", "),
                body
            );
            let resp = self.post(&insert, &[]).send().await.map_err(|e| {
                Error::new(ErrorCode::Internal, "clickhouse", format!("ch write: {e}"))
            })?;
            if !resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default();
                // 表被外部 drop/改表：清缓存重新建表后重试一次
                if text.contains("doesn't exist") || text.contains("Unknown table") {
                    self.created
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .remove(measurement);
                    self.create_table(measurement, &tag_keys, &field_cols)
                        .await?;
                    let resp = self.post(&insert, &[]).send().await.map_err(|e| {
                        Error::new(ErrorCode::Internal, "clickhouse", format!("ch write: {e}"))
                    })?;
                    if !resp.status().is_success() {
                        return Err(Error::new(
                            ErrorCode::Internal,
                            "clickhouse",
                            format!("ch write failed: {}", resp.text().await.unwrap_or_default()),
                        ));
                    }
                } else {
                    return Err(Error::new(
                        ErrorCode::Internal,
                        "clickhouse",
                        format!("ch write failed: {text}"),
                    ));
                }
            }
        }
        Ok(())
    }

    async fn query(&self, query: &str) -> Result<serde_json::Value, Error> {
        let resp = self
            .post(query, &[("default_format", "JSONEachRow".to_string())])
            .send()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "clickhouse", format!("ch query: {e}")))?;
        if !resp.status().is_success() {
            return Err(Error::new(
                ErrorCode::Internal,
                "clickhouse",
                format!("ch query failed: {}", resp.text().await.unwrap_or_default()),
            ));
        }
        let text = resp
            .text()
            .await
            .map_err(|e| Error::new(ErrorCode::Internal, "clickhouse", format!("ch read: {e}")))?;
        let mut rows = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let v: serde_json::Value = serde_json::from_str(line).map_err(|e| {
                Error::new(
                    ErrorCode::Internal,
                    "clickhouse",
                    format!("ch query parse: {e}"),
                )
            })?;
            rows.push(v);
        }
        Ok(serde_json::json!(rows))
    }

    async fn delete(&self, query: &str) -> Result<(), Error> {
        // ClickHouse 轻量删除语法：ALTER TABLE <t> DELETE WHERE ...
        let resp = self.post(query, &[]).send().await.map_err(|e| {
            Error::new(ErrorCode::Internal, "clickhouse", format!("ch delete: {e}"))
        })?;
        if !resp.status().is_success() {
            return Err(Error::new(
                ErrorCode::Internal,
                "clickhouse",
                format!(
                    "ch delete failed: {}",
                    resp.text().await.unwrap_or_default()
                ),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
