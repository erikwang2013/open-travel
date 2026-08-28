<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat 코드 리뷰 보고서 (2차)

**날짜**: 2026-07-29  
**브랜치**: main  
**프로젝트**: e-cat (Rust workspace, 17개 crate)

---

## 1. 리뷰 개요

1차 clippy 수정과 테스트 보완을 바탕으로, 이번 차수는 심층 코드 로직 리뷰를 진행했습니다. 런타임 정확성, 동시성 안전성, API 의미론 일관성에 중점을 두었습니다. 총 32개 소스 파일을 리뷰했습니다.

### 검증 베이스라인

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

---

## 2. 발견된 Bug 및 수정

### Bug 1: [중요] TracingLayer span 가드 수명주기 오류

- **파일**: `ecat-middleware/src/tracing.rs:37`
- **심각도**: **높음**
- **영향**: TracingLayer를 통과하는 모든 요청이 tracing span에 포함되지 않음

**근본 원인 분석**:

```rust
// 수정 전
fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let _guard = span.enter();  // guard는 call() 반환 시 drop
    let fut = self.inner.call(req);
    Box::pin(fut)               // future는 이후 poll 시 실행
}
```

`span.enter()`가 반환하는 guard는 현재 동기 컨텍스트에서만 span을 활성 상태로 유지합니다. `call()`이 반환하는 것은 아직 poll되지 않은 future이며, 실제 비동기 실행은 이후 poll 단계에서 발생합니다 — 이때 guard는 이미 drop되어 span이 적용되지 않습니다. TracingLayer를 통과하는 모든 요청이 tracing 출력에 나타나지 않습니다.

**수정**:

```rust
// 수정 후
use tracing::Instrument;

fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let fut = self.inner.call(req);
    Box::pin(fut.instrument(span))  // span을 future 수명주기에 부착
}
```

`tracing::Instrument::instrument()`로 span을 future에 부착하여, span이 future의 전체 poll 수명주기 동안 활성 상태로 유지되도록 보장합니다.

---

### Bug 2: [중요] LifecycleHook 클로저 구현 결함 — on_stop이 영원히 실행되지 않음

- **파일**: `ecat/src/hook.rs:14-23`, `ecat/src/lib.rs:11-16`
- **심각도**: **높음**
- **영향**: `.on_stop()`으로 등록한 클로저 훅이 shutdown 시 아무것도 하지 않음

**근본 원인 분석**:

기존 설계에서 `on_start()`와 `on_stop()` 메서드는 훅을 같은 `lifecycle_hooks` Vec에 넣습니다. `run()` 시 모든 훅이 순서대로 `on_start()`를 호출하고, shutdown 시 모든 훅이 순서대로 `on_stop()`을 호출합니다.

문제는 `LifecycleHook` trait이 클로저 `Fn() -> Fut`에 대한 blanket impl에서 **`on_start()`만 구현하고, `on_stop()`은 trait 기본 구현(no-op)을 사용**한다는 점입니다.

즉, 사용자가 클로저 문법으로 `.on_stop(|| async { ... })`을 쓰면 클로저가 hooks 목록에 추가되지만, shutdown 시에는 기본 빈 `on_stop()`만 실행되어 사용자 로직이 영원히 실행되지 않습니다.

**수정 (두 부분)**:

1. **start_hooks와 stop_hooks 분리** (`ecat/src/lib.rs`):

```rust
// App 구조체 — 두 개의 독립 Vec
pub struct App {
    start_hooks: Vec<Box<dyn LifecycleHook>>,
    stop_hooks: Vec<Box<dyn LifecycleHook>>,
    // ...
}

// on_start() → start_hooks, on_stop() → stop_hooks
pub fn on_start(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.start_hooks.push(Box::new(hook));
    self
}
pub fn on_stop(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.stop_hooks.push(Box::new(hook));
    self
}
```

2. **클로저 blanket impl 보완** (`ecat/src/hook.rs`):

```rust
impl<F, Fut> LifecycleHook for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Result<...>> + Send,
{
    async fn on_start(&self) -> ... { (self)().await }
    async fn on_stop(&self) -> ...  { (self)().await }  // 추가
}
```

이제 클로저가 `on_start`와 `on_stop`을 동시에 구현하며, 분리된 Vec과 결합해 각 훅이 올바른 수명주기 단계에서만 호출됩니다.

---

### Bug 3: [중간] SqlxClient Row 값 타입 추출 우선순위 오류

