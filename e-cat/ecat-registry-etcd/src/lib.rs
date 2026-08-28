// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_registry::{Registration, Registry, RegistryError, ServiceInfo};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct EtcdRegistry {
    client: reqwest::Client,
    endpoints: Vec<String>,
    prefix: String,
    lease_ttl: u64,
    keepalives: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl EtcdRegistry {
    pub fn new(endpoints: Vec<String>, prefix: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoints,
            prefix: prefix.into(),
            lease_ttl: 30,
            keepalives: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn lease_ttl(mut self, ttl: u64) -> Self {
        self.lease_ttl = ttl;
        self
    }

    fn base_url(&self) -> &str {
        self.endpoints
            .first()
            .map(|s| s.as_str())
            .unwrap_or("http://127.0.0.1:2379")
    }

    /// 后台任务周期调用 lease keepalive，防止注册的租约过期被 etcd 回收。
    /// 任务句柄按注册 id 保存，deregister 时 abort。
    fn spawn_keepalive(&self, id: &str, lease_id: i64) {
        let keepalive_url = format!("{}/v3/lease/keepalive", self.base_url());
        let client = self.client.clone();
        let interval_secs = (self.lease_ttl / 3).max(1);
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            // interval 首个 tick 立即触发，先消耗掉，避免注册后立刻续约
            interval.tick().await;
            loop {
                interval.tick().await;
                if let Err(e) = client
                    .post(&keepalive_url)
                    .json(&serde_json::json!({"ID": lease_id.to_string()}))
                    .send()
                    .await
                {
                    eprintln!("etcd lease keepalive failed: {e}");
                }
            }
        });
        self.keepalives
            .lock()
            .unwrap()
            .insert(id.to_string(), handle);
    }
}

#[async_trait]
impl Registry for EtcdRegistry {
    async fn register(&self, service: ServiceInfo) -> Result<Registration, RegistryError> {
        let id = format!("{}/{}", self.prefix, service.name);
        let lease_id = create_lease(&self.client, self.base_url(), self.lease_ttl)
            .await
            .map_err(RegistryError::Other)?;
        let key = format!(
            "/ecat/services/{}/{}/{}",
            self.prefix,
            service.name,
            uuid::Uuid::new_v4()
        );
        let value = serde_json::to_string(&service)
            .map_err(|e| RegistryError::Other(format!("serialize: {e}")))?;
        let body = serde_json::json!({
            "key": b64(&key),
            "value": b64(&value),
            "lease": lease_id.to_string(),
        });
        self.client
            .post(format!("{}/v3/kv/put", self.base_url()))
            .json(&body)
            .send()
            .await
            .map_err(|e| RegistryError::Other(format!("etcd put: {e}")))?;
        self.spawn_keepalive(&id, lease_id);
        Ok(Registration::new(id, service, Arc::new(self.clone())))
    }

    async fn deregister(&self, id: &str) -> Result<(), RegistryError> {
        if let Some(handle) = self.keepalives.lock().unwrap().remove(id) {
            handle.abort();
        }
        // 注册键为 /ecat/services/{id}/{uuid}，用范围删除前缀匹配的所有实例键
        let prefix = format!("/ecat/services/{id}/");
        self.client
            .post(format!("{}/v3/kv/deleterange", self.base_url()))
            .json(&serde_json::json!({
                "key": b64(&prefix),
                "range_end": b64(&prefix_end(&prefix)),
            }))
            .send()
            .await
            .map_err(|e| RegistryError::Other(format!("etcd del: {e}")))?;
        Ok(())
    }

    async fn discover(&self, name: &str) -> Result<Vec<ServiceInfo>, RegistryError> {
        let prefix = format!("/ecat/services/{}/{}/", self.prefix, name);
        let resp = self
            .client
            .post(format!("{}/v3/kv/range", self.base_url()))
            .json(&serde_json::json!({"key": b64(&prefix), "range_end": b64(&prefix_end(&prefix))}))
            .send()
            .await
            .map_err(|e| RegistryError::Other(format!("etcd range: {e}")))?;
        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| RegistryError::Other(format!("etcd parse: {e}")))?;
        let mut services = Vec::new();
        if let Some(kvs) = result.get("kvs").and_then(|v| v.as_array()) {
            for kv in kvs {
                if let Some(v) = kv.get("value").and_then(|v| v.as_str())
                    && let Ok(svc) = decode_b64_str(v).and_then(|s| {
                        serde_json::from_str::<ServiceInfo>(&s).map_err(|e| e.to_string())
                    })
                {
                    services.push(svc);
                }
            }
        }
        Ok(services)
    }

    async fn list_services(&self) -> Result<Vec<String>, RegistryError> {
        let svcs = self.discover("").await?;
        let mut names: Vec<String> = svcs.into_iter().map(|s| s.name).collect();
        names.sort();
        names.dedup();
        Ok(names)
    }
}

