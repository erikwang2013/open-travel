# 专项审计报告（安全与性能）— 2026-08-14

审计范围：55 crate workspace（v2.3.5）。方法：Cargo.lock 人工核查（cargo-audit 未安装）、认证/TLS 路径源码审计、并发与资源生命周期检查。未提交代码。

## 依赖 CVE 核查

- 核心依赖版本均较新且无已知未修复 CVE：rustls 0.23.43、ring 0.17.14、aws-lc-rs 1.17.3、jsonwebtoken 9.3.1、tokio 1.53.1、h2 0.4.15、quinn 0.11.11、sqlx 0.8.6、zerocopy 0.8.55、time 0.3.54、openssl 0.10.81。
- hyper 0.14.32（仅来自 rust-s3 0.35.1，经 hyper-tls 0.5）已高于 0.14.28 修复线。
- 注意：CI 未安装 cargo-audit，建议加入工作流自动化核查。

## 发现（按严重度排序）

### S1 [中] HTTP TLS 握手串行化 → 慢握手 DoS
- 位置：`ecat-transport-http/src/lib.rs:134-150`（TlsListener::accept）
- 现象：TLS 握手在 `accept()` 内同步完成，axum::serve 串行调用 accept——一个不完成握手的连接会阻塞整个 accept 循环。
- 影响：攻击者批量建立慢速/僵尸 TCP 连接即可使服务完全停止接受新连接（gRPC 侧 tonic 对每连接 spawn 握手，不受影响）。
- 建议：accept 后 `tokio::spawn` 握手并加 `tokio::time::timeout(10s)`，失败即关闭连接。

### S2 [中] OAuth2 内省缓存无界增长 → 内存 DoS
- 位置：`ecat-auth/src/oauth2.rs:45,84-92`
- 现象：`HashMap<String,(String,Instant)>` 以 token 为键，TTL 仅控制新鲜度，无容量上限、无驱逐。
- 影响：海量唯一 token 请求可无限增长内存（每次 miss 还会触发上游 introspection）。
- 建议：加容量上限（如 10k）+ 定期清理，或换 moka/LRU 带容量与 TTL 驱逐。

### S3 [低-中] ecat-data-s3 使用旧版 rust-s3 0.35.1（hyper 0.14 + native-tls/openssl）
- 位置：`ecat-data-s3/Cargo.toml` → rust-s3 0.35.1
- 现象：S3 客户端独立使用 hyper-tls/openssl 栈，ecat-tls::TlsClientConfig（自定义 CA、客户端证书、skip_verify）对 S3 无效；TLS 配置面不一致。
- 影响：企业环境 S3 私有 CA/mTLS 无法配置；依赖 2023 年后维护缓慢。
- 建议：评估升级 rust-s3 或改用统一 reqwest/rustls 客户端。

### S4 [低] JWT 默认校验不含 iss/aud
- 位置：`ecat-auth/src/jwt.rs:125` — `Validation::new(HS256)` 仅签名+exp。
- 影响：HS256 共享密钥下，一个服务的 token 可被另一服务接受（无发行方隔离）。
- 建议：文档明确要求生产配置 issuer/audience；或默认加 iss 校验入口。

### S5 [低] TlsClientConfig.skip_verify 单独即令 is_enabled() 为真
- 位置：`ecat-tls/src/lib.rs:23-29`
- 现象：仅配 `skip_verify: true` 时 TLS 视为"启用"且不校验证书，静默关闭验证。
- 建议：skip_verify 与 ca_cert 互斥校验，或 require 显式双重确认。

## 性能与资源

### P1 [低] OAuth2 缓存命中路径每请求 JSON 反序列化
- 位置：`ecat-auth/src/oauth2.rs:87` — 缓存存序列化字符串，命中后仍 `serde_json::from_str`。
- 建议：缓存直接存 `AuthClaims` 结构体，省掉每请求 parse。

### P2 [低] ecat-bench 无预热与稳态判断
- 位置：`ecat-bench/src/lib.rs:run_bench` — 直接计时，无 warmup，冷启动/连接池首次分配混入 p99。
- 建议：加预热轮与稳态收敛判断，结果更可信。

### P3 [低] Kafka 消费者 100ms poll + 100ms sleep 串行
- 位置：`ecat-mq-kafka/src/lib.rs:84-92` — 消息端到端延迟上限约 200ms。
- 建议：poll 后无需再 sleep；低吞吐场景可缩短 poll 间隔。

## 良好实践确认

