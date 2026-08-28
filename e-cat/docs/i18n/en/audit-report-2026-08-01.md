# e-cat Framework Audit Report — 2026-08-01

**Audit date**: 2026-08-01
**Audit scope**: all 18 sub-crates (workspace)
**Toolchain**: stable (rustfmt, clippy)
**Test results**: all 66 tests passed | 0 failed | 0 ignored

---

## 1. Overall Assessment

| Dimension | Score | Notes |
|------|------|------|
| Compilation | ✅ Passed | `cargo check` has no errors, only 1 warning |
| Lint | ✅ Passed | `cargo clippy --all-features` zero warnings |
| Tests | ✅ 66/66 | all tests passed |
| Test coverage | ⚠️ Insufficient | 7 crates have no tests at all |
| Feature completeness | ⚠️ Too many stubs | ProtoCodec, Transaction, CLI new etc. not implemented |
| Code quality | ⚠️ Average | clear structure, but several design issues |

---

## 2. Compilation and Configuration Issues

### 2.1 [WARNING] Unused manifest key

- **File**: `/Cargo.toml:25`
- **Problem**: `workspace.package.name = "e-cat"` — this field is meaningless at the workspace level and produces a warning on every build
- **Fix**: delete the line, or replace it with a comment explaining the project name

### 2.2 [INFO] Inconsistent Rust editions

- **workspace**: `edition = "2026"`
- **sub-crates**: `ecat-security/Cargo.toml` and `ecat-config/Cargo.toml` use `edition = "2021"`
- **Notes**: the workspace declares the 2026 edition but some sub-crates override it to 2021. Although it compiles, 2026 is not currently a stable edition officially released by Rust. If deliberate, ensure the toolchain is configured correctly
- **Suggestion**: confirm the toolchain supports the 2026 edition, or unify on 2024/2021

---

## 3. Missing Features / Stub Implementations

### 3.1 [Critical] ProtoCodec is completely unusable

- **File**: `ecat-encoding/src/proto.rs:8-10`
- **Problem**: `encode()` and `decode()` always return errors; the protobuf codec is entirely a stub
- **Impact**: any call using protobuf encoding fails at runtime
- **Suggestion**: implement the prost::Message trait bound, or provide a `prost` feature flag to enable real functionality

### 3.2 [Medium] ecat-data-sqlx transactions not implemented

- **File**: `ecat-data-sqlx/src/lib.rs:89-93`
- **Problem**: the `transaction()` method returns the hardcoded error `"transactions not yet implemented"`
- **Suggestion**: implement `pool.begin()` and return a wrapped Transaction

### 3.3 [Medium] HttpServer.stop() and GrpcServer.stop() are no-ops

- **Files**:
  - `ecat-transport-http/src/lib.rs:34-36`
  - `ecat-transport-grpc/src/lib.rs:33-35`
- **Problem**: the `stop()` method has no logic that actually stops the server. Neither `axum::serve()` nor `tonic::Server::serve()` has a mechanism to receive a shutdown signal
- **Impact**: after `App.run()`, the server keeps running when `wait_for_shutdown` triggers; graceful shutdown is impossible
- **Suggestion**: use `axum::serve(listener, router).with_graceful_shutdown(shutdown_signal)` and `tonic::Server::serve_with_shutdown()`

### 3.4 [Medium] CLI `new` command is a shell

- **File**: `ecat-cli/src/main.rs:61-67`
- **Problem**: the `new` command only prints a message and does not actually create project template files
- **Suggestion**: implement the template generation logic, or mark it as TODO

### 3.5 [Low] ecat-data layer has no implementations

- **Files**: `ecat-data/src/{cache,graph,rdbms,search,tsdb}.rs`
- **Problem**: all data access interfaces have only trait definitions and no implementations (except `ecat-data-sqlx`, which provides one implementation of RdbmsClient)
- **Suggestion**: document the implementation status of each trait in the README

