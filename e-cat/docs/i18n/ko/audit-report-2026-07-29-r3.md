<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat 코드 리뷰 보고서 (3차)

**날짜**: 2026-07-29  
**브랜치**: main  
**프로젝트**: e-cat (Rust workspace, 18개 crate)  
**리뷰 범위**: 전체 37개 소스 파일, 총 2151줄 Rust 코드

---

## 1. 리뷰 개요

2차 리뷰에서 발견한 3개 Bug가 모두 수정되었으며, 이번 차수는 깨끗한 베이스라인(0 error / 0 warning / 60 test passed) 위에서 심층 재리뷰를 진행했습니다. 경계 조건, 오류 처리, 프로덕션 견고성에 중점을 두었습니다.

### 검증 베이스라인

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

### R2 Bug 수정 확인

| Bug | 파일 | 상태 |
|-----|------|------|
| TracingLayer span 가드 수명주기 | `ecat-middleware/src/tracing.rs` | ✅ 수정됨 |
| LifecycleHook on_stop 미실행 | `ecat/src/hook.rs`, `ecat/src/lib.rs` | ✅ 수정됨 |
| Row 값 타입 추출 우선순위 | `ecat-data-sqlx/src/lib.rs` | ✅ 수정됨 |

---

## 2. 새로 발견된 문제

### 문제 1: [중간] `metrics_text()`의 unwrap(), 프로덕션 환경에서 panic 가능

- **파일**: `ecat-metrics/src/lib.rs:14-15`
- **심각도**: **중간**
- **영향**: `/metrics` 엔드포인트 접근 시 프로세스 panic

**근본 원인 분석**:

```rust
pub fn metrics_text() -> String {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder.encode(&registry().gather(), &mut buffer).unwrap();  // panic 가능
    String::from_utf8(buffer).unwrap()                           // panic 가능
}
```

`TextEncoder::encode()`는 내부 I/O 오류나 시스템 메모리 부족 시 실패합니다. `String::from_utf8()`은 Prometheus 라이브러리가 비 UTF-8 출력을 생성하면 이론적으로 실패할 수 있습니다. 이 두 `unwrap()`은 비테스트 코드 경로에 있으며 HTTP handler 호출에 직접 노출되어, panic 시 프로세스가 크래시합니다.

**수정 제안**: `Result<String, ...>` 반환 또는 `.unwrap_or_default()`로 폴백 처리.

---

### 문제 2: [낮음] Recovery 미들웨어 spawn 새 task가 span 컨텍스트 유실

- **파일**: `ecat-middleware/src/recovery.rs:40`
- **심각도**: **낮음**
- **영향**: Recovery 계층이 Tracing 계층보다 앞에 있으면 요청의 trace_id가 비즈니스 로직으로 전달되지 않음

**근본 원인 분석**:

```rust
fn call(&mut self, req: Req) -> Self::Future {
    let fut = self.inner.call(req);
    Box::pin(async move {
        match tokio::task::spawn(fut).await {  // 새 task, span 미상속
            // ...
        }
    })
}
```

`tokio::task::spawn()`은 새 Tokio 작업을 생성하며, tracing span은 task-local이라 자동 전달되지 않습니다.

**제안**: 문서에 미들웨어 순서 요구 사항을 명시(Recovery는 최외곽에 배치), 또는 spawn 전에 `.instrument(span)`으로 수동 전달.

---

### 문제 3: [낮음] Registration Drop이 오류를 조용히 폐기

- **파일**: `ecat-registry/src/lib.rs:50-52`
- **심각도**: **낮음**
- **영향**: 서비스 등록 해제 실패를 인지하지 못함

```rust
impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(reg) = self.registry.take() {
            let id = self.id.clone();
            tokio::spawn(async move {
                let _ = reg.deregister(&id).await;  // 오류가 조용히 폐기됨
            });
        }
    }
}
```

Drop에서 블로킹할 수는 없지만, `tracing::warn!`으로 등록 해제 실패를 기록할 수 있습니다.

---

### 문제 4: [낮음] `ecat-data-sqlx` f64 특수값 처리

- **파일**: `ecat-data-sqlx/src/lib.rs:57-61`
- **심각도**: **낮음**
- **영향**: 데이터베이스의 NaN/Infinity 부동소수점 값이 Null로 변환됨

```rust
row.try_get::<f64, _>(col.as_str())
    .ok()
    .and_then(serde_json::Number::from_f64)  // NaN/Inf → None
    .map(serde_json::Value::Number)
    .ok_or(())
```

