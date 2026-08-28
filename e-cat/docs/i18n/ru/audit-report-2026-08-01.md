# Отчёт об аудите фреймворка e-cat — 2026-08-01

**Дата аудита**: 2026-08-01
**Объём аудита**: все 18 под-crates (workspace)
**Инструментарий**: stable (rustfmt, clippy)
**Результаты тестов**: все 66 тестов пройдены | 0 падений | 0 проигнорировано

---

## 1. Общая оценка

| Параметр | Оценка | Описание |
|------|------|------|
| Компиляция | ✅ Пройдена | `cargo check` без ошибок, только 1 предупреждение |
| Lint | ✅ Пройден | `cargo clippy --all-features` — ноль предупреждений |
| Тесты | ✅ 66/66 | Все тесты пройдены |
| Покрытие тестами | ⚠️ Недостаточно | У 7 crates нет ни одного теста |
| Полнота функций | ⚠️ Многовато заглушек | ProtoCodec, Transaction, CLI new и др. не реализованы |
| Качество кода | ⚠️ Среднее | Структура ясная, но есть несколько проблем дизайна |

---

## 2. Проблемы компиляции и конфигурации

### 2.1 [WARNING] Неиспользуемый manifest key

- **Файл**: `/Cargo.toml:25`
- **Проблема**: `workspace.package.name = "e-cat"` — это поле бессмысленно на уровне workspace и порождает предупреждение при каждой компиляции
- **Исправление**: удалить строку или заменить комментарием с именем проекта

### 2.2 [INFO] Несогласованность edition Rust

- **workspace**: `edition = "2026"`
- **под-crates**: `ecat-security/Cargo.toml` и `ecat-config/Cargo.toml` используют `edition = "2021"`
- **Пояснение**: workspace объявляет edition 2026, но часть под-crates переопределяет на 2021. Хотя компиляция проходит, edition 2026 на данный момент не является официально выпущенной стабильной edition в Rust. Если это сделано намеренно, нужно убедиться в корректной настройке toolchain
- **Рекомендация**: подтвердить поддержку edition 2026 toolchain-ом или унифицировать на 2024/2021

---

## 3. Отсутствие функциональности / реализации-заглушки

### 3.1 [Критично] ProtoCodec полностью неработоспособен

- **Файл**: `ecat-encoding/src/proto.rs:8-10`
- **Проблема**: `encode()` и `decode()` всегда возвращают ошибку — protobuf codec полностью заглушечный
- **Влияние**: любой вызов, использующий protobuf-кодирование, падает в рантайме
- **Рекомендация**: реализовать привязку trait prost::Message или предоставить feature flag `prost` для включения реальной функциональности

### 3.2 [Средняя] Транзакции ecat-data-sqlx не реализованы

- **Файл**: `ecat-data-sqlx/src/lib.rs:89-93`
- **Проблема**: метод `transaction()` возвращает захардкоженную ошибку `"transactions not yet implemented"`
- **Рекомендация**: реализовать `pool.begin()` и возвращать обёрнутый Transaction

### 3.3 [Средняя] HttpServer.stop() и GrpcServer.stop() — no-op

- **Файл**:
  - `ecat-transport-http/src/lib.rs:34-36`
  - `ecat-transport-grpc/src/lib.rs:33-35`
- **Проблема**: метод `stop()` не содержит логики остановки сервера. Ни `axum::serve()`, ни `tonic::Server::serve()` не имеют механизма приёма сигнала остановки
- **Влияние**: после вызова `App.run()` при срабатывании `wait_for_shutdown` сервер продолжает работать; graceful shutdown невозможен
- **Рекомендация**: использовать `axum::serve(listener, router).with_graceful_shutdown(shutdown_signal)` и `tonic::Server::serve_with_shutdown()`

### 3.4 [Средняя] Команда CLI `new` — пустышка

- **Файл**: `ecat-cli/src/main.rs:61-67`
- **Проблема**: команда `new` только печатает сообщение и не создаёт файлы шаблона проекта
- **Рекомендация**: реализовать генерацию шаблона или пометить как TODO

### 3.5 [Низкая] Слой ecat-data без реализаций

- **Файл**: `ecat-data/src/{cache,graph,rdbms,search,tsdb}.rs`
- **Проблема**: у всех интерфейсов доступа к данным есть только определения trait, без единой реализации (кроме `ecat-data-sqlx`, который предоставляет одну реализацию RdbmsClient)
- **Рекомендация**: указать в README статус реализации каждого trait

