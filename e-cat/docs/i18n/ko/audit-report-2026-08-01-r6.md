# e-cat 심층 감사 보고서 — 2026-08-01 R6

## 종합 평가

| 차원 | 상태 | 설명 |
|------|------|------|
| 컴파일 | 통과 | 50 crates, 오류 0 |
| 테스트 | 통과 | 전부 통과, 실패 0 |
| Clippy | 통과 | 경고 0 (`-D warnings`) |
| unsafe | 0 | 코드베이스에 unsafe 블록 없음 |
| 파일 규모 | 양호 | `ecat-auth`(540줄)만 500줄 권장치 초과 |

## 발견 항목 (15건)

### 보안 관련

#### 1. [심각] XOR「암호화」는 진짜 암호화가 아님
**파일:** `ecat-config/src/encrypted.rs:45-56`
**문제:** `decrypt()`가 XOR + 반복 키를 사용 — 이는 난독화이지 암호화가 아니며 쉽게 해독됩니다. 키가 모든 바이트 위치에서 반복 사용되어 암호문이 빈도 분석 공격에 매우 취약합니다.
**제안:** AES-256-GCM(`aes-gcm` crate)으로 교체하거나, 「암호화」가 아닌 「난독화」로 명시적으로 표기하세요.

#### 2. [심각] `execute_with`/`query_with` 기본 구현이 매개변수를 조용히 폐기
**파일:** `ecat-data/src/rdbms.rs:86-103`
**문제:** trait의 기본 구현이 매개변수를 받지만 무시하고(`let _ = params;`) 원래 `execute(sql)`을 직접 호출합니다. `ecat-data-sqlx`를 제외한 모든 백엔드(ClickHouse, QuestDB)가 이 동작을 상속합니다. 사용자가 백엔드를 매개변수화 메서드로 교체하면 매개변수가 조용히 버려져 SQL 인젝션 취약점이 발생합니다.
**제안:** 기본 구현이 「지원하지 않음」 오류를 반환하거나, 각 백엔드가 매개변수 바인딩을 올바르게 구현하도록 하세요.

#### 3. [고위험] 비밀번호가 URL에 평문으로 내장
**파일:** `ecat-data-sqlx/src/lib.rs:40`, `ecat-data-redis/src/lib.rs:43`
**문제:** `connect_with_auth()`가 `replacen("://", "://user:pass@")`로 자격 증명을 URL에 직접 내장합니다. 이 URL은 로그, 오류 메시지, 디버그 출력에 기록될 수 있습니다.
**제안:** 각 백엔드의 네이티브 인증 메커니즘을 사용하거나, 최소한 이어붙이기 전에 사용자 이름/비밀번호를 URL 인코딩하세요.

#### 4. [중위험] TLS 설정 실패 시 panic
**파일:** 8개 data-* crate(ClickHouse, QuestDB, Elasticsearch, OpenSearch, ArangoDB, Neo4j, NebulaGraph, InfluxDB, IoTDB)
**패턴:** `.expect("TLS client build failed")` — 모든 `from_config()` 생성자가 TLS 설정 오류 시 panic합니다.
**제안:** `from_config()`를 `Result` 반환으로 바꾸거나, TLS 클라이언트 구성을 지연/내결함 방식으로 바꾸세요.

### 기능 정확성

#### 5. [고위험] `ecat-versioning` Header 라우팅 무효
**파일:** `ecat-versioning/src/lib.rs:56-64`
**문제:** `build_header_router()`가 모든 버전을 동일한 `/api` 경로 아래에 중첩하지만 버전 header로 필터링하지 않습니다. axum은 모든 버전 라우트를 같은 경로에 등록하여 라우트 충돌과 예측 불가능한 동작이 발생합니다. `extract_version()` 함수는 존재하지만 라우트에서 한 번도 사용되지 않습니다.
**제안:** axum middleware/layer로 Accept header를 검사해 올바른 버전 라우트로 보내는 대신, 모든 버전을 같은 경로에 평탄화하지 마세요.

