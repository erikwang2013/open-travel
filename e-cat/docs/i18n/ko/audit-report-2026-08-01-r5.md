# E-CAT 감사 보고서 — r5

**날짜**: 2026-08-01  
**브랜치**: main  
**버전**: 2.1.7  
**Crate 수**: 47 (workspace members)
**상태**: ✅ 수정 가능한 모든 문제 해결 + 데이터 백엔드 전면 설정 파일 지원

---

## 0. 수정 기록 (2026-08-01)

| # | 문제 | 파일 | 수정 |
|---|------|------|------|
| 1 | unused import `axum::routing::get` | `ecat-versioning/src/lib.rs:3` | 최상위 import 제거, `#[cfg(test)]` 내부로 이동 |
| 2 | unused variable `version` | `ecat-versioning/src/lib.rs:61` | `_version`으로 변경 |
| 3 | dead code `extract_version` | `ecat-versioning/src/lib.rs:68` | `pub fn`으로 변경 |
| 4 | `useless_format!` | `ecat-versioning/src/lib.rs:62` | `"/api"` 직접 사용으로 변경 |
| 5 | `unnecessary_to_owned` | `ecat-data-questdb/src/lib.rs:39` | `"true".to_string()` → `"true"` |
| 6 | 오류 메시지 삼킴 | `ecat-data-questdb/src/lib.rs:30` | `unwrap_or_default()` → `unwrap_or_else(...)` |
| 7 | `derivable_impls` | `ecat-client/src/lib.rs:249` | `GrpcClientBuilder`를 `#[derive(Default)]`로 변경 |
| 8 | `manual_is_multiple_of` | `ecat-config/src/encrypted.rs:60` | `s.len() % 2 != 0` → `!s.len().is_multiple_of(2)` |
| 9 | `collapsible_if` | `ecat-registry-etcd/src/lib.rs:92` | 중첩 `if let` 병합 |
| 10 | `collapsible_if` | `ecat-data-clickhouse/src/lib.rs:56` | 중첩 `if let` 병합 |
| 11 | `type_complexity` | `ecat-data-memcached/src/lib.rs:9` | `type CacheEntry` 별칭 추가 |

**최종 결과**: `cargo build` warning 0, `cargo clippy --all-targets` warning 0, `cargo test` 전부 통과 (실패 0).

### 12 ─ 데이터 백엔드 전면 설정 파일 지원 (Cargo + lib.rs)

12개 데이터 백엔드 crate에 `Config` 구조체(`#[derive(Deserialize)]`)와 `from_config()` 생성자를 추가하여, JSON/YAML 설정 파일에서 연결 정보를 로드할 수 있게 했습니다. 하드코딩 불필요.

| Crate | Config 구조체 | 필드 |
|-------|--------------|------|
| `ecat-data-redis` | `RedisConfig` | `url` |
| `ecat-data-sqlx` | `SqlxConfig` | `url` |
| `ecat-data-clickhouse` | `ClickhouseConfig` | `base_url`, `database`(기본 "default") |
| `ecat-data-questdb` | `QuestdbConfig` | `base_url` |
| `ecat-data-elasticsearch` | `ElasticsearchConfig` | `base_url` |
| `ecat-data-opensearch` | `OpenSearchConfig` | `base_url` |
| `ecat-data-influxdb` | `InfluxConfig` | `base_url`, `org`, `bucket`, `token` |
| `ecat-data-memcached` | `MemcachedConfig` | (빈 값 — 메모리 구현) |
| `ecat-data-neo4j` | `Neo4jConfig` | `base_url`, `username`, `password` |
| `ecat-data-nebulagraph` | `NebulaGraphConfig` | `base_url`, `space` |
| `ecat-data-arangodb` | `ArangoConfig` | `base_url`, `db`, `username`, `password` |
| `ecat-data-iotdb` | `IotdbConfig` | `base_url`, `username`, `password` |

**사용 예시**:
```rust
// YAML 설정 파일에서 로드
let cfg: ClickhouseConfig = serde_json::from_str(r#"{"base_url":"http://localhost:8123"}"#)?;
let client = ClickhouseClient::from_config(cfg);
```

### 13 ─ HTTP 백엔드에 선택적 인증 지원 (5개 crate)

5개 순수 HTTP 백엔드에 선택적 `username` / `password` 필드와 `with_auth()` 생성자를 추가했습니다. 전부 `Option<String>`(`#[serde(default)]`)이며, 설정하지 않으면 인증 없음.

| Crate | 추가된 Config 필드 | 추가된 생성자 |
|-------|-----------------|-------------|
| `ecat-data-elasticsearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-opensearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-clickhouse` | `username?`, `password?` | `with_auth()` |
| `ecat-data-questdb` | `username?`, `password?` | `with_auth()` |
| `ecat-data-nebulagraph` | `username?`, `password?` | `with_auth()` |

