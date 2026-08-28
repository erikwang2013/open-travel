// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use serde::Deserialize;
use std::sync::OnceLock;
use std::time::Duration;

/// 数据库连接默认超时：连接 5s、整体请求 30s，防止后端挂起时请求永久悬挂
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// 安装默认 rustls CryptoProvider（ring）。
/// reqwest 以 rustls-tls-no-provider 编译时不会自带 provider，未安装的进程在
/// 构造 TLS 客户端时会 panic；与 ecat-transport-* 的安装保持一致（首装生效）。
fn ensure_crypto_provider() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let _ = rustls::crypto::CryptoProvider::install_default(
            rustls::crypto::ring::default_provider(),
        );
    });
}

/// TLS client configuration for database connections.
/// All fields optional — omit to skip TLS entirely.
#[derive(Debug, Clone, Deserialize)]
pub struct TlsClientConfig {
    #[serde(default)]
    pub ca_cert: Option<String>,
    #[serde(default)]
    pub client_cert: Option<String>,
    #[serde(default)]
    pub client_key: Option<String>,
    /// 跳过服务器证书校验（danger_accept_invalid_certs）。仅限测试/开发环境，
    /// 生产配置请勿开启。与 ca_cert 互斥：同时配置时构建客户端会报错，
    /// 防止误配静默关闭证书校验。
    #[serde(default)]
    pub skip_verify: Option<bool>,
}

impl TlsClientConfig {
    pub fn is_enabled(&self) -> bool {
        self.ca_cert.is_some()
            || self.client_cert.is_some()
            || self.client_key.is_some()
            || self.skip_verify == Some(true)
    }

    pub fn build_reqwest_client(&self) -> Result<reqwest::Client, String> {
        ensure_crypto_provider();
        // S5：skip_verify 与 ca_cert 是矛盾配置（跳过校验却配置信任锚），
        // 构建时拒绝，防止误配静默关闭证书校验。
        if self.skip_verify == Some(true) && self.ca_cert.is_some() {
            return Err(
                "skip_verify=true cannot be combined with ca_cert: certificate verification \
                 would be disabled while a trust anchor is configured"
                    .into(),
            );
        }

        let mut builder = reqwest::Client::builder().use_rustls_tls();

        if self.skip_verify == Some(true) {
            builder = builder.danger_accept_invalid_certs(true);
        }

        if let Some(ref ca_path) = self.ca_cert {
            let ca_bytes = std::fs::read(ca_path).map_err(|e| format!("read ca {ca_path}: {e}"))?;
            let ca = reqwest::tls::Certificate::from_pem(&ca_bytes)
                .map_err(|e| format!("parse ca: {e}"))?;
            builder = builder.add_root_certificate(ca);
        }

        if let (Some(cert_path), Some(key_path)) = (&self.client_cert, &self.client_key) {
            let cert_bytes =
                std::fs::read(cert_path).map_err(|e| format!("read cert {cert_path}: {e}"))?;
            let key_bytes =
                std::fs::read(key_path).map_err(|e| format!("read key {key_path}: {e}"))?;
            let id = reqwest::tls::Identity::from_pem(&[cert_bytes, key_bytes].concat())
                .map_err(|e| format!("parse identity: {e}"))?;
            builder = builder.identity(id);
        }

        builder
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| format!("build tls client: {e}"))
    }
}

/// Build a `reqwest::Client` with optional TLS configuration.
/// Returns a default client when `tls` is `None` or not enabled.
pub fn build_reqwest_client(tls: &Option<TlsClientConfig>) -> Result<reqwest::Client, String> {
    ensure_crypto_provider();
    match tls {
        Some(cfg) if cfg.is_enabled() => cfg.build_reqwest_client(),
        _ => reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| format!("build client: {e}")),
    }
}

/// Apply basic auth to a request builder when both username and password are set.
pub fn apply_basic_auth(
    req: reqwest::RequestBuilder,
    username: &Option<String>,
    password: &Option<String>,
) -> reqwest::RequestBuilder {
    match (username, password) {
        (Some(u), Some(p)) => req.basic_auth(u, Some(p)),
        _ => req,
    }
}

// ── Certificate Generation ──────────────────────────────────

#[derive(Debug, Clone)]
pub struct CertPair {
    pub cert_pem: String,
    pub key_pem: String,
}

pub type CaCert = CertPair;

/// Generate a self-signed CA certificate (PEM).
pub fn generate_ca(organization: &str) -> Result<CaCert, String> {
    let mut params = rcgen::CertificateParams::default();
    params
        .distinguished_name
        .push(rcgen::DnType::OrganizationName, organization);
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, format!("{organization} CA"));
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages = vec![
        rcgen::KeyUsagePurpose::KeyCertSign,
        rcgen::KeyUsagePurpose::CrlSign,
    ];

    let key = rcgen::KeyPair::generate().map_err(|e| format!("key: {e}"))?;
    let cert = params.self_signed(&key).map_err(|e| format!("ca: {e}"))?;

    Ok(CertPair {
        cert_pem: cert.pem(),
        key_pem: key.serialize_pem(),
    })
}

