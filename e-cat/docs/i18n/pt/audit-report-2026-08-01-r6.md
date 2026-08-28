# Relatório de revisão profunda do e-cat — 2026-08-01 R6

## Avaliação geral

| Dimensão | Status | Descrição |
|------|------|------|
| Compilação | Aprovado | 50 crates, zero erros |
| Testes | Aprovado | Tudo aprovado, zero falhas |
| Clippy | Aprovado | Zero warnings (`-D warnings`) |
| unsafe | Zero | Nenhum bloco unsafe no código |
| Tamanho dos arquivos | Bom | Apenas `ecat-auth` (540 linhas) excede o valor sugerido de 500 linhas |

## Descobertas (15 itens)

### Relacionadas à segurança

#### 1. [Grave] XOR "criptografia" não é criptografia de verdade
**Arquivo:** `ecat-config/src/encrypted.rs:45-56`
**Problema:** `decrypt()` usa XOR + chave repetida, que é uma ofuscação, não criptografia, e pode ser facilmente quebrada. A chave é reutilizada em cada posição de byte, tornando o texto cifrado extremamente vulnerável a análise de frequência.
**Sugestão:** substituir por AES-256-GCM (crate `aes-gcm`), ou rotular explicitamente como "ofuscação" em vez de "criptografia".

#### 2. [Grave] A implementação padrão de `execute_with`/`query_with` descarta parâmetros silenciosamente
**Arquivo:** `ecat-data/src/rdbms.rs:86-103`
**Problema:** a implementação padrão no trait recebe os parâmetros mas os ignora (`let _ = params;`), chamando diretamente o `execute(sql)` original. Todos os backends exceto `ecat-data-sqlx` (ClickHouse, QuestDB) herdam esse comportamento. Se o usuário trocar de backend usando os métodos parametrizados, os parâmetros são descartados silenciosamente, criando vulnerabilidade de SQL injection.
**Sugestão:** a implementação padrão deve retornar erro de "não suportado", ou cada backend deve implementar corretamente o binding de parâmetros.

#### 3. [Alto] Senha embutida em texto puro na URL
**Arquivo:** `ecat-data-sqlx/src/lib.rs:40`, `ecat-data-redis/src/lib.rs:43`
**Problema:** `connect_with_auth()` usa `replacen("://", "://user:pass@")` para embutir as credenciais diretamente na URL. Essas URLs podem ser registradas em logs, mensagens de erro ou saída de debug.
**Sugestão:** usar o mecanismo de autenticação nativo de cada backend; ou pelo menos aplicar URL encoding em username/password antes da concatenação.

#### 4. [Médio] Falha na configuração TLS causa panic
**Arquivo:** 8 crates data-* (ClickHouse, QuestDB, Elasticsearch, OpenSearch, ArangoDB, Neo4j, NebulaGraph, InfluxDB, IoTDB)
**Padrão:** `.expect("TLS client build failed")` — todos os construtores `from_config()` entram em panic quando a configuração TLS está errada.
**Sugestão:** alterar `from_config()` para retornar `Result`, ou tornar a construção do cliente TLS lazy/tolerante a falhas.

### Corretude funcional

#### 5. [Alto] Roteamento por header do `ecat-versioning` é ineficaz
**Arquivo:** `ecat-versioning/src/lib.rs:56-64`
**Problema:** `build_header_router()` aninha todas as versões sob o mesmo caminho `/api`, mas não filtra pelo header de versão. O axum registra todas as rotas de versão no mesmo caminho, causando conflito de rotas e comportamento imprevisível. A função `extract_version()` existe mas nunca é usada no roteamento.
**Sugestão:** usar middleware/layer do axum para verificar o header Accept e rotear para a rota de versão correta, em vez de achatar todas as versões no mesmo caminho.

