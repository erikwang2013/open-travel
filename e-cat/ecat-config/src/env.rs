// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use super::{ConfigError, ConfigSource};
use async_trait::async_trait;
use std::collections::HashMap;

pub struct EnvSource {
    prefix: String,
}

impl EnvSource {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

fn parse_env_value(s: &str) -> serde_json::Value {
    if s.eq_ignore_ascii_case("true") {
        return serde_json::Value::Bool(true);
    }
    if s.eq_ignore_ascii_case("false") {
        return serde_json::Value::Bool(false);
    }
    if let Ok(n) = s.parse::<i64>() {
        return serde_json::Value::Number(n.into());
    }
    if let Ok(n) = s.parse::<f64>()
        && let Some(num) = serde_json::Number::from_f64(n)
    {
        return serde_json::Value::Number(num);
    }
    serde_json::Value::String(s.to_string())
}

#[async_trait]
impl ConfigSource for EnvSource {
    async fn load(&self) -> Result<HashMap<String, serde_json::Value>, ConfigError> {
        let mut map = HashMap::new();
        for (key, value) in std::env::vars() {
            if key.starts_with(&self.prefix) {
                let k = key[self.prefix.len()..].to_lowercase();
                let v = parse_env_value(&value);
                map.insert(k, v);
            }
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bool_true() {
        assert_eq!(parse_env_value("true"), serde_json::Value::Bool(true));
        assert_eq!(parse_env_value("TRUE"), serde_json::Value::Bool(true));
    }

    #[test]
    fn parse_bool_false() {
        assert_eq!(parse_env_value("false"), serde_json::Value::Bool(false));
    }

    #[test]
    fn parse_int() {
        assert_eq!(parse_env_value("42"), serde_json::json!(42));
        assert_eq!(parse_env_value("-1"), serde_json::json!(-1));
    }

    #[test]
    fn parse_float() {
        let v = parse_env_value("3.14");
        assert!(v.is_number());
    }

    #[test]
    fn parse_string_fallback() {
        assert_eq!(
            parse_env_value("hello"),
            serde_json::Value::String("hello".into())
        );
    }

    #[test]
    fn parse_edge_cases() {
        // 前导/尾随空格不是数字：保持字符串
        assert_eq!(
            parse_env_value(" 42"),
            serde_json::Value::String(" 42".into())
        );
        assert_eq!(
            parse_env_value("true "),
            serde_json::Value::String("true ".into())
        );
        // 十六进制/空串：保持字符串
        assert_eq!(
            parse_env_value("0x10"),
            serde_json::Value::String("0x10".into())
        );
        assert_eq!(
            parse_env_value(""),
            serde_json::Value::String(String::new())
        );
        // "-0" 解析为整数 0
        assert_eq!(parse_env_value("-0"), serde_json::json!(0));
        // 科学计数法解析为浮点
        assert_eq!(parse_env_value("1e3"), serde_json::json!(1000.0));
    }

    #[tokio::test]
    async fn env_source_filters_by_prefix() {
        static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
        let _guard = ENV_LOCK.lock().await;

        let prefix = format!("ECAT_TEST_{}_", std::process::id());
        let keys = ["PORT", "FLAG", "NAME", "RATE", "RATE2"];
        for k in keys {
            unsafe { std::env::remove_var(format!("{prefix}{k}")) };
        }
        unsafe {
            std::env::set_var(format!("{prefix}PORT"), "8080");
            std::env::set_var(format!("{prefix}FLAG"), "true");
            std::env::set_var(format!("{prefix}NAME"), "svc");
            std::env::set_var(format!("{prefix}RATE"), "1.5");
            std::env::set_var(format!("{prefix}RATE2"), "2.5.3");
        }

        let map = EnvSource::new(prefix.clone()).load().await.unwrap();
        assert_eq!(map.get("port"), Some(&serde_json::json!(8080)));
        assert_eq!(map.get("flag"), Some(&serde_json::json!(true)));
        assert_eq!(map.get("name"), Some(&serde_json::json!("svc")));
        assert_eq!(map.get("rate"), Some(&serde_json::json!(1.5)));
        assert_eq!(
            map.get("rate2"),
            Some(&serde_json::Value::String("2.5.3".into()))
        );
        assert_eq!(map.len(), 5, "unrelated env vars must be excluded");

        for k in keys {
            unsafe { std::env::remove_var(format!("{prefix}{k}")) };
        }
    }
}