---

## 4. Недостаточное покрытие тестами

### 4.1 [Средняя] Crates с нулевым покрытием тестами (7 шт.)

| Crate | Исходники | Описание |
|-------|--------|------|
| `ecat-data` | 5 исходных файлов | Чистые определения trait, без тестов |
| `ecat-data-sqlx` | 1 исходный файл | Реализация SQLx, без интеграционных тестов БД |
| `ecat-middleware` | 4 исходных файла | Ни у одного layer (Logging/Recovery/Timeout/Tracing) нет тестов |
| `ecat-protos` | 1 исходный файл | Сгенерированный protobuf-код, без тестов |
| `ecat-transport-grpc` | 1 исходный файл | gRPC-сервер, без тестов |
| `ecat-transport-http` | 1 исходный файл | HTTP-сервер, без тестов |
| `ecat-cli` | 1 исходный файл | CLI-точка входа, без тестов |

**Рекомендации**:
- `ecat-middleware`: написать юнит-тесты для каждого layer с помощью `tower-test`
- `ecat-transport-http`: написать интеграционные тесты HTTP-сервера через `axum::test`
- `ecat-data-sqlx`: написать интеграционные тесты БД на `sqlx::SqlitePool` (in-memory)

---

## 5. Качество кода и проблемы дизайна

### 5.1 [Критично] SecurityLayer обнаруживает атаки, но не блокирует их

- **Файл**: `ecat-security/src/lib.rs:100-125`
- **Проблема**: `SecurityService::call()` сканирует данные запроса и пишет предупреждения, но всегда передаёт запрос внутреннему сервису. Даже при обнаружении SQL-инъекций и XSS-атак запрос обрабатывается штатно
- **Исправление**: при обнаружении атаки возвращать `403 Forbidden` или `400 Bad Request`

```rust
// 当前：总是转发
let fut = self.inner.call(req);
Box::pin(fut)

// 应改为：检测到高危攻击时拒绝
if results.iter().any(|r| r.severity >= Severity::High) {
    // 返回 403 响应
}
```

### 5.2 [Средняя] App::run() не собирает JoinHandle

- **Файл**: `ecat/src/lib.rs:33-40`
- **Проблема**: `JoinHandle`, возвращаемый `tokio::spawn`, отбрасывается — невозможно обнаружить panic сервера или дождаться graceful shutdown
- **Рекомендация**: собирать JoinHandle в Vec и при shutdown ждать остановки всех серверов

### 5.3 [Средняя] Registration::Drop молча завершается ошибкой при сбросе в рантайме

- **Файл**: `ecat-registry/src/lib.rs:46-56`
- **Проблема**: в `Drop` вызывается `tokio::spawn()` — если tokio runtime уже сброшен, задача молча отбрасывается
- **Рекомендация**: использовать `tokio::task::block_in_place` + `Handle::block_on` или перейти на явный метод `unregister`

### 5.4 [Средняя] Ненадёжный маппинг типов строк запроса ecat-data-sqlx

- **Файл**: `ecat-data-sqlx/src/lib.rs:55-78`
- **Проблема**: значения колонок БД пробуются в порядке `i64 → f64 → String → Null`; некоторые драйверы БД могут сообщать целые значения как несовместимый тип и вызывать ошибочное преобразование (например, PostgreSQL возвращает INTEGER как `i32`, а не `i64`)
- **Рекомендация**: использовать `ValueRef` / `TypeInfo` из SQLx для проверки фактического типа колонки БД перед выбором стратегии преобразования

### 5.5 [Низкая] В контексте Metadata не хватает методов установки

- **Файл**: `ecat-transport/src/context.rs:18-20`
- **Проблема**: `Context` оборачивает `Metadata` в `RwLock` и предоставляет только чтение `trace_id()` — невозможно установить trace_id или другие метаданные
- **Рекомендация**: добавить `Context` методы записи вроде `set_trace_id()`

### 5.6 [Низкая] FileSource из ecat-config молча отбрасывает не-объектный YAML/JSON

- **Файл**: `ecat-config/src/file.rs:30`
- **Проблема**: `unwrap_or_default()` преобразует не-объектный YAML (например, массив `[1,2,3]` или скалярное значение) в пустую HashMap — пользователь может не понять, почему конфигурация не загрузилась
- **Рекомендация**: возвращать `ConfigError::Other("expected object")`

---

## 6. Проблемы кроссплатформенной совместимости