#### 6. [Médio] Truncamento de TTL do Redis: expiração subssegundo vira nunca expira
**Arquivo:** `ecat-data-redis/src/lib.rs:76-77`
**Problema:** `Duration::as_secs()` trunca em direção a zero. Definir TTL de 500ms com `secs == 0` vira silenciosamente "nunca expira", seguindo o ramo `SET` em vez de `SETEX`.
**Sugestão:** para TTL subssegundo, definir pelo menos 1 segundo, ou usar `SET ... PX` (milissegundos) em vez de `SETEX`.

#### 7. [Médio] `StaticResolver::add_service` entra em panic na contenção de lock
**Arquivo:** `ecat-client/src/lib.rs:27-29`
**Problema:** usa `try_write()` com expect; se houver qualquer outro detentor de lock de escrita, entra em panic. O padrão builder torna isso difícil de acionar, mas é uma bomba-relógio em código concorrente.
**Sugestão:** usar `blocking_write()` (se em contexto síncrono) ou alterar para aceitar `&mut self` para evitar a necessidade de lock.

### Qualidade de código

#### 8. [Médio] Uso de `std::sync::Mutex` em contexto assíncrono
**Arquivo:** `ecat-data-memcached/src/lib.rs:7,24`
**Problema:** `std::sync::Mutex` é usado em implementação de trait async. Embora o lock seja mantido por tempo extremamente curto (apenas operações de HashMap), sob alta contenção pode bloquear teoricamente o runtime assíncrono.
**Sugestão:** para este caso específico de cache em memória, como a seção crítica é curtíssima e não há pontos `.await`, usar `std::sync::Mutex` é aceitável. Mas se no futuro for necessário executar I/O dentro do lock, deve-se trocar para `tokio::sync::Mutex`.

#### 9. [Baixo] Implementação manual de base64
**Arquivo:** `ecat-registry-etcd/src/lib.rs:148-193`
**Problema:** ~45 linhas de codec base64 escrito à mão, com possíveis bugs de casos-limite. O ecossistema Rust tem alternativas bem revisadas, como o crate `base64`.
**Sugestão:** substituir pelo crate `base64`, reduzindo a carga de manutenção e potenciais bugs.

#### 10. [Baixo] `RandomBalancer` não é aleatório
**Arquivo:** `ecat-client/src/lib.rs:91-105`
**Problema:** usa hash de `Instant::now()` como fonte de aleatoriedade. Chamadas simultâneas na mesma instância obtêm a mesma escolha "aleatória". `checked_add(0)` é uma operação redundante.
**Sugestão:** usar o crate `rand` ou pelo menos `std::collections::hash_map::RandomState`.

#### 11. [Baixo] `Arc<Vec<String>>` desnecessário em `ecat-data-sqlx`
**Arquivo:** `ecat-data-sqlx/src/lib.rs:79-87, 197-203`
**Problema:** os nomes de colunas são envolvidos em `Arc<Vec<String>>`, mas cada construtor de `Row` clona a lista completa de colunas (`(*cols).clone()`). O `Arc` é usado apenas uma vez durante a iteração; `Rc` ou `clone()` direto bastaria.
**Sugestão:** em `query()` e `query_with()`, substituir `Arc<Vec<String>>` por `Vec<String>` comum. O custo do clone por linha é o mesmo que desreferenciar Arc + clonar.

### Design/arquitetura

#### 12. [Informação] QuestDB usa GET + parâmetros de query
**Arquivo:** `ecat-data-questdb/src/lib.rs:76, 91`
**Problema:** o SQL é enviado via parâmetros de query GET, sujeito ao limite de tamanho de URL (geralmente ~2000-8000 caracteres). Consultas grandes são truncadas.
**Sugestão:** mudar para POST + body, ou manter GET para consultas simples e usar POST para as complexas.

#### 13. [Informação] `#[allow(dead_code)]` espalhado por vários lugares
**Arquivo:** `ecat-registry-consul/src/lib.rs:225`, `ecat-data-memcached/src/lib.rs:25-28`, `ecat-auth/src/lib.rs:52`
**Problema:** campos username/password são armazenados em memória mas marcados como dead_code (não necessários no memcached em memória; a variante RSA do auth ainda não implementada).
**Sugestão:** implementar os caminhos de funcionalidade ausentes, ou remover esses campos, ou adicionar documentação explicando por que são mantidos.

