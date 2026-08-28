# Plan d'écosystème e-cat v2 — terminé et suite

**Version :** 2.1.7  
**Date :** 2026-08-01  
**Statut :** tout le plan est terminé, 47 crates

---

## I. Terminé (tout livré)

| Phase | Crate | Capacité | Tests |
|------|-------|------|------|
| Phase 1 | `ecat-health` | Contrôles de santé (/health, /ready) | 4 |
| Phase 1 | `ecat-client` | Clients HTTP/gRPC + découverte de services + équilibrage de charge | 7 |
| Phase 1 | `ecat-circuit-breaker` | Circuit-breaker à trois états (Tower Layer) | 4 |
| Phase 1 | `ecat-auth` | Middleware d'authentification JWT + API Key + OAuth2 | 8 |
| Phase 1 | `ecat-registry-consul` | Enregistrement de services Consul | 2 |
| Phase 2 | `ecat-data-redis` | Cache Redis (trait Cache) | 1 |
| Phase 2 | `ecat-mq` | Abstraction de file de messages + InMemoryMq | 2 |
| Phase 2 | `ecat-events` | Bus d'événements local + distant | 2 |
| Phase 2 | `ecat-config-remote` | Configuration distante Consul KV | 2 |
| Phase 3 | `ecat-testing` | MockServer + ChaosConfig | 5 |
| Phase 3 | `ecat-openapi` | Génération de spec OpenAPI 3.0 | 2 |
| Phase 3 | `ecat-bench` | Benchmark de performance concurrente | 2 |
| Phase 3 | `ecat-deploy` | Dockerfile + K8s + Helm | — |
| Phase 4 | `ecat-tracing` | Traçage distribué (span + trace_id) | 2 |
| Phase 4 | `ecat-client` (extension) | GrpcClient + TlsConfig | — |
| Phase 4 | `ecat-auth` (extension) | OAuth2Layer | — |
| Phase 5 | `ecat-registry-etcd` | Enregistrement de services etcd | 4 |
| Phase 5 | `ecat-mq-kafka` | File de messages Kafka | 1 |
| Phase 5 | `ecat-data-opensearch` | Recherche OpenSearch | 1 |
| Phase 5 | `ecat-data-influxdb` | Séries temporelles InfluxDB | 2 |
| Phase 5 | `ecat-data-elasticsearch` | Recherche Elasticsearch | 2 |
| Phase 5 | `ecat-data-clickhouse` | OLAP ClickHouse | 1 |
| Phase 5 | `ecat-data-memcached` | Cache Memcached | 3 |
| Phase 5 | `ecat-data-neo4j` | Base de graphes Neo4j | 1 |
| Phase 5 | `ecat-data-nebulagraph` | Base de graphes NebulaGraph | 1 |
| Phase 5 | `ecat-data-arangodb` | Base de graphes ArangoDB | 1 |
| Phase 5 | `ecat-data-iotdb` | Séries temporelles IoTDB | 1 |
| Phase 5 | `ecat-data-questdb` | Séries temporelles QuestDB | 1 |
| Phase 6 | `ecat-transport-ws` | Prise en charge WebSocket | 2 |
| Phase 6 | `ecat-versioning` | Routage par version d'API | 2 |
| Phase 6 | `ecat-graphql` | Endpoint GraphQL | 9 |
| Phase 6 | Modèles CI/CD | GitHub Actions | — |

---

## II. Lacunes restantes (3 éléments)

| # | Lacune | Charge de travail |
|---|------|--------|
| 1 | **Intégration mTLS au transport** | Faible |
| 2 | **Backend de limitation Redis** | Faible |
| 3 | **Modèles GitLab CI** | Faible |

---

## III. Feuille de route des versions

```
v1.0.x  Squelette central (18 crates)                    ✅ Terminé
v2.0.x  Écosystème phases 1 à 3 (+13 crates = 31 au total)   ✅ Terminé
v2.1.x  Communication et sécurité + backends de données + expérience d'exploitation             ✅ Terminé (actuellement 47 crates)
```

## IV. Hors de l'écosystème

| Besoin | Solution | Raison |
|------|------|------|
| Passerelle API | Kong / Envoy | Indépendant du langage |
| Service mesh | Linkerd | Pas de solution mature en Rust |
| Orchestration de conteneurs | Kubernetes | Standard de l'industrie |
| Collecte de logs | Vector | Natif Rust |
