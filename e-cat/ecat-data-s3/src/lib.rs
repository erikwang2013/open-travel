// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
//! S3 / MinIO object storage client (reqwest + rustls, AWS SigV4 signing).
//!
//! TLS is handled through the shared [`ecat_tls::TlsClientConfig`] surface
//! (custom CA, mTLS, skip_verify), consistent with the other HTTP data
//! crates. `endpoint` without a scheme defaults to `https://`; prefix an
//! explicit `http://` (e.g. local MinIO) to opt out. `tls.skip_verify`
//! covers the "insecure" case.
//!
//! Requests are signed with AWS Signature V4 (path-style addressing) and
//! every response status is checked — non-2xx responses surface the status
//! and body instead of being silently dropped.

mod signing;
mod xml;

use async_trait::async_trait;
use ecat_data::StorageClient;
use ecat_errors::{Error as StorageError, ErrorCode};
use ecat_tls::TlsClientConfig;
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use signing::{Credentials, SigTime, canonical_query, encode_uri_component, hex, sign};
use time::OffsetDateTime;

#[derive(Debug, Clone, Deserialize)]
pub struct S3Config {
    pub endpoint: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    #[serde(default)]
    pub tls: Option<TlsClientConfig>,
}

pub struct S3Client {
    client: reqwest::Client,
    endpoint: String,
    host: String,
    region: String,
    access_key: String,
    secret_key: String,
}

impl S3Client {
    pub fn from_config(cfg: S3Config) -> Result<Self, StorageError> {
        let client = ecat_tls::build_reqwest_client(&cfg.tls)
            .map_err(|e| StorageError::new(ErrorCode::Internal, "s3", format!("s3 tls: {e}")))?;
        // 无 scheme 的 endpoint 默认 https（凭据走加密链路）；显式
        // "http://" 前缀是唯一的明文 opt-out（本地 MinIO 开发）。
        let endpoint = if cfg.endpoint.contains("://") {
            cfg.endpoint.clone()
        } else {
            format!("https://{}", cfg.endpoint)
        };
        let host = endpoint
            .strip_prefix("https://")
            .or_else(|| endpoint.strip_prefix("http://"))
            .unwrap_or(&endpoint)
            .to_string();
        Ok(Self {
            client,
            endpoint,
            host,
            region: cfg.region,
            access_key: cfg.access_key,
            secret_key: cfg.secret_key,
        })
    }

    /// 返回原始（未编码）路径；编码统一在 signed_request 的 URL 构建与
    /// sign 的 canonical URI 处各做一次，避免双重 percent-encoding。
    fn object_path(&self, bucket: &str, key: &str) -> String {
        format!("/{bucket}/{key}")
    }

    fn signed_request(
        &self,
        method: &str,
        path: &str,
        query: &[(&str, &str)],
        payload: &[u8],
    ) -> (String, String, String, String) {
        let now = OffsetDateTime::now_utc();
        let payload_hash = hex(&Sha256::digest(payload));
        let time = SigTime {
            amz_date: now.format(&signing::AMZ_DATE_FMT).expect("amz date format"),
            date_stamp: now
                .format(&signing::DATE_STAMP_FMT)
                .expect("date stamp format"),
        };
        let auth = sign(
            method,
            &self.host,
            path,
            query,
            &payload_hash,
            &Credentials {
                access_key: &self.access_key,
                secret_key: &self.secret_key,
                region: &self.region,
            },
            &time,
        );
        let q = canonical_query(query);
        let url = if q.is_empty() {
            format!("{}{}", self.endpoint, encode_uri_component(path, true))
        } else {
            format!("{}{}?{q}", self.endpoint, encode_uri_component(path, true))
        };
        (url, auth, time.amz_date, payload_hash)
    }

    async fn check_status(
        resp: reqwest::Response,
        op: &str,
    ) -> Result<reqwest::Response, StorageError> {
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(StorageError::new(
                ErrorCode::Internal,
                "s3",
                format!("s3 {op}: HTTP {status}: {body}"),
            ));
        }
        Ok(resp)
    }
}