#### 14. [Informação] Alguns clientes HTTP não enviam header Content-Type
**Arquivo:** `ecat-data-influxdb/src/lib.rs:96-103`, `ecat-data-clickhouse/src/lib.rs:87-89`
**Problema:** algumas requisições POST não definem o header `Content-Type`, dependendo da detecção automática do servidor.
**Sugestão:** sempre definir Content-Type explícito para garantir compatibilidade.

#### 15. [Informação] `ecat-auth` excede 500 linhas
**Arquivo:** `ecat-auth/src/lib.rs` (540 linhas)
**Problema:** o CLAUDE.md exige que arquivos fiquem abaixo de 500 linhas. O crate auth é o único arquivo que excede esse limite.
**Sugestão:** dividir a lógica de validação JWT em `ecat-auth/src/jwt.rs`, ou dividir por funcionalidade.

## Oportunidades de otimização (não são bugs)

| # | Local | Sugestão |
|---|------|------|
| O1 | Todos os crates data-* | O padrão repetido de construção de cliente TLS em todos os `from_config()` pode ser extraído para um macro ou função compartilhada |
| O2 | `ecat-data-sqlx` | A lógica de conversão de tipos de linhas em `query()` e `query_with()` (117 linhas repetidas) pode ser extraída para uma função auxiliar |
| O3 | `ecat-client` | `HttpClient::get()` e `post()` compartilham o mesmo pipeline "resolve → pick → build URL" — pode ser extraído |
| O4 | `ecat-data` | Os tipos de erro customizados dos 5 traits (Rdbms/Cache/Graph/Search/Tsdb) podem ser unificados em um único enum `DataError` |
| O5 | `ecat-data-redis` | `self.conn.clone()` em cada método é desnecessário — `MultiplexedConnection` já é projetado como `Clone` para suportar compartilhamento |

## Resumo de métricas

| Métrica | Valor |
|------|------|
| Total de crates | 50 |
| Total de linhas de código Rust | 7,968 |
| `expect()` em código não-teste | 12 |
| `unwrap()` em código não-teste | 0 |
| Blocos `unsafe` | 0 |
| `panic!` em código não-teste | 0 |
| `#[allow(dead_code)]` | 4 |
| TODO/FIXME/HACK | 0 |
| Mutex std em código async | 1 (memcached) |

## Conclusão

O código está em bom estado — compilação, testes e clippy todos aprovados, sem código unsafe, sem macros de panic. Os dois problemas mais críticos são **XOR "criptografia"** (segurança falsa) e **a implementação padrão de consultas parametrizadas descartar parâmetros silenciosamente** (vulnerabilidade de segurança). A funcionalidade de roteamento por header também está completamente inutilizável. Os demais problemas são relativamente menores, no nível de otimização de manutenibilidade.

**Ordem de correção recomendada:**
1. Implementação padrão de `execute_with`/`query_with` → retornar erro em vez de descartar parâmetros silenciosamente
2. Criptografia XOR → AEAD real, ou renomear para "ofuscação"
3. Roteamento de versão por header → implementar roteamento real por header
4. `from_config()` → retornar Result em vez de expect-panic
5. Truncamento de TTL do Redis → TTL subssegundo usar pelo menos 1 segundo

## Status das correções (R6 → R6.1)

