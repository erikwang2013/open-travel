<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Referência da API do Ecat

Esta página resume a superfície de interface (API) do framework Ecat: convenções de porta, endpoints embutidos, formato de erro e interfaces de extensão. As rotas de negócio são registradas por cada serviço.

## Convenções de porta

| Protocolo | Endereço de escuta | Descrição |
|------|----------|------|
| HTTP | `0.0.0.0:8000` | Rotas axum, porta padrão dos exemplos |
| gRPC | `0.0.0.0:9000` | Servidor tonic, porta padrão dos exemplos |

## Endpoints embutidos

Os seguintes endpoints são fornecidos pelos crates do ecossistema e montados junto com o serviço:

| Endpoint | Origem | Descrição |
|------|------|------|
| `/health` | ecat-health | Verificação de liveness (retorna nome do serviço, versão, tempo de inicialização) |
| `/ready` | ecat-health | Verificação de readiness (retorna 200 quando as dependências estão prontas) |
| `/metrics` | ecat-metrics | Exposição de métricas Prometheus (`ecat_http_requests_total` / `ecat_http_request_duration_seconds`) |
| `/{service}/{method}` | Rotas do usuário | Exemplo: `/helloworld/ecat` |

> Em cenários de alta cardinalidade (ex.: caminhos contendo IDs), use `MetricsLayer::new().with_path_fn(...)` para normalizar e evitar explosão de cardinalidade de métricas.

## Fluxo de processamento de requisições

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

## Formato de erro

`ecat-errors` fornece `ErrorCode` + `Error`, com mapeamento de status HTTP em tempo de compilação:

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

A resposta de erro é codificada como JSON (ou Protobuf) pelo middleware, carregando code / reason / message.

## Interfaces de extensão

| Capacidade | Crate | Interface |
|------|-------|------|
| GraphQL | ecat-graphql | Endpoint `/graphql`; suporta argumentos de campo e selections aninhadas; não suporta aliases, fragments nem múltiplos campos de nível superior |
| OpenAPI | ecat-openapi | Gera spec OpenAPI a partir das rotas |
| WebSocket | ecat-transport-ws | Transporte WS atualizado (upgrade) |
| Roteamento de versão de API | ecat-versioning | Roteamento por prefixo de versão `/v1/...` |
| Autenticação | ecat-auth | Middlewares JWT / API Key; a chave JWT deve ter ≥32 bytes, com `required_issuer`/`required_audience` encadeáveis |
| Cliente gRPC | ecat-transport-grpc | Integra descoberta de serviço e balanceamento de carga |

## Comunicação entre serviços

- `HttpClient` (ecat-client): integra descoberta de serviço e balanceamento de carga, com proteção do CircuitBreaker
- `GrpcClient` (ecat-transport-grpc): o mesmo, via protocolo gRPC
- Middlewares combinados de forma unificada com `tower::ServiceBuilder` (Recovery / Tracing / Logging / Timeout / RateLimit / Security / CircuitBreaker / Metrics / Retry / Validate / CORS)

## Interfaces de backend de dados

Todos os backends de dados (`ecat-data-*`) são abstraídos por traits unificados (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`); backends do tipo REST (Neo4j / NebulaGraph / ArangoDB / InfluxDB / IoTDB / QuestDB / TDengine / OpenSearch / Elasticsearch / S3) acessam as interfaces HTTP correspondentes via `base_url`. Consulte o [Tutorial de configuração de banco de dados](database-config-tutorial.md) para a configuração de conexão.
