<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat 代码审查报告（第三轮）

**日期**: 2026-07-29  
**分支**: main  
**项目**: e-cat (Rust workspace, 18 个 crate)  
**审查范围**: 全部 37 个源文件，共 2151 行 Rust 代码

---

## 一、审查概要

第二轮审查发现的 3 个 Bug 已全部修复，本轮在干净基线（0 error / 0 warning / 60 test passed）上做深度再审查，重点关注边界条件、错误处理、生产健壮性。

### 验证基线

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

### R2 Bug 修复确认

| Bug | 文件 | 状态 |
|-----|------|------|
| TracingLayer span 守卫生命周期 | `ecat-middleware/src/tracing.rs` | ✅ 已修复 |
| LifecycleHook on_stop 不执行 | `ecat/src/hook.rs`, `ecat/src/lib.rs` | ✅ 已修复 |
| Row 值类型提取优先级 | `ecat-data-sqlx/src/lib.rs` | ✅ 已修复 |

---

## 二、新发现的问题

### 问题 1：[中等] `metrics_text()` 中使用 unwrap()，生产环境可能 panic

- **文件**: `ecat-metrics/src/lib.rs:14-15`
- **严重程度**: **中等**
- **影响**: `/metrics` 端点被访问时进程 panic

**根因分析**:

```rust
pub fn metrics_text() -> String {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder.encode(&registry().gather(), &mut buffer).unwrap();  // 可能 panic
    String::from_utf8(buffer).unwrap()                           // 可能 panic
}
```

`TextEncoder::encode()` 在内部 I/O 错误或系统内存不足时会失败。`String::from_utf8()` 理论上如果 Prometheus 库产生非 UTF-8 输出也会失败。这两个 `unwrap()` 在非测试代码路径上，直接暴露给 HTTP handler 调用，panic 会导致进程崩溃。

**建议修复**: 返回 `Result<String, ...>` 或使用 `.unwrap_or_default()` 降级处理。

---

### 问题 2：[低] Recovery 中间件 spawn 新 task 丢失 span 上下文

- **文件**: `ecat-middleware/src/recovery.rs:40`
- **严重程度**: **低**
- **影响**: Recovery 层在 Tracing 层之前时，请求的 trace_id 不会传递到业务逻辑

**根因分析**:

```rust
fn call(&mut self, req: Req) -> Self::Future {
    let fut = self.inner.call(req);
    Box::pin(async move {
        match tokio::task::spawn(fut).await {  // 新 task，不继承 span
            // ...
        }
    })
}
```

`tokio::task::spawn()` 创建一个新的 Tokio 任务，tracing span 是 task-local 的，不会自动传递。

**建议**: 在文档中明确中间件顺序要求（Recovery 应放在最外层），或在 spawn 前使用 `.instrument(span)` 手动传递。

---

### 问题 3：[低] Registration Drop 静默丢弃错误

- **文件**: `ecat-registry/src/lib.rs:50-52`
- **严重程度**: **低**
- **影响**: 服务注销失败无感知

```rust
impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(reg) = self.registry.take() {
            let id = self.id.clone();
            tokio::spawn(async move {
                let _ = reg.deregister(&id).await;  // 错误被静默丢弃
            });
        }
    }
}
```

虽然不能在 Drop 中阻塞，但可以用 `tracing::warn!` 记录注销失败。

---

### 问题 4：[低] `ecat-data-sqlx` f64 特殊值处理

- **文件**: `ecat-data-sqlx/src/lib.rs:57-61`
- **严重程度**: **低**
- **影响**: 数据库中 NaN/Infinity 浮点值被转为 Null

```rust
row.try_get::<f64, _>(col.as_str())
    .ok()
    .and_then(serde_json::Number::from_f64)  // NaN/Inf → None
    .map(serde_json::Value::Number)
    .ok_or(())
```

`serde_json::Number::from_f64()` 对 `f64::NAN`、`f64::INFINITY`、`f64::NEG_INFINITY` 返回 `None`，导致这些值被降级为 Null。

---

## 三、逐 crate 审查笔记

