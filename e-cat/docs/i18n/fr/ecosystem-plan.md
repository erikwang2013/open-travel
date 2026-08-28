# Plan d'écosystème e-cat

**Version :** 2.1.7  
**Date :** 2026-08-01  
**Statut :** entièrement terminé · 47 crates

| Domaine | Couvert | Statut |
|------|--------|------|
| Couche transport | HTTP (axum), gRPC (tonic), WebSocket | ✅ |
| Encodage | JSON, Protobuf | ✅ |
| Middleware | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth (JWT/API Key/OAuth2) | ✅ |
| Configuration | env, file (JSON/YAML), Consul KV distant, chiffrement | ✅ |
| Registre | memory, Consul, etcd | ✅ |
| Sécurité | Détection d'attaques, JWT, API Key, OAuth2, TlsConfig | ✅ |
| Données | RDBMS (sqlx), Redis, Memcached, OpenSearch, Elasticsearch, ClickHouse, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB | ✅ |
| Observabilité | tracing, Prometheus, Health, traçage distribué | ✅ |
| Communication | Client HTTP/gRPC, MessageQueue (InMemory/Kafka), EventBus | ✅ |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | ✅ |
| Outils API | OpenAPI, Versioning, GraphQL | ✅ |

## Lacunes restantes (3 petites optimisations)

1. **Intégration mTLS au transport** — TlsConfig existe déjà, pas encore branché sur HttpServer/GrpcServer
2. **Backend de limitation Redis** — RateLimitLayer est en mémoire uniquement, un partage est nécessaire pour plusieurs instances
3. **Modèles GitLab CI** — actuellement GitHub Actions uniquement

## Évolution des versions

```
v1.0.x  Squelette central (18 crates)                    ✅
v2.0.x  Écosystème phases 1 à 3 (+13 crates)              ✅
v2.1.x  Renforcement communication et sécurité + complétion des backends de données + expérience d'exploitation   ✅ (actuel)
```

## Hors de l'écosystème

| Besoin | Solution | Raison |
|------|------|------|
| Passerelle API | Kong / Envoy | Indépendant du langage |
| Service mesh | Linkerd | Pas de solution mature en Rust |
| Orchestration de conteneurs | Kubernetes | Standard de l'industrie |
| Collecte de logs | Vector | Natif Rust |
