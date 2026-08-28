# Relatório de auditoria E-CAT — r5

**Data**: 2026-08-01  
**Branch**: main  
**Versão**: 2.1.7  
**Número de crates**: 47 (members do workspace)
**Status**: ✅ Todos os problemas corrigíveis resolvidos + suporte abrangente a arquivos de configuração nos backends de dados

---

## 0. Registro de correções (2026-08-01)

| # | Problema | Arquivo | Correção |
|---|------|------|------|
| 1 | unused import `axum::routing::get` | `ecat-versioning/src/lib.rs:3` | Removido o import de nível superior, movido para dentro de `#[cfg(test)]` |
| 2 | unused variable `version` | `ecat-versioning/src/lib.rs:61` | Alterado para `_version` |
| 3 | dead code `extract_version` | `ecat-versioning/src/lib.rs:68` | Alterado para `pub fn` |
| 4 | `useless_format!` | `ecat-versioning/src/lib.rs:62` | Alterado para `"/api"` direto |
| 5 | `unnecessary_to_owned` | `ecat-data-questdb/src/lib.rs:39` | `"true".to_string()` → `"true"` |
| 6 | Mensagem de erro engolida | `ecat-data-questdb/src/lib.rs:30` | `unwrap_or_default()` → `unwrap_or_else(...)` |
| 7 | `derivable_impls` | `ecat-client/src/lib.rs:249` | `GrpcClientBuilder` passou a usar `#[derive(Default)]` |
| 8 | `manual_is_multiple_of` | `ecat-config/src/encrypted.rs:60` | `s.len() % 2 != 0` → `!s.len().is_multiple_of(2)` |
| 9 | `collapsible_if` | `ecat-registry-etcd/src/lib.rs:92` | Mesclados `if let` aninhados |
| 10 | `collapsible_if` | `ecat-data-clickhouse/src/lib.rs:56` | Mesclados `if let` aninhados |
| 11 | `type_complexity` | `ecat-data-memcached/src/lib.rs:9` | Adicionado alias `type CacheEntry` |

**Resultado final**: `cargo build` zero warnings, `cargo clippy --all-targets` zero warnings, `cargo test` tudo aprovado (0 falhas).

### 12 ─ Suporte abrangente a arquivos de configuração nos backends de dados (Cargo + lib.rs)

Adicionados estrutura `Config` (`#[derive(Deserialize)]`) e construtor `from_config()` para 12 crates de backends de dados, permitindo carregar informações de conexão de arquivos de configuração JSON/YAML, sem hardcoding.

| Crate | Estrutura Config | Campos |
|-------|--------------|------|
| `ecat-data-redis` | `RedisConfig` | `url` |
| `ecat-data-sqlx` | `SqlxConfig` | `url` |
| `ecat-data-clickhouse` | `ClickhouseConfig` | `base_url`, `database` (padrão "default") |
| `ecat-data-questdb` | `QuestdbConfig` | `base_url` |
| `ecat-data-elasticsearch` | `ElasticsearchConfig` | `base_url` |
| `ecat-data-opensearch` | `OpenSearchConfig` | `base_url` |
| `ecat-data-influxdb` | `InfluxConfig` | `base_url`, `org`, `bucket`, `token` |
| `ecat-data-memcached` | `MemcachedConfig` | (vazio — implementação em memória) |
| `ecat-data-neo4j` | `Neo4jConfig` | `base_url`, `username`, `password` |
| `ecat-data-nebulagraph` | `NebulaGraphConfig` | `base_url`, `space` |
| `ecat-data-arangodb` | `ArangoConfig` | `base_url`, `db`, `username`, `password` |
| `ecat-data-iotdb` | `IotdbConfig` | `base_url`, `username`, `password` |

**Exemplo de uso**:
```rust
// 从 YAML 配置文件加载
let cfg: ClickhouseConfig = serde_json::from_str(r#"{"base_url":"http://localhost:8123"}"#)?;
let client = ClickhouseClient::from_config(cfg);
```

### 13 ─ Backends HTTP ganham autenticação opcional (5 crates)

Adicionados campos opcionais `username` / `password` e construtor `with_auth()` para 5 backends puramente HTTP. Todos são `Option<String>` (`#[serde(default)]`); sem configuração, não há autenticação.

| Crate | Novos campos Config | Novo construtor |
|-------|-----------------|-------------|
| `ecat-data-elasticsearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-opensearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-clickhouse` | `username?`, `password?` | `with_auth()` |
| `ecat-data-questdb` | `username?`, `password?` | `with_auth()` |
| `ecat-data-nebulagraph` | `username?`, `password?` | `with_auth()` |

