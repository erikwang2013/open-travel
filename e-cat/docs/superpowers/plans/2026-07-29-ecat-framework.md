<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat Framework Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the e-cat microservices framework — a Rust port of go-kratos/kratos v3 with Cargo workspace structure, tower-based middleware, transport abstraction, service registry, configuration management, data access layer (15 storage backends), and CLI toolchain.

**Architecture:** Cargo workspace with 25+ crates organized in 4 layers: infrastructure (protos/errors/metadata/encoding/logging), core components (transport/middleware/registry/config/metrics), data access (RDBMS/Cache/OLAP/Search/Graph/TSDB), and application orchestration (App lifecycle + CLI).

**Tech Stack:** tokio, axum, tonic, prost, tower, tracing, opentelemetry-rust, prometheus, sqlx, redis-rs, clickhouse-rs, opensearch-rs, elasticsearch-rs, neo4rs, influxdb2, clap

---

## File Structure Map

```
e-cat/                            # workspace root
├── Cargo.toml                    # [workspace] members + [workspace.dependencies]
├── rust-toolchain.toml           # channel = "stable"
├── .github/workflows/ci.yml
│
├── ecat-protos/                  # IDL: shared .proto definitions
│   ├── Cargo.toml
│   ├── proto/
│   │   ├── errors.proto
│   │   └── metadata.proto
│   └── src/lib.rs                # prost-generated + manual extensions
│
├── ecat-errors/                  # Error code system (depends on ecat-protos)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Error type, ErrorCode enum, ErrorBuilder
│       └── codes.rs              # standard error codes
│
├── ecat-metadata/                # Metadata key-value transport
│   ├── Cargo.toml
│   └── src/lib.rs                # Metadata struct, const keys
│
├── ecat-encoding/                # Serialization abstraction
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Codec trait, Encoding enum
│       ├── json.rs               # serde_json Codec
│       └── proto.rs              # prost Codec
│
├── ecat-logging/                 # tracing integration
│   ├── Cargo.toml
│   └── src/lib.rs                # init_tracing(), log macros
│
├── ecat-transport/               # Transport trait + Server trait
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Server trait, Transport enum
│       ├── request.rs            # ecat::Request
│       ├── response.rs           # ecat::Response
│       └── context.rs            # ecat::Context (metadata + trace_id)
│
├── ecat-transport-http/          # axum integration
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # HttpServer, router builder
│       └── codec.rs              # HTTP body decode/encode
│
├── ecat-transport-grpc/          # tonic integration
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # GrpcServer
│       └── codec.rs              # gRPC body decode/encode
│
├── ecat-middleware/              # tower::Layer implementations
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # re-exports
│       ├── recovery.rs           # RecoveryLayer
│       ├── tracing.rs            # TracingLayer
│       ├── logging.rs            # LoggingLayer
│       ├── metrics.rs            # MetricsLayer
│       └── timeout.rs            # TimeoutLayer
│
├── ecat/                         # Core: App lifecycle
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # App, AppBuilder
│       ├── signal.rs             # OS signal handling
│       └── hook.rs               # LifecycleHook trait
│
├── ecat-registry/                # Registry trait
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Registry trait, ServiceInfo
│       └── memory.rs             # in-memory registry
│
├── ecat-registry-etcd/           # etcd implementation
│   ├── Cargo.toml
│   └── src/lib.rs
│
├── ecat-config/                  # ConfigSource trait
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # ConfigSource trait, Config struct
│       ├── file.rs               # file source
│       └── env.rs                # env source
│
├── ecat-metrics/                 # Prometheus integration
│   ├── Cargo.toml
│   └── src/lib.rs
│
├── ecat-data/                    # Data access traits
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # re-exports
│       ├── rdbms.rs              # RdbmsClient trait
│       ├── cache.rs              # Cache trait
│       ├── olap.rs               # OlapClient trait
│       ├── search.rs             # SearchClient trait
│       ├── graph.rs              # GraphClient trait
│       └── tsdb.rs               # TsdbClient trait
│
├── ecat-data-sqlx/               # sqlx implementation
│   ├── Cargo.toml
│   └── src/lib.rs
│
├── ecat-data-redis/              # ... (11 total data contrib crates)
│   └── ...
│
├── ecat-cli/                     # CLI tool
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               # clap app
│       ├── cmd/
│       │   ├── new.rs
│       │   ├── proto.rs
│       │   ├── run.rs
│       │   └── build.rs
│       └── template/             # embedded project template
│
└── examples/
    └── helloworld/
        ├── Cargo.toml
        ├── proto/
        │   └── helloworld.proto
        └── src/
            └── main.rs
```

