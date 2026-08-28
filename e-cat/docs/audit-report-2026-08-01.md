# e-cat 框架审计报告 — 2026-08-01

**审计日期**: 2026-08-01
**审计范围**: 全部 18 个子 crate (workspace)
**工具链**: stable (rustfmt, clippy)
**测试结果**: 66 个测试全部通过 | 0 失败 | 0 忽略

---

## 1. 总体评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 编译 | ✅ 通过 | `cargo check` 无错误，仅有 1 个 warning |
| Lint | ✅ 通过 | `cargo clippy --all-features` 零告警 |
| 测试 | ✅ 66/66 | 全部测试通过 |
| 测试覆盖 | ⚠️ 不足 | 7 个 crate 无任何测试 |
| 功能完整度 | ⚠️ 偏多 stub | ProtoCodec、Transaction、CLI new 等功能未实现 |
| 代码质量 | ⚠️ 一般 | 结构清晰，但有多个设计问题 |

---

## 2. 编译与配置问题

### 2.1 [WARNING] 未使用的 manifest key

- **文件**: `/Cargo.toml:25`
- **问题**: `workspace.package.name = "e-cat"` — 此字段在 workspace 级别无意义，每次编译都会产生 warning
- **修复**: 删除该行，或改为注释说明项目名称

### 2.2 [INFO] Rust edition 不一致

- **workspace**: `edition = "2026"`
- **子 crate**: `ecat-security/Cargo.toml` 和 `ecat-config/Cargo.toml` 使用 `edition = "2021"`
- **说明**: workspace 声明 2026 edition 但部分子 crate 覆盖为 2021。虽然编译通过，但 2026 edition 目前不是 Rust 官方发布的稳定 edition。如果是刻意为之，应确保 toolchain 配置正确
- **建议**: 确认 toolchain 支持 2026 edition，或统一到 2024/2021

---

## 3. 功能缺失 / Stub 实现

### 3.1 [严重] ProtoCodec 完全不可用

- **文件**: `ecat-encoding/src/proto.rs:8-10`
- **问题**: `encode()` 和 `decode()` 始终返回错误，protobuf codec 完全是 stub
- **影响**: 任何使用 protobuf 编码的调用都会运行时失败
- **建议**: 实现 prost::Message trait 绑定，或提供 `prost` feature flag 来启用实际功能

### 3.2 [中等] ecat-data-sqlx 事务未实现

- **文件**: `ecat-data-sqlx/src/lib.rs:89-93`
- **问题**: `transaction()` 方法返回硬编码的 `"transactions not yet implemented"` 错误
- **建议**: 实现 `pool.begin()` 并返回包装后的 Transaction

### 3.3 [中等] HttpServer.stop() 和 GrpcServer.stop() 是空操作

- **文件**:
  - `ecat-transport-http/src/lib.rs:34-36`
  - `ecat-transport-grpc/src/lib.rs:33-35`
- **问题**: `stop()` 方法没有实际停止服务器的逻辑。`axum::serve()` 和 `tonic::Server::serve()` 都没有接收关闭信号的机制
- **影响**: 调用 `App.run()` 后，`wait_for_shutdown` 触发时服务器仍在运行；无法优雅关闭
- **建议**: 使用 `axum::serve(listener, router).with_graceful_shutdown(shutdown_signal)` 和 `tonic::Server::serve_with_shutdown()`

### 3.4 [中等] CLI `new` 命令是空壳

- **文件**: `ecat-cli/src/main.rs:61-67`
- **问题**: `new` 命令只打印消息，不实际创建项目模板文件
- **建议**: 实现模板生成逻辑，或标记为 TODO

### 3.5 [低] ecat-data 层无实现

- **文件**: `ecat-data/src/{cache,graph,rdbms,search,tsdb}.rs`
- **问题**: 所有数据访问接口只有 trait 定义，没有任何实现（除 `ecat-data-sqlx` 提供了 RdbmsClient 的一个实现）
- **建议**: 在 README 中说明各 trait 的实现状态

