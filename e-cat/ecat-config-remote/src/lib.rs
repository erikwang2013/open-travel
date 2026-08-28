// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use base64::Engine as _;
use ecat_config::{ConfigError, ConfigSource};
use std::collections::HashMap;
use std::time::Duration;

/// 阻塞查询 wait 参数：服务端最长持有连接等待变更的时长。
const CONSUL_BLOCK_WAIT: &str = "5m";
/// 请求超时秒数：须大于 wait（300s），预留 30s 余量。
const REQUEST_TIMEOUT_SECS: u64 = 330;
/// watch 出错后的重试间隔秒数。
const WATCH_RETRY_DELAY_SECS: u64 = 1;
/// watch mpsc channel 容量。
const WATCH_CHANNEL_CAPACITY: usize = 8;

#[derive(Clone)]
pub struct ConsulConfigSource {
    client: reqwest::Client,
    base_url: String,
    prefix: String,
    token: Option<String>,
}

impl ConsulConfigSource {
    pub fn new(base_url: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            prefix: prefix.into(),
            token: None,
        }
    }

    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// 拉取 KV。index 为 Some 时使用 Consul 阻塞查询（index + wait=5m），
    /// 返回 (配置 map, 新 X-Consul-Index)。
    async fn fetch(
        &self,
        index: Option<&str>,
    ) -> Result<(HashMap<String, serde_json::Value>, Option<String>), ConfigError> {
        let url = format!("{}/v1/kv/{}", self.base_url, self.prefix);
        let mut builder = self.client.get(&url).query(&[("recurse", "true")]);
        if let Some(idx) = index {
            builder = builder.query(&[("index", idx), ("wait", CONSUL_BLOCK_WAIT)]);
        }
        builder = builder.timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS));
        if let Some(ref token) = self.token {
            builder = builder.header("X-Consul-Token", token);
        }

        let resp = builder
            .send()
            .await
            .map_err(|e| ConfigError::Other(format!("consul kv: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ConfigError::Other(format!("consul kv failed: {body}")));
        }

        let new_index = resp
            .headers()
            .get("x-consul-index")
            .and_then(|h| h.to_str().ok())
            .map(str::to_string);

        // 阻塞查询响应必须携带 X-Consul-Index；缺失时视为错误，
        // 避免 index 恒为 Some 而 new_index 恒为 None 导致的重复推送/忙等。
        if index.is_some() && new_index.is_none() {
            return Err(ConfigError::Other(
                "consul kv: blocking query response missing X-Consul-Index".to_string(),
            ));
        }

        let entries: Vec<ConsulKvEntry> = resp
            .json()
            .await
            .map_err(|e| ConfigError::Other(format!("consul parse: {e}")))?;

        let mut map = HashMap::new();
        for entry in entries {
            let key = entry
                .key
                .strip_prefix(&self.prefix)
                .unwrap_or(&entry.key)
                .trim_matches('/')
                .replace('/', ".");
            if let Some(decoded) = entry.decoded_value() {
                if let Ok(v) = serde_json::from_str(&decoded) {
                    map.insert(key, v);
                } else {
                    map.insert(key, serde_json::Value::String(decoded));
                }
            }
        }

        Ok((map, new_index))
    }

    /// 启动 Consul 阻塞查询 watch，配置变更通过 mpsc channel 推送。
    /// receiver 被 drop 后后台任务自动退出。
    pub fn watch(
        &self,
    ) -> tokio::sync::mpsc::Receiver<Result<HashMap<String, serde_json::Value>, ConfigError>> {
        let (tx, rx) = tokio::sync::mpsc::channel(WATCH_CHANNEL_CAPACITY);
        let source = self.clone();
        tokio::spawn(async move {
            let mut index: Option<String> = None;
            loop {
                match source.fetch(index.as_deref()).await {
                    Ok((map, new_index)) => {
                        // 首帧（index=None）强制推送：兼容缺失 X-Consul-Index 的服务器，
                        // 否则 None != None 恒为 false 会丢首帧。
                        if (index.as_deref() != new_index.as_deref() || index.is_none())
                            && tx.send(Ok(map)).await.is_err()
                        {
                            break; // receiver dropped
                        }
                        index = new_index;
                        // 服务器不发送 X-Consul-Index 时无法做阻塞查询：fetch 立即返回
                        // 且 index 恒为 None，不加退避就会高速重复推送（紧循环）。
                        if index.is_none() {
                            tokio::time::sleep(Duration::from_secs(WATCH_RETRY_DELAY_SECS)).await;
                        }
                    }
                    Err(e) => {
                        if tx.send(Err(e)).await.is_err() {
                            break;
                        }
                        tokio::time::sleep(Duration::from_secs(WATCH_RETRY_DELAY_SECS)).await;
                    }
                }
            }
        });
        rx
    }
}

#[async_trait]
impl ConfigSource for ConsulConfigSource {
    async fn load(&self) -> Result<HashMap<String, serde_json::Value>, ConfigError> {
        Ok(self.fetch(None).await?.0)
    }
}

