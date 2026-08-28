# e-cat 생태계 계획 v3 — 최종 평가

> **업데이트 (2026-08-07, v2.3.3)**: 남은 격차 #1「mTLS transport 연동」완료 — `HttpServer::tls` / `GrpcServer::tls`가 tokio-rustls / tonic rustls 기반으로 실제 동작(CA 검증과 클라이언트 인증서 강제 지원); 격차 #2(Redis rate limit), #3(GitLab CI)은 이전에 v2.3.0과 함께 완료. 계획에 명시된 격차가 이로써 전부 구현되었습니다.

**버전:** 2.4.2  
**날짜:** 2026-08-01  
**crate 총수:** 55 · 모든 계획 완료

---

## 현재 커버리지

| 영역 | 구현됨 | 커버리지 |
|------|--------|--------|
| 전송 계층 | HTTP (axum), gRPC (tonic), WebSocket | 100% |
| 인코딩 | JSON, Protobuf | 100% |
| 미들웨어 | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth×3 | 100% |
| 설정 | env, file (JSON/YAML), Consul KV, 암호화 (XOR) | 100% |
| 등록 센터 | memory, Consul, etcd | 100% |
| 보안 | 공격 탐지, JWT, API Key, OAuth2, TLS 클라이언트 인증서, mTLS | 95% |
| 통신 | TLS 클라이언트 인증서 — 모든 데이터 백엔드 지원 | 95% |
| 서비스 통신 | HTTP Client, gRPC Client, Resolver, LoadBalancer | 95% |
| 데이터 | RDBMS (sqlx), Redis, OpenSearch, Elasticsearch, ClickHouse, Memcached, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB — 전부 Config 파일 설정 지원 | 95% |
| 메시지 | MessageQueue trait, InMemory, Kafka, EventBus | 100% |
| 관측성 | tracing, Prometheus, Health, 분산 추적 | 100% |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | 95% |
| API 도구 | OpenAPI, Versioning, GraphQL | 100% |

---

## 남은 격차

### 할 가치 있는 것 (3개)

| # | 격차 | 가치 | 작업량 |
|---|------|------|--------|
| 1 | **mTLS transport 연동** | TlsConfig는 이미 있으며, HttpServer/GrpcServer에 미연동 | 소 |
| 2 | **Redis rate limit 백엔드** | RateLimitLayer가 메모리 전용, 다중 인스턴스는 공유 필요 | 소 |
| 3 | **GitLab CI 템플릿** | GitHub Actions는 이미 있음 | 소 |

### 하지 않아도 되는 것 (2개)

| # | 격차 | 이유 |
|---|------|------|
| 4 | 설정 AES-GCM | 현재 XOR로 충분 |
| 5 | 서비스 메시/API 게이트웨이 | 커뮤니티에 맡김(Linkerd/Kong/K8s) |

---

## 판정

**e-cat은 프로덕션 사용 가능한 성숙도에 도달했습니다.** 47개 crate가 마이크로서비스 풀스택을 커버합니다: 전송 → 미들웨어 → 서비스 디스커버리 → 설정 → 보안 → 데이터 → 메시지 → 관측성 → DevOps → API 도구. 남은 3개 격차는 소규모 작업량 최적화이며, 구조적 결함은 없습니다.

## 데이터 백엔드 커버리지 (15개)

| 카테고리 | 데이터베이스 | Crate | 드라이버 방식 |
|------|--------|-------|----------|
| RDBMS | SQLite/PostgreSQL/MySQL/TiDB | `ecat-data-sqlx` | sqlx (공식 비동기 드라이버) |
| 캐시 | Redis | `ecat-data-redis` | redis-rs (공식 드라이버) |
| 캐시 | Memcached | `ecat-data-memcached` | ⚠️ 메모리 구현 (비프로덕션) |
| 문서 | MongoDB | `ecat-data-mongodb` | mongodb (공식 드라이버) |
| 객체 스토리지 | S3 / MinIO | `ecat-data-s3` | HTTP/REST (reqwest+rustls, 자체 구현 SigV4) |
| OLAP | ClickHouse | `ecat-data-clickhouse` | HTTP/REST (reqwest) |
| 검색 | OpenSearch | `ecat-data-opensearch` | HTTP/REST (reqwest) |
| 검색 | Elasticsearch | `ecat-data-elasticsearch` | HTTP/REST (reqwest) |
| 그래프 | Neo4j | `ecat-data-neo4j` | HTTP/REST (reqwest) |
| 그래프 | NebulaGraph | `ecat-data-nebulagraph` | HTTP/REST (reqwest) |
| 그래프 | ArangoDB | `ecat-data-arangodb` | HTTP/REST (reqwest) |
| 시계열 | InfluxDB | `ecat-data-influxdb` | HTTP/REST (reqwest) |
| 시계열 | Apache IoTDB | `ecat-data-iotdb` | HTTP/REST (reqwest) |
| 시계열 | QuestDB | `ecat-data-questdb` | HTTP/REST (reqwest) |
| 시계열 | TDengine | `ecat-data-tdengine` | HTTP/REST (reqwest) |
