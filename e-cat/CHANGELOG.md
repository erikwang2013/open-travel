# Changelog

## [3.0.3] — 2026-08-27

### Added
- 全球转账打赏：README / README.en 及全部 12 语言 README 新增 ZA Bank 汇款信息（WANG KEXUN / SWIFT AABLHKHHXXX / 银行编号 387），含 Citibank（港元/人民币/美元）与 BNY Mellon（其他币种）跨境汇款代理行附注
- API 参考文档 `docs/api.md`：端口约定、/health /ready /metrics 内置端点、错误格式、GraphQL/OpenAPI/WebSocket/版本路由扩展接口
- 12 语言文档目录 `docs/i18n/{en,ja,ko,ru,de,fr,es,pt,hi,ar,bn,id}/`：全部 24 份文档翻译 + 3 张图片副本，README 顶部语言切换器互链

## [3.0.2] — 2026-08-27

### Fixed
- `ecat-security` SQL 注入扫描绕过：URI 百分号编码载荷（`?q=SELECT%20*%20...`）此前可绕过 header 层检测（检测正则要求字面空白），现先 percent-decode 再匹配，仅检测用解码、转发/日志 URI 不变
- `ecat-data-sqlx` AnyPool 未安装驱动：`connect()/from_config()` 首次连接即 panic "No drivers installed"，现入口处一次安装（幂等，覆盖全部连接路径）
- `ecat-data-influxdb` line protocol 过度转义：字符串 field 值不再转义空格（规范只需转义 `"` 与 `\`）；tag/field 输出经排序保证确定性
- `ecat-data-clickhouse` 建表缓存永不失效：TTL 60s 过期重建；INSERT 报缺表错误时清缓存重试一次
- `ecat-events` dev-dependencies 补 tokio features（macros/rt/time）：此前单独编译该 crate 测试目标必失败，被 workspace feature 并集掩盖

### Tests
- 全面单元测试补写：51 个 crate 全覆盖，+206 测试（核心 40 / data 66 / mq-transport 54 / app 46），workspace 总计 675 测试全绿；测试团队报告见 docs/test-report-2026-08-26.md

## [3.0.1] — 2026-08-17

### Fixed
- `ecat-mq-kafka` auto_commit=true 消息丢失量化告警：消费者流被 drop 时 warn 记录通道内未消费条数（librdkafka 已交付 offset 无法恢复，告警为可达最优解；默认 auto_commit=false 保持安全基线）
- `ecat-data` rdbms rollback 误报：显式 `rollback()` 后 Drop 不再触发 "dropped without commit — rolling back" 误导性 warn（新增 rolled_back 标志）
- `ecat-auth` OAuth2 内省安全：响应体 1MiB 有界读取（防无界内存）、`active=true` 但 sub 缺失/为空即拒绝

### Performance
- 根 Cargo.toml 新增 `[profile.release] lto = "thin"`：60 个跨 crate 热点调用可跨 crate 内联

### Tests
- `ecat-config` FileSource::load() 补 4 个测试（JSON/YAML 值断言、解析错误、顶层非 object 报错）——启动必经路径此前零测试
- `ecat-circuit-breaker` 补状态机迁移测试（open 拒绝请求、half-open 失败重开）

## [3.0.0] — 2026-08-14

### Breaking
- `ecat-mq` MessageStream Bytes 化：`poll_recv` 返回 `Vec<u8>` → `bytes::Bytes`（新增 `bytes = "1"` 依赖）；`publish(&[u8])` 签名不变零迁移。mqtt/nats 原生 Bytes 零拷贝透传、rabbitmq Vec→Bytes 所有权转移；`ecat-events` Handler 消费路径同步 Bytes 化，消除每消息拷贝
- `from_config` 签名统一：12 个数据 crate 统一为 `Result<Self, XxxError>`（保留领域错误）；`ecat-data-memcached` 由 `Self` → `Result<Self, CacheError>`（恒 Ok，唯一功能性破坏），redis/sqlx/mongodb 核实已符合
- workspace 依赖同步：`workspace.dependencies` 15 处 `ecat-*` 版本统一 3.0.0（修复 v2.4.3 发布时的遗漏）

### Added
- `ecat-bench` BenchResult 新增 `pub p95_latency_us`：p95 计算与 p50/p99 同公式（count*0.95 索引界内、空样本 0.0）、`print` 与 `http_bench compare` 补 p95 行、不变式测试 p95_between_p50_and_p99

## [2.4.3] — 2026-08-14

### Added
- `ecat-graphql` 字段参数与嵌套 selection 支持：`FieldRequest`（args/variables/selection）与 `GraphQLField` 富 resolver（`query_field` / `mutation_field` 注册）；两阶段手写解析器（字面量参数、`$var` 解引用、嵌套 selection 树、MAX_DEPTH=32）；legacy resolver 自动获得字段参数（合并进 variables），无参时逐字节兼容旧行为
- `ecat-auth` OAuth2 内省缓存加固：claims 白名单过滤（默认缓存 sub/exp/iat/role + extra 的 iss/aud/scope/roles，`cache_claims_whitelist` 可配置、"*" 逃生门；miss 仍返回完整 claims）；TTL 过期条目写路径主动清除（`purge_expired`）
- CI：`.github/workflows/ci.yml` 新增独立 cargo-audit 闸门 job（预编译 musl 二进制 + `--deny warnings` + `.cargo/audit.toml` 8 项已评估 ignore）

### Fixed
- `ecat-data-s3` quick-xml 0.32.0→0.41.0：修复 2 个高危 CVE（RUSTSEC-2026-0194/0195），`parse_list_xml` 适配新事件模型（文本追加累积 + 实体 GeneralRef 还原）
- CI 原 cargo-audit 步骤失效：`--deny medium` 为 cargo-audit ≥0.20 已废弃语法（被 continue-on-error 掩盖），改为 `--deny warnings` 并移除 continue-on-error

### Docs
- docs/dependency-cve-tracking.md：CI 接入说明、新增 protobuf / instant 评估、各条目补 RUSTSEC 编号
- README×2：Known Limitations 更新（GraphQL 支持字段参数与嵌套 selection；OAuth2 缓存白名单过滤 + TTL 清除）

## [2.4.2] — 2026-08-14

### Added
- `ecat-middleware` M3 `ValidateLayer`：请求校验中间件——`RequestValidator` trait（`validate(&Request) -> Result<(), ValidateError>`）、`ValidateError`（支持自定义 HTTP 状态码，默认 400）、`ValidateLayer::from_fn` 闭包入口（`FnValidator` 包装）；校验失败短路返回错误响应，不进入内层服务
- `ecat-middleware` M4 CORS：`cors` feature 引入 `tower-http`（optional，0.6 线）并 re-export `AllowOrigin` / `Any` / `CorsLayer`
- 示例补齐 U2（3 个）：`examples/databases`（多数据库后端连接）、`examples/middleware`（中间件组合）、`examples/websocket`（WebSocket）
- `ecat-bench`：新增 `http_bench.rs` 正式 bench 示例——bare（裸 axum）/ metrics（+MetricsLayer）/ full（+MetricsLayer+TracingLayer+LoggingLayer）三端点对比，输出 requests/QPS/p50/p99 及相对 bare 的开销；`BENCH_TOTAL` / `BENCH_CONCURRENCY` / `BENCH_WARMUP` / `BENCH_BASE_URL` 环境变量可调

### Fixed
- `ecat-metrics` MetricsLayer 挂载缺陷：`Service::Error` 由 `Box<dyn Error>` 改为透传 `S::Error`——修复 axum `Router::layer` 的 `Into<Infallible>` 约束失败（错误类型不匹配导致 layer 无法挂载）
- `ecat-middleware` ValidateLayer 同款两处：`Service::Error` 透传 `S::Error`；`FnValidator` 手动实现 `Clone`（泛型闭包无法自动 derive）

### Docs
- 新增 docs/dependency-cve-tracking.md：依赖 CVE 跟踪表（rustls-webpki 0.102.8 RUSTSEC-2026-0049 系列、rdkafka-sys cJSON CVE-2025-57052、rustls-pemfile / rsa 低危）+ 跟踪原则
- README×2：构造器命名约定注（`ecat-mq-*` 用 connect、`ecat-data-*` 多数 new，redis/sqlx 例外 connect、mongodb/s3 仅 from_config；既有约定不强制统一，3.0 窗口可评估）

## [2.4.1] — 2026-08-14

### Added
- `ecat-metrics` M1 `MetricsLayer`（tower Layer）：记录请求计数与时延直方图到全局 registry（与 /metrics 端点共享）；指标 `ecat_http_requests_total` / `ecat_http_request_duration_seconds`，标签 method/path/status；`with_path_fn` 自定义 path 标签（高基数路径归一化/脱敏，避免指标基数爆炸）
- `ecat-middleware` M2 `RetryLayer` / `RetryRule`：指数退避重试（`new(max_attempts, base_delay, max_delay)`，含首次共 max_attempts 次）；`RetryRule` trait 自定义重试判定（如按 HTTP 状态码/响应内容），默认规则仅重试服务错误；⚠️ 仅对幂等请求（GET/HEAD/PUT/DELETE）安全
- `ecat` U1 聚合 crate：feature-gated re-export 入口——12 个 feature（http/grpc/middleware/auth/client/events/metrics/tracing/circuit-breaker/consul/remote/redis），默认 http+grpc，`--no-default-features --features <组件>` 精简依赖树

### Fixed
- `ecat-transport-http`：tls_listener accept 通道关闭 panic——accept 循环退出（任务 abort/panic 致 sender 释放、通道关闭）时记录错误并挂起，不再 panic 杀死服务线程；在途连接与优雅停机信号照常处理
- `ecat-middleware` 限流：P1 flaky 测试重构——日志捕获断言改为 limiter 状态断言（消除 writer 捕获竞态）
- `ecat-metrics`：空指标体可区分——无指标注册时输出 `# no metrics registered`（原空响应体无法区分「无数据」与「有数据但输出为空」）
- 数据后端补测（4 个 crate 16 个测试）：`ecat-data-iotdb`（5）、`ecat-data-neo4j`（2）、`ecat-data-arangodb`（4）、`ecat-data-mongodb`（5）

