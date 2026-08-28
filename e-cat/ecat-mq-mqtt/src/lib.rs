// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use bytes::Bytes;
use ecat_mq::{MessageQueue, MessageStream, MqError};
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS, Transport};
use serde::Deserialize;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::mpsc;

/// rumqttc 以 use-rustls-no-provider 编译时不自带 CryptoProvider；与
/// ecat-tls / ecat-transport-* 一致，构造 TLS 前安装 ring（首装生效）。
fn ensure_crypto_provider() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let _ = rustls::crypto::CryptoProvider::install_default(
            rustls::crypto::ring::default_provider(),
        );
    });
}

#[derive(Debug, Clone, Deserialize)]
pub struct MqttConfig {
    pub url: String,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    /// 启用 TLS（rustls）。url 缺省端口时 TLS 下默认 8883（明文为 1883）。
    /// 仅设 tls=true 用系统信任根；配 ca_file 用自定义 CA，可选
    /// cert_file/key_file 做 mTLS。
    #[serde(default)]
    pub tls: bool,
    /// 自定义 CA 证书 PEM 路径；缺省用系统信任根。
    #[serde(default)]
    pub ca_file: Option<String>,
    /// 客户端证书 PEM 路径（mTLS），与 key_file 成对。
    #[serde(default)]
    pub cert_file: Option<String>,
    #[serde(default)]
    pub key_file: Option<String>,
}

pub struct MqttMq {
    client: AsyncClient,
    config: MqttConfig,
    sub_counter: AtomicU32,
}

impl MqttMq {
    pub async fn connect(url: &str) -> Result<Self, MqError> {
        Self::from_config(MqttConfig {
            url: url.to_string(),
            client_id: None,
            username: None,
            password: None,
            tls: false,
            ca_file: None,
            cert_file: None,
            key_file: None,
        })
        .await
    }

    pub async fn from_config(cfg: MqttConfig) -> Result<Self, MqError> {
        let (host, port) = parse_url(&cfg.url, cfg.tls);
        let client_id = cfg.client_id.clone().unwrap_or_else(|| "ecat-mqtt".into());
        let (client, eventloop) =
            AsyncClient::new(client_options(&cfg, host, port, client_id)?, 10);
        tokio::spawn(pump(eventloop));
        Ok(Self {
            client,
            config: cfg,
            sub_counter: AtomicU32::new(0),
        })
    }
}

fn client_options(
    cfg: &MqttConfig,
    host: String,
    port: u16,
    client_id: String,
) -> Result<MqttOptions, MqError> {
    let mut opts = MqttOptions::new(client_id, host, port);
    opts.set_keep_alive(Duration::from_secs(10));
    if let (Some(u), Some(p)) = (&cfg.username, &cfg.password) {
        opts.set_credentials(u, p);
    }
    if cfg.tls || cfg.ca_file.is_some() || cfg.cert_file.is_some() || cfg.key_file.is_some() {
        ensure_crypto_provider();
        let transport = if let Some(ca) = &cfg.ca_file {
            let ca = std::fs::read(ca)
                .map_err(|e| MqError::Other(format!("mqtt tls: read ca {ca}: {e}")))?;
            let client_auth = match (&cfg.cert_file, &cfg.key_file) {
                (Some(c), Some(k)) => {
                    let cert = std::fs::read(c)
                        .map_err(|e| MqError::Other(format!("mqtt tls: read cert {c}: {e}")))?;
                    let key = std::fs::read(k)
                        .map_err(|e| MqError::Other(format!("mqtt tls: read key {k}: {e}")))?;
                    Some((cert, key))
                }
                (None, None) => None,
                _ => {
                    return Err(MqError::Other(
                        "mqtt tls: cert_file and key_file must be set together".into(),
                    ));
                }
            };
            Transport::tls(ca, client_auth, None)
        } else {
            Transport::tls_with_default_config()
        };
        opts.set_transport(transport);
    }
    Ok(opts)
}

/// Keeps the publisher connection alive; retries after transient errors.
async fn pump(mut eventloop: EventLoop) {
    loop {
        if eventloop.poll().await.is_err() {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

fn parse_url(url: &str, tls: bool) -> (String, u16) {
    let trimmed = url
        .strip_prefix("tcp://")
        .or_else(|| url.strip_prefix("mqtt://"))
        .or_else(|| url.strip_prefix("ssl://"))
        .or_else(|| url.strip_prefix("mqtts://"))
        .unwrap_or(url);
    match trimmed.rsplit_once(':') {
        Some((host, port)) => (
            host.to_string(),
            port.parse().unwrap_or(if tls { 8883 } else { 1883 }),
        ),
        None => (trimmed.to_string(), if tls { 8883 } else { 1883 }),
    }
}

#[async_trait]
impl MessageQueue for MqttMq {
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), MqError> {
        self.client
            .publish(topic, QoS::AtMostOnce, false, payload.to_vec())
            .await
            .map_err(|e| MqError::Other(format!("mqtt publish: {e}")))?;
        Ok(())
    }

    async fn subscribe(&self, topic: &str) -> Result<Box<dyn MessageStream>, MqError> {
        let base_id = self
            .config
            .client_id
            .clone()
            .unwrap_or_else(|| "ecat-mqtt".into());
        let n = self.sub_counter.fetch_add(1, Ordering::SeqCst);
        let (host, port) = parse_url(&self.config.url, self.config.tls);
        // Dedicated connection per subscription so one slow consumer
        // never stalls another (and the broker never kicks the publisher).
        let (client, eventloop) = AsyncClient::new(
            client_options(&self.config, host, port, format!("{base_id}-sub{n}"))?,
            10,
        );
        client
            .subscribe(topic, QoS::AtMostOnce)
            .await
            .map_err(|e| MqError::Other(format!("mqtt subscribe: {e}")))?;

        let (tx, rx) = mpsc::channel::<Bytes>(256);
        tokio::spawn(async move {
            let mut eventloop = eventloop;
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(msg))) => {
                        // msg.payload 是 bytes::Bytes，直接透传，零拷贝。
                        if tx.send(msg.payload).await.is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
                }
            }
        });
        Ok(Box::new(MqttStream { rx }))
    }
}

