// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use ecat_registry::{Registration, Registry, RegistryError, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ConsulRegistry {
    client: reqwest::Client,
    base_url: String,
    datacenter: String,
    token: Option<String>,
}

impl ConsulRegistry {
    /// base_url 必须为 https（本机回环地址允许 http，便于开发）。拒绝
    /// 明文远程地址：Consul API 带 token 时走明文会泄露凭据，构造期
    /// 强制即可保证 token 只出现在受保护连接上。
    pub fn new(base_url: impl Into<String>) -> Result<Self, String> {
        let base_url = base_url.into();
        let url = reqwest::Url::parse(&base_url)
            .map_err(|e| format!("invalid consul base_url '{base_url}': {e}"))?;
        let host = url.host_str().unwrap_or("");
        if url.scheme() != "https" && !is_loopback_host(host) {
            return Err(format!(
                "consul base_url must be https (loopback http allowed for dev), got '{base_url}'"
            ));
        }
        Ok(Self {
            client: reqwest::Client::new(),
            base_url,
            datacenter: "dc1".into(),
            token: None,
        })
    }

    pub fn datacenter(self, dc: impl Into<String>) -> Self {
        Self {
            datacenter: dc.into(),
            ..self
        }
    }

    /// token 只会随请求发送；base_url 已在构造期被强制为 https 或回环
    /// http，凭据不会流经明文远程连接。
    pub fn token(self, token: impl Into<String>) -> Self {
        Self {
            token: Some(token.into()),
            ..self
        }
    }

    fn headers(&self) -> HashMap<String, String> {
        let mut h = HashMap::new();
        if let Some(ref token) = self.token {
            h.insert("X-Consul-Token".into(), token.clone());
        }
        h
    }
}

#[async_trait]
impl Registry for ConsulRegistry {
    async fn register(&self, service: ServiceInfo) -> Result<Registration, RegistryError> {
        let id = format!("{}-{}", service.name, uuid::Uuid::new_v4());
        let endpoint = service.endpoints.first();
        let address = endpoint
            .map(|e| extract_host(e))
            .unwrap_or("127.0.0.1")
            .to_string();
        let port = endpoint.and_then(|e| extract_port(e));

        // 有地址和端口时注册 HTTP 健康检查，Consul 据此判定实例存活；
        // 健康检查协议随服务端点派生（https 端点 → https 探活）
        let check = port.map(|p| {
            // IPv6 地址需加方括号才能组成合法 URL 主机
            let host = if address.contains(':') {
                format!("[{address}]")
            } else {
                address.clone()
            };
            let scheme = if endpoint.is_some_and(|e| e.starts_with("https://")) {
                "https"
            } else {
                "http"
            };
            ConsulHealthCheck {
                http: format!("{scheme}://{host}:{p}/health"),
                interval: "10s".into(),
                timeout: "3s".into(),
            }
        });

        let req = ConsulRegisterRequest {
            id: id.clone(),
            name: service.name.clone(),
            address,
            port,
            check,
        };

        let url = format!("{}/v1/agent/service/register", self.base_url);
        let mut builder = self.client.put(&url).json(&req);
        for (k, v) in self.headers() {
            builder = builder.header(k, v);
        }

        let resp = builder
            .send()
            .await
            .map_err(|e| RegistryError::Other(format!("consul register: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(RegistryError::Other(format!(
                "consul register failed: {body}"
            )));
        }