#[derive(Debug, serde::Deserialize)]
struct ConsulKvEntry {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Value")]
    value: Option<String>,
}

impl ConsulKvEntry {
    fn decoded_value(&self) -> Option<String> {
        self.value.as_ref().and_then(|v| {
            base64::engine::general_purpose::STANDARD
                .decode(v)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::State;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::watch;

    /// mock Consul 的 watch 状态：(index, KV entries)
    type MockConsulState = (u64, Vec<(String, String)>);

    #[test]
    fn consul_source_constructs() {
        let _src = ConsulConfigSource::new("http://consul:8500", "app/config").token("secret");
    }

    #[test]
    fn base64_decode_simple() {
        let result = base64::engine::general_purpose::STANDARD
            .decode("aGVsbG8=")
            .unwrap();
        assert_eq!(String::from_utf8(result).unwrap(), "hello");
    }

    #[test]
    fn kv_entry_decoded_value_cases() {
        let enc = |s: &str| base64::engine::general_purpose::STANDARD.encode(s);
        let entry = ConsulKvEntry {
            key: "k".into(),
            value: Some(enc("hello")),
        };
        assert_eq!(entry.decoded_value().as_deref(), Some("hello"));

        let entry = ConsulKvEntry {
            key: "k".into(),
            value: None,
        };
        assert_eq!(entry.decoded_value(), None);

        let entry = ConsulKvEntry {
            key: "k".into(),
            value: Some("!!not-base64!!".into()),
        };
        assert_eq!(entry.decoded_value(), None);

        // base64 合法但解码后不是 UTF-8
        let entry = ConsulKvEntry {
            key: "k".into(),
            value: Some(base64::engine::general_purpose::STANDARD.encode(b"\xff\xfe")),
        };
        assert_eq!(entry.decoded_value(), None);
    }

    #[tokio::test]
    async fn fetch_with_index_requires_x_consul_index_header() {
        let base_url = spawn_mock_consul_no_index().await;
        let source = ConsulConfigSource::new(base_url, "app");
        let err = source.fetch(Some("1")).await.unwrap_err();
        assert!(
            err.to_string().contains("missing X-Consul-Index"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn load_nested_keys_become_dotted() {
        let (base_url, tx) = spawn_mock_consul().await;
        tx.send((
            1u64,
            vec![
                ("app/nested/key".to_string(), "{\"a\":1}".to_string()),
                ("app/simple".to_string(), "\"str\"".to_string()),
                ("app/badjson".to_string(), "not-json".to_string()),
            ],
        ))
        .unwrap();
        let source = ConsulConfigSource::new(base_url, "app");
        let map = source.load().await.unwrap();
        assert_eq!(map.get("nested.key"), Some(&serde_json::json!({"a": 1})));
        assert_eq!(map.get("simple"), Some(&serde_json::json!("str")));
        assert_eq!(
            map.get("badjson"),
            Some(&serde_json::Value::String("not-json".into()))
        );
    }

    async fn spawn_mock_consul() -> (String, watch::Sender<MockConsulState>) {
        let (tx, rx) =
            watch::channel((1u64, vec![("app/key".to_string(), "{\"a\":1}".to_string())]));
        let app = axum::Router::new()
            .route("/v1/kv/{prefix}", axum::routing::get(kv_handler))
            .with_state(rx);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{addr}"), tx)
    }

    async fn kv_handler(
        State(mut rx): State<watch::Receiver<MockConsulState>>,
        axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
    ) -> axum::response::Response {
        let requested: u64 = params
            .get("index")
            .and_then(|i| i.parse().ok())
            .unwrap_or(0);
        if params.contains_key("index") {
            // 模拟 Consul 阻塞查询：等待状态变化或 1s 超时后返回
            let deadline = tokio::time::sleep(Duration::from_secs(1));
            tokio::pin!(deadline);
            loop {
                if rx.borrow().0 > requested {
                    break;
                }
                tokio::select! {
                    _ = &mut deadline => break,
                    _ = rx.changed() => {
                        if rx.borrow().0 > requested {
                            break;
                        }
                    }
                }
            }
        }
        let (cur_idx, entries) = rx.borrow().clone();
        kv_response(cur_idx, &entries)
    }

    fn kv_response(cur_idx: u64, entries: &[(String, String)]) -> axum::response::Response {
        let body: Vec<serde_json::Value> = entries
            .iter()
            .map(|(k, v)| {
                serde_json::json!({
                    "Key": k,
                    "Value": base64::engine::general_purpose::STANDARD.encode(v),
                })
            })
            .collect();
        let mut resp = axum::response::Response::new(axum::body::Body::from(
            serde_json::to_string(&body).unwrap(),
        ));
        resp.headers_mut()
            .insert("X-Consul-Index", cur_idx.to_string().parse().unwrap());
        resp
    }

    /// 可切换故障的 mock：fail=true 时返回 500，否则返回正常 KV 数据。
    async fn spawn_mock_consul_fail() -> (String, Arc<AtomicBool>) {
        let fail = Arc::new(AtomicBool::new(true));
        let app = axum::Router::new()
            .route("/v1/kv/{prefix}", axum::routing::get(fail_kv_handler))
            .with_state(fail.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{addr}"), fail)
    }

    async fn fail_kv_handler(State(fail): State<Arc<AtomicBool>>) -> axum::response::Response {
        if fail.load(Ordering::SeqCst) {
            return axum::http::Response::builder()
                .status(500)
                .body(axum::body::Body::from("mock failure"))
                .unwrap();
        }
        kv_response(1u64, &[("app/key".to_string(), "{\"a\":1}".to_string())])
    }

    #[tokio::test]
    async fn watch_first_frame_and_change() {
        let (base_url, tx) = spawn_mock_consul().await;
        let source = ConsulConfigSource::new(base_url, "app");
        let mut rx = source.watch();
        let first = tokio::time::timeout(Duration::from_secs(3), rx.recv())
            .await
            .expect("first frame timed out")
            .unwrap()
            .unwrap();
        assert_eq!(first.get("key"), Some(&serde_json::json!({"a": 1})));

        tx.send((
            2u64,
            vec![("app/key2".to_string(), "{\"b\":2}".to_string())],
        ))
        .unwrap();
        let second = tokio::time::timeout(Duration::from_secs(3), rx.recv())
            .await
            .expect("change timed out")
            .unwrap()
            .unwrap();
        assert_eq!(second.get("key2"), Some(&serde_json::json!({"b": 2})));
        assert!(!second.contains_key("key"));
    }

    #[tokio::test]
    async fn watch_same_index_dedupes() {
        let (base_url, _tx) = spawn_mock_consul().await;
        let source = ConsulConfigSource::new(base_url, "app");
        let mut rx = source.watch();
        tokio::time::timeout(Duration::from_secs(3), rx.recv())
            .await
            .expect("first frame timed out")
            .unwrap()
            .unwrap();
        // 状态不变：服务器 1s 后返回同 index，不应推送
        let result = tokio::time::timeout(Duration::from_millis(2000), rx.recv()).await;
        assert!(result.is_err(), "no message expected on same index");
    }

    /// 不发送 X-Consul-Index 的 mock：模拟不支持索引的服务器。
    async fn spawn_mock_consul_no_index() -> String {
        let app =
            axum::Router::new().route("/v1/kv/{prefix}", axum::routing::get(no_index_kv_handler));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    async fn no_index_kv_handler() -> axum::response::Response {
        // 只有 body，没有任何 X-Consul-Index 响应头
        kv_response_without_index(1u64, &[("app/key".to_string(), "{\"a\":1}".to_string())])
    }

    fn kv_response_without_index(
        _cur_idx: u64,
        entries: &[(String, String)],
    ) -> axum::response::Response {
        let body: Vec<serde_json::Value> = entries
            .iter()
            .map(|(k, v)| {
                serde_json::json!({
                    "Key": k,
                    "Value": base64::engine::general_purpose::STANDARD.encode(v),
                })
            })
            .collect();
        axum::response::Response::new(axum::body::Body::from(
            serde_json::to_string(&body).unwrap(),
        ))
    }

    #[tokio::test]
    async fn watch_without_index_first_frame_then_backoff() {
        let base_url = spawn_mock_consul_no_index().await;
        let source = ConsulConfigSource::new(base_url, "app");
        let mut rx = source.watch();

        // 首帧必须推送（否则数据永远拿不到）
        let first = tokio::time::timeout(Duration::from_secs(3), rx.recv())
            .await
            .expect("first frame timed out")
            .unwrap()
            .unwrap();
        assert_eq!(first.get("key"), Some(&serde_json::json!({"a": 1})));

        // 缺 X-Consul-Index 时应有 1s 退避：500ms 内不应有第二帧（紧循环保护）
        let result = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await;
        assert!(result.is_err(), "no message expected during backoff");

        // 退避后应收到第二帧（轮询语义：服务器数据仍在推送）
        let second = tokio::time::timeout(Duration::from_secs(3), rx.recv())
            .await
            .expect("second frame timed out")
            .unwrap()
            .unwrap();
        assert_eq!(second.get("key"), Some(&serde_json::json!({"a": 1})));
    }

    #[tokio::test]
    async fn watch_error_then_recovers() {
        let (base_url, fail) = spawn_mock_consul_fail().await;
        let source = ConsulConfigSource::new(base_url, "app");
        let mut rx = source.watch();

        // 服务器 500：应收到 Err，且 channel 未关闭（后台任务仍在运行）
        let err = tokio::time::timeout(Duration::from_secs(3), rx.recv())
            .await
            .expect("error frame timed out")
            .expect("channel closed, task must keep running");
        assert!(err.is_err(), "expected Err, got {err:?}");

        // 恢复后：1s 退避重试，应收到正常配置帧
        fail.store(false, Ordering::SeqCst);
        let ok = tokio::time::timeout(Duration::from_secs(3), rx.recv())
            .await
            .expect("recovery frame timed out")
            .expect("channel closed, task must keep running")
            .expect("expected Ok after recovery");
        assert_eq!(ok.get("key"), Some(&serde_json::json!({"a": 1})));
    }
}