struct MqttStream {
    rx: mpsc::Receiver<Bytes>,
}

impl MessageStream for MqttStream {
    fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, MqError>>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(data)) => Poll::Ready(Some(Ok(data))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Unpin for MqttStream {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_deserializes() {
        let cfg: MqttConfig = serde_json::from_value(serde_json::json!({
            "url": "tcp://localhost:1883",
            "client_id": "sensor-1",
            "username": "user",
            "password": "pass",
            "tls": true,
            "ca_file": "/ca.pem",
        }))
        .unwrap();
        assert_eq!(cfg.client_id.as_deref(), Some("sensor-1"));
        assert!(cfg.tls);
        assert_eq!(cfg.ca_file.as_deref(), Some("/ca.pem"));
    }

    #[test]
    fn url_parses_host_and_port() {
        assert_eq!(
            parse_url("tcp://mqtt.local:2883", false),
            ("mqtt.local".into(), 2883)
        );
        assert_eq!(parse_url("mqtt://broker", false), ("broker".into(), 1883));
        // TLS 缺省端口 8883；ssl:// 前缀也识别
        assert_eq!(parse_url("mqtt://broker", true), ("broker".into(), 8883));
        assert_eq!(
            parse_url("ssl://broker:4443", true),
            ("broker".into(), 4443)
        );
    }

    #[test]
    fn client_options_builds_with_tls_default_roots() {
        let cfg = MqttConfig {
            url: "mqtt://broker".into(),
            client_id: None,
            username: None,
            password: None,
            tls: true,
            ca_file: None,
            cert_file: None,
            key_file: None,
        };
        let opts = client_options(&cfg, "broker".into(), 8883, "ecat-mqtt".into()).unwrap();
        assert_eq!(opts.broker_address(), ("broker".into(), 8883));
    }

    #[test]
    fn client_options_rejects_cert_without_key() {
        let cfg = MqttConfig {
            url: "mqtt://broker".into(),
            client_id: None,
            username: None,
            password: None,
            tls: true,
            ca_file: Some("/ca.pem".into()),
            cert_file: Some("/cert.pem".into()),
            key_file: None,
        };
        assert!(client_options(&cfg, "broker".into(), 8883, "id".into()).is_err());
    }

    #[test]
    fn client_options_rejects_key_without_cert() {
        let cfg = MqttConfig {
            url: "mqtt://broker".into(),
            client_id: None,
            username: None,
            password: None,
            tls: true,
            ca_file: Some("/ca.pem".into()),
            cert_file: None,
            key_file: Some("/key.pem".into()),
        };
        assert!(client_options(&cfg, "broker".into(), 8883, "id".into()).is_err());
    }

    #[test]
    fn client_options_errors_when_ca_file_missing() {
        let cfg = MqttConfig {
            url: "mqtt://broker".into(),
            client_id: None,
            username: None,
            password: None,
            tls: true,
            ca_file: Some("/no-such-ca.pem".into()),
            cert_file: None,
            key_file: None,
        };
        let err = client_options(&cfg, "broker".into(), 8883, "id".into()).unwrap_err();
        assert!(err.to_string().contains("read ca"), "got: {err}");
    }

    #[test]
    fn client_options_errors_when_client_cert_files_missing() {
        // CA 先于 cert 读取，先落地一个存在的 ca 文件让错误落在 cert 读取上
        let ca = std::env::temp_dir().join(format!("ecat-mqtt-ca-{}", std::process::id()));
        std::fs::write(&ca, b"dummy").unwrap();
        let cfg = MqttConfig {
            url: "mqtt://broker".into(),
            client_id: None,
            username: None,
            password: None,
            tls: true,
            ca_file: Some(ca.to_str().unwrap().into()),
            cert_file: Some("/no-such-cert.pem".into()),
            key_file: Some("/no-such-key.pem".into()),
        };
        let err = client_options(&cfg, "broker".into(), 8883, "id".into()).unwrap_err();
        assert!(err.to_string().contains("read cert"), "got: {err}");
        let _ = std::fs::remove_file(&ca);
    }

    #[test]
    fn parse_url_defaults_ports_for_all_schemes() {
        assert_eq!(parse_url("localhost", false), ("localhost".into(), 1883));
        assert_eq!(parse_url("localhost", true), ("localhost".into(), 8883));
        assert_eq!(parse_url("tcp://h:9999", false), ("h".into(), 9999));
        assert_eq!(parse_url("mqtts://h", true), ("h".into(), 8883));
        assert_eq!(parse_url("ssl://h", true), ("h".into(), 8883));
        assert_eq!(parse_url("mqtt://h", false), ("h".into(), 1883));
    }

    #[test]
    fn parse_url_falls_back_on_invalid_or_missing_port() {
        assert_eq!(parse_url("mqtt://h:abc", false), ("h".into(), 1883));
        assert_eq!(parse_url("mqtt://h:", true), ("h".into(), 8883));
    }
}
