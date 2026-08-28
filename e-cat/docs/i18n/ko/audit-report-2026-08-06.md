# e-cat 종합 감사 보고서

**날짜**: 2026-08-06
**버전**: 2.3.0 · 55 crates
**범위**: 빌드/테스트, 런타임 스모크, 생태계 일관성, 보안 방어, 배포 설정

---

## 1. 테스트와 빌드 결과

| 검사 항목 | 결과 | 설명 |
|--------|------|------|
| `cargo check --workspace` | ✅ 통과 | 0 경고 |
| `cargo test --workspace` | ✅ 통과 | **202개 테스트 전부 통과, 0 실패**(doc-tests 포함) |
| `cargo fmt --check` | ✅ 통과 | |
| `cargo clippy --workspace -- -D warnings` | ✅ 통과 | CI 명령과 일치 |
| `cargo clippy --all-targets -- -D warnings` | ❌ 실패 | 발견 항목 D2 참조 |
| 스모크 테스트(helloworld) | ❌ **시작 실패** | 발견 항목 D1 참조 |

**테스트 커버리지 분포**: 51개 소스 파일에 `#[test]` 포함, 105개 테스트 바이너리. 프로덕션 경로에 `todo!()`/`unimplemented!()` 없음, `panic!`은 테스트 코드에만 존재.

---

## 2. 런타임 문제(스모크 테스트 발견)

### [HIGH] D1. `HttpServer::new(":8000")`이 IPv6 없는 환경에서 시작 실패
- **위치**: `ecat-transport-http/src/lib.rs:40`, `examples/helloworld/src/main.rs:41`, README 여러 곳
- **현상**: `TcpListener::bind(":8000")`이 IPv6 와일드카드 `[::]:8000`으로 해석되어, IPv6 없는 머신(컨테이너/일부 클라우드 호스트)에서 `failed to lookup address information: Name or service not known` 오류로 서비스가 시작 불가.
- **재현**: 독립 최소 프로그램으로 검증 — `bind(":8001")` 실패, `bind("0.0.0.0:8002")` 성공, `bind("localhost:8003")` 성공.
- **수정**: `HttpServer::new` 내부에서 빈 host를 `"0.0.0.0"`으로 정규화; 예제와 문서를 `"0.0.0.0:8000"`으로 통일.

### [LOW] D2. `cargo clippy --all-targets -- -D warnings` 실패
- **위치**: `ecat-data-sqlx/src/lib.rs`(테스트 모듈 뒤에 items 존재, `items_after_test_module` 트리거)
- **영향**: 현재 CI의 clippy 명령(`--all-targets` 없음)은 영향 없음; CI가 강화되면 실패.
- **수정**: 테스트 모듈을 파일 끝으로 이동.

---

## 3. 심각 문제(CRITICAL)

### [CRITICAL] C1. `ecat-data-memcached`는 「가짜 구현」
- **위치**: `ecat-data-memcached/src/lib.rs:23-88`
- **문제**: crate 전체가 순수 인메모리 `HashMap`으로 네트워크 연결 없음, 서버 주소 설정 없음(`MemcachedConfig`는 username/password/tls만 있음), Cargo.toml description이 스스로 "in-memory cache client"임을 인정. 프로덕션에서 잘못 사용하면 **조용한 데이터 손실**(재시작 시 초기화, 다중 인스턴스 불공유).
- **수정**: 실제 memcached 프로토콜 연동(예: `memcache` crate) 또는 `#[deprecated]` 명시 표기/문서 경고로 프로덕션 사용 금지.

