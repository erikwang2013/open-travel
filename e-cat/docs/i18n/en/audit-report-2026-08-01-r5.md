# E-CAT Audit Report — r5

**Date**: 2026-08-01  
**Branch**: main  
**Version**: 2.1.7  
**Crate count**: 47 (workspace members)
**Status**: ✅ all fixable issues resolved + data backends fully support config files

---

## 0. Fix Record (2026-08-01)

| # | Issue | File | Fix |
|---|------|------|------|
| 1 | unused import `axum::routing::get` | `ecat-versioning/src/lib.rs:3` | removed the top-level import, moved it inside `#[cfg(test)]` |
| 2 | unused variable `version` | `ecat-versioning/src/lib.rs:61` | renamed to `_version` |
| 3 | dead code `extract_version` | `ecat-versioning/src/lib.rs:68` | changed to `pub fn` |
| 4 | `useless_format!` | `ecat-versioning/src/lib.rs:62` | changed to a direct `"/api"` |
| 5 | `unnecessary_to_owned` | `ecat-data-questdb/src/lib.rs:39` | `"true".to_string()` → `"true"` |
| 6 | error message swallowed | `ecat-data-questdb/src/lib.rs:30` | `unwrap_or_default()` → `unwrap_or_else(...)` |
| 7 | `derivable_impls` | `ecat-client/src/lib.rs:249` | `GrpcClientBuilder` switched to `#[derive(Default)]` |
| 8 | `manual_is_multiple_of` | `ecat-config/src/encrypted.rs:60` | `s.len() % 2 != 0` → `!s.len().is_multiple_of(2)` |
| 9 | `collapsible_if` | `ecat-registry-etcd/src/lib.rs:92` | merged nested `if let` |
| 10 | `collapsible_if` | `ecat-data-clickhouse/src/lib.rs:56` | merged nested `if let` |
| 11 | `type_complexity` | `ecat-data-memcached/src/lib.rs:9` | added a `type CacheEntry` alias |

**Final result**: `cargo build` zero warnings, `cargo clippy --all-targets` zero warnings, `cargo test` all passed (0 failures).

### 12 ─ Data backends fully support config files (Cargo + lib.rs)

Added `Config` structs (`#[derive(Deserialize)]`) and `from_config()` constructors to 12 data backend crates, supporting loading connection information from JSON/YAML config files without hardcoding.

| Crate | Config struct | Fields |
|-------|--------------|------|
| `ecat-data-redis` | `RedisConfig` | `url` |
| `ecat-data-sqlx` | `SqlxConfig` | `url` |
| `ecat-data-clickhouse` | `ClickhouseConfig` | `base_url`, `database` (default "default") |
| `ecat-data-questdb` | `QuestdbConfig` | `base_url` |
| `ecat-data-elasticsearch` | `ElasticsearchConfig` | `base_url` |
| `ecat-data-opensearch` | `OpenSearchConfig` | `base_url` |
| `ecat-data-influxdb` | `InfluxConfig` | `base_url`, `org`, `bucket`, `token` |
| `ecat-data-memcached` | `MemcachedConfig` | (empty — in-memory implementation) |
| `ecat-data-neo4j` | `Neo4jConfig` | `base_url`, `username`, `password` |
| `ecat-data-nebulagraph` | `NebulaGraphConfig` | `base_url`, `space` |
| `ecat-data-arangodb` | `ArangoConfig` | `base_url`, `db`, `username`, `password` |
| `ecat-data-iotdb` | `IotdbConfig` | `base_url`, `username`, `password` |

**Usage example**:
```rust
// 从 YAML 配置文件加载
let cfg: ClickhouseConfig = serde_json::from_str(r#"{"base_url":"http://localhost:8123"}"#)?;
let client = ClickhouseClient::from_config(cfg);
```

### 13 ─ Optional authentication support for HTTP backends (5 crates)

Added optional `username` / `password` fields and `with_auth()` constructors to 5 pure-HTTP backends. All are `Option<String>` (`#[serde(default)]`); no authentication when not configured.