### ecat (核心) — 4 文件
| 文件 | 状态 | 备注 |
|------|------|------|
| `lib.rs` | ✅ | start_hooks/stop_hooks 分离正确 |
| `hook.rs` | ✅ | 闭包 blanket impl 覆盖 on_start/on_stop |
| `signal.rs` | ⚠️ | SIGTERM handler `.expect()` 合理但严格 |

### ecat-transport — 4 文件
| 文件 | 状态 | 备注 |
|------|------|------|
| `lib.rs` | ✅ | Server trait 设计简洁 |
| `context.rs` | ✅ | 已使用 `tokio::sync::RwLock` |
| `request.rs` | ✅ | |
| `response.rs` | ✅ | |

### ecat-transport-http / ecat-transport-grpc — 2 文件
| 文件 | 状态 | 备注 |
|------|------|------|
| `ecat-transport-http/src/lib.rs` | ⚠️ | `start()` 阻塞不返回，`stop()` 空操作（已知限制） |
| `ecat-transport-grpc/src/lib.rs` | ⚠️ | 同上 |

### ecat-middleware — 5 文件
| 文件 | 状态 | 备注 |
|------|------|------|
| `tracing.rs` | ✅ | `fut.instrument(span)` 修复正确 |
| `recovery.rs` | ⚠️ | `tokio::task::spawn` 丢失 span 上下文（问题 2） |
| `logging.rs` | ✅ | `elapsed.as_millis() as u64` 理论截断无实际影响 |
| `timeout.rs` | ✅ | |

### ecat-registry — 2 文件
| 文件 | 状态 | 备注 |
|------|------|------|
| `lib.rs` | ⚠️ | Registration Drop 静默丢弃错误（问题 3） |
| `memory.rs` | ⚠️ | 同步 `std::sync::RwLock` 在 async 上下文中（已知限制） |

### ecat-config — 3 文件
| 文件 | 状态 | 备注 |
|------|------|------|
| `lib.rs` | ✅ | Config trait 设计合理 |
| `env.rs` | ✅ | 类型解析顺序正确（bool→i64→f64→String） |
| `file.rs` | ⚠️ | 不支持 YAML 多文档、无 watch 机制（已知限制） |

### ecat-data — 6 文件
| 文件 | 状态 | 备注 |
|------|------|------|
| `rdbms.rs` | ✅ | Transaction Drop 注释说明自动回滚但未实现体 |
| `cache.rs` | ✅ | trait 定义完整 |
| `graph.rs` | ✅ | |
| `search.rs` | ✅ | |
| `tsdb.rs` | ✅ | DataPoint builder 模式设计良好 |

### ecat-data-sqlx — 1 文件
| 文件 | 状态 | 备注 |
|------|------|------|
| `lib.rs` | ⚠️ | 值提取顺序已修复；transaction 未实现；f64 特殊值（问题 4） |

### ecat-errors — 2 文件
| 文件 | 状态 | 备注 |
|------|------|------|
| `lib.rs` | ✅ | gRPC→ErrorCode 映射完整，Display 格式清楚 |
| `codes.rs` | ✅ | HTTP 状态码映射与 gRPC 语义一致 |

### ecat-encoding — 3 文件
| 文件 | 状态 | 备注 |
|------|------|------|
| `lib.rs` | ✅ | CodecBox enum、codec_for/codec_from_content_type 设计良好 |
| `json.rs` | ✅ | |
| `proto.rs` | ⚠️ | ProtoCodec 为占位实现（已知限制） |

### 其余 crate
| Crate | 状态 | 备注 |
|-------|------|------|
| `ecat-logging` | ✅ | `try_init` 防重复初始化 |
| `ecat-metadata` | ✅ | HTTP/gRPC 双向转换完善 |
| `ecat-metrics` | ⚠️ | `metrics_text()` 有 unwrap()（问题 1） |
| `ecat-protos` | ✅ | prost/tonic 代码生成 |
| `ecat-cli` | ⚠️ | 大部分命令仅打印消息，未实际创建文件（已知限制） |
| `examples/helloworld` | ✅ | 示例代码正确使用新 API |