#### 6. [중위험] Redis TTL 절단: 1초 미만 만료가 영구 만료 없음으로
**파일:** `ecat-data-redis/src/lib.rs:76-77`
**문제:** `Duration::as_secs()`가 0 방향으로 절단됩니다. 500ms TTL을 설정하면 `secs == 0`이 되어 조용히 영구 만료 없음이 되고, `SETEX` 대신 `SET` 분기를 타게 됩니다.
**제안:** 1초 미만 TTL은 최소 1초로 설정하거나, `SETEX` 대신 `SET ... PX`(밀리초)를 사용하세요.

#### 7. [중위험] `StaticResolver::add_service`가 락 경쟁 시 panic
**파일:** `ecat-client/src/lib.rs:27-29`
**문제:** `try_write()` + expect를 사용 — 다른 쓰기 락 보유자가 있으면 panic합니다. builder 패턴으로 발동이 어렵지만 동시성 코드에서는 시한폭탄입니다.
**제안:** `blocking_write()`(동기 컨텍스트인 경우)를 사용하거나 `&mut self`를 받도록 바꿔 락 필요성을 없애세요.

### 코드 품질

#### 8. [중위험] 비동기 컨텍스트에서 `std::sync::Mutex` 사용
**파일:** `ecat-data-memcached/src/lib.rs:7,24`
**문제:** async trait 구현에서 `std::sync::Mutex`를 사용합니다. 락 보유 시간이 매우 짧고(HashMap 연산뿐) 경쟁이 심할 때 이론적으로 비동기 런타임을 블로킹할 수 있습니다.
**제안:** 이 인메모리 캐시의 특정 사용 시나리오에서는 임계 구간이 매우 짧고 `.await` 지점이 없으므로 `std::sync::Mutex`가 실제로 허용됩니다. 그러나 향후 락 내부에서 I/O를 수행해야 한다면 `tokio::sync::Mutex`로 바꾸세요.

#### 9. [낮음] 수제 base64 구현
**파일:** `ecat-registry-etcd/src/lib.rs:148-193`
**문제:** ~45줄의 수제 base64 코덱 — 경계 조건 버그 가능성이 있습니다. Rust 생태계에는 `base64` crate 등 충분히 검증된 대안이 있습니다.
**제안:** `base64` crate로 교체해 유지보수 부담과 잠재적 버그를 줄이세요.

#### 10. [낮음] `RandomBalancer`가 무작위가 아님
**파일:** `ecat-client/src/lib.rs:91-105`
**문제:** `Instant::now()` 해시를 난수 소스로 사용합니다. 같은 인스턴스에서 동시에 발생한 호출은 동일한 「무작위」 선택을 받습니다. `checked_add(0)`은 불필요한 연산입니다.
**제안:** `rand` crate 또는 최소한 `std::collections::hash_map::RandomState`를 사용하세요.

#### 11. [낮음] `ecat-data-sqlx`의 불필요한 `Arc<Vec<String>>`
**파일:** `ecat-data-sqlx/src/lib.rs:79-87, 197-203`
**문제:** 컬럼 이름이 `Arc<Vec<String>>`로 감싸져 있지만 각 `Row` 생성자가 컬럼 이름 목록 전체를 클론합니다(`(*cols).clone()`). `Arc`는 반복 중 단 한 번만 사용되므로 `Rc`나 직접 `clone()`으로 충분합니다.
**제안:** `query()`와 `query_with()`에서 `Arc<Vec<String>>`를 일반 `Vec<String>`으로 교체하세요. 행별 개별 클론 비용은 Arc 역참조 + 클론과 동일합니다.

### 설계/아키텍처

#### 12. [정보] QuestDB가 GET + 쿼리 매개변수 사용
**파일:** `ecat-data-questdb/src/lib.rs:76, 91`
**문제:** SQL이 GET 쿼리 매개변수로 전송되어 URL 길이 제한(보통 ~2000-8000자)을 받습니다. 큰 쿼리는 잘립니다.
**제안:** POST + body 방식으로 바꾸거나, 단순 쿼리는 GET을 유지하고 복잡한 쿼리는 POST를 사용하세요.

#### 13. [정보] `#[allow(dead_code)]`가 여기저기 흩어져 있음
**파일:** `ecat-registry-consul/src/lib.rs:225`, `ecat-data-memcached/src/lib.rs:25-28`, `ecat-auth/src/lib.rs:52`
**문제:** username/password 필드가 메모리에 저장되지만 dead_code로 표시됨(인메모리 memcached에서는 불필요; auth의 RSA 변형은 아직 미구현).
**제안:** 누락된 기능 경로를 구현하거나, 필드를 삭제하거나, 유지 이유를 설명하는 문서를 추가하세요.

