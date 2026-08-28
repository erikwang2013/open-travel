<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat API Reference

This page summarizes the API surface of the Ecat framework: port conventions, built-in endpoints, error formats, and extension interfaces. Business routes are registered by each service itself.

## Port Conventions

| Protocol | Listen address | Notes |
|------|----------|------|
| HTTP | `0.0.0.0:8000` | axum routes, default example port |
| gRPC | `0.0.0.0:9000` | tonic Server, default example port |

## Built-in Endpoints

The following endpoints are provided by ecosystem crates and mounted with the service:

| Endpoint | Source | Notes |
|------|------|------|
| `/health` | ecat-health | Liveness check (returns service name, version, start time) |
| `/ready` | ecat-health | Readiness check (returns 200 once dependencies are ready) |
| `/metrics` | ecat-metrics | Prometheus metrics exposure (`ecat_http_requests_total` / `ecat_http_request_duration_seconds`) |
| `/{service}/{method}` | User routes | Example: `/helloworld/ecat` |

> For high-cardinality scenarios where metric endpoint paths contain IDs, use `MetricsLayer::new().with_path_fn(...)` to normalize and avoid metric cardinality explosion.

## Request Handling Flow

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

## Error Format

`ecat-errors` provides `ErrorCode` + `Error` with compile-time HTTP status mapping:

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

Error responses are encoded as JSON (or Protobuf) by middleware, carrying code / reason / message.

## Extension Interfaces

| Capability | Crate | Interface |
|------|-------|------|
| GraphQL | ecat-graphql | `/graphql` endpoint; supports field arguments and nested selections, does not support aliases, fragments, or multiple top-level fields |
| OpenAPI | ecat-openapi | Generate OpenAPI spec from routes |
| WebSocket | ecat-transport-ws | Upgraded WS transport |
| API version routing | ecat-versioning | `/v1/...` prefixed version routes |
| Authentication | ecat-auth | JWT / API Key middleware; JWT secret must be ≥32 bytes, chainable `required_issuer`/`required_audience` |
| gRPC client | ecat-transport-grpc | Integrated service discovery and load balancing |

## Inter-service Communication

- `HttpClient` (ecat-client): integrates service discovery and load balancing, with CircuitBreaker protection
- `GrpcClient` (ecat-transport-grpc): same, over the gRPC protocol
- Middleware is uniformly composed via `tower::ServiceBuilder` (Recovery / Tracing / Logging / Timeout / RateLimit / Security / CircuitBreaker / Metrics / Retry / Validate / CORS)

## Data Backend Interfaces

All data backends (`ecat-data-*`) are abstracted through unified traits (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`); REST-style backends (Neo4j / NebulaGraph / ArangoDB / InfluxDB / IoTDB / QuestDB / TDengine / OpenSearch / Elasticsearch / S3) access their corresponding HTTP interfaces via `base_url`. Connection configuration: see [Database Configuration Tutorial](database-config-tutorial.md).