### Security
- `ecat-auth` OAuth2 内省缓存：缓存 key 由 token 明文改为 SHA-256 hash（明文 token 不再驻留内存）；解析出的 claims 仍以明文存于 FIFO 有界缓存（默认 10_000）

### Docs
- README×2：新增聚合 crate（ecat）用法（12 feature 列表/默认 http+grpc）、M1 MetricsLayer 用法（指标名/标签/with_path_fn）、M2 RetryLayer 用法（指数退避/自定义规则/幂等性警告）；已知限制移除 2 条（WebSocket 优雅关闭、熔断判定——均已落地），保留 3 条（GraphQL、OAuth2 内省缓存、Kafka offset）
- README×2 已知限制：Kafka offset 行为说明——默认 `auto_commit=false` 重启从分区末尾（latest）重读、停机期消息被跳过；显式 `auto_commit=true` 才具备 at-least-once 语义
- docs/ecosystem-plan-v3.md：数据后端表按实测逐 crate 核对修正（驱动/能力列）
- CI：新增 cargo-audit 步骤（依赖漏洞扫描，--deny medium，continue-on-error）

## [2.4.0] — 2026-08-14

### Added
- `ecat-auth` JWT：新增 `required_issuer()` / `required_audience()` builder，强制校验 iss/aud 声明（默认不校验，向后兼容）
- `ecat-auth` OAuth2：新增 `cache_capacity(n)` builder（FIFO 有界缓存，默认 10_000，达容量逐出最旧条目）

