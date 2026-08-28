<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat: отчёт о ревью кода (третий раунд)

**Дата**: 2026-07-29  
**Ветка**: main  
**Проект**: e-cat (Rust workspace, 18 crates)  
**Объём ревью**: все 37 исходных файлов, всего 2151 строка кода Rust

---

## 1. Обзор ревью

Все 3 бага, найденные во втором раунде, исправлены; этот раунд — глубокое повторное ревью на чистой базовой линии (0 error / 0 warning / 60 test passed), с фокусом на граничные условия, обработку ошибок и производственную надёжность.

### Базовая линия проверки

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

### Подтверждение исправлений R2

| Баг | Файл | Статус |
|-----|------|------|
| Жизненный цикл guard-а span в TracingLayer | `ecat-middleware/src/tracing.rs` | ✅ Исправлено |
| on_stop в LifecycleHook не выполняется | `ecat/src/hook.rs`, `ecat/src/lib.rs` | ✅ Исправлено |
| Приоритет извлечения типов значений Row | `ecat-data-sqlx/src/lib.rs` | ✅ Исправлено |

---

## 2. Новые найденные проблемы

### Проблема 1: [Средняя] `unwrap()` в `metrics_text()` — возможен panic в продакшене

- **Файл**: `ecat-metrics/src/lib.rs:14-15`
- **Серьёзность**: **средняя**
- **Влияние**: panic процесса при обращении к эндпоинту `/metrics`

**Анализ корневой причины**:

```rust
pub fn metrics_text() -> String {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder.encode(&registry().gather(), &mut buffer).unwrap();  // 可能 panic
    String::from_utf8(buffer).unwrap()                           // 可能 panic
}
```

`TextEncoder::encode()` завершается ошибкой при внутренней ошибке I/O или нехватке памяти в системе. `String::from_utf8()` теоретически тоже может завершиться ошибкой, если библиотека Prometheus выдаёт не-UTF8 вывод. Оба `unwrap()` находятся на не-тестовых путях кода и напрямую вызываются из HTTP-handler-а; panic приведёт к краху процесса.

**Рекомендуемое исправление**: возвращать `Result<String, ...>` или использовать `.unwrap_or_default()` для деградации.

---

### Проблема 2: [Низкая] Recovery middleware через spawn новой задачи теряет контекст span

- **Файл**: `ecat-middleware/src/recovery.rs:40`
- **Серьёзность**: **низкая**
- **Влияние**: если Recovery-слой стоит перед Tracing-слоем, trace_id запроса не передаётся в бизнес-логику

**Анализ корневой причины**:

```rust
fn call(&mut self, req: Req) -> Self::Future {
    let fut = self.inner.call(req);
    Box::pin(async move {
        match tokio::task::spawn(fut).await {  // 新 task，不继承 span
            // ...
        }
    })
}
```

`tokio::task::spawn()` создаёт новую задачу Tokio; tracing span является task-local и автоматически не передаётся.

**Рекомендация**: явно задокументировать требование к порядку middleware (Recovery должен быть самым внешним слоем) или передавать span вручную через `.instrument(span)` перед spawn.

---

### Проблема 3: [Низкая] Drop у Registration молча отбрасывает ошибки

- **Файл**: `ecat-registry/src/lib.rs:50-52`
- **Серьёзность**: **низкая**
- **Влияние**: сбой дерегистрации сервиса остаётся незамеченным

```rust
impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(reg) = self.registry.take() {
            let id = self.id.clone();
            tokio::spawn(async move {
                let _ = reg.deregister(&id).await;  // 错误被静默丢弃
            });
        }
    }
}
```

Хотя в Drop нельзя блокировать, сбой дерегистрации можно фиксировать через `tracing::warn!`.

---

### Проблема 4: [Низкая] Обработка особых значений f64 в `ecat-data-sqlx`

- **Файл**: `ecat-data-sqlx/src/lib.rs:57-61`
- **Серьёзность**: **низкая**
- **Влияние**: значения NaN/Infinity в БД преобразуются в Null

```rust
row.try_get::<f64, _>(col.as_str())
    .ok()
    .and_then(serde_json::Number::from_f64)  // NaN/Inf → None
    .map(serde_json::Value::Number)
    .ok_or(())
```

