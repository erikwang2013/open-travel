# e-cat 深度审查报告 — 2026-08-01 R6

## 总体评估

| 维度 | 状态 | 说明 |
|------|------|------|
| 编译 | 通过 | 50 crates, 零错误 |
| 测试 | 通过 | 全部通过, 零失败 |
| Clippy | 通过 | 零警告 (`-D warnings`) |
| unsafe | 零 | 代码库无 unsafe 块 |
| 文件规模 | 良好 | 仅 `ecat-auth` (540行) 超过 500行 建议值 |

## 发现项 (15 项)

### 安全相关

#### 1. [严重] XOR「加密」不是真正的加密
**文件:** `ecat-config/src/encrypted.rs:45-56`
**问题:** `decrypt()` 使用 XOR + 重复密钥，这是一种混淆而非加密，可被轻易破解。密钥会在每个字节位置重复使用，使密文极易受到频率分析攻击。
**建议:** 用 AES-256-GCM (`aes-gcm` crate) 替换，或明确标注为「混淆」而非「加密」。

#### 2. [严重] `execute_with`/`query_with` 默认实现静默丢弃参数
**文件:** `ecat-data/src/rdbms.rs:86-103`
**问题:** trait 中的默认实现接收参数但忽略它们 (`let _ = params;`)，直接调用原始 `execute(sql)`。除 `ecat-data-sqlx` 外的所有后端（ClickHouse、QuestDB）都继承此行为。如果用户用参数化方法替换后端，参数会被静默丢弃，导致 SQL 注入漏洞。
**建议:** 默认实现应返回「不支持」错误，或让每个后端正确实现参数绑定。

#### 3. [高危] 密码通过 URL 明文嵌入
**文件:** `ecat-data-sqlx/src/lib.rs:40`, `ecat-data-redis/src/lib.rs:43`
**问题:** `connect_with_auth()` 使用 `replacen("://", "://user:pass@")` 将凭证直接嵌入 URL。这些 URL 可能被日志、错误消息或调试输出记录。
**建议:** 使用各后端原生认证机制；或至少在拼接前对用户名/密码进行 URL 编码。

#### 4. [中危] TLS 配置失败导致 panic
**文件:** 8 个 data-* crate（ClickHouse、QuestDB、Elasticsearch、OpenSearch、ArangoDB、Neo4j、NebulaGraph、InfluxDB、IoTDB）
**模式:** `.expect("TLS client build failed")` — 所有 `from_config()` 构造器在 TLS 配置错误时都会 panic。
**建议:** 将 `from_config()` 改为返回 `Result`，或将 TLS 客户端构建改为惰性/容错方式。

### 功能正确性

#### 5. [高危] `ecat-versioning` Header 路由无效
**文件:** `ecat-versioning/src/lib.rs:56-64`
**问题:** `build_header_router()` 将所有版本嵌套在同一 `/api` 路径下，但不按版本 header 过滤。axum 会将所有版本路由注册到同一路径，导致路由冲突和不可预测的行为。`extract_version()` 函数存在但从未在路由中使用。
**建议:** 使用 axum middleware/layer 检查 Accept header 并路由到正确的版本路由，而非将所有版本扁平化到同一路径。

#### 6. [中危] Redis TTL 截断：亚秒过期变为永不过期
**文件:** `ecat-data-redis/src/lib.rs:76-77`
**问题:** `Duration::as_secs()` 向零截断。设置为 500ms TTL 会在 `secs == 0` 时静默变为永不过期，走的是 `SET` 而非 `SETEX` 分支。
**建议:** 对亚秒 TTL，至少设为 1 秒，或使用 `SET ... PX`（毫秒）替代 `SETEX`。

#### 7. [中危] `StaticResolver::add_service` 在锁竞争时 panic
**文件:** `ecat-client/src/lib.rs:27-29`
**问题:** 使用 `try_write()` 并 expect，若有任何其他写锁持有者存在就会 panic。builder 模式使此问题难以触发，但在并发代码中是定时炸弹。
**建议:** 使用 `blocking_write()`（若在同步上下文中）或改为接受 `&mut self` 以避免锁需求。

### 代码质量

#### 8. [中危] `std::sync::Mutex` 在异步上下文中的使用
**文件:** `ecat-data-memcached/src/lib.rs:7,24`
**问题:** 在 async trait 实现中使用 `std::sync::Mutex`。虽然锁持有时间极短（仅 HashMap 操作），但在高竞争下理论上可能阻塞异步运行时。
**建议:** 对于这个内存缓存的特定使用场景，由于临界区极短且无 `.await` 点，使用 `std::sync::Mutex` 实际上是可以接受的。但若未来需要在锁内执行 I/O 操作，应改用 `tokio::sync::Mutex`。

