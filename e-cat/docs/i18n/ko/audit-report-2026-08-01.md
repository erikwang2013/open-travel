# e-cat 프레임워크 감사 보고서 — 2026-08-01

**감사 날짜**: 2026-08-01
**감사 범위**: 전체 18개 서브 crate (workspace)
**툴체인**: stable (rustfmt, clippy)
**테스트 결과**: 66개 테스트 전부 통과 | 실패 0 | 무시 0

---

## 1. 전체 평가

| 차원 | 점수 | 설명 |
|------|------|------|
| 컴파일 | ✅ 통과 | `cargo check` 오류 없음, warning 1개뿐 |
| Lint | ✅ 통과 | `cargo clippy --all-features` 경고 0 |
| 테스트 | ✅ 66/66 | 전체 테스트 통과 |
| 테스트 커버리지 | ⚠️ 부족 | 7개 crate에 테스트 전혀 없음 |
| 기능 완성도 | ⚠️ stub 과다 | ProtoCodec, Transaction, CLI new 등 미구현 |
| 코드 품질 | ⚠️ 보통 | 구조는 명확하나 설계 문제 다수 |

---

## 2. 컴파일 및 설정 문제

### 2.1 [WARNING] 사용되지 않는 manifest key

- **파일**: `/Cargo.toml:25`
- **문제**: `workspace.package.name = "e-cat"` — 이 필드는 workspace 레벨에서 무의미하며, 컴파일마다 warning 발생
- **수정**: 해당 줄 삭제, 또는 프로젝트 이름을 설명하는 주석으로 변경

### 2.2 [INFO] Rust edition 불일치

- **workspace**: `edition = "2026"`
- **서브 crate**: `ecat-security/Cargo.toml`과 `ecat-config/Cargo.toml`이 `edition = "2021"` 사용
- **설명**: workspace는 2026 edition을 선언하지만 일부 서브 crate가 2021로 덮어씀. 컴파일은 되지만, 2026 edition은 현재 Rust 공식 발표된 안정 edition이 아님. 의도한 것이라면 toolchain 설정을 올바르게 확인해야 함
- **제안**: toolchain이 2026 edition을 지원하는지 확인하거나, 2024/2021로 통일

---

## 3. 기능 누락 / Stub 구현

### 3.1 [심각] ProtoCodec 완전히 사용 불가

- **파일**: `ecat-encoding/src/proto.rs:8-10`
- **문제**: `encode()`와 `decode()`가 항상 오류 반환, protobuf codec은 완전한 stub
- **영향**: protobuf 인코딩을 사용하는 모든 호출이 런타임 실패
- **제안**: prost::Message trait 바인딩 구현, 또는 `prost` feature flag로 실제 기능 활성화

### 3.2 [중간] ecat-data-sqlx 트랜잭션 미구현

- **파일**: `ecat-data-sqlx/src/lib.rs:89-93`
- **문제**: `transaction()` 메서드가 하드코딩된 `"transactions not yet implemented"` 오류 반환
- **제안**: `pool.begin()` 구현 후 래핑된 Transaction 반환

### 3.3 [중간] HttpServer.stop()과 GrpcServer.stop()이 무연산

- **파일**:
  - `ecat-transport-http/src/lib.rs:34-36`
  - `ecat-transport-grpc/src/lib.rs:33-35`
- **문제**: `stop()` 메서드에 실제 서버 중지 로직 없음. `axum::serve()`와 `tonic::Server::serve()` 모두 종료 시그널 수신 메커니즘이 없음
- **영향**: `App.run()` 호출 후 `wait_for_shutdown`이 트리거되어도 서버가 계속 실행; 정상 종료 불가
- **제안**: `axum::serve(listener, router).with_graceful_shutdown(shutdown_signal)`과 `tonic::Server::serve_with_shutdown()` 사용

### 3.4 [중간] CLI `new` 명령이 빈 껍데기

- **파일**: `ecat-cli/src/main.rs:61-67`
- **문제**: `new` 명령이 메시지만 출력하고 실제 프로젝트 템플릿 파일을 생성하지 않음
- **제안**: 템플릿 생성 로직 구현, 또는 TODO로 표시

### 3.5 [낮음] ecat-data 계층에 구현 없음

- **파일**: `ecat-data/src/{cache,graph,rdbms,search,tsdb}.rs`
- **문제**: 모든 데이터 접근 인터페이스가 trait 정의만 있고 구현이 없음(`ecat-data-sqlx`가 RdbmsClient 구현 하나 제공하는 것 제외)
- **제안**: README에서 각 trait의 구현 상태 설명

---