- 生产路径无 unwrap/expect panic（transport/auth/middleware 仅测试中）。
- API key 查询参数回退带泄漏警告日志；HashMap 使用 SipHash 防碰撞。
- SQL 层透传调用方 SQL（框架性质），连接串 user:pass 百分号编码正确。
- Kafka 消费通道满时阻塞背压而非丢弃；rx drop 后 poll 任务正常退出。
- config-remote 拉取带超时（5s/30s），阻塞查询缺 index 报错防忙等。

---

## 核心域正确性审计（补充，与上述安全/性能专项互补）

审计方法：全 workspace 生产代码扫描（unwrap/expect/panic 定位、静默吞错、异步停止、并发状态）+ `cargo test --workspace` 全量复验（首轮全绿；S1 修复进行中导致 transport-http 中途编译告警，收尾后需复跑）。未提交代码。

### N1 [中] ecat-events 消费任务退出后 handle 泄漏 → 事件静默丢失
- 位置：`ecat-events/src/lib.rs:97-101`（消费循环 89-95 行 `None => break`）
- 现象：mq stream 返回 None（如 kafka broadcast channel 关闭）或任务 panic 时消费循环退出，但 `consumers` map 中 JoinHandle 残留；之后同事件类型再 `subscribe()` 因 68 行 `contains_key` 恒真而不再重启消费任务 → 该类型事件永久静默丢失。
- 影响：远端事件流中断后无法自愈，恢复需重启进程。
- 建议：任务退出路径从 map 移除 handle（spawn watcher 或 `handle.is_finished()` 惰性清理）。

### N2 [中] ecat-mq-kafka subscribe 的 group_id 语义错误
- 位置：`ecat-mq-kafka/src/lib.rs:71-84`
- a. `group_id` 默认 None 时 rdkafka `consumer.subscribe()` 要求 group.id（librdkafka 报 INVALID_ARG），默认配置下订阅大概率直接失败（需真机验证）。
- b. 配置了 group_id 时（ecat-events 每事件类型各 subscribe 一次、同 group），Kafka 在同组多消费者间按分区瓜分 topic → 某事件类型可能落到其他类型的消费任务被静默丢弃（auto.offset.reset=latest 且不提交）。
- 影响：事件总线在 kafka 后端下丢事件。
- 建议：无 group_id 时生成随机唯一 group.id；或消费端用 assign() 显式分配分区；文档明确多订阅须独立 group。

### N3 [低] GrpcServer/WsServer 空 host 未规范化（D1 修复不完整）
- 位置：`ecat-transport-grpc/src/lib.rs:52`、`ecat-transport-ws/src/lib.rs:58`
- 现象：`GrpcServer::new(":8000")` 的 `addr.parse::<SocketAddr>()` 返回 AddrParseError（已实测验证）；WsServer `TcpListener::bind(":8000")` 解析到 IPv6 通配，无 IPv6 环境启动失败。HttpServer 已做 0.0.0.0 规范化，三个 server API 行为不一致。
- 建议：统一在 new 内规范化空 host。

### N4 [低] TracingLayer 未注入 trace_id，与 CHANGELOG 2.3.3 声明不符
- 位置：`ecat-tracing/src/lib.rs:72-84`（span 仅含 service 字段，代码注释自认泛型 Req 无法取头）；`inject_trace_id()` 每次生成新 UUID，不沿用上游 extract 的 trace_id。
- 影响：按文档配置的分布式追踪无法跨服务关联。
- 建议：span 字段延迟绑定或特化 http::Request<B>；inject 支持携带上游 id。

### N5 [低] ecat-scheduler job panic 静默停摆
- 位置：`ecat-scheduler/src/lib.rs:53-57,83`（`run()` 中 `let _ = handle.await`）
- 现象：定时任务 panic 后任务死亡，无重启、无日志；`run()` 丢弃 JoinHandle 错误。
- 建议：捕获 panic 打日志 + 可选重启策略。

### N6 [低] 生产代码残留 unwrap（中毒/panic 路径）
- `ecat-events/src/lib.rs:68,98` std `Mutex::lock().unwrap()`（中毒即 panic）；`ecat-versioning/src/lib.rs:86` Response builder unwrap（不可失败但属 panic 路径）；`ecat-mq/src/lib.rs:110` expect 已由 is_none 守卫（安全）。
- 建议：events 两处改 `unwrap_or_else(|e| e.into_inner())`。

### N7 [信息] WsServer::stop() 不等待已升级的 WebSocket 连接
- 位置：`ecat-transport-ws/src/lib.rs:63-87`
- axum on_upgrade 连接在独立任务运行，graceful shutdown 不覆盖；长连接 handler 在 stop() 后仍滞留，进程退出不干净（App::stop 语义不完整）。

