# Отчёт о тестировании — 2026-08-26

Полное дописывание юнит-тестов (покрытие всех 51 crates), 4 группы старших Rust-тест-инженеров параллельно.

## Обзор

| Группа | crates | Было | Добавлено | Стало | Ворота |
|---|---|---|---|---|---|
| core/фреймворк | 12 | 102 | +40 | 142 | ✅ тесты зелёные + clippy 0 предупреждений |
| data | 14 | 87 | +66 | 153 | ✅ то же |
| mq/transport | 12 | 82 | +54 | 136 | ✅ то же |
| app прикладной уровень | 13 | ~178 | +46 | ~224 | ✅ то же |
| **Итого** | **51** | **~449** | **+206** | **~655** | ✅ |

Примечание: «было» на прикладном уровне включает ecat-auth 24 / ecat-graphql 35 / ecat-bench 11 / ecat-scheduler 6 / ecat-events 7 / ecat-cli 12 / ecat-middleware 34 / ecat-circuit-breaker 10 / ecat-health 4 / ecat-client 7 / ecat-security 12 / ecat-versioning 4 / ecat 4. Для каждого crate отдельно `cargo test -p` + `cargo clippy -p --all-targets -- -D warnings` проходят; параллельный запуск с изоляцией CARGO_TARGET_DIR.

## Детализация по crates

### Группа core/фреймворк (test-core, +40)

| crate | Было→Стало | Ключевые точки покрытия |
|---|---|---|
| ecat-protos | 4→8 | ErrorCode — полное сопоставление enum с proto; decode усечённого buffer; пустой buffer — сообщение по умолчанию; roundtrip metadata |
| ecat-errors | 4→9 | Полное отображение http_status (409/429/500); from_status; неотображённое→Internal; cause source() |
| ecat-metadata | 9→12 | Извлечение trace_id из HTTP-заголовков; приведение ключа к нижнему регистру; пустой header map |
| ecat-encoding | 18→22 | NaN→null (по умолчанию serde_json, документировано); decode пустых байт; CodecBox невалидный JSON; roundtrip proto |
| ecat-lock | 7→9 | release без удержания блокировки — ошибка; пустой ключ |
| ecat-logging | 1→1 | Совместимый shim не паникует |
| ecat-tracing | 9→12 | Пропуск не-UTF8 trace-заголовка; canonical-заголовок; проброс ответа |
| ecat-tls | 7→12 | basic_auth одно/два поля; отсутствующий ca-файл; is_enabled; клиент по умолчанию |
| ecat-config | 14→26 | Фильтрация префикса env + границы разбора типов (hex/пустая строка/-0/1e3); слияние и перекрытие нескольких source; пути ошибок obfs; отсутствующий файл/невалидный YAML |
| ecat-config-remote | 6→9 | Границы ConsulKvEntry; ошибка при отсутствии X-Consul-Index; вложенные ключи |
| ecat-openapi | 4→11 | components/schema_ref; повторное перекрытие; 200 по умолчанию; tags |
| ecat-metrics | 8→11 | Текст уже зарегистрированных метрик; 404/405 |

### Группа data (test-data, +66)

| crate | Было→Стало | Ключевые точки покрытия |
|---|---|---|
| ecat-data | 12→14 | Разбор синтаксиса поиска |
| ecat-data-sqlx | 7→14 | Сквозной in-memory SQLite; привязка параметров всех типов; Blob→base64; config |
| ecat-data-redis | 6→12 | Построение URL redis:///rediss://; auth; пути ошибок config |
| ecat-data-opensearch | 4→10 | mock HTTP: percent-encode, Basic auth, проброс ошибок |
| ecat-data-elasticsearch | 6→11 | То же |
| ecat-data-influxdb | 5→10 | Экранирование line protocol; заголовок Token; проброс ошибок |
| ecat-data-clickhouse | 12→22 | SQL создания таблицы; JSONEachRow; число записанных строк; группировка |
| ecat-data-memcached | 4→8 | TTL секунды→миллисекунды; упаковка flag |
| ecat-data-nebulagraph | 6→7 | Разбор config |
| ecat-data-arangodb | 5→7 | config/URL |
| ecat-data-iotdb | 5→10 | mock HTTP: параметры session-пути |
| ecat-data-questdb | 4→9 | line protocol; транзакции не поддерживаются |
| ecat-data-tdengine | 6→11 | Генерация INSERT; пакетная разбивка по 100 |
| ecat-data-mongodb | 5→8 | Roundtrip bson; URI |

### Группа mq/transport/registry (test-mq, +54)