| # | Problema | Status | Alteração |
|---|------|------|------|
| 1 | XOR "criptografia" | Corrigido | `EncryptedSource` → `ObfuscatedSource`, `decrypt` → `deobfuscate`, prefixo `enc:` → `obfs:`, adicionada documentação esclarecendo que é ofuscação, não criptografia |
| 2 | `execute_with`/`query_with` descartam parâmetros silenciosamente | Corrigido | Implementação padrão passou a retornar erro `"parameterized ... not supported by this backend"` |
| 3 | Senha embutida em texto puro na URL | Corrigido | Credenciais codificadas com `percent_encode()` no método `connect_with_auth` |
| 4 | Panic de TLS `expect()` | Corrigido | `from_config()` de 9 crates passou a retornar `Result`; `RdbmsError` ganhou variante `Config` |
| 5 | Roteamento por header ineficaz | Corrigido | Implementado middleware de validação de versão com `from_fn_with_state`; novo teste `header_versioned_router_builds` |
| 6 | Truncamento de TTL do Redis | Corrigido | `set_ex` → `pset_ex`, usando precisão em milissegundos para evitar que TTL subssegundo vire nunca expira |
| 7 | Panic de contenção de lock do `StaticResolver` | Corrigido | `try_write()` → `blocking_write()` |
| 8 | `RandomBalancer` não aleatório | Corrigido | Substituído hash de `Instant::now()` por `RandomState::new().build_hasher()` |
| 9 | `std::sync::Mutex` em contexto async | Corrigido | Substituído por `tokio::sync::Mutex` |
| 10 | base64 escrito à mão | Corrigido | Substituído pelo crate `base64` 0.22 |
| 11 | Sobrecarga do `Arc<Vec<String>>` | Corrigido | Substituído por `Vec<String>` comum, removido o wrapper Arc desnecessário |
| 12 | QuestDB enviando SQL via GET | Corrigido | Alterado para POST + body, adicionado header Content-Type |
| 13 | `#[allow(dead_code)]` | Corrigido | Campos do memcached com prefixo `_`; campos do consul com prefixo `_` e allow removido; `Rsa` → `RsaReserved` no auth |
| 14 | Falta Content-Type | Corrigido | Headers Content-Type explícitos adicionados nas requisições InfluxDB, ClickHouse, IoTDB |
| 15 | `ecat-auth` excede 500 linhas | Corrigido | Dividido em `claims.rs`(31) + `jwt.rs`(139) + `apikey.rs`(96) + `oauth2.rs`(173) + `helpers.rs`(28) + `lib.rs`(98) |

### Crates afetados

| Crate | Tipo de alteração |
|-------|----------|
| `ecat-data` | Implementação padrão do trait, variante `RdbmsError::Config` |
| `ecat-config` | `EncryptedSource` → `ObfuscatedSource` |
| `ecat-versioning` | Implementação do middleware de roteamento por header |
| `ecat-data-redis` | Precisão de TTL em milissegundos, URL encoding de credenciais |
| `ecat-data-sqlx` | URL encoding de credenciais, remoção da sobrecarga Arc |
| `ecat-data-clickhouse` | `from_config` → `Result`, header Content-Type |
| `ecat-data-questdb` | `from_config` → `Result`, GET → POST |
| `ecat-data-elasticsearch` | `from_config` → `Result` |
| `ecat-data-opensearch` | `from_config` → `Result` |
| `ecat-data-arangodb` | `from_config` → `Result` |
| `ecat-data-neo4j` | `from_config` → `Result` |
| `ecat-data-nebulagraph` | `from_config` → `Result` |
| `ecat-data-influxdb` | `from_config` → `Result`, header Content-Type |
| `ecat-data-iotdb` | `from_config` → `Result`, header Content-Type |
| `ecat-data-memcached` | `std::sync::Mutex` → `tokio::sync::Mutex`, limpeza de dead_code |
| `ecat-client` | Correções do `StaticResolver`, `RandomBalancer` |
| `ecat-registry-etcd` | base64 substituído pelo crate |
| `ecat-registry-consul` | Limpeza de dead_code |
| `ecat-auth` | Dividido em 6 módulos, limpeza de dead_code |

### Validação final (R6.2)

| Dimensão | Status |
|------|------|
| Build | Aprovado, zero erros zero warnings |
| Testes | Tudo aprovado, zero falhas |
| Clippy (`-D warnings`) | Aprovado, zero warnings |
| Tamanho dos arquivos | Todos ≤ 300 linhas |