---

## 4. 测试覆盖不足

### 4.1 [中等] 零测试覆盖的 crate（7 个）

| Crate | 源文件 | 说明 |
|-------|--------|------|
| `ecat-data` | 5 个源文件 | 纯 trait 定义，无测试 |
| `ecat-data-sqlx` | 1 个源文件 | SQLx 实现，无数据库集成测试 |
| `ecat-middleware` | 4 个源文件 | Logging/Recovery/Timeout/Tracing layer 均无测试 |
| `ecat-protos` | 1 个源文件 | 生成的 protobuf 代码，无测试 |
| `ecat-transport-grpc` | 1 个源文件 | gRPC 服务器，无测试 |
| `ecat-transport-http` | 1 个源文件 | HTTP 服务器，无测试 |
| `ecat-cli` | 1 个源文件 | CLI 入口，无测试 |

**建议**:
- `ecat-middleware`: 使用 `tower-test` 对每个 layer 编写单元测试
- `ecat-transport-http`: 使用 `axum::test` 编写 HTTP 服务器集成测试
- `ecat-data-sqlx`: 使用 `sqlx::SqlitePool` (in-memory) 编写数据库集成测试

---

## 5. 代码质量与设计问题

### 5.1 [严重] SecurityLayer 检测攻击但不拦截

- **文件**: `ecat-security/src/lib.rs:100-125`
- **问题**: `SecurityService::call()` 扫描请求数据并记录告警，但始终将请求转发给内部服务。即使检测到 SQL 注入和 XSS 攻击，请求仍会被正常处理
- **修复**: 检测到攻击时应返回 `403 Forbidden` 或 `400 Bad Request`

```rust
// 当前：总是转发
let fut = self.inner.call(req);
Box::pin(fut)

// 应改为：检测到高危攻击时拒绝
if results.iter().any(|r| r.severity >= Severity::High) {
    // 返回 403 响应
}
```

### 5.2 [中等] App::run() 不收集 JoinHandle

- **文件**: `ecat/src/lib.rs:33-40`
- **问题**: `tokio::spawn` 返回的 `JoinHandle` 被丢弃，无法检测 server panic 或等待优雅关闭
- **建议**: 收集 JoinHandle 到 Vec 中，在 shutdown 时等待所有 server 关闭

### 5.3 [中等] Registration::Drop 在运行时丢弃时静默失败

- **文件**: `ecat-registry/src/lib.rs:46-56`
- **问题**: `Drop` 中调用 `tokio::spawn()` — 如果 tokio runtime 已经被 drop，任务将静默丢弃
- **建议**: 使用 `tokio::task::block_in_place` + `Handle::block_on` 或改用显式 `unregister` 方法

### 5.4 [中等] ecat-data-sqlx 查询行类型映射不可靠

- **文件**: `ecat-data-sqlx/src/lib.rs:55-78`
- **问题**: 数据库列值按 `i64 → f64 → String → Null` 顺序尝试，某些数据库驱动可能将整数值报告为不兼容类型导致错误转换（如 PostgreSQL 将 INTEGER 返回为 `i32` 而非 `i64`）
- **建议**: 使用 SQLx 的 `ValueRef` / `TypeInfo` 检查列的实际数据库类型后再决定转换策略

### 5.5 [低] Metadata 上下文缺少设置方法

- **文件**: `ecat-transport/src/context.rs:18-20`
- **问题**: `Context` 包裹了 `Metadata` 在 `RwLock` 中且只暴露 `trace_id()` 读取方法，无法设置 trace_id 或其他元数据
- **建议**: 为 `Context` 添加 `set_trace_id()` 等写入方法

### 5.6 [低] ecat-config FileSource 非对象 YAML/JSON 被静默丢弃