async fn create_lease(client: &reqwest::Client, base_url: &str, ttl: u64) -> Result<i64, String> {
    let resp = client
        .post(format!("{base_url}/v3/lease/grant"))
        .json(&serde_json::json!({"TTL": ttl.to_string()}))
        .send()
        .await
        .map_err(|e| format!("etcd lease: {e}"))?;
    let body: serde_json::Value = resp.json().await.map_err(|e| format!("etcd parse: {e}"))?;
    body.get("ID")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "no lease ID".into())
}

fn prefix_end(prefix: &str) -> String {
    let mut bytes = prefix.as_bytes().to_vec();
    for i in (0..bytes.len()).rev() {
        if bytes[i] < 0xff {
            bytes[i] += 1;
            bytes.truncate(i + 1);
            return String::from_utf8_lossy(&bytes).into_owned();
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

use base64::Engine;

fn b64(s: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
}

fn decode_b64_str(s: &str) -> Result<String, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| format!("base64: {e}"))?;
    String::from_utf8(bytes).map_err(|e| format!("utf8: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn etcd_registry_constructs() {
        let _reg = EtcdRegistry::new(vec!["http://localhost:2379".into()], "ecat").lease_ttl(60);
    }

    #[test]
    fn b64_roundtrip() {
        let input = "hello-world";
        let encoded = b64(input);
        let decoded = decode_b64_str(&encoded).unwrap();
        assert_eq!(decoded, input);
    }

    /// mock etcd 的 lease/kv 端点：记录 keepalive 调用次数与 deleterange 请求体。
    async fn spawn_mock_etcd() -> (
        String,
        Arc<std::sync::atomic::AtomicUsize>,
        Arc<Mutex<Vec<serde_json::Value>>>,
    ) {
        let keepalives = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let ka = keepalives.clone();
        let deletes = Arc::new(Mutex::new(Vec::new()));
        let del = deletes.clone();
        let app = axum::Router::new()
            .route(
                "/v3/lease/grant",
                axum::routing::post(|| async { axum::Json(serde_json::json!({"ID": "123"})) }),
            )
            .route(
                "/v3/kv/put",
                axum::routing::post(|| async { axum::Json(serde_json::json!({"header": {}})) }),
            )
            .route(
                "/v3/kv/deleterange",
                axum::routing::post(move |body: axum::Json<serde_json::Value>| async move {
                    del.lock().unwrap_or_else(|e| e.into_inner()).push(body.0);
                    axum::Json(serde_json::json!({"header": {}}))
                }),
            )
            .route(
                "/v3/lease/keepalive",
                axum::routing::post(move || async move {
                    ka.fetch_add(1, Ordering::SeqCst);
                    axum::Json(serde_json::json!({"result": {"ID": "123"}}))
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{addr}"), keepalives, deletes)
    }

    #[tokio::test]
    async fn register_keeps_lease_alive_until_deregister() {
        let (base_url, keepalives, _) = spawn_mock_etcd().await;
        let reg = EtcdRegistry::new(vec![base_url], "ecat").lease_ttl(3);
        let registration = reg.register(ServiceInfo::new("svc", "1.0")).await.unwrap();

        // lease_ttl=3 → 每 1 秒续约一次；等待首个 keepalive 到达
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while keepalives.load(Ordering::SeqCst) == 0 {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("keepalive never arrived");
        assert!(keepalives.load(Ordering::SeqCst) >= 1);
        assert!(!reg.keepalives.lock().unwrap().is_empty());

        reg.deregister(&registration.id).await.unwrap();
        assert!(reg.keepalives.lock().unwrap().is_empty());
    }

    #[test]
    fn prefix_end_bounds_empty_and_regular() {
        // 空前缀：无字节可自增，走兜底原样返回（etcd 约定空 range_end 表示范围到末尾）
        assert_eq!(prefix_end(""), "");
        // 常规：末字节自增并截断
        assert_eq!(prefix_end("/a/b"), "/a/c");
        assert_eq!(
            prefix_end("/ecat/services/ecat/svc/"),
            "/ecat/services/ecat/svc0"
        );
        // 非 ASCII 末字节按字节自增（0xBF+1 越出合法 UTF-8 后 lossy 替换），与 etcd 字节键语义一致
        assert_eq!(prefix_end("ÿ"), "\u{fffd}\u{fffd}");
    }

    #[tokio::test]
    async fn deregister_deletes_registered_key_range() {
        let (base_url, _, deletes) = spawn_mock_etcd().await;
        let reg = EtcdRegistry::new(vec![base_url], "ecat");
        let registration = reg.register(ServiceInfo::new("svc", "1.0")).await.unwrap();
        assert_eq!(registration.id, "ecat/svc");

        reg.deregister(&registration.id).await.unwrap();
        let reqs = deletes.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs.len(), 1);
        assert_eq!(
            reqs[0]["key"],
            b64("/ecat/services/ecat/svc/"),
            "deleterange 必须按注册前缀删除"
        );
        assert_eq!(reqs[0]["range_end"], b64("/ecat/services/ecat/svc0"));
        assert!(reg.keepalives.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn deregister_unknown_id_still_sends_range_delete() {
        let (base_url, _, deletes) = spawn_mock_etcd().await;
        let reg = EtcdRegistry::new(vec![base_url], "ecat");
        reg.deregister("missing").await.unwrap();

        let reqs = deletes.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0]["key"], b64("/ecat/services/missing/"));
        assert_eq!(reqs[0]["range_end"], b64("/ecat/services/missing0"));
    }

    /// 按 (路径, 响应体) 列表起一个 mock etcd 服务，返回 base_url。
    async fn spawn_mock(routes: &[(&str, serde_json::Value)]) -> String {
        let mut app = axum::Router::new();
        for (path, body) in routes {
            let body = body.clone();
            app = app.route(
                path,
                axum::routing::post(move || {
                    let body = body.clone();
                    async move { axum::Json(body) }
                }),
            );
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn discover_decodes_and_parses_kvs_skipping_bad_values() {
        let svc = ServiceInfo::new("web", "2.0").with_endpoint("http://10.0.0.5:8080");
        let value = serde_json::to_string(&svc).unwrap();
        let body = serde_json::json!({"kvs": [
            {"key": b64("k1"), "value": b64(&value)},
            {"key": b64("k2"), "value": "%%%not-base64%%%"},
            {"key": b64("k3"), "value": b64("not json")},
        ]});
        let base_url = spawn_mock(&[("/v3/kv/range", body)]).await;
        let reg = EtcdRegistry::new(vec![base_url], "ecat");
        let services = reg.discover("web").await.unwrap();
        assert_eq!(services.len(), 1, "invalid values must be skipped");
        assert_eq!(services[0].name, "web");
        assert_eq!(services[0].version, "2.0");
        assert_eq!(services[0].endpoints, vec!["http://10.0.0.5:8080"]);
    }

    #[tokio::test]
    async fn list_services_dedups_and_sorts() {
        let a = serde_json::to_string(&ServiceInfo::new("b-svc", "1.0")).unwrap();
        let b = serde_json::to_string(&ServiceInfo::new("a-svc", "1.0")).unwrap();
        let body = serde_json::json!({"kvs": [
            {"key": "x", "value": b64(&a)},
            {"key": "y", "value": b64(&b)},
            {"key": "z", "value": b64(&a)},
        ]});
        let base_url = spawn_mock(&[("/v3/kv/range", body)]).await;
        let reg = EtcdRegistry::new(vec![base_url], "ecat");
        assert_eq!(reg.list_services().await.unwrap(), vec!["a-svc", "b-svc"]);
    }

    #[tokio::test]
    async fn register_puts_base64_encoded_key_value_with_lease() {
        let puts = Arc::new(Mutex::new(Vec::new()));
        let p = puts.clone();
        let app = axum::Router::new()
            .route(
                "/v3/lease/grant",
                axum::routing::post(|| async { axum::Json(serde_json::json!({"ID": "42"})) }),
            )
            .route(
                "/v3/kv/put",
                axum::routing::post(move |body: axum::Json<serde_json::Value>| async move {
                    p.lock().unwrap().push(body.0);
                    axum::Json(serde_json::json!({"header": {}}))
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let reg = EtcdRegistry::new(vec![format!("http://{addr}")], "ecat");
        let registration = reg
            .register(ServiceInfo::new("svc", "1.0").with_endpoint("http://h:1"))
            .await
            .unwrap();

        let put = puts.lock().unwrap().first().expect("put body").clone();
        assert_eq!(put["lease"], "42");
        let key = decode_b64_str(put["key"].as_str().unwrap()).unwrap();
        assert!(
            key.starts_with("/ecat/services/ecat/svc/"),
            "key must be under service prefix, got: {key}"
        );
        let val: ServiceInfo =
            serde_json::from_str(&decode_b64_str(put["value"].as_str().unwrap()).unwrap()).unwrap();
        assert_eq!(val.endpoints, vec!["http://h:1"]);

        reg.deregister(&registration.id).await.unwrap();
    }

    #[tokio::test]
    async fn register_fails_when_lease_grant_malformed() {
        let base_url = spawn_mock(&[("/v3/lease/grant", serde_json::json!({"TTL": "30"}))]).await;
        let reg = EtcdRegistry::new(vec![base_url], "ecat");
        let err = match reg.register(ServiceInfo::new("svc", "1.0")).await {
            Ok(_) => panic!("register must fail when lease grant lacks ID"),
            Err(e) => e,
        };
        assert!(err.to_string().contains("no lease ID"), "got: {err}");
    }
}
