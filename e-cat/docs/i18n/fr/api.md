<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Référence API d'Ecat

Cette page récapitule la surface d'interface (API) du framework Ecat : conventions de ports, endpoints intégrés, format d'erreur et interfaces d'extension. Les routes métier sont enregistrées par chaque service.

## Conventions de ports

| Protocole | Adresse d'écoute | Description |
|------|----------|------|
| HTTP | `0.0.0.0:8000` | Routes axum, port d'exemple par défaut |
| gRPC | `0.0.0.0:9000` | Serveur tonic, port d'exemple par défaut |

## Endpoints intégrés

Les endpoints suivants sont fournis par les crates d'écosystème et montés avec le service :

| Endpoint | Source | Description |
|------|------|------|
| `/health` | ecat-health | Contrôle de survie (renvoie le nom du service, la version, l'heure de démarrage) |
| `/ready` | ecat-health | Contrôle de disponibilité (renvoie 200 quand les dépendances sont prêtes) |
| `/metrics` | ecat-metrics | Exposition des métriques Prometheus (`ecat_http_requests_total` / `ecat_http_request_duration_seconds`) |
| `/{service}/{method}` | Routes utilisateur | Exemple : `/helloworld/ecat` |

> Pour les chemins de métriques à haute cardinalité (contenant des ID, etc.), utilisez `MetricsLayer::new().with_path_fn(...)` pour normaliser et éviter l'explosion de cardinalité.

## Flux de traitement des requêtes

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

## Format d'erreur

`ecat-errors` fournit `ErrorCode` + `Error`, avec mappage du statut HTTP à la compilation :

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

La réponse d'erreur est encodée par le middleware en JSON (ou Protobuf), avec code / reason / message.

## Interfaces d'extension

| Capacité | Crate | Interface |
|------|-------|------|
| GraphQL | ecat-graphql | Endpoint `/graphql` ; prend en charge les paramètres de champ et les sélections imbriquées, pas les alias, fragments ni champs multiples de niveau supérieur |
| OpenAPI | ecat-openapi | Génère une spec OpenAPI depuis les routes |
| WebSocket | ecat-transport-ws | Transport WS mis à niveau |
| Routage par version d'API | ecat-versioning | Routage par version avec préfixe `/v1/...` |
| Authentification | ecat-auth | Middleware JWT / API Key ; la clé JWT doit faire ≥32 octets, chaînage possible `required_issuer`/`required_audience` |
| Client gRPC | ecat-transport-grpc | Intégration de la découverte de services et de l'équilibrage de charge |

## Communication inter-services

- `HttpClient` (ecat-client) : intégration de la découverte de services et de l'équilibrage de charge, protection par CircuitBreaker
- `GrpcClient` (ecat-transport-grpc) : idem, protocole gRPC
- Les middleware sont composés uniformément avec `tower::ServiceBuilder` (Recovery / Tracing / Logging / Timeout / RateLimit / Security / CircuitBreaker / Metrics / Retry / Validate / CORS)

## Interfaces des backends de données

Tous les backends de données (`ecat-data-*`) sont abstraits par des traits unifiés (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`) ; les backends de type REST (Neo4j / NebulaGraph / ArangoDB / InfluxDB / IoTDB / QuestDB / TDengine / OpenSearch / Elasticsearch / S3) accèdent à l'interface HTTP correspondante via `base_url`. Voir [Tutoriel de configuration des bases de données](database-config-tutorial.md) pour la configuration de connexion.
