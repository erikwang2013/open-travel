# e-cat 프레임워크 감사 보고서 R2 — 2026-08-01

**버전**: 1.0.5
**범위**: 전체 18개 서브 crate
**결론**: `cargo check` / `cargo clippy --all-features` / `cargo test` 모두 통과, 70 tests ✅

---

## 1. 지난 수정 회고 (16/16 수정됨)

지난 감사(R1)에서 발견한 문제가 모두 수정되었습니다: SecurityLayer 공격 차단, ProtoCodec prost 지원, Server graceful shutdown, JoinHandle 수집, Transaction 구현, Registration Drop 안전 감지, 컬럼 타입 매핑 강화, CLI new 파일 생성, 버전/edition 통일, FileSource 오류 처리, Context 메타데이터 메서드, discover Arc 최적화, query columns Arc 최적화, RateLimitLayer 신규.

---

## 2. 이번 차수 새로 발견된 문제

### 2.1 [심각] CLI `new`가 생성한 템플릿 코드가 컴파일 불가

- **파일**: `ecat-cli/src/main.rs:79-97`
- **문제**: 생성된 `Cargo.toml`이 `workspace = true` 의존성 참조와 `path = "../ecat"` 상대 경로를 사용하지만, `ecat new myapp`으로 만든 독립 프로젝트는 e-cat workspace 안에 없어 모든 참조가 해석 실패
- **영향**: `ecat new`로 만든 프로젝트가 아예 컴파일 불가
- **수정**: 템플릿은 workspace 참조가 아닌 버전이 있는 실제 의존성을 사용해야 함

```toml
# 현재 (오류):
tokio.workspace = true           # 프로젝트가 workspace에 없어 오류
ecat = { path = "../ecat" }      # 상대 경로 무효

# 변경:
tokio = { version = "1", features = ["full"] }
ecat = "1.0.5"
```

### 2.2 [심각] ecat-data-sqlx `transaction()`이 실제 DB 트랜잭션 핸들러를 폐기

- **파일**: `ecat-data-sqlx/src/lib.rs:100-106`
- **문제**: `pool.begin()`은 실제 DB 트랜잭션 핸들러 `Transaction<'_, DB>`를 반환하지만, 코드가 `_tx`로 바인딩한 후 즉시 폐기. `_tx`가 drop되면 DB 트랜잭션이 자동 롤백. 반환된 `ecat_data::Transaction`은 빈 껍데기이며, `commit()/rollback()` 메서드가 아무 효과 없음
- **영향**: `transaction()`을 사용하는 모든 코드가 트랜잭션 보호 없이 실행되어 데이터 일관성 보장 불가
- **수정**: `ecat_data::Transaction` 구조체를 재설계하여 실제 DB 트랜잭션 핸들러를 보유해야 함

### 2.3 [중간] SecurityLayer가 요청 본문을 스캔하지 않음

- **파일**: `ecat-security/src/lib.rs:117-127`
- **문제**: `call()`이 URI와 HTTP 헤더만 스캔하고 요청 본문은 전혀 확인하지 않음. 공격자가 SQL 인젝션/XSS payload를 POST body에 넣어 손쉽게 탐지를 우회할 수 있음
- **영향**: 공격 탐지의 유효 커버리지가 크게 낮아짐
- **수정**: body 스캔 기능 추가, 또는 `scan_body()` 공개 메서드를 제공하여 호출자가 body 읽기 후 사용

### 2.4 [중간] RateLimitLayer가 동기 Mutex + 만료 정리 없음

- **파일**: `ecat-middleware/src/ratelimit.rs:10-38`
- **문제 1**: `std::sync::Mutex`를 async 컨텍스트에서 사용 — 락 경쟁 시 전체 tokio worker 스레드 블로킹
- **문제 2**: `buckets: HashMap<String, (u32, Instant)>`가 만료 키를 정리하지 않아 장기 실행 서버의 메모리가 무한 증가(새 IP/key마다 영구 점유)
- **영향**: 고동시성에서 성능 저하, 장시간 실행 후 메모리 누수
- **수정**: `tokio::sync::Mutex`로 변경하고, `allow()`에서 만료 항목을 주기적으로 정리

### 2.5 [중간] ecat-data-sqlx의 원시 SQL에 파라미터화 API 없음

- **파일**: `ecat-data-sqlx/src/lib.rs:24-29, 32-36`
- **문제**: `execute(&self, sql: &str)`과 `query(&self, sql: &str)`이 원시 SQL 문자열만 받고, trait 레벨에 파라미터 바인딩 메서드가 없음. 호출자가 사용자 입력을 SQL에 이어붙이면 SQL 인젝션 발생
- **영향**: trait 자체가 직접 보안 취약점을 노출하지는 않지만, 파라미터화 API 부재가 호출자가 안전하지 않은 코드를 작성하도록 유도
- **제안**: `RdbmsClient` trait에 `execute_with`와 `query_with` 메서드를 추가해 파라미터 바인딩 사용

### 2.6 [낮음] query()에서 Arc::clone이 여전히 클로저 내부에 있음