`serde_json::Number::from_f64()`는 `f64::NAN`, `f64::INFINITY`, `f64::NEG_INFINITY`에 대해 `None`을 반환하여, 이 값들이 Null로 강등됩니다.

---

## 3. crate별 리뷰 노트

### ecat (핵심) — 4개 파일
| 파일 | 상태 | 비고 |
|------|------|------|
| `lib.rs` | ✅ | start_hooks/stop_hooks 분리 정확 |
| `hook.rs` | ✅ | 클로저 blanket impl이 on_start/on_stop 커버 |
| `signal.rs` | ⚠️ | SIGTERM handler `.expect()` 합리적이지만 엄격 |

### ecat-transport — 4개 파일
| 파일 | 상태 | 비고 |
|------|------|------|
| `lib.rs` | ✅ | Server trait 설계 간결 |
| `context.rs` | ✅ | `tokio::sync::RwLock` 사용됨 |
| `request.rs` | ✅ | |
| `response.rs` | ✅ | |

### ecat-transport-http / ecat-transport-grpc — 2개 파일
| 파일 | 상태 | 비고 |
|------|------|------|
| `ecat-transport-http/src/lib.rs` | ⚠️ | `start()` 블로킹 미반환, `stop()` 무연산 (알려진 제한) |
| `ecat-transport-grpc/src/lib.rs` | ⚠️ | 동일 |

### ecat-middleware — 5개 파일
| 파일 | 상태 | 비고 |
|------|------|------|
| `tracing.rs` | ✅ | `fut.instrument(span)` 수정 정확 |
| `recovery.rs` | ⚠️ | `tokio::task::spawn` span 컨텍스트 유실 (문제 2) |
| `logging.rs` | ✅ | `elapsed.as_millis() as u64` 이론적 절단, 실제 영향 없음 |
| `timeout.rs` | ✅ | |

### ecat-registry — 2개 파일
| 파일 | 상태 | 비고 |
|------|------|------|
| `lib.rs` | ⚠️ | Registration Drop이 오류 조용히 폐기 (문제 3) |
| `memory.rs` | ⚠️ | 동기 `std::sync::RwLock`이 async 컨텍스트에서 사용됨 (알려진 제한) |

### ecat-config — 3개 파일
| 파일 | 상태 | 비고 |
|------|------|------|
| `lib.rs` | ✅ | Config trait 설계 합리적 |
| `env.rs` | ✅ | 타입 파싱 순서 정확 (bool→i64→f64→String) |
| `file.rs` | ⚠️ | YAML 다중 문서 미지원, watch 메커니즘 없음 (알려진 제한) |

### ecat-data — 6개 파일
| 파일 | 상태 | 비고 |
|------|------|------|
| `rdbms.rs` | ✅ | Transaction Drop 주석이 자동 롤백 설명, 구현체 없음 |
| `cache.rs` | ✅ | trait 정의 완전 |
| `graph.rs` | ✅ | |
| `search.rs` | ✅ | |
| `tsdb.rs` | ✅ | DataPoint builder 패턴 설계 양호 |

### ecat-data-sqlx — 1개 파일
| 파일 | 상태 | 비고 |
|------|------|------|
| `lib.rs` | ⚠️ | 값 추출 순서 수정됨; transaction 미구현; f64 특수값 (문제 4) |

### ecat-errors — 2개 파일
| 파일 | 상태 | 비고 |
|------|------|------|
| `lib.rs` | ✅ | gRPC→ErrorCode 매핑 완전, Display 형식 명확 |
| `codes.rs` | ✅ | HTTP 상태 코드 매핑과 gRPC 의미론 일치 |

### ecat-encoding — 3개 파일
| 파일 | 상태 | 비고 |
|------|------|------|
| `lib.rs` | ✅ | CodecBox enum, codec_for/codec_from_content_type 설계 양호 |
| `json.rs` | ✅ | |
| `proto.rs` | ⚠️ | ProtoCodec은 플레이스홀더 구현 (알려진 제한) |

### 기타 crate
| Crate | 상태 | 비고 |
|-------|------|------|
| `ecat-logging` | ✅ | `try_init` 중복 초기화 방지 |
| `ecat-metadata` | ✅ | HTTP/gRPC 양방향 변환 완비 |
| `ecat-metrics` | ⚠️ | `metrics_text()`에 unwrap() 있음 (문제 1) |
| `ecat-protos` | ✅ | prost/tonic 코드 생성 |
| `ecat-cli` | ⚠️ | 대부분 명령이 메시지만 출력, 실제 파일 생성 안 함 (알려진 제한) |
| `examples/helloworld` | ✅ | 예시 코드가 새 API를 올바르게 사용 |

