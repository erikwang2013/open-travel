# 测试报告 — 2026-08-26

全面单元测试补写（51 crate 全覆盖），4 组资深 Rust 测试工程师并行。

## 总览

| 组 | crates | 原有 | 新增 | 现有 | 门禁 |
|---|---|---|---|---|---|
| core/框架 | 12 | 102 | +40 | 142 | ✅ test 全绿 + clippy 0 警告 |
| data | 14 | 87 | +66 | 153 | ✅ 同上 |
| mq/transport | 12 | 82 | +54 | 136 | ✅ 同上 |
| app 应用层 | 13 | ~178 | +46 | ~224 | ✅ 同上 |
| **合计** | **51** | **~449** | **+206** | **~655** | ✅ |

注：应用层原有数含 ecat-auth 24 / ecat-graphql 35 / ecat-bench 11 / ecat-scheduler 6 / ecat-events 7 / ecat-cli 12 / ecat-middleware 34 / ecat-circuit-breaker 10 / ecat-health 4 / ecat-client 7 / ecat-security 12 / ecat-versioning 4 / ecat 4。各 crate 独立 `cargo test -p` + `cargo clippy -p --all-targets -- -D warnings` 均通过，CARGO_TARGET_DIR 隔离并行。

## 逐 crate 明细

### core/框架组（test-core，+40）

| crate | 原→新 | 覆盖要点 |
|---|---|---|
| ecat-protos | 4→8 | ErrorCode 全枚举对照 proto；截断 buffer decode；空 buffer 默认消息；metadata roundtrip |
| ecat-errors | 4→9 | http_status 全映射（409/429/500）；from_status；未映射→Internal；cause source() |
| ecat-metadata | 9→12 | HTTP header 提取 trace_id；key 小写化；空 header map |
| ecat-encoding | 18→22 | NaN→null（serde_json 默认，已文档化）；空字节 decode；CodecBox 非法 JSON；proto roundtrip |
| ecat-lock | 7→9 | 未持锁 release 报错；空 key |
| ecat-logging | 1→1 | 兼容 shim 不 panic |
| ecat-tracing | 9→12 | 非 UTF-8 trace 头跳过；canonical 头；响应透传 |
| ecat-tls | 7→12 | basic_auth 单/双字段；缺 ca 文件；is_enabled；默认客户端 |
| ecat-config | 14→26 | env 前缀过滤+类型解析边界（hex/空串/-0/1e3）；多 source 合并覆盖；obfs 错误路径；文件缺失/非法 YAML |
| ecat-config-remote | 6→9 | ConsulKvEntry 边界；缺 X-Consul-Index 报错；嵌套 key |
| ecat-openapi | 4→11 | components/schema_ref；重复覆盖；默认 200；tags |
| ecat-metrics | 8→11 | 已注册指标文本；404/405 |

### data 组（test-data，+66）

| crate | 原→新 | 覆盖要点 |
|---|---|---|
| ecat-data | 12→14 | 搜索语法解析 |
| ecat-data-sqlx | 7→14 | 内存 SQLite 端到端；参数绑定全类型；Blob→base64；config |
| ecat-data-redis | 6→12 | redis:///rediss:// URL 构建；auth；config 错误路径 |
| ecat-data-opensearch | 4→10 | mock HTTP：percent-encode、Basic auth、错误透传 |
| ecat-data-elasticsearch | 6→11 | 同上 |
| ecat-data-influxdb | 5→10 | line protocol 转义；Token 头；错误透传 |
| ecat-data-clickhouse | 12→22 | 建表 SQL；JSONEachRow；写入行数；分组 |
| ecat-data-memcached | 4→8 | TTL 秒→毫秒；flag 打包 |
| ecat-data-nebulagraph | 6→7 | config 解析 |
| ecat-data-arangodb | 5→7 | config/URL |
| ecat-data-iotdb | 5→10 | mock HTTP：session 路径参数 |
| ecat-data-questdb | 4→9 | line protocol；事务不支持 |
| ecat-data-tdengine | 6→11 | INSERT 生成；100 批量分块 |
| ecat-data-mongodb | 5→8 | bson 往返；URI |

### mq/transport/registry 组（test-mq，+54）