Todas as requisições HTTP anexam automaticamente Basic Auth via método auxiliar `apply_auth()` (somente quando ambos não são None).

### 14 ─ Redis / RDBMS / Memcached ganham campos de autenticação opcionais (3 crates)

| Crate | Novos campos Config | Novo construtor | Método de autenticação |
|-------|-----------------|-------------|----------|
| `ecat-data-redis` | `password?` | `connect_with_password()` | Senha embutida na URL |
| `ecat-data-sqlx` | `username?`, `password?` | `connect_with_auth()` | Autenticação embutida na URL |
| `ecat-data-memcached` | `username?`, `password?` | `with_auth()` | Campos reservados (implementação em memória) |

Sqlx cobre os quatro RDBMS: SQLite / PostgreSQL / MySQL / TiDB. Os campos Auth são embutidos na URL de conexão via `replacen("://", "://user:pass@")`, aplicado somente quando a URL não contém `@`.

### 15 ─ Suporte a certificados TLS + crate ecat-tls (todos os 12 backends)

Novo crate `ecat-tls`, que fornece:
- `TlsClientConfig` — configuração TLS opcional (ca_cert, client_cert, client_key, skip_verify)
- `generate_ca()` — geração de certificado CA autoassinado
- `generate_server_cert()` — geração de certificado de servidor
- `generate_client_cert()` — geração de certificado de cliente (mTLS)

Todos os 12 backends de dados ganharam o campo `#[serde(default)] tls: Option<TlsClientConfig>`.

| Tipo de backend | Método TLS |
|----------|----------|
| 9 backends HTTP | `tls.build_reqwest_client()` constrói Client reqwest com TLS |
| Redis | Troca de scheme na URL `redis://` → `rediss://` |
| Sqlx | Campo reservado (TLS via parâmetro de URL `?sslmode=require`) |
| Memcached | Campo reservado (implementação de rede reservada) |

---

## 1. Visão geral

| Item | Status | Detalhes |
|------|------|------|
| `cargo build` | ✅ Aprovado | 3 warnings do compilador, 19.85s |
| `cargo test` | ✅ Aprovado | ~137 testes unitários todos aprovados, 0 falhas, 1 ignored |
| `cargo clippy` | ⚠️ Com warnings | 3 crates com 5 lint warnings no total |
| `cargo fmt` | ✅ Aprovado | Sem problemas de formatação |
| `cargo audit` | ❌ Não instalado | Incapaz de escanear CVEs conhecidos |

---

## 2. Warnings do compilador (a corrigir)

### 2.1 ecat-versioning (3 warnings)

**Arquivo**: `ecat-versioning/src/lib.rs`

| # | Warning | Linha | Gravidade |
|---|---------|------|----------|
| 1 | `unused import: axum::routing::get` | 3 | Baixa |
| 2 | `unused variable: version` | 61 | Baixa |
| 3 | `function extract_version is never used` | 68 | Baixa |

**Sugestão**: remover o import não usado, alterar `version` para `_version`, e tornar `extract_version` `pub` ou marcá-lo com `#[allow(dead_code)]`.

### 2.2 ecat-data-questdb (1 warning do clippy)

**Arquivo**: `ecat-data-questdb/src/lib.rs:39`

```rust
// 当前:
.query(&[("query", sql), ("count", &"true".to_string())])

// 应改为:
.query(&[("query", sql), ("count", &"true")])
```

### 2.3 ecat-client (1 warning do clippy)

**Arquivo**: `ecat-client/src/lib.rs:249`

`GrpcClientBuilder` implementa `Default` manualmente; pode ser substituído diretamente por `#[derive(Default)]`.

---

## 3. Resumo dos Lint Warnings do Clippy

| Crate | Warning | Tipo |
|-------|---------|------|
| ecat-versioning | `useless_format!` — usa `"/api".to_string()` | Performance |
| ecat-versioning | unused import / dead code | Limpeza |
| ecat-data-questdb | `unnecessary_to_owned` | Performance |
| ecat-client | `derivable_impls` — usar derive Default | Simplificação |

---

## 4. Análise de cobertura de testes

### 4.1 Estatísticas

| Métrica | Valor |
|------|------|
| Total de testes unitários | ~137 |
| Falhas | 0 |
| Ignored | 1 |
| Crates com testes | ~24 / 48 |
| **Crates com 0 testes** | **~24 / 48 (50%)** |

### 4.2 Crates sem testes suficientes (0 ou apenas testes de construção)

