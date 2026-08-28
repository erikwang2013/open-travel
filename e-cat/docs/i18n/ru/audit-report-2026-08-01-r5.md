# E-CAT: отчёт об аудите — r5

**Дата**: 2026-08-01  
**Ветка**: main  
**Версия**: 2.1.7  
**Количество crates**: 47 (workspace members)  
**Статус**: ✅ все исправимые проблемы решены + полная поддержка конфигурационных файлов в бэкендах данных

---

## 0. Журнал исправлений (2026-08-01)

| # | Проблема | Файл | Исправление |
|---|------|------|------|
| 1 | unused import `axum::routing::get` | `ecat-versioning/src/lib.rs:3` | Удалён import верхнего уровня, перенесён в `#[cfg(test)]` |
| 2 | unused variable `version` | `ecat-versioning/src/lib.rs:61` | Переименовано в `_version` |
| 3 | dead code `extract_version` | `ecat-versioning/src/lib.rs:68` | Изменено на `pub fn` |
| 4 | `useless_format!` | `ecat-versioning/src/lib.rs:62` | Используется напрямую `"/api"` |
| 5 | `unnecessary_to_owned` | `ecat-data-questdb/src/lib.rs:39` | `"true".to_string()` → `"true"` |
| 6 | Съедание сообщений об ошибках | `ecat-data-questdb/src/lib.rs:30` | `unwrap_or_default()` → `unwrap_or_else(...)` |
| 7 | `derivable_impls` | `ecat-client/src/lib.rs:249` | `GrpcClientBuilder` переведён на `#[derive(Default)]` |
| 8 | `manual_is_multiple_of` | `ecat-config/src/encrypted.rs:60` | `s.len() % 2 != 0` → `!s.len().is_multiple_of(2)` |
| 9 | `collapsible_if` | `ecat-registry-etcd/src/lib.rs:92` | Объединены вложенные `if let` |
| 10 | `collapsible_if` | `ecat-data-clickhouse/src/lib.rs:56` | Объединены вложенные `if let` |
| 11 | `type_complexity` | `ecat-data-memcached/src/lib.rs:9` | Добавлен алиас `type CacheEntry` |

**Итог**: `cargo build` без предупреждений, `cargo clippy --all-targets` без предупреждений, `cargo test` полностью зелёный (0 failures).

### 12 ─ Полная поддержка конфигурационных файлов в бэкендах данных (Cargo + lib.rs)

Для 12 crates-бэкендов данных добавлены структуры `Config` (`#[derive(Deserialize)]`) и конструкторы `from_config()`, позволяющие загружать информацию о подключении из JSON/YAML-конфигурации без хардкода.

| Crate | Структура Config | Поля |
|-------|--------------|------|
| `ecat-data-redis` | `RedisConfig` | `url` |
| `ecat-data-sqlx` | `SqlxConfig` | `url` |
| `ecat-data-clickhouse` | `ClickhouseConfig` | `base_url`, `database` (по умолчанию "default") |
| `ecat-data-questdb` | `QuestdbConfig` | `base_url` |
| `ecat-data-elasticsearch` | `ElasticsearchConfig` | `base_url` |
| `ecat-data-opensearch` | `OpenSearchConfig` | `base_url` |
| `ecat-data-influxdb` | `InfluxConfig` | `base_url`, `org`, `bucket`, `token` |
| `ecat-data-memcached` | `MemcachedConfig` | (пусто — in-memory реализация) |
| `ecat-data-neo4j` | `Neo4jConfig` | `base_url`, `username`, `password` |
| `ecat-data-nebulagraph` | `NebulaGraphConfig` | `base_url`, `space` |
| `ecat-data-arangodb` | `ArangoConfig` | `base_url`, `db`, `username`, `password` |
| `ecat-data-iotdb` | `IotdbConfig` | `base_url`, `username`, `password` |

**Пример использования**:
```rust
// 从 YAML 配置文件加载
let cfg: ClickhouseConfig = serde_json::from_str(r#"{"base_url":"http://localhost:8123"}"#)?;
let client = ClickhouseClient::from_config(cfg);
```

### 13 ─ Опциональная поддержка аутентификации в HTTP-бэкендах (5 crates)

Для 5 чисто HTTP-бэкендов добавлены опциональные поля `username` / `password` и конструкторы `with_auth()`. Все поля — `Option<String>` (`#[serde(default)]`), без конфигурации аутентификация отсутствует.

| Crate | Новые поля Config | Новый конструктор |
|-------|-----------------|-------------|
| `ecat-data-elasticsearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-opensearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-clickhouse` | `username?`, `password?` | `with_auth()` |
| `ecat-data-questdb` | `username?`, `password?` | `with_auth()` |
| `ecat-data-nebulagraph` | `username?`, `password?` | `with_auth()` |