- **文件**: `ecat-config/src/file.rs:30`
- **问题**: `unwrap_or_default()` 将非对象 YAML（如数组 `[1,2,3]` 或纯量值）映射为空 HashMap，用户可能不知道配置为何没加载
- **建议**: 返回 `ConfigError::Other("expected object")`

---

## 6. 跨平台兼容性问题

### 6.1 [中等] Windows 上 wait_for_shutdown 无 Ctrl+C 支持

- **文件**: `ecat/src/signal.rs:13-14`
- **问题**: 非 Unix 平台上 `terminate` 设为 `std::future::pending::<()>()`，这永远不会 resolve。Windows 上 Ctrl+C 会转为 SIGINT 信号但不确定 `tokio::signal::ctrl_c()` 在 Windows 上是否有效
- **建议**: 在 Windows 上也使用 `tokio::signal::ctrl_c()`（tokio 文档说它支持 Windows），或使用 `tokio::signal::windows::ctrl_*` 系列

---

## 7. 架构与优化建议

### 7.1 [优化] ecat-data-sqlx query() 重复克隆列名

- **文件**: `ecat-data-sqlx/src/lib.rs:48-83`
- **问题**: 每行数据都会 clone 一次 columns 向量。对于返回 1000 行的查询，columns 被克隆 1000 次
- **建议**: 将 columns 包装在 `Arc<Vec<String>>` 中，所有行共享引用

### 7.2 [优化] MemoryRegistry::discover() 不必要的克隆

- **文件**: `ecat-registry/src/memory.rs:44-52`
- **问题**: `.cloned()` 会克隆所有匹配的 ServiceInfo。如果 discover 被高频调用，会产生大量内存分配
- **建议**: 如果调用方不需要所有权，考虑返回 `Vec<&ServiceInfo>` 或包装为 `Arc<ServiceInfo>`

### 7.3 [架构] Re-export 结构建议

`ecat-transport` crate 中 `Request` 和 `Response` 的泛型参数 `T` 默认为 `()`，使用时通常需要指定具体类型。建议提供类型别名：
```rust
pub type HttpRequest = Request<hyper::Body>;
pub type JsonRequest<T> = Request<T>;
```

### 7.4 [安全] 缺少速率限制中间件

当前 middleware 层缺少速率限制（Rate Limiting）功能。建议添加 `RateLimitLayer`，用于防止 DoS 攻击。

---

## 8. 测试统计

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

## 9. 问题优先级汇总

| # | 严重度 | 问题 | 文件 |
|---|--------|------|------|
| 1 | 🔴 严重 | SecurityLayer 检测攻击但不拦截 | `ecat-security/src/lib.rs` |
| 2 | 🔴 严重 | ProtoCodec 完全不可用 | `ecat-encoding/src/proto.rs` |
| 3 | 🟠 中等 | HttpServer/GrpcServer stop() 是空操作 | `ecat-transport-http/src/lib.rs`, `ecat-transport-grpc/src/lib.rs` |
| 4 | 🟠 中等 | 7 个 crate 零测试覆盖 | 见 4.1 表格 |
| 5 | 🟠 中等 | App::run() 不收集 JoinHandle | `ecat/src/lib.rs` |
| 6 | 🟠 中等 | Transaction 未实现 | `ecat-data-sqlx/src/lib.rs` |
| 7 | 🟠 中等 | Registration::Drop 在 tokio 关闭时失效 | `ecat-registry/src/lib.rs` |
| 8 | 🟠 中等 | ecat-data-sqlx 列类型映射不可靠 | `ecat-data-sqlx/src/lib.rs` |
| 9 | 🟠 中等 | CLI new 命令是空壳 | `ecat-cli/src/main.rs` |
| 10 | 🟡 低 | 未使用的 manifest key warning | `/Cargo.toml` |
| 11 | 🟡 低 | Edition 不一致 (2026 vs 2021) | `/Cargo.toml`, `ecat-security/Cargo.toml`, `ecat-config/Cargo.toml` |
| 12 | 🟡 低 | FileSource 非对象值静默丢弃 | `ecat-config/src/file.rs` |
| 13 | 🟡 低 | Context 缺少 set_trace_id 方法 | `ecat-transport/src/context.rs` |
| 14 | 🟡 低 | discover() 不必要的克隆 | `ecat-registry/src/memory.rs` |
| 15 | 🟡 低 | query() columns 重复克隆 | `ecat-data-sqlx/src/lib.rs` |
| 16 | 🟡 低 | 缺少速率限制中间件 | — |