| Crate | New Config fields | New constructor |
|-------|-----------------|-------------|
| `ecat-data-elasticsearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-opensearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-clickhouse` | `username?`, `password?` | `with_auth()` |
| `ecat-data-questdb` | `username?`, `password?` | `with_auth()` |
| `ecat-data-nebulagraph` | `username?`, `password?` | `with_auth()` |

All HTTP requests automatically attach Basic Auth via the `apply_auth()` helper (only when both are non-None).

### 14 ─ Optional authentication fields for Redis / RDBMS / Memcached (3 crates)

| Crate | New Config fields | New constructor | Auth method |
|-------|-----------------|-------------|----------|
| `ecat-data-redis` | `password?` | `connect_with_password()` | URL-embedded password |
| `ecat-data-sqlx` | `username?`, `password?` | `connect_with_auth()` | URL-embedded auth |
| `ecat-data-memcached` | `username?`, `password?` | `with_auth()` | reserved fields (in-memory implementation) |

Sqlx covers the four RDBMS types SQLite / PostgreSQL / MySQL / TiDB. Auth fields are embedded into the connection URL via `replacen("://", "://user:pass@")`, taking effect only when the URL contains no `@`.

### 15 ─ TLS certificate authentication support + ecat-tls crate (all 12 backends)

Added the `ecat-tls` crate providing:
- `TlsClientConfig` — optional TLS config (ca_cert, client_cert, client_key, skip_verify)
- `generate_ca()` — self-signed CA certificate generation
- `generate_server_cert()` — server certificate generation
- `generate_client_cert()` — client certificate generation (mTLS)

All 12 data backend Configs gained a `#[serde(default)] tls: Option<TlsClientConfig>` field.

| Backend type | TLS method |
|----------|----------|
| 9 HTTP backends | `tls.build_reqwest_client()` builds a TLS reqwest Client |
| Redis | URL scheme switch `redis://` → `rediss://` |
| Sqlx | reserved field (TLS via URL parameter `?sslmode=require`) |
| Memcached | reserved field (reserved for a network implementation) |

---

## 1. Overview

| Item | Status | Details |
|------|------|------|
| `cargo build` | ✅ Passed | 3 compiler warnings, 19.85s |
| `cargo test` | ✅ Passed | ~137 unit tests all passed, 0 failures, 1 ignored |
| `cargo clippy` | ⚠️ Has warnings | 5 lint warnings across 3 crates |
| `cargo fmt` | ✅ Passed | no formatting issues |
| `cargo audit` | ❌ Not installed | cannot scan for known CVEs |

---

## 2. Compiler Warnings (Need Fixing)

### 2.1 ecat-versioning (3 warnings)

**File**: `ecat-versioning/src/lib.rs`

| # | Warning | Line | Severity |
|---|---------|------|----------|
| 1 | `unused import: axum::routing::get` | 3 | Low |
| 2 | `unused variable: version` | 61 | Low |
| 3 | `function extract_version is never used` | 68 | Low |

**Suggestion**: remove the unused import, rename `version` to `_version`, and make `extract_version` `pub` or mark it `#[allow(dead_code)]`.

### 2.2 ecat-data-questdb (1 clippy warning)

**File**: `ecat-data-questdb/src/lib.rs:39`

```rust
// 当前:
.query(&[("query", sql), ("count", &"true".to_string())])

// 应改为:
.query(&[("query", sql), ("count", &"true")])
```

### 2.3 ecat-client (1 clippy warning)

**File**: `ecat-client/src/lib.rs:249`

`GrpcClientBuilder` manually implements `Default`; it can be replaced with `#[derive(Default)]` directly.

---

## 3. Clippy Lint Warning Summary

| Crate | Warning | Type |
|-------|---------|------|
| ecat-versioning | `useless_format!` — use `"/api".to_string()` | Performance |
| ecat-versioning | unused import / dead code | Cleanup |
| ecat-data-questdb | `unnecessary_to_owned` | Performance |
| ecat-client | `derivable_impls` — use derive Default | Simplification |

---

## 4. Test Coverage Analysis

### 4.1 Statistics

| Metric | Value |
|------|------|
| Total unit tests | ~137 |
| Failures | 0 |
| Ignored | 1 |
| Crates with tests | ~24 / 48 |
| **Crates with 0 tests** | **~24 / 48 (50%)** |