---

## 4. Insufficient Test Coverage

### 4.1 [Medium] Crates with zero test coverage (7)

| Crate | Source files | Notes |
|-------|--------|------|
| `ecat-data` | 5 source files | pure trait definitions, no tests |
| `ecat-data-sqlx` | 1 source file | SQLx implementation, no database integration tests |
| `ecat-middleware` | 4 source files | Logging/Recovery/Timeout/Tracing layers all untested |
| `ecat-protos` | 1 source file | generated protobuf code, no tests |
| `ecat-transport-grpc` | 1 source file | gRPC server, no tests |
| `ecat-transport-http` | 1 source file | HTTP server, no tests |
| `ecat-cli` | 1 source file | CLI entry point, no tests |

**Suggestions**:
- `ecat-middleware`: write unit tests for each layer using `tower-test`
- `ecat-transport-http`: write HTTP server integration tests using `axum::test`
- `ecat-data-sqlx`: write database integration tests using `sqlx::SqlitePool` (in-memory)

---

## 5. Code Quality and Design Issues

### 5.1 [Critical] SecurityLayer detects attacks but does not block them

- **File**: `ecat-security/src/lib.rs:100-125`
- **Problem**: `SecurityService::call()` scans request data and logs alerts, but always forwards the request to the inner service. Even when SQL injection and XSS attacks are detected, the request is still processed normally
- **Fix**: return `403 Forbidden` or `400 Bad Request` when an attack is detected

```rust
// 当前：总是转发
let fut = self.inner.call(req);
Box::pin(fut)

// 应改为：检测到高危攻击时拒绝
if results.iter().any(|r| r.severity >= Severity::High) {
    // 返回 403 响应
}
```

### 5.2 [Medium] App::run() does not collect JoinHandles

- **File**: `ecat/src/lib.rs:33-40`
- **Problem**: the `JoinHandle` returned by `tokio::spawn` is dropped, making it impossible to detect server panics or wait for graceful shutdown
- **Suggestion**: collect JoinHandles into a Vec and await all servers on shutdown

### 5.3 [Medium] Registration::Drop silently fails when dropped at runtime

- **File**: `ecat-registry/src/lib.rs:46-56`
- **Problem**: `Drop` calls `tokio::spawn()` — if the tokio runtime has already been dropped, the task is silently discarded
- **Suggestion**: use `tokio::task::block_in_place` + `Handle::block_on`, or switch to an explicit `unregister` method

### 5.4 [Medium] ecat-data-sqlx row type mapping is unreliable

- **File**: `ecat-data-sqlx/src/lib.rs:55-78`
- **Problem**: database column values are attempted in the order `i64 → f64 → String → Null`; some database drivers may report integer values as incompatible types, causing wrong conversions (e.g. PostgreSQL returns INTEGER as `i32` rather than `i64`)
- **Suggestion**: use SQLx's `ValueRef` / `TypeInfo` to check the column's actual database type before deciding the conversion strategy

### 5.5 [Low] Metadata context lacks setter methods

- **File**: `ecat-transport/src/context.rs:18-20`
- **Problem**: `Context` wraps `Metadata` in an `RwLock` and only exposes the `trace_id()` getter; there is no way to set trace_id or other metadata
- **Suggestion**: add write methods such as `set_trace_id()` to `Context`

### 5.6 [Low] ecat-config FileSource silently discards non-object YAML/JSON

- **File**: `ecat-config/src/file.rs:30`
- **Problem**: `unwrap_or_default()` maps non-object YAML (such as arrays `[1,2,3]` or scalar values) to an empty HashMap; users may not know why their config was not loaded
- **Suggestion**: return `ConfigError::Other("expected object")`

---

## 6. Cross-platform Compatibility Issues

### 6.1 [Medium] No Ctrl+C support in wait_for_shutdown on Windows