        Ok(Registration::new(id, service, Arc::new(self.clone())))
    }

    async fn deregister(&self, id: &str) -> Result<(), RegistryError> {
        let url = format!("{}/v1/agent/service/deregister/{id}", self.base_url);
        let mut builder = self.client.put(&url);
        for (k, v) in self.headers() {
            builder = builder.header(k, v);
        }

        let resp = builder
            .send()
            .await
            .map_err(|e| RegistryError::Other(format!("consul deregister: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(RegistryError::Other(format!(
                "consul deregister failed: {body}"
            )));
        }

        Ok(())
    }

    async fn discover(&self, name: &str) -> Result<Vec<ServiceInfo>, RegistryError> {
        let url = format!(
            "{}/v1/health/service/{}?dc={}&passing=true",
            self.base_url,
            percent_encode_segment(name),
            percent_encode_segment(&self.datacenter)
        );
        let mut builder = self.client.get(&url);
        for (k, v) in self.headers() {
            builder = builder.header(k, v);
        }

        let resp = builder
            .send()
            .await
            .map_err(|e| RegistryError::Other(format!("consul discover: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(RegistryError::Other(format!(
                "consul discover failed: {body}"
            )));
        }

        let entries: Vec<ConsulHealthEntry> = resp
            .json()
            .await
            .map_err(|e| RegistryError::Other(format!("consul parse: {e}")))?;

        let services = entries
            .into_iter()
            .map(|e| {
                let addr = e.service.address.as_deref().unwrap_or(&e.node.address);
                // IPv6 字面量需加方括号才能组成合法 URL 主机
                let host = if addr.contains(':') {
                    format!("[{addr}]")
                } else {
                    addr.to_string()
                };
                // 服务带 "https" tag 时用 https 协议；Consul 无原生协议字段，
                // 以显式 tag 为准（缺省 http）。
                let scheme = if e.service.tags.iter().any(|t| t == "https") {
                    "https"
                } else {
                    "http"
                };
                let endpoint = format!("{scheme}://{host}:{}", e.service.port);
                // Consul has no native version field; parse it from a
                // "version=x" service tag when present, else leave it empty.
                let version = e
                    .service
                    .tags
                    .iter()
                    .find_map(|t| t.strip_prefix("version="))
                    .unwrap_or("");
                ServiceInfo::new(&e.service.service, version).with_endpoint(endpoint)
            })
            .collect();

        Ok(services)
    }

    async fn list_services(&self) -> Result<Vec<String>, RegistryError> {
        let url = format!("{}/v1/agent/services", self.base_url);
        let mut builder = self.client.get(&url);
        for (k, v) in self.headers() {
            builder = builder.header(k, v);
        }

        let resp = builder
            .send()
            .await
            .map_err(|e| RegistryError::Other(format!("consul list: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(RegistryError::Other(format!("consul list failed: {body}")));
        }

        let services: HashMap<String, ConsulServiceEntry> = resp
            .json()
            .await
            .map_err(|e| RegistryError::Other(format!("consul parse: {e}")))?;

        Ok(services.into_values().map(|s| s.service).collect())
    }
}

fn is_loopback_host(host: &str) -> bool {
    // url crate 的 host_str() 对 IPv6 保留方括号（"[::1]"）
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host == "localhost" || host == "::1" || host.starts_with("127.")
}

fn extract_host(endpoint: &str) -> &str {
    let rest = endpoint
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    // IPv6 字面量形如 [::1]:8080：取方括号内部分
    if let Some(rest) = rest.strip_prefix('[') {
        return rest.split(']').next().unwrap_or("127.0.0.1");
    }
    rest.split(':').next().unwrap_or("127.0.0.1")
}

fn extract_port(endpoint: &str) -> Option<u32> {
    endpoint
        .rsplit(':')
        .next()
        // 丢弃路径段（如 :8080/health），仅保留端口数字
        .and_then(|p| p.split('/').next())
        .and_then(|p| p.parse().ok())
}

// ── Consul API types ──

#[derive(Debug, Serialize)]
struct ConsulRegisterRequest {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Address")]
    address: String,
    #[serde(rename = "Port", skip_serializing_if = "Option::is_none")]
    port: Option<u32>,
    #[serde(rename = "Check", skip_serializing_if = "Option::is_none")]
    check: Option<ConsulHealthCheck>,
}

#[derive(Debug, Serialize)]
struct ConsulHealthCheck {
    #[serde(rename = "HTTP")]
    http: String,
    #[serde(rename = "Interval")]
    interval: String,
    #[serde(rename = "Timeout")]
    timeout: String,
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

#[derive(Debug, Deserialize)]
struct ConsulServiceEntry {
    #[serde(rename = "Service")]
    service: String,
}

#[derive(Debug, Deserialize)]
struct ConsulHealthEntry {
    #[serde(rename = "Node")]
    node: ConsulNode,
    #[serde(rename = "Service")]
    service: ConsulHealthService,
}

#[derive(Debug, Deserialize)]
struct ConsulNode {
    #[serde(rename = "Address")]
    address: String,
}

#[derive(Debug, Deserialize)]
struct ConsulHealthService {
    #[serde(rename = "Service")]
    service: String,
    #[serde(rename = "Address")]
    address: Option<String>,
    #[serde(rename = "Port")]
    port: u16,
    #[serde(rename = "Tags")]
    tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consul_registry_constructs() {
        let _reg = ConsulRegistry::new("http://localhost:8500")
            .unwrap()
            .datacenter("dc2")
            .token("secret-token");
    }

    #[test]
    fn consul_registry_clone() {
        let reg = ConsulRegistry::new("http://localhost:8500").unwrap();
        let _reg2 = reg.clone();
    }

    #[test]
    fn new_rejects_insecure_remote_url() {
        let err = ConsulRegistry::new("http://consul.internal:8500").unwrap_err();
        assert!(err.contains("https"), "got: {err}");
        assert!(ConsulRegistry::new("https://consul.internal:8500").is_ok());
    }

    #[test]
    fn new_accepts_loopback_http_for_dev() {
        assert!(ConsulRegistry::new("http://127.0.0.1:8500").is_ok());
        assert!(ConsulRegistry::new("http://[::1]:8500").is_ok());
        assert!(ConsulRegistry::new("http://localhost:8500").is_ok());
    }

    #[test]
    fn is_loopback_detects_dev_hosts() {
        assert!(is_loopback_host("localhost"));
        assert!(is_loopback_host("127.0.0.1"));
        assert!(is_loopback_host("127.8.8.8"));
        assert!(is_loopback_host("::1"));
        assert!(!is_loopback_host("10.0.0.5"));
        assert!(!is_loopback_host("consul.internal"));
    }

    #[test]
    fn extract_host_ipv4() {
        assert_eq!(extract_host("http://10.0.0.5:8080"), "10.0.0.5");
        assert_eq!(extract_host("https://example.com:443"), "example.com");
        assert_eq!(extract_host("http://10.0.0.5:8080/health"), "10.0.0.5");
    }

    #[test]
    fn extract_host_ipv6_literal() {
        assert_eq!(extract_host("http://[::1]:8080"), "::1");
        assert_eq!(extract_host("https://[2001:db8::1]:443"), "2001:db8::1");
        assert_eq!(extract_host("http://[::1]"), "::1");
    }

    #[test]
    fn extract_port_ipv6() {
        assert_eq!(extract_port("http://[::1]:8080"), Some(8080));
        assert_eq!(extract_port("http://[::1]:8080/health"), Some(8080));
        assert_eq!(extract_port("http://[::1]"), None);
    }

    /// mock Consul 的 /v1/health/service/<name> 端点，返回给定 JSON 文本。
    async fn spawn_mock_health(body: &'static str) -> String {
        let app = axum::Router::new().route(
            "/v1/health/service/{name}",
            axum::routing::get(move || async move {
                axum::response::Response::new(axum::body::Body::from(body))
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
    async fn discover_uses_https_tag_and_brackets_ipv6() {
        let body = r#"[
            {"Node":{"Address":"10.0.0.5"},
             "Service":{"Service":"web","Address":"2001:db8::1","Port":8443,"Tags":["version=2.0","https"]}}
        ]"#;
        let base_url = spawn_mock_health(body).await;
        let reg = ConsulRegistry::new(base_url).unwrap();
        let services = reg.discover("web").await.unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].endpoints, vec!["https://[2001:db8::1]:8443"]);
        assert_eq!(services[0].version, "2.0");
    }

    #[tokio::test]
    async fn discover_defaults_to_http() {
        let body = r#"[
            {"Node":{"Address":"10.0.0.5"},
             "Service":{"Service":"api","Address":"10.0.0.9","Port":9000,"Tags":[]}}
        ]"#;
        let base_url = spawn_mock_health(body).await;
        let reg = ConsulRegistry::new(base_url).unwrap();
        let services = reg.discover("api").await.unwrap();
        assert_eq!(services[0].endpoints, vec!["http://10.0.0.9:9000"]);
    }

    #[tokio::test]
    async fn register_sends_health_check() {
        let seen = Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
        let s = seen.clone();
        let app = axum::Router::new().route(
            "/v1/agent/service/register",
            axum::routing::put(
                move |axum::Json(body): axum::Json<serde_json::Value>| async move {
                    *s.lock().unwrap() = Some(body);
                    axum::http::StatusCode::OK
                },
            ),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let reg = ConsulRegistry::new(format!("http://{addr}")).unwrap();
        let service = ServiceInfo::new("web", "1.0").with_endpoint("http://10.0.0.5:8080");
        let _registration = reg.register(service).await.unwrap();

        let body = seen.lock().unwrap().take().expect("register body");
        let check = body.get("Check").expect("health check");
        assert_eq!(check["HTTP"], "http://10.0.0.5:8080/health");
        assert_eq!(check["Interval"], "10s");
        assert_eq!(check["Timeout"], "3s");
    }

    #[test]
    fn percent_encode_segment_encodes_reserved_chars() {
        assert_eq!(percent_encode_segment("api/v2"), "api%2Fv2");
        assert_eq!(percent_encode_segment("my dc"), "my%20dc");
        // 非 ASCII 按 UTF-8 字节逐字节编码
        assert_eq!(percent_encode_segment("服务"), "%E6%9C%8D%E5%8A%A1");
    }

    /// 捕获 PUT /v1/agent/service/register 请求体的 mock，返回 (base_url, 捕获体)。
    async fn spawn_register_capture() -> (String, Arc<std::sync::Mutex<Option<serde_json::Value>>>)
    {
        let seen = Arc::new(std::sync::Mutex::new(None));
        let s = seen.clone();
        let app = axum::Router::new().route(
            "/v1/agent/service/register",
            axum::routing::put(
                move |axum::Json(body): axum::Json<serde_json::Value>| async move {
                    *s.lock().unwrap() = Some(body);
                    axum::http::StatusCode::OK
                },
            ),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{addr}"), seen)
    }

    #[tokio::test]
    async fn register_without_endpoint_uses_default_address_and_no_check() {
        let (base_url, seen) = spawn_register_capture().await;
        let reg = ConsulRegistry::new(base_url).unwrap();
        reg.register(ServiceInfo::new("web", "1.0")).await.unwrap();
        let body = seen.lock().unwrap().take().expect("register body");
        assert_eq!(body["Address"], "127.0.0.1");
        assert!(body.get("Port").is_none(), "no port → no health check");
        assert!(body.get("Check").is_none());
        assert_eq!(body["Name"], "web");
    }

    #[tokio::test]
    async fn register_https_endpoint_probes_https() {
        let (base_url, seen) = spawn_register_capture().await;
        let reg = ConsulRegistry::new(base_url).unwrap();
        let service = ServiceInfo::new("web", "1.0").with_endpoint("https://10.0.0.5:8443");
        reg.register(service).await.unwrap();
        let body = seen.lock().unwrap().take().expect("register body");
        assert_eq!(body["Address"], "10.0.0.5");
        assert_eq!(body["Port"], 8443);
        assert_eq!(body["Check"]["HTTP"], "https://10.0.0.5:8443/health");
    }

    #[tokio::test]
    async fn register_ipv6_endpoint_brackets_host() {
        let (base_url, seen) = spawn_register_capture().await;
        let reg = ConsulRegistry::new(base_url).unwrap();
        let service = ServiceInfo::new("web", "1.0").with_endpoint("http://[2001:db8::1]:8080");
        reg.register(service).await.unwrap();
        let body = seen.lock().unwrap().take().expect("register body");
        assert_eq!(body["Address"], "2001:db8::1");
        assert_eq!(body["Check"]["HTTP"], "http://[2001:db8::1]:8080/health");
    }

    #[tokio::test]
    async fn register_error_response_is_err() {
        let app = axum::Router::new().route(
            "/v1/agent/service/register",
            axum::routing::put(|| async { axum::http::StatusCode::INTERNAL_SERVER_ERROR }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let reg = ConsulRegistry::new(format!("http://{addr}")).unwrap();
        let err = match reg.register(ServiceInfo::new("web", "1.0")).await {
            Ok(_) => panic!("register must fail on 500"),
            Err(e) => e,
        };
        assert!(err.to_string().contains("register failed"), "got: {err}");
    }

    #[tokio::test]
    async fn deregister_passes_id_in_path() {
        let seen = Arc::new(std::sync::Mutex::new(None::<String>));
        let s = seen.clone();
        let app = axum::Router::new().route(
            "/v1/agent/service/deregister/{id}",
            axum::routing::put(
                move |axum::extract::Path(id): axum::extract::Path<String>| async move {
                    *s.lock().unwrap() = Some(id);
                    axum::http::StatusCode::OK
                },
            ),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let reg = ConsulRegistry::new(format!("http://{addr}")).unwrap();
        reg.deregister("web-uuid").await.unwrap();
        assert_eq!(seen.lock().unwrap().as_deref(), Some("web-uuid"));
    }

    #[tokio::test]
    async fn token_header_sent_when_configured() {
        let seen = Arc::new(std::sync::Mutex::new(None::<String>));
        let s = seen.clone();
        let app = axum::Router::new().route(
            "/v1/agent/services",
            axum::routing::get(move |headers: axum::http::HeaderMap| async move {
                if let Some(tok) = headers.get("x-consul-token") {
                    *s.lock().unwrap() = Some(tok.to_str().unwrap().to_string());
                }
                axum::Json(serde_json::json!({}))
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let reg = ConsulRegistry::new(format!("http://{addr}"))
            .unwrap()
            .token("tok-1");
        assert!(reg.list_services().await.unwrap().is_empty());
        assert_eq!(seen.lock().unwrap().as_deref(), Some("tok-1"));
    }

    #[tokio::test]
    async fn token_header_absent_when_not_configured() {
        let seen = Arc::new(std::sync::Mutex::new(false));
        let s = seen.clone();
        let app = axum::Router::new().route(
            "/v1/agent/services",
            axum::routing::get(move |headers: axum::http::HeaderMap| async move {
                if headers.contains_key("x-consul-token") {
                    *s.lock().unwrap() = true;
                }
                axum::Json(serde_json::json!({}))
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let reg = ConsulRegistry::new(format!("http://{addr}")).unwrap();
        assert!(reg.list_services().await.unwrap().is_empty());
        assert!(
            !*seen.lock().unwrap(),
            "token must not be sent without config"
        );
    }

    #[tokio::test]
    async fn list_services_parses_agent_services_map() {
        let app = axum::Router::new().route(
            "/v1/agent/services",
            axum::routing::get(|| async {
                axum::Json(serde_json::json!({
                    "id1": {"Service": "auth"},
                    "id2": {"Service": "gw"},
                }))
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let reg = ConsulRegistry::new(format!("http://{addr}")).unwrap();
        let names = reg.list_services().await.unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"auth".to_string()));
        assert!(names.contains(&"gw".to_string()));
    }

    #[tokio::test]
    async fn discover_falls_back_to_node_address() {
        let body = r#"[
            {"Node":{"Address":"10.1.2.3"},
             "Service":{"Service":"api","Port":9000,"Tags":[]}}
        ]"#;
        let base_url = spawn_mock_health(body).await;
        let reg = ConsulRegistry::new(base_url).unwrap();
        let services = reg.discover("api").await.unwrap();
        assert_eq!(services[0].endpoints, vec!["http://10.1.2.3:9000"]);
    }

    #[tokio::test]
    async fn discover_empty_result_is_ok() {
        let base_url = spawn_mock_health("[]").await;
        let reg = ConsulRegistry::new(base_url).unwrap();
        assert!(reg.discover("nope").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn discover_error_response_is_err() {
        let app = axum::Router::new().route(
            "/v1/health/service/{name}",
            axum::routing::get(|| async { axum::http::StatusCode::NOT_FOUND }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let reg = ConsulRegistry::new(format!("http://{addr}")).unwrap();
        let err = reg.discover("web").await.unwrap_err();
        assert!(err.to_string().contains("discover failed"), "got: {err}");
    }

    #[tokio::test]
    async fn discover_percent_encodes_service_name_and_dc() {
        let seen = Arc::new(std::sync::Mutex::new(None::<String>));
        let s = seen.clone();
        let app = axum::Router::new().route(
            "/v1/health/service/{name}",
            axum::routing::get(
                move |axum::extract::RawQuery(q): axum::extract::RawQuery| async move {
                    *s.lock().unwrap() = q;
                    axum::Json(serde_json::json!([]))
                },
            ),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let reg = ConsulRegistry::new(format!("http://{addr}"))
            .unwrap()
            .datacenter("my dc");
        reg.discover("api/v2").await.unwrap();
        let guard = seen.lock().unwrap();
        let q = guard.as_deref().unwrap_or_default();
        assert!(q.contains("dc=my%20dc"), "got: {q}");
        assert!(q.contains("passing=true"), "got: {q}");
    }
}