#### 14. [정보] 일부 HTTP 클라이언트에 Content-Type header 부재
**파일:** `ecat-data-influxdb/src/lib.rs:96-103`, `ecat-data-clickhouse/src/lib.rs:87-89`
**문제:** 일부 POST 요청이 `Content-Type` header를 설정하지 않아 서버측 자동 감지에 의존합니다.
**제안:** 호환성 보장을 위해 항상 명시적 Content-Type을 설정하세요.

#### 15. [정보] `ecat-auth`가 500줄 초과
**파일:** `ecat-auth/src/lib.rs` (540줄)
**문제:** CLAUDE.md는 파일을 500줄 미만으로 유지하도록 요구합니다. auth crate가 유일한 초과 파일입니다.
**제안:** JWT 검증 로직을 `ecat-auth/src/jwt.rs`로 분리하거나 기능별로 분리하세요.

## 최적화 기회(버그 아님)

| # | 위치 | 제안 |
|---|------|------|
| O1 | 모든 data-* crate | 모든 `from_config()`의 반복되는 TLS 클라이언트 빌드 패턴을 공유 매크로나 함수로 추출 가능 |
| O2 | `ecat-data-sqlx` | `query()`와 `query_with()`의 행 타입 변환 로직(117줄 중복)을 헬퍼 함수로 추출 가능 |
| O3 | `ecat-client` | `HttpClient::get()`과 `post()`가 동일한 「resolve → pick → build URL」 파이프라인 공유 — 추출 가능 |
| O4 | `ecat-data` | 5개 traits(Rdbms/Cache/Graph/Search/Tsdb)의 사용자 정의 오류 타입을 단일 `DataError` 열거형으로 통일 가능 |
| O5 | `ecat-data-redis` | 각 메서드의 `self.conn.clone()`은 불필요 — `MultiplexedConnection`은 공유 지원을 위해 `Clone` 설계됨 |

## 지표 요약

| 지표 | 수치 |
|------|------|
| 총 crate 수 | 50 |
| Rust 소스 파일 총 줄 수 | 7,968 |
| 비테스트 코드의 `expect()` | 12 |
| 비테스트 코드의 `unwrap()` | 0 |
| `unsafe` 블록 | 0 |
| 비테스트 코드의 `panic!` | 0 |
| `#[allow(dead_code)]` | 4 |
| TODO/FIXME/HACK | 0 |
| 비동기 코드의 std Mutex | 1 (memcached) |

## 결론

코드베이스는 양호한 상태입니다 — 컴파일, 테스트, clippy 전부 통과, unsafe 코드 없음, panic 매크로 없음. 가장 중요한 두 가지 문제는 **XOR「암호화」**(가짜 보안)와 **매개변수화 쿼리 기본 구현의 조용한 매개변수 폐기**(보안 취약점)입니다. Header 라우팅 기능도 완전히 사용 불가합니다. 다른 문제는 상대적으로 작으며 유지보수성 차원의 최적화입니다.

**권장 우선 수정 순서:**
1. `execute_with`/`query_with` 기본 구현 → 매개변수를 조용히 버리는 대신 오류 반환
2. XOR 암호화 → 진짜 AEAD 암호화, 또는 「난독화」로 이름 변경
3. Header 버전 라우팅 → 실제 header 라우팅 구현
4. `from_config()` → expect-panic 대신 Result 반환
5. Redis TTL 절단 → 1초 미만 TTL은 최소 1초 사용

## 수정 상태 (R6 → R6.1)

