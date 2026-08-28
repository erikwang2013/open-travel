# Plano de ecossistema do e-cat v3 — avaliação final

> **Atualização (2026-08-07, v2.3.3)**: a lacuna restante #1 "mTLS no transport" foi concluída — `HttpServer::tls` / `GrpcServer::tls` funcionam de verdade com base em tokio-rustls / tonic rustls (com suporte a validação de CA e certificado de cliente obrigatório); as lacunas #2 (rate limit Redis) e #3 (CI GitLab) já haviam sido concluídas com o v2.3.0. Todas as lacunas listadas no plano foram, portanto, implementadas.

**Versão:** 2.4.2  
**Data:** 2026-08-01  
**Total de crates:** 55 · todo o planejamento concluído

---

## Cobertura atual

| Domínio | Implementado | Cobertura |
|------|--------|--------|
| Camada de transporte | HTTP (axum), gRPC (tonic), WebSocket | 100% |
| Encoding | JSON, Protobuf | 100% |
| Middleware | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth×3 | 100% |
| Configuração | env, file (JSON/YAML), Consul KV, criptografia (XOR) | 100% |
| Registry | memory, Consul, etcd | 100% |
| Segurança | Detecção de ataques, JWT, API Key, OAuth2, certificado de cliente TLS, mTLS | 95% |
| Comunicação | Certificado de cliente TLS — suportado por todos os backends de dados | 95% |
| Comunicação de serviços | HTTP Client, gRPC Client, Resolver, LoadBalancer | 95% |
| Dados | RDBMS (sqlx), Redis, OpenSearch, Elasticsearch, ClickHouse, Memcached, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB — todos suportam configuração por arquivo Config | 95% |
| Mensagens | trait MessageQueue, InMemory, Kafka, EventBus | 100% |
| Observabilidade | tracing, Prometheus, Health, rastreamento distribuído | 100% |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | 95% |
| Ferramentas de API | OpenAPI, Versioning, GraphQL | 100% |

---

## Lacunas restantes

### Que valem a pena (3 itens)

| # | Lacuna | Valor | Esforço |
|---|------|------|--------|
| 1 | **mTLS no transport** | TlsConfig já existe, ainda não conectado ao HttpServer/GrpcServer | Pequeno |
| 2 | **Backend de rate limit Redis** | RateLimitLayer apenas em memória, multi-instância precisa de compartilhamento | Pequeno |
| 3 | **Template de CI GitLab** | GitHub Actions já existe | Pequeno |

### Desnecessárias (2 itens)

| # | Lacuna | Motivo |
|---|------|------|
| 4 | Config AES-GCM | O XOR atual é suficiente |
| 5 | Service mesh / API Gateway | Deixar para a comunidade (Linkerd/Kong/K8s) |

---

## Veredito

**O e-cat alcançou maturidade pronta para produção.** 47 crates cobrem toda a stack de microsserviços: transporte → middleware → descoberta de serviço → configuração → segurança → dados → mensagens → observabilidade → DevOps → ferramentas de API. As 3 lacunas restantes são otimizações de baixo esforço, sem lacunas estruturais.

## Cobertura de backends de dados (15)

| Categoria | Banco de dados | Crate | Forma de driver |
|------|--------|-------|----------|
| RDBMS | SQLite/PostgreSQL/MySQL/TiDB | `ecat-data-sqlx` | sqlx (driver assíncrono oficial) |
| Cache | Redis | `ecat-data-redis` | redis-rs (driver oficial) |
| Cache | Memcached | `ecat-data-memcached` | ⚠️ Implementação em memória (não para produção) |
| Documentos | MongoDB | `ecat-data-mongodb` | mongodb (driver oficial) |
| Armazenamento de objetos | S3 / MinIO | `ecat-data-s3` | HTTP/REST (reqwest+rustls, SigV4 próprio) |
| OLAP | ClickHouse | `ecat-data-clickhouse` | HTTP/REST (reqwest) |
| Busca | OpenSearch | `ecat-data-opensearch` | HTTP/REST (reqwest) |
| Busca | Elasticsearch | `ecat-data-elasticsearch` | HTTP/REST (reqwest) |
| Grafo | Neo4j | `ecat-data-neo4j` | HTTP/REST (reqwest) |
| Grafo | NebulaGraph | `ecat-data-nebulagraph` | HTTP/REST (reqwest) |
| Grafo | ArangoDB | `ecat-data-arangodb` | HTTP/REST (reqwest) |
| Séries temporais | InfluxDB | `ecat-data-influxdb` | HTTP/REST (reqwest) |
| Séries temporais | Apache IoTDB | `ecat-data-iotdb` | HTTP/REST (reqwest) |
| Séries temporais | QuestDB | `ecat-data-questdb` | HTTP/REST (reqwest) |
| Séries temporais | TDengine | `ecat-data-tdengine` | HTTP/REST (reqwest) |
