# e-cat 生态规划

**版本:** 2.1.7  
**日期:** 2026-08-01  
**状态:** 全部完成 · 47 crates

| 领域 | 已覆盖 | 状态 |
|------|--------|------|
| 传输层 | HTTP (axum), gRPC (tonic), WebSocket | ✅ |
| 编码 | JSON, Protobuf | ✅ |
| 中间件 | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth (JWT/API Key/OAuth2) | ✅ |
| 配置 | env, file (JSON/YAML), Consul KV 远程, 加密 | ✅ |
| 注册 | memory, Consul, etcd | ✅ |
| 安全 | 攻击检测, JWT, API Key, OAuth2, TlsConfig | ✅ |
| 数据 | RDBMS (sqlx), Redis, Memcached, OpenSearch, Elasticsearch, ClickHouse, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB | ✅ |
| 可观测 | tracing, Prometheus, Health, 分布式追踪 | ✅ |
| 通信 | HTTP/gRPC Client, MessageQueue (InMemory/Kafka), EventBus | ✅ |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | ✅ |
| API 工具 | OpenAPI, Versioning, GraphQL | ✅ |

## 剩余缺口（3 项小优化）

1. **mTLS 接入 transport** — TlsConfig 已有，未接入 HttpServer/GrpcServer
2. **Redis 限流后端** — RateLimitLayer 仅内存，多实例需共享
3. **GitLab CI 模板** — 当前仅 GitHub Actions

## 版本演进

```
v1.0.x  核心骨架（18 crates）                    ✅
v2.0.x  生态一期～三期（+13 crates）              ✅
v2.1.x  通信与安全强化 + 数据后端补齐 + 运维体验   ✅ (当前)
```

## 不纳入生态

| 需求 | 方案 | 理由 |
|------|------|------|
| API 网关 | Kong / Envoy | 语言无关 |
| 服务网格 | Linkerd | Rust 无成熟方案 |
| 容器编排 | Kubernetes | 行业标准 |
| 日志收集 | Vector | Rust 原生 |