### Fixed
- `ecat-transport-http`：TLS 握手 DoS 修复——新增 src/tls_listener.rs（后台 accept_loop + 每连接独立 spawn 握手 + 10s 握手超时），慢握手连接不再阻塞其他连接；行为不变，无 API 变更
- `ecat-auth` OAuth2：内省结果缓存由无界改为 FIFO 有界（默认 10_000），防海量唯一 token 内存无界增长
- `ecat-tls`：`skip_verify=true` 与 `ca_cert` 同时配置改为构建报错（跳过校验却配置信任锚的矛盾配置）
- `ecat-events`：消费任务退出（正常/panic）后清理占位，再次 subscribe 可重启消费，修复事件永久静默丢失
- `ecat-data-s3`：TLS 配置面重写——`tls` 字段由 bool 改为 `TlsClientConfig`，复用 `ecat_tls::build_reqwest_client`（rust-s3 → reqwest+rustls）；请求签名改为自实现 AWS SigV4（path-style 寻址，AUTHORIZATION / x-amz-date / x-amz-content-sha256 请求头），修复 S3-1/S3-2 的签名请求头装配与双重 percent-encoding
- `ecat-mq-kafka`：消费改 StreamConsumer（tokio 驱动），消除 ~200ms 固定轮询延迟
- `ecat-mq-kafka`（语义变更，⚠️ 破坏性）：group_id 派生规则——显式配置时派生为 `{group_id}-{topic_hash}`（SHA-256 取 8 位 hex：同一 (group, topic) 跨实例一致，共享消费组负载均衡、offset 组名稳定，hash 后缀消除 "-" 直接拼接的歧义碰撞）；未配置时生成随机组 `ecat-mq-{uuid}`。⚠️ 升级影响：有 group_id 的部署组名由 `{g}-{topic}` 变为 `{g}-{hash8}`，旧 committed offset 孤儿化——升级后按 offset 重置策略（默认 latest）从分区末尾重读，停机期间产生的消息会被跳过；未配置 group_id 的实例各自独立消费组（不再共享负载均衡）。新增 `auto_commit` 配置（默认 false，向后兼容）：true 时 `enable.auto.commit=true`（librdkafka 每 ~5s 自动提交，at-least-once，重启从最近提交点继续，避免停机期消息静默跳过）。消费错误分支新增 tracing::warn
- `ecat-tracing`：TracingLayer span 记录 trace_id（提取自请求头，canonical `x-ecat-trace-id` 优先、`traceparent` 兜底，无 id 时空字段）；⚠️ `TracingService` 的 `Service` 实现由完全泛型特化为 `Service<http::Request<B>>`——使用非 HTTP 请求类型的调用方需调整（编译期变更）
- `ecat-transport`：地址规范化共享——normalize_addr 统一空 host（`:8000`）→ `0.0.0.0:8000`，http/grpc/ws 三端一致，避免无 IPv6 环境绑定失败
- `ecat-scheduler`：任务 panic 韧性——job 改 JoinSet 子任务，panic 记日志后继续下一 tick（不再静默死亡）；run() 同步 panic 记录 warn
- `ecat-versioning`：未知版本 404 路径去掉 builder+unwrap（消除生产 panic 面）

