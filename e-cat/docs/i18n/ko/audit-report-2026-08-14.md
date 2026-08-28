# 특수 감사 보고서(보안과 성능) — 2026-08-14

감사 범위: 55 crate workspace(v2.3.5). 방법: Cargo.lock 수동 확인(cargo-audit 미설치), 인증/TLS 경로 소스 감사, 동시성·리소스 수명 주기 검사. 커밋된 코드 없음.

## 의존성 CVE 확인

- 핵심 의존성 버전이 모두 비교적 최신이고 알려진 미수정 CVE 없음: rustls 0.23.43, ring 0.17.14, aws-lc-rs 1.17.3, jsonwebtoken 9.3.1, tokio 1.53.1, h2 0.4.15, quinn 0.11.11, sqlx 0.8.6, zerocopy 0.8.55, time 0.3.54, openssl 0.10.81.
- hyper 0.14.32(rust-s3 0.35.1에서만 유입, hyper-tls 0.5 경유)가 0.14.28 수정선보다 높음.
- 주의: CI에 cargo-audit 미설치, 워크플로우에 자동화 확인 추가 제안.

## 발견 항목(심각도순 정렬)

### S1 [중] HTTP TLS 핸드셰이크 직렬화 → 느린 핸드셰이크 DoS
- 위치: `ecat-transport-http/src/lib.rs:134-150`(TlsListener::accept)
- 현상: TLS 핸드셰이크가 `accept()` 내부에서 동기 완료, axum::serve가 accept를 직렬 호출 — 핸드셰이크를 완료하지 않는 연결 하나가 accept 루프 전체를 차단.
- 영향: 공격자가 느린/좀비 TCP 연결을 대량 생성하면 서비스가 새 연결 수용을 완전히 중단(gRPC측 tonic은 연결마다 핸드셰이크를 spawn하므로 영향 없음).
- 제안: accept 후 `tokio::spawn`으로 핸드셰이크 + `tokio::time::timeout(10s)` 추가, 실패 시 연결 종료.

### S2 [중] OAuth2 인트로스펙션 캐시 무한 성장 → 메모리 DoS
- 위치: `ecat-auth/src/oauth2.rs:45,84-92`
- 현상: `HashMap<String,(String,Instant)>`가 token을 키로, TTL은 신선도만 제어, 용량 상한 없음, 축출 없음.
- 영향: 대량의 고유 token 요청이 메모리를 무한 성장(miss마다 상류 인트로스펙션도 트리거).
- 제안: 용량 상한(예: 10k) + 주기적 정리, 또는 용량·TTL 축출이 있는 moka/LRU로 교체.

### S3 [저-중] ecat-data-s3가 구버전 rust-s3 0.35.1 사용(hyper 0.14 + native-tls/openssl)
- 위치: `ecat-data-s3/Cargo.toml` → rust-s3 0.35.1
- 현상: S3 클라이언트가 독립적으로 hyper-tls/openssl 스택 사용, ecat-tls::TlsClientConfig(사용자 정의 CA, 클라이언트 인증서, skip_verify)가 S3에 무효; TLS 설정 면 불일치.
- 영향: 기업 환경의 S3 사설 CA/mTLS를 설정할 수 없음; 2023년 이후 유지보수 느림.
- 제안: rust-s3 업그레이드 평가 또는 통일된 reqwest/rustls 클라이언트로 교체.

### S4 [저] JWT 기본 검증에 iss/aud 미포함
- 위치: `ecat-auth/src/jwt.rs:125` — `Validation::new(HS256)`이 서명+exp만 검사.
- 영향: HS256 공유 키 환경에서 한 서비스의 token이 다른 서비스에 수용될 수 있음(발행자 격리 없음).
- 제안: 문서에서 프로덕션 설정에 issuer/audience 요구를 명시; 또는 기본 iss 검증 엔트리 추가.

### S5 [저] TlsClientConfig.skip_verify만으로 is_enabled()가 참이 됨
- 위치: `ecat-tls/src/lib.rs:23-29`
- 현상: `skip_verify: true`만 설정해도 TLS가 「활성화」로 간주되고 인증서를 검증하지 않아 검증이 조용히 꺼짐.
- 제안: skip_verify와 ca_cert 상호 배타 검증, 또는 명시적 이중 확인 요구.