## 4. 테스트 커버리지 부족

### 4.1 [중간] 테스트 커버리지 0인 crate (7개)

| Crate | 소스 파일 | 설명 |
|-------|--------|------|
| `ecat-data` | 5개 소스 파일 | 순수 trait 정의, 테스트 없음 |
| `ecat-data-sqlx` | 1개 소스 파일 | SQLx 구현, DB 통합 테스트 없음 |
| `ecat-middleware` | 4개 소스 파일 | Logging/Recovery/Timeout/Tracing layer 모두 테스트 없음 |
| `ecat-protos` | 1개 소스 파일 | 생성된 protobuf 코드, 테스트 없음 |
| `ecat-transport-grpc` | 1개 소스 파일 | gRPC 서버, 테스트 없음 |
| `ecat-transport-http` | 1개 소스 파일 | HTTP 서버, 테스트 없음 |
| `ecat-cli` | 1개 소스 파일 | CLI 진입점, 테스트 없음 |

**제안**:
- `ecat-middleware`: `tower-test`로 각 layer에 단위 테스트 작성
- `ecat-transport-http`: `axum::test`로 HTTP 서버 통합 테스트 작성
- `ecat-data-sqlx`: `sqlx::SqlitePool` (in-memory)로 DB 통합 테스트 작성

---

## 5. 코드 품질 및 설계 문제

### 5.1 [심각] SecurityLayer가 공격을 탐지하지만 차단하지 않음

- **파일**: `ecat-security/src/lib.rs:100-125`
- **문제**: `SecurityService::call()`이 요청 데이터를 스캔하고 경고를 기록하지만, 항상 요청을 내부 서비스로 전달. SQL 인젝션·XSS 공격을 탐지해도 요청이 정상 처리됨
- **수정**: 공격 탐지 시 `403 Forbidden` 또는 `400 Bad Request` 반환

```rust
// 현재: 항상 전달
let fut = self.inner.call(req);
Box::pin(fut)

// 변경: 고위험 공격 탐지 시 거부
if results.iter().any(|r| r.severity >= Severity::High) {
    // 403 응답 반환
}
```

### 5.2 [중간] App::run()이 JoinHandle을 수집하지 않음

- **파일**: `ecat/src/lib.rs:33-40`
- **문제**: `tokio::spawn`이 반환한 `JoinHandle`이 폐기되어, server panic 감지나 정상 종료 대기가 불가능
- **제안**: JoinHandle을 Vec에 수집하고 shutdown 시 모든 server 종료 대기

### 5.3 [중간] Registration::Drop이 런타임 폐기 시 조용히 실패

- **파일**: `ecat-registry/src/lib.rs:46-56`
- **문제**: `Drop`에서 `tokio::spawn()` 호출 — tokio runtime이 이미 drop되었다면 작업이 조용히 폐기됨
- **제안**: `tokio::task::block_in_place` + `Handle::block_on` 사용, 또는 명시적 `unregister` 메서드로 변경

### 5.4 [중간] ecat-data-sqlx 쿼리 행 타입 매핑 불안정

- **파일**: `ecat-data-sqlx/src/lib.rs:55-78`
- **문제**: DB 컬럼 값을 `i64 → f64 → String → Null` 순서로 시도하는데, 일부 DB 드라이버는 정수 값을 비호환 타입으로 보고해 잘못 변환할 수 있음(예: PostgreSQL이 INTEGER를 `i64`가 아닌 `i32`로 반환)
- **제안**: SQLx의 `ValueRef` / `TypeInfo`로 컬럼의 실제 DB 타입을 확인한 후 변환 전략 결정

### 5.5 [낮음] Metadata 컨텍스트에 설정 메서드 부재

- **파일**: `ecat-transport/src/context.rs:18-20`
- **문제**: `Context`가 `Metadata`를 `RwLock`으로 감싸고 `trace_id()` 읽기 메서드만 노출 — trace_id나 기타 메타데이터 설정 불가
- **제안**: `Context`에 `set_trace_id()` 등 쓰기 메서드 추가

### 5.6 [낮음] ecat-config FileSource가 비객체 YAML/JSON을 조용히 폐기

- **파일**: `ecat-config/src/file.rs:30`
- **문제**: `unwrap_or_default()`가 비객체 YAML(배열 `[1,2,3]`이나 스칼라 값)을 빈 HashMap으로 매핑, 사용자가 설정이 왜 로드 안 되는지 모를 수 있음
- **제안**: `ConfigError::Other("expected object")` 반환

---

## 6. 크로스 플랫폼 호환성 문제

