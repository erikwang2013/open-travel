# e-cat Relatório de auditoria do framework R2 — 2026-08-01

**Versão**: 1.0.5
**Escopo**: todos os 18 sub-crates
**Conclusão**: `cargo check` / `cargo clippy --all-features` / `cargo test` todos aprovados, 70 tests ✅

---

## 1. Retrospectiva das correções anteriores (16/16 corrigidos)

Todos os problemas encontrados na auditoria anterior (R1) foram corrigidos: SecurityLayer bloqueando ataques, suporte prost do ProtoCodec, graceful shutdown do servidor, coleta de JoinHandle, implementação de Transaction, detecção segura no Drop de Registration, reforço do mapeamento de tipos de colunas, geração de arquivos do CLI new, unificação de versão/edition, tratamento de erros do FileSource, métodos de metadados do Context, otimização Arc do discover, otimização Arc das columns do query, novo RateLimitLayer.

---

## 2. Novos problemas desta rodada

### 2.1 [Crítico] Código de template gerado pelo CLI `new` não compila

- **Arquivo**: `ecat-cli/src/main.rs:79-97`
- **Problema**: o `Cargo.toml` gerado usa referências de dependência `workspace = true` e caminho relativo `path = "../ecat"`, mas o projeto independente criado por `ecat new myapp` não está dentro do workspace do e-cat; todas essas referências falham na resolução
- **Impacto**: projetos criados com `ecat new` nem compilam
- **Correção**: o template deve usar dependências reais com número de versão, não referências de workspace

```toml
# 当前（错误）：
tokio.workspace = true           # 项目不在 workspace 中，报错
ecat = { path = "../ecat" }      # 相对路径无效

# 应改为：
tokio = { version = "1", features = ["full"] }
ecat = "1.0.5"
```

### 2.2 [Crítico] `transaction()` do ecat-data-sqlx descarta o handle real de transação do banco

- **Arquivo**: `ecat-data-sqlx/src/lib.rs:100-106`
- **Problema**: `pool.begin()` retorna o handle real de transação do banco `Transaction<'_, DB>`, mas o código o vincula a `_tx` e o descarta imediatamente. Quando `_tx` é dropado, a transação do banco é revertida automaticamente. O `ecat_data::Transaction` retornado é uma casca vazia; seus métodos `commit()/rollback()` não têm efeito algum
- **Impacto**: todo código que usa `transaction()` roda sem proteção de transação; a consistência de dados não é garantida
- **Correção**: redesenhar a struct `ecat_data::Transaction` para que ela segure o handle real da transação do banco

### 2.3 [Médio] SecurityLayer não varre o corpo da requisição

- **Arquivo**: `ecat-security/src/lib.rs:117-127`
- **Problema**: `call()` varre apenas a URI e os cabeçalhos HTTP, sem verificar o corpo da requisição. Atacantes podem colocar payloads de SQL injection/XSS no corpo POST e contornar a detecção facilmente
- **Impacto**: reduz drasticamente a cobertura efetiva da detecção de ataques
- **Correção**: adicionar capacidade de varredura do corpo, ou fornecer um método público `scan_body()` para o chamador usar após ler o corpo

### 2.4 [Médio] RateLimitLayer usa Mutex síncrono + sem limpeza de expiração

- **Arquivo**: `ecat-middleware/src/ratelimit.rs:10-38`
- **Problema 1**: `std::sync::Mutex` usado em contexto async — sob contenção de lock, bloqueia a thread do worker tokio inteira
- **Problema 2**: `buckets: HashMap<String, (u32, Instant)>` nunca limpa chaves expiradas; servidores de longa duração crescem sem limite na memória (cada novo IP/key ocupa memória para sempre)
- **Impacto**: perda de desempenho sob alta concorrência; vazamento de memória após execução longa
- **Correção**: trocar para `tokio::sync::Mutex` e limpar periodicamente entradas expiradas em `allow()`

### 2.5 [Médio] ecat-data-sqlx: SQL cru sem API parametrizada

- **Arquivo**: `ecat-data-sqlx/src/lib.rs:24-29, 32-36`
- **Problema**: `execute(&self, sql: &str)` e `query(&self, sql: &str)` aceitam apenas strings SQL cruas; no nível do trait não há método de binding de parâmetros. Se o chamador concatena entrada do usuário no SQL, ocorre SQL injection
- **Impacto**: embora o trait em si não exponha vulnerabilidade diretamente, a falta de API parametrizada induz chamadores a escrever código inseguro
- **Sugestão**: adicionar métodos `execute_with` e `query_with` ao trait `RdbmsClient`, usando binding de parâmetros

### 2.6 [Baixo] Arc::clone em query() ainda dentro da closure

