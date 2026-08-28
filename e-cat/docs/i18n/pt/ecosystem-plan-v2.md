# Plano de ecossistema do e-cat v2 — concluído e próximos passos

**Versão:** 2.1.7  
**Data:** 2026-08-01  
**Status:** todo o planejamento concluído, 47 crates

---

## 1. Concluído (tudo entregue)

| Fase | Crate | Capacidade | Testes |
|------|-------|------|------|
| Fase 1 | `ecat-health` | Health check (/health, /ready) | 4 |
| Fase 1 | `ecat-client` | Cliente HTTP/gRPC + descoberta de serviço + balanceamento de carga | 7 |
| Fase 1 | `ecat-circuit-breaker` | Circuit breaker de três estados (Tower Layer) | 4 |
| Fase 1 | `ecat-auth` | Middlewares de autenticação JWT + API Key + OAuth2 | 8 |
| Fase 1 | `ecat-registry-consul` | Registro de serviço Consul | 2 |
| Fase 2 | `ecat-data-redis` | Cache Redis (trait Cache) | 1 |
| Fase 2 | `ecat-mq` | Abstração de fila de mensagens + InMemoryMq | 2 |
| Fase 2 | `ecat-events` | Barramento de eventos local + remoto | 2 |
| Fase 2 | `ecat-config-remote` | Configuração remota Consul KV | 2 |
| Fase 3 | `ecat-testing` | MockServer + ChaosConfig | 5 |
| Fase 3 | `ecat-openapi` | Geração de spec OpenAPI 3.0 | 2 |
| Fase 3 | `ecat-bench` | Benchmark de desempenho concorrente | 2 |
| Fase 3 | `ecat-deploy` | Dockerfile + K8s + Helm | — |
| Fase 4 | `ecat-tracing` | Rastreamento distribuído (span + trace_id) | 2 |
| Fase 4 | extensão de `ecat-client` | GrpcClient + TlsConfig | — |
| Fase 4 | extensão de `ecat-auth` | OAuth2Layer | — |
| Fase 5 | `ecat-registry-etcd` | Registro de serviço etcd | 4 |
| Fase 5 | `ecat-mq-kafka` | Fila de mensagens Kafka | 1 |
| Fase 5 | `ecat-data-opensearch` | Busca OpenSearch | 1 |
| Fase 5 | `ecat-data-influxdb` | Séries temporais InfluxDB | 2 |
| Fase 5 | `ecat-data-elasticsearch` | Busca Elasticsearch | 2 |
| Fase 5 | `ecat-data-clickhouse` | OLAP ClickHouse | 1 |
| Fase 5 | `ecat-data-memcached` | Cache Memcached | 3 |
| Fase 5 | `ecat-data-neo4j` | Grafo Neo4j | 1 |
| Fase 5 | `ecat-data-nebulagraph` | Grafo NebulaGraph | 1 |
| Fase 5 | `ecat-data-arangodb` | Grafo ArangoDB | 1 |
| Fase 5 | `ecat-data-iotdb` | Séries temporais IoTDB | 1 |
| Fase 5 | `ecat-data-questdb` | Séries temporais QuestDB | 1 |
| Fase 6 | `ecat-transport-ws` | Suporte a WebSocket | 2 |
| Fase 6 | `ecat-versioning` | Roteamento de versão de API | 2 |
| Fase 6 | `ecat-graphql` | Endpoint GraphQL | 9 |
| Fase 6 | Templates CI/CD | GitHub Actions | — |

---

## 2. Lacunas restantes (3 itens)

| # | Lacuna | Esforço |
|---|------|--------|
| 1 | **mTLS no transport** | Pequeno |
| 2 | **Backend de rate limit Redis** | Pequeno |
| 3 | **Template de CI GitLab** | Pequeno |

---

## 3. Roadmap de versões

```
v1.0.x  Esqueleto central (18 crates)                    ✅ concluído
v2.0.x  Ecossistema fases 1 a 3 (+13 crates = 31 total)   ✅ concluído
v2.1.x  Comunicação e segurança + backends de dados + experiência de operação  ✅ concluído (atualmente 47 crates)
```

## 4. Fora do ecossistema

| Necessidade | Solução | Motivo |
|------|------|------|
| API Gateway | Kong / Envoy | Independente de linguagem |
| Service mesh | Linkerd | Rust não tem solução madura |
| Orquestração de contêineres | Kubernetes | Padrão da indústria |
| Coleta de logs | Vector | Nativo em Rust |
