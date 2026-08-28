// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use std::collections::HashMap;

pub const TRACE_ID: &str = "x-ecat-trace-id";
pub const SERVICE_NAME: &str = "x-ecat-service";
pub const CLIENT_IP: &str = "x-ecat-client-ip";

#[derive(Debug, Clone, Default)]
pub struct Metadata {
    inner: HashMap<String, String>,
}

impl Metadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.inner.get(key).map(|v| v.as_str())
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.inner.insert(key.into(), value.into());
    }

    pub fn trace_id(&self) -> Option<&str> {
        self.get(TRACE_ID)
    }
}

// HTTP header -> Metadata
impl From<&http::HeaderMap> for Metadata {
    fn from(headers: &http::HeaderMap) -> Self {
        let mut m = Metadata::new();
        for (k, v) in headers.iter() {
            if let Ok(val) = v.to_str() {
                m.set(k.as_str(), val);
            }
        }
        m
    }
}

// gRPC metadata -> Metadata
impl From<&tonic::metadata::MetadataMap> for Metadata {
    fn from(map: &tonic::metadata::MetadataMap) -> Self {
        let mut m = Metadata::new();
        for entry in map.iter() {
            use tonic::metadata::KeyAndValueRef;
            match entry {
                KeyAndValueRef::Ascii(key, value) => {
                    if let Ok(val) = value.to_str() {
                        m.set(key.as_str(), val);
                    }
                }
                KeyAndValueRef::Binary(_, _) => {}
            }
        }
        m
    }
}

impl IntoIterator for Metadata {
    type Item = (String, String);
    type IntoIter = std::collections::hash_map::IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        let m = Metadata::new();
        assert!(m.get("anything").is_none());
        assert!(m.trace_id().is_none());
    }

    #[test]
    fn set_and_get() {
        let mut m = Metadata::new();
        m.set("key1", "val1");
        m.set("key2".to_string(), "val2".to_string());
        assert_eq!(m.get("key1"), Some("val1"));
        assert_eq!(m.get("key2"), Some("val2"));
        assert_eq!(m.get("missing"), None);
    }

    #[test]
    fn set_overwrites() {
        let mut m = Metadata::new();
        m.set("key", "old");
        m.set("key", "new");
        assert_eq!(m.get("key"), Some("new"));
    }

    #[test]
    fn trace_id_returns_value() {
        let mut m = Metadata::new();
        assert!(m.trace_id().is_none());
        m.set(TRACE_ID, "abc-123");
        assert_eq!(m.trace_id(), Some("abc-123"));
    }

    #[test]
    fn from_http_header_map() {
        let mut headers = http::HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("x-custom", "custom-val".parse().unwrap());

        let m = Metadata::from(&headers);
        assert_eq!(m.get("content-type"), Some("application/json"));
        assert_eq!(m.get("x-custom"), Some("custom-val"));
    }

    #[test]
    fn from_http_header_map_skips_non_utf8() {
        let mut headers = http::HeaderMap::new();
        headers.insert("valid", "ok".parse().unwrap());
        headers.insert(
            "binary",
            http::HeaderValue::from_bytes(b"\xff\xfe").unwrap(),
        );

        let m = Metadata::from(&headers);
        assert_eq!(m.get("valid"), Some("ok"));
        assert!(m.get("binary").is_none());
    }

    #[test]
    fn from_tonic_metadata_map_ascii() {
        let mut map = tonic::metadata::MetadataMap::new();
        map.insert("grpc-key", "grpc-val".parse().unwrap());

        let m = Metadata::from(&map);
        assert_eq!(m.get("grpc-key"), Some("grpc-val"));
    }

    #[test]
    fn from_tonic_metadata_map_binary_skipped() {
        let mut map = tonic::metadata::MetadataMap::new();
        map.insert_bin(
            "bin-key-bin",
            tonic::metadata::MetadataValue::from_bytes(b"\x00\x01\x02"),
        );

        let m = Metadata::from(&map);
        assert!(m.get("bin-key-bin").is_none());
    }

    #[test]
    fn into_iter_yields_all_pairs() {
        let mut m = Metadata::new();
        m.set("a", "1");
        m.set("b", "2");

        let mut pairs: Vec<(String, String)> = m.into_iter().collect();
        pairs.sort();
        assert_eq!(
            pairs,
            vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
            ]
        );
    }

    #[test]
    fn trace_id_propagates_from_http_headers() {
        let mut headers = http::HeaderMap::new();
        headers.insert(TRACE_ID, "t-123".parse().unwrap());
        let m = Metadata::from(&headers);
        assert_eq!(m.trace_id(), Some("t-123"));
    }

    #[test]
    fn http_header_keys_are_lowercased() {
        let mut headers = http::HeaderMap::new();
        headers.insert("X-Trace-Id", "abc".parse().unwrap());
        let m = Metadata::from(&headers);
        assert_eq!(m.get("x-trace-id"), Some("abc"));
    }

    #[test]
    fn empty_header_map_yields_empty_metadata() {
        let m = Metadata::from(&http::HeaderMap::new());
        assert!(m.into_iter().next().is_none());
    }
}
