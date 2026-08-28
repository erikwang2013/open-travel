<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat: отчёт о ревью кода и TDD-тестировании

**Дата**: 2026-07-29  
**Ветка**: main  
**Проект**: e-cat (Rust workspace, 17 crates)

---

## 1. Объём ревью

Проверены все исходники Rust во всех 17 crates workspace-а (38 файлов `.rs`).

| Crate | Описание | Файлов |
|-------|------|--------|
| `ecat-protos` | Определения Protobuf и генерация кода | 2 |
| `ecat-errors` | Единый тип ошибок | 2 |
| `ecat-metadata` | Абстракция метаданных запроса | 1 |
| `ecat-encoding` | Кодирование/декодирование JSON/Protobuf | 3 |
| `ecat-logging` | Инициализация логирования/Tracing | 1 |
| `ecat-config` | Загрузка конфигурации (файл/переменные окружения) | 3 |
| `ecat-data` | Абстракции trait слоя данных | 5 |
| `ecat-data-sqlx` | Реализация RDBMS на SQLx | 1 |
| `ecat-registry` | Регистрация и discovery сервисов | 2 |
| `ecat-metrics` | Метрики Prometheus | 1 |
| `ecat-middleware` | Слои middleware Tower | 4 |
| `ecat-transport` | Абстракция транспортного слоя | 4 |
| `ecat-transport-http` | Реализация транспорта HTTP/Axum | 1 |
| `ecat-transport-grpc` | Реализация транспорта gRPC/Tonic | 1 |
| `ecat` | Ядро прикладного фреймворка | 3 |
| `ecat-cli` | Инструменты CLI | 1 |
| `examples/helloworld` | Пример проекта | 1 |

---

## 2. Найденные проблемы и исправления

### Проблема 1: [Clippy] `map_identity` — бессмысленный identity map

- **Файл**: `ecat-config/src/file.rs:30`
- **Серьёзность**: низкая
- **Проблема**: `map(|(k, v)| (k, v))` не делает никаких преобразований — это мёртвый код
- **Исправление**: убрать лишний вызов `.map()`

### Проблема 2: [Clippy] `new_without_default` — у Config нет реализации Default

- **Файл**: `ecat-config/src/lib.rs:27`
- **Серьёзность**: низкая
- **Проблема**: у `Config` есть метод `new()`, но не реализован trait `Default`
- **Исправление**: заменить ручную реализацию на `#[derive(Default)]`

### Проблема 3: [Clippy] `io_other_error` — устаревший способ создания Error

- **Файл**: `ecat-middleware/src/recovery.rs:42`
- **Серьёзность**: низкая
- **Проблема**: у `std::io::Error::new(std::io::ErrorKind::Other, ...)` уже есть более лаконичная альтернатива
- **Исправление**: использовать `std::io::Error::other("task panicked")`

### Проблема 4: [Clippy] `redundant_async_block` — лишний async-блок

- **Файл**: `ecat-middleware/src/tracing.rs:38`
- **Серьёзность**: низкая
- **Проблема**: в `Box::pin(async move { fut.await })` async-блок избыточен
- **Исправление**: упростить до `Box::pin(fut)`

### Проблема 5: [Clippy] `redundant_closure` — лишнее замыкание

- **Файл**: `ecat-data-sqlx/src/lib.rs:63`
- **Серьёзность**: низкая
- **Проблема**: замыкание в `.and_then(|f| serde_json::Number::from_f64(f))` можно опустить
- **Исправление**: использовать напрямую `.and_then(serde_json::Number::from_f64)`

### Проблема 6: [Clippy] `unwrap_or_default` — можно упростить через unwrap_or_default

- **Файл**: `ecat-transport-http/src/lib.rs:27`
- **Серьёзность**: низкая
- **Проблема**: `unwrap_or_else(Router::new)` эквивалентно `unwrap_or_default()`
- **Исправление**: использовать `unwrap_or_default()`

---

## 3. Покрытие тестами

### До исправлений

| Crate | Тестов |
|-------|--------|
| `ecat-errors` | 4 |
| `ecat-transport` | 11 |
| остальные 15 crates | **0** |
| **Итого** | **15** |

### После исправлений

| Crate | Тестов | Добавлено | Содержание тестов |
|-------|--------|------|----------|
| `ecat-encoding` | 15 | +15 | Roundtrip кодирования/декодирования JsonCodec, некорректное декодирование, content_type; диспетчеризация CodecBox; пути codec_from_content_type нормальный/ошибочный; варианты Encoding |
| `ecat-errors` | 4 | — | Сопоставление HTTP-статусов, преобразование gRPC-статусов, накопление metadata, формат Display |
| `ecat-metadata` | 9 | +9 | Хранение ключ-значение, trace_id, From\<HeaderMap\> (включая пропуск не-UTF8 значений), From\<MetadataMap\> (пропуск ASCII и бинарных), IntoIterator |
| `ecat-logging` | 1 | +1 | Smoke-тест init |
| `ecat-config` | 4 | +4 | Создание/значения по умолчанию, типизированное чтение, загрузка из ConfigSource |
| `ecat-registry` | 5 | +5 | Регистрация/discovery, дерегистрация/удаление, ошибка при отсутствии, список сервисов, фильтрация по имени |
| `ecat-metrics` | 2 | +2 | Singleton registry, metrics_text не паникует |
| `ecat` | 4 | +4 | Значения Builder по умолчанию, имя/версия, регистрация server, lifecycle hook |
| `ecat-transport` | 11 | — | Создание Context/Request/Response и значения по умолчанию, trait Server |
| **Итого** | **55** | **+40** | |

### Crate без юнит-тестов

- `ecat-protos` — только генерация кода protobuf
- `ecat-data` — чистые определения trait, без логики реализации
- `ecat-data-sqlx` — требует подключения к БД, относится к интеграционным тестам
- `ecat-middleware` — реализации Tower Service, нужны интеграционные тесты
- `ecat-transport-http` / `ecat-transport-grpc` — требуют прослушивания сети, относятся к интеграционным тестам
- `ecat-cli` — только вывод на печать, без логики

---

## 4. Результаты проверки

```
cargo test   → 55 passed, 0 failed
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
```

---

## 5. Список изменённых файлов

| Файл | Изменение |
|------|------|
| `ecat-config/src/file.rs` | Убран identity map |
| `ecat-config/src/lib.rs` | `#[derive(Default)]` + 4 теста |
| `ecat-data-sqlx/src/lib.rs` | Упрощено лишнее замыкание |
| `ecat-middleware/src/recovery.rs` | Использован `std::io::Error::other()` |
| `ecat-middleware/src/tracing.rs` | Убран лишний async-блок |
| `ecat-transport-http/src/lib.rs` | `unwrap_or_else` → `unwrap_or_default` |
| `ecat-metrics/src/lib.rs` | 2 теста |
| `ecat-registry/src/memory.rs` | 5 тестов |
| `ecat/src/lib.rs` | 4 теста |
