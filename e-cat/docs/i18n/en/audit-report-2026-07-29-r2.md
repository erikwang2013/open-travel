<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat Code Review Report (Round 2)

**Date**: 2026-07-29  
**Branch**: main  
**Project**: e-cat (Rust workspace, 17 crates)

---

## 1. Review Summary

Building on the first round's clippy fixes and test additions, this round performed a deep code-logic review, focusing on runtime correctness, concurrency safety, and API semantic consistency. A total of 32 source files were reviewed.

### Verification Baseline

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

---

## 2. Bugs Found and Fixed

### Bug 1: [Critical] TracingLayer span guard lifetime bug

- **File**: `ecat-middleware/src/tracing.rs:37`
- **Severity**: **High**
- **Impact**: no request passing through TracingLayer is covered by a tracing span

**Root cause analysis**:

```rust
// 修复前
fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let _guard = span.enter();  // guard 在 call() 返回时 drop
    let fut = self.inner.call(req);
    Box::pin(fut)               // future 在后续 poll 时才执行
}
```

The guard returned by `span.enter()` keeps the span active only within the current synchronous context. `call()` returns a future that has not yet been polled; the actual async execution happens in the later poll phase — by then the guard has long been dropped and the span has no effect. No request passing through TracingLayer appears in the tracing output.

**Fix**:

```rust
// 修复后
use tracing::Instrument;

fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let fut = self.inner.call(req);
    Box::pin(fut.instrument(span))  // span 附着在 future 生命周期上
}
```

Using `tracing::Instrument::instrument()` attaches the span to the future, ensuring the span stays active throughout the future's entire poll lifecycle.

---

### Bug 2: [Critical] LifecycleHook closure implementation defect — on_stop never runs

- **File**: `ecat/src/hook.rs:14-23`, `ecat/src/lib.rs:11-16`
- **Severity**: **High**
- **Impact**: closure hooks registered via `.on_stop()` do nothing on shutdown

**Root cause analysis**:

In the original design, both `on_start()` and `on_stop()` pushed hooks into the same `lifecycle_hooks` Vec. During `run()`, all hooks had `on_start()` called in sequence, and on shutdown all hooks had `on_stop()` called in sequence.

The problem lies in the blanket impl of the `LifecycleHook` trait for closures `Fn() -> Fut`: **it only covers `on_start()`; `on_stop()` uses the trait's default implementation (no-op)**.

This means that when a user uses the closure syntax `.on_stop(|| async { ... })`, the closure is added to the hooks list, but on shutdown only the default empty `on_stop()` runs — the user's logic never executes.

**Fix (two parts)**:

1. **Separate start_hooks and stop_hooks** (`ecat/src/lib.rs`):

```rust
// App 结构体 — 两个独立的 Vec
pub struct App {
    start_hooks: Vec<Box<dyn LifecycleHook>>,
    stop_hooks: Vec<Box<dyn LifecycleHook>>,
    // ...
}

// on_start() → start_hooks, on_stop() → stop_hooks
pub fn on_start(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.start_hooks.push(Box::new(hook));
    self
}
pub fn on_stop(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.stop_hooks.push(Box::new(hook));
    self
}
```

2. **Complete the closure blanket impl** (`ecat/src/hook.rs`):

```rust
impl<F, Fut> LifecycleHook for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Result<...>> + Send,
{
    async fn on_start(&self) -> ... { (self)().await }
    async fn on_stop(&self) -> ...  { (self)().await }  // 新增
}
```

Now the closure implements both `on_start` and `on_stop`; combined with the separated Vecs, each hook is only called at the correct lifecycle phase.

---

### Bug 3: [Medium] SqlxClient Row value type extraction priority bug

- **File**: `ecat-data-sqlx/src/lib.rs:53-68`
- **Severity**: Medium
- **Impact**: integer and float values in the database are extracted as JSON strings instead of numbers

**Root cause analysis**:

`try_get::<String>()` was attempted first. Most database drivers can successfully execute `try_get::<String>()` on numeric columns (implicit conversion), causing the integer value `42` to be extracted as `"42"` instead of `42`.

**Fix**: reordered the `try_get` attempts to `i64 → f64 → String → Null`, preserving numeric types first.

---

## 3. Other Review Findings (Unchanged / Known Limitations)

| Category | File | Notes | Suggestion |
|------|------|------|------|
| Incomplete feature | `ecat-transport-http/src/lib.rs:30` | `axum::serve().await` blocks forever, `stop()` is a no-op | implement graceful shutdown |
| Incomplete feature | `ecat-transport-grpc/src/lib.rs:29` | same as above | implement graceful shutdown |
| Incomplete feature | `ecat-data-sqlx/src/lib.rs:79` | `transaction()` returns an unimplemented error | implement transaction support |
| Code style | `ecat-middleware/src/logging.rs:42` | `elapsed.as_millis() as u64` u128→u64 theoretical truncation | no practical impact |
| Missing tests | `ecat-middleware/` | 4 Tower Services have no unit tests | need integration tests |
| Missing tests | `ecat-data/` | pure trait definitions | acceptable for now |
| RwLock blocking | `ecat-registry/src/memory.rs` | synchronous RwLock may block in async contexts | consider `tokio::sync::RwLock` |

---

## 4. Test Results

```
cargo test → 60 passed, 0 failed

按 crate 分布:
  ecat                  4   (Builder/默认值/生命周期 hook)
  ecat-config           9   (env parse ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (HTTP映射/gRPC转换/metadata/Display)
  ecat-logging          1   (init冒烟)
  ecat-metadata         9   (存取/From HeaderMap/From MetadataMap/迭代器)
  ecat-metrics          2   (单例/text不panic)
  ecat-registry         5   (注册/发现/注销/列表/过滤)
  ecat-transport       11   (Context/Request/Response/Server trait)
  其他 8 crate          0   (纯trait/代码生成/需集成测试/纯打印)
```

---

## 5. Modified Files

| File | Change type | Change description |
|------|----------|----------|
| `ecat/src/lib.rs` | Bug fix | App separated into start_hooks/stop_hooks; AppBuilder updated accordingly; tests adapted |
| `ecat/src/hook.rs` | Bug fix | closure blanket impl completed with on_stop() implementation |
| `ecat-middleware/src/tracing.rs` | Bug fix | span guard → `fut.instrument(span)` |
| `ecat-data-sqlx/src/lib.rs` | Bug fix | Row value extraction order i64→f64→String→Null |

---

## 6. Summary

This round found 2 high-severity runtime bugs and 1 medium-severity data correctness issue:

1. **TracingLayer span ineffective** — affects observability of all requests
2. **LifecycleHook on_stop not executing** — affects correctness of all shutdown logic
3. **Row numeric type loss** — affects the type correctness of database query results

All three issues are fixed; after the fixes all 60 tests pass with zero compile errors and warnings.

### Follow-up Suggestions

- Implement graceful shutdown for HTTP/gRPC servers
- Add integration tests for `ecat-middleware` (mock Service + verify span/timeout/recovery behavior)
- Add integration tests for `ecat-data-sqlx` (using an in-memory SQLite database)
- Replace the synchronous RwLock in `ecat-registry/memory.rs` with `tokio::sync::RwLock`