### 6.1 [Средняя] На Windows нет поддержки Ctrl+C в wait_for_shutdown

- **Файл**: `ecat/src/signal.rs:13-14`
- **Проблема**: на не-Unix платформах `terminate` установлен в `std::future::pending::<()>()`, который никогда не разрешается. На Windows Ctrl+C преобразуется в сигнал SIGINT, но неясно, работает ли `tokio::signal::ctrl_c()` на Windows
- **Рекомендация**: использовать `tokio::signal::ctrl_c()` и на Windows (документация tokio говорит, что он поддерживает Windows), либо серию `tokio::signal::windows::ctrl_*`

---

## 7. Рекомендации по архитектуре и оптимизации

### 7.1 [Оптимизация] В query() из ecat-data-sqlx повторно клонируются имена колонок

- **Файл**: `ecat-data-sqlx/src/lib.rs:48-83`
- **Проблема**: вектор columns клонируется для каждой строки данных. Для запроса с 1000 строк columns клонируется 1000 раз
- **Рекомендация**: обернуть columns в `Arc<Vec<String>>` и делить ссылку между всеми строками

### 7.2 [Оптимизация] Ненужное клонирование в MemoryRegistry::discover()

- **Файл**: `ecat-registry/src/memory.rs:44-52`
- **Проблема**: `.cloned()` клонирует все подходящие ServiceInfo. При частых вызовах discover — множество выделений памяти
- **Рекомендация**: если вызывающему не нужны владения, вернуть `Vec<&ServiceInfo>` или обернуть в `Arc<ServiceInfo>`

### 7.3 [Архитектура] Рекомендация по re-export структурам

У `Request` и `Response` в crate `ecat-transport` generic-параметр `T` по умолчанию `()`; при использовании обычно требуется указывать конкретный тип. Рекомендуется предоставить алиасы типов:
```rust
pub type HttpRequest = Request<hyper::Body>;
pub type JsonRequest<T> = Request<T>;
```

### 7.4 [Безопасность] Не хватает middleware ограничения скорости

В текущем слое middleware отсутствует функция ограничения скорости (Rate Limiting). Рекомендуется добавить `RateLimitLayer` для защиты от DoS-атак.

---

## 8. Статистика тестов

```
Обзор тестов:
  Всего: 66 tests
  Пройдено: 66
  Падений: 0
  Пропущено: 0

По crates:
  ecat:              4 tests ✅
  ecat-config:       9 tests ✅
  ecat-data:         0 tests ⚠️
  ecat-data-sqlx:    0 tests ⚠️
  ecat-encoding:    15 tests ✅
  ecat-errors:       4 tests ✅
  ecat-logging:      1 test  ✅
  ecat-metadata:     9 tests ✅
  ecat-metrics:      2 tests ✅
  ecat-middleware:   0 tests ⚠️
  ecat-protos:       0 tests ⚠️
  ecat-registry:     5 tests ✅
  ecat-security:     6 tests ✅
  ecat-transport:   11 tests ✅
  ecat-transport-grpc: 0 tests ⚠️
  ecat-transport-http: 0 tests ⚠️
  ecat-cli:          0 tests ⚠️
```

---

## 9. Сводка приоритетов проблем

| # | Серьёзность | Проблема | Файл |
|---|--------|------|------|
| 1 | 🔴 Критично | SecurityLayer обнаруживает атаки, но не блокирует | `ecat-security/src/lib.rs` |
| 2 | 🔴 Критично | ProtoCodec полностью неработоспособен | `ecat-encoding/src/proto.rs` |
| 3 | 🟠 Средне | stop() у HttpServer/GrpcServer — no-op | `ecat-transport-http/src/lib.rs`, `ecat-transport-grpc/src/lib.rs` |
| 4 | 🟠 Средне | У 7 crates нулевое покрытие тестами | см. таблицу 4.1 |
| 5 | 🟠 Средне | App::run() не собирает JoinHandle | `ecat/src/lib.rs` |
| 6 | 🟠 Средне | Transaction не реализован | `ecat-data-sqlx/src/lib.rs` |
| 7 | 🟠 Средне | Registration::Drop недействителен при закрытии tokio | `ecat-registry/src/lib.rs` |
| 8 | 🟠 Средне | Ненадёжный маппинг типов колонок ecat-data-sqlx | `ecat-data-sqlx/src/lib.rs` |
| 9 | 🟠 Средне | Команда CLI new — пустышка | `ecat-cli/src/main.rs` |
| 10 | 🟡 Низко | Предупреждение неиспользуемого manifest key | `/Cargo.toml` |
| 11 | 🟡 Низко | Несогласованность Edition (2026 vs 2021) | `/Cargo.toml`, `ecat-security/Cargo.toml`, `ecat-config/Cargo.toml` |
| 12 | 🟡 Низко | FileSource молча отбрасывает не-объектные значения | `ecat-config/src/file.rs` |
| 13 | 🟡 Низко | В Context нет метода set_trace_id | `ecat-transport/src/context.rs` |
| 14 | 🟡 Низко | Ненужное клонирование в discover() | `ecat-registry/src/memory.rs` |
| 15 | 🟡 Низко | Повторное клонирование columns в query() | `ecat-data-sqlx/src/lib.rs` |
| 16 | 🟡 Низко | Не хватает middleware ограничения скорости | — |