#### 9. [低] 手写 base64 实现
**文件:** `ecat-registry-etcd/src/lib.rs:148-193`
**问题:** ~45 行手写 base64 编解码器，可能存在边界情况 bug。Rust 生态中有 `base64` crate 等经过充分审查的替代方案。
**建议:** 用 `base64` crate 替换，减少维护负担和潜在 bug。

#### 10. [低] `RandomBalancer` 不随机
**文件:** `ecat-client/src/lib.rs:91-105`
**问题:** 使用 `Instant::now()` 哈希作为随机源。同一实例内同时发出的调用会获得相同的「随机」选择。`checked_add(0)` 是多余的操作。
**建议:** 使用 `rand` crate 或至少使用 `std::collections::hash_map::RandomState`。

#### 11. [低] `ecat-data-sqlx` 中不必要的 `Arc<Vec<String>>`
**文件:** `ecat-data-sqlx/src/lib.rs:79-87, 197-203`
**问题:** 列名被包装在 `Arc<Vec<String>>` 中，但每个 `Row` 构造函数都会克隆整个列名列表 (`(*cols).clone()`)。`Arc` 仅在迭代期间使用一次，用 `Rc` 或直接 `clone()` 即可。
**建议:** 在 `query()` 和 `query_with()` 中，将 `Arc<Vec<String>>` 替换为普通的 `Vec<String>`。每行的单独克隆成本与通过 Arc 解引用 + 克隆相同。

### 设计/架构

#### 12. [信息] QuestDB 使用 GET + 查询参数
**文件:** `ecat-data-questdb/src/lib.rs:76, 91`
**问题:** SQL 通过 GET 查询参数发送，受 URL 长度限制（通常 ~2000-8000 字符）。大查询会被截断。
**建议:** 改为 POST + body 方式，或为简单查询保留 GET，复杂查询使用 POST。

#### 13. [信息] `#[allow(dead_code)]` 散落各处
**文件:** `ecat-registry-consul/src/lib.rs:225`, `ecat-data-memcached/src/lib.rs:25-28`, `ecat-auth/src/lib.rs:52`
**问题:** username/password 字段存储在内存中但标记为 dead_code（in-memory memcached 中不需要；auth 中的 RSA 变体尚未实现）。
**建议:** 要么实现缺失的功能路径，要么删除这些字段，要么添加文档说明为何保留。

#### 14. [信息] 部分 HTTP 客户端缺少 Content-Type header
**文件:** `ecat-data-influxdb/src/lib.rs:96-103`, `ecat-data-clickhouse/src/lib.rs:87-89`
**问题:** 部分 POST 请求未设置 `Content-Type` header，依赖服务端自动检测。
**建议:** 始终设置显式 Content-Type 以确保兼容性。

#### 15. [信息] `ecat-auth` 超过 500 行
**文件:** `ecat-auth/src/lib.rs` (540 行)
**问题:** CLAUDE.md 要求文件保持在 500 行以下。auth crate 是唯一超过此限制的文件。
**建议:** 将 JWT 验证逻辑拆分到 `ecat-auth/src/jwt.rs`，或按功能拆分。

## 优化机会（非 Bug）

| # | 位置 | 建议 |
|---|------|------|
| O1 | 所有 data-* crate | 所有 `from_config()` 中重复的 TLS 客户端构建模式可提取到共享宏或函数 |
| O2 | `ecat-data-sqlx` | `query()` 和 `query_with()` 中的行类型转换逻辑（117行重复）可提取到辅助函数 |
| O3 | `ecat-client` | `HttpClient::get()` 和 `post()` 共享相同的「resolve → pick → build URL」管道 — 可提取 |
| O4 | `ecat-data` | 所有 5 个 traits（Rdbms/Cache/Graph/Search/Tsdb）的自定义错误类型可统一为单一 `DataError` 枚举 |
| O5 | `ecat-data-redis` | 每个方法中的 `self.conn.clone()` 是不必要的 — `MultiplexedConnection` 已为 `Clone` 设计以支持共享 |

## 指标汇总

| 指标 | 数值 |
|------|------|
| 总 crate 数 | 50 |
| Rust 源文件总行数 | 7,968 |
| `expect()` 在非测试代码中 | 12 |
| `unwrap()` 在非测试代码中 | 0 |
| `unsafe` 块 | 0 |
| `panic!` 在非测试代码中 | 0 |
| `#[allow(dead_code)]` | 4 |
| TODO/FIXME/HACK | 0 |
| std Mutex 在异步代码中 | 1 (memcached) |

## 结论

代码库处于良好状态——编译、测试和 clippy 全部通过，无 unsafe 代码，无 panic 宏。最关键的两个问题是 **XOR「加密」**（安全性为假）和 **参数化查询默认实现静默丢弃参数**（安全漏洞）。Header 路由功能也完全不可用。其他问题相对较小，属于可维护性层面的优化。