Все HTTP-запросы автоматически получают Basic Auth через вспомогательный метод `apply_auth()` (только когда оба поля не None).

### 14 ─ Опциональные поля аутентификации для Redis / RDBMS / Memcached (3 crates)

| Crate | Новые поля Config | Новый конструктор | Способ аутентификации |
|-------|-----------------|-------------|----------|
| `ecat-data-redis` | `password?` | `connect_with_password()` | Пароль, встроенный в URL |
| `ecat-data-sqlx` | `username?`, `password?` | `connect_with_auth()` | Аутентификация, встроенная в URL |
| `ecat-data-memcached` | `username?`, `password?` | `with_auth()` | Поле-заглушка (in-memory реализация) |

Sqlx покрывает четыре RDBMS: SQLite / PostgreSQL / MySQL / TiDB. Поля Auth встраиваются в URL подключения через `replacen("://", "://user:pass@")` — только если в URL нет `@`.

### 15 ─ Поддержка TLS-сертификатов + crate ecat-tls (все 12 бэкендов)

Добавлен crate `ecat-tls`, предоставляющий:
- `TlsClientConfig` — опциональная TLS-конфигурация (ca_cert, client_cert, client_key, skip_verify)
- `generate_ca()` — генерация самоподписанного CA-сертификата
- `generate_server_cert()` — генерация серверного сертификата
- `generate_client_cert()` — генерация клиентского сертификата (mTLS)

Во все 12 Config бэкендов данных добавлено поле `#[serde(default)] tls: Option<TlsClientConfig>`.

| Тип бэкенда | Способ TLS |
|----------|----------|
| 9 HTTP-бэкендов | `tls.build_reqwest_client()` — построение TLS-клиента reqwest |
| Redis | Переключение схемы URL `redis://` → `rediss://` |
| Sqlx | Поле-заглушка (TLS через параметр URL `?sslmode=require`) |
| Memcached | Поле-заглушка (зарезервировано для сетевой реализации) |

---

## 1. Обзор

| Пункт | Статус | Детали |
|------|------|------|
| `cargo build` | ✅ Пройдено | 3 предупреждения компилятора, 19.85s |
| `cargo test` | ✅ Пройдено | ~137 юнит-тестов, все пройдены, 0 failed, 1 ignored |
| `cargo clippy` | ⚠️ Есть warnings | 5 lint-предупреждений в 3 crates |
| `cargo fmt` | ✅ Пройдено | Проблем с форматированием нет |
| `cargo audit` | ❌ Не установлен | Невозможно сканировать известные CVE |

---

## 2. Предупреждения компилятора (требуют исправления)

### 2.1 ecat-versioning (3 warning)

**Файл**: `ecat-versioning/src/lib.rs`

| # | Warning | Строка | Серьёзность |
|---|---------|------|----------|
| 1 | `unused import: axum::routing::get` | 3 | Низкая |
| 2 | `unused variable: version` | 61 | Низкая |
| 3 | `function extract_version is never used` | 68 | Низкая |

**Рекомендация**: удалить неиспользуемый import, переименовать `version` в `_version`, изменить `extract_version` на `pub` или пометить `#[allow(dead_code)]`.

### 2.2 ecat-data-questdb (1 clippy warning)

**Файл**: `ecat-data-questdb/src/lib.rs:39`

```rust
// 当前:
.query(&[("query", sql), ("count", &"true".to_string())])

// 应改为:
.query(&[("query", sql), ("count", &"true")])
```

### 2.3 ecat-client (1 clippy warning)

**Файл**: `ecat-client/src/lib.rs:249`

`GrpcClientBuilder` реализует `Default` вручную — можно заменить на `#[derive(Default)]`.

---

## 3. Сводка clippy-предупреждений

| Crate | Warning | Тип |
|-------|---------|------|
| ecat-versioning | `useless_format!` — использовать `"/api".to_string()` | Производительность |
| ecat-versioning | unused import / dead code | Зачистка |
| ecat-data-questdb | `unnecessary_to_owned` | Производительность |
| ecat-client | `derivable_impls` — использовать derive Default | Упрощение |

---

## 4. Анализ покрытия тестами

### 4.1 Статистика

| Метрика | Значение |
|------|------|
| Всего юнит-тестов | ~137 |
| Неудачных | 0 |
| Ignored | 1 |
| Crates с тестами | ~24 / 48 |
| **Crates с 0 тестами** | **~24 / 48 (50%)** |

