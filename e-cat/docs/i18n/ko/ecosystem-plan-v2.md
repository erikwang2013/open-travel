# e-cat 생태계 계획 v2 — 완료와 후속

**버전:** 2.1.7  
**날짜:** 2026-08-01  
**상태:** 모든 계획 완료, 47 crates

---

## 1. 완료 (전부 납품)

| 기수 | Crate | 기능 | 테스트 |
|------|-------|------|------|
| 1기 | `ecat-health` | 헬스 체크(/health, /ready) | 4 |
| 1기 | `ecat-client` | HTTP/gRPC 클라이언트 + 서비스 디스커버리 + 로드 밸런싱 | 7 |
| 1기 | `ecat-circuit-breaker` | 3상태 회로 차단기 (Tower Layer) | 4 |
| 1기 | `ecat-auth` | JWT + API Key + OAuth2 인증 미들웨어 | 8 |
| 1기 | `ecat-registry-consul` | Consul 서비스 등록 | 2 |
| 2기 | `ecat-data-redis` | Redis 캐시 (Cache trait) | 1 |
| 2기 | `ecat-mq` | 메시지 큐 추상화 + InMemoryMq | 2 |
| 2기 | `ecat-events` | 로컬 + 원격 이벤트 버스 | 2 |
| 2기 | `ecat-config-remote` | Consul KV 원격 설정 | 2 |
| 3기 | `ecat-testing` | MockServer + ChaosConfig | 5 |
| 3기 | `ecat-openapi` | OpenAPI 3.0 spec 생성 | 2 |
| 3기 | `ecat-bench` | 동시성 성능 벤치마크 | 2 |
| 3기 | `ecat-deploy` | Dockerfile + K8s + Helm | — |
| 4기 | `ecat-tracing` | 분산 추적 (span + trace_id) | 2 |
| 4기 | `ecat-client` 확장 | GrpcClient + TlsConfig | — |
| 4기 | `ecat-auth` 확장 | OAuth2Layer | — |
| 5기 | `ecat-registry-etcd` | etcd 서비스 등록 | 4 |
| 5기 | `ecat-mq-kafka` | Kafka 메시지 큐 | 1 |
| 5기 | `ecat-data-opensearch` | OpenSearch 검색 | 1 |
| 5기 | `ecat-data-influxdb` | InfluxDB 시계열 | 2 |
| 5기 | `ecat-data-elasticsearch` | Elasticsearch 검색 | 2 |
| 5기 | `ecat-data-clickhouse` | ClickHouse OLAP | 1 |
| 5기 | `ecat-data-memcached` | Memcached 캐시 | 3 |
| 5기 | `ecat-data-neo4j` | Neo4j 그래프 데이터베이스 | 1 |
| 5기 | `ecat-data-nebulagraph` | NebulaGraph 그래프 데이터베이스 | 1 |
| 5기 | `ecat-data-arangodb` | ArangoDB 그래프 데이터베이스 | 1 |
| 5기 | `ecat-data-iotdb` | IoTDB 시계열 | 1 |
| 5기 | `ecat-data-questdb` | QuestDB 시계열 | 1 |
| 6기 | `ecat-transport-ws` | WebSocket 지원 | 2 |
| 6기 | `ecat-versioning` | API 버전 라우팅 | 2 |
| 6기 | `ecat-graphql` | GraphQL endpoint | 9 |
| 6기 | CI/CD 템플릿 | GitHub Actions | — |

---

## 2. 남은 격차 (3개)

| # | 격차 | 작업량 |
|---|------|--------|
| 1 | **mTLS transport 연동** | 소 |
| 2 | **Redis rate limit 백엔드** | 소 |
| 3 | **GitLab CI 템플릿** | 소 |

---

## 3. 버전 로드맵

```
v1.0.x  핵심 골격 (18 crates)                    ✅ 완료
v2.0.x  생태계 1기~3기 (+13 crates = 31 total)   ✅ 완료
v2.1.x  통신·보안 + 데이터 백엔드 + 운영 경험             ✅ 완료 (현재 47 crates)
```

## 4. 생태계에 포함하지 않는 것

| 요구사항 | 방안 | 이유 |
|------|------|------|
| API 게이트웨이 | Kong / Envoy | 언어 무관 |
| 서비스 메시 | Linkerd | Rust에 성숙한 방안 없음 |
| 컨테이너 오케스트레이션 | Kubernetes | 업계 표준 |
| 로그 수집 | Vector | Rust 네이티브 |
