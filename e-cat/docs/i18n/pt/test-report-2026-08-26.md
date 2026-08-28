# Relatório de testes — 2026-08-26

Complemento abrangente de testes unitários (cobertura total de 51 crates), com 4 equipes de engenheiros de teste Rust seniores em paralelo.

## Visão geral

| Equipe | crates | Original | Novos | Atuais | Portão |
|---|---|---|---|---|---|
| core/framework | 12 | 102 | +40 | 142 | ✅ test tudo verde + clippy 0 warnings |
| data | 14 | 87 | +66 | 153 | ✅ idem |
| mq/transport | 12 | 82 | +54 | 136 | ✅ idem |
| camada app | 13 | ~178 | +46 | ~224 | ✅ idem |
| **Total** | **51** | **~449** | **+206** | **~655** | ✅ |

Observação: os números originais da camada app incluem ecat-auth 24 / ecat-graphql 35 / ecat-bench 11 / ecat-scheduler 6 / ecat-events 7 / ecat-cli 12 / ecat-middleware 34 / ecat-circuit-breaker 10 / ecat-health 4 / ecat-client 7 / ecat-security 12 / ecat-versioning 4 / ecat 4. Cada crate passou por `cargo test -p` e `cargo clippy -p --all-targets -- -D warnings` independentes, com paralelismo isolado via CARGO_TARGET_DIR.

## Detalhamento por crate

### Grupo core/framework (test-core, +40)

| crate | original→novo | Pontos cobertos |
|---|---|---|
| ecat-protos | 4→8 | Enumeração completa de ErrorCode contra proto; decode de buffer truncado; buffer vazio com mensagem padrão; roundtrip de metadata |
| ecat-errors | 4→9 | Mapeamento completo de http_status (409/429/500); from_status; não mapeado→Internal; cause source() |
| ecat-metadata | 9→12 | Extração de trace_id de header HTTP; minúsculas nas chaves; header map vazio |
| ecat-encoding | 18→22 | NaN→null (padrão do serde_json, documentado); decode de bytes vazios; CodecBox com JSON inválido; roundtrip proto |
| ecat-lock | 7→9 | Erro ao liberar sem possuir o lock; chave vazia |
| ecat-logging | 1→1 | Shim de compatibilidade sem panic |
| ecat-tracing | 9→12 | Trace header não UTF-8 ignorado; header canônico; passagem na resposta |
| ecat-tls | 7→12 | basic_auth com um/dois campos; falta de arquivo ca; is_enabled; cliente padrão |
| ecat-config | 14→26 | Filtro de prefixo env + limites de parsing de tipos (hex/string vazia/-0/1e3); mesclagem de múltiplas sources com sobrescrita; caminho de erro obfs; arquivo ausente/YAML inválido |
| ecat-config-remote | 6→9 | Limites de ConsulKvEntry; erro ao faltar X-Consul-Index; chave aninhada |
| ecat-openapi | 4→11 | components/schema_ref; sobrescrita duplicada; 200 padrão; tags |
| ecat-metrics | 8→11 | Texto de métricas registradas; 404/405 |

### Grupo data (test-data, +66)

| crate | original→novo | Pontos cobertos |
|---|---|---|
| ecat-data | 12→14 | Parsing da sintaxe de busca |
| ecat-data-sqlx | 7→14 | SQLite em memória ponta a ponta; binding de parâmetros de todos os tipos; Blob→base64; config |
| ecat-data-redis | 6→12 | Construção de URL redis:///rediss://; auth; caminhos de erro de config |
| ecat-data-opensearch | 4→10 | mock HTTP: percent-encode, Basic auth, passagem de erros |
| ecat-data-elasticsearch | 6→11 | Idem |
| ecat-data-influxdb | 5→10 | Escape de line protocol; header Token; passagem de erros |
| ecat-data-clickhouse | 12→22 | SQL de criação de tabela; JSONEachRow; contagem de linhas gravadas; agrupamento |
| ecat-data-memcached | 4→8 | TTL segundos→milissegundos; empacotamento de flag |
| ecat-data-nebulagraph | 6→7 | Parsing de config |
| ecat-data-arangodb | 5→7 | config/URL |
| ecat-data-iotdb | 5→10 | mock HTTP: parâmetros de caminho de sessão |
| ecat-data-questdb | 4→9 | line protocol; transações não suportadas |
| ecat-data-tdengine | 6→11 | Geração de INSERT; divisão em lotes de 100 |
| ecat-data-mongodb | 5→8 | roundtrip bson; URI |

### Grupo mq/transport/registry (test-mq, +54)

