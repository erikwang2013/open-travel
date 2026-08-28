# e-cat 코드 리뷰 보고서 — 2026-08-01 (4차 · 전부 수정)

**프로젝트 버전:** 2.1.0  
**최종 상태:** 0 warnings, ~116 tests, clippy clean, fmt clean

**5차 정리:** 미사용 의존성 12개 제거 (ecat-health/reqwest, ecat-circuit-breaker/tokio, ecat-bench/tracing, ecat-mq/serde+serde_json, ecat-events/async-trait, ecat-config-remote/tracing, ecat-testing/transport-http+axum, ecat-client/serde+serde_json)
**리뷰 범위:** 전체 18개 crate

## 최종 상태

| 도구 | 상태 |
|------|------|
| `cargo build` | 통과 (0 warnings) |
| `cargo test` | 77 passed, 0 failed, 1 ignored |
| `cargo clippy` | 통과 (0 warnings) |
| `cargo fmt` | 통과 |

---

## 수정 목록 (전체)

### 중간 위험

1. **[수정됨]** `Mutex::lock().unwrap()` → `ecat-transport-http/lib.rs`, `ecat-transport-grpc/lib.rs`
2. **[수정됨]** CLI `fs::write().unwrap()` → `ecat-cli/src/main.rs`

### 낮은 위험

3. **[수정됨]** ProtoCodec doc-test → `ecat-encoding/src/proto.rs`
4. **[수정됨]** 단위 테스트 0 crate → transport-http/grpc에 각각 3개 테스트 추가
5. **[수정됨]** `Transaction::commit()` 무연산 → `TransactionInner` trait 신규
6. **[수정됨]** `SecurityScanner::new()` 주석 수정
7. **[수정됨]** 미사용 `opentelemetry` 의존성 → `ecat-logging` 및 workspace 루트 Cargo.toml
8. **[수정됨]** Doc-test 형식

### 최적화

9. **[수정됨]** `scan_parts` 사전 할당 → `Vec::with_capacity`
10. **[수정됨]** `serde_yaml` 0.9 폐기 → `yaml_serde` 0.10으로 마이그레이션
11. **[수정됨]** `Transaction::commit()` 더 이상 무연산 아님 → `SqlxTransactionWrapper`로 실제 commit/rollback 구현

### 수정 불필요 (설계 결정)

- **`ecat` crate 추가 의존성** — 의도된「meta crate」패턴, 하위 프로젝트에 편리한 전이 의존성 제공
- **ProtoCodec Codec trait 오류 반환** — serde와 prost::Message의 근본적 타입 차이, `encode_message()`/`decode_message()` 분리 API와 명확한 문서로 설명됨
- **`ecat-data` 구체 구현 없음** — trait 인터페이스 설계, 구현은 `ecat-data-sqlx`에 있음

---

## 변경 파일 요약

| 파일 | 변경 |
|------|------|
| `ecat-transport-http/src/lib.rs` | Mutex 포이즌 방어 + 테스트 3개 추가 |
| `ecat-transport-grpc/src/lib.rs` | Mutex 포이즌 방어 + 테스트 3개 추가 |
| `ecat-cli/src/main.rs` | 통일 오류 처리 |
| `ecat-security/src/lib.rs` | 주석 수정 + 사전 할당 최적화 |
| `ecat-logging/Cargo.toml` | 미사용 opentelemetry 제거 |
| `ecat-encoding/src/proto.rs` | doc-test 개선 |
| `ecat-data/src/lib.rs` | TransactionInner 내보내기 |
| `ecat-data/src/rdbms.rs` | TransactionInner trait 신규 |
| `ecat-data-sqlx/src/lib.rs` | SqlxTransactionWrapper가 TransactionInner 구현 |
| `ecat-config/Cargo.toml` | serde_yaml → yaml_serde |
| `ecat-config/src/file.rs` | serde_yaml → yaml_serde |
| `Cargo.toml` | 고아 opentelemetry workspace 의존성 제거 |
| `README.md` | 버전 번호 업데이트, 관측성 설명 수정, 생태계 계획 링크 추가 |
| `docs/ecosystem-plan.md` | 생태계 계획 문서 신규 (3기 15개 crate) |
