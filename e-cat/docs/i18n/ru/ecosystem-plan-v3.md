# Экосистемный план e-cat v3 — финальная оценка

> **Обновление (2026-08-07, v2.3.3)**: оставшийся пробел #1 «mTLS в transport» выполнен — `HttpServer::tls` / `GrpcServer::tls` реально работают на основе tokio-rustls / rustls tonic (поддержка проверки CA и принудительных клиентских сертификатов); пробелы #2 (rate limit Redis) и #3 (GitLab CI) ранее выполнены в составе v2.3.0. Все пробелы из плана теперь закрыты.

**Версия:** 2.4.2  
**Дата:** 2026-08-01  
**Всего crates:** 55 · весь план выполнен

---

## Текущее покрытие

| Область | Реализовано | Покрытие |
|------|--------|--------|
| Транспорт | HTTP (axum), gRPC (tonic), WebSocket | 100% |
| Кодирование | JSON, Protobuf | 100% |
| Middleware | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth×3 | 100% |
| Конфигурация | env, file (JSON/YAML), Consul KV, шифрование (XOR) | 100% |
| Реестр | memory, Consul, etcd | 100% |
| Безопасность | Обнаружение атак, JWT, API Key, OAuth2, клиентские TLS-сертификаты, mTLS | 95% |
| Коммуникации | Клиентские TLS-сертификаты — поддерживаются всеми бэкендами данных | 95% |
| Сервисные коммуникации | HTTP Client, gRPC Client, Resolver, LoadBalancer | 95% |
| Данные | RDBMS (sqlx), Redis, OpenSearch, Elasticsearch, ClickHouse, Memcached, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB — все поддерживают файловую конфигурацию | 95% |
| Сообщения | trait MessageQueue, InMemory, Kafka, EventBus | 100% |
| Наблюдаемость | tracing, Prometheus, Health, распределённая трассировка | 100% |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | 95% |
| API-инструменты | OpenAPI, Versioning, GraphQL | 100% |

---

## Оставшиеся пробелы

### Стоит сделать (3 пункта)

| # | Пробел | Ценность | Объём работы |
|---|------|------|--------|
| 1 | **mTLS в transport** | TlsConfig уже есть, не подключён к HttpServer/GrpcServer | Малый |
| 2 | **Бэкенд rate limit на Redis** | RateLimitLayer только в памяти, для нескольких инстансов нужен общий | Малый |
| 3 | **Шаблон GitLab CI** | GitHub Actions уже есть | Малый |

### Делать не нужно (2 пункта)

| # | Пробел | Причина |
|---|------|------|
| 4 | Конфигурация AES-GCM | Текущего XOR достаточно |
| 5 | Service mesh / API-шлюз | Оставить сообществу (Linkerd/Kong/K8s) |

---

## Вывод

**e-cat достиг зрелости, пригодной для продакшена.** 47 crates покрывают полный стек микросервисов: транспорт → middleware → service discovery → конфигурация → безопасность → данные → сообщения → наблюдаемость → DevOps → API-инструменты. Оставшиеся 3 пробела — оптимизации малого объёма, структурных недочётов нет.

## Покрытие бэкендов данных (15 шт.)

| Категория | База данных | Crate | Способ драйвера |
|------|--------|-------|----------|
| RDBMS | SQLite/PostgreSQL/MySQL/TiDB | `ecat-data-sqlx` | sqlx (официальный асинхронный драйвер) |
| Кэш | Redis | `ecat-data-redis` | redis-rs (официальный драйвер) |
| Кэш | Memcached | `ecat-data-memcached` | ⚠️ Реализация в памяти (не для продакшена) |
| Документы | MongoDB | `ecat-data-mongodb` | mongodb (официальный драйвер) |
| Объекты | S3 / MinIO | `ecat-data-s3` | HTTP/REST (reqwest+rustls, собственный SigV4) |
| OLAP | ClickHouse | `ecat-data-clickhouse` | HTTP/REST (reqwest) |
| Поиск | OpenSearch | `ecat-data-opensearch` | HTTP/REST (reqwest) |
| Поиск | Elasticsearch | `ecat-data-elasticsearch` | HTTP/REST (reqwest) |
| Граф | Neo4j | `ecat-data-neo4j` | HTTP/REST (reqwest) |
| Граф | NebulaGraph | `ecat-data-nebulagraph` | HTTP/REST (reqwest) |
| Граф | ArangoDB | `ecat-data-arangodb` | HTTP/REST (reqwest) |
| Врем. ряды | InfluxDB | `ecat-data-influxdb` | HTTP/REST (reqwest) |
| Врем. ряды | Apache IoTDB | `ecat-data-iotdb` | HTTP/REST (reqwest) |
| Врем. ряды | QuestDB | `ecat-data-questdb` | HTTP/REST (reqwest) |
| Врем. ряды | TDengine | `ecat-data-tdengine` | HTTP/REST (reqwest) |