### Docs
- README×2 同步：S3 实现状态表更新（rust-s3 → reqwest+rustls）、JWT 中间件示例补充 `required_issuer` / `required_audience` 用法

## [2.3.5] — 2026-08-07

> 2.3.4 未发布：workspace 版本由 2.3.3 直接跳至 2.3.5（无 v2.3.4 tag）。

### Fixed
- mTLS 测试竞态（2 个 crate）：全量 workspace 测试下 rustls 因同时编译 aws-lc-rs + ring 无法自动选择 CryptoProvider 而 panic——`ecat-transport-grpc` 两个 TLS 测试开头同步调用 `ensure_crypto_provider()`；`ecat-transport-http` 新增 OnceLock 保护的 `ensure_crypto_provider()`，在 `build_server_config`（生产路径）与测试辅助 `client_config` 内调用，一次覆盖全部 3 个 TLS/mTLS 测试
- clippy 告警清零（5 个 crate，11 处）：之前修复引入的嵌套 if / unused_mut / map_or 告警，全部折叠为 let-chain 或等价形式（ecat-cli 5、ecat-auth 2、ecat-events 1、ecat-data-questdb 1、ecat-circuit-breaker 2）

### Docs
- README×2 版本号同步 v2.3.5；docs/alipay.png、docs/weixinpay.png 底部增加 44px 边界并加水印 https://erik.xyz（已验证不遮挡二维码）
- docs/ecosystem-plan-v3.md 更新；Helm Chart appVersion 同步 2.3.5
- 新增团队协作设计（docs/superpowers/specs/2026-08-14-team-design.md）与建队实施计划（docs/superpowers/plans/2026-08-14-team-setup.md）