모든 HTTP 요청은 `apply_auth()` 헬퍼 메서드로 Basic Auth를 자동 부착합니다(둘 다 None이 아닐 때만).

### 14 ─ Redis / RDBMS / Memcached에 선택적 인증 필드 추가 (3개 crate)

| Crate | 추가된 Config 필드 | 추가된 생성자 | 인증 방식 |
|-------|-----------------|-------------|----------|
| `ecat-data-redis` | `password?` | `connect_with_password()` | URL 내장 비밀번호 |
| `ecat-data-sqlx` | `username?`, `password?` | `connect_with_auth()` | URL 내장 인증 |
| `ecat-data-memcached` | `username?`, `password?` | `with_auth()` | 예약 필드 (메모리 구현) |

Sqlx는 SQLite / PostgreSQL / MySQL / TiDB 4종 RDBMS를 커버합니다. Auth 필드는 `replacen("://", "://user:pass@")`로 연결 URL에 내장되며, URL에 `@`가 없을 때만 적용됩니다.

### 15 ─ TLS 인증서 인증 지원 + ecat-tls crate (전체 12개 백엔드)

`ecat-tls` crate 신규, 제공:
- `TlsClientConfig` — 선택적 TLS 설정 (ca_cert, client_cert, client_key, skip_verify)
- `generate_ca()` — 자체 서명 CA 인증서 생성
- `generate_server_cert()` — 서버 인증서 생성
- `generate_client_cert()` — 클라이언트 인증서 생성 (mTLS)

전체 12개 데이터 백엔드 Config에 `#[serde(default)] tls: Option<TlsClientConfig>` 필드 추가.

| 백엔드 유형 | TLS 방식 |
|----------|----------|
| 9개 HTTP 백엔드 | `tls.build_reqwest_client()`로 TLS reqwest Client 구축 |
| Redis | URL scheme 전환 `redis://` → `rediss://` |
| Sqlx | 예약 필드 (TLS는 URL 파라미터 `?sslmode=require`) |
| Memcached | 예약 필드 (네트워크 구현 예정) |

---

## 1. 개요

| 항목 | 상태 | 상세 |
|------|------|------|
| `cargo build` | ✅ 통과 | 컴파일러 warnings 3개, 19.85s |
| `cargo test` | ✅ 통과 | ~137개 단위 테스트 전부 통과, 실패 0, ignored 1 |
| `cargo clippy` | ⚠️ warning 있음 | 3개 crate에서 총 5개 lint warnings |
| `cargo fmt` | ✅ 통과 | 형식 문제 없음 |
| `cargo audit` | ❌ 미설치 | 알려진 CVE 스캔 불가 |

---

## 2. 컴파일러 Warnings (수정 필요)

### 2.1 ecat-versioning (warning 3개)

**파일**: `ecat-versioning/src/lib.rs`

| # | Warning | 줄 번호 | 심각도 |
|---|---------|------|----------|
| 1 | `unused import: axum::routing::get` | 3 | 낮음 |
| 2 | `unused variable: version` | 61 | 낮음 |
| 3 | `function extract_version is never used` | 68 | 낮음 |

**제안**: 미사용 import 삭제, `version`을 `_version`으로 변경, `extract_version`을 `pub`로 만들거나 `#[allow(dead_code)]` 표시.

### 2.2 ecat-data-questdb (clippy warning 1개)

**파일**: `ecat-data-questdb/src/lib.rs:39`

```rust
// 현재:
.query(&[("query", sql), ("count", &"true".to_string())])

// 변경:
.query(&[("query", sql), ("count", &"true")])
```

### 2.3 ecat-client (clippy warning 1개)

**파일**: `ecat-client/src/lib.rs:249`

`GrpcClientBuilder`가 `Default`를 수동 구현했는데, `#[derive(Default)]`로 대체 가능.

---

## 3. Clippy Lint Warnings 요약

| Crate | Warning | 유형 |
|-------|---------|------|
| ecat-versioning | `useless_format!` — `"/api".to_string()` 사용 | 성능 |
| ecat-versioning | unused import / dead code | 정리 |
| ecat-data-questdb | `unnecessary_to_owned` | 성능 |
| ecat-client | `derivable_impls` — derive Default 사용 | 단순화 |

---

## 4. 테스트 커버리지 분석

### 4.1 통계

| 지표 | 수치 |
|------|------|
| 단위 테스트 총수 | ~137 |
| 실패 | 0 |
| Ignored | 1 |
| 테스트 있는 crate | ~24 / 48 |
| **테스트 0인 crate** | **~24 / 48 (50%)** |

### 4.2 테스트 부족한 Crate (0개 또는 생성자 테스트만)