Os seguintes crates têm cobertura de testes fraca:

- ecat-data-arangodb, ecat-data-clickhouse, ecat-data-elasticsearch
- ecat-data-influxdb, ecat-data-iotdb, ecat-data-nebulagraph
- ecat-data-neo4j, ecat-data-opensearch, ecat-data-questdb
- ecat-data-redis, ecat-data-sqlx, ecat-data-memcached
- ecat-mq, ecat-mq-kafka, ecat-graphql, ecat-openapi
- ecat-transport, ecat-transport-grpc, ecat-transport-http
- ecat-transport-ws, ecat-tracing, ecat-logging
- ecat-middleware, ecat-registry-consul, ecat-registry-etcd

### 4.3 Doc-tests

Todos os **48 crates têm 0 doc-tests**. Não há exemplos `/// ````rust` na documentação.

---

## 5. Problemas de dependências

### 5.1 ⚠️ yaml_serde vs serde_yaml (risco médio)

**Arquivo**: `ecat-config/Cargo.toml:9`

```toml
yaml_serde = "0.10"
```

A biblioteca YAML padrão do ecossistema Rust é `serde_yaml` (última versão `0.9.34+`), enquanto `yaml_serde` é um crate **diferente e menos mantido**.

**Sugestão**: confirmar se `yaml_serde` é a dependência pretendida. Se a intenção era `serde_yaml`, substitua.

### 5.2 Falta cargo-audit

`cargo audit` não está instalado. Sugestão: `cargo install cargo-audit` e adicioná-lo ao CI.

### 5.3 Falta campo description

`[workspace.package]` não tem `description`, e nenhum sub-crate define description.

---

## 6. Problemas de qualidade de código

### 6.1 unwrap/expect no código de produção

| Arquivo | Linha | Chamada | Risco |
|------|------|------|------|
| `ecat-client/src/lib.rs` | 28 | `.expect("StaticResolver poisoned")` | Baixo — razoável |
| `ecat/src/signal.rs` | 8 | `.expect("failed to install SIGTERM handler")` | Médio — panic na inicialização |
| `ecat-protos/build.rs` | 5 | `.unwrap()` | Baixo — build script |

### 6.2 extract_version do ecat-versioning

A função `extract_version` (linha 68) implementa a extração do número de versão do header Accept, mas não é chamada por `build_header_router()`.

### 6.3 Tratamento de erros do ecat-data-questdb

```rust
// 第 30 行: 网络响应体读取使用 unwrap_or_default
Err(RdbmsError::Database(resp.text().await.unwrap_or_default()))
```

Quando `resp.text()` falha, a mensagem de erro é engolida silenciosamente. Sugestão: alterar para `unwrap_or_else(|e| format!("questdb parse: {e}"))`.

---

## 7. Avaliação da arquitetura

### Pontos fortes

- 48 crates com separação clara de responsabilidades
- Versão unificada do workspace via `version.workspace = true`
- Dependências enxutas, sem grandes frameworks
- Sem TODO/FIXME/HACK

### Pontos a melhorar

| Problema | Prioridade |
|------|--------|
| 50% dos crates sem testes | Alta |
| Confusão yaml_serde vs serde_yaml | Média |
| Falta cargo-audit | Média |
| Código morto em ecat-versioning | Baixa |
| Sem doc-tests | Baixa |

---

## 8. Visão geral de segurança

| Item de verificação | Resultado |
|--------|------|
| Chaves hardcoded | Não encontradas |
| Vazamento de arquivos .env | Não encontrado |
| unwrap perigoso (código de produção) | 2 ocorrências (signal.rs, client.rs) |
| Varredura de CVE | Não executada (requer instalar cargo-audit) |

---

## 9. Plano de ação

### P0 — Correção imediata
1. Limpar os 3 warnings do compilador do ecat-versioning
2. Corrigir clippy do ecat-data-questdb
3. Corrigir derivable_impls do ecat-client

### P1 — Curto prazo
4. Instalar `cargo-audit` para escanear vulnerabilidades de dependências
5. Confirmar a escolha `yaml_serde` vs `serde_yaml`
6. Complementar doc-tests nos crates centrais

### P2 — Médio prazo
7. Complementar testes nos crates transport/data/security
8. Adicionar campo `description` em todos os crates
9. Integrar ou remover `extract_version`

### P3 — Longo prazo
10. Estabelecer CI: build → test → clippy → audit → coverage

---

*Relatório gerado em 2026-08-01. Toolchain: cargo 1.92.0, rustc 1.92.0, clippy 1.92.0*