- **Arquivo**: `ecat-data-sqlx/src/lib.rs:50-53`
- **Problema**: `let cols = std::sync::Arc::clone(&columns)` é executado dentro da closure de `rows.iter().map()`. Embora Arc::clone seja leve (apenas incremento de contagem atômica), pode ser movido para fora da closure para evitar uma operação atômica por linha
- **Sugestão**: fazer um clone antes de `iter()` e capturar esse clone na closure

### 2.7 [Baixo] Trait impl do ProtoCodec inconsistente com a nova API

- **Arquivo**: `ecat-encoding/src/proto.rs`
- **Problema**: `encode/decode` do trait `Codec` ainda retornam apenas erro; os novos `encode_message/decode_message` são o caminho correto, mas os nomes de método não correspondem ao trait. Usuários podem tentar `codec.encode()` primeiro e ficarem confusos com a falha
- **Sugestão**: explicar na documentação/comentários: tipos proto devem usar `encode_message/decode_message` em vez dos métodos do trait Codec

---

## 3. Visão geral do estado atual

| Dimensão | Status |
|------|------|
| `cargo check` | ✅ zero warnings |
| `cargo clippy --all-features` | ✅ zero avisos |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 aprovados |
| Versão unificada | ✅ 1.0.5 |
| Edition unificada | ✅ 2024 |

### Distribuição de testes

| Crate | Tests | Descrição |
|-------|-------|------|
| ecat | 4 | ✅ |
| ecat-config | 9 | ✅ |
| ecat-encoding | 15 | ✅ |
| ecat-errors | 4 | ✅ |
| ecat-logging | 1 | ✅ |
| ecat-metadata | 9 | ✅ |
| ecat-metrics | 2 | ✅ |
| ecat-middleware | 4 | ✅ (inclui RateLimitLayer) |
| ecat-registry | 5 | ✅ |
| ecat-security | 6 | ✅ |
| ecat-transport | 11 | ✅ |
| ecat-data | 0 | — (apenas definições de trait) |
| ecat-data-sqlx | 0 | ⚠️ sem testes de integração com banco |
| ecat-protos | 0 | — (código gerado) |
| ecat-transport-grpc | 0 | ⚠️ |
| ecat-transport-http | 0 | ⚠️ |
| ecat-cli | 0 | ⚠️ |

---

## 4. Prioridades de problemas

| # | Gravidade | Problema | Arquivo | Impacto no usuário |
|---|--------|------|------|----------|
| 1 | 🔴 | Template do CLI `new` gera código não compilável | `ecat-cli/src/main.rs:79` | Primeiro comando do novo usuário já falha |
| 2 | 🔴 | transaction() descarta o handle real da transação DB | `ecat-data-sqlx/src/lib.rs:100` | Consistência de dados sem garantia |
| 3 | 🟠 | SecurityLayer não varre body | `ecat-security/src/lib.rs:117` | Atacantes podem contornar a detecção |
| 4 | 🟠 | RateLimitLayer Mutex std + vazamento de memória | `ecat-middleware/src/ratelimit.rs:10,25` | Desempenho concorrente + OOM |
| 5 | 🟠 | SQL cru sem API parametrizada | `ecat-data-sqlx/src/lib.rs:24` | Risco de SQL injection |
| 6 | 🟡 | Posição do Arc clone em query() | `ecat-data-sqlx/src/lib.rs:53` | Otimização de desempenho mínima |
| 7 | 🟡 | API do ProtoCodec inconsistente | `ecat-encoding/src/proto.rs` | Confusão do usuário |

---

## 6. Registro de correções (2026-08-01 R2)

| # | Problema | Forma de correção | Status |
|---|------|----------|------|
| 1 | Template do CLI new não compilável | Dependências versionadas (`ecat = "1.0"`, `tokio = "1"` etc.) | ✅ |
| 2 | transaction() descarta transação DB | `Transaction::with_inner()` segura o handle real; sqlx passa via `Box<dyn Any>` | ✅ |
| 3 | SecurityLayer não varre body | Novo método público `scan_body(&[u8])` | ✅ |
| 4 | RateLimitLayer Mutex + vazamento | `tokio::sync::Mutex` + limpeza de entradas expiradas a cada 100 chaves | ✅ |
| 5 | SQL cru sem API parametrizada | `RdbmsClient` ganhou métodos parametrizados `execute_with`/`query_with` | ✅ |
| 6 | Posição do Arc clone em query() | `Arc::clone` movido para fora de `iter()`, todas as linhas compartilham a referência | ✅ |
| 7 | API do ProtoCodec inconsistente | Documentação em nível de módulo + doc da struct explicando o uso | ✅ |

### Estado final

| Item de verificação | Resultado |
|--------|------|
| `cargo check` | ✅ zero errors / zero warnings |
| `cargo clippy --all-features` | ✅ zero warnings |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 aprovados |
| Versão | 1.0.5 (todos unificados por herança do workspace) |
| Edition | 2024 |
