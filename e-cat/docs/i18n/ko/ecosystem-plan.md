# e-cat 생태계 계획

**버전:** 2.1.7  
**날짜:** 2026-08-01  
**상태:** 전부 완료 · 47 crates

| 영역 | 커버됨 | 상태 |
|------|--------|------|
| 전송 계층 | HTTP (axum), gRPC (tonic), WebSocket | ✅ |
| 인코딩 | JSON, Protobuf | ✅ |
| 미들웨어 | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth (JWT/API Key/OAuth2) | ✅ |
| 설정 | env, file (JSON/YAML), Consul KV 원격, 암호화 | ✅ |
| 등록 | memory, Consul, etcd | ✅ |
| 보안 | 공격 탐지, JWT, API Key, OAuth2, TlsConfig | ✅ |
| 데이터 | RDBMS (sqlx), Redis, Memcached, OpenSearch, Elasticsearch, ClickHouse, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB | ✅ |
| 관측성 | tracing, Prometheus, Health, 분산 추적 | ✅ |
| 통신 | HTTP/gRPC Client, MessageQueue (InMemory/Kafka), EventBus | ✅ |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | ✅ |
| API 도구 | OpenAPI, Versioning, GraphQL | ✅ |

## 남은 격차 (3개 소규모 최적화)

1. **mTLS transport 연동** — TlsConfig는 이미 있으며, HttpServer/GrpcServer에 미연동
2. **Redis rate limit 백엔드** — RateLimitLayer가 메모리 전용, 다중 인스턴스는 공유 필요
3. **GitLab CI 템플릿** — 현재 GitHub Actions만 있음

## 버전 진화

```
v1.0.x  핵심 골격 (18 crates)                    ✅
v2.0.x  생태계 1기~3기 (+13 crates)              ✅
v2.1.x  통신·보안 강화 + 데이터 백엔드 보완 + 운영 경험   ✅ (현재)
```

## 생태계에 포함하지 않는 것

| 요구사항 | 방안 | 이유 |
|------|------|------|
| API 게이트웨이 | Kong / Envoy | 언어 무관 |
| 서비스 메시 | Linkerd | Rust에 성숙한 방안 없음 |
| 컨테이너 오케스트레이션 | Kubernetes | 업계 표준 |
| 로그 수집 | Vector | Rust 네이티브 |
