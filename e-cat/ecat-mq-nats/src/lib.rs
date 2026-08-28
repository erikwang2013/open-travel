// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_nats::rustls;
use async_nats::{Client, Message};
use async_trait::async_trait;
use bytes::Bytes;
use ecat_mq::{MessageQueue, MessageStream, MqError};
use futures_core::Stream;
use serde::Deserialize;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug, Clone, Deserialize)]
pub struct NatsConfig {
    pub url: String,
    /// 强制 TLS（require_tls）。配合下方 ca/cert/key 文件可做自定义 CA 与
    /// mTLS；缺省（false）与现状一致，由服务端协商。
    #[serde(default)]
    pub tls: bool,
    /// 自定义 CA 证书 PEM 路径；缺省用系统信任根。
    #[serde(default)]
    pub tls_ca_file: Option<String>,
    /// 客户端证书 PEM 路径（mTLS），必须与 tls_key_file 成对。
    #[serde(default)]
    pub tls_cert_file: Option<String>,
    #[serde(default)]
    pub tls_key_file: Option<String>,
}

pub struct NatsMq {
    client: Client,
}

impl NatsMq {
    pub async fn connect(url: &str) -> Result<Self, MqError> {
        let client = async_nats::connect(url)
            .await
            .map_err(|e| MqError::Other(format!("nats connect: {e}")))?;
        Ok(Self { client })
    }

    pub async fn from_config(cfg: NatsConfig) -> Result<Self, MqError> {
        let client = build_connect_options(&cfg)?
            .connect(&cfg.url)
            .await
            .map_err(|e| MqError::Other(format!("nats connect: {e}")))?;
        Ok(Self { client })
    }
}

/// TLS 全可选用 async-nats 自带 rustls（re-export）与系统信任根（async-nats
/// 依赖 rustls-native-certs），此处只组装配置，不新增网络依赖。
fn build_connect_options(cfg: &NatsConfig) -> Result<async_nats::ConnectOptions, MqError> {
    let opts = async_nats::ConnectOptions::new();
    if !cfg.tls
        && cfg.tls_ca_file.is_none()
        && cfg.tls_cert_file.is_none()
        && cfg.tls_key_file.is_none()
    {
        return Ok(opts);
    }
    if cfg.tls_cert_file.is_some() != cfg.tls_key_file.is_some() {
        return Err(MqError::Other(
            "nats tls: tls_cert_file and tls_key_file must be set together".into(),
        ));
    }
    if cfg.tls_ca_file.is_none() && (cfg.tls_cert_file.is_some() || cfg.tls_key_file.is_some()) {
        return Err(MqError::Other(
            "nats tls: tls_ca_file is required for client certificates".into(),
        ));
    }

    let mut roots = rustls::RootCertStore::empty();
    match &cfg.tls_ca_file {
        Some(ca) => {
            let pem = std::fs::read(ca)
                .map_err(|e| MqError::Other(format!("nats tls: read ca {ca}: {e}")))?;
            let certs = rustls_pemfile::certs(&mut pem.as_slice())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| MqError::Other(format!("nats tls: parse ca: {e}")))?;
            roots.add_parsable_certificates(certs);
        }
        None => {
            let certs = rustls_native_certs::load_native_certs()
                .map_err(|e| MqError::Other(format!("nats tls: load system certs: {e}")))?;
            for cert in certs {
                let _ = roots.add(cert);
            }
        }
    }

    let builder = rustls::ClientConfig::builder().with_root_certificates(roots);
    let tls = match (&cfg.tls_cert_file, &cfg.tls_key_file) {
        (Some(cert), Some(key)) => {
            let cert_pem = std::fs::read(cert)
                .map_err(|e| MqError::Other(format!("nats tls: read cert {cert}: {e}")))?;
            let key_pem = std::fs::read(key)
                .map_err(|e| MqError::Other(format!("nats tls: read key {key}: {e}")))?;
            let certs = rustls_pemfile::certs(&mut cert_pem.as_slice())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| MqError::Other(format!("nats tls: parse cert: {e}")))?;
            let key = rustls_pemfile::private_key(&mut key_pem.as_slice())
                .map_err(|e| MqError::Other(format!("nats tls: parse key: {e}")))?
                .ok_or_else(|| MqError::Other("nats tls: no private key found".into()))?;
            builder
                .with_client_auth_cert(certs, key)
                .map_err(|e| MqError::Other(format!("nats tls: client auth: {e}")))?
        }
        _ => builder.with_no_client_auth(),
    };
    Ok(opts.tls_client_config(tls))
}