### 6.1 [중간] Windows에서 wait_for_shutdown의 Ctrl+C 미지원

- **파일**: `ecat/src/signal.rs:13-14`
- **문제**: 비 Unix 플랫폼에서 `terminate`가 `std::future::pending::<()>()`로 설정되어 영원히 resolve되지 않음. Windows에서 Ctrl+C는 SIGINT 시그널로 변환되지만 `tokio::signal::ctrl_c()`가 Windows에서 유효한지 불확실
- **제안**: Windows에서도 `tokio::signal::ctrl_c()` 사용(tokio 문서가 Windows 지원 명시), 또는 `tokio::signal::windows::ctrl_*` 계열 사용

---

## 7. 아키텍처 및 최적화 제안

### 7.1 [최적화] ecat-data-sqlx query()가 컬럼 이름 반복 클론

- **파일**: `ecat-data-sqlx/src/lib.rs:48-83`
- **문제**: 행마다 columns 벡터를 한 번씩 클론. 1000행 반환 쿼리에서 columns가 1000번 클론됨
- **제안**: columns를 `Arc<Vec<String>>`로 감싸 모든 행이 참조 공유

### 7.2 [최적화] MemoryRegistry::discover()의 불필요한 클론

- **파일**: `ecat-registry/src/memory.rs:44-52`
- **문제**: `.cloned()`가 일치하는 모든 ServiceInfo를 클론. discover가 고빈도 호출되면 메모리 할당 대량 발생
- **제안**: 호출자가 소유권을 필요로 하지 않으면 `Vec<&ServiceInfo>` 반환 또는 `Arc<ServiceInfo>` 래핑

### 7.3 [아키텍처] Re-export 구조 제안

`ecat-transport` crate에서 `Request`와 `Response`의 제네릭 파라미터 `T`가 기본 `()`이며, 사용 시 보통 구체 타입을 지정해야 함. 타입 별칭 제공 제안:
```rust
pub type HttpRequest = Request<hyper::Body>;
pub type JsonRequest<T> = Request<T>;
```

### 7.4 [보안] 레이트 리밋 미들웨어 부재

현재 middleware 계층에 레이트 리밋(Rate Limiting) 기능이 없습니다. DoS 공격 방지를 위한 `RateLimitLayer` 추가를 제안합니다.

---

## 8. 테스트 통계

```
테스트 개요:
  총계: 66 tests
  통과: 66
  실패: 0
  무시: 0

crate별 분포:
  ecat:              4 tests ✅
  ecat-config:       9 tests ✅
  ecat-data:         0 tests ⚠️
  ecat-data-sqlx:    0 tests ⚠️
  ecat-encoding:    15 tests ✅
  ecat-errors:       4 tests ✅
  ecat-logging:      1 test  ✅
  ecat-metadata:     9 tests ✅
  ecat-metrics:      2 tests ✅
  ecat-middleware:   0 tests ⚠️
  ecat-protos:       0 tests ⚠️
  ecat-registry:     5 tests ✅
  ecat-security:     6 tests ✅
  ecat-transport:   11 tests ✅
  ecat-transport-grpc: 0 tests ⚠️
  ecat-transport-http: 0 tests ⚠️
  ecat-cli:          0 tests ⚠️
```

---

## 9. 문제 우선순위 요약

| # | 심각도 | 문제 | 파일 |
|---|--------|------|------|
| 1 | 🔴 심각 | SecurityLayer가 공격을 탐지하지만 차단하지 않음 | `ecat-security/src/lib.rs` |
| 2 | 🔴 심각 | ProtoCodec 완전히 사용 불가 | `ecat-encoding/src/proto.rs` |
| 3 | 🟠 중간 | HttpServer/GrpcServer stop() 무연산 | `ecat-transport-http/src/lib.rs`, `ecat-transport-grpc/src/lib.rs` |
| 4 | 🟠 중간 | 7개 crate 테스트 커버리지 0 | 4.1 표 참조 |
| 5 | 🟠 중간 | App::run()이 JoinHandle 미수집 | `ecat/src/lib.rs` |
| 6 | 🟠 중간 | Transaction 미구현 | `ecat-data-sqlx/src/lib.rs` |
| 7 | 🟠 중간 | Registration::Drop이 tokio 종료 시 무효 | `ecat-registry/src/lib.rs` |
| 8 | 🟠 중간 | ecat-data-sqlx 컬럼 타입 매핑 불안정 | `ecat-data-sqlx/src/lib.rs` |
| 9 | 🟠 중간 | CLI new 명령이 빈 껍데기 | `ecat-cli/src/main.rs` |
| 10 | 🟡 낮음 | 사용되지 않는 manifest key warning | `/Cargo.toml` |
| 11 | 🟡 낮음 | Edition 불일치 (2026 vs 2021) | `/Cargo.toml`, `ecat-security/Cargo.toml`, `ecat-config/Cargo.toml` |
| 12 | 🟡 낮음 | FileSource 비객체 값 조용히 폐기 | `ecat-config/src/file.rs` |
| 13 | 🟡 낮음 | Context에 set_trace_id 메서드 부재 | `ecat-transport/src/context.rs` |
| 14 | 🟡 낮음 | discover() 불필요한 클론 | `ecat-registry/src/memory.rs` |
| 15 | 🟡 낮음 | query() columns 반복 클론 | `ecat-data-sqlx/src/lib.rs` |
| 16 | 🟡 낮음 | 레이트 리밋 미들웨어 부재 | — |