| # | 문제 | 상태 | 변경 |
|---|------|------|------|
| 1 | XOR "암호화" | 수정됨 | `EncryptedSource` → `ObfuscatedSource`, `decrypt` → `deobfuscate`, 접두사 `enc:` → `obfs:`, 난독화이지 암호화가 아니라는 문서 추가 |
| 2 | `execute_with`/`query_with` 조용한 매개변수 폐기 | 수정됨 | 기본 구현이 `"parameterized ... not supported by this backend"` 오류 반환으로 변경 |
| 3 | 비밀번호 URL 평문 내장 | 수정됨 | `connect_with_auth` 메서드에서 `percent_encode()`로 자격 증명 인코딩 |
| 4 | TLS `expect()` panic | 수정됨 | 9개 crate의 `from_config()`가 `Result` 반환으로 변경, `RdbmsError`에 `Config` 변형 추가 |
| 5 | Header 라우팅 무효 | 수정됨 | `from_fn_with_state` 미들웨어로 버전 검증 구현, `header_versioned_router_builds` 테스트 추가 |
| 6 | Redis TTL 절단 | 수정됨 | `set_ex` → `pset_ex`, 밀리초 정밀도로 1초 미만 TTL이 영구 만료 없음으로 잘리는 것 방지 |
| 7 | `StaticResolver` 락 경쟁 panic | 수정됨 | `try_write()` → `blocking_write()` |
| 8 | `RandomBalancer` 무작위 아님 | 수정됨 | `Instant::now()` 해시를 `RandomState::new().build_hasher()`로 대체 |
| 9 | `std::sync::Mutex` 비동기 컨텍스트 | 수정됨 | `tokio::sync::Mutex`로 교체 |
| 10 | 수제 base64 | 수정됨 | `base64` crate 0.22로 교체 |
| 11 | `Arc<Vec<String>>` 오버헤드 | 수정됨 | 일반 `Vec<String>`으로 교체, 불필요한 Arc 래핑 제거 |
| 12 | QuestDB GET 방식 SQL 전송 | 수정됨 | POST + body로 변경, Content-Type header 추가 |
| 13 | `#[allow(dead_code)]` | 수정됨 | memcached 필드에 `_` 접두사; consul 필드에 `_` 접두사 및 allow 제거; auth에서 `Rsa` → `RsaReserved` |
| 14 | Content-Type 부재 | 수정됨 | InfluxDB, ClickHouse, IoTDB 요청에 명시적 Content-Type 추가 |
| 15 | `ecat-auth` 500줄 초과 | 수정됨 | `claims.rs`(31) + `jwt.rs`(139) + `apikey.rs`(96) + `oauth2.rs`(173) + `helpers.rs`(28) + `lib.rs`(98)로 분리 |

### 영향받은 Crate

| Crate | 변경 유형 |
|-------|----------|
| `ecat-data` | trait 기본 구현, `RdbmsError::Config` 변형 |
| `ecat-config` | `EncryptedSource` → `ObfuscatedSource` |
| `ecat-versioning` | Header 라우팅 미들웨어 구현 |
| `ecat-data-redis` | TTL 밀리초 정밀도, 자격 증명 URL 인코딩 |
| `ecat-data-sqlx` | 자격 증명 URL 인코딩, Arc 오버헤드 제거 |
| `ecat-data-clickhouse` | `from_config` → `Result`, Content-Type header |
| `ecat-data-questdb` | `from_config` → `Result`, GET → POST |
| `ecat-data-elasticsearch` | `from_config` → `Result` |
| `ecat-data-opensearch` | `from_config` → `Result` |
| `ecat-data-arangodb` | `from_config` → `Result` |
| `ecat-data-neo4j` | `from_config` → `Result` |
| `ecat-data-nebulagraph` | `from_config` → `Result` |
| `ecat-data-influxdb` | `from_config` → `Result`, Content-Type header |
| `ecat-data-iotdb` | `from_config` → `Result`, Content-Type header |
| `ecat-data-memcached` | `std::sync::Mutex` → `tokio::sync::Mutex`, dead_code 정리 |
| `ecat-client` | `StaticResolver`, `RandomBalancer` 수정 |
| `ecat-registry-etcd` | base64를 crate로 교체 |
| `ecat-registry-consul` | dead_code 정리 |
| `ecat-auth` | 6개 모듈로 분리, dead_code 정리 |

### 최종 검증 (R6.2)

| 차원 | 상태 |
|------|------|
| Build | 통과, 오류 0 경고 0 |
| Test | 전부 통과, 실패 0 |
| Clippy (`-D warnings`) | 통과, 경고 0 |
| 파일 규모 | 전부 ≤ 300줄 |