## [2.3.3] — 2026-08-07

### Added
- mTLS 接入 transport：`HttpServer::tls` / `GrpcServer::tls` 真正生效（tokio-rustls / tonic rustls，支持 CA 校验与强制客户端证书），附自签证书握手测试
- `ecat-cli` proto 子命令真实实现：`proto add` 创建 proto 文件；`proto client/server` 生成 tonic-build `build.rs` 并自动补齐 Cargo.toml 依赖
- `ecat run --watch`：unix 下按进程组终止服务（libc::kill），修复服务二进制成孤儿占端口
- `ecat upgrade`：真实批量升级 ecat-* 依赖版本（改写 Cargo.toml 版本要求 + cargo update）
- `ecat new` 模板：ecat-* 依赖版本与当前版本一致（原硬编码 1.0）
- `ecat-testing` MockServer：真实 axum mock（set_response / received_requests），不再仅翻转布尔标志
- Dockerfile：CMD 改为运行示例服务（helloworld），新增 .dockerignore

### Fixed
- `ecat` App::run()：已有 tracing subscriber 时跳过 `ecat_logging::init()`，修复与 ecat-tracing / ecat-tracing-otlp 的 init 冲突
- `ecat-tracing`：inject/extract 头名统一为 `x-ecat-trace-id`（与 ecat-metadata 一致），trace_id 改用 uuid 生成，TracingLayer span 注入 trace_id
- `ecat-circuit-breaker`：half-open 探活成功后清空滑动窗口，修复闭环后旧失败率立刻再次触发 open
- `ecat-transport-ws`：实现 stop()（关闭信号 + 等待结束），修复 App 关闭时挂起
- `ecat-middleware` 限流：内存/Redis store 放行语义统一（`>=` → `>`）；超限响应状态码 429
- `ecat-security`：攻击拦截响应按 `to_http_status` 映射（403）
- `ecat-auth` OAuth2：introspect 结果按 cache_ttl 缓存，不再每请求打 introspection
- `ecat-encoding` ProtoCodec：真实 prost encode/decode（原恒返回 Err）
- `ecat-transport`：删除无引用的 Request/Response/Context 死代码
- `ecat-data-redis`：Cache 补齐 increment（INCRBY）/ ttl（TTL）/ multi_get（MGET）
- `ecat-registry-etcd`：注册后后台 keepalive 续约（lease_ttl/3 周期），修复 30s 注册自动失效；deregister 取消续约
- `ecat-registry-consul`：register 附带 HTTP 健康检查（/health，10s 间隔）；discover 路径参数 URL 编码
- `ecat-data-influxdb` / `ecat-data-questdb`：query 增加 HTTP 状态码检查，错误不再静默吞掉
- `ecat-data-nebulagraph`：params 非空返回明确错误（不再静默丢弃）
- `ecat-data-tdengine` / `ecat-data-arangodb`：URL 路径段 percent-encoding
- `ecat-events`：remote 模式真实订阅——后台消费循环按事件类型分发到本地 handler（无回环重复）
- `ecat-graphql`：轻量解析器重写（嵌套字段/括号配对/字符串字面量/指令跳过），失败返回明确错误
- `ecat-bench`：修复请求数整除截断与空样本 p50/p99 越界 panic