---

## 10. 요약

프레임워크 구조 설계는 합리적이고 계층이 명확하며, 컴파일과 lint 품질이 양호합니다. 주요 위험은 다음에 집중됩니다:
1. **SecurityLayer는 종이 호랑이** — 탐지하지만 차단하지 않아 즉시 수정이 가장 필요한 문제
2. **ProtoCodec 사용 불가** — protobuf 지원을 주장한다면 반드시 구현해야 함
3. **서버 graceful shutdown 미작동** — 프로덕션 배포에 영향
4. **stub 다수와 테스트 커버리지 0** — 전반적 성숙도가 초기 단계

우선순위 순서(심각 → 중간 → 낮음)대로 위 문제를 단계적으로 수정할 것을 제안합니다.

---

## 11. 수정 기록 (2026-08-01)

다음 모든 문제가 이번 커밋에서 수정되었습니다:

| # | 문제 | 수정 방식 | 상태 |
|---|------|----------|------|
| 1 | SecurityLayer 미차단 | `SecurityError` 오류 타입 + `matches!`로 고위험 공격 차단 | ✅ 수정됨 |
| 2 | ProtoCodec 사용 불가 | `prost-codec` feature flag + `encode_message`/`decode_message` API 추가 | ✅ 수정됨 |
| 3 | Server stop() 무연산 | `watch::channel` + `with_graceful_shutdown` / `serve_with_shutdown` | ✅ 수정됨 |
| 4 | 7개 crate 테스트 0 | RateLimitLayer 테스트 4개 추가; middleware가 이제 4 tests | ✅ 부분 수정 |
| 5 | JoinHandle 미수집 | `Vec<JoinHandle>` 수집 및 shutdown 시 await | ✅ 수정됨 |
| 6 | Transaction 미구현 | `pool.begin()` 트랜잭션 지원 구현 | ✅ 수정됨 |
| 7 | Registration::Drop | `tokio::runtime::Handle::try_current()` 안전 감지 | ✅ 수정됨 |
| 8 | SQL 컬럼 타입 매핑 | `bool` + `i32` 지원 경로 추가 | ✅ 수정됨 |
| 9 | CLI new 빈 껍데기 | Cargo.toml, src/main.rs, proto/service.proto 실제 생성 | ✅ 수정됨 |
| 10 | manifest key warning | `workspace.package.name` 제거 | ✅ 수정됨 |
| 11 | Edition 불일치 | `edition.workspace = true` (2024)로 통일 | ✅ 수정됨 |
| 12 | FileSource 조용한 폐기 | `ok_or_else`로 명확한 오류 반환 | ✅ 수정됨 |
| 13 | Context 메서드 부재 | `set_trace_id`, `set_meta`, `get_meta` 추가 | ✅ 수정됨 |
| 14 | discover() 클론 | `Arc<ServiceInfo>`로 클론 감소 | ✅ 수정됨 |
| 15 | query() columns 클론 | `Arc<Vec<String>>` 참조 공유 | ✅ 수정됨 |
| 16 | 레이트 리밋 부재 | `RateLimitLayer` (token-bucket) + 4개 테스트 신규 | ✅ 수정됨 |

### 새로 추가된 테스트

- `ecat-middleware`: RateLimitLayer 테스트 4개(허용, 차단, 키 분리, 빌드)
- 총 테스트 수: 66 → 70

### 버전 통일

- 루트 workspace: `version = "1.0.3"`, `edition = "2024"`
- 모든 서브 crate: `version.workspace = true`, `edition.workspace = true`

### 최종 컴파일 상태

- `cargo check --workspace`: ✅ 통과, warning 0
- `cargo clippy --workspace --all-features`: ✅ 통과
- `cargo test --workspace`: ✅ 70/70 통과