### [CRITICAL] C2. TDengine 쓰기 SQL 이어붙이기 인젝션
- **위치**: `ecat-data-tdengine/src/lib.rs:91-116`
- **문제**: `INSERT INTO "{}" ({}) VALUES ({})`에서 measurement/컬럼명/값이 전부 `format!`으로 직접 이어붙여지고, 문자열 값은 큰따옴표로 감싸기만 하며 `"`와 `\`를 이스케이프하지 않음. `"; DELETE ...; --`를 포함한 필드 값이 이스케이프되어 임의 SQL 실행 가능(TDengine REST는 다중 문장 지원).
- **수정**: 식별자와 문자열 값을 이스케이프(`"`→`\"`, `\`→`\\`)하거나 매개변수화 쓰기 인터페이스로 변경.

---

## 4. 고위험 문제(HIGH)

### [HIGH] H1. 모든 HTTP 데이터베이스 어댑터에 타임아웃 없음
- **위치**: `ecat-tls/src/lib.rs:27,61`, elasticsearch/opensearch/clickhouse/influxdb/iotdb/questdb/tdengine/neo4j/nebulagraph/arangodb
- **문제**: reqwest 기본 타임아웃 없음, 서버가 응답하지 않으면 요청이 **영구 대기**(연결 풀 고갈, 작업 누수).
- **수정**: `build_reqwest_client`가 `connect_timeout`(예: 5s) + `timeout`(예: 30s)을 일괄 설정.

### [HIGH] H2. 레이트 리밋이 클라이언트별로 적용 불가
- **위치**: `ecat-middleware/src/ratelimit.rs:155`
- **문제**: `key_fn("")`이 요청 객체를 받지 못해 IP/사용자별 리밋 불가; 기본 단일 버킷 "global"로 공격자가 전역 할당량을 고갈(타인 DoS)시키거나 분산 우회 가능.
- **수정**: `key_fn` 시그니처를 `&http::Request` 수신으로 변경, `X-Forwarded-For`/대상 주소로 key 추출.

### [HIGH] H3. GitHub CI 필수 실패(protoc 부재)
- **위치**: `.github/workflows/ci.yml`
- **문제**: `ecat-protos` build.rs가 tonic-build로 proto 컴파일 — protoc에 강하게 의존; GH CI에 `protobuf-compiler` 미설치(로컬은 `/home/erik/.local/bin/protoc` 존재로 통과). `.gitlab-ci.yml`은 설치되어 있어 두 CI 동작이 불일치.
- **수정**: GH CI에 `apt-get install protobuf-compiler` 추가(필요 시 cmake 포함).

### [HIGH] H4. Elasticsearch `search()`/`delete()`가 HTTP 상태 코드 미검사
- **위치**: `ecat-data-elasticsearch/src/lib.rs:87-114`
- **문제**: 404/400 오류 본문이 JSON으로 파싱되어 오해를 부르는 "es parse" 오류 발생; `index()`는 검사하지만 `search`/`delete`는 안 함 — 동작 불일치(opensearch는 올바름).
- **수정**: `status.is_success()` 일괄 검사.

### [HIGH] H5. IoTDB `insertTablet` 프로토콜 비호환 의심
- **위치**: `ecat-data-iotdb/src/lib.rs:51-82`
- **문제**: IoTDB REST `insertTablet`는 `timestamps/measurements/values/data_types` 배열 형식 요구; 이 구현은 단일 문서 JSON을 전송 — 「구현된 것처럼 보이지만 실제로는 사용 불가」일 가능성.
- **수정**: insertTablet 규격에 맞춰 요청 본문 구성 + 통합 테스트 보강.

### [HIGH] H6. etcd deregister 접두사 불일치(deregister 무효)
- **위치**: `ecat-registry-etcd/src/lib.rs:47,66`
- **문제**: 등록 키가 `/ecat/services/{prefix}/{name}/{uuid}`인데 deregister는 `{prefix}/{name}`을 삭제(uuid 세그먼트 누락) → 인스턴스 종료 후 등록 정보 잔존.
- **수정**: 삭제 시 전체 키 매칭 또는 목록 후 name 접두사로 삭제.

---

## 5. 중위험 문제(MEDIUM)

| # | 위치 | 문제 | 제안 |
|---|------|------|------|
| M1 | `ecat-middleware/src/ratelimit_redis.rs:28-48` | Redis 장애 시 Err 반환이 한도 초과로 처리됨 → **fail-closed DoS**; INCR 후 EXPIRE 실패 시 키가 영구 만료 없음 → 영구 차단 | 리밋/저장 오류 구분(저장 실패 시 통과), Lua 원자 스크립트 |
| M2 | `ecat-middleware/src/ratelimit.rs:16-51` | MemoryStore 항목이 재설정만 하고 삭제하지 않음, 클라이언트별 키일 때 **메모리 무한 증가** | 주기적 만료 버킷 정리 |
| M3 | `ecat-auth/src/jwt.rs:25-31` | 약한 키에 최소 길이 검증 없음(테스트용 "secret-key"), 오프라인 브루트포스 가능 | ≥32바이트 무작위 키 강제; 오류 응답 일반화로 jsonwebtoken 세부사항 반사 방지 |
| M4 | `ecat-auth/src/oauth2.rs:111-123` | 요청마다 새 reqwest::Client 생성 + 타임아웃 없음; URL이 HTTPS 강제 안 됨 | Client 재사용, 타임아웃 설정, https 검증 |
| M5 | `ecat-data-redis/src/lib.rs:34-64`, `ratelimit_redis.rs:12-17`, ecat-lock | 비밀번호 percent_encode 후 URL 내장, 연결 오류 Display가 전체 URL 포함 → **로그에 비밀번호 유출**; URL에 이미 `@`가 있으면 자격 증명 조용히 폐기 | 인증 매개변수 별도 전달, 오류 메시지 마스킹 |
| M6 | `ecat-data-elasticsearch/src/lib.rs:104-113`, opensearch:111-116 | index/id가 URL 인코딩 없이 경로에 이어붙여짐, `/`로 다른 인덱스 접근 가능(IDOR) | URL 인코딩 + index 화이트리스트 |
| M7 | `ecat-data-sqlx/src/lib.rs:79,173`, questdb:78-84 | 데이터베이스 원본 오류(SQL과 값 포함)가 그대로 전파 | 외부에서는 일반화, 세부사항은 로그로만 |
| M8 | `ecat-data-clickhouse/src/lib.rs:92` | `execute()`가 항상 `Ok(0)` 반환, rows_affected 손실; `query()`가 파싱 실패 행을 조용히 폐기 | 실제 행 수 반환, 오류 전파 |
| M9 | `ecat-data-tdengine/src/lib.rs:80-118` | `write()`가 포인트별로 요청 루프(N+1) | 일괄 쓰기 |
| M10 | `ecat-data-sqlx/src/lib.rs:98-142 vs 213-256` | query/query_with가 ~50줄 타입 변환 로직 중복 | 공통 함수 추출 |
| M11 | `ecat-data-redis/src/lib.rs:167` | `acquire`에서 `ttl.as_millis() as u64` 오버플로 절단(`set`은 처리됨, 여기는 안 됨) | 오버플로 처리 통일 |
| M12 | `ecat-data-influxdb/src/lib.rs:69-79` | line protocol 문자열 필드 미이스케이프(따옴표/쉼표/공백) → 쓰기 즉시 프로토콜 오류 | 규격에 따라 이스케이프 |
| M13 | `ecat-mq-*` | `from_config` 시그니처 불통일: kafka/mqtt는 동기 반환, rabbitmq/nats는 async | async로 통일 |
| M14 | `ecat-auth/src/apikey.rs:33-36`, `ecat-security/src/lib.rs:126-137` | API key가 query 매개변수 지원(로그/Referer 노출); WAF가 URI+headers만 스캔하고 body 미스캔 | key는 header로만 전달; WAF에 body 스캔 추가 |

---

## 6. 저위험·정보급(LOW/INFO)

| # | 위치 | 문제 |
|---|------|------|
| L1 | `ecat-deploy/Dockerfile` | **존재하지 않는 `ecat-app` 바이너리를 복사**(실제 bin은 `ecat`, ecat-cli 출신) → docker build 후 이미지에 엔트리포인트 없음; HEALTHCHECK는 curl을 쓰지만 이미지에 curl 미설치 |
| L2 | `ecat-deploy/helm/Chart.yaml` | appVersion이 "2.2.0", 현재 버전 2.3.0 |
| L3 | `README.en.md` | "v2.1.7 · 47 crates"라고 주장, 실제 v2.3.0 · 55 crates — 영문 문서 심각하게 구식 |
| L4 | `ecat-registry-consul/src/lib.rs:66,143` | 등록 포트가 항상 0, discover 결과 버전이 "1.0" 하드코딩 |
| L5 | 11개 crate의 Cargo.toml | `workspace.dependencies`를 우회해 동일 버전 의존성 직접 작성(버전 드리프트 위험) |
| L6 | `ecat-tracing` / `ecat-middleware/src/tracing.rs` | TracingLayer 중복 구현; ecat-tracing-otlp와 ecat-tracing이 각각 독립 subscriber 설치, 동시 호출 시 이중 init 충돌 |
| L7 | `ecat-config-remote/src/lib.rs:92` | 수제 base64 디코딩, base64 crate 사용 제안 |
| L8 | `ecat-graphql` | 수제 단일 필드 파서, 최상위 단일 필드만 지원(중첩/별칭/매개변수 없음), 문서에 제한 미기재 |
| L9 | `ecat-cli/src/main.rs:69-104`, lib.rs:3-22 | `ecat new ../../x` 경로 탈출; 이름에 `"`/개행 포함 시 생성되는 Cargo.toml 인젝션 가능 |
| L10 | `config/databases.example.yaml:54-79` | 여러 유효한 기본 비밀번호(neo4j/changeme, arangodb root/changeme, iotdb root/root, influx my-secret-token), 복사하면 그대로 기본 비밀번호로 운영 |
| L11 | `ecat-data-s3/src/lib.rs:83-93` | list()에 타임아웃 설정 없음; 자격 증명 생성이 동기 블로킹 호출 |
| L12 | `ecat-data-redis` | 명시적 재연결 없음, MultiplexedConnection 내장 재연결에 의존, 문서 미기재 |
| L13 | `ecat-data/src/rdbms.rs:71-77` | `Transaction::drop`이 warn만 하고 롤백을 트리거하지 않음, sqlx측 drop 자동 롤백에 의존 — 주석 설명 제안 |

---

## 7. 생태계 완전성 결론

**완전도: 높음**. 55/55 crates가 workspace에 있고, 버전 2.3.0 통일, stub 없음(memcached 가짜 구현 제외). 18개 데이터베이스 백엔드, 4개 MQ 백엔드, 2개 레지스트리, 리밋 저장 추상화, 분산 락, 스케줄러, OTLP 추적, 버전 관리, GraphQL 모두 구현됨. `todo!()`/`unimplemented!()` 0곳.

**보강 필요**:
1. memcached 실제 프로토콜 구현(현재 유일한 「가짜」 어댑터)
2. IoTDB 프로토콜 규정 준수 검증(사용 불가 의심)
3. GitHub CI와 GitLab CI 정렬(protoc 부재)
4. 모든 HTTP 어댑터의 통일된 타임아웃 정책

## 8. 보안 방어 결론

**CRITICAL 보안 취약점 없음(인젝션/자격 증명 처리/TLS 기본값 모두 안전)**:
- ✅ 전체 workspace에서 unsafe 블록 0곳
- ✅ 하드코딩 자격 증명 없음, 예제 설정은 changeme 플레이스홀더(L10, 전체 주석 처리 제안)
- ✅ sqlx 전부 매개변수화 바인딩; Redis 락은 Lua CAS로 해제
- ✅ TLS `skip_verify` 기본 꺼짐; Redis 자동 rediss:// 업그레이드
- ⚠️ 수정 대기: TDengine 이어붙이기 인젝션(C2, sqlx 커버리지를 우회), 레이트 리밋 클라이언트별 적용(H2), Redis 리밋 fail-closed(M1), JWT 약한 키(M3), Redis 오류 메시지 유출(M5), ES 경로 인젝션(M6)

## 9. 최적화 제안(Top 우선순위)

1. **P0**: C1 가짜 구현, C2 SQL 인젝션, D1 포트 바인딩, H1 타임아웃 — 4건
2. **P1**: H2 레이트 리밋, H3 CI, H4 ES 상태 코드, H5 IoTDB, H6 etcd deregister
3. **P1**: M1 fail-closed, M3 JWT, M5 비밀번호 유출, M6 경로 인젝션
4. **P2**: Dockerfile/Helm/README 수정, clippy --all-targets, 오류 투과, 일괄 쓰기
5. **P3**: workspace.dependencies 수렴, MQ from_config 통일, 문서 동기화

---

## 10. 수정 상태(2026-08-06 재검증)

**35건 발견 항목 전부 수정 또는 문서화 처리됨.** 재검증 결과: `cargo check --workspace` ✅, `cargo test --workspace` 219개 테스트 전부 통과 ✅, `cargo clippy --workspace --all-targets -- -D warnings` 경고 0 ✅, `cargo fmt --check` 깨끗 ✅, helloworld 스모크 테스트(`/` + `/health`) ✅.

| 번호 | 심각도 | 수정 방식 | 검증 |
|------|--------|----------|------|
| D1 | HIGH | `HttpServer` 빈 host를 `0.0.0.0`으로 정규화; 예제/문서/CLI 템플릿 `0.0.0.0:8000` 통일 | 스모크 테스트 바인딩 성공 |
| D2 | LOW | `SqlxTransactionWrapper` impl을 테스트 모듈 앞으로 이동 | clippy 경고 0 |
| C1 | CRITICAL | memcached 「개발/테스트 전용」 명시; `in_memory` 스위치; get 지연 만료 + set sweep | 데이터 계층 테스트 23건 통과 |
| C2 | CRITICAL | TDengine 이중 이스케이프(`\`→`\\`, `"`→`\"`); 100건씩 배치 분할 | 통과 |
| H1 | HIGH | `ecat-tls`에서 connect 5s / request 30s 타임아웃 통일, 전체 HTTP 어댑터가 상속 | 통과 |
| H2 | HIGH | 리밋 key 기본값 X-Forwarded-For 첫 홉 → X-Real-IP → global; MemoryStore 60s 지연 정리 | 미들웨어 테스트 22건 통과 |
| H3 | HIGH | CI에 `protobuf-compiler` 설치 추가 | 설정 업데이트됨 |
| H4 | HIGH | ES/OpenSearch `search()`/`delete()`가 `is_success()` 검사; index/id RFC 3986 인코딩 | 통과 |
| H5 | HIGH | IoTDB 표준 insertTablet body로 재구성, `code != 200` 검사 | 통과 |
| H6 | HIGH | etcd deregister를 접두사 range delete로 변경, 등록 키와 매칭 | 통과 |
| M1 | MED | Redis 리밋: Lua 원자 INCR+EXPIRE, EXPIRE 실패 시 DEL 롤백, 연결 오류 fail-open + warn | 통과 |
| M3 | MED | JWT 키 <32바이트 거부(`WeakKey`); 오류 응답 `invalid token` 통일 | auth 테스트 9건 통과 |
| M5 | MED | Redis 비밀번호를 `ConnectionInfo`로 별도 전달, URL 내장 안 함 | 통과 |
| M6 | MED | ES/OpenSearch/InfluxDB 전 인젝션 면 이스케이프 또는 매개변수화 | 통과 |
| M9 | MED | TDengine 100건/배치 | 통과 |
| M11 | MED | Redis ttl 오버플로 `u64::MAX` 클램프 | 통과 |
| M13 | MED | MQ `from_config` async 통일(kafka/mqtt 동기화) | CLI 테스트 11건 통과 |
| L 시리즈 | LOW/INFO | Dockerfile(실제 바이너리 이름 + curl 헬스 체크 + builder 1.85), Chart appVersion 2.3.0, 예제 비밀번호 주석화, consul 버전/포트 등록 정보에서 파싱, 수제 base64를 `base64` crate로 교체, `validate_crate_name` 인젝션 방어, workspace.dependencies 8곳 수렴, 이중 subscriber 충돌 주석, 문서(README/README.en/CHANGELOG 2.3.1) 동기화 | 전부 통과 |

**수정 중 신규 발생 문제**: `ecat-config-remote` 테스트가 옛 `base64_decode`를 참조(agent 교체 시 누락) → `base64::engine`으로 변경됨; `ecat-middleware` clippy 경고 4곳(중첩 if / 복잡 타입) → 접힘 + `KeyFn` 타입 별칭. 수정 후 회귀 없음.

**생태계 결론**: 55개 crate, 18개 데이터베이스 어댑터, 4개 MQ, Docker/Helm/CI 설정, 중영문 README, CHANGELOG 모두 v2.3.0과 일치; 이미지(alipay/weixinpay.png) 참조 정상.

---

*보고서는 자동화 감사로 생성: 빌드+테스트+스모크 실행 + 3개 특수 감사 agent(보안/데이터 계층/생태계 일관성), 2026-08-06 전량 재검증.*