| crate | Было→Стало | Ключевые точки покрытия |
|---|---|---|
| ecat-mq | 5→9 | Полный буфер — кадр ошибки задержки; закрытие потока при полном drop; несколько подписчиков; publish без подписчиков |
| ecat-mq-kafka | 12→14 | Config по умолчанию; поля SASL действуют независимо |
| ecat-mq-rabbitmq | 2→5 | exchange по умолчанию; пути ошибок url |
| ecat-mq-mqtt | 5→9 | Проверка парности cert/key; отсутствующий файл; порты по умолчанию 1883/8883; откат при невалидном порте |
| ecat-mq-nats | 6→9 | Обычный текст по умолчанию; пути ошибок отсутствующих ca/cert |
| ecat-transport | 4→7 | TlsConfig по умолчанию/with_client_auth; границы normalize_addr |
| ecat-transport-http | 17→20 | Интеграционные тесты: stop — пустышка, занятый порт — неудача, реальный приём/передача |
| ecat-transport-grpc | 7→13 | TLS с отсутствующим файлом; жизненный цикл plaintext; отклонение mTLS |
| ecat-transport-ws | 4→8 | Без handler — неудача; занятый порт; эхо masked-кадров RFC 6455 |
| ecat-registry | 5→8 | discover нескольких экземпляров; авторазрегистрация при drop; builder по умолчанию |
| ecat-registry-consul | 10→24 | percent-encode; варианты регистрации; ответы об ошибках; X-Consul-Token; разбор agent/services; откат node |
| ecat-registry-etcd | 5→10 | discover пропускает битые значения; тело kv-запроса; lease grant; keepalive |

### Группа app прикладной уровень (test-app, +46)

| crate | Было→Стало | Ключевые точки покрытия |
|---|---|---|
| ecat-auth | 20→46 | Белый список кэша oauth2/SHA-256 ключи/FIFO-вытеснение; три состояния apikey; принудительные iss/aud в jwt; истёкшие/неверно подписанные |
| ecat-health | 4→8 | Агрегация readiness (все ok/любой fail/пустой реестр); liveness |
| ecat-versioning | 4→7 | Маршрутизация path-стратегии; границы extract_version |
| ecat-security | 12→20 | Сквозной header-слой; JSON-форма блокировки атак |
| ecat-middleware | 34→37 | Истечение окна MemoryStore; panic внутреннего слоя→Err |
| ecat-circuit-breaker | 10→12 | Исчерпание зонда half-open; деградация classify |
| ecat-client | 7→10 | grpc с невалидным эндпоинтом — ошибка без сети |
| ecat-graphql | 35→35 | Существующее покрытие достаточное, пробелов нет |
| ecat-scheduler / ecat-bench / ecat-events / ecat-cli / ecat | Существующее покрытие достаточное | Пробелов нет |

## Обнаруженные дефекты

| Уровень | Место | Описание | Статус |
|---|---|---|---|
| P1 | ecat-events/Cargo.toml | В dev-dependencies не хватает tokio features macros/rt/time — компиляция тестовой цели этого crate отдельно гарантированно падает (полная сборка workspace маскирует объединением feature) | ✅ Исправлено (дополнены features + комментарий) |
| P2 | ecat-security src/lib.rs:118-127 | SQLi с percent-кодированным URI (`?q=SELECT%20*%20...`) обходит header-сканирование (детектор требует литеральный пробел, сканирует сырой URI без декодирования); сканирование тела не затронуто | ⏳ Ожидает исправления |
| P3 | ecat-data-sqlx | `connect()/from_config()` используют AnyPool без установленного драйвера — sqlx 0.8.6 при первом подключении паникует "No drivers installed" | ⏳ Ожидает исправления |
| P3 | ecat-data-influxdb | Строковый field экранирует пробел (`\ `), спецификация line protocol требует экранировать только `"` и `\`; порядок tag/field недетерминирован | ⏳ Ожидает исправления |
| P3 | ecat-data-clickhouse | Кэш создания таблицы никогда не инвалидируется — после внешнего drop/изменения таблицы CREATE не повторяется | ⏳ Ожидает исправления |
| P3 | ecat-circuit-breaker | Верхняя граница half_open_probes недостижима при последовательном зондировании (достижима только при параллельных в полёте); покрыто white-box тестом | ℹ️ Известно, не дефект |
| P3 | ecat-health | `with_check` использует blocking_write() — вызов из async-контекста паникует; сейчас доступен только синхронный контекст | ℹ️ Известно, ограничение API |

## Пропущенные модули (нужна интеграционная среда, не мокались)

- Реальные roundtrip брокеров: kafka/rabbitmq/mqtt/nats publish-subscribe (покрыты конфигурация и пути ошибок)
- Реальные кластеры: жизненный цикл consul/etcd регистрация-обнаружение (axum mock покрывает форму запросов)
- Реальные базы данных: операции redis/memcached, mongod, серверная проверка influxdb, драйверы sqlx postgres/mysql, API nebulagraph/arangodb
- Реальные внешние сервисы: интроспекция OAuth2 (покрыта локальным mock), roundtrip gRPC/HTTP (локальный mock покрывает 302 без перехода)
