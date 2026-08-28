<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat: отчёт о ревью кода (второй раунд)

**Дата**: 2026-07-29  
**Ветка**: main  
**Проект**: e-cat (Rust workspace, 17 crates)

---

## 1. Обзор ревью

После первого раунда исправлений clippy и дополнения тестов, в этом раунде проведено глубокое ревью логики кода: корректность времени выполнения, конкурентная безопасность, согласованность семантики API. Всего проверено 32 исходных файла.

### Базовая линия проверки

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

---

## 2. Найденные баги и исправления

### Баг 1: [Критичный] Ошибка жизненного цикла guard-а span в TracingLayer

- **Файл**: `ecat-middleware/src/tracing.rs:37`
- **Серьёзность**: **высокая**
- **Влияние**: ни один запрос, проходящий через TracingLayer, не покрывается tracing span-ом

**Анализ корневой причины**:

```rust
// 修复前
fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let _guard = span.enter();  // guard 在 call() 返回时 drop
    let fut = self.inner.call(req);
    Box::pin(fut)               // future 在后续 poll 时才执行
}
```

Guard, возвращаемый `span.enter()`, удерживает span активным только в текущем синхронном контексте. `call()` возвращает ещё не опрошенный future; фактическое асинхронное выполнение происходит на этапе последующего poll — к этому моменту guard уже сброшен, и span не действует. Ни один запрос, проходящий через TracingLayer, не появляется в выводе tracing.

**Исправление**:

```rust
// 修复后
use tracing::Instrument;

fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let fut = self.inner.call(req);
    Box::pin(fut.instrument(span))  // span 附着在 future 生命周期上
}
```

`tracing::Instrument::instrument()` привязывает span к future, гарантируя, что span остаётся активным на протяжении всего жизненного цикла poll future.

---

### Баг 2: [Критичный] Дефект реализации замыканий LifecycleHook — on_stop никогда не выполняется

- **Файл**: `ecat/src/hook.rs:14-23`, `ecat/src/lib.rs:11-16`
- **Серьёзность**: **высокая**
- **Влияние**: hook-замыкание, зарегистрированное через `.on_stop()`, при shutdown не делает ничего

**Анализ корневой причины**:

В исходном дизайне методы `on_start()` и `on_stop()` помещали hook-и в один и тот же Vec `lifecycle_hooks`. В `run()` все hook-и по очереди вызывают `on_start()`, при shutdown все hook-и по очереди вызывают `on_stop()`.

Проблема в blanket impl trait `LifecycleHook` для замыканий `Fn() -> Fut`: **покрыт только `on_start()`, а `on_stop()` использует реализацию trait по умолчанию (no-op)**.

Это значит: при использовании синтаксиса замыканий `.on_stop(|| async { ... })` замыкание хоть и попадает в список hook-ов, но при shutdown выполняется только пустой `on_stop()` по умолчанию — логика пользователя не запускается никогда.

**Исправление (из двух частей)**:

1. **Разделение start_hooks и stop_hooks** (`ecat/src/lib.rs`):