---

## Phase 1: Project Skeleton & Foundation Crates

### Task 1: Workspace Root Setup

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
[workspace]
members = [
    "ecat-protos",
    "ecat-errors",
    "ecat-metadata",
    "ecat-encoding",
    "ecat-logging",
]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
prost = "0.13"
prost-types = "0.13"
tonic = "0.12"
tonic-build = "0.12"
tower = "0.5"
axum = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
opentelemetry = "0.26"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
async-trait = "0.1"
thiserror = "2"
```

- [ ] **Step 2: Create rust-toolchain.toml**

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

- [ ] **Step 3: Create CI workflow**

File: `.github/workflows/ci.yml`

```yaml
name: CI
on: [push, pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo check --workspace
      - run: cargo test --workspace
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
```

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml rust-toolchain.toml .github/
git commit -m "chore: init workspace skeleton with CI"
```

### Task 2: ecat-protos — Shared Protobuf Definitions

**Files:**
- Create: `ecat-protos/Cargo.toml`
- Create: `ecat-protos/build.rs`
- Create: `ecat-protos/proto/errors.proto`
- Create: `ecat-protos/proto/metadata.proto`
- Create: `ecat-protos/src/lib.rs`

- [ ] **Step 1: Create ecat-protos/Cargo.toml**

```toml
[package]
name = "ecat-protos"
version = "0.1.0"
edition = "2021"

[dependencies]
prost.workspace = true
prost-types.workspace = true

[build-dependencies]
tonic-build.workspace = true
```

- [ ] **Step 2: Create build.rs**

```rust
fn main() {
    tonic_build::configure()
        .compile_protos(
            &["proto/errors.proto", "proto/metadata.proto"],
            &["proto"],
        )
        .unwrap();
}
```

- [ ] **Step 3: Write proto/errors.proto**

```protobuf
syntax = "proto3";
package ecat.errors;

enum ErrorCode {
    OK = 0;
    UNKNOWN = 1000;
    INVALID_ARGUMENT = 1001;
    NOT_FOUND = 1002;
    ALREADY_EXISTS = 1003;
    PERMISSION_DENIED = 1004;
    UNAUTHENTICATED = 1005;
    RESOURCE_EXHAUSTED = 1006;
    INTERNAL = 1007;
    UNAVAILABLE = 1008;
    DEADLINE_EXCEEDED = 1009;
}

message Error {
    int32 code = 1;
    string reason = 2;
    string message = 3;
    map<string, string> metadata = 4;
}
```

- [ ] **Step 4: Write proto/metadata.proto**

```protobuf
syntax = "proto3";
package ecat.metadata;

message Metadata {
    map<string, string> pairs = 1;
}
```

- [ ] **Step 5: Write src/lib.rs**

```rust
pub mod errors {
    tonic::include_proto!("ecat.errors");
}

pub mod metadata {
    tonic::include_proto!("ecat.metadata");
}
```

- [ ] **Step 6: Build and test**

```bash
cd ecat-protos && cargo build
```

Expected: `ecat-protos` compiles with generated protobuf code.

- [ ] **Step 7: Commit**

```bash
git add ecat-protos/
git commit -m "feat(protos): add shared protobuf definitions for errors and metadata"
```

### Task 3: ecat-errors — Error Code System

**Files:**
- Create: `ecat-errors/Cargo.toml`
- Create: `ecat-errors/src/lib.rs`
- Create: `ecat-errors/src/codes.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "ecat-errors"
version = "0.1.0"
edition = "2021"

[dependencies]
ecat-protos = { path = "../ecat-protos" }
prost.workspace = true
thiserror.workspace = true
serde.workspace = true
http = "1"
```

- [ ] **Step 2: Write src/codes.rs**

```rust
use ecat_protos::errors::ErrorCode;

impl ErrorCode {
    pub fn http_status(&self) -> http::StatusCode {
        match self {
            ErrorCode::Ok => http::StatusCode::OK,
            ErrorCode::InvalidArgument => http::StatusCode::BAD_REQUEST,
            ErrorCode::NotFound => http::StatusCode::NOT_FOUND,
            ErrorCode::AlreadyExists => http::StatusCode::CONFLICT,
            ErrorCode::PermissionDenied => http::StatusCode::FORBIDDEN,
            ErrorCode::Unauthenticated => http::StatusCode::UNAUTHORIZED,
            ErrorCode::ResourceExhausted => http::StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::Internal | ErrorCode::Unknown => {
                http::StatusCode::INTERNAL_SERVER_ERROR
            }
            ErrorCode::Unavailable => http::StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::DeadlineExceeded => http::StatusCode::GATEWAY_TIMEOUT,
        }
    }
}
```

- [ ] **Step 3: Write src/lib.rs**

```rust
mod codes;

use ecat_protos::errors::ErrorCode;
use std::collections::HashMap;

#[derive(Debug, Clone, thiserror::Error)]
pub struct Error {
    pub code: ErrorCode,
    pub reason: String,
    #[source]
    pub cause: Option<Box<dyn std::error::Error + Send + Sync>>,
    pub metadata: HashMap<String, String>,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}: {}", self.code, self.reason, self.message())
    }
}

impl Error {
    pub fn new(code: ErrorCode, reason: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code,
            reason: reason.into(),
            cause: Some(message.into().into()),
            metadata: HashMap::new(),
        }
    }

    pub fn message(&self) -> &str {
        self.cause
            .as_ref()
            .map(|e| e.to_string())
            .unwrap_or_default()
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn from_status(status: tonic::Status) -> Self {
        Self::new(ErrorCode::Internal, "grpc_error", status.message())
    }

    pub fn to_http_status(&self) -> http::StatusCode {
        self.code.http_status()
    }
}
```

- [ ] **Step 4: Build and test**

```bash
cargo build -p ecat-errors && cargo test -p ecat-errors
```

- [ ] **Step 5: Commit**

```bash
git add ecat-errors/
git commit -m "feat(errors): add protobuf-based error code system"
```

### Task 4: ecat-metadata — Metadata Transport

**Files:**
- Create: `ecat-metadata/Cargo.toml`
- Create: `ecat-metadata/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "ecat-metadata"
version = "0.1.0"
edition = "2021"

[dependencies]
ecat-protos = { path = "../ecat-protos" }
prost.workspace = true
http = "1"
tonic.workspace = true
```

- [ ] **Step 2: Write src/lib.rs**

```rust
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
        for (k, v) in map.iter() {
            if let (Ok(key), Ok(val)) = (k.to_str(), v.to_str()) {
                m.set(key, val);
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
```

- [ ] **Step 3: Build**

```bash
cargo build -p ecat-metadata
```

- [ ] **Step 4: Commit**

```bash
git add ecat-metadata/
git commit -m "feat(metadata): add metadata key-value transport with HTTP/gRPC conversion"
```

### Task 5: ecat-encoding — Serialization Abstraction

**Files:**
- Create: `ecat-encoding/Cargo.toml`
- Create: `ecat-encoding/src/lib.rs`
- Create: `ecat-encoding/src/json.rs`
- Create: `ecat-encoding/src/proto.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "ecat-encoding"
version = "0.1.0"
edition = "2021"

[dependencies]
serde.workspace = true
serde_json.workspace = true
prost.workspace = true
thiserror.workspace = true
```

- [ ] **Step 2: Write src/lib.rs**

```rust
mod json;
mod proto;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Json,
    Protobuf,
    Form,
}

pub trait Codec: Send + Sync {
    fn encode<T: serde::Serialize>(&self, val: &T) -> Result<Vec<u8>, CodecError>;
    fn decode<T: serde::de::DeserializeOwned>(&self, data: &[u8]) -> Result<T, CodecError>;
    fn content_type(&self) -> &str;
}

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("encode error: {0}")]
    Encode(String),
    #[error("decode error: {0}")]
    Decode(String),
}

pub fn codec_for(encoding: Encoding) -> Box<dyn Codec> {
    match encoding {
        Encoding::Json => Box::new(json::JsonCodec),
        Encoding::Protobuf => Box::new(proto::ProtoCodec),
        Encoding::Form => Box::new(json::JsonCodec),
    }
}

pub fn codec_from_content_type(ct: &str) -> Box<dyn Codec> {
    match ct {
        "application/json" => Box::new(json::JsonCodec),
        "application/protobuf" | "application/x-protobuf" => Box::new(proto::ProtoCodec),
        _ => Box::new(json::JsonCodec),
    }
}
```

- [ ] **Step 3: Write src/json.rs**

```rust
use super::{Codec, CodecError};

pub struct JsonCodec;

impl Codec for JsonCodec {
    fn encode<T: serde::Serialize>(&self, val: &T) -> Result<Vec<u8>, CodecError> {
        serde_json::to_vec(val).map_err(|e| CodecError::Encode(e.to_string()))
    }

    fn decode<T: serde::de::DeserializeOwned>(&self, data: &[u8]) -> Result<T, CodecError> {
        serde_json::from_slice(data).map_err(|e| CodecError::Decode(e.to_string()))
    }

    fn content_type(&self) -> &str {
        "application/json"
    }
}
```

- [ ] **Step 4: Write src/proto.rs**

```rust
use super::{Codec, CodecError};

pub struct ProtoCodec;

impl Codec for ProtoCodec {
    fn encode<T: serde::Serialize>(&self, _val: &T) -> Result<Vec<u8>, CodecError> {
        Err(CodecError::Encode(
            "proto codec requires prost::Message trait".into(),
        ))
    }

    fn decode<T: serde::de::DeserializeOwned>(&self, _data: &[u8]) -> Result<T, CodecError> {
        Err(CodecError::Decode(
            "proto codec requires prost::Message trait".into(),
        ))
    }

    fn content_type(&self) -> &str {
        "application/protobuf"
    }
}
```

- [ ] **Step 5: Build**

```bash
cargo build -p ecat-encoding
```

- [ ] **Step 6: Commit**

```bash
git add ecat-encoding/
git commit -m "feat(encoding): add JSON and Protobuf codec abstraction"
```

### Task 6: ecat-logging — Tracing Setup

**Files:**
- Create: `ecat-logging/Cargo.toml`
- Create: `ecat-logging/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "ecat-logging"
version = "0.1.0"
edition = "2021"

[dependencies]
tracing.workspace = true
tracing-subscriber.workspace = true
opentelemetry.workspace = true
```

- [ ] **Step 2: Write src/lib.rs**

```rust
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init(service_name: &str) {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true)
        .compact();

    let env_layer = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_layer)
        .with(fmt_layer)
        .init();
}
```

- [ ] **Step 3: Build and commit**

```bash
cargo build -p ecat-logging
git add ecat-logging/
git commit -m "feat(logging): add tracing subscriber initialization"
```

### Task 7: Register All Phase 1 Crates in Workspace

- [ ] **Step 1: Verify workspace builds**

```bash
cargo build --workspace
```

Expected: all 5 crates compile without errors.

- [ ] **Step 2: Commit**

```bash
git commit -m "chore: verify Phase 1 workspace build"
```

---

## Phase 2: Transport Layer

### Task 8: ecat-transport — Transport Trait

**Files:**
- Create: `ecat-transport/Cargo.toml`
- Create: `ecat-transport/src/lib.rs`
- Create: `ecat-transport/src/request.rs`
- Create: `ecat-transport/src/response.rs`
- Create: `ecat-transport/src/context.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "ecat-transport"
version = "0.1.0"
edition = "2021"

[dependencies]
ecat-metadata = { path = "../ecat-metadata" }
ecat-encoding = { path = "../ecat-encoding" }
ecat-errors = { path = "../ecat-errors" }
tower.workspace = true
tokio.workspace = true
async-trait.workspace = true
http = "1"
bytes = "1"
```

- [ ] **Step 2: Write src/context.rs**

```rust
use ecat_metadata::Metadata;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Context {
    metadata: Arc<RwLock<Metadata>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            metadata: Arc::new(RwLock::new(Metadata::new())),
        }
    }

    pub async fn trace_id(&self) -> Option<String> {
        self.metadata.read().await.trace_id().map(|s| s.to_string())
    }
}
```

- [ ] **Step 3: Write src/request.rs**

```rust
use ecat_metadata::Metadata;
use http::{HeaderMap, Method, Uri};
use std::collections::HashMap;

pub struct Request<T = ()> {
    pub method: Method,
    pub uri: Uri,
    pub headers: HeaderMap,
    pub metadata: Metadata,
    pub body: T,
    pub params: HashMap<String, String>,
}
```

- [ ] **Step 4: Write src/response.rs**

```rust
use ecat_metadata::Metadata;
use http::StatusCode;

pub struct Response<T = ()> {
    pub status: StatusCode,
    pub headers: http::HeaderMap,
    pub metadata: Metadata,
    pub body: T,
}
```

- [ ] **Step 5: Write src/lib.rs**

```rust
mod context;
mod request;
mod response;

pub use context::Context;
pub use request::Request;
pub use response::Response;

use async_trait::async_trait;

#[async_trait]
pub trait Server: Send + Sync {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}
```

- [ ] **Step 6: Build and commit**

```bash
cargo build -p ecat-transport
git add ecat-transport/
git commit -m "feat(transport): add transport trait, Context, Request, Response"
```

### Task 9: ecat-transport-http — Axum Integration

**Files:**
- Create: `ecat-transport-http/Cargo.toml`
- Create: `ecat-transport-http/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "ecat-transport-http"
version = "0.1.0"
edition = "2021"

[dependencies]
ecat-transport = { path = "../ecat-transport" }
axum.workspace = true
tokio.workspace = true
tower.workspace = true
async-trait.workspace = true
```

- [ ] **Step 2: Write src/lib.rs**

```rust
use axum::Router;
use ecat_transport::Server as TransportServer;
use tokio::net::TcpListener;

pub struct HttpServer {
    addr: String,
    router: Option<Router>,
}

impl HttpServer {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            router: None,
        }
    }

    pub fn router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }
}

#[async_trait::async_trait]
impl TransportServer for HttpServer {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let router = self.router.clone().unwrap_or_else(Router::new);
        let listener = TcpListener::bind(&self.addr).await?;
        axum::serve(listener, router).await?;
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}
```

- [ ] **Step 3: Build and commit**

```bash
cargo build -p ecat-transport-http
git add ecat-transport-http/
git commit -m "feat(transport-http): add axum-based HTTP server"
```

### Task 10: ecat-transport-grpc — Tonic Integration

**Files:**
- Create: `ecat-transport-grpc/Cargo.toml`
- Create: `ecat-transport-grpc/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml** (same pattern, depends on ecat-transport + tonic)

- [ ] **Step 2: Write src/lib.rs** with GrpcServer wrapping tonic::transport::Server

- [ ] **Step 3: Build and commit** `feat(transport-grpc): add tonic-based gRPC server`

---

## Phase 3: Middleware

### Task 11: ecat-middleware — tower::Layer Implementations

**Files:**
- Create: `ecat-middleware/Cargo.toml`
- Create: `ecat-middleware/src/lib.rs`
- Create: `ecat-middleware/src/recovery.rs`
- Create: `ecat-middleware/src/tracing.rs`
- Create: `ecat-middleware/src/logging.rs`
- Create: `ecat-middleware/src/timeout.rs`

- [ ] Implement RecoveryLayer (catch panics via tokio::spawn)
- [ ] Implement TracingLayer (create tracing span per request)
- [ ] Implement LoggingLayer (log method/uri/duration)
- [ ] Implement TimeoutLayer (tokio::time::timeout wrapper)
- [ ] Build and commit: `feat(middleware): add tower::Layer implementations`

---

## Phase 4: App Lifecycle

### Task 12: ecat — Core Application

**Files:**
- Create: `ecat/Cargo.toml`
- Create: `ecat/src/lib.rs`
- Create: `ecat/src/signal.rs`
- Create: `ecat/src/hook.rs`

- [ ] Implement AppBuilder with name/version/server/hooks
- [ ] Implement App::run() — start servers, wait for SIGTERM/SIGINT, graceful shutdown
- [ ] Implement LifecycleHook trait (on_start / on_stop)
- [ ] Implement signal handling (ctrl_c + SIGTERM)
- [ ] Build and commit: `feat(core): add App lifecycle with builder, signal handling, and hooks`

---

## Phase 5: Registry, Config & Metrics

### Task 13: ecat-registry — Registry Trait + Memory Backend

### Task 14: ecat-registry-etcd — etcd Backend

### Task 15: ecat-config — ConfigSource Trait + File/Env Sources

### Task 16: ecat-metrics — Prometheus Integration

---

## Phase 5.5: Data Access Layer

### Task 17: ecat-data — 6 Trait Definitions (RdbmsClient, Cache, OlapClient, SearchClient, GraphClient, TsdbClient)

### Tasks 18–29: 12 Data Contrib Crates

| Task | Crate | Trait | Driver |
|------|-------|-------|--------|
| 18 | `ecat-data-sqlx` | RdbmsClient | sqlx |
| 19 | `ecat-data-redis` | Cache | redis-rs |
| 20 | `ecat-data-memcached` | Cache | memcache-rs |
| 21 | `ecat-data-clickhouse` | OlapClient | clickhouse-rs |
| 22 | `ecat-data-opensearch` | SearchClient | opensearch-rs |
| 23 | `ecat-data-elasticsearch` | SearchClient | elasticsearch-rs |
| 24 | `ecat-data-neo4j` | GraphClient | neo4rs |
| 25 | `ecat-data-nebulagraph` | GraphClient | nebula-client |
| 26 | `ecat-data-arangodb` | GraphClient | arangors |
| 27 | `ecat-data-influxdb` | TsdbClient | influxdb2 |
| 28 | `ecat-data-iotdb` | TsdbClient | iotdb-client-rs |
| 29 | `ecat-data-questdb` | TsdbClient | questdb-rs (ILP) |

---

## Phase 6: CLI Tool

### Task 30: ecat-cli — Command Line Interface

- [ ] `ecat new <name>` — scaffold project from embedded template
- [ ] `ecat proto add|client|server <file>` — proto code generation
- [ ] `ecat run` — cargo run wrapper
- [ ] `ecat build` — cargo build --release wrapper

---

## Phase 7: Ecosystem & Documentation

### Task 31: examples/helloworld — Full Working Example

### Task 32: Workspace Finalization — add all crates to members, verify cargo build --workspace