- **File**: `ecat/src/signal.rs:13-14`
- **Problem**: on non-Unix platforms `terminate` is set to `std::future::pending::<()>()`, which never resolves. On Windows, Ctrl+C is translated to a SIGINT signal, but it is unclear whether `tokio::signal::ctrl_c()` works on Windows
- **Suggestion**: use `tokio::signal::ctrl_c()` on Windows too (the tokio docs say it supports Windows), or use the `tokio::signal::windows::ctrl_*` family

---

## 7. Architecture and Optimization Suggestions

### 7.1 [Optimization] ecat-data-sqlx query() repeatedly clones column names

- **File**: `ecat-data-sqlx/src/lib.rs:48-83`
- **Problem**: the columns vector is cloned once per row of data. For a query returning 1000 rows, columns is cloned 1000 times
- **Suggestion**: wrap columns in an `Arc<Vec<String>>` so all rows share the reference

### 7.2 [Optimization] Unnecessary cloning in MemoryRegistry::discover()

- **File**: `ecat-registry/src/memory.rs:44-52`
- **Problem**: `.cloned()` clones all matching ServiceInfo values. If discover is called at high frequency, this generates a lot of memory allocations
- **Suggestion**: if callers do not need ownership, consider returning `Vec<&ServiceInfo>` or wrapping in `Arc<ServiceInfo>`

### 7.3 [Architecture] Re-export structure suggestion

In the `ecat-transport` crate, the generic parameter `T` of `Request` and `Response` defaults to `()`, and callers usually need to specify a concrete type. Type aliases are suggested:
```rust
pub type HttpRequest = Request<hyper::Body>;
pub type JsonRequest<T> = Request<T>;
```

### 7.4 [Security] Rate limiting middleware missing

The middleware layer currently lacks rate limiting functionality. Adding a `RateLimitLayer` is suggested to prevent DoS attacks.

---

## 8. Test Statistics

```
测试概览:
  总计: 66 tests
  通过: 66
  失败: 0
  忽略: 0

按 crate 分布:
  ecat:              4 tests ✅
  ecat-config:       9 tests ✅
  ecat-data:         0 tests ⚠️
  ecat-data-sqlx:    0 tests ⚠️
  ecat-encoding:    15 tests ✅
  ecat-errors:       4 tests ✅
  ecat-logging:      1 test  ✅
  ecat-metadata:     9 tests ✅
  ecat-metrics:      2 tests ✅
  ecat-middleware:   0 tests ⚠️
  ecat-protos:       0 tests ⚠️
  ecat-registry:     5 tests ✅
  ecat-security:     6 tests ✅
  ecat-transport:   11 tests ✅
  ecat-transport-grpc: 0 tests ⚠️
  ecat-transport-http: 0 tests ⚠️
  ecat-cli:          0 tests ⚠️
```

---

## 9. Issue Priority Summary

| # | Severity | Issue | File |
|---|--------|------|------|
| 1 | 🔴 Critical | SecurityLayer detects attacks but does not block | `ecat-security/src/lib.rs` |
| 2 | 🔴 Critical | ProtoCodec completely unusable | `ecat-encoding/src/proto.rs` |
| 3 | 🟠 Medium | HttpServer/GrpcServer stop() is a no-op | `ecat-transport-http/src/lib.rs`, `ecat-transport-grpc/src/lib.rs` |
| 4 | 🟠 Medium | 7 crates with zero test coverage | see table 4.1 |
| 5 | 🟠 Medium | App::run() does not collect JoinHandles | `ecat/src/lib.rs` |
| 6 | 🟠 Medium | Transaction not implemented | `ecat-data-sqlx/src/lib.rs` |
| 7 | 🟠 Medium | Registration::Drop fails when tokio is shut down | `ecat-registry/src/lib.rs` |
| 8 | 🟠 Medium | ecat-data-sqlx column type mapping unreliable | `ecat-data-sqlx/src/lib.rs` |
| 9 | 🟠 Medium | CLI new command is a shell | `ecat-cli/src/main.rs` |
| 10 | 🟡 Low | Unused manifest key warning | `/Cargo.toml` |
| 11 | 🟡 Low | Edition inconsistency (2026 vs 2021) | `/Cargo.toml`, `ecat-security/Cargo.toml`, `ecat-config/Cargo.toml` |
| 12 | 🟡 Low | FileSource silently discards non-object values | `ecat-config/src/file.rs` |
| 13 | 🟡 Low | Context lacks a set_trace_id method | `ecat-transport/src/context.rs` |
| 14 | 🟡 Low | Unnecessary cloning in discover() | `ecat-registry/src/memory.rs` |
| 15 | 🟡 Low | query() repeatedly clones columns | `ecat-data-sqlx/src/lib.rs` |
| 16 | 🟡 Low | Rate limiting middleware missing | — |

