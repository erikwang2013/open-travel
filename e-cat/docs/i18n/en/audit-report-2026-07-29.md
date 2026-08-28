<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat Code Review and TDD Test Report

**Date**: 2026-07-29  
**Branch**: main  
**Project**: e-cat (Rust workspace, 17 crates)

---

## 1. Review Scope

Reviewed all Rust source code in the workspace's 17 crates (38 `.rs` files).

| Crate | Description | File count |
|-------|------|--------|
| `ecat-protos` | Protobuf definitions and code generation | 2 |
| `ecat-errors` | Unified error types | 2 |
| `ecat-metadata` | Request metadata abstraction | 1 |
| `ecat-encoding` | JSON/Protobuf encoding/decoding | 3 |
| `ecat-logging` | Logging/Tracing initialization | 1 |
| `ecat-config` | Config loading (file/environment variables) | 3 |
| `ecat-data` | Data layer trait abstractions | 5 |
| `ecat-data-sqlx` | SQLx RDBMS implementation | 1 |
| `ecat-registry` | Service registration and discovery | 2 |
| `ecat-metrics` | Prometheus metrics | 1 |
| `ecat-middleware` | Tower middleware layers | 4 |
| `ecat-transport` | Transport layer abstraction | 4 |
| `ecat-transport-http` | HTTP/Axum transport implementation | 1 |
| `ecat-transport-grpc` | gRPC/Tonic transport implementation | 1 |
| `ecat` | Application framework core | 3 |
| `ecat-cli` | CLI tool | 1 |
| `examples/helloworld` | Example project | 1 |

---

## 2. Findings and Fixes

### Issue 1: [Clippy] `map_identity` — meaningless identity map

- **File**: `ecat-config/src/file.rs:30`
- **Severity**: Low
- **Problem**: `map(|(k, v)| (k, v))` performs no transformation and is dead code
- **Fix**: removed the redundant `.map()` call

### Issue 2: [Clippy] `new_without_default` — Config lacks a Default implementation

- **File**: `ecat-config/src/lib.rs:27`
- **Severity**: Low
- **Problem**: `Config` has a `new()` method but does not implement the `Default` trait
- **Fix**: replaced the manual implementation with `#[derive(Default)]`

### Issue 3: [Clippy] `io_other_error` — old-style Error construction

- **File**: `ecat-middleware/src/recovery.rs:42`
- **Severity**: Low
- **Problem**: `std::io::Error::new(std::io::ErrorKind::Other, ...)` has a more concise alternative
- **Fix**: switched to `std::io::Error::other("task panicked")`

### Issue 4: [Clippy] `redundant_async_block` — redundant async block

- **File**: `ecat-middleware/src/tracing.rs:38`
- **Severity**: Low
- **Problem**: the async block in `Box::pin(async move { fut.await })` is redundant
- **Fix**: simplified to `Box::pin(fut)`

### Issue 5: [Clippy] `redundant_closure` — redundant closure

- **File**: `ecat-data-sqlx/src/lib.rs:63`
- **Severity**: Low
- **Problem**: the closure in `.and_then(|f| serde_json::Number::from_f64(f))` can be omitted
- **Fix**: use `.and_then(serde_json::Number::from_f64)` directly

### Issue 6: [Clippy] `unwrap_or_default` — can be simplified

- **File**: `ecat-transport-http/src/lib.rs:27`
- **Severity**: Low
- **Problem**: `unwrap_or_else(Router::new)` is equivalent to `unwrap_or_default()`
- **Fix**: switched to `unwrap_or_default()`

---

## 3. Test Coverage

### Before Fixes

| Crate | Test count |
|-------|--------|
| `ecat-errors` | 4 |
| `ecat-transport` | 11 |
| Other 15 crates | **0** |
| **Total** | **15** |

### After Fixes

| Crate | Test count | Added | Test content |
|-------|--------|------|----------|
| `ecat-encoding` | 15 | +15 | JsonCodec encode/decode round trips, invalid decode, content_type; CodecBox dispatch; codec_from_content_type happy/error paths; Encoding variants |
| `ecat-errors` | 4 | — | HTTP status mapping, gRPC status conversion, metadata accumulation, Display format |
| `ecat-metadata` | 9 | +9 | key-value access, trace_id, From\<HeaderMap\> (non-UTF8 values skipped), From\<MetadataMap\> (ASCII and binary skipped), IntoIterator |
| `ecat-logging` | 1 | +1 | init smoke test |
| `ecat-config` | 4 | +4 | new/default values, typed reads, loading from ConfigSource |
| `ecat-registry` | 5 | +5 | register/discover, deregister/remove, error on missing, service list, name filtering |
| `ecat-metrics` | 2 | +2 | singleton registry, metrics_text does not panic |
| `ecat` | 4 | +4 | Builder defaults, custom name/version, server registration, lifecycle hooks |
| `ecat-transport` | 11 | — | Context/Request/Response creation and defaults, Server trait |
| **Total** | **55** | **+40** | |

### Crates Not Requiring Unit Tests

- `ecat-protos` — protobuf code generation only
- `ecat-data` — pure trait definitions, no implementation logic
- `ecat-data-sqlx` — requires a database connection, integration-test territory
- `ecat-middleware` — Tower Service implementations, need integration tests
- `ecat-transport-http` / `ecat-transport-grpc` — require network listening, integration-test territory
- `ecat-cli` — prints output only, no logic

---

## 4. Verification Results

```
cargo test   → 55 passed, 0 failed
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
```

---

## 5. Modified Files

| File | Change |
|------|------|
| `ecat-config/src/file.rs` | removed identity map |
| `ecat-config/src/lib.rs` | `#[derive(Default)]` + 4 tests |
| `ecat-data-sqlx/src/lib.rs` | simplified redundant closure |
| `ecat-middleware/src/recovery.rs` | use `std::io::Error::other()` |
| `ecat-middleware/src/tracing.rs` | removed redundant async block |
| `ecat-transport-http/src/lib.rs` | `unwrap_or_else` → `unwrap_or_default` |
| `ecat-metrics/src/lib.rs` | 2 tests |
| `ecat-registry/src/memory.rs` | 5 tests |
| `ecat/src/lib.rs` | 4 tests |