### N8 [信息] 零测试 crate：ecat-data / ecat-lock / ecat-protos
- 均为 trait/定义型 crate；已验证默认方法 fail-loud（返回错误而非静默），但 trait 契约（Transaction drop 回滚语义、锁 token 校验）无任何单测。
- 建议：为 RdbmsError/Transaction 与 DistributedLock 语义补最小单测。

### N9 [信息] graphql 参数与嵌套字段仍被丢弃
- `ecat-graphql/src/lib.rs` execute 仅传 `variables` 给 resolver，`{ hello(name: "x") }` 的字段参数、嵌套 selection 均不传递；README 未注明该限制（旧报告 L8 要求文档化，2.3.3 重写后仍未补）。

### N10 [信息] circuit-breaker 仅统计传输层错误
- `ecat-circuit-breaker/src/lib.rs:203-209` 只把 inner Err 记为失败，HTTP 5xx 视为成功 → 熔断对服务不可用（5xx 风暴）无效；文档未说明。

**验证状态**：首轮 `cargo test --workspace` 全绿（含 doc-tests，尾部输出未见任何失败）；S1 修复 agent 编辑期间 transport-http 曾现编译错误与 2 处告警（unused import `ensure_crypto_provider`、`shutdown_tx` 未读）——属中间态，S1 收尾后需全量复跑测试与 `clippy --all-targets -D warnings`。

---

## 第三轮：动态验证 + CVE 复查 + panic 面（专项，2026-08-14）

### CVE 复查（新增发现，按严重度）

1. **[中] rustls-webpki 0.102.8 残留在依赖树**（RUSTSEC-2026-0049/0098/0099/0104：CRL distributionPoint 绕过、URI/wildcard name-constraints，修复版 0.103.10）。主链为 0.103.13（经 rustls 0.23.43，安全）；0.102.8 经 async-nats 0.38.0 / rumqttc 0.25.1 引入，覆盖 NATS/MQTT TLS 客户端链。上游未迁移 rustls 0.23，无修复版本——受控风险，建议注释跟踪。
2. **[中-低] rdkafka 0.36.2 内嵌 librdkafka 携带 cJSON 1.7.14**（CVE-2023-53154 及 cJSON 系列；CVE-2025-57052 标 CVSS 9.8 但受影响文件 cJSON_utils.c 未被 librdkafka 使用，适用性存疑）。上游修复在 librdkafka 2.10+（2026-03 PR #5346）。ecat-mq-kafka 静态链接，需核对 librdkafka-sys 打包版本并跟踪升级。
3. **[低] rustls-pemfile 2.2.0 未维护**（RUSTSEC-2025-0134）— ecat-transport-http 启动期解析本地文件，非攻击者输入。
4. **[低] rsa 0.9.10**（RUSTSEC-2023-0071 Marvin 计时侧信道）— 经 sqlx-mysql TLS 引入，仅 MySQL + RSA 密钥交换场景相关。
5. async-nats 0.38.0 已高于 RUSTSEC-2023-0027（CN 校验绕过）修复线，无问题。

### 动态验证（examples/helloworld，debug 构建，临时端口 18080，已清理）

- /health 200、/（JSON 序列化）200（27B）、404 正常；Logging 中间件正常记录请求。
- **/metrics 挂载但返回 200 + 空 body（0 字节）**：无指标注册时无任何输出，监控侧无法区分"健康/无指标"。建议空 registry 输出注释行或 503。
- 畸形请求（头含 0x01/0x02）→ 400 Bad Request，服务存活、后续 /health 仍 200，无 panic。
- TLS/mTLS 路径与熔断/限流中间件：由 ecat-transport-http/grpc、ecat-middleware 测试覆盖（mTLS 竞态修复后全绿，拒绝匿名/错误客户端证书用例通过）。

### bench 基线

- ecat-bench 无 [[bench]]/bin 目标，无 cargo bench 入口；run_bench_with_warmup 已带预热（P2 修复落地），harness 测试全绿。
- 实测为 debug 构建 smoke：/ 约 1.3ms、/health 约 1.8ms（含 curl 进程开销，无基线意义）。建议 release 构建 + wrk/hey 压测出真实基线。

### panic 面复查（全 workspace，排除测试模块）

- 共 31 处 unwrap/expect/panic，均低风险：Response::builder().body().unwrap()（jwt/apikey/oauth2 不可失败分支）、锁中毒兜底（etcd/testing）、clickhouse serde_json::to_string().unwrap()（极端 NaN/inf 输入理论 panic）。
- **1 处需留意**：`ecat-transport-http/src/tls_listener.rs:234` — 后台 accept 循环异常退出时 `accept()` 内 panic!，服务线程死亡（触发条件苛刻：仅监听器致命错误），建议降级为错误返回并打日志。