## 성능과 리소스

### P1 [저] OAuth2 캐시 히트 경로가 요청마다 JSON 역직렬화
- 위치: `ecat-auth/src/oauth2.rs:87` — 캐시에 직렬화 문자열 저장, 히트 후에도 `serde_json::from_str` 실행.
- 제안: 캐시에 `AuthClaims` 구조체를 직접 저장해 요청마다 parse 생략.

### P2 [저] ecat-bench에 예열과 정상 상태 판단 없음
- 위치: `ecat-bench/src/lib.rs:run_bench` — 직접 타이밍, warmup 없음, 콜드 스타트/연결 풀 최초 할당이 p99에 혼입.
- 제안: 예열 라운드와 정상 상태 수렴 판단 추가로 결과 신뢰성 향상.

### P3 [저] Kafka 소비자 100ms poll + 100ms sleep 직렬
- 위치: `ecat-mq-kafka/src/lib.rs:84-92` — 메시지 엔드투엔드 지연 상한 약 200ms.
- 제안: poll 후 sleep 불필요; 낮은 처리량 시나리오에서는 poll 간격 단축.

## 좋은 관행 확인

- 프로덕션 경로에 unwrap/expect panic 없음(transport/auth/middleware는 테스트에만).
- API key query 매개변수 폴백에 유출 경고 로그 포함; HashMap이 SipHash로 충돌 방어.
- SQL 계층이 호출자 SQL을 그대로 전달(프레임워크 성질), 연결 문자열 user:pass 퍼센트 인코딩 정확.
- Kafka 소비 채널이 가득 차면 폐기 대신 블로킹 백프레셔; rx drop 후 poll 작업 정상 종료.
- config-remote 풀이 타임아웃 포함(5s/30s), 블로킹 쿼리에 인덱스 부재 오류로 바쁜 대기 방지.

---

## 핵심 도메인 정확성 감사(추가, 위 보안/성능 특수 감사와 상호 보완)

감사 방법: 전체 workspace 프로덕션 코드 스캔(unwrap/expect/panic 위치, 조용한 오류 삼킴, 비동기 정지, 동시성 상태) + `cargo test --workspace` 전량 재검증(1차 전부 초록; S1 수정 진행 중 transport-http가 중간 컴파일 경고 발생, 마무리 후 재실행 필요). 커밋된 코드 없음.

### N1 [중] ecat-events 소비 작업 종료 후 handle 누수 → 이벤트 조용한 유실
- 위치: `ecat-events/src/lib.rs:97-101`(소비 루프 89-95줄 `None => break`)
- 현상: mq 스트림이 None을 반환(예: kafka broadcast channel 종료)하거나 작업이 panic하면 소비 루프가 종료되는데 `consumers` map에 JoinHandle이 잔존; 이후 같은 이벤트 타입으로 `subscribe()`를 해도 68줄 `contains_key`가 항상 참이라 소비 작업이 재시작되지 않음 → 해당 타입 이벤트가 영구히 조용히 유실.
- 영향: 원격 이벤트 스트림 중단 후 자가 치유 불가, 복구하려면 프로세스 재시작 필요.
- 제안: 작업 종료 경로에서 map에서 handle 제거(watcher spawn 또는 `handle.is_finished()` 지연 정리).

### N2 [중] ecat-mq-kafka subscribe의 group_id 의미 오류
- 위치: `ecat-mq-kafka/src/lib.rs:71-84`
- a. `group_id` 기본값 None이면 rdkafka `consumer.subscribe()`가 group.id를 요구(librdkafka가 INVALID_ARG 보고), 기본 설정에서 구독이 대부분 바로 실패할 가능성(실기기 검증 필요).
- b. group_id를 설정하면(ecat-events가 이벤트 타입마다 각각 subscribe, 같은 group) Kafka가 같은 group의 다중 소비자 간 파티션을 분할 → 특정 이벤트 타입이 다른 타입의 소비 작업에 떨어져 조용히 폐기(auto.offset.reset=latest이고 커밋 안 함).
- 영향: 이벤트 버스가 kafka 백엔드에서 이벤트 유실.
- 제안: group_id 없으면 무작위 고유 group.id 생성; 또는 소비측에서 assign()으로 파티션 명시 할당; 다중 구독은 독립 group이어야 한다고 문서 명시.

