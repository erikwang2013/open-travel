# e-cat 框架审计报告 R2 — 2026-08-01

**版本**: 1.0.5
**范围**: 全部 18 个子 crate
**结论**: `cargo check` / `cargo clippy --all-features` / `cargo test` 全部通过，70 tests ✅

---

## 1. 上次修复回顾（16/16 已修复）

上次审计（R1）发现的问题已全部修复：SecurityLayer 阻断攻击、ProtoCodec prost 支持、Server 优雅关闭、JoinHandle 收集、Transaction 实现、Registration Drop 安全检测、列类型映射增强、CLI new 文件生成、版本/edition 统一、FileSource 错误处理、Context 元数据方法、discover Arc 优化、query columns Arc 优化、RateLimitLayer 新增。

---

## 2. 本轮新发现问题

### 2.1 [严重] CLI `new` 生成的模板代码无法编译

- **文件**: `ecat-cli/src/main.rs:79-97`
- **问题**: 生成的 `Cargo.toml` 使用 `workspace = true` 依赖引用和 `path = "../ecat"` 相对路径，但 `ecat new myapp` 创建的独立项目不在 e-cat workspace 内，这些引用全部会解析失败
- **影响**: `ecat new` 创建的项目根本无法编译
- **修复**: 模板应使用带版本号的实际依赖，而非 workspace 引用

```toml
# 当前（错误）：
tokio.workspace = true           # 项目不在 workspace 中，报错
ecat = { path = "../ecat" }      # 相对路径无效

# 应改为：
tokio = { version = "1", features = ["full"] }
ecat = "1.0.5"
```

### 2.2 [严重] ecat-data-sqlx `transaction()` 丢弃真实数据库事务句柄

- **文件**: `ecat-data-sqlx/src/lib.rs:100-106`
- **问题**: `pool.begin()` 返回真实的数据库事务句柄 `Transaction<'_, DB>`，但代码以 `_tx` 绑定后立即丢弃。当 `_tx` drop 时，数据库事务自动回滚。返回的 `ecat_data::Transaction` 是空壳，其 `commit()/rollback()` 方法毫无效果
- **影响**: 所有使用 `transaction()` 的代码都在无事务保护下运行，数据一致性无法保证
- **修复**: 需要重新设计 `ecat_data::Transaction` 结构体，使其持有真实的数据库事务句柄

### 2.3 [中等] SecurityLayer 不扫描请求体

- **文件**: `ecat-security/src/lib.rs:117-127`
- **问题**: `call()` 只扫描 URI 和 HTTP 头部，完全不检查请求体。攻击者可将 SQL 注入/XSS payload 放在 POST body 中轻松绕过检测
- **影响**: 大幅降低攻击检测的有效覆盖范围
- **修复**: 需要添加 body 扫描能力，或提供 `scan_body()` 公开方法供调用方在读取 body 后使用

### 2.4 [中等] RateLimitLayer 使用同步 Mutex + 无过期清理

- **文件**: `ecat-middleware/src/ratelimit.rs:10-38`
- **问题 1**: `std::sync::Mutex` 在 async 上下文中使用 — 如果锁竞争，会阻塞整个 tokio worker 线程
- **问题 2**: `buckets: HashMap<String, (u32, Instant)>` 从不清理过期 key，长期运行的服务器内存无限增长（每个新 IP/key 永久占据内存）
- **影响**: 高并发下性能下降，长时间运行后内存泄漏
- **修复**: 改用 `tokio::sync::Mutex`，并在 `allow()` 中定期清理过期条目

### 2.5 [中等] ecat-data-sqlx 裸 SQL 无参数化 API

- **文件**: `ecat-data-sqlx/src/lib.rs:24-29, 32-36`
- **问题**: `execute(&self, sql: &str)` 和 `query(&self, sql: &str)` 仅接受原始 SQL 字符串，trait 层面无参数绑定方法。调用方若拼接用户输入到 SQL 中，会导致 SQL 注入
- **影响**: 虽然 trait 本身不直接暴露安全漏洞，但缺少参数化 API 会诱导调用方写出不安全代码
- **建议**: 为 `RdbmsClient` trait 添加 `execute_with` 和 `query_with` 方法使用参数绑定

### 2.6 [低] query() 中 Arc::clone 仍在闭包内

- **文件**: `ecat-data-sqlx/src/lib.rs:50-53`
- **问题**: `let cols = std::sync::Arc::clone(&columns)` 在 `rows.iter().map()` 闭包内执行。虽然 Arc::clone 很轻量（仅原子引用计数递增），但可提到闭包外部避免每行一次原子操作
- **建议**: 在 `iter()` 前做一次 clone，闭包内捕获该 clone

