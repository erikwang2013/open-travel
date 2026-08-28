# e-cat: отчёт о ревью кода — 2026-08-01 (раунд 4 · всё исправлено)

**Версия проекта:** 2.1.0  
**Итоговый статус:** 0 warnings, ~116 тестов, clippy clean, fmt clean

**Зачистка раунда 5:** удалено 12 неиспользуемых зависимостей (ecat-health/reqwest, ecat-circuit-breaker/tokio, ecat-bench/tracing, ecat-mq/serde+serde_json, ecat-events/async-trait, ecat-config-remote/tracing, ecat-testing/transport-http+axum, ecat-client/serde+serde_json)
**Объём ревью:** все 18 crates

## Итоговый статус

| Инструмент | Статус |
|------|------|
| `cargo build` | Пройдено (0 warnings) |
| `cargo test` | 77 passed, 0 failed, 1 ignored |
| `cargo clippy` | Пройдено (0 warnings) |
| `cargo fmt` | Пройдено |

---

## Список исправлений (все)

### Средний риск

1. **[Исправлено]** `Mutex::lock().unwrap()` → `ecat-transport-http/lib.rs`, `ecat-transport-grpc/lib.rs`
2. **[Исправлено]** CLI `fs::write().unwrap()` → `ecat-cli/src/main.rs`

### Низкий риск

3. **[Исправлено]** ProtoCodec doc-test → `ecat-encoding/src/proto.rs`
4. **[Исправлено]** Крейты без юнит-тестов → transport-http/grpc добавлено по 3 теста
5. **[Исправлено]** `Transaction::commit()` — пустышка → добавлен trait `TransactionInner`
6. **[Исправлено]** Исправлен комментарий в `SecurityScanner::new()`
7. **[Исправлено]** Неиспользуемая зависимость `opentelemetry` → `ecat-logging` и корневой Cargo.toml workspace
8. **[Исправлено]** Формат doc-test

### Оптимизации

9. **[Исправлено]** Преаллокация в `scan_parts` → `Vec::with_capacity`
10. **[Исправлено]** Устаревший `serde_yaml` 0.9 → миграция на `yaml_serde` 0.10
11. **[Исправлено]** `Transaction::commit()` больше не пустышка → реальный commit/rollback через `SqlxTransactionWrapper`

### Без исправления (проектные решения)

- **Дополнительные зависимости crate `ecat`** — намеренный паттерн «meta crate» для удобных транзитивных зависимостей у потребителей
- **Codec trait у ProtoCodec возвращает ошибку** — фундаментальное типовое различие serde и `prost::Message`, решено раздельными API `encode_message()`/`decode_message()` и ясной документацией
- **`ecat-data` без конкретной реализации** — проектирование через trait-интерфейсы, реализация в `ecat-data-sqlx`

---

## Сводка изменённых файлов

| Файл | Изменение |
|------|------|
| `ecat-transport-http/src/lib.rs` | Защита от отравления Mutex + 3 новых теста |
| `ecat-transport-grpc/src/lib.rs` | Защита от отравления Mutex + 3 новых теста |
| `ecat-cli/src/main.rs` | Унифицированная обработка ошибок |
| `ecat-security/src/lib.rs` | Исправлен комментарий + оптимизация преаллокации |
| `ecat-logging/Cargo.toml` | Удалён неиспользуемый opentelemetry |
| `ecat-encoding/src/proto.rs` | Улучшен doc-test |
| `ecat-data/src/lib.rs` | Экспорт TransactionInner |
| `ecat-data/src/rdbms.rs` | Новый trait TransactionInner |
| `ecat-data-sqlx/src/lib.rs` | SqlxTransactionWrapper реализует TransactionInner |
| `ecat-config/Cargo.toml` | serde_yaml → yaml_serde |
| `ecat-config/src/file.rs` | serde_yaml → yaml_serde |
| `Cargo.toml` | Удалена осиротевшая workspace-зависимость opentelemetry |
| `README.md` | Обновлён номер версии, исправлено описание наблюдаемости, добавлены ссылки на план экосистемы |
| `docs/ecosystem-plan.md` | Новый документ плана экосистемы (3 фазы, 15 crates) |
