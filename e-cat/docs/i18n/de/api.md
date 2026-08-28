<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat API-Referenz

Diese Seite fasst die Schnittstellen (API) des Ecat-Frameworks zusammen: Port-Konventionen, eingebaute Endpunkte, Fehlerformat und Erweiterungsschnittstellen. Geschäfts-Routen werden von den jeweiligen Diensten selbst registriert.

## Port-Konventionen

| Protokoll | Lauschadresse | Beschreibung |
|------|----------|------|
| HTTP | `0.0.0.0:8000` | axum-Routing, Standard-Port im Beispiel |
| gRPC | `0.0.0.0:9000` | tonic Server, Standard-Port im Beispiel |

## Eingebaute Endpunkte

Die folgenden Endpunkte werden von den Ökosystem-Crates bereitgestellt und mit dem Dienst gemountet:

| Endpunkt | Quelle | Beschreibung |
|------|------|------|
| `/health` | ecat-health | Liveness-Check (liefert Dienstname, Version, Startzeit) |
| `/ready` | ecat-health | Readiness-Check (liefert 200, sobald Abhängigkeiten bereit sind) |
| `/metrics` | ecat-metrics | Prometheus-Metrik-Export (`ecat_http_requests_total` / `ecat_http_request_duration_seconds`) |
| `/{service}/{method}` | Benutzer-Routen | Beispiel: `/helloworld/ecat` |

> Bei Hochkardinalität wie IDs im Metrik-Endpunkt-Pfad bitte `MetricsLayer::new().with_path_fn(...)` zur Normalisierung verwenden, um Metrik-Kardinalitätsexplosion zu vermeiden.

## Anfrageverarbeitungsablauf

```
Client-Anfrage
  ├─ HTTP :8000 ──→ axum::Router ─┐
  └─ gRPC :9000 ──→ tonic::Server ─┤
                              ┌─────┴──────┐
                              │ Middleware │  Recovery→Tracing→Logging→Auth→Metrics→Security→CircuitBreaker
                              └─────┬──────┘
                                    ▼
                               Handler (tower::Service)
                                    ▼
                               Response (JSON/Protobuf-Codierung)
```

## Fehlerformat

`ecat-errors` bietet `ErrorCode` + `Error` mit HTTP-Statuscode-Zuordnung zur Compilezeit:

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

Fehler-Responses werden über die Middleware als JSON (oder Protobuf) codiert und tragen code / reason / message.

## Erweiterungsschnittstellen

| Fähigkeit | Crate | Schnittstelle |
|------|-------|------|
| GraphQL | ecat-graphql | `/graphql`-Endpunkt; unterstützt Feldargumente und verschachtelte Selections, keine Aliase, Fragmente oder mehrere Top-Level-Felder |
| OpenAPI | ecat-openapi | Generiert OpenAPI-spec aus den Routen |
| WebSocket | ecat-transport-ws | Aktualisierter WS-Transport |
| API-Versions-Routing | ecat-versioning | Versions-Routing mit `/v1/...`-Präfix |
| Authentifizierung | ecat-auth | JWT-/API-Key-Middleware; JWT-Schlüssel muss ≥32 Bytes sein, verkettbar `required_issuer`/`required_audience` |
| gRPC-Client | ecat-transport-grpc | Integriert Service Discovery und Load Balancing |

## Dienstkommunikation

- `HttpClient` (ecat-client): integriert Service Discovery und Load Balancing, Schutz durch CircuitBreaker
- `GrpcClient` (ecat-transport-grpc): wie oben, gRPC-Protokoll
- Middleware wird einheitlich über `tower::ServiceBuilder` kombiniert (Recovery / Tracing / Logging / Timeout / RateLimit / Security / CircuitBreaker / Metrics / Retry / Validate / CORS)

## Daten-Backend-Schnittstellen

Alle Daten-Backends (`ecat-data-*`) sind über einheitliche Traits abstrahiert (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`); REST-artige Backends (Neo4j / NebulaGraph / ArangoDB / InfluxDB / IoTDB / QuestDB / TDengine / OpenSearch / Elasticsearch / S3) greifen über `base_url` auf die jeweiligen HTTP-Schnittstellen zu. Verbindungskonfiguration siehe [Tutorial zur Datenbankkonfiguration](database-config-tutorial.md).
