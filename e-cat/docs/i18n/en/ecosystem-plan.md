# e-cat Ecosystem Plan

**Version:** 2.1.7  
**Date:** 2026-08-01  
**Status:** All complete · 47 crates

| Domain | Covered | Status |
|------|--------|------|
| Transport | HTTP (axum), gRPC (tonic), WebSocket | ✅ |
| Encoding | JSON, Protobuf | ✅ |
| Middleware | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth (JWT/API Key/OAuth2) | ✅ |
| Config | env, file (JSON/YAML), Consul KV remote, encryption | ✅ |
| Registry | memory, Consul, etcd | ✅ |
| Security | Attack detection, JWT, API Key, OAuth2, TlsConfig | ✅ |
| Data | RDBMS (sqlx), Redis, Memcached, OpenSearch, Elasticsearch, ClickHouse, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB | ✅ |
| Observability | tracing, Prometheus, Health, distributed tracing | ✅ |
| Communication | HTTP/gRPC Client, MessageQueue (InMemory/Kafka), EventBus | ✅ |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | ✅ |
| API tools | OpenAPI, Versioning, GraphQL | ✅ |

## Remaining Gaps (3 Small Optimizations)

1. **mTLS into transport** — TlsConfig exists but is not wired into HttpServer/GrpcServer
2. **Redis rate-limit backend** — RateLimitLayer is in-memory only; multiple instances need sharing
3. **GitLab CI template** — currently only GitHub Actions

## Version Evolution

```
v1.0.x  Core skeleton (18 crates)                    ✅
v2.0.x  Ecosystem phases 1–3 (+13 crates)            ✅
v2.1.x  Communication & security hardening + data backends + ops experience   ✅ (current)
```

## Excluded from the Ecosystem

| Need | Solution | Reason |
|------|------|------|
| API gateway | Kong / Envoy | Language-agnostic |
| Service mesh | Linkerd | No mature Rust solution |
| Container orchestration | Kubernetes | Industry standard |
| Log collection | Vector | Rust-native |