- **파일**: `ecat-data-sqlx/src/lib.rs:50-53`
- **문제**: `let cols = std::sync::Arc::clone(&columns)`이 `rows.iter().map()` 클로저 내부에서 실행. Arc::clone은 가볍지만(원자 참조 카운트 증가), 클로저 밖으로 옮기면 행마다 원자 연산을 피할 수 있음
- **제안**: `iter()` 전에 클론 한 번 수행하고, 클로저가 해당 클론을 캡처

### 2.7 [낮음] ProtoCodec의 trait impl과 새 API 불일치

- **파일**: `ecat-encoding/src/proto.rs`
- **문제**: `Codec` trait의 `encode/decode`가 여전히 오류만 반환; 새로 추가된 `encode_message/decode_message`는 올바른 경로지만 메서드 이름이 trait과 불일치. 사용자가 `codec.encode()`를 먼저 시도하고 왜 실패하는지 혼란스러워할 수 있음
- **제안**: 문서/주석에서 설명: proto 타입은 Codec trait 메서드가 아닌 `encode_message/decode_message`를 사용해야 함

---

## 3. 현재 상태 개요

| 차원 | 상태 |
|------|------|
| `cargo check` | ✅ warning 0 |
| `cargo clippy --all-features` | ✅ 경고 0 |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 통과 |
| 버전 통일 | ✅ 1.0.5 |
| Edition 통일 | ✅ 2024 |

### 테스트 분포

| Crate | Tests | 설명 |
|-------|-------|------|
| ecat | 4 | ✅ |
| ecat-config | 9 | ✅ |
| ecat-encoding | 15 | ✅ |
| ecat-errors | 4 | ✅ |
| ecat-logging | 1 | ✅ |
| ecat-metadata | 9 | ✅ |
| ecat-metrics | 2 | ✅ |
| ecat-middleware | 4 | ✅ (RateLimitLayer 포함) |
| ecat-registry | 5 | ✅ |
| ecat-security | 6 | ✅ |
| ecat-transport | 11 | ✅ |
| ecat-data | 0 | — (순수 trait 정의) |
| ecat-data-sqlx | 0 | ⚠️ DB 통합 테스트 없음 |
| ecat-protos | 0 | — (생성 코드) |
| ecat-transport-grpc | 0 | ⚠️ |
| ecat-transport-http | 0 | ⚠️ |
| ecat-cli | 0 | ⚠️ |

---

## 4. 문제 우선순위

| # | 심각도 | 문제 | 파일 | 사용자 영향 |
|---|--------|------|------|----------|
| 1 | 🔴 | CLI `new` 템플릿이 컴파일 불가 코드 생성 | `ecat-cli/src/main.rs:79` | 신규 사용자의 첫 명령이 실패 |
| 2 | 🔴 | transaction()이 실제 DB 트랜잭션 핸들러 폐기 | `ecat-data-sqlx/src/lib.rs:100` | 데이터 일관성 보장 없음 |
| 3 | 🟠 | SecurityLayer가 body 미스캔 | `ecat-security/src/lib.rs:117` | 공격자가 탐지 우회 가능 |
| 4 | 🟠 | RateLimitLayer std Mutex + 메모리 누수 | `ecat-middleware/src/ratelimit.rs:10,25` | 동시성 성능 + OOM |
| 5 | 🟠 | 원시 SQL에 파라미터화 API 없음 | `ecat-data-sqlx/src/lib.rs:24` | SQL 인젝션 위험 |
| 6 | 🟡 | query() Arc clone 위치 | `ecat-data-sqlx/src/lib.rs:53` | 미세 성능 최적화 |
| 7 | 🟡 | ProtoCodec API 불일치 | `ecat-encoding/src/proto.rs` | 사용자 혼란 |

---

## 6. 수정 기록 (2026-08-01 R2)

| # | 문제 | 수정 방식 | 상태 |
|---|------|----------|------|
| 1 | CLI new 템플릿 컴파일 불가 | 버전화 의존성으로 변경 (`ecat = "1.0"`, `tokio = "1"` 등) | ✅ |
| 2 | transaction() DB 트랜잭션 폐기 | `Transaction::with_inner()`가 실제 핸들러 보유, sqlx가 `Box<dyn Any>`로 전달 | ✅ |
| 3 | SecurityLayer body 미스캔 | `scan_body(&[u8])` 공개 메서드 신규 | ✅ |
| 4 | RateLimitLayer Mutex + 누수 | `tokio::sync::Mutex` + 100개 키마다 만료 항목 정리 | ✅ |
| 5 | 원시 SQL 파라미터화 API 없음 | `RdbmsClient`에 `execute_with`/`query_with` 파라미터화 메서드 신규 | ✅ |
| 6 | query() Arc clone 위치 | `Arc::clone`을 `iter()` 밖으로 이동, 모든 행이 참조 공유 | ✅ |
| 7 | ProtoCodec API 불일치 | 모듈 레벨 문서 + struct 문서로 사용 방식 설명 | ✅ |

### 최종 상태

| 검사 항목 | 결과 |
|--------|------|
| `cargo check` | ✅ error 0 / warning 0 |
| `cargo clippy --all-features` | ✅ warning 0 |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 통과 |
| 버전 | 1.0.5 (전부 workspace 상속 통일) |
| Edition | 2024 |