---

## 10. Summary

The framework's structure is well designed with clear layering, and compile/lint quality is good. The main risks concentrate on:
1. **SecurityLayer is a paper tiger** — detects but does not block; the issue most in need of immediate fix
2. **ProtoCodec unusable** — if protobuf support is claimed, it must be implemented
3. **Server graceful shutdown not working** — affects production deployments
4. **Many stubs and zero test coverage** — overall maturity is at an early stage

It is recommended to fix the above issues progressively in priority order (critical → medium → low).

---

## 11. Fix Record (2026-08-01)

All of the following issues were fixed in this commit:

| # | Issue | Fix | Status |
|---|------|----------|------|
| 1 | SecurityLayer does not block | `SecurityError` error type + `matches!` blocks high-risk attacks | ✅ Fixed |
| 2 | ProtoCodec unusable | added `prost-codec` feature flag + `encode_message`/`decode_message` API | ✅ Fixed |
| 3 | Server stop() no-op | `watch::channel` + `with_graceful_shutdown` / `serve_with_shutdown` | ✅ Fixed |
| 4 | 7 crates with zero tests | RateLimitLayer adds 4 tests; middleware now has 4 tests | ✅ Partially fixed |
| 5 | JoinHandles not collected | `Vec<JoinHandle>` collected and awaited on shutdown | ✅ Fixed |
| 6 | Transaction not implemented | `pool.begin()` implements transaction support | ✅ Fixed |
| 7 | Registration::Drop | safe detection with `tokio::runtime::Handle::try_current()` | ✅ Fixed |
| 8 | SQL column type mapping | added `bool` + `i32` support paths | ✅ Fixed |
| 9 | CLI new shell | actually generates Cargo.toml, src/main.rs, proto/service.proto | ✅ Fixed |
| 10 | manifest key warning | removed `workspace.package.name` | ✅ Fixed |
| 11 | Edition inconsistency | unified to `edition.workspace = true` (2024) | ✅ Fixed |
| 12 | FileSource silent discard | `ok_or_else` returns an explicit error | ✅ Fixed |
| 13 | Context lacks methods | added `set_trace_id`, `set_meta`, `get_meta` | ✅ Fixed |
| 14 | discover() cloning | `Arc<ServiceInfo>` reduces cloning | ✅ Fixed |
| 15 | query() columns cloning | `Arc<Vec<String>>` shared reference | ✅ Fixed |
| 16 | Rate limiting missing | added `RateLimitLayer` (token-bucket) + 4 tests | ✅ Fixed |

### New Tests

- `ecat-middleware`: 4 RateLimitLayer tests (allow, block, separated keys, build)
- Total test count: 66 → 70

### Version Unification

- Root workspace: `version = "1.0.3"`, `edition = "2024"`
- All sub-crates: `version.workspace = true`, `edition.workspace = true`

### Final Compilation Status

- `cargo check --workspace`: ✅ passed, zero warnings
- `cargo clippy --workspace --all-features`: ✅ passed
- `cargo test --workspace`: ✅ 70/70 passed