- **파일**: `ecat-data-sqlx/src/lib.rs:53-68`
- **심각도**: 중간
- **영향**: 데이터베이스의 정수·부동소수점 값이 숫자가 아닌 JSON 문자열로 추출됨

**근본 원인 분석**:

`try_get::<String>()`가 첫 번째로 시도됩니다. 대부분의 데이터베이스 드라이버는 숫자 컬럼에 대해 `try_get::<String>()`을 성공시킬 수 있어(암묵 변환), 정수 값 `42`가 `42`가 아닌 `"42"`로 추출됩니다.

**수정**: `try_get` 시도 순서를 `i64 → f64 → String → Null`로 조정하여 숫자 타입을 우선 보존합니다.

---

## 3. 기타 리뷰 발견 (미수정 / 알려진 제한)

| 카테고리 | 파일 | 설명 | 제안 |
|------|------|------|------|
| 기능 미완성 | `ecat-transport-http/src/lib.rs:30` | `axum::serve().await`가 블로킹되어 영원히 반환하지 않음, `stop()`은 무연산 | graceful shutdown 구현 |
| 기능 미완성 | `ecat-transport-grpc/src/lib.rs:29` | 동일 | graceful shutdown 구현 |
| 기능 미완성 | `ecat-data-sqlx/src/lib.rs:79` | `transaction()`이 미구현 오류 반환 | 트랜잭션 지원 구현 |
| 코드 스타일 | `ecat-middleware/src/logging.rs:42` | `elapsed.as_millis() as u64` u128→u64 이론적 절단 | 실제 영향 없음 |
| 테스트 부재 | `ecat-middleware/` | 4개 Tower Service에 단위 테스트 없음 | 통합 테스트 필요 |
| 테스트 부재 | `ecat-data/` | 순수 trait 정의 | 현재 수용 가능 |
| RwLock 블로킹 | `ecat-registry/src/memory.rs` | 동기 RwLock이 비동기 컨텍스트에서 블로킹 가능 | tokio::sync::RwLock 검토 |

---

## 4. 테스트 결과

```
cargo test → 60 passed, 0 failed

crate별 분포:
  ecat                  4   (Builder/기본값/생명주기 hook)
  ecat-config           9   (env parse ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (HTTP 매핑/gRPC 변환/metadata/Display)
  ecat-logging          1   (init 스모크)
  ecat-metadata         9   (저장/From HeaderMap/From MetadataMap/이터레이터)
  ecat-metrics          2   (싱글턴/text panic 없음)
  ecat-registry         5   (등록/디스커버리/등록 해제/목록/필터)
  ecat-transport       11   (Context/Request/Response/Server trait)
  기타 8 crate          0   (순수 trait/코드 생성/통합 테스트 필요/순수 출력)
```

---

## 5. 수정 파일 목록

| 파일 | 변경 유형 | 변경 설명 |
|------|----------|----------|
| `ecat/src/lib.rs` | Bug 수정 | App start_hooks/stop_hooks 분리; AppBuilder 대응 업데이트; 테스트 적응 |
| `ecat/src/hook.rs` | Bug 수정 | 클로저 blanket impl에 on_stop() 구현 보완 |
| `ecat-middleware/src/tracing.rs` | Bug 수정 | span 가드 → `fut.instrument(span)` |
| `ecat-data-sqlx/src/lib.rs` | Bug 수정 | Row 값 추출 순서 i64→f64→String→Null |

---

## 6. 요약

이번 차수 리뷰에서 고심각도 런타임 Bug 2개와 중간 심각도 데이터 정확성 문제 1개를 발견했습니다:

1. **TracingLayer span 무효** — 모든 요청의 관측성에 영향
2. **LifecycleHook on_stop 미실행** — 모든 shutdown 로직의 정확성에 영향
3. **Row 숫자 타입 손실** — 데이터베이스 쿼리 결과의 타입 정확성에 영향

세 문제 모두 수정했으며, 수정 후 전체 60개 테스트가 통과하고 컴파일 오류·경고가 0입니다.

### 후속 제안

- HTTP/gRPC server에 graceful shutdown 구현
- `ecat-middleware`에 통합 테스트 추가 (mock Service + span/타임아웃/복구 동작 검증)
- `ecat-data-sqlx`에 통합 테스트 추가 (SQLite 인메모리 데이터베이스 사용)
- `ecat-registry/memory.rs`의 동기 RwLock을 `tokio::sync::RwLock`으로 교체