/// Generate a self-signed server certificate (PEM).
pub fn generate_server_cert(hostname: &str) -> Result<CertPair, String> {
    let subject_alt_names = vec![hostname.to_string()];
    let ck = rcgen::generate_simple_self_signed(subject_alt_names)
        .map_err(|e| format!("server cert: {e}"))?;

    Ok(CertPair {
        cert_pem: ck.cert.pem(),
        key_pem: ck.signing_key.serialize_pem(),
    })
}

/// Generate a self-signed client certificate (PEM) for mTLS.
pub fn generate_client_cert(common_name: &str) -> Result<CertPair, String> {
    let mut params = rcgen::CertificateParams::new(vec![common_name.to_string()])
        .map_err(|e| format!("params: {e}"))?;
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, common_name);
    params
        .extended_key_usages
        .push(rcgen::ExtendedKeyUsagePurpose::ClientAuth);

    let key = rcgen::KeyPair::generate().map_err(|e| format!("key: {e}"))?;
    let cert = params
        .self_signed(&key)
        .map_err(|e| format!("client cert: {e}"))?;

    Ok(CertPair {
        cert_pem: cert.pem(),
        key_pem: key.serialize_pem(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_ca_works() {
        let ca = generate_ca("TestOrg").unwrap();
        assert!(ca.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(ca.key_pem.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn generate_server_cert_works() {
        let srv = generate_server_cert("localhost").unwrap();
        assert!(srv.cert_pem.contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn generate_client_cert_works() {
        let client = generate_client_cert("myapp").unwrap();
        assert!(client.cert_pem.contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn tls_config_not_enabled_by_default() {
        let cfg: TlsClientConfig = serde_json::from_str(r#"{}"#).unwrap();
        assert!(!cfg.is_enabled());
    }

    #[test]
    fn tls_config_deserializes() {
        let cfg: TlsClientConfig = serde_json::from_str(
            r#"{"ca_cert":"/ca.pem","client_cert":"/cert.pem","client_key":"/key.pem"}"#,
        )
        .unwrap();
        assert!(cfg.is_enabled());
    }

    /// S5：skip_verify 单独设置即视为 TLS 启用（显式跳过校验的合法用法）。
    #[test]
    fn skip_verify_alone_enables_tls() {
        let cfg: TlsClientConfig = serde_json::from_str(r#"{"skip_verify": true}"#).unwrap();
        assert!(cfg.is_enabled());
        assert!(
            cfg.build_reqwest_client().is_ok(),
            "skip_verify alone must build a client"
        );
    }

    /// S5：skip_verify 与 ca_cert 同时配置是矛盾配置（校验被跳过却配置信任锚），
    /// 构建客户端必须报错，防止误配静默关闭证书校验。
    #[test]
    fn skip_verify_conflicts_with_ca_cert() {
        let cfg: TlsClientConfig =
            serde_json::from_str(r#"{"skip_verify": true, "ca_cert": "/nonexistent/ca.pem"}"#)
                .unwrap();
        let err = cfg.build_reqwest_client().unwrap_err();
        assert!(
            err.contains("skip_verify"),
            "expected skip_verify conflict error, got: {err}"
        );
    }

    #[test]
    fn missing_ca_file_reports_read_error() {
        let cfg: TlsClientConfig =
            serde_json::from_str(r#"{"ca_cert": "/nonexistent/ecat-ca.pem"}"#).unwrap();
        let err = cfg.build_reqwest_client().unwrap_err();
        assert!(err.contains("read ca"), "got: {err}");
    }

    #[test]
    fn each_field_alone_enables_tls() {
        let ca: TlsClientConfig = serde_json::from_str(r#"{"ca_cert": "/ca.pem"}"#).unwrap();
        let cert: TlsClientConfig = serde_json::from_str(r#"{"client_cert": "/c.pem"}"#).unwrap();
        let key: TlsClientConfig = serde_json::from_str(r#"{"client_key": "/k.pem"}"#).unwrap();
        assert!(ca.is_enabled());
        assert!(cert.is_enabled());
        assert!(key.is_enabled());
    }

    #[test]
    fn build_default_client_without_tls() {
        let _client = build_reqwest_client(&None).unwrap();
        let cfg: TlsClientConfig = serde_json::from_str(r#"{}"#).unwrap();
        let _client = build_reqwest_client(&Some(cfg)).unwrap();
    }

    #[test]
    fn apply_basic_auth_sets_header_when_both_present() {
        let req = reqwest::Client::new().get("http://example.com");
        let req = apply_basic_auth(req, &Some("user".into()), &Some("pass".into()));
        let auth = req
            .build()
            .unwrap()
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(auth.starts_with("Basic "), "got: {auth}");
    }

    #[test]
    fn apply_basic_auth_skips_when_partial() {
        let client = reqwest::Client::new();
        let req = apply_basic_auth(
            client.get("http://example.com"),
            &Some("user".into()),
            &None,
        );
        assert!(
            req.build()
                .unwrap()
                .headers()
                .get(reqwest::header::AUTHORIZATION)
                .is_none()
        );
        let req = apply_basic_auth(
            client.get("http://example.com"),
            &None,
            &Some("pass".into()),
        );
        assert!(
            req.build()
                .unwrap()
                .headers()
                .get(reqwest::header::AUTHORIZATION)
                .is_none()
        );
    }
}