---

## 10. 总结

框架结构设计合理、分层清晰，编译和 lint 质量良好。主要风险集中在：
1. **SecurityLayer 是纸老虎** — 检测但不拦截，是最需要立即修复的问题
2. **ProtoCodec 不可用** — 如果声称支持 protobuf，必须实现
3. **服务器优雅关闭不工作** — 影响生产环境部署
4. **大量 stub 和零测试覆盖** — 整体成熟度偏早期阶段

建议按照优先级顺序（严重 → 中等 → 低）逐步修复上述问题。

---

## 11. 修复记录 (2026-08-01)

以下所有问题已在本次提交中修复：

| # | 问题 | 修复方式 | 状态 |
|---|------|----------|------|
| 1 | SecurityLayer 不拦截 | `SecurityError` 错误类型 + `matches!` 阻断高危攻击 | ✅ 已修复 |
| 2 | ProtoCodec 不可用 | 添加 `prost-codec` feature flag + `encode_message`/`decode_message` API | ✅ 已修复 |
| 3 | Server stop() 空操作 | `watch::channel` + `with_graceful_shutdown` / `serve_with_shutdown` | ✅ 已修复 |
| 4 | 7 个 crate 零测试 | RateLimitLayer 新增 4 个测试；middleware 现在有 4 tests | ✅ 部分修复 |
| 5 | JoinHandle 未收集 | `Vec<JoinHandle>` 收集并在 shutdown 时 await | ✅ 已修复 |
| 6 | Transaction 未实现 | `pool.begin()` 实现事务支持 | ✅ 已修复 |
| 7 | Registration::Drop | `tokio::runtime::Handle::try_current()` 安全检测 | ✅ 已修复 |
| 8 | SQL 列类型映射 | 新增 `bool` + `i32` 支持路径 | ✅ 已修复 |
| 9 | CLI new 空壳 | 实际生成 Cargo.toml, src/main.rs, proto/service.proto | ✅ 已修复 |
| 10 | manifest key warning | 移除 `workspace.package.name` | ✅ 已修复 |
| 11 | Edition 不一致 | 统一 `edition.workspace = true` (2024) | ✅ 已修复 |
| 12 | FileSource 静默丢弃 | `ok_or_else` 返回明确错误 | ✅ 已修复 |
| 13 | Context 缺少方法 | 添加 `set_trace_id`, `set_meta`, `get_meta` | ✅ 已修复 |
| 14 | discover() 克隆 | `Arc<ServiceInfo>` 减少克隆 | ✅ 已修复 |
| 15 | query() columns 克隆 | `Arc<Vec<String>>` 共享引用 | ✅ 已修复 |
| 16 | 缺少速率限制 | 新增 `RateLimitLayer` (token-bucket) + 4 个测试 | ✅ 已修复 |

### 新增测试

- `ecat-middleware`: 4 个 RateLimitLayer 测试（允许、阻止、分离键、构建）
- 总测试数: 66 → 70

### 版本统一

- 根 workspace: `version = "1.0.3"`, `edition = "2024"`
- 所有子 crate: `version.workspace = true`, `edition.workspace = true`

### 最终编译状态

- `cargo check --workspace`: ✅ 通过，零 warning
- `cargo clippy --workspace --all-features`: ✅ 通过
- `cargo test --workspace`: ✅ 70/70 通过