```rust
// App 结构体 — 两个独立的 Vec
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

2. **Дополнение blanket impl для замыканий** (`ecat/src/hook.rs`):

```rust
impl<F, Fut> LifecycleHook for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Result<...>> + Send,
{
    async fn on_start(&self) -> ... { (self)().await }
    async fn on_stop(&self) -> ...  { (self)().await }  // 新增
}
```

Теперь замыкание реализует и `on_start`, и `on_stop`; в сочетании с раздельными Vec каждый hook вызывается только в правильной фазе жизненного цикла.

---

### Баг 3: [Средний] Ошибочный приоритет извлечения типов значений Row в SqlxClient

- **Файл**: `ecat-data-sqlx/src/lib.rs:53-68`
- **Серьёзность**: средняя
- **Влияние**: целочисленные и вещественные значения из БД извлекаются как JSON-строки, а не как числа

**Анализ корневой причины**:

`try_get::<String>()` стоит первым в очереди попыток. Большинство драйверов БД успешно выполняют `try_get::<String>()` для числовых колонок (неявное преобразование), из-за чего целое значение `42` извлекается как `"42"`, а не `42`.

**Исправление**: изменить порядок попыток `try_get` на `i64 → f64 → String → Null`, чтобы в приоритете сохранялись числовые типы.

---

## 3. Прочие находки ревью (не изменено / известные ограничения)

| Категория | Файл | Описание | Рекомендация |
|------|------|------|------|
| Функция не завершена | `ecat-transport-http/src/lib.rs:30` | `axum::serve().await` блокируется и никогда не возвращается, `stop()` — no-op | Реализовать graceful shutdown |
| Функция не завершена | `ecat-transport-grpc/src/lib.rs:29` | То же самое | Реализовать graceful shutdown |
| Функция не завершена | `ecat-data-sqlx/src/lib.rs:79` | `transaction()` возвращает ошибку «не реализовано» | Реализовать поддержку транзакций |
| Стиль кода | `ecat-middleware/src/logging.rs:42` | `elapsed.as_millis() as u64` — теоретическое усечение u128→u64 | На практике без влияния |
| Отсутствие тестов | `ecat-middleware/` | У 4 Tower Service нет юнит-тестов | Нужны интеграционные тесты |
| Отсутствие тестов | `ecat-data/` | Чистые определения trait | Приемлемо на данный момент |
| Блокировка RwLock | `ecat-registry/src/memory.rs` | Синхронный RwLock в асинхронном контексте может блокировать | Рассмотреть замену на tokio::sync::RwLock |

---

## 4. Результаты тестов

```
cargo test → 60 passed, 0 failed

По crates:
  ecat                  4   (Builder/значения по умолчанию/lifecycle hook)
  ecat-config           9   (env parse ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (HTTP-маппинг/gRPC-конвертация/metadata/Display)
  ecat-logging          1   (init smoke)
  ecat-metadata         9   (доступ/From HeaderMap/From MetadataMap/итератор)
  ecat-metrics          2   (singleton/text не паникует)
  ecat-registry         5   (регистрация/discovery/дерегистрация/список/фильтр)
  ecat-transport       11   (Context/Request/Response/trait Server)
  остальные 8 crates    0   (чистый trait/генерация кода/нужны интеграционные тесты/чистая печать)
```

---

## 5. Список изменённых файлов

| Файл | Тип изменения | Описание |
|------|----------|----------|
| `ecat/src/lib.rs` | Исправление бага | App: разделение start_hooks/stop_hooks; соответствующее обновление AppBuilder; адаптация тестов |
| `ecat/src/hook.rs` | Исправление бага | Дополнение blanket impl замыканий реализацией on_stop() |
| `ecat-middleware/src/tracing.rs` | Исправление бага | guard span → `fut.instrument(span)` |
| `ecat-data-sqlx/src/lib.rs` | Исправление бага | Порядок извлечения значений Row: i64→f64→String→Null |

---

## 6. Итоги

В этом раунде обнаружены 2 бага времени выполнения высокой серьёзности и 1 проблема корректности данных средней серьёзности:

1. **Недействительность span в TracingLayer** — влияет на наблюдаемость всех запросов
2. **on_stop в LifecycleHook не выполняется** — влияет на корректность всей логики shutdown
3. **Потеря числовых типов Row** — влияет на типовую корректность результатов запросов к БД

Все три проблемы исправлены; после исправлений проходят все 60 тестов, компиляция без ошибок и предупреждений.

### Дальнейшие рекомендации

- Реализовать graceful shutdown для HTTP/gRPC серверов
- Добавить интеграционные тесты для `ecat-middleware` (mock Service + проверка поведения span/таймаута/восстановления)
- Добавить интеграционные тесты для `ecat-data-sqlx` (с использованием in-memory SQLite)
- Заменить синхронный RwLock в `ecat-registry/memory.rs` на `tokio::sync::RwLock`
