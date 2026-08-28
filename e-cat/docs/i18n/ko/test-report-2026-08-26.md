# 테스트 보고서 — 2026-08-26

전면 단위 테스트 보완(51 crate 전 커버), 4개 그룹의 시니어 Rust 테스트 엔지니어 병렬 진행.

## 총람

| 그룹 | crates | 기존 | 추가 | 현재 | 게이트 |
|---|---|---|---|---|---|
| core/프레임워크 | 12 | 102 | +40 | 142 | ✅ test 전부 초록 + clippy 0 경고 |
| data | 14 | 87 | +66 | 153 | ✅ 동일 |
| mq/transport | 12 | 82 | +54 | 136 | ✅ 동일 |
| app 애플리케이션 계층 | 13 | ~178 | +46 | ~224 | ✅ 동일 |
| **합계** | **51** | **~449** | **+206** | **~655** | ✅ |

주: 애플리케이션 계층 기존 수는 ecat-auth 24 / ecat-graphql 35 / ecat-bench 11 / ecat-scheduler 6 / ecat-events 7 / ecat-cli 12 / ecat-middleware 34 / ecat-circuit-breaker 10 / ecat-health 4 / ecat-client 7 / ecat-security 12 / ecat-versioning 4 / ecat 4 포함. 각 crate 독립 `cargo test -p` + `cargo clippy -p --all-targets -- -D warnings` 모두 통과, CARGO_TARGET_DIR 격리 병렬 실행.

## crate별 상세

### core/프레임워크 그룹(test-core, +40)

| crate | 기존→신규 | 커버 요점 |
|---|---|---|
| ecat-protos | 4→8 | ErrorCode 전 열거형 proto 대조; 잘린 buffer decode; 빈 buffer 기본 메시지; metadata roundtrip |
| ecat-errors | 4→9 | http_status 전 매핑(409/429/500); from_status; 미매핑→Internal; cause source() |
| ecat-metadata | 9→12 | HTTP header trace_id 추출; key 소문자화; 빈 header map |
| ecat-encoding | 18→22 | NaN→null(serde_json 기본, 문서화됨); 빈 바이트 decode; CodecBox 잘못된 JSON; proto roundtrip |
| ecat-lock | 7→9 | 미보유 락 release 오류; 빈 key |
| ecat-logging | 1→1 | 호환 shim이 panic하지 않음 |
| ecat-tracing | 9→12 | 비 UTF-8 trace 헤더 건너뜀; canonical 헤더; 응답 투과 |
| ecat-tls | 7→12 | basic_auth 단일/이중 필드; ca 파일 부재; is_enabled; 기본 클라이언트 |
| ecat-config | 14→26 | env 접두사 필터 + 타입 파싱 경계(hex/빈 문자열/-0/1e3); 다중 source 병합 덮어쓰기; obfs 오류 경로; 파일 부재/잘못된 YAML |
| ecat-config-remote | 6→9 | ConsulKvEntry 경계; X-Consul-Index 부재 오류; 중첩 key |
| ecat-openapi | 4→11 | components/schema_ref; 중복 덮어쓰기; 기본 200; tags |
| ecat-metrics | 8→11 | 등록된 지표 텍스트; 404/405 |

### data 그룹(test-data, +66)

| crate | 기존→신규 | 커버 요점 |
|---|---|---|
| ecat-data | 12→14 | 검색 문법 파싱 |
| ecat-data-sqlx | 7→14 | 인메모리 SQLite 엔드투엔드; 매개변수 바인딩 전 타입; Blob→base64; config |
| ecat-data-redis | 6→12 | redis:///rediss:// URL 구성; auth; config 오류 경로 |
| ecat-data-opensearch | 4→10 | mock HTTP: percent-encode, Basic auth, 오류 투과 |
| ecat-data-elasticsearch | 6→11 | 동일 |
| ecat-data-influxdb | 5→10 | line protocol 이스케이프; Token 헤더; 오류 투과 |
| ecat-data-clickhouse | 12→22 | 테이블 생성 SQL; JSONEachRow; 쓰기 행 수; 그룹핑 |
| ecat-data-memcached | 4→8 | TTL 초→밀리초; flag 패킹 |
| ecat-data-nebulagraph | 6→7 | config 파싱 |
| ecat-data-arangodb | 5→7 | config/URL |
| ecat-data-iotdb | 5→10 | mock HTTP: session 경로 매개변수 |
| ecat-data-questdb | 4→9 | line protocol; 트랜잭션 미지원 |
| ecat-data-tdengine | 6→11 | INSERT 생성; 100건 배치 분할 |
| ecat-data-mongodb | 5→8 | bson 왕복; URI |

### mq/transport/registry 그룹(test-mq, +54)

