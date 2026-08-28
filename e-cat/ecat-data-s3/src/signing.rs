// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
//! AWS Signature V4 (SigV4) request signing.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use time::macros::format_description;

pub(crate) const SERVICE: &str = "s3";
pub(crate) const AMZ_DATE_FMT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year][month][day]T[hour][minute][second]Z");
pub(crate) const DATE_STAMP_FMT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year][month][day]");

type HmacSha256 = Hmac<Sha256>;

/// RFC 3986 percent-encoding of a path/query segment. Slashes are kept when
/// `keep_slash` is set (S3 keys use "/" as a path separator), and are encoded
/// to %2F otherwise (query values).
pub(crate) fn encode_uri_component(s: &str, keep_slash: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(b))
            }
            b'/' if keep_slash => out.push('/'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub(crate) fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key).expect("hmac accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

pub(crate) fn canonical_query(query: &[(&str, &str)]) -> String {
    let mut q: Vec<(String, String)> = query
        .iter()
        .map(|(k, v)| {
            (
                encode_uri_component(k, false),
                encode_uri_component(v, false),
            )
        })
        .collect();
    q.sort();
    q.iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

pub(crate) struct Credentials<'a> {
    pub access_key: &'a str,
    pub secret_key: &'a str,
    pub region: &'a str,
}

pub(crate) struct SigTime {
    pub amz_date: String,
    pub date_stamp: String,
}

/// Returns the SigV4 `Authorization` header value. `payload_hash` is the hex
/// SHA-256 of the payload; the caller must send the same value in the
/// `x-amz-content-sha256` request header.
pub(crate) fn sign(
    method: &str,
    host: &str,
    path: &str,
    query: &[(&str, &str)],
    payload_hash: &str,
    creds: &Credentials<'_>,
    time: &SigTime,
) -> String {
    let mut headers: Vec<(String, String)> = [
        ("host", host.to_string()),
        ("x-amz-content-sha256", payload_hash.to_string()),
        ("x-amz-date", time.amz_date.clone()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    headers.sort_by(|a, b| a.0.cmp(&b.0));
    let signed_headers: Vec<String> = headers.iter().map(|(k, _)| k.clone()).collect();
    let canonical_headers = headers
        .iter()
        .map(|(k, v)| format!("{k}:{v}\n"))
        .collect::<String>();
    let canonical_request = format!(
        "{method}\n{}\n{}\n{canonical_headers}\n{}\n{payload_hash}",
        encode_uri_component(path, true),
        canonical_query(query),
        signed_headers.join(";"),
    );
    let scope = format!(
        "{}/{}/{SERVICE}/aws4_request",
        time.date_stamp, creds.region
    );
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{scope}\n{}",
        time.amz_date,
        hex(&Sha256::digest(canonical_request.as_bytes())),
    );
    let k_date = hmac_sha256(
        format!("AWS4{}", creds.secret_key).as_bytes(),
        time.date_stamp.as_bytes(),
    );
    let k_region = hmac_sha256(&k_date, creds.region.as_bytes());
    let k_service = hmac_sha256(&k_region, SERVICE.as_bytes());
    let k_signing = hmac_sha256(&k_service, b"aws4_request");
    let signature = hex(&hmac_sha256(&k_signing, string_to_sign.as_bytes()));
    format!(
        "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={}, Signature={signature}",
        creds.access_key,
        signed_headers.join(";"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const AK: &str = "AKIAIOSFODNN7EXAMPLE";
    const SK: &str = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

    // 期望值由独立的 Python SigV4 参考实现生成，该实现先通过 AWS 官方
    // GetObject 示例（Signature f0e8bdb8...）自检。
    fn auth_for(method: &str, path: &str, query: &[(&str, &str)], payload: &[u8]) -> String {
        sign(
            method,
            "localhost:9000",
            path,
            query,
            &hex(&Sha256::digest(payload)),
            &Credentials {
                access_key: AK,
                secret_key: SK,
                region: "us-east-1",
            },
            &SigTime {
                amz_date: "20130524T000000Z".into(),
                date_stamp: "20130524".into(),
            },
        )
    }

    #[test]
    fn sigv4_put_matches_reference_vector() {
        assert_eq!(
            auth_for("PUT", "/bucket/test.txt", &[], b"Welcome to Amazon S3."),
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
             SignedHeaders=host;x-amz-content-sha256;x-amz-date, \
             Signature=57c36bcbd9ad566ed192c7a24f438759ba5c9f10563fcdb9011bdcf7de314bd6"
        );
    }

    #[test]
    fn sigv4_get_matches_reference_vector() {
        assert_eq!(
            auth_for("GET", "/bucket/test.txt", &[], b""),
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
             SignedHeaders=host;x-amz-content-sha256;x-amz-date, \
             Signature=0170e04fbd0f581a7b0073ce899f93dfe5161eaa474c84e004a1ce7d5d7ed4f9"
        );
    }

    #[test]
    fn sigv4_delete_matches_reference_vector() {
        assert_eq!(
            auth_for("DELETE", "/bucket/test.txt", &[], b""),
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
             SignedHeaders=host;x-amz-content-sha256;x-amz-date, \
             Signature=4335c7bab27a372c2424c4b5792612d54e0b6599e1ef48fa84250001211ab476"
        );
    }

    #[test]
    fn sigv4_list_signs_canonical_query() {
        assert_eq!(
            auth_for(
                "GET",
                "/bucket",
                &[("list-type", "2"), ("prefix", "logs/2026")],
                b""
            ),
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
             SignedHeaders=host;x-amz-content-sha256;x-amz-date, \
             Signature=a6981ac88c050c98ca9ff5f26b690394c6f71ed9888c34e2d3bcb988862ef6d8"
        );
    }

    #[test]
    fn sigv4_signs_percent_encoded_key_path() {
        assert_eq!(
            auth_for("PUT", "/bucket/a b#c?d%e.txt", &[], b"data"),
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
             SignedHeaders=host;x-amz-content-sha256;x-amz-date, \
             Signature=a2fd63590a7e9036b1384bcdf0034d8f714b02c92403642bfea60ba21adb64f9"
        );
    }

    #[test]
    fn encode_uri_component_encodes_reserved_chars() {
        assert_eq!(encode_uri_component("logs-2026", false), "logs-2026");
        assert_eq!(
            encode_uri_component("a/b c#d?e%f", true),
            "a/b%20c%23d%3Fe%25f"
        );
        assert_eq!(
            encode_uri_component("a/b c#d?e%f", false),
            "a%2Fb%20c%23d%3Fe%25f"
        );
        assert_eq!(encode_uri_component("你好", false), "%E4%BD%A0%E5%A5%BD");
    }
}