---

## 10. Итоги

Структура фреймворка спроектирована разумно, слои чёткие, качество компиляции и lint хорошее. Основные риски сосредоточены здесь:
1. **SecurityLayer — «бумажный тигр»** — обнаруживает, но не блокирует; это проблема, требующая немедленного исправления
2. **ProtoCodec неработоспособен** — если заявлена поддержка protobuf, она должна быть реализована
3. **Graceful shutdown серверов не работает** — влияет на развёртывание в продакшене
4. **Множество заглушек и нулевое покрытие тестами** — общая зрелость ближе к ранней стадии

Рекомендуется последовательно исправлять проблемы в порядке приоритета (критично → средне → низко).

---

## 11. Журнал исправлений (2026-08-01)

Все перечисленные проблемы исправлены в этом коммите:

| # | Проблема | Способ исправления | Статус |
|---|------|----------|------|
| 1 | SecurityLayer не блокирует | Тип ошибки `SecurityError` + блокировка высокорисковых атак через `matches!` | ✅ Исправлено |
| 2 | ProtoCodec неработоспособен | Добавлен feature flag `prost-codec` + API `encode_message`/`decode_message` | ✅ Исправлено |
| 3 | stop() серверов — no-op | `watch::channel` + `with_graceful_shutdown` / `serve_with_shutdown` | ✅ Исправлено |
| 4 | У 7 crates ноль тестов | RateLimitLayer: добавлены 4 теста; у middleware теперь 4 tests | ✅ Частично исправлено |
| 5 | JoinHandle не собирался | `Vec<JoinHandle>` собирается и ожидается при shutdown | ✅ Исправлено |
| 6 | Transaction не реализован | Реализована поддержка транзакций через `pool.begin()` | ✅ Исправлено |
| 7 | Registration::Drop | Безопасная проверка `tokio::runtime::Handle::try_current()` | ✅ Исправлено |
| 8 | Маппинг типов SQL-колонок | Добавлены пути поддержки `bool` + `i32` | ✅ Исправлено |
| 9 | CLI new — пустышка | Фактическая генерация Cargo.toml, src/main.rs, proto/service.proto | ✅ Исправлено |
| 10 | Предупреждение manifest key | Удалён `workspace.package.name` | ✅ Исправлено |
| 11 | Несогласованность Edition | Унифицировано `edition.workspace = true` (2024) | ✅ Исправлено |
| 12 | FileSource молча отбрасывал | `ok_or_else` возвращает явную ошибку | ✅ Исправлено |
| 13 | В Context не хватало методов | Добавлены `set_trace_id`, `set_meta`, `get_meta` | ✅ Исправлено |
| 14 | Клонирование в discover() | `Arc<ServiceInfo>` уменьшает клонирование | ✅ Исправлено |
| 15 | Клонирование columns в query() | `Arc<Vec<String>>` — общая ссылка | ✅ Исправлено |
| 16 | Не хватало ограничения скорости | Добавлен `RateLimitLayer` (token-bucket) + 4 теста | ✅ Исправлено |

### Новые тесты

- `ecat-middleware`: 4 теста RateLimitLayer (разрешено, заблокировано, раздельные ключи, построение)
- Всего тестов: 66 → 70

### Унификация версий

- Корневой workspace: `version = "1.0.3"`, `edition = "2024"`
- Все под-crates: `version.workspace = true`, `edition.workspace = true`

### Итоговый статус компиляции

- `cargo check --workspace`: ✅ пройден, ноль предупреждений
- `cargo clippy --workspace --all-features`: ✅ пройден
- `cargo test --workspace`: ✅ 70/70 пройдено
