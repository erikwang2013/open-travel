# Plan del ecosistema de e-cat

**Versión:** 2.1.7  
**Fecha:** 2026-08-01  
**Estado:** todo completado · 47 crates

| Ámbito | Cubierto | Estado |
|------|--------|------|
| Capa de transporte | HTTP (axum), gRPC (tonic), WebSocket | ✅ |
| Codificación | JSON, Protobuf | ✅ |
| Middleware | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth (JWT/API Key/OAuth2) | ✅ |
| Configuración | env, file (JSON/YAML), Consul KV remoto, cifrado | ✅ |
| Registro | memory, Consul, etcd | ✅ |
| Seguridad | detección de ataques, JWT, API Key, OAuth2, TlsConfig | ✅ |
| Datos | RDBMS (sqlx), Redis, Memcached, OpenSearch, Elasticsearch, ClickHouse, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB | ✅ |
| Observabilidad | tracing, Prometheus, Health, trazado distribuido | ✅ |
| Comunicación | cliente HTTP/gRPC, MessageQueue (InMemory/Kafka), EventBus | ✅ |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | ✅ |
| Herramientas de API | OpenAPI, Versioning, GraphQL | ✅ |

## Brechas restantes (3 pequeñas optimizaciones)

1. **mTLS en transport** — TlsConfig ya existe, aún no está conectado a HttpServer/GrpcServer
2. **Backend de límite de tasa para Redis** — RateLimitLayer es solo en memoria; las instancias múltiples necesitan compartirlo
3. **Plantilla de GitLab CI** — actualmente solo GitHub Actions

## Evolución de versiones

```
v1.0.x  Núcleo base (18 crates)                    ✅
v2.0.x  Ecosistema fases 1 a 3 (+13 crates)         ✅
v2.1.x  Refuerzo de comunicaciones y seguridad + backends de datos + experiencia operativa  ✅ (actual)
```

## Fuera del ecosistema

| Necesidad | Solución | Razón |
|------|------|------|
| API Gateway | Kong / Envoy | independiente del lenguaje |
| Service mesh | Linkerd | Rust no tiene solución madura |
| Orquestación de contenedores | Kubernetes | estándar de la industria |
| Recopilación de logs | Vector | nativo en Rust |