### 4.2 Crates без тестов (0 или только конструкторы)

Следующие crates имеют слабое тестовое покрытие:

- ecat-data-arangodb, ecat-data-clickhouse, ecat-data-elasticsearch
- ecat-data-influxdb, ecat-data-iotdb, ecat-data-nebulagraph
- ecat-data-neo4j, ecat-data-opensearch, ecat-data-questdb
- ecat-data-redis, ecat-data-sqlx, ecat-data-memcached
- ecat-mq, ecat-mq-kafka, ecat-graphql, ecat-openapi
- ecat-transport, ecat-transport-grpc, ecat-transport-http
- ecat-transport-ws, ecat-tracing, ecat-logging
- ecat-middleware, ecat-registry-consul, ecat-registry-etcd

### 4.3 Doc-тесты

У всех **48 crates doc-тестов 0**. В коде нет ни одного примера `/// ````rust`.

---

## 5. Проблемы зависимостей

### 5.1 ⚠️ yaml_serde vs serde_yaml (средний риск)

**Файл**: `ecat-config/Cargo.toml:9`

```toml
yaml_serde = "0.10"
```

Стандартная YAML-библиотека в экосистеме Rust — `serde_yaml` (последняя версия `0.9.34+`), а `yaml_serde` — это **другой, менее поддерживаемый crate**.

**Рекомендация**: подтвердить, что `yaml_serde` — ожидаемая зависимость. Если подразумевался `serde_yaml`, заменить.

### 5.2 Отсутствует cargo-audit

`cargo audit` не установлен. Рекомендуется `cargo install cargo-audit` и добавление в CI.

### 5.3 Отсутствует поле description

В `[workspace.package]` нет `description`, и ни один под-crate не определяет description.

---

## 6. Проблемы качества кода

### 6.1 unwrap/expect в production-коде

| Файл | Строка | Вызов | Риск |
|------|------|------|------|
| `ecat-client/src/lib.rs` | 28 | `.expect("StaticResolver poisoned")` | Низкий — обоснованно |
| `ecat/src/signal.rs` | 8 | `.expect("failed to install SIGTERM handler")` | Средний — panic при запуске |
| `ecat-protos/build.rs` | 5 | `.unwrap()` | Низкий — build script |

### 6.2 extract_version в ecat-versioning

Функция `extract_version` (строка 68) извлекает номер версии из Accept-заголовка, но не вызывается в `build_header_router()`.

### 6.3 Обработка ошибок в ecat-data-questdb

```rust
// 第 30 行: 网络响应体读取使用 unwrap_or_default
Err(RdbmsError::Database(resp.text().await.unwrap_or_default()))
```

При сбое `resp.text()` сообщение об ошибке молча отбрасывается. Рекомендуется `unwrap_or_else(|e| format!("questdb parse: {e}"))`.

---

## 7. Оценка архитектуры

### Достоинства

- Чёткое разделение ответственности по 48 crates
- Единая версия workspace: `version.workspace = true`
- Минимальные зависимости, без тяжёлых фреймворков
- Нет TODO/FIXME/HACK

### Требует улучшения

| Проблема | Приоритет |
|------|--------|
| У 50% crates нет тестов | Высокий |
| Путаница yaml_serde vs serde_yaml | Средний |
| Отсутствует cargo-audit | Средний |
| Мёртвый код в ecat-versioning | Низкий |
| Нет doc-тестов | Низкий |

---

## 8. Обзор безопасности

| Проверка | Результат |
|--------|------|
| Захардкоженные ключи | Не обнаружены |
| Утечка .env-файлов | Не обнаружена |
| Опасные unwrap (production-код) | 2 места (signal.rs, client.rs) |
| Сканирование CVE | Не выполнено (нужно установить cargo-audit) |

---

## 9. План действий

### P0 — исправить немедленно
1. Зачистить 3 предупреждения компилятора в ecat-versioning
2. Исправить clippy в ecat-data-questdb
3. Исправить derivable_impls в ecat-client

### P1 — краткосрочно
4. Установить `cargo-audit` для сканирования уязвимостей зависимостей
5. Подтвердить выбор `yaml_serde` vs `serde_yaml`
6. Дополнить doc-тестами ключевые crates

### P2 — среднесрочно
7. Дополнить тестами transport/data/security crates
8. Добавить поле `description` во все crates
9. Интегрировать или удалить `extract_version`

### P3 — долгосрочно
10. Настроить CI: build → test → clippy → audit → coverage

---

*Отчёт сгенерирован 2026-08-01. Инструменты: cargo 1.92.0, rustc 1.92.0, clippy 1.92.0*