### N3 [저] GrpcServer/WsServer 빈 host 미정규화(D1 수정 불완전)
- 위치: `ecat-transport-grpc/src/lib.rs:52`, `ecat-transport-ws/src/lib.rs:58`
- 현상: `GrpcServer::new(":8000")`의 `addr.parse::<SocketAddr>()`가 AddrParseError 반환(실측 검증 완료); WsServer `TcpListener::bind(":8000")`이 IPv6 와일드카드로 해석되어 IPv6 없는 환경에서 시작 실패. HttpServer는 0.0.0.0 정규화를 이미 적용, 세 server API 동작 불일치.
- 제안: new 내부에서 빈 host 정규화 통일.

### N4 [저] TracingLayer가 trace_id를 주입하지 않음, CHANGELOG 2.3.3 선언과 불일치
- 위치: `ecat-tracing/src/lib.rs:72-84`(span에 service 필드만 포함, 코드 주석이 스스로 제네릭 Req로 헤더를 못 꺼낸다고 인정); `inject_trace_id()`가 매번 새 UUID 생성, 상류 extract의 trace_id를 이어받지 않음.
- 영향: 문서대로 설정한 분산 추적이 서비스 간 연관 불가.
- 제안: span 필드 지연 바인딩 또는 `http::Request<B>` 특수화; inject가 상류 id를 이어받도록 지원.

### N5 [저] ecat-scheduler 작업 panic 시 조용한 정지
- 위치: `ecat-scheduler/src/lib.rs:53-57,83`(`run()`에서 `let _ = handle.await`)
- 현상: 예약 작업이 panic하면 작업이 사망, 재시작 없음, 로그 없음; `run()`이 JoinHandle 오류를 폐기.
- 제안: panic 포착 로그 + 선택적 재시작 정책.

### N6 [저] 프로덕션 코드 잔여 unwrap(중독/panic 경로)
- `ecat-events/src/lib.rs:68,98` std `Mutex::lock().unwrap()`(중독 시 panic); `ecat-versioning/src/lib.rs:86` Response builder unwrap(실패 불가지만 panic 경로); `ecat-mq/src/lib.rs:110` expect는 is_none 가드로 보호됨(안전).
- 제안: events 두 곳을 `unwrap_or_else(|e| e.into_inner())`로 변경.

### N7 [정보] WsServer::stop()이 업그레이드된 WebSocket 연결을 대기하지 않음
- 위치: `ecat-transport-ws/src/lib.rs:63-87`
- axum on_upgrade 연결이 독립 작업으로 실행되어 graceful shutdown이 커버하지 않음; 긴 연결 핸들러가 stop() 후에도 잔류, 프로세스 종료가 깨끗하지 않음(App::stop 의미론 불완전).

### N8 [정보] 테스트 0 crate: ecat-data / ecat-lock / ecat-protos
- 전부 trait/정의형 crate; 기본 메서드가 fail-loud(오류 반환, 조용하지 않음)임을 검증했지만 trait 계약(Transaction drop 롤백 의미론, 락 token 검증)에 단위 테스트가 전혀 없음.
- 제안: RdbmsError/Transaction과 DistributedLock 의미론에 최소 단위 테스트 보강.

### N9 [정보] graphql 매개변수와 중첩 필드가 여전히 폐기됨
- `ecat-graphql/src/lib.rs` execute가 `variables`만 resolver에 전달, `{ hello(name: "x") }`의 필드 매개변수, 중첩 selection을 전부 전달하지 않음; README에 이 제한 미기재(구 보고서 L8이 문서화를 요구했지만 2.3.3 재작성 후에도 미보완).

### N10 [정보] circuit-breaker가 전송 계층 오류만 집계
- `ecat-circuit-breaker/src/lib.rs:203-209`가 inner Err만 실패로 기록, HTTP 5xx는 성공으로 간주 → 서비스 불가(5xx 폭풍)에 대한 차단이 무효; 문서 미기재.

**검증 상태**: 1차 `cargo test --workspace` 전부 초록(doc-tests 포함, 끝부분 출력에 실패 없음); S1 수정 agent 편집 중 transport-http에 컴파일 오류와 경고 2곳 발생(unused import `ensure_crypto_provider`, `shutdown_tx` 미읽음) — 중간 상태로, S1 마무리 후 테스트와 `clippy --all-targets -D warnings` 전량 재실행 필요.