#[async_trait]
impl MessageQueue for NatsMq {
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), MqError> {
        // async-nats requires a 'static subject; copy at the boundary.
        self.client
            .publish(topic.to_owned(), payload.to_vec().into())
            .await
            .map_err(|e| MqError::Other(format!("nats publish: {e}")))?;
        Ok(())
    }

    async fn subscribe(&self, topic: &str) -> Result<Box<dyn MessageStream>, MqError> {
        // The concrete Subscription type is private; erase it behind the Stream trait.
        let sub: Box<dyn Stream<Item = Message> + Send + Unpin> = Box::new(
            self.client
                .subscribe(topic.to_owned())
                .await
                .map_err(|e| MqError::Other(format!("nats subscribe: {e}")))?,
        );
        Ok(Box::new(NatsStream { sub }))
    }
}

struct NatsStream {
    sub: Box<dyn Stream<Item = Message> + Send + Unpin>,
}

impl MessageStream for NatsStream {
    fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, MqError>>> {
        match Pin::new(&mut *self.sub).poll_next(cx) {
            Poll::Ready(Some(msg)) => Poll::Ready(Some(Ok(msg.payload))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_deserializes() {
        let cfg: NatsConfig = serde_json::from_value(serde_json::json!({
            "url": "nats://localhost:4222",
            "tls": true,
            "tls_ca_file": "/ca.pem",
            "tls_cert_file": "/cert.pem",
            "tls_key_file": "/key.pem",
        }))
        .unwrap();
        assert_eq!(cfg.url, "nats://localhost:4222");
        assert!(cfg.tls);
        assert_eq!(cfg.tls_ca_file.as_deref(), Some("/ca.pem"));
    }

    #[test]
    fn tls_defaults_to_plaintext() {
        let cfg: NatsConfig =
            serde_json::from_value(serde_json::json!({"url": "nats://localhost:4222"})).unwrap();
        assert!(!cfg.tls);
        assert!(cfg.tls_ca_file.is_none());
        assert!(build_connect_options(&cfg).is_ok());
    }

    #[test]
    fn tls_rejects_cert_without_key() {
        let cfg = NatsConfig {
            url: "nats://localhost:4222".into(),
            tls: true,
            tls_ca_file: Some("/ca.pem".into()),
            tls_cert_file: Some("/cert.pem".into()),
            tls_key_file: None,
        };
        let err = build_connect_options(&cfg).unwrap_err();
        assert!(err.to_string().contains("tls_cert_file"));
    }

    #[test]
    fn tls_rejects_missing_ca_for_client_cert() {
        let cfg = NatsConfig {
            url: "nats://localhost:4222".into(),
            tls: true,
            tls_ca_file: None,
            tls_cert_file: Some("/cert.pem".into()),
            tls_key_file: Some("/key.pem".into()),
        };
        let err = build_connect_options(&cfg).unwrap_err();
        assert!(err.to_string().contains("tls_ca_file"));
    }

    #[test]
    fn tls_plain_mode_builds_options() {
        let cfg = NatsConfig {
            url: "nats://localhost:4222".into(),
            tls: true,
            tls_ca_file: None,
            tls_cert_file: None,
            tls_key_file: None,
        };
        assert!(build_connect_options(&cfg).is_ok());
    }

    #[tokio::test]
    async fn connect_fails_bad_url() {
        let result = NatsMq::connect("nats://127.0.0.1:1").await;
        assert!(result.is_err());
    }

    #[test]
    fn config_defaults_plaintext() {
        let cfg: NatsConfig =
            serde_json::from_value(serde_json::json!({"url": "nats://h"})).unwrap();
        assert!(!cfg.tls);
        assert!(cfg.tls_ca_file.is_none());
        assert!(cfg.tls_cert_file.is_none());
        assert!(cfg.tls_key_file.is_none());
    }

    #[test]
    fn tls_errors_when_ca_file_missing() {
        let cfg = NatsConfig {
            url: "nats://h".into(),
            tls: true,
            tls_ca_file: Some("/no-such-ca.pem".into()),
            tls_cert_file: None,
            tls_key_file: None,
        };
        let err = build_connect_options(&cfg).unwrap_err();
        assert!(err.to_string().contains("read ca"), "got: {err}");
    }

    #[test]
    fn tls_errors_when_cert_or_key_file_missing() {
        // CA 先于 cert 读取，先落地一个可解析的 ca 文件让错误落在 cert 读取上
        let ca = std::env::temp_dir().join(format!("ecat-nats-ca-{}", std::process::id()));
        std::fs::write(&ca, TEST_CA_PEM).unwrap();
        let cfg = NatsConfig {
            url: "nats://h".into(),
            tls: true,
            tls_ca_file: Some(ca.to_str().unwrap().into()),
            tls_cert_file: Some("/no-such-cert.pem".into()),
            tls_key_file: Some("/no-such-key.pem".into()),
        };
        let err = build_connect_options(&cfg).unwrap_err();
        assert!(err.to_string().contains("read cert"), "got: {err}");
        let _ = std::fs::remove_file(&ca);
    }

    const TEST_CA_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIDCTCCAfGgAwIBAgIUNSLyimXxkzffqmPDFPatoQhc8T8wDQYJKoZIhvcNAQEL
BQAwFDESMBAGA1UEAwwJZWNhdC10ZXN0MB4XDTI2MDgyNjEwMzgxOFoXDTM2MDgy
MzEwMzgxOFowFDESMBAGA1UEAwwJZWNhdC10ZXN0MIIBIjANBgkqhkiG9w0BAQEF
AAOCAQ8AMIIBCgKCAQEAuS6OMYElLtSoI4r91d+jPYgYb/HGCXbto+g128NnAx5b
qtQxrEZp6zdqQLlZ2XwREwlPmC3bWHiXR/3SWYc2V3vE8ktmkUl1mnOWiWQb0YXZ
pUxErK77OWVm1nmrv6leeOrL7wv2U5SP+HyUu5bSlqrCrZNNvdIZYbRIFy0+0GNz
HZAJ2EL08hKWL1JycbimRLhhaSZMQrh/+csgHodpgmVsnLwkmiEtfseIVH84LYkH
Ri/ymIYTi33U/kDszJ6U+qUERMoB2wj53PjOm95jxw2KgI+bEg3GsRslmopc3llr
atvaeHHP40ST3Papl297o70znYaMCdzm6MRlC3LSQwIDAQABo1MwUTAdBgNVHQ4E
FgQU6AAofOJXIFa+59UHxwoUvFzybyAwHwYDVR0jBBgwFoAU6AAofOJXIFa+59UH
xwoUvFzybyAwDwYDVR0TAQH/BAUwAwEB/zANBgkqhkiG9w0BAQsFAAOCAQEAb7zC
RXW3uUgQOu3GIdoQTNwSrD7YROQ0/yhwL3WZIn4oWQnntE/Tq8t1i9Zhu2xd1Udj
UMwk9QGAOsJ/ycWJrHEVjgA+x/hkMmM65SqYZkhvuI47zBakr8BJXaNZdd3oR6k/
7Ea+BkWyc3bkkK5VpKgcCDSuuSQIiowMkVskNFlEFciEu+CW/mC4naQ+aFCeUDgj
nui3TBl1jEZte/XtniJE1ERalanpdEcRu/ex6NsUrpN3s94bdeVvzCZxghRTzBcW
mxELYjL4RLXGjiSyLUi96jXOprRuU4ZSauBx+8Ap5ZbRRvAy6SJFDgeNFYk4MbVQ
Liyq7reczvbYaICvzA==
-----END CERTIFICATE-----";
}
