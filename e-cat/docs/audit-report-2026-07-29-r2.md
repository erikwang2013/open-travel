<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat 代码审查报告（第二轮）

**日期**: 2026-07-29  
**分支**: main  
**项目**: e-cat (Rust workspace, 17 个 crate)

---

## 一、审查概要

在第一轮 clippy 修复和测试补充的基础上，本轮进行了深度代码逻辑审查，重点关注运行时正确性、并发安全、API 语义一致性。共审查 32 个源文件。

### 验证基线

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

---

## 二、发现的 Bug 及修复

### Bug 1：[关键] TracingLayer span 守卫生命周期错误

- **文件**: `ecat-middleware/src/tracing.rs:37`
- **严重程度**: **高**
- **影响**: 所有经过 TracingLayer 的请求都不会被 tracing span 覆盖

**根因分析**:

```rust
// 修复前
fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let _guard = span.enter();  // guard 在 call() 返回时 drop
    let fut = self.inner.call(req);
    Box::pin(fut)               // future 在后续 poll 时才执行
}
```

`span.enter()` 返回的 guard 只在当前同步上下文中保持 span 活跃。`call()` 返回的是尚未 poll 的 future，实际异步执行发生在后续的 poll 阶段 — 此时 guard 早已被 drop，span 不会生效。所有经过 TracingLayer 的请求都不会出现在 tracing 输出中。

**修复**:

```rust
// 修复后
use tracing::Instrument;

fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let fut = self.inner.call(req);
    Box::pin(fut.instrument(span))  // span 附着在 future 生命周期上
}
```

使用 `tracing::Instrument::instrument()` 将 span 附着在 future 上，确保 span 在 future 的整个 poll 生命周期内保持活跃。

---

### Bug 2：[关键] LifecycleHook 闭包实现缺陷 — on_stop 永不执行

- **文件**: `ecat/src/hook.rs:14-23`、`ecat/src/lib.rs:11-16`
- **严重程度**: **高**
- **影响**: 通过 `.on_stop()` 注册的闭包 hook 在 shutdown 时什么也不做

**根因分析**:

原有设计中，`on_start()` 和 `on_stop()` 方法都将 hook 推入同一个 `lifecycle_hooks` Vec。在 `run()` 时，所有 hook 依次调用 `on_start()`，shutdown 时所有 hook 依次调用 `on_stop()`。

问题出在 `LifecycleHook` trait 对闭包 `Fn() -> Fut` 的 blanket impl：**只覆盖了 `on_start()`，`on_stop()` 使用 trait 默认实现（no-op）**。

这意味着用户使用闭包语法 `.on_stop(|| async { ... })` 时，闭包虽然被加入 hooks 列表，但 shutdown 时只会执行默认的空 `on_stop()`，用户的逻辑永远不会运行。

**修复（两部分）**:

1. **分离 start_hooks 和 stop_hooks**（`ecat/src/lib.rs`）：

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

2. **补全闭包 blanket impl**（`ecat/src/hook.rs`）：

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

现在闭包同时实现 `on_start` 和 `on_stop`，配合分离的 Vec，每个 hook 只在正确的生命周期阶段被调用。

---

### Bug 3：[中等] SqlxClient Row 值类型提取优先级错误

- **文件**: `ecat-data-sqlx/src/lib.rs:53-68`
- **严重程度**: 中
- **影响**: 数据库中的整型和浮点数值会被提取为 JSON 字符串而非数字

**根因分析**:

`try_get::<String>()` 被放在第一位尝试。大多数数据库驱动对数值列可以成功执行 `try_get::<String>()`（隐式转换），导致整数值 `42` 被提取为 `"42"` 而非 `42`。

**修复**: 调整 `try_get` 尝试顺序为 `i64 → f64 → String → Null`，优先保留数值类型。

---

## 三、其他审查发现（未修改 / 已知限制）

| 类别 | 文件 | 说明 | 建议 |
|------|------|------|------|
| 功能未完成 | `ecat-transport-http/src/lib.rs:30` | `axum::serve().await` 阻塞永不返回，`stop()` 为空操作 | 实现 graceful shutdown |
| 功能未完成 | `ecat-transport-grpc/src/lib.rs:29` | 同上 | 实现 graceful shutdown |
| 功能未完成 | `ecat-data-sqlx/src/lib.rs:79` | `transaction()` 返回未实现错误 | 实现事务支持 |
| 代码风格 | `ecat-middleware/src/logging.rs:42` | `elapsed.as_millis() as u64` u128→u64 理论截断 | 实际无影响 |
| 测试缺失 | `ecat-middleware/` | 4 个 Tower Service 无单元测试 | 需集成测试 |
| 测试缺失 | `ecat-data/` | 纯 trait 定义 | 当前可接受 |
| RwLock 阻塞 | `ecat-registry/src/memory.rs` | 同步 RwLock 在异步上下文中可能阻塞 | 考虑换 tokio::sync::RwLock |

---

## 四、测试结果

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

## 五、修改文件清单

| 文件 | 变更类型 | 变更说明 |
|------|----------|----------|
| `ecat/src/lib.rs` | Bug 修复 | App 分离 start_hooks/stop_hooks；AppBuilder 对应更新；测试适配 |
| `ecat/src/hook.rs` | Bug 修复 | 闭包 blanket impl 补全 on_stop() 实现 |
| `ecat-middleware/src/tracing.rs` | Bug 修复 | span 守卫 → `fut.instrument(span)` |
| `ecat-data-sqlx/src/lib.rs` | Bug 修复 | Row 值提取顺序 i64→f64→String→Null |

---

## 六、总结

本轮审查发现了 2 个高严重度运行时 Bug 和 1 个中等严重度数据正确性问题：

1. **TracingLayer span 失效** — 影响所有请求的可观测性
2. **LifecycleHook on_stop 不执行** — 影响所有 shutdown 逻辑的正确性
3. **Row 数值类型丢失** — 影响数据库查询结果的类型正确性

三个问题均已修复，修复后全部 60 个测试通过，编译零错误零警告。

### 后续建议

- 为 HTTP/gRPC server 实现 graceful shutdown
- 为 `ecat-middleware` 添加集成测试（mock Service + 验证 span/超时/恢复行为）
- 为 `ecat-data-sqlx` 添加集成测试（使用 SQLite 内存数据库）
- 将 `ecat-registry/memory.rs` 的同步 RwLock 替换为 `tokio::sync::RwLock`
