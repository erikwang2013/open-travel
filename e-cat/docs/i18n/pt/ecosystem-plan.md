# Plano de ecossistema do e-cat

**Versão:** 2.1.7  
**Data:** 2026-08-01  
**Status:** tudo concluído · 47 crates

| Domínio | Coberto | Status |
|------|--------|------|
| Camada de transporte | HTTP (axum), gRPC (tonic), WebSocket | ✅ |
| Encoding | JSON, Protobuf | ✅ |
| Middleware | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth (JWT/API Key/OAuth2) | ✅ |
| Configuração | env, file (JSON/YAML), Consul KV remoto, criptografia | ✅ |
| Registro | memory, Consul, etcd | ✅ |
| Segurança | Detecção de ataques, JWT, API Key, OAuth2, TlsConfig | ✅ |
| Dados | RDBMS (sqlx), Redis, Memcached, OpenSearch, Elasticsearch, ClickHouse, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB | ✅ |
| Observabilidade | tracing, Prometheus, Health, rastreamento distribuído | ✅ |
| Comunicação | HTTP/gRPC Client, MessageQueue (InMemory/Kafka), EventBus | ✅ |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | ✅ |
| Ferramentas de API | OpenAPI, Versioning, GraphQL | ✅ |

## Lacunas restantes (3 otimizações menores)

1. **mTLS no transport** — TlsConfig já existe, ainda não conectado ao HttpServer/GrpcServer
2. **Backend de rate limit Redis** — RateLimitLayer apenas em memória, multi-instância precisa de compartilhamento
3. **Template de CI GitLab** — atualmente apenas GitHub Actions

## Evolução de versões

```
v1.0.x  Esqueleto central (18 crates)                    ✅
v2.0.x  Ecossistema fases 1 a 3 (+13 crates)              ✅
v2.1.x  Comunicação e segurança reforçadas + backends de dados completos + experiência de operação   ✅ (atual)
```

## Fora do ecossistema

| Necessidade | Solução | Motivo |
|------|------|------|
| API Gateway | Kong / Envoy | Independente de linguagem |
| Service mesh | Linkerd | Rust não tem solução madura |
| Orquestração de contêineres | Kubernetes | Padrão da indústria |
| Coleta de logs | Vector | Nativo em Rust |
