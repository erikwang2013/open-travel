// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz

/// Validate a project name for `ecat new`.
///
/// Rejects path separators (`/`, `\`), quotes, newlines and any character
/// outside `^[a-z][a-z0-9_-]*$`, so the name can be safely used as a
/// directory and embedded in a generated Cargo.toml.
pub fn validate_crate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("name must not be empty".into());
    }
    if name.contains(['/', '\\', '"', '\'', '\n', '\r']) {
        return Err("name must not contain path separators, quotes or newlines".into());
    }
    let valid = name.char_indices().all(|(i, c)| {
        (i == 0 && c.is_ascii_lowercase())
            || (i > 0 && (c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'))
    });
    if !valid {
        return Err("name must match ^[a-z][a-z0-9_-]*$ (start with a lowercase letter)".into());
    }
    Ok(())
}

pub fn generate_cargo_toml(name: &str) -> String {
    let ecat_version = env!("CARGO_PKG_VERSION");
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
ecat = "{ecat_version}"
ecat-transport-http = "{ecat_version}"
ecat-middleware = "{ecat_version}"
ecat-logging = "{ecat_version}"
tokio = {{ version = "1", features = ["full"] }}
tracing = "0.1"
axum = "0.8"
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
"#
    )
}

pub fn generate_main_rs() -> &'static str {
    r#"use axum::{routing::get, Json, Router};
use ecat::App;
use ecat_middleware::{LoggingLayer, TracingLayer};
use ecat_transport_http::HttpServer;
use serde::Serialize;
use tower::ServiceBuilder;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let middleware = ServiceBuilder::new()
        .layer(TracingLayer::new(env!("CARGO_PKG_NAME")))
        .layer(LoggingLayer);

    let router = Router::new()
        .route("/health", get(health))
        .layer(middleware);

    let http_srv = HttpServer::new("0.0.0.0:8000").router(router);

    let mut app = App::builder()
        .name(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .server(http_srv)
        .build()?;

    app.run().await?;
    Ok(())
}
"#
}

pub fn generate_proto_file() -> &'static str {
    r#"syntax = "proto3";
package service;

service Service {
    rpc Health(HealthRequest) returns (HealthResponse);
}

message HealthRequest {}
message HealthResponse {
    string status = 1;
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_toml_contains_package_name() {
        let toml = generate_cargo_toml("myapp");
        assert!(toml.contains("name = \"myapp\""));
        assert!(toml.contains("edition = \"2024\""));
    }

    #[test]
    fn cargo_toml_includes_dependencies() {
        let toml = generate_cargo_toml("test-app");
        let ecat_version = env!("CARGO_PKG_VERSION");
        assert!(toml.contains(&format!("ecat = \"{ecat_version}\"")));
        assert!(toml.contains(&format!("ecat-transport-http = \"{ecat_version}\"")));
        assert!(toml.contains("axum = \"0.8\""));
        assert!(toml.contains("tokio"));
    }

    #[test]
    fn cargo_toml_uses_current_ecat_version() {
        let toml = generate_cargo_toml("test-app");
        let ecat_version = env!("CARGO_PKG_VERSION");
        assert!(!toml.contains("ecat = \"1.0\""));
        assert!(toml.contains(&format!("ecat-logging = \"{ecat_version}\"")));
    }

    #[test]
    fn main_rs_contains_health_endpoint() {
        let src = generate_main_rs();
        assert!(src.contains("/health"));
        assert!(src.contains("HealthResponse"));
        assert!(src.contains("HttpServer::new"));
    }

    #[test]
    fn main_rs_uses_middleware() {
        let src = generate_main_rs();
        assert!(src.contains("TracingLayer"));
        assert!(src.contains("LoggingLayer"));
    }

    #[test]
    fn proto_file_has_service_definition() {
        let proto = generate_proto_file();
        assert!(proto.contains("service Service"));
        assert!(proto.contains("HealthRequest"));
        assert!(proto.contains("HealthResponse"));
        assert!(proto.contains("syntax = \"proto3\""));
    }

    #[test]
    fn proto_file_valid_syntax() {
        let proto = generate_proto_file();
        assert!(proto.starts_with("syntax = \"proto3\";"));
        assert!(proto.contains("message HealthRequest {}"));
    }

    #[test]
    fn cargo_toml_special_chars_in_name() {
        let toml = generate_cargo_toml("my_app-123");
        assert!(toml.contains("name = \"my_app-123\""));
    }

    #[test]
    fn validate_crate_name_accepts_valid_names() {
        assert!(validate_crate_name("myapp").is_ok());
        assert!(validate_crate_name("my-app-123").is_ok());
        assert!(validate_crate_name("my_app").is_ok());
    }

    #[test]
    fn validate_crate_name_rejects_path_traversal() {
        assert!(validate_crate_name("../../x").is_err());
        assert!(validate_crate_name("a/b").is_err());
        assert!(validate_crate_name("a\\b").is_err());
    }

    #[test]
    fn validate_crate_name_rejects_injection_chars() {
        assert!(validate_crate_name("a\"b").is_err());
        assert!(validate_crate_name("a'b").is_err());
        assert!(validate_crate_name("a\nb").is_err());
        assert!(validate_crate_name("a\rb").is_err());
    }

    #[test]
    fn validate_crate_name_rejects_bad_patterns() {
        assert!(validate_crate_name("").is_err());
        assert!(validate_crate_name("1app").is_err());
        assert!(validate_crate_name("App").is_err());
        assert!(validate_crate_name("app name").is_err());
        assert!(validate_crate_name("app.name").is_err());
    }
}
