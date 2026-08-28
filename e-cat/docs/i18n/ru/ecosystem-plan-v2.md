# Экосистемный план e-cat v2 — выполнено и далее

**Версия:** 2.1.7  
**Дата:** 2026-08-01  
**Статус:** весь план выполнен, 47 crates

---

## 1. Выполнено (всё сдано)

| Этап | Crate | Возможность | Тесты |
|------|-------|------|------|
| Этап 1 | `ecat-health` | Проверки здоровья (/health, /ready) | 4 |
| Этап 1 | `ecat-client` | HTTP/gRPC клиент + service discovery + балансировка нагрузки | 7 |
| Этап 1 | `ecat-circuit-breaker` | Трёхсостоянийный circuit breaker (Tower Layer) | 4 |
| Этап 1 | `ecat-auth` | Middleware аутентификации JWT + API Key + OAuth2 | 8 |
| Этап 1 | `ecat-registry-consul` | Регистрация сервисов Consul | 2 |
| Этап 2 | `ecat-data-redis` | Redis-кэш (trait Cache) | 1 |
| Этап 2 | `ecat-mq` | Абстракция очередей сообщений + InMemoryMq | 2 |
| Этап 2 | `ecat-events` | Локальная + удалённая шина событий | 2 |
| Этап 2 | `ecat-config-remote` | Удалённая конфигурация Consul KV | 2 |
| Этап 3 | `ecat-testing` | MockServer + ChaosConfig | 5 |
| Этап 3 | `ecat-openapi` | Генерация OpenAPI 3.0 spec | 2 |
| Этап 3 | `ecat-bench` | Бенчмарки конкурентности | 2 |
| Этап 3 | `ecat-deploy` | Dockerfile + K8s + Helm | — |
| Этап 4 | `ecat-tracing` | Распределённая трассировка (span + trace_id) | 2 |
| Этап 4 | расширение `ecat-client` | GrpcClient + TlsConfig | — |
| Этап 4 | расширение `ecat-auth` | OAuth2Layer | — |
| Этап 5 | `ecat-registry-etcd` | Регистрация сервисов etcd | 4 |
| Этап 5 | `ecat-mq-kafka` | Очередь сообщений Kafka | 1 |
| Этап 5 | `ecat-data-opensearch` | Поиск OpenSearch | 1 |
| Этап 5 | `ecat-data-influxdb` | Временные ряды InfluxDB | 2 |
| Этап 5 | `ecat-data-elasticsearch` | Поиск Elasticsearch | 2 |
| Этап 5 | `ecat-data-clickhouse` | OLAP ClickHouse | 1 |
| Этап 5 | `ecat-data-memcached` | Кэш Memcached | 3 |
| Этап 5 | `ecat-data-neo4j` | Графовая БД Neo4j | 1 |
| Этап 5 | `ecat-data-nebulagraph` | Графовая БД NebulaGraph | 1 |
| Этап 5 | `ecat-data-arangodb` | Графовая БД ArangoDB | 1 |
| Этап 5 | `ecat-data-iotdb` | Временные ряды IoTDB | 1 |
| Этап 5 | `ecat-data-questdb` | Временные ряды QuestDB | 1 |
| Этап 6 | `ecat-transport-ws` | Поддержка WebSocket | 2 |
| Этап 6 | `ecat-versioning` | Версионирование API-маршрутов | 2 |
| Этап 6 | `ecat-graphql` | GraphQL endpoint | 9 |
| Этап 6 | шаблоны CI/CD | GitHub Actions | — |

---

## 2. Оставшиеся пробелы (3 пункта)

| # | Пробел | Объём работы |
|---|------|--------|
| 1 | **mTLS в transport** | Малый |
| 2 | **Бэкенд rate limit на Redis** | Малый |
| 3 | **Шаблон GitLab CI** | Малый |

---

## 3. Дорожная карта версий

```
v1.0.x  Базовый скелет (18 crates)                    ✅ Выполнено
v2.0.x  Экосистема, этапы 1–3 (+13 crates = 31 total)  ✅ Выполнено
v2.1.x  Коммуникации и безопасность + бэкенды данных + эксплуатация ✅ Выполнено (текущие 47 crates)
```

## 4. Что не входит в экосистему

| Потребность | Решение | Причина |
|------|------|------|
| API-шлюз | Kong / Envoy | Независимо от языка |
| Service mesh | Linkerd | Нет зрелого решения на Rust |
| Оркестрация контейнеров | Kubernetes | Отраслевой стандарт |
| Сбор логов | Vector | Нативный Rust |
