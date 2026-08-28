# e-cat 生态规划 v2 — 已完成与后续

**版本:** 2.1.7  
**日期:** 2026-08-01  
**状态:** 全部规划已完成，47 crates

---

## 一、已完成（全部交付）

| 期次 | Crate | 能力 | 测试 |
|------|-------|------|------|
| 一期 | `ecat-health` | 健康检查（/health、/ready） | 4 |
| 一期 | `ecat-client` | HTTP/gRPC 客户端 + 服务发现 + 负载均衡 | 7 |
| 一期 | `ecat-circuit-breaker` | 三态熔断器（Tower Layer） | 4 |
| 一期 | `ecat-auth` | JWT + API Key + OAuth2 认证中间件 | 8 |
| 一期 | `ecat-registry-consul` | Consul 服务注册 | 2 |
| 二期 | `ecat-data-redis` | Redis 缓存（Cache trait） | 1 |
| 二期 | `ecat-mq` | 消息队列抽象 + InMemoryMq | 2 |
| 二期 | `ecat-events` | 本地 + 远程事件总线 | 2 |
| 二期 | `ecat-config-remote` | Consul KV 远程配置 | 2 |
| 三期 | `ecat-testing` | MockServer + ChaosConfig | 5 |
| 三期 | `ecat-openapi` | OpenAPI 3.0 spec 生成 | 2 |
| 三期 | `ecat-bench` | 并发性能基准 | 2 |
| 三期 | `ecat-deploy` | Dockerfile + K8s + Helm | — |
| 四期 | `ecat-tracing` | 分布式追踪（span + trace_id） | 2 |
| 四期 | `ecat-client` 扩展 | GrpcClient + TlsConfig | — |
| 四期 | `ecat-auth` 扩展 | OAuth2Layer | — |
| 五期 | `ecat-registry-etcd` | etcd 服务注册 | 4 |
| 五期 | `ecat-mq-kafka` | Kafka 消息队列 | 1 |
| 五期 | `ecat-data-opensearch` | OpenSearch 搜索 | 1 |
| 五期 | `ecat-data-influxdb` | InfluxDB 时序 | 2 |
| 五期 | `ecat-data-elasticsearch` | Elasticsearch 搜索 | 2 |
| 五期 | `ecat-data-clickhouse` | ClickHouse OLAP | 1 |
| 五期 | `ecat-data-memcached` | Memcached 缓存 | 3 |
| 五期 | `ecat-data-neo4j` | Neo4j 图数据库 | 1 |
| 五期 | `ecat-data-nebulagraph` | NebulaGraph 图数据库 | 1 |
| 五期 | `ecat-data-arangodb` | ArangoDB 图数据库 | 1 |
| 五期 | `ecat-data-iotdb` | IoTDB 时序 | 1 |
| 五期 | `ecat-data-questdb` | QuestDB 时序 | 1 |
| 六期 | `ecat-transport-ws` | WebSocket 支持 | 2 |
| 六期 | `ecat-versioning` | API 版本路由 | 2 |
| 六期 | `ecat-graphql` | GraphQL endpoint | 9 |
| 六期 | CI/CD 模板 | GitHub Actions | — |

---

## 二、剩余缺口（3 项）

| # | 缺口 | 工作量 |
|---|------|--------|
| 1 | **mTLS 接入 transport** | 小 |
| 2 | **Redis 限流后端** | 小 |
| 3 | **GitLab CI 模板** | 小 |

---

## 三、版本路线图

```
v1.0.x  核心骨架（18 crates）                    ✅ 已完成
v2.0.x  生态一期～三期（+13 crates = 31 total）   ✅ 已完成
v2.1.x  通信与安全 + 数据后端 + 运维体验             ✅ 已完成（当前 47 crates）
```

## 四、不纳入生态

| 需求 | 方案 | 理由 |
|------|------|------|
| API 网关 | Kong / Envoy | 语言无关 |
| 服务网格 | Linkerd | Rust 无成熟方案 |
| 容器编排 | Kubernetes | 行业标准 |
| 日志收集 | Vector | Rust 原生 |
