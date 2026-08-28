<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat 코드 리뷰 및 TDD 테스트 보고서

**날짜**: 2026-07-29  
**브랜치**: main  
**프로젝트**: e-cat (Rust workspace, 17개 crate)

---

## 1. 리뷰 범위

workspace 전체 17개 crate의 모든 Rust 소스(38개 `.rs` 파일)를 리뷰했습니다.

| Crate | 설명 | 파일 수 |
|-------|------|--------|
| `ecat-protos` | Protobuf 정의와 코드 생성 | 2 |
| `ecat-errors` | 통일 오류 타입 | 2 |
| `ecat-metadata` | 요청 메타데이터 추상화 | 1 |
| `ecat-encoding` | JSON/Protobuf 인코딩·디코딩 | 3 |
| `ecat-logging` | 로그/Tracing 초기화 | 1 |
| `ecat-config` | 설정 로드 (파일/환경 변수) | 3 |
| `ecat-data` | 데이터 계층 trait 추상화 | 5 |
| `ecat-data-sqlx` | SQLx RDBMS 구현 | 1 |
| `ecat-registry` | 서비스 등록·디스커버리 | 2 |
| `ecat-metrics` | Prometheus 메트릭 | 1 |
| `ecat-middleware` | Tower 미들웨어 계층 | 4 |
| `ecat-transport` | 전송 계층 추상화 | 4 |
| `ecat-transport-http` | HTTP/Axum 전송 구현 | 1 |
| `ecat-transport-grpc` | gRPC/Tonic 전송 구현 | 1 |
| `ecat` | 애플리케이션 프레임워크 핵심 | 3 |
| `ecat-cli` | CLI 도구 | 1 |
| `examples/helloworld` | 예시 프로젝트 | 1 |

---

## 2. 발견된 문제 및 수정

### 문제 1: [Clippy] `map_identity` — 무의미한 identity map

- **파일**: `ecat-config/src/file.rs:30`
- **심각도**: 낮음
- **문제**: `map(|(k, v)| (k, v))`는 아무 변환도 하지 않는 무효 코드
- **수정**: 불필요한 `.map()` 호출 제거

### 문제 2: [Clippy] `new_without_default` — Config에 Default 구현 누락

- **파일**: `ecat-config/src/lib.rs:27`
- **심각도**: 낮음
- **문제**: `Config`에 `new()` 메서드가 있지만 `Default` trait이 구현되지 않음
- **수정**: 수동 구현 대신 `#[derive(Default)]` 사용

### 문제 3: [Clippy] `io_other_error` — 구식 Error 생성 방식 사용

- **파일**: `ecat-middleware/src/recovery.rs:42`
- **심각도**: 낮음
- **문제**: `std::io::Error::new(std::io::ErrorKind::Other, ...)`에 더 간결한 대안이 이미 있음
- **수정**: `std::io::Error::other("task panicked")`로 변경

### 문제 4: [Clippy] `redundant_async_block` — 중복 async 블록

- **파일**: `ecat-middleware/src/tracing.rs:38`
- **심각도**: 낮음
- **문제**: `Box::pin(async move { fut.await })`의 async 블록이 불필요
- **수정**: `Box::pin(fut)`로 단순화

### 문제 5: [Clippy] `redundant_closure` — 중복 클로저

- **파일**: `ecat-data-sqlx/src/lib.rs:63`
- **심각도**: 낮음
- **문제**: `.and_then(|f| serde_json::Number::from_f64(f))` 클로저 생략 가능
- **수정**: `.and_then(serde_json::Number::from_f64)` 직접 사용

### 문제 6: [Clippy] `unwrap_or_default` — unwrap_or_default로 단순화 가능

- **파일**: `ecat-transport-http/src/lib.rs:27`
- **심각도**: 낮음
- **문제**: `unwrap_or_else(Router::new)`는 `unwrap_or_default()`와 동일
- **수정**: `unwrap_or_default()`로 변경

---

## 3. 테스트 커버리지 현황

### 수정 전

| Crate | 테스트 수 |
|-------|--------|
| `ecat-errors` | 4 |
| `ecat-transport` | 11 |
| 기타 15개 crate | **0** |
| **합계** | **15** |

### 수정 후

| Crate | 테스트 수 | 추가 | 테스트 내용 |
|-------|--------|------|----------|
| `ecat-encoding` | 15 | +15 | JsonCodec 인코딩·디코딩 왕복, 잘못된 디코딩, content_type; CodecBox 디스패치; codec_from_content_type 정상/오류 경로; Encoding 변형 |
| `ecat-errors` | 4 | — | HTTP 상태 코드 매핑, gRPC 상태 변환, metadata 누적, Display 형식 |
| `ecat-metadata` | 9 | +9 | 키-값 저장, trace_id, From\<HeaderMap\>(비UTF8 값 건너뜀 포함), From\<MetadataMap\>(ASCII 및 바이너리 건너뜀), IntoIterator |
| `ecat-logging` | 1 | +1 | init 스모크 테스트 |
| `ecat-config` | 4 | +4 | 신규/기본값, 타입화 읽기, ConfigSource에서 로드 |
| `ecat-registry` | 5 | +5 | 등록/디스커버리, 등록 해제/삭제, 미존재 오류, 서비스 목록, 이름 필터 |
| `ecat-metrics` | 2 | +2 | 싱글턴 registry, metrics_text panic 없음 |
| `ecat` | 4 | +4 | Builder 기본값, 사용자 지정 이름/버전, server 등록, lifecycle hook |
| `ecat-transport` | 11 | — | Context/Request/Response 생성 및 기본값, Server trait |
| **합계** | **55** | **+40** | |

### 단위 테스트가 필요 없는 crate

- `ecat-protos` — protobuf 코드 생성만 함
- `ecat-data` — 순수 trait 정의, 구현 로직 없음
- `ecat-data-sqlx` — 데이터베이스 연결 필요, 통합 테스트 범주
- `ecat-middleware` — Tower Service 구현, 통합 테스트 필요
- `ecat-transport-http` / `ecat-transport-grpc` — 네트워크 리슨 필요, 통합 테스트 범주
- `ecat-cli` — 출력만 함, 로직 없음

---

## 4. 검증 결과

```
cargo test   → 55 passed, 0 failed
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
```

---

## 5. 수정 파일 목록

| 파일 | 변경 |
|------|------|
| `ecat-config/src/file.rs` | identity map 제거 |
| `ecat-config/src/lib.rs` | `#[derive(Default)]` + 4개 테스트 |
| `ecat-data-sqlx/src/lib.rs` | 중복 클로저 단순화 |
| `ecat-middleware/src/recovery.rs` | `std::io::Error::other()` 사용 |
| `ecat-middleware/src/tracing.rs` | 중복 async 블록 제거 |
| `ecat-transport-http/src/lib.rs` | `unwrap_or_else` → `unwrap_or_default` |
| `ecat-metrics/src/lib.rs` | 2개 테스트 |
| `ecat-registry/src/memory.rs` | 5개 테스트 |
| `ecat/src/lib.rs` | 4개 테스트 |
