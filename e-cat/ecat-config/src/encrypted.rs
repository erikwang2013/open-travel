// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
//
// ObfuscatedSource applies XOR-based obfuscation to config values prefixed with `obfs:`.
// This is NOT encryption — it only protects against accidental exposure in plain-text
// config files. For real encryption use a dedicated secrets manager.
use super::{ConfigError, ConfigSource};
use async_trait::async_trait;
use std::collections::HashMap;

pub struct ObfuscatedSource<S> {
    inner: S,
    key: Vec<u8>,
}

impl<S: ConfigSource> ObfuscatedSource<S> {
    pub fn new(inner: S, key: impl Into<Vec<u8>>) -> Self {
        Self {
            inner,
            key: key.into(),
        }
    }
}

#[async_trait]
impl<S: ConfigSource> ConfigSource for ObfuscatedSource<S> {
    async fn load(&self) -> Result<HashMap<String, serde_json::Value>, ConfigError> {
        let map = self.inner.load().await?;
        let mut deobfuscated = HashMap::new();
        for (k, v) in map {
            if let Some(s) = v.as_str() {
                if let Some(stripped) = s.strip_prefix("obfs:") {
                    let dec = deobfuscate(stripped, &self.key)?;
                    if let Ok(json_val) = serde_json::from_str(&dec) {
                        deobfuscated.insert(k, json_val);
                    } else {
                        deobfuscated.insert(k, serde_json::Value::String(dec));
                    }
                } else {
                    deobfuscated.insert(k, v);
                }
            } else {
                deobfuscated.insert(k, v);
            }
        }
        Ok(deobfuscated)
    }
}

fn deobfuscate(encoded: &str, key: &[u8]) -> Result<String, ConfigError> {
    let bytes = hex_decode(encoded).map_err(|e| ConfigError::Other(format!("hex: {e}")))?;
    if key.is_empty() {
        return Err(ConfigError::Other("empty obfuscation key".into()));
    }
    let result: Vec<u8> = bytes
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect();
    String::from_utf8(result).map_err(|e| ConfigError::Other(format!("utf8: {e}")))
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return Err("odd hex length".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_trait]
    impl ConfigSource for HashMap<String, serde_json::Value> {
        async fn load(&self) -> Result<HashMap<String, serde_json::Value>, ConfigError> {
            Ok(self.clone())
        }
    }

    #[tokio::test]
    async fn deobfuscates_obfuscated_values() {
        let key = b"mykey1234567890";
        let secret = "hello";
        let obfuscated: Vec<u8> = secret
            .bytes()
            .enumerate()
            .map(|(i, b)| b ^ key[i % key.len()])
            .collect();
        let obfs_hex: String = obfuscated.iter().map(|b| format!("{b:02x}")).collect();

        let mut data = HashMap::new();
        data.insert(
            "password".into(),
            serde_json::Value::String(format!("obfs:{obfs_hex}")),
        );
        data.insert("host".into(), serde_json::Value::String("localhost".into()));

        let source = ObfuscatedSource::new(data, key.to_vec());
        let result = source.load().await.unwrap();
        assert_eq!(result.get("password").unwrap().as_str().unwrap(), "hello");
        assert_eq!(result.get("host").unwrap().as_str().unwrap(), "localhost");
    }

    #[test]
    fn deobfuscate_rejects_empty_key() {
        let err = deobfuscate("aa", b"").unwrap_err();
        assert!(err.to_string().contains("empty obfuscation key"));
    }

    #[test]
    fn deobfuscate_rejects_odd_hex() {
        let err = deobfuscate("abc", b"k").unwrap_err();
        assert!(err.to_string().contains("odd hex length"));
    }

    #[test]
    fn deobfuscate_rejects_invalid_hex() {
        let err = deobfuscate("zz", b"k").unwrap_err();
        assert!(err.to_string().contains("hex"));
    }

    #[test]
    fn deobfuscate_rejects_non_utf8() {
        // 0xff ^ 0x00 = 0xff：非法 UTF-8 字节
        let err = deobfuscate("ff", b"\x00").unwrap_err();
        assert!(err.to_string().contains("utf8"));
    }

    #[tokio::test]
    async fn deobfuscated_json_values_become_typed() {
        // 加密后值若本身是合法 JSON（如 "42"），还原为数字而非字符串
        let key = b"x";
        let obfuscated: Vec<u8> = b"42"
            .iter()
            .copied()
            .enumerate()
            .map(|(i, b)| b ^ key[i % key.len()])
            .collect();
        let hex: String = obfuscated.iter().map(|b| format!("{b:02x}")).collect();

        let mut data = HashMap::new();
        data.insert(
            "port".into(),
            serde_json::Value::String(format!("obfs:{hex}")),
        );
        // 非字符串值（布尔）原样透传
        data.insert("flag".into(), serde_json::Value::Bool(true));

        let source = ObfuscatedSource::new(data, key.to_vec());
        let result = source.load().await.unwrap();
        assert_eq!(result.get("port"), Some(&serde_json::json!(42)));
        assert_eq!(result.get("flag"), Some(&serde_json::Value::Bool(true)));
    }
}
