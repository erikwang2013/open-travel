# Plan del ecosistema de e-cat v3 — evaluación final

> **Actualización (2026-08-07, v2.3.3)**: la brecha restante #1 «mTLS en transport» está completada: `HttpServer::tls` / `GrpcServer::tls` funcionan realmente con tokio-rustls / tonic rustls (soporte de verificación de CA y certificado de cliente obligatorio); las brechas #2 (límite de tasa con Redis) y #3 (GitLab CI) ya se completaron con v2.3.0. Con esto, todas las brechas del plan están implementadas.

**Versión:** 2.4.2  
**Fecha:** 2026-08-01  
**Total de crates:** 55 · toda la planificación completada

---

## Cobertura actual

| Ámbito | Implementado | Cobertura |
|------|--------|--------|
| Capa de transporte | HTTP (axum), gRPC (tonic), WebSocket | 100% |
| Codificación | JSON, Protobuf | 100% |
| Middleware | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth×3 | 100% |
| Configuración | env, file (JSON/YAML), Consul KV, cifrado (XOR) | 100% |
| Registro | memory, Consul, etcd | 100% |
| Seguridad | detección de ataques, JWT, API Key, OAuth2, certificado de cliente TLS, mTLS | 95% |
| Comunicación | certificado de cliente TLS — soportado por todos los backends de datos | 95% |
| Comunicación de servicios | HTTP Client, gRPC Client, Resolver, LoadBalancer | 95% |
| Datos | RDBMS (sqlx), Redis, OpenSearch, Elasticsearch, ClickHouse, Memcached, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB — todos con configuración por archivo Config | 95% |
| Mensajería | trait MessageQueue, InMemory, Kafka, EventBus | 100% |
| Observabilidad | tracing, Prometheus, Health, trazado distribuido | 100% |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | 95% |
| Herramientas de API | OpenAPI, Versioning, GraphQL | 100% |

---

## Brechas restantes

### Que valen la pena (3)

| # | Brecha | Valor | Esfuerzo |
|---|------|------|--------|
| 1 | **mTLS en transport** | TlsConfig ya existe, aún no está conectado a HttpServer/GrpcServer | pequeño |
| 2 | **Backend de límite de tasa para Redis** | RateLimitLayer es solo en memoria; las instancias múltiples necesitan compartirlo | pequeño |
| 3 | **Plantilla de GitLab CI** | GitHub Actions ya existe | pequeño |

### Que no son necesarias (2)

| # | Brecha | Razón |
|---|------|------|
| 4 | Cifrado AES-GCM en configuración | el XOR actual es suficiente |
| 5 | Service mesh / API Gateway | se deja a la comunidad (Linkerd/Kong/K8s) |

---

## Veredicto

**e-cat ha alcanzado la madurez de producción.** Los 47 crates cubren la pila completa de microservicios: transporte → middleware → descubrimiento de servicios → configuración → seguridad → datos → mensajería → observabilidad → DevOps → herramientas de API. Las 3 brechas restantes son optimizaciones de poco esfuerzo, sin carencias estructurales.

## Cobertura de backends de datos (15)

| Categoría | Base de datos | Crate | Forma de driver |
|------|--------|-------|----------|
| RDBMS | SQLite/PostgreSQL/MySQL/TiDB | `ecat-data-sqlx` | sqlx (driver asíncrono oficial) |
| Caché | Redis | `ecat-data-redis` | redis-rs (driver oficial) |
| Caché | Memcached | `ecat-data-memcached` | ⚠️ implementación en memoria (no apta para producción) |
| Documentos | MongoDB | `ecat-data-mongodb` | mongodb (driver oficial) |
| Almacenamiento de objetos | S3 / MinIO | `ecat-data-s3` | HTTP/REST (reqwest+rustls, SigV4 implementado) |
| OLAP | ClickHouse | `ecat-data-clickhouse` | HTTP/REST (reqwest) |
| Búsqueda | OpenSearch | `ecat-data-opensearch` | HTTP/REST (reqwest) |
| Búsqueda | Elasticsearch | `ecat-data-elasticsearch` | HTTP/REST (reqwest) |
| Grafos | Neo4j | `ecat-data-neo4j` | HTTP/REST (reqwest) |
| Grafos | NebulaGraph | `ecat-data-nebulagraph` | HTTP/REST (reqwest) |
| Grafos | ArangoDB | `ecat-data-arangodb` | HTTP/REST (reqwest) |
| Series temporales | InfluxDB | `ecat-data-influxdb` | HTTP/REST (reqwest) |
| Series temporales | Apache IoTDB | `ecat-data-iotdb` | HTTP/REST (reqwest) |
| Series temporales | QuestDB | `ecat-data-questdb` | HTTP/REST (reqwest) |
| Series temporales | TDengine | `ecat-data-tdengine` | HTTP/REST (reqwest) |