#[async_trait]
impl StorageClient for S3Client {
    async fn put(&self, bucket: &str, key: &str, data: &[u8]) -> Result<(), StorageError> {
        let path = self.object_path(bucket, key);
        let (url, auth, amz_date, payload_hash) = self.signed_request("PUT", &path, &[], data);
        let resp = self
            .client
            .put(url)
            .header(AUTHORIZATION, auth)
            .header("x-amz-date", amz_date)
            .header("x-amz-content-sha256", payload_hash)
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| StorageError::new(ErrorCode::Internal, "s3", format!("s3 put: {e}")))?;
        Self::check_status(resp, "put").await?;
        Ok(())
    }

    async fn get(&self, bucket: &str, key: &str) -> Result<Vec<u8>, StorageError> {
        let path = self.object_path(bucket, key);
        let (url, auth, amz_date, payload_hash) = self.signed_request("GET", &path, &[], b"");
        let resp = self
            .client
            .get(url)
            .header(AUTHORIZATION, auth)
            .header("x-amz-date", amz_date)
            .header("x-amz-content-sha256", payload_hash)
            .send()
            .await
            .map_err(|e| StorageError::new(ErrorCode::Internal, "s3", format!("s3 get: {e}")))?;
        let resp = Self::check_status(resp, "get").await?;
        Ok(resp
            .bytes()
            .await
            .map_err(|e| StorageError::new(ErrorCode::Internal, "s3", format!("s3 get body: {e}")))?
            .to_vec())
    }

    async fn delete(&self, bucket: &str, key: &str) -> Result<(), StorageError> {
        let path = self.object_path(bucket, key);
        let (url, auth, amz_date, payload_hash) = self.signed_request("DELETE", &path, &[], b"");
        let resp = self
            .client
            .delete(url)
            .header(AUTHORIZATION, auth)
            .header("x-amz-date", amz_date)
            .header("x-amz-content-sha256", payload_hash)
            .send()
            .await
            .map_err(|e| StorageError::new(ErrorCode::Internal, "s3", format!("s3 delete: {e}")))?;
        Self::check_status(resp, "delete").await?;
        Ok(())
    }

    /// List object keys under `prefix`, following continuation tokens across
    /// pages (same behavior as the previous rust-s3 backend).
    async fn list(&self, bucket: &str, prefix: &str) -> Result<Vec<String>, StorageError> {
        let path = format!("/{bucket}");
        let mut keys = Vec::new();
        let mut token: Option<String> = None;
        loop {
            let mut query: Vec<(&str, &str)> = vec![("list-type", "2"), ("prefix", prefix)];
            let token_owned;
            if let Some(t) = &token {
                token_owned = t.clone();
                query.push(("continuation-token", &token_owned));
            }
            let (url, auth, amz_date, payload_hash) =
                self.signed_request("GET", &path, &query, b"");
            let resp = self
                .client
                .get(url)
                .header(AUTHORIZATION, auth)
                .header("x-amz-date", amz_date)
                .header("x-amz-content-sha256", payload_hash)
                .send()
                .await
                .map_err(|e| {
                    StorageError::new(ErrorCode::Internal, "s3", format!("s3 list: {e}"))
                })?;
            let resp = Self::check_status(resp, "list").await?;
            let body = resp.text().await.map_err(|e| {
                StorageError::new(ErrorCode::Internal, "s3", format!("s3 list body: {e}"))
            })?;
            let (page_keys, next) = xml::parse_list_xml(&body);
            keys.extend(page_keys);
            match next {
                Some(t) => token = Some(t),
                None => break,
            }
        }
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn config_deserializes_with_tls() {
        let cfg: S3Config = serde_json::from_value(serde_json::json!({
            "endpoint": "localhost:9000",
            "region": "us-east-1",
            "access_key": "minioadmin",
            "secret_key": "minioadmin",
            "tls": {"skip_verify": true},
        }))
        .unwrap();
        assert_eq!(cfg.region, "us-east-1");
        assert!(cfg.tls.unwrap().skip_verify == Some(true));
    }

    #[test]
    fn client_defaults_to_https_without_scheme() {
        let client = S3Client::from_config(S3Config {
            endpoint: "localhost:9000".into(),
            region: "us-east-1".into(),
            access_key: "minioadmin".into(),
            secret_key: "minioadmin".into(),
            tls: None,
        })
        .unwrap();
        assert_eq!(client.endpoint, "https://localhost:9000");
        assert_eq!(client.host, "localhost:9000");
    }

    #[test]
    fn client_keeps_explicit_http_for_local_dev() {
        let client = S3Client::from_config(S3Config {
            endpoint: "http://localhost:9000".into(),
            region: "us-east-1".into(),
            access_key: "minioadmin".into(),
            secret_key: "minioadmin".into(),
            tls: None,
        })
        .unwrap();
        assert_eq!(client.endpoint, "http://localhost:9000");
        assert_eq!(client.host, "localhost:9000");
    }

    #[test]
    fn client_constructs_https_when_tls_enabled() {
        let client = S3Client::from_config(S3Config {
            endpoint: "localhost:9000".into(),
            region: "us-east-1".into(),
            access_key: "a".into(),
            secret_key: "b".into(),
            tls: Some(TlsClientConfig {
                ca_cert: None,
                client_cert: None,
                client_key: None,
                skip_verify: Some(true),
            }),
        })
        .unwrap();
        assert_eq!(client.endpoint, "https://localhost:9000");
    }

    fn test_client() -> S3Client {
        S3Client::from_config(S3Config {
            endpoint: "localhost:9000".into(),
            region: "us-east-1".into(),
            access_key: "minioadmin".into(),
            secret_key: "minioadmin".into(),
            tls: None,
        })
        .unwrap()
    }

    #[test]
    fn object_path_returns_raw_key() {
        let client = test_client();
        assert_eq!(
            client.object_path("bucket", "a b#c?d%e.txt"),
            "/bucket/a b#c?d%e.txt"
        );
    }

    #[test]
    fn signed_request_url_encodes_path_exactly_once() {
        let client = test_client();
        let path = client.object_path("bucket", "a b#c?d%e.txt");
        let (url, _, _, _) = client.signed_request("PUT", &path, &[], b"data");
        assert!(url.contains("/bucket/a%20b%23c%3Fd%25e.txt"), "url: {url}");
        assert!(!url.contains("%2520"), "double encoding: {url}");
    }

    #[test]
    fn signed_request_returns_headers_matching_signature() {
        let client = test_client();
        let path = client.object_path("bucket", "key");
        let (_, auth, amz_date, payload_hash) = client.signed_request("PUT", &path, &[], b"data");
        // 签名使用的时间与 payload 哈希必须与请求装配值一致（同一来源）。
        let expected_hash = hex(&Sha256::digest(b"data"));
        assert_eq!(payload_hash, expected_hash);
        assert!(
            amz_date.ends_with('Z') && amz_date.len() == 16,
            "amz_date: {amz_date}"
        );
        // Authorization 的 SignedHeaders 与 credential scope 使用同一 amz_date。
        let scope_date = amz_date[..8].to_string();
        assert!(auth.contains(&format!("{scope_date}/us-east-1/s3/aws4_request")));
        assert!(auth.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date"));
        // 空 payload（GET/DELETE/list）哈希固定。
        let (_, _, _, empty_hash) = client.signed_request("GET", &path, &[], b"");
        assert_eq!(
            empty_hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[tokio::test]
    async fn put_surfaces_http_error_status() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut sock, _)) = listener.accept() {
                let mut buf = [0u8; 8192];
                let _ = sock.read(&mut buf);
                let _ = sock.write_all(
                    b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
        });
        let client = S3Client::from_config(S3Config {
            endpoint: format!("http://{addr}"),
            region: "us-east-1".into(),
            access_key: "a".into(),
            secret_key: "b".into(),
            tls: None,
        })
        .unwrap();
        let err = client.put("bucket", "key", b"data").await.unwrap_err();
        assert!(err.to_string().contains("HTTP 500"), "got: {err}");
    }

    #[tokio::test]
    async fn requests_carry_all_signed_headers() {
        // 请求装配层：SignedHeaders 列出的头必须实际出现在请求中，
        // 否则真实 S3 对所有操作返回 403 SignatureDoesNotMatch。
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (got_tx, got_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Ok((mut sock, _)) = listener.accept() {
                let mut buf = [0u8; 8192];
                let n = sock.read(&mut buf).unwrap();
                let _ = got_tx.send(String::from_utf8_lossy(&buf[..n]).to_string());
                let _ = sock.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
        });
        let client = S3Client::from_config(S3Config {
            endpoint: format!("http://{addr}"),
            region: "us-east-1".into(),
            access_key: "a".into(),
            secret_key: "b".into(),
            tls: None,
        })
        .unwrap();
        client.put("bucket", "key", b"data").await.unwrap();
        let raw = got_rx.recv().unwrap();
        let (head, _) = raw.split_once("\r\n\r\n").unwrap();
        let auth = head
            .lines()
            .find_map(|l| l.strip_prefix("authorization: "))
            .or_else(|| head.lines().find_map(|l| l.strip_prefix("Authorization: ")))
            .unwrap();
        let signed = auth
            .split(", ")
            .find_map(|p| p.strip_prefix("SignedHeaders="))
            .unwrap();
        for name in signed.split(';') {
            assert!(
                head.to_ascii_lowercase().contains(&format!("{name}:")),
                "missing signed header {name} in:\n{head}"
            );
        }
        // payload 哈希头与 body 一致。
        assert!(head.contains(
            "x-amz-content-sha256: 3a6eb0790f39ac87c94f3856b2dd2c5d110e6811602261a9a923d3bb23adc8b7"
        ), "hash mismatch in:\n{head}");
    }
}