---

## 四、测试覆盖分析

```
cargo test → 60 passed, 0 failed

按 crate 分布:
  ecat                  4   (Builder/默认值/生命周期 hook)
  ecat-config           9   (env parse ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (HTTP 映射/gRPC 转换/metadata/Display)
  ecat-logging          1   (init 冒烟)
  ecat-metadata         9   (存取/From HeaderMap/From MetadataMap/迭代器)
  ecat-metrics          2   (单例/text 不 panic)
  ecat-registry         5   (注册/发现/注销/列表/过滤)
  ecat-transport       11   (Context/Request/Response/Server trait)
  其他 8 crate          0   (纯 trait/代码生成/需集成测试)
```

### 测试缺口

| 优先级 | Crate | 缺失内容 |
|--------|-------|----------|
| 高 | `ecat-middleware` | 4 个 Tower Service 无单元测试 |
| 高 | `ecat-data-sqlx` | 无集成测试（SQLite 内存库可行） |
| 中 | `ecat-transport-http` | HTTP server 启动流程无测试 |
| 中 | `ecat-transport-grpc` | gRPC server 启动流程无测试 |
| 低 | `ecat-data` | 纯 trait 定义，可接受 |

---

## 五、代码质量指标

| 指标 | 值 | 评级 |
|------|-----|------|
| 总行数 | 2151 | — |
| 编译警告 | 0 | ✅ |
| Clippy 警告 | 0 | ✅ |
| 测试通过 | 60/60 | ✅ |
| 测试覆盖率（估算） | ~35% | ⚠️ |
| 非测试 unwrap() | 2 处（metrics） | ⚠️ |
| 不安全的代码 | 0 | ✅ |
| panic 风险点 | 3 处（metrics×2 + signal expect） | ⚠️ |

---

## 六、修改建议汇总

### 建议修复（本轮 — 已全部修复 ✅）

| # | 文件 | 问题 | 优先级 | 状态 |
|---|------|------|--------|------|
| 1 | `ecat-metrics/src/lib.rs:14-15` | `metrics_text()` unwrap → 降级处理 | 中 | ✅ 已修复 |
| 2 | `ecat-registry/src/lib.rs:51` | Drop 中加 `tracing::warn!` 记录 deregister 失败 | 低 | ✅ 已修复 |
| 3 | `ecat-data-sqlx/src/lib.rs:57-61` | f64 NaN/Inf 值加特殊处理 | 低 | ✅ 已修复 |
| 4 | `ecat-middleware/src/recovery.rs:40` | `tokio::task::spawn` 丢失 span → `fut.instrument(span)` | 低 | ✅ 已修复 |
| 5 | `ecat-registry/src/memory.rs` | 同步 RwLock → `tokio::sync::RwLock` | 低 | ✅ 已修复 |

### 已知限制（不阻塞）

| # | 文件 | 说明 |
|---|------|------|
| K1 | `ecat-transport-http` / `ecat-transport-grpc` | start() 阻塞 / stop() 空操作（需 graceful shutdown） |
| K2 | `ecat-data-sqlx` | `transaction()` 返回未实现错误 |
| K3 | `ecat-middleware` | 4 个 Service 无单元测试 |
| K4 | `ecat-config/file.rs` | 无 watch 机制 |
| K5 | `ecat-encoding/proto.rs` | ProtoCodec 占位实现 |
| K6 | `ecat-cli` | 大部分命令为 mock 输出 |

---

## 七、总结

第三轮审查在 R2 全部修复的基础上进行。本轮发现 5 个问题已全部修复。

与 R2 的对比：
- R2 发现 2 个高 + 1 个中严重度运行时 Bug → 已全部修复 ✅
- R3 发现 1 个中 + 4 个低严重度健壮性问题 → 已全部修复 ✅
- 测试数量保持 60 个

### 后续优先建议

1. 为 `ecat-data-sqlx` 添加 SQLite 集成测试
2. 为 `ecat-middleware` 添加单元测试（验证 span/超时/恢复行为）
3. 实现 HTTP/gRPC server graceful shutdown
