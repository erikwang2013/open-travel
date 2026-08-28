# Экосистемный план e-cat

**Версия:** 2.1.7  
**Дата:** 2026-08-01  
**Статус:** всё выполнено · 47 crates

| Область | Покрытие | Статус |
|------|--------|------|
| Транспорт | HTTP (axum), gRPC (tonic), WebSocket | ✅ |
| Кодирование | JSON, Protobuf | ✅ |
| Middleware | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth (JWT/API Key/OAuth2) | ✅ |
| Конфигурация | env, file (JSON/YAML), Consul KV remote, шифрование | ✅ |
| Регистрация | memory, Consul, etcd | ✅ |
| Безопасность | Обнаружение атак, JWT, API Key, OAuth2, TlsConfig | ✅ |
| Данные | RDBMS (sqlx), Redis, Memcached, OpenSearch, Elasticsearch, ClickHouse, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB | ✅ |
| Наблюдаемость | tracing, Prometheus, Health, распределённая трассировка | ✅ |
| Коммуникации | HTTP/gRPC Client, MessageQueue (InMemory/Kafka), EventBus | ✅ |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | ✅ |
| API-инструменты | OpenAPI, Versioning, GraphQL | ✅ |

## Оставшиеся пробелы (3 небольшие оптимизации)

1. **mTLS в transport** — TlsConfig уже есть, не подключён к HttpServer/GrpcServer
2. **Бэкенд rate limit на Redis** — RateLimitLayer только в памяти, для нескольких инстансов нужен общий
3. **Шаблон GitLab CI** — сейчас только GitHub Actions

## Эволюция версий

```
v1.0.x  Базовый скелет (18 crates)                    ✅
v2.0.x  Экосистема, этапы 1–3 (+13 crates)             ✅
v2.1.x  Коммуникации и безопасность + бэкенды данных + эксплуатация ✅ (текущая)
```

## Что не входит в экосистему

| Потребность | Решение | Причина |
|------|------|------|
| API-шлюз | Kong / Envoy | Независимо от языка |
| Service mesh | Linkerd | Нет зрелого решения на Rust |
| Оркестрация контейнеров | Kubernetes | Отраслевой стандарт |
| Сбор логов | Vector | Нативный Rust |