### 2.7 [低] ProtoCodec 的 trait impl 与新 API 不一致

- **文件**: `ecat-encoding/src/proto.rs`
- **问题**: `Codec` trait 的 `encode/decode` 仍只返回错误；新增的 `encode_message/decode_message` 是正确路径但方法名不匹配 trait。使用者可能先尝试 `codec.encode()` 然后困惑为何失败
- **建议**: 在文档/注释中说明：proto 类型应使用 `encode_message/decode_message` 而非 Codec trait 方法

---

## 3. 当前状态总览

| 维度 | 状态 |
|------|------|
| `cargo check` | ✅ 零 warning |
| `cargo clippy --all-features` | ✅ 零告警 |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 通过 |
| 版本统一 | ✅ 1.0.5 |
| Edition 统一 | ✅ 2024 |

### 测试分布

| Crate | Tests | 说明 |
|-------|-------|------|
| ecat | 4 | ✅ |
| ecat-config | 9 | ✅ |
| ecat-encoding | 15 | ✅ |
| ecat-errors | 4 | ✅ |
| ecat-logging | 1 | ✅ |
| ecat-metadata | 9 | ✅ |
| ecat-metrics | 2 | ✅ |
| ecat-middleware | 4 | ✅ (含 RateLimitLayer) |
| ecat-registry | 5 | ✅ |
| ecat-security | 6 | ✅ |
| ecat-transport | 11 | ✅ |
| ecat-data | 0 | — (纯 trait 定义) |
| ecat-data-sqlx | 0 | ⚠️ 无 DB 集成测试 |
| ecat-protos | 0 | — (生成代码) |
| ecat-transport-grpc | 0 | ⚠️ |
| ecat-transport-http | 0 | ⚠️ |
| ecat-cli | 0 | ⚠️ |

---

## 4. 问题优先级

| # | 严重度 | 问题 | 文件 | 用户影响 |
|---|--------|------|------|----------|
| 1 | 🔴 | CLI `new` 模板生成不可编译代码 | `ecat-cli/src/main.rs:79` | 新用户首个命令即失败 |
| 2 | 🔴 | transaction() 丢弃真实 DB 事务句柄 | `ecat-data-sqlx/src/lib.rs:100` | 数据一致性无保障 |
| 3 | 🟠 | SecurityLayer 不扫描 body | `ecat-security/src/lib.rs:117` | 攻击者可绕过检测 |
| 4 | 🟠 | RateLimitLayer std Mutex + 内存泄漏 | `ecat-middleware/src/ratelimit.rs:10,25` | 并发性能 + OOM |
| 5 | 🟠 | 裸 SQL 无参数化 API | `ecat-data-sqlx/src/lib.rs:24` | SQL 注入风险 |
| 6 | 🟡 | query() Arc clone 位置 | `ecat-data-sqlx/src/lib.rs:53` | 微小性能优化 |
| 7 | 🟡 | ProtoCodec API 不一致 | `ecat-encoding/src/proto.rs` | 使用者困惑 |

---

## 6. 修复记录 (2026-08-01 R2)

| # | 问题 | 修复方式 | 状态 |
|---|------|----------|------|
| 1 | CLI new 模板不可编译 | 改用版本化依赖 (`ecat = "1.0"`, `tokio = "1"` 等) | ✅ |
| 2 | transaction() 丢弃 DB 事务 | `Transaction::with_inner()` 持有真实句柄，sqlx 通过 `Box<dyn Any>` 传递 | ✅ |
| 3 | SecurityLayer 不扫描 body | 新增 `scan_body(&[u8])` 公开方法 | ✅ |
| 4 | RateLimitLayer Mutex + 泄漏 | `tokio::sync::Mutex` + 每 100 key 清理过期条目 | ✅ |
| 5 | 裸 SQL 无参数化 API | `RdbmsClient` 新增 `execute_with`/`query_with` 参数化方法 | ✅ |
| 6 | query() Arc clone 位置 | `Arc::clone` 移到 `iter()` 外部，所有行共享引用 | ✅ |
| 7 | ProtoCodec API 不一致 | 模块级文档 + struct 文档说明使用方式 | ✅ |

### 最终状态

| 检查项 | 结果 |
|--------|------|
| `cargo check` | ✅ 零 error / 零 warning |
| `cargo clippy --all-features` | ✅ 零 warning |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 通过 |
| 版本 | 1.0.5 (全部统一 workspace 继承) |
| Edition | 2024 |
