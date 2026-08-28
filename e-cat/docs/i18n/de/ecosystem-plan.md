# e-cat Ökosystemplan

**Version:** 2.1.7  
**Datum:** 2026-08-01  
**Status:** vollständig abgeschlossen · 47 crates

| Bereich | Abgedeckt | Status |
|------|--------|------|
| Transport | HTTP (axum), gRPC (tonic), WebSocket | ✅ |
| Codierung | JSON, Protobuf | ✅ |
| Middleware | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth (JWT/API Key/OAuth2) | ✅ |
| Konfiguration | env, file (JSON/YAML), Consul KV remote, verschlüsselt | ✅ |
| Registrierung | memory, Consul, etcd | ✅ |
| Sicherheit | Angriffserkennung, JWT, API Key, OAuth2, TlsConfig | ✅ |
| Daten | RDBMS (sqlx), Redis, Memcached, OpenSearch, Elasticsearch, ClickHouse, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB | ✅ |
| Beobachtbarkeit | tracing, Prometheus, Health, verteilte Ablaufverfolgung | ✅ |
| Kommunikation | HTTP/gRPC-Client, MessageQueue (InMemory/Kafka), EventBus | ✅ |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | ✅ |
| API-Werkzeuge | OpenAPI, Versioning, GraphQL | ✅ |

## Verbleibende Lücken (3 kleine Optimierungen)

1. **mTLS-Anbindung an transport** — TlsConfig vorhanden, noch nicht an HttpServer/GrpcServer angebunden
2. **Redis-Rate-Limit-Backend** — RateLimitLayer nur im Speicher, für mehrere Instanzen müsste es geteilt werden
3. **GitLab-CI-Vorlage** — aktuell nur GitHub Actions

## Versionsentwicklung

```
v1.0.x  Kern-Gerüst (18 crates)                      ✅
v2.0.x  Ökosystem Phase 1–3 (+13 crates)             ✅
v2.1.x  Kommunikation & Sicherheit gestärkt + Daten-Backends + Betriebserlebnis  ✅ (aktuell)
```

## Nicht ins Ökosystem aufgenommen

| Anforderung | Lösung | Grund |
|------|------|------|
| API-Gateway | Kong / Envoy | sprachunabhängig |
| Service Mesh | Linkerd | kein ausgereiftes Rust-Angebot |
| Container-Orchestrierung | Kubernetes | Branchenstandard |
| Log-Sammlung | Vector | Rust-nativ |
