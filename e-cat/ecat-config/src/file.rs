// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use super::{ConfigError, ConfigSource};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct FileSource {
    path: PathBuf,
}

impl FileSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait]
impl ConfigSource for FileSource {
    async fn load(&self) -> Result<HashMap<String, serde_json::Value>, ConfigError> {
        let content = tokio::fs::read_to_string(&self.path)
            .await
            .map_err(|e| ConfigError::Other(format!("read {}: {}", self.path.display(), e)))?;

        let value: serde_json::Value = if self
            .path
            .extension()
            .map_or(false, |e| e == "yaml" || e == "yml")
        {
            yaml_serde::from_str(&content).map_err(|e| ConfigError::Other(e.to_string()))?
        } else {
            serde_json::from_str(&content).map_err(|e| ConfigError::Other(e.to_string()))?
        };

        let map = value
            .as_object()
            .cloned()
            .ok_or_else(|| ConfigError::Other("expected a JSON/YAML object at top level".into()))?;
        Ok(map.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    /// 每测试唯一临时目录（pid + 自增序号），避免并发测试互相覆盖；
    /// 测试结束删除。
    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ecat-config-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn load_parses_json_by_extension() {
        let dir = tempdir();
        let path = dir.join("config.json");
        std::fs::write(&path, r#"{"port": 8080, "name": "api"}"#).unwrap();

        let map = FileSource::new(&path).load().await.unwrap();
        assert_eq!(map.get("port"), Some(&serde_json::json!(8080)));
        assert_eq!(map.get("name"), Some(&serde_json::json!("api")));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn load_parses_yaml_by_extension() {
        let dir = tempdir();
        let path = dir.join("config.yaml");
        std::fs::write(&path, "name: my-app\nport: 8080\n").unwrap();

        let map = FileSource::new(&path).load().await.unwrap();
        assert_eq!(map.get("name"), Some(&serde_json::json!("my-app")));
        assert_eq!(map.get("port"), Some(&serde_json::json!(8080)));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn load_reports_parse_errors() {
        let dir = tempdir();
        let path = dir.join("config.json");
        std::fs::write(&path, "{invalid").unwrap();

        let err = FileSource::new(&path).load().await.unwrap_err();
        assert!(err.to_string().contains("line 1"), "got: {err}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn load_rejects_non_object_top_level() {
        let dir = tempdir();
        let path = dir.join("config.json");
        std::fs::write(&path, r#"[1, 2, 3]"#).unwrap();

        let err = FileSource::new(&path).load().await.unwrap_err();
        assert!(
            err.to_string().contains("expected a JSON/YAML object"),
            "got: {err}"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn load_missing_file_reports_path() {
        let err = FileSource::new("/nonexistent/ecat-config-test.json")
            .load()
            .await
            .unwrap_err();
        assert!(err.to_string().contains("read"), "got: {err}");
    }

    #[tokio::test]
    async fn load_empty_file_reports_error() {
        let dir = tempdir();
        let path = dir.join("config.json");
        std::fs::write(&path, "").unwrap();

        let err = FileSource::new(&path).load().await.unwrap_err();
        assert!(err.to_string().contains("line 1"), "got: {err}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn load_yaml_non_object_top_level_rejected() {
        let dir = tempdir();
        let path = dir.join("config.yaml");
        std::fs::write(&path, "- a\n- b\n").unwrap();

        let err = FileSource::new(&path).load().await.unwrap_err();
        assert!(
            err.to_string().contains("expected a JSON/YAML object"),
            "got: {err}"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn load_invalid_yaml_reports_error() {
        let dir = tempdir();
        let path = dir.join("config.yaml");
        std::fs::write(&path, "a: [unclosed").unwrap();

        let err = FileSource::new(&path).load().await.unwrap_err();
        assert!(!err.to_string().is_empty(), "yaml parse must fail");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