다음 crate의 테스트가 취약합니다:

- ecat-data-arangodb, ecat-data-clickhouse, ecat-data-elasticsearch
- ecat-data-influxdb, ecat-data-iotdb, ecat-data-nebulagraph
- ecat-data-neo4j, ecat-data-opensearch, ecat-data-questdb
- ecat-data-redis, ecat-data-sqlx, ecat-data-memcached
- ecat-mq, ecat-mq-kafka, ecat-graphql, ecat-openapi
- ecat-transport, ecat-transport-grpc, ecat-transport-http
- ecat-transport-ws, ecat-tracing, ecat-logging
- ecat-middleware, ecat-registry-consul, ecat-registry-etcd

### 4.3 Doc-tests

전체 **48개 crate의 doc-tests가 모두 0**입니다. 코드에 `/// ````rust` 문서 예시가 없습니다.

---

## 5. 의존성 문제

### 5.1 ⚠️ yaml_serde vs serde_yaml (중간 위험)

**파일**: `ecat-config/Cargo.toml:9`

```toml
yaml_serde = "0.10"
```

Rust 생태계의 표준 YAML 라이브러리는 `serde_yaml`(최신판 `0.9.34+`)이며, `yaml_serde`는 **다르고 유지보수가 덜 되는 crate**입니다.

**제안**: `yaml_serde`가 의도된 의존성인지 확인. 본래 `serde_yaml`을 의도했다면 교체하세요.

### 5.2 cargo-audit 부재

`cargo audit`가 설치되어 있지 않습니다. `cargo install cargo-audit` 후 CI에 추가를 제안합니다.

### 5.3 description 필드 부재

`[workspace.package]`에 `description`이 없으며, 모든 서브 crate도 description을 정의하지 않았습니다.

---

## 6. 코드 품질 문제

### 6.1 프로덕션 코드의 unwrap/expect

| 파일 | 줄 번호 | 호출 | 위험 |
|------|------|------|------|
| `ecat-client/src/lib.rs` | 28 | `.expect("StaticResolver poisoned")` | 낮음 — 합리적 |
| `ecat/src/signal.rs` | 8 | `.expect("failed to install SIGTERM handler")` | 중간 — 시작 시 panic |
| `ecat-protos/build.rs` | 5 | `.unwrap()` | 낮음 — build script |

### 6.2 ecat-versioning의 extract_version

`extract_version` 함수(68행)가 Accept header에서 버전 번호를 추출하도록 구현했지만, `build_header_router()`에서 호출되지 않습니다.

### 6.3 ecat-data-questdb 오류 처리

```rust
// 30행: 네트워크 응답 본문 읽기에 unwrap_or_default 사용
Err(RdbmsError::Database(resp.text().await.unwrap_or_default()))
```

`resp.text()` 실패 시 오류 메시지를 조용히 삼킵니다. `unwrap_or_else(|e| format!("questdb parse: {e}"))`로 변경을 제안합니다.

---

## 7. 아키텍처 평가

### 장점

- 48개 crate의 책임 분리가 명확
- workspace 통일 버전 `version.workspace = true`
- 의존성 간소화, 대형 프레임워크 없음
- TODO/FIXME/HACK 없음

### 개선 필요

| 문제 | 우선순위 |
|------|--------|
| crate 50% 테스트 없음 | 높음 |
| yaml_serde vs serde_yaml 혼동 | 중간 |
| cargo-audit 부재 | 중간 |
| ecat-versioning 죽은 코드 | 낮음 |
| doc-tests 없음 | 낮음 |

---

## 8. 보안 개요

| 검사 항목 | 결과 |
|--------|------|
| 하드코딩 키 | 발견 안 됨 |
| .env 파일 유출 | 발견 안 됨 |
| 위험한 unwrap (프로덕션 코드) | 2곳 (signal.rs, client.rs) |
| CVE 스캔 | 미실행 (cargo-audit 설치 필요) |

---

## 9. 실행 계획

### P0 — 즉시 수정
1. ecat-versioning의 컴파일러 warnings 3개 정리
2. ecat-data-questdb clippy 수정
3. ecat-client derivable_impls 수정

### P1 — 단기
4. `cargo-audit` 설치하여 의존성 취약점 스캔
5. `yaml_serde` vs `serde_yaml` 선택 확인
6. 핵심 crate에 doc-tests 보완

### P2 — 중기
7. transport/data/security crate에 테스트 보완
8. 모든 crate에 `description` 필드 추가
9. `extract_version` 통합 또는 제거

### P3 — 장기
10. CI 구축: build → test → clippy → audit → coverage

---

*보고서 생성일: 2026-08-01. 툴체인: cargo 1.92.0, rustc 1.92.0, clippy 1.92.0*