**推荐优先修复顺序:**
1. `execute_with`/`query_with` 默认实现 → 返回错误而非静默丢弃参数
2. XOR 加密 → 真正的 AEAD 加密，或重命名为「混淆」
3. Header 版本路由 → 实现实际的 header 路由
4. `from_config()` → 返回 Result 而非 expect-panic
5. Redis TTL 截断 → 亚秒 TTL 至少使用 1 秒

## 修复状态 (R6 → R6.1)

| # | 问题 | 状态 | 变更 |
|---|------|------|------|
| 1 | XOR "加密" | 已修复 | `EncryptedSource` → `ObfuscatedSource`，`decrypt` → `deobfuscate`，前缀 `enc:` → `obfs:`，添加文档说明这是混淆而非加密 |
| 2 | `execute_with`/`query_with` 静默丢弃参数 | 已修复 | 默认实现改为返回错误 `"parameterized ... not supported by this backend"` |
| 3 | 密码明文嵌入 URL | 已修复 | `connect_with_auth` 方法中使用 `percent_encode()` 对凭证进行编码 |
| 4 | TLS `expect()` panic | 已修复 | 9 个 crate 的 `from_config()` 改为返回 `Result`，`RdbmsError` 新增 `Config` 变体 |
| 5 | Header 路由无效 | 已修复 | 使用 `from_fn_with_state` 中间件实现版本验证，新增测试 `header_versioned_router_builds` |
| 6 | Redis TTL 截断 | 已修复 | `set_ex` → `pset_ex`，使用毫秒精度避免亚秒 TTL 被截断为永不过期 |
| 7 | `StaticResolver` 锁竞争 panic | 已修复 | `try_write()` → `blocking_write()` |
| 8 | `RandomBalancer` 不随机 | 已修复 | 用 `RandomState::new().build_hasher()` 替代 `Instant::now()` 哈希 |
| 9 | `std::sync::Mutex` 在异步上下文 | 已修复 | 替换为 `tokio::sync::Mutex` |
| 10 | 手写 base64 | 已修复 | 替换为 `base64` crate 0.22 |
| 11 | `Arc<Vec<String>>` 开销 | 已修复 | 替换为普通 `Vec<String>`，移除不必要的 Arc 包装 |
| 12 | QuestDB GET 方式发送 SQL | 已修复 | 改为 POST + body，添加 Content-Type header |
| 13 | `#[allow(dead_code)]` | 已修复 | memcached 字段加 `_` 前缀；consul 字段加 `_` 前缀并移除 allow；auth 中 `Rsa` → `RsaReserved` |
| 14 | 缺少 Content-Type | 已修复 | InfluxDB、ClickHouse、IoTDB 请求添加显式 Content-Type |
| 15 | `ecat-auth` 超过 500 行 | 已修复 | 拆分为 `claims.rs`(31) + `jwt.rs`(139) + `apikey.rs`(96) + `oauth2.rs`(173) + `helpers.rs`(28) + `lib.rs`(98) |

### 受影响的 Crate

| Crate | 变更类型 |
|-------|----------|
| `ecat-data` | trait 默认实现、`RdbmsError::Config` 变体 |
| `ecat-config` | `EncryptedSource` → `ObfuscatedSource` |
| `ecat-versioning` | Header 路由中间件实现 |
| `ecat-data-redis` | TTL 毫秒精度、凭证 URL 编码 |
| `ecat-data-sqlx` | 凭证 URL 编码、移除 Arc 开销 |
| `ecat-data-clickhouse` | `from_config` → `Result`、Content-Type header |
| `ecat-data-questdb` | `from_config` → `Result`、GET → POST |
| `ecat-data-elasticsearch` | `from_config` → `Result` |
| `ecat-data-opensearch` | `from_config` → `Result` |
| `ecat-data-arangodb` | `from_config` → `Result` |
| `ecat-data-neo4j` | `from_config` → `Result` |
| `ecat-data-nebulagraph` | `from_config` → `Result` |
| `ecat-data-influxdb` | `from_config` → `Result`、Content-Type header |
| `ecat-data-iotdb` | `from_config` → `Result`、Content-Type header |
| `ecat-data-memcached` | `std::sync::Mutex` → `tokio::sync::Mutex`、dead_code 清理 |
| `ecat-client` | `StaticResolver`、`RandomBalancer` 修复 |
| `ecat-registry-etcd` | base64 替换为 crate |
| `ecat-registry-consul` | dead_code 清理 |
| `ecat-auth` | 拆分为 6 个模块、dead_code 清理 |

### 最终验证 (R6.2)

| 维度 | 状态 |
|------|------|
| Build | 通过，零错误零警告 |
| Test | 全部通过，零失败 |
| Clippy (`-D warnings`) | 通过，零警告 |
| 文件规模 | 全部 ≤ 300 行 |