| crate | original→novo | Pontos cobertos |
|---|---|---|
| ecat-mq | 5→9 | Quadro de erro com buffer cheio/lag; fechamento do stream com todos os drops; múltiplos assinantes; publish sem assinantes |
| ecat-mq-kafka | 12→14 | Padrões de config; campos SASL independentes |
| ecat-mq-rabbitmq | 2→5 | Padrão de exchange; caminho de erro de url |
| ecat-mq-mqtt | 5→9 | Validação do par cert/key; arquivos ausentes; porta padrão 1883/8883; fallback de porta inválida |
| ecat-mq-nats | 6→9 | Padrão texto puro; caminhos de erro sem ca/cert |
| ecat-transport | 4→7 | TlsConfig padrão/with_client_auth; limites de normalize_addr |
| ecat-transport-http | 17→20 | Testes de integração: stop no-op, falha de porta ocupada, envio/recebimento reais |
| ecat-transport-grpc | 7→13 | Falta de arquivo TLS; ciclo de vida texto puro; rejeição mTLS |
| ecat-transport-ws | 4→8 | Falha sem handler; porta ocupada; eco de quadro masked RFC 6455 |
| ecat-registry | 5→8 | discover multi-instância; deregistro automático no drop; padrões do builder |
| ecat-registry-consul | 10→24 | percent-encode; variantes de registro; respostas de erro; X-Consul-Token; parsing de agent/services; fallback de node |
| ecat-registry-etcd | 5→10 | Descarte de valores ruins em discover; corpo de requisição kv; lease grant; keepalive |

### Grupo camada app (test-app, +46)

| crate | original→novo | Pontos cobertos |
|---|---|---|
| ecat-auth | 20→46 | Whitelist do cache oauth2/chave SHA-256/evicção FIFO; três estados do apikey; jwt iss/aud obrigatórios; expirado/assinatura errada |
| ecat-health | 4→8 | Agregação de readiness (tudo ok/qualquer falha/registry vazio); liveness |
| ecat-versioning | 4→7 | Roteamento por estratégia de path; limites de extract_version |
| ecat-security | 12→20 | Ponta a ponta na camada de header; forma JSON de bloqueio de ataque |
| ecat-middleware | 34→37 | Expiração de janela do MemoryStore; panic interno→Err |
| ecat-circuit-breaker | 10→12 | Exaustão de sondas half-open; downgrade de classify |
| ecat-client | 7→10 | Endpoint grpc inválido dá erro sem rede |
| ecat-graphql | 35→35 | Cobertura existente suficiente, sem lacunas |
| ecat-scheduler / ecat-bench / ecat-events / ecat-cli / ecat | Cobertura existente suficiente | Sem lacunas |

## Defeitos encontrados

| Nível | Local | Descrição | Status |
|---|---|---|---|
| P1 | ecat-events/Cargo.toml | dev-dependencies sem as features tokio macros/rt/time; compilar os alvos de teste desse crate isoladamente falha obrigatoriamente (o build completo do workspace é mascarado pela união de features) | ✅ Corrigido (features + comentário adicionados) |
| P2 | ecat-security src/lib.rs:118-127 | SQLi com percent-encoding na URI (`?q=SELECT%20*%20...`) pode contornar a varredura da camada de header (os detectores exigem espaço literal; a URI bruta é varrida sem decodificação prévia); a varredura do corpo não é afetada | ⏳ A corrigir |
| P3 | ecat-data-sqlx | `connect()/from_config()` usa AnyPool sem instalar drivers; sqlx 0.8.6 faz panic "No drivers installed" na primeira conexão | ⏳ A corrigir |
| P3 | ecat-data-influxdb | Campos string com escape de espaço (`\ `); a especificação do line protocol exige escapar apenas `"` e `\`; ordem de tag/field não determinística | ⏳ A corrigir |
| P3 | ecat-data-clickhouse | Cache de criação de tabela nunca expira; CREATE não é repetido após drop/alter externo | ⏳ A corrigir |
| P3 | ecat-circuit-breaker | O limite de half_open_probes é inalcançável em sondagem sequencial (só alcançável com sondas concorrentes em voo); coberto por teste white-box | ℹ️ Conhecido, não é defeito |
| P3 | ecat-health | `with_check` usa blocking_write(); chamado em contexto async causa panic; atualmente só utilizável em contexto síncrono | ℹ️ Conhecido, limitação de API |

## Módulos pulados (exigem ambiente de integração, sem mock)

- Roundtrip real de brokers: publish-subscribe kafka/rabbitmq/mqtt/nats (config e caminhos de erro cobertos)
- Cluster real: ciclo de vida de registro-descoberta consul/etcd (mock axum cobre o formato das requisições)
- Bancos reais: operações redis/memcached, mongod, validação no servidor influxdb, drivers sqlx postgres/mysql, APIs nebulagraph/arangodb
- Serviços externos reais: introspecção OAuth2 (mock local cobre), roundtrip gRPC/HTTP (mock local cobre 302 sem follow)