### Docs
- README×2 同步 v2.3.3；许可证统一 Apache-2.0（与全部 Cargo.toml 一致）
- README 中间件示例修复（补 CircuitBreakerLayer / SecurityLayer 导入、JWT 密钥 ≥32 字节）
- 数据库表：Memcached 标注 ⚠️ 内存实现（非生产）
- 项目结构树补齐 6 个数据后端 crate
- CLI 快速开始对齐 proto/ 目录实际行为
- Helm `appVersion` 同步 2.3.3
- 支付码图片底部边界扩展并添加水印 https://erik.xyz

## [2.3.2] — 2026-08-07

### Fixed
- `ecat-mq` InMemoryMq：`poll_recv` 改用 `Arc<Notify>` 唤醒（`OwnedNotified`），修复空队列时的忙等自旋
- `ecat-middleware` 限流：默认 key 优先取 `ConnectInfo` 客户端地址，不再信任可伪造的转发头
- `ecat-transport-http`：用户 router 与内置 `/metrics` 路径冲突时捕获 panic 并降级为用户路由（原直接 panic）
- `ecat-data-sqlx`：Blob/BYTEA 列以 base64 字符串返回（原静默变 Null）；NaN/Inf 浮点转为字符串；Any 驱动不支持时间类型，fetch 时报错而非静默（调用方需 CAST 成文本）
- `ecat-data-clickhouse`：`write` 按 measurement 分组改为引用传递，消除全量点克隆
- `ecat-config-remote`：watch 首帧强制推送（兼容缺 X-Consul-Index 服务器）；缺 index 时 1s 退避防紧循环；阻塞查询响应缺失 X-Consul-Index 视为错误
- `ecat-registry-consul`：discover 支持 IPv6 地址（方括号）与 `https` service tag 自动切换 scheme

## [2.3.1] — 2026-08-06