### 4.2 Crates Lacking Tests (0 or constructor-only)

The following crates have weak test coverage:

- ecat-data-arangodb, ecat-data-clickhouse, ecat-data-elasticsearch
- ecat-data-influxdb, ecat-data-iotdb, ecat-data-nebulagraph
- ecat-data-neo4j, ecat-data-opensearch, ecat-data-questdb
- ecat-data-redis, ecat-data-sqlx, ecat-data-memcached
- ecat-mq, ecat-mq-kafka, ecat-graphql, ecat-openapi
- ecat-transport, ecat-transport-grpc, ecat-transport-http
- ecat-transport-ws, ecat-tracing, ecat-logging
- ecat-middleware, ecat-registry-consul, ecat-registry-etcd

### 4.3 Doc-tests

All **48 crates have 0 doc-tests**. There are no `/// ````rust` doc examples in the code.

---

## 5. Dependency Issues

### 5.1 ⚠️ yaml_serde vs serde_yaml (medium risk)

**File**: `ecat-config/Cargo.toml:9`

```toml
yaml_serde = "0.10"
```

The standard YAML library in the Rust ecosystem is `serde_yaml` (latest `0.9.34+`), while `yaml_serde` is a **different and less-maintained crate**.

**Suggestion**: confirm whether `yaml_serde` is the intended dependency. If `serde_yaml` was intended, replace it.

### 5.2 cargo-audit missing

`cargo audit` is not installed. It is recommended to `cargo install cargo-audit` and add it to CI.

### 5.3 Missing description field

There is no `description` in `[workspace.package]`, and none of the sub-crates define a description either.

---

## 6. Code Quality Issues

### 6.1 unwrap/expect in production code

| File | Line | Call | Risk |
|------|------|------|------|
| `ecat-client/src/lib.rs` | 28 | `.expect("StaticResolver poisoned")` | Low — reasonable |
| `ecat/src/signal.rs` | 8 | `.expect("failed to install SIGTERM handler")` | Medium — panics at startup |
| `ecat-protos/build.rs` | 5 | `.unwrap()` | Low — build script |

### 6.2 ecat-versioning's extract_version

The `extract_version` function (line 68) implements version extraction from the Accept header, but it is never called by `build_header_router()`.

### 6.3 ecat-data-questdb error handling

```rust
// 第 30 行: 网络响应体读取使用 unwrap_or_default
Err(RdbmsError::Database(resp.text().await.unwrap_or_default()))
```

When `resp.text()` fails, the error message is silently swallowed. It is suggested to change to `unwrap_or_else(|e| format!("questdb parse: {e}"))`.

---

## 7. Architecture Assessment

### Strengths

- 48 crates with clear separation of responsibilities
- workspace-wide unified version via `version.workspace = true`
- lean dependencies, no heavy frameworks
- no TODO/FIXME/HACK

### Needs Improvement

| Issue | Priority |
|------|--------|
| 50% of crates have no tests | High |
| yaml_serde vs serde_yaml confusion | Medium |
| cargo-audit missing | Medium |
| ecat-versioning dead code | Low |
| No doc-tests | Low |

---

## 8. Security Overview

| Check | Result |
|--------|------|
| Hardcoded secrets | None found |
| .env file leakage | None found |
| Dangerous unwrap (production code) | 2 (signal.rs, client.rs) |
| CVE scanning | Not performed (cargo-audit needs to be installed) |

---

## 9. Action Plan

### P0 — Fix immediately
1. Clean up the 3 compiler warnings in ecat-versioning
2. Fix the ecat-data-questdb clippy issue
3. Fix ecat-client derivable_impls

### P1 — Short term
4. Install `cargo-audit` to scan dependency vulnerabilities
5. Confirm the `yaml_serde` vs `serde_yaml` choice
6. Add doc-tests to core crates

### P2 — Medium term
7. Add tests to transport/data/security crates
8. Add a `description` field to all crates
9. Integrate or remove `extract_version`

### P3 — Long term
10. Establish CI: build → test → clippy → audit → coverage

---

*Report generated 2026-08-01. Toolchain: cargo 1.92.0, rustc 1.92.0, clippy 1.92.0*
