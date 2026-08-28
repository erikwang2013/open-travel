<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Справочник API Ecat

На этой странице собраны интерфейсы (API) фреймворка Ecat: соглашения о портах, встроенные эндпоинты, формат ошибок и интерфейсы расширений. Бизнес-маршруты регистрируются каждым сервисом самостоятельно.

## Соглашения о портах

| Протокол | Адрес прослушивания | Описание |
|------|----------|------|
| HTTP | `0.0.0.0:8000` | Маршруты axum, порт примера по умолчанию |
| gRPC | `0.0.0.0:9000` | tonic Server, порт примера по умолчанию |

## Встроенные эндпоинты

Следующие эндпоинты предоставляются экосистемными crate-ами и монтируются вместе с сервисом:

| Эндпоинт | Источник | Описание |
|------|------|------|
| `/health` | ecat-health | Проверка живости (возвращает имя сервиса, версию, время запуска) |
| `/ready` | ecat-health | Проверка готовности (возвращает 200, когда зависимости готовы) |
| `/metrics` | ecat-metrics | Выдача метрик Prometheus (`ecat_http_requests_total` / `ecat_http_request_duration_seconds`) |
| `/{service}/{method}` | Маршруты пользователя | Пример: `/helloworld/ecat` |

> В сценариях высокой кардинальности путей (например, пути с ID) используйте `MetricsLayer::new().with_path_fn(...)` для нормализации и предотвращения взрыва кардинальности метрик.

## Поток обработки запроса

```
客户端请求
  ├─ HTTP :8000 ──→ axum::Router ─┐
  └─ gRPC :9000 ──→ tonic::Server ─┤
                              ┌─────┴──────┐
                              │ Middleware │  Recovery→Tracing→Logging→Auth→Metrics→Security→CircuitBreaker
                              └─────┬──────┘
                                    ▼
                               Handler（tower::Service）
                                    ▼
                               Response（JSON/Protobuf 编码）
```

## Формат ошибок

`ecat-errors` предоставляет `ErrorCode` + `Error` с сопоставлением HTTP-статусов на этапе компиляции:

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

Ответы с ошибками кодируются middleware в JSON (или Protobuf) и содержат code / reason / message.

## Интерфейсы расширений

| Возможность | Crate | Интерфейс |
|------|-------|------|
| GraphQL | ecat-graphql | Эндпоинт `/graphql`; поддерживает параметры полей и вложенные selection, не поддерживает алиасы, fragment-ы и несколько полей верхнего уровня |
| OpenAPI | ecat-openapi | Генерация OpenAPI spec из маршрутов |
| WebSocket | ecat-transport-ws | Апгрейднутый WS-транспорт |
| Версионирование API | ecat-versioning | Версионирование маршрутов с префиксом `/v1/...` |
| Аутентификация | ecat-auth | Middleware JWT / API Key; ключ JWT должен быть ≥32 байт, доступны цепочные `required_issuer`/`required_audience` |
| gRPC-клиент | ecat-transport-grpc | Интеграция service discovery и балансировки нагрузки |

## Взаимодействие между сервисами

- `HttpClient` (ecat-client): интеграция service discovery и балансировки нагрузки, защита CircuitBreaker
- `GrpcClient` (ecat-transport-grpc): то же самое, по протоколу gRPC
- Middleware единообразно комбинируются через `tower::ServiceBuilder` (Recovery / Tracing / Logging / Timeout / RateLimit / Security / CircuitBreaker / Metrics / Retry / Validate / CORS)

## Интерфейсы бэкендов данных

Все бэкенды данных (`ecat-data-*`) абстрагированы через единые trait-ы (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`); REST-подобные бэкенды (Neo4j / NebulaGraph / ArangoDB / InfluxDB / IoTDB / QuestDB / TDengine / OpenSearch / Elasticsearch / S3) обращаются к соответствующим HTTP-интерфейсам через `base_url`. Настройка подключения — в [Руководстве по настройке баз данных](database-config-tutorial.md).