`serde_json::Number::from_f64()` возвращает `None` для `f64::NAN`, `f64::INFINITY` и `f64::NEG_INFINITY`, из-за чего эти значения деградируют до Null.

---

## 3. Заметки ревью по crates

### ecat (ядро) — 4 файла
| Файл | Статус | Заметка |
|------|------|------|
| `lib.rs` | ✅ | Разделение start_hooks/stop_hooks корректно |
| `hook.rs` | ✅ | Blanket impl замыканий покрывает on_start/on_stop |
| `signal.rs` | ⚠️ | `.expect()` в SIGTERM handler-е разумен, но строг |

### ecat-transport — 4 файла
| Файл | Статус | Заметка |
|------|------|------|
| `lib.rs` | ✅ | Дизайн trait Server лаконичен |
| `context.rs` | ✅ | Уже используется `tokio::sync::RwLock` |
| `request.rs` | ✅ | |
| `response.rs` | ✅ | |

### ecat-transport-http / ecat-transport-grpc — 2 файла
| Файл | Статус | Заметка |
|------|------|------|
| `ecat-transport-http/src/lib.rs` | ⚠️ | `start()` блокируется и не возвращается, `stop()` — no-op (известное ограничение) |
| `ecat-transport-grpc/src/lib.rs` | ⚠️ | То же самое |

### ecat-middleware — 5 файлов
| Файл | Статус | Заметка |
|------|------|------|
| `tracing.rs` | ✅ | Исправление `fut.instrument(span)` корректно |
| `recovery.rs` | ⚠️ | `tokio::task::spawn` теряет контекст span (проблема 2) |
| `logging.rs` | ✅ | `elapsed.as_millis() as u64` — теоретическое усечение без фактического влияния |
| `timeout.rs` | ✅ | |

### ecat-registry — 2 файла
| Файл | Статус | Заметка |
|------|------|------|
| `lib.rs` | ⚠️ | Drop у Registration молча отбрасывает ошибки (проблема 3) |
| `memory.rs` | ⚠️ | Синхронный `std::sync::RwLock` в асинхронном контексте (известное ограничение) |

### ecat-config — 3 файла
| Файл | Статус | Заметка |
|------|------|------|
| `lib.rs` | ✅ | Дизайн trait Config разумен |
| `env.rs` | ✅ | Порядок парсинга типов корректен (bool→i64→f64→String) |
| `file.rs` | ⚠️ | Не поддерживаются multi-document YAML, нет механизма watch (известное ограничение) |

### ecat-data — 6 файлов
| Файл | Статус | Заметка |
|------|------|------|
| `rdbms.rs` | ✅ | Комментарий Drop у Transaction объясняет автооткат, но тела нет |
| `cache.rs` | ✅ | Определение trait полное |
| `graph.rs` | ✅ | |
| `search.rs` | ✅ | |
| `tsdb.rs` | ✅ | Паттерн builder у DataPoint хорошо спроектирован |

### ecat-data-sqlx — 1 файл
| Файл | Статус | Заметка |
|------|------|------|
| `lib.rs` | ⚠️ | Порядок извлечения значений исправлен; transaction не реализован; особые значения f64 (проблема 4) |

### ecat-errors — 2 файла
| Файл | Статус | Заметка |
|------|------|------|
| `lib.rs` | ✅ | Маппинг gRPC→ErrorCode полный, формат Display понятен |
| `codes.rs` | ✅ | Сопоставление HTTP-статусов согласовано с семантикой gRPC |

### ecat-encoding — 3 файла
| Файл | Статус | Заметка |
|------|------|------|
| `lib.rs` | ✅ | enum CodecBox, codec_for/codec_from_content_type — хорошо спроектированы |
| `json.rs` | ✅ | |
| `proto.rs` | ⚠️ | ProtoCodec — заглушка (известное ограничение) |