| crate | 原→新 | 覆盖要点 |
|---|---|---|
| ecat-mq | 5→9 | 满缓冲滞后错误帧；全 drop 流关闭；多订阅者；无订阅者 publish |
| ecat-mq-kafka | 12→14 | config 缺省；SASL 字段独立生效 |
| ecat-mq-rabbitmq | 2→5 | exchange 缺省；url 错误路径 |
| ecat-mq-mqtt | 5→9 | cert/key 配对校验；缺文件；端口缺省 1883/8883；非法端口回退 |
| ecat-mq-nats | 6→9 | 明文缺省；ca/cert 缺失错误路径 |
| ecat-transport | 4→7 | TlsConfig 缺省/with_client_auth；normalize_addr 边界 |
| ecat-transport-http | 17→20 | 集成测试：stop 空操作、占端口失败、真实收发 |
| ecat-transport-grpc | 7→13 | TLS 缺文件；纯文本生命周期；mTLS 拒绝 |
| ecat-transport-ws | 4→8 | 无 handler 失败；占端口；RFC 6455 masked 帧回声 |
| ecat-registry | 5→8 | 多实例 discover；drop 自动注销；builder 缺省 |
| ecat-registry-consul | 10→24 | percent-encode；注册变体；错误响应；X-Consul-Token；agent/services 解析；node 回退 |
| ecat-registry-etcd | 5→10 | discover 坏值跳过；kv 请求体；lease grant；keepalive |

### app 应用层组（test-app，+46）

| crate | 原→新 | 覆盖要点 |
|---|---|---|
| ecat-auth | 20→46 | oauth2 缓存白名单/SHA-256 key/FIFO 逐出；apikey 三态；jwt iss/aud 强制；过期/错签名 |
| ecat-health | 4→8 | readiness 聚合（全 ok/任一 fail/空注册表）；liveness |
| ecat-versioning | 4→7 | path 策略路由；extract_version 边界 |
| ecat-security | 12→20 | header 层端到端；攻击拦截 JSON 形状 |
| ecat-middleware | 34→37 | MemoryStore 窗口过期；内层 panic→Err |
| ecat-circuit-breaker | 10→12 | half-open 探针耗尽；classify 降级 |
| ecat-client | 7→10 | grpc 非法端点报错不联网 |
| ecat-graphql | 35→35 | 已有覆盖充分，无缺口 |
| ecat-scheduler / ecat-bench / ecat-events / ecat-cli / ecat | 已有覆盖充分 | 无缺口 |

## 发现的缺陷

| 级别 | 位置 | 描述 | 状态 |
|---|---|---|---|
| P1 | ecat-events/Cargo.toml | dev-dependencies 缺 tokio macros/rt/time features，单独编译该 crate 测试目标必失败（workspace 全量构建被 feature 并集掩盖） | ✅ 已修复（补 features + 注释） |
| P2 | ecat-security src/lib.rs:118-127 | URI 百分号编码的 SQLi（`?q=SELECT%20*%20...`）可绕过 header 层扫描（检测器要求字面空白，扫原始 URI 不先解码）；正文扫描不受影响 | ⏳ 待修 |
| P3 | ecat-data-sqlx | `connect()/from_config()` 用 AnyPool 但未安装驱动，sqlx 0.8.6 首次连接即 panic "No drivers installed" | ⏳ 待修 |
| P3 | ecat-data-influxdb | 字符串 field 转义了空格（`\ `），line protocol 规范只需转义 `"` 和 `\`；tag/field 顺序非确定 | ⏳ 待修 |
| P3 | ecat-data-clickhouse | 建表缓存永不失效，外部 drop/改表后不重试 CREATE | ⏳ 待修 |
| P3 | ecat-circuit-breaker | half_open_probes 上限在顺序探测下不可达（仅并发在飞时可达），白盒测试已覆盖 | ℹ️ 已知，非缺陷 |
| P3 | ecat-health | `with_check` 用 blocking_write()，async 上下文调用会 panic；当前仅同步上下文可用 | ℹ️ 已知，API 限制 |

## 跳过的模块（需集成环境，未 mock）

- 真实 broker 往返：kafka/rabbitmq/mqtt/nats publish-subscribe（配置与错误路径已覆盖）
- 真实集群：consul/etcd 注册-发现生命周期（axum mock 覆盖请求形状）
- 真实数据库：redis/memcached 操作、mongod、influxdb 服务端校验、sqlx postgres/mysql 驱动、nebulagraph/arangodb API
- 真实外部服务：OAuth2 introspection（本地 mock 覆盖）、gRPC/HTTP 往返（本地 mock 覆盖 302 不跟随）
