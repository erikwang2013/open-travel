<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat API 참조

이 페이지는 Ecat 프레임워크의 인터페이스(API) 면을 요약합니다: 포트 규약, 내장 엔드포인트, 오류 형식과 확장 인터페이스. 비즈니스 라우팅은 각 서비스가 직접 등록합니다.

## 포트 규약

| 프로토콜 | 리슨 주소 | 설명 |
|------|----------|------|
| HTTP | `0.0.0.0:8000` | axum 라우팅, 기본 예시 포트 |
| gRPC | `0.0.0.0:9000` | tonic Server, 기본 예시 포트 |

## 내장 엔드포인트

다음 엔드포인트는 생태계 crate가 제공하며, 서비스에 함께 마운트됩니다:

| 엔드포인트 | 출처 | 설명 |
|------|------|------|
| `/health` | ecat-health | 생존 확인(서비스 이름, 버전, 시작 시간 반환) |
| `/ready` | ecat-health | 준비 확인(의존성 준비 후 200 반환) |
| `/metrics` | ecat-metrics | Prometheus 메트릭 노출(`ecat_http_requests_total` / `ecat_http_request_duration_seconds`) |
| `/{service}/{method}` | 사용자 라우팅 | 예시: `/helloworld/ecat` |

> 메트릭 엔드포인트 경로에 ID 등 고기수(高基数) 시나리오는 `MetricsLayer::new().with_path_fn(...)`으로 정규화하여 메트릭 카디널리티 폭발을 방지하세요.

## 요청 처리 흐름

```
클라이언트 요청
  ├─ HTTP :8000 ──→ axum::Router ─┐
  └─ gRPC :9000 ──→ tonic::Server ─┤
                              ┌─────┴──────┐
                              │ Middleware │  Recovery→Tracing→Logging→Auth→Metrics→Security→CircuitBreaker
                              └─────┬──────┘
                                    ▼
                               Handler（tower::Service）
                                    ▼
                               Response（JSON/Protobuf 인코딩）
```

## 오류 형식

`ecat-errors`가 `ErrorCode` + `Error`를 제공하며, 컴파일 타임에 HTTP 상태 코드를 매핑합니다:

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

오류 응답은 middleware를 통해 JSON(또는 Protobuf)으로 인코딩되며, code / reason / message를 담습니다.

## 확장 인터페이스

| 기능 | Crate | 인터페이스 |
|------|-------|------|
| GraphQL | ecat-graphql | `/graphql` 엔드포인트; 필드 파라미터와 중첩 selection 지원, 별칭·fragment·다중 최상위 필드 미지원 |
| OpenAPI | ecat-openapi | 라우팅에서 OpenAPI spec 생성 |
| WebSocket | ecat-transport-ws | 업그레이드된 WS 전송 |
| API 버전 라우팅 | ecat-versioning | `/v1/...` 접두사 버전 라우팅 |
| 인증 | ecat-auth | JWT / API Key 미들웨어; JWT 키는 ≥32바이트 필요, 체이닝 `required_issuer`/`required_audience` |
| gRPC 클라이언트 | ecat-transport-grpc | 서비스 디스커버리·로드 밸런싱 통합 |

## 서비스 간 통신

- `HttpClient`(ecat-client): 서비스 디스커버리·로드 밸런싱 통합, CircuitBreaker 회로 차단 보호
- `GrpcClient`(ecat-transport-grpc): 동일, gRPC 프로토콜
- 미들웨어는 `tower::ServiceBuilder`로 통합 조합(Recovery / Tracing / Logging / Timeout / RateLimit / Security / CircuitBreaker / Metrics / Retry / Validate / CORS)

## 데이터 백엔드 인터페이스

모든 데이터 백엔드(`ecat-data-*`)는 통일된 trait(`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`)으로 추상화됩니다; REST 계열 백엔드(Neo4j / NebulaGraph / ArangoDB / InfluxDB / IoTDB / QuestDB / TDengine / OpenSearch / Elasticsearch / S3)는 `base_url` 기반으로 해당 HTTP 인터페이스에 접근합니다. 연결 설정은 [데이터베이스 설정 튜토리얼](database-config-tutorial.md)을 참조하세요.
