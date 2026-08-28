<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Referencia de API de Ecat

Esta página resume la superficie de interfaz (API) del framework Ecat: convenciones de puertos, endpoints integrados, formato de errores e interfaces extensibles. Las rutas de negocio son registradas por cada servicio.

## Convenciones de puertos

| Protocolo | Dirección de escucha | Descripción |
|------|----------|------|
| HTTP | `0.0.0.0:8000` | Router axum, puerto de ejemplo por defecto |
| gRPC | `0.0.0.0:9000` | Servidor tonic, puerto de ejemplo por defecto |

## Endpoints integrados

Los siguientes endpoints los proporcionan los crates del ecosistema y se montan junto con el servicio:

| Endpoint | Origen | Descripción |
|------|------|------|
| `/health` | ecat-health | Comprobación de liveness (devuelve nombre del servicio, versión, tiempo de arranque) |
| `/ready` | ecat-health | Comprobación de readiness (devuelve 200 cuando las dependencias están listas) |
| `/metrics` | ecat-metrics | Exposición de métricas Prometheus (`ecat_http_requests_total` / `ecat_http_request_duration_seconds`) |
| `/{service}/{method}` | Rutas del usuario | Ejemplo: `/helloworld/ecat` |

> En escenarios de alta cardinalidad (rutas que contienen IDs, etc.), usa `MetricsLayer::new().with_path_fn(...)` para normalizar y evitar la explosión de cardinalidad de métricas.

## Flujo de procesamiento de peticiones

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

## Formato de errores

`ecat-errors` proporciona `ErrorCode` + `Error`, con mapeo de códigos de estado HTTP en tiempo de compilación:

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

La respuesta de error se codifica como JSON (o Protobuf) a través del middleware, con code / reason / message.

## Interfaces extensibles

| Capacidad | Crate | Interfaz |
|------|-------|------|
| GraphQL | ecat-graphql | Endpoint `/graphql`; admite parámetros de campo y selections anidadas; no admite alias, fragments ni múltiples campos de nivel superior |
| OpenAPI | ecat-openapi | Genera el spec OpenAPI a partir de las rutas |
| WebSocket | ecat-transport-ws | Transporte WS actualizado |
| Enrutado por versión de API | ecat-versioning | Enrutado con prefijo de versión `/v1/...` |
| Autenticación | ecat-auth | Middleware JWT / API Key; la clave JWT debe tener ≥32 bytes, admite encadenar `required_issuer`/`required_audience` |
| Cliente gRPC | ecat-transport-grpc | Integra descubrimiento de servicios y balanceo de carga |

## Comunicación entre servicios

- `HttpClient` (ecat-client): integra descubrimiento de servicios y balanceo de carga, con protección por disyuntor CircuitBreaker
- `GrpcClient` (ecat-transport-grpc): igual, sobre protocolo gRPC
- Los middleware se componen unificadamente con `tower::ServiceBuilder` (Recovery / Tracing / Logging / Timeout / RateLimit / Security / CircuitBreaker / Metrics / Retry / Validate / CORS)

## Interfaces de backends de datos

Todos los backends de datos (`ecat-data-*`) se abstraen mediante traits unificados (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`); los backends tipo REST (Neo4j / NebulaGraph / ArangoDB / InfluxDB / IoTDB / QuestDB / TDengine / OpenSearch / Elasticsearch / S3) acceden a sus interfaces HTTP correspondientes a través de `base_url`. Consulta la configuración de conexión en el [Tutorial de configuración de base de datos](database-config-tutorial.md).