### Fixed
- 端口绑定规范化：`HttpServer` 空 host 统一为 `0.0.0.0`，示例/文档/CLI 模板的监听地址从 `:8000` 改为 `0.0.0.0:8000`（修复无 IPv6 环境启动失败）
- 全部 HTTP 数据库适配器（ES/OpenSearch/ClickHouse/InfluxDB/IoTDB/QuestDB/TDengine/Neo4j/NebulaGraph/ArangoDB）与 TLS 客户端统一设置 connect/timeout，修复请求永久悬挂
- `ecat-data-memcached` 标记为内存实现并明确文档警告，禁止生产误用（静默数据丢失风险）
- TDengine 写入 SQL 拼接转义标识符与字符串值（`"`/`\`），修复注入逃逸
- 限流修复：`key_fn` 支持按请求取客户端 key；Redis 限流区分存储错误（fail-open）；内存桶定期清理防止无界增长
- JWT 最小密钥长度校验（≥32 字节随机密钥）与错误泛化；OAuth2 客户端复用、设置超时并强制 HTTPS
- Redis 凭据改为 `ConnectionInfo` 单独传参，错误消息不再泄露口令；锁 TTL 溢出统一钳制
- Elasticsearch `search`/`delete` 补充 HTTP 状态码检查；index/id 路径 URL 编码（IDOR）
- etcd deregister 修正为按完整注册键删除，修复实例退出后注册信息残留
- GitHub Actions CI 增加 `protobuf-compiler` 安装，与 GitLab CI 对齐（修复 protoc 缺失必然失败）
- Dockerfile 修复：拷贝实际 `ecat` 二进制（原 `ecat-app` 不存在）、安装 curl 以支持 HEALTHCHECK、builder 镜像升至 1.85（edition 2024）
- 其他：Helm appVersion 更新为 2.3.0；配置示例默认口令全部注释化；consul 注册端口从端点解析、discover 版本不再硬编码；MQ `from_config` 签名统一为 async；11 处 Cargo.toml 依赖收敛至 `workspace.dependencies`；`ecat new` 增加 crate 名校验（防路径穿越与注入）；README.en.md 同步至 v2.3.0

## [2.3.0] — 2026-08-06

### Added
- `ecat-mq-kafka` 真 Kafka 实现（rdkafka，替换内存存根）
- 消息后端：`ecat-mq-rabbitmq`（lapin）、`ecat-mq-mqtt`（rumqttc）、`ecat-mq-nats`（async-nats）
- 数据后端：`ecat-data-mongodb`（DocumentClient）、`ecat-data-s3`（StorageClient，rust-s3）、`ecat-data-tdengine`（REST 时序）
- `ecat-lock` 分布式锁 trait + `ecat-data-redis` 的 `RedisLock`（SET NX PX + token 校验）
- `ecat-scheduler` tokio 定时任务调度（every / once）
- `ecat-tracing-otlp` OpenTelemetry OTLP/gRPC 追踪导出
- `ecat-data` trait 扩展：`DocumentClient`、`StorageClient`；`Cache::increment/ttl/multi_get`、`SearchClient::bulk_index/update`、`TsdbClient::delete` 加法默认方法
- `ecat-middleware` 限流后端抽象（`RateLimitStore`）+ `RedisRateLimitStore`（可选 feature）
- CLI：`--version`、`upgrade`（批量更新 ecat-* 依赖）、`run --watch`（notify 文件监听 + 500ms 防抖重启）
- `.gitlab-ci.yml`（镜像 GitHub Actions CI）

### Changed
- Workspace 扩展至 55 crates
- 数据库后端增至 18 个（+MongoDB、S3、TDengine）

## [2.1.8] — 2026-08-01

### Added
- Per-crate `license.workspace` and `description` metadata for crates.io publishing
- Workspace `repository` and `documentation` URLs
- `.gitignore` for Rust project conventions

### Changed
- `EncryptedSource` → `ObfuscatedSource` (honest naming: XOR is obfuscation, not encryption)
- Config prefix `enc:` → `obfs:`
- All `from_config()` methods return `Result` instead of panicking on TLS errors
- `RdbmsError` gains `Config` variant
- `execute_with`/`query_with` default impls return error instead of silently dropping params
- QuestDB client: GET → POST for SQL execution
- Redis TTL: `set_ex` → `pset_ex` for sub-second precision
- `ecat-data-memcached`: `std::sync::Mutex` → `tokio::sync::Mutex`
- `ecat-registry-etcd`: hand-rolled base64 → `base64` crate
- `ecat-client`: `RandomBalancer` uses `RandomState` instead of `Instant::now()` hash
- `ecat-client`: `StaticResolver::add_service` uses `blocking_write` instead of `try_write`

### Fixed
- `ecat-versioning` header-based routing now actually validates version headers
- Credential URL encoding in `connect_with_auth` methods
- Missing `json` feature for reqwest in `ecat-data-influxdb` and `ecat-data-clickhouse`
- Content-Type headers on HTTP requests (InfluxDB, ClickHouse, IoTDB)
- Removed `#[allow(dead_code)]` annotations via field renaming

### Split
- `ecat-auth` (540 lines) → `claims.rs` + `jwt.rs` + `apikey.rs` + `oauth2.rs` + `helpers.rs` + `lib.rs`

## [2.1.7] — 2026-07-29

### Added
- 11 new database backends: ArangoDB, ClickHouse, Elasticsearch, InfluxDB, IoTDB,
  Memcached, NebulaGraph, Neo4j, OpenSearch, QuestDB, Redis
- `ecat-tls` crate for shared TLS configuration
- `ecat-transport-ws` WebSocket server
- `ecat-versioning` API version routing
- `ecat-deploy` Docker/K8s/Helm deployment templates
- `ecat-registry-etcd` backend
- `ecat-mq-kafka` backend

### Changed
- All data backend configs include optional TLS fields
- `ecat-data` trait system: RdbmsClient, Cache, GraphClient, SearchClient, TsdbClient