| crate | 기존→신규 | 커버 요점 |
|---|---|---|
| ecat-mq | 5→9 | 꽉 찬 버퍼 지연 오류 프레임; 전 drop 스트림 종료; 다중 구독자; 구독자 없는 publish |
| ecat-mq-kafka | 12→14 | config 기본값; SASL 필드 독립 적용 |
| ecat-mq-rabbitmq | 2→5 | exchange 기본값; url 오류 경로 |
| ecat-mq-mqtt | 5→9 | cert/key 짝 검증; 파일 부재; 포트 기본 1883/8883; 잘못된 포트 폴백 |
| ecat-mq-nats | 6→9 | 평문 기본값; ca/cert 부재 오류 경로 |
| ecat-transport | 4→7 | TlsConfig 기본값/with_client_auth; normalize_addr 경계 |
| ecat-transport-http | 17→20 | 통합 테스트: stop 무연산, 포트 점유 실패, 실제 송수신 |
| ecat-transport-grpc | 7→13 | TLS 파일 부재; plaintext 수명 주기; mTLS 거부 |
| ecat-transport-ws | 4→8 | handler 없음 실패; 포트 점유; RFC 6455 masked 프레임 에코 |
| ecat-registry | 5→8 | 다중 인스턴스 discover; drop 자동 등록 해제; builder 기본값 |
| ecat-registry-consul | 10→24 | percent-encode; 등록 변형; 오류 응답; X-Consul-Token; agent/services 파싱; node 폴백 |
| ecat-registry-etcd | 5→10 | discover 잘못된 값 건너뜀; kv 요청 본문; lease grant; keepalive |

### app 애플리케이션 계층 그룹(test-app, +46)

| crate | 기존→신규 | 커버 요점 |
|---|---|---|
| ecat-auth | 20→46 | oauth2 캐시 화이트리스트/SHA-256 key/FIFO 축출; apikey 삼태; jwt iss/aud 강제; 만료/잘못된 서명 |
| ecat-health | 4→8 | readiness 집계(전 ok/임의 fail/빈 레지스트리); liveness |
| ecat-versioning | 4→7 | path 전략 라우팅; extract_version 경계 |
| ecat-security | 12→20 | header 계층 엔드투엔드; 공격 차단 JSON 형태 |
| ecat-middleware | 34→37 | MemoryStore 창구 만료; 내부 panic→Err |
| ecat-circuit-breaker | 10→12 | half-open 프로브 소진; classify 폴백 |
| ecat-client | 7→10 | grpc 잘못된 엔드포인트 오류 시 네트워크 미접속 |
| ecat-graphql | 35→35 | 기존 커버리지 충분, 공백 없음 |
| ecat-scheduler / ecat-bench / ecat-events / ecat-cli / ecat | 기존 커버리지 충분 | 공백 없음 |

## 발견된 결함

| 레벨 | 위치 | 설명 | 상태 |
|---|---|---|---|
| P1 | ecat-events/Cargo.toml | dev-dependencies에 tokio macros/rt/time features 부재, 해당 crate 테스트 타깃 단독 컴파일 시 필수 실패(workspace 전체 빌드는 feature 병합으로 가려짐) | ✅ 수정됨(features + 주석 추가) |
| P2 | ecat-security src/lib.rs:118-127 | URI 퍼센트 인코딩 SQLi(`?q=SELECT%20*%20...`)가 header 계층 스캔을 우회 가능(검출기가 리터럴 공백을 요구, 원시 URI를 먼저 디코딩하지 않음); 본문 스캔은 영향 없음 | ⏳ 수정 대기 |
| P3 | ecat-data-sqlx | `connect()/from_config()`가 AnyPool을 사용하지만 드라이버 미설치, sqlx 0.8.6 첫 연결 시 "No drivers installed" panic | ⏳ 수정 대기 |
| P3 | ecat-data-influxdb | 문자열 field가 공백을 이스케이프(`\ `), line protocol 규격은 `"`와 `\`만 이스케이프하면 됨; tag/field 순서 비결정적 | ⏳ 수정 대기 |
| P3 | ecat-data-clickhouse | 테이블 생성 캐시가 영구 유효, 외부 drop/테이블 변경 후 CREATE 재시도 안 함 | ⏳ 수정 대기 |
| P3 | ecat-circuit-breaker | half_open_probes 상한이 순차 프로브에서 도달 불가(동시 in-flight일 때만 도달), 화이트박스 테스트로 커버됨 | ℹ️ 알려짐, 결함 아님 |
| P3 | ecat-health | `with_check`가 blocking_write() 사용, async 컨텍스트에서 호출 시 panic; 현재 동기 컨텍스트에서만 사용 가능 | ℹ️ 알려짐, API 제한 |

## 건너뛴 모듈(통합 환경 필요, mock 안 함)

- 실제 broker 왕복: kafka/rabbitmq/mqtt/nats publish-subscribe(설정과 오류 경로는 커버됨)
- 실제 클러스터: consul/etcd 등록-발견 수명 주기(axum mock이 요청 형태 커버)
- 실제 데이터베이스: redis/memcached 연산, mongod, influxdb 서버측 검증, sqlx postgres/mysql 드라이버, nebulagraph/arangodb API
- 실제 외부 서비스: OAuth2 introspection(로컬 mock 커버), gRPC/HTTP 왕복(로컬 mock이 302 비팔로우 커버)