---

## 3차: 동적 검증 + CVE 재확인 + panic 면(특수, 2026-08-14)

### CVE 재확인(신규 발견, 심각도순)

1. **[중] rustls-webpki 0.102.8이 의존성 트리에 잔존**(RUSTSEC-2026-0049/0098/0099/0104: CRL distributionPoint 우회, URI/wildcard name-constraints, 수정판 0.103.10). 주 체인은 0.103.13(rustls 0.23.43 경유, 안전); 0.102.8은 async-nats 0.38.0 / rumqttc 0.25.1로 유입되어 NATS/MQTT TLS 클라이언트 체인을 커버. 상류가 rustls 0.23으로 마이그레이션하지 않아 수정 버전 없음 — 통제된 리스크, 주석 추적 제안.
2. **[중-저] rdkafka 0.36.2 내장 librdkafka가 cJSON 1.7.14 탑재**(CVE-2023-53154 및 cJSON 시리즈; CVE-2025-57052는 CVSS 9.8로 표시되지만 영향 파일 cJSON_utils.c가 librdkafka에서 사용되지 않아 적용성 불명). 상류 수정은 librdkafka 2.10+(2026-03 PR #5346). ecat-mq-kafka가 정적 링크, librdkafka-sys 패키징 버전 확인 후 업그레이드 추적 필요.
3. **[저] rustls-pemfile 2.2.0 미유지보수**(RUSTSEC-2025-0134) — ecat-transport-http 시작 시 로컬 파일만 파싱, 공격자 입력 아님.
4. **[저] rsa 0.9.10**(RUSTSEC-2023-0071 Marvin 타이밍 부채널) — sqlx-mysql TLS로 유입, MySQL + RSA 키 교환 시나리오에서만 관련.
5. async-nats 0.38.0이 RUSTSEC-2023-0027(CN 검증 우회) 수정선보다 높음, 문제 없음.

### 동적 검증(examples/helloworld, debug 빌드, 임시 포트 18080, 정리 완료)

- /health 200, /(JSON 직렬화) 200(27B), 404 정상; Logging 미들웨어가 요청을 정상 기록.
- **/metrics가 마운트되어 있지만 200 + 빈 body(0바이트) 반환**: 지표 등록이 없으면 아무 출력도 없어 모니터링측에서 「정상/지표 없음」을 구분할 수 없음. 빈 registry에 주석 줄 또는 503 출력 제안.
- 변형 요청(헤더에 0x01/0x02 포함) → 400 Bad Request, 서비스 생존, 이후 /health도 여전히 200, panic 없음.
- TLS/mTLS 경로와 차단기/레이트 리밋 미들웨어: ecat-transport-http/grpc, ecat-middleware 테스트가 커버(mTLS 경쟁 수정 후 전부 초록, 익명/잘못된 클라이언트 인증서 거부 케이스 통과).

### bench 기준선

- ecat-bench에 [[bench]]/bin 타깃 없음, cargo bench 엔트리 없음; run_bench_with_warmup가 예열 포함(P2 수정 반영), harness 테스트 전부 초록.
- 실측은 debug 빌드 smoke: / 약 1.3ms, /health 약 1.8ms(curl 프로세스 오버헤드 포함, 기준선 의미 없음). release 빌드 + wrk/hey 부하 테스트로 실제 기준선 제안.

### panic 면 재확인(전체 workspace, 테스트 모듈 제외)

- unwrap/expect/panic 총 31곳, 전부 저위험: Response::builder().body().unwrap()(jwt/apikey/oauth2의 실패 불가 분기), 락 중독 폴백(etcd/testing), clickhouse serde_json::to_string().unwrap()(극단 NaN/inf 입력 시 이론적 panic).
- **1곳 주의**: `ecat-transport-http/src/tls_listener.rs:234` — 백그라운드 accept 루프가 비정상 종료 시 `accept()` 내부에서 panic!, 서비스 스레드 사망(트리거 조건 까다로움: 리스너 치명적 오류뿐), 오류 반환 + 로그로 완화 제안.