### Прочие crates
| Crate | Статус | Заметка |
|-------|------|------|
| `ecat-logging` | ✅ | `try_init` защищает от повторной инициализации |
| `ecat-metadata` | ✅ | Двусторонняя конвертация HTTP/gRPC завершена |
| `ecat-metrics` | ⚠️ | В `metrics_text()` есть unwrap() (проблема 1) |
| `ecat-protos` | ✅ | Генерация кода prost/tonic |
| `ecat-cli` | ⚠️ | Большинство команд только печатают сообщения, файлы не создаются (известное ограничение) |
| `examples/helloworld` | ✅ | Пример корректно использует новый API |

---

## 4. Анализ покрытия тестами

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
  остальные 8 crates    0   (чистый trait/генерация кода/нужны интеграционные тесты)
```

### Пробелы в тестах

| Приоритет | Crate | Чего не хватает |
|--------|-------|----------|
| Высокий | `ecat-middleware` | Нет юнит-тестов у 4 Tower Service |
| Высокий | `ecat-data-sqlx` | Нет интеграционных тестов (подойдёт in-memory SQLite) |
| Средний | `ecat-transport-http` | Не тестируется процедура запуска HTTP-сервера |
| Средний | `ecat-transport-grpc` | Не тестируется процедура запуска gRPC-сервера |
| Низкий | `ecat-data` | Чистые определения trait, приемлемо |

---

## 5. Метрики качества кода

| Метрика | Значение | Оценка |
|------|-----|------|
| Всего строк | 2151 | — |
| Предупреждения компиляции | 0 | ✅ |
| Предупреждения Clippy | 0 | ✅ |
| Пройденные тесты | 60/60 | ✅ |
| Покрытие тестами (оценка) | ~35% | ⚠️ |
| unwrap() вне тестов | 2 места (metrics) | ⚠️ |
| Небезопасный код | 0 | ✅ |
| Точки риска panic | 3 места (metrics×2 + expect в signal) | ⚠️ |

---

## 6. Сводка рекомендаций по исправлению

### Рекомендованные исправления (этот раунд — все исправлены ✅)

| # | Файл | Проблема | Приоритет | Статус |
|---|------|------|--------|------|
| 1 | `ecat-metrics/src/lib.rs:14-15` | unwrap в `metrics_text()` → деградация | Средний | ✅ Исправлено |
| 2 | `ecat-registry/src/lib.rs:51` | Добавить `tracing::warn!` при сбое deregister в Drop | Низкий | ✅ Исправлено |
| 3 | `ecat-data-sqlx/src/lib.rs:57-61` | Особая обработка значений f64 NaN/Inf | Низкий | ✅ Исправлено |
| 4 | `ecat-middleware/src/recovery.rs:40` | `tokio::task::spawn` теряет span → `fut.instrument(span)` | Низкий | ✅ Исправлено |
| 5 | `ecat-registry/src/memory.rs` | Синхронный RwLock → `tokio::sync::RwLock` | Низкий | ✅ Исправлено |

### Известные ограничения (не блокируют)

| # | Файл | Описание |
|---|------|------|
| K1 | `ecat-transport-http` / `ecat-transport-grpc` | start() блокируется / stop() — no-op (нужен graceful shutdown) |
| K2 | `ecat-data-sqlx` | `transaction()` возвращает ошибку «не реализовано» |
| K3 | `ecat-middleware` | Нет юнит-тестов у 4 Service |
| K4 | `ecat-config/file.rs` | Нет механизма watch |
| K5 | `ecat-encoding/proto.rs` | ProtoCodec — заглушка |
| K6 | `ecat-cli` | Большинство команд — mock-вывод |

---

## 7. Итоги

Третий раунд ревью проведён после полных исправлений R2. В этом раунде найдены 5 проблем — все исправлены.

Сравнение с R2:
- R2: 2 бага высокой + 1 средней серьёзности → все исправлены ✅
- R3: 1 проблема средней + 4 низкой серьёзности (надёжность) → все исправлены ✅
- Количество тестов осталось 60

### Дальнейшие приоритетные рекомендации

1. Добавить интеграционные тесты SQLite для `ecat-data-sqlx`
2. Добавить юнит-тесты для `ecat-middleware` (проверка поведения span/таймаута/восстановления)
3. Реализовать graceful shutdown HTTP/gRPC серверов