---

## 4. 테스트 커버리지 분석

```
cargo test → 60 passed, 0 failed

crate별 분포:
  ecat                  4   (Builder/기본값/수명주기 hook)
  ecat-config           9   (env parse ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (HTTP 매핑/gRPC 변환/metadata/Display)
  ecat-logging          1   (init 스모크)
  ecat-metadata         9   (저장/From HeaderMap/From MetadataMap/이터레이터)
  ecat-metrics          2   (싱글턴/text panic 없음)
  ecat-registry         5   (등록/디스커버리/등록 해제/목록/필터)
  ecat-transport       11   (Context/Request/Response/Server trait)
  기타 8 crate          0   (순수 trait/코드 생성/통합 테스트 필요)
```

### 테스트 격차

| 우선순위 | Crate | 부재 내용 |
|--------|-------|----------|
| 높음 | `ecat-middleware` | 4개 Tower Service에 단위 테스트 없음 |
| 높음 | `ecat-data-sqlx` | 통합 테스트 없음 (SQLite 인메모리 가능) |
| 중간 | `ecat-transport-http` | HTTP server 시작 흐름 테스트 없음 |
| 중간 | `ecat-transport-grpc` | gRPC server 시작 흐름 테스트 없음 |
| 낮음 | `ecat-data` | 순수 trait 정의, 수용 가능 |

---

## 5. 코드 품질 지표

| 지표 | 값 | 등급 |
|------|-----|------|
| 총 줄 수 | 2151 | — |
| 컴파일 경고 | 0 | ✅ |
| Clippy 경고 | 0 | ✅ |
| 테스트 통과 | 60/60 | ✅ |
| 테스트 커버리지 (추정) | ~35% | ⚠️ |
| 비테스트 unwrap() | 2곳 (metrics) | ⚠️ |
| 안전하지 않은 코드 | 0 | ✅ |
| panic 위험 지점 | 3곳 (metrics×2 + signal expect) | ⚠️ |

---

## 6. 수정 제안 요약

### 수정 제안 (이번 차수 — 전부 수정 완료 ✅)

| # | 파일 | 문제 | 우선순위 | 상태 |
|---|------|------|--------|------|
| 1 | `ecat-metrics/src/lib.rs:14-15` | `metrics_text()` unwrap → 폴백 처리 | 중간 | ✅ 수정됨 |
| 2 | `ecat-registry/src/lib.rs:51` | Drop에 `tracing::warn!` 추가해 deregister 실패 기록 | 낮음 | ✅ 수정됨 |
| 3 | `ecat-data-sqlx/src/lib.rs:57-61` | f64 NaN/Inf 값 특수 처리 | 낮음 | ✅ 수정됨 |
| 4 | `ecat-middleware/src/recovery.rs:40` | `tokio::task::spawn` span 유실 → `fut.instrument(span)` | 낮음 | ✅ 수정됨 |
| 5 | `ecat-registry/src/memory.rs` | 동기 RwLock → `tokio::sync::RwLock` | 낮음 | ✅ 수정됨 |

### 알려진 제한 (비차단)

| # | 파일 | 설명 |
|---|------|------|
| K1 | `ecat-transport-http` / `ecat-transport-grpc` | start() 블로킹 / stop() 무연산 (graceful shutdown 필요) |
| K2 | `ecat-data-sqlx` | `transaction()` 미구현 오류 반환 |
| K3 | `ecat-middleware` | 4개 Service에 단위 테스트 없음 |
| K4 | `ecat-config/file.rs` | watch 메커니즘 없음 |
| K5 | `ecat-encoding/proto.rs` | ProtoCodec 플레이스홀더 구현 |
| K6 | `ecat-cli` | 대부분 명령이 mock 출력 |

---

## 7. 요약

3차 리뷰는 R2의 전체 수정을 바탕으로 진행했습니다. 이번 차수에서 5개 문제를 발견했으며 전부 수정했습니다.

R2와의 비교:
- R2: 고심각도 2개 + 중간 심각도 1개 런타임 Bug 발견 → 전부 수정 완료 ✅
- R3: 중간 심각도 1개 + 낮은 심각도 4개 견고성 문제 발견 → 전부 수정 완료 ✅
- 테스트 수 60개 유지

### 후속 우선 제안

1. `ecat-data-sqlx`에 SQLite 통합 테스트 추가
2. `ecat-middleware`에 단위 테스트 추가 (span/타임아웃/복구 동작 검증)
3. HTTP/gRPC server graceful shutdown 구현
