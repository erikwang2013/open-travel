# e-cat Relatório de auditoria do framework — 2026-08-01

**Data da auditoria**: 2026-08-01
**Escopo da auditoria**: todos os 18 sub-crates (workspace)
**Toolchain**: stable (rustfmt, clippy)
**Resultado dos testes**: 66 testes aprovados | 0 falhas | 0 ignorados

---

## 1. Avaliação geral

| Dimensão | Nota | Descrição |
|------|------|------|
| Compilação | ✅ Aprovado | `cargo check` sem erros, apenas 1 warning |
| Lint | ✅ Aprovado | `cargo clippy --all-features` zero avisos |
| Testes | ✅ 66/66 | Todos os testes aprovados |
| Cobertura de testes | ⚠️ Insuficiente | 7 crates sem nenhum teste |
| Completeza funcional | ⚠️ Muitos stubs | ProtoCodec, Transaction, CLI new etc. não implementados |
| Qualidade de código | ⚠️ Regular | Estrutura clara, mas vários problemas de design |

---

## 2. Problemas de compilação e configuração

### 2.1 [WARNING] Chave de manifest não usada

- **Arquivo**: `/Cargo.toml:25`
- **Problema**: `workspace.package.name = "e-cat"` — este campo não tem significado em nível de workspace; gera warning a cada compilação
- **Correção**: remover a linha, ou transformar em comentário explicando o nome do projeto

### 2.2 [INFO] Edition Rust inconsistente

- **workspace**: `edition = "2026"`
- **sub-crates**: `ecat-security/Cargo.toml` e `ecat-config/Cargo.toml` usam `edition = "2021"`
- **Observação**: o workspace declara edition 2026, mas alguns sub-crates a sobrescrevem para 2021. Embora compile, a edition 2026 atualmente não é uma edition estável oficialmente publicada pelo Rust. Se for intencional, garanta que o toolchain esteja configurado corretamente
- **Sugestão**: confirmar se o toolchain suporta a edition 2026, ou unificar para 2024/2021

---

## 3. Funcionalidades ausentes / Implementações stub

### 3.1 [Crítico] ProtoCodec totalmente inutilizável

- **Arquivo**: `ecat-encoding/src/proto.rs:8-10`
- **Problema**: `encode()` e `decode()` sempre retornam erro; o codec protobuf é totalmente stub
- **Impacto**: qualquer chamada que use codificação protobuf falha em runtime
- **Sugestão**: implementar o binding do trait prost::Message, ou fornecer um feature flag `prost` para habilitar a funcionalidade real

### 3.2 [Médio] Transação do ecat-data-sqlx não implementada

- **Arquivo**: `ecat-data-sqlx/src/lib.rs:89-93`
- **Problema**: o método `transaction()` retorna o erro hardcoded `"transactions not yet implemented"`
- **Sugestão**: implementar `pool.begin()` e retornar o Transaction encapsulado

### 3.3 [Médio] HttpServer.stop() e GrpcServer.stop() são no-op

- **Arquivos**:
  - `ecat-transport-http/src/lib.rs:34-36`
  - `ecat-transport-grpc/src/lib.rs:33-35`
- **Problema**: `stop()` não tem lógica real de parada do servidor. `axum::serve()` e `tonic::Server::serve()` não têm mecanismo para receber sinal de shutdown
- **Impacto**: após `App.run()`, quando `wait_for_shutdown` dispara, o servidor ainda está rodando; impossível desligar com elegância
- **Sugestão**: usar `axum::serve(listener, router).with_graceful_shutdown(shutdown_signal)` e `tonic::Server::serve_with_shutdown()`

### 3.4 [Médio] Comando CLI `new` é casca vazia

- **Arquivo**: `ecat-cli/src/main.rs:61-67`
- **Problema**: o comando `new` apenas imprime mensagens; não cria os arquivos do template do projeto
- **Sugestão**: implementar a lógica de geração de templates, ou marcar como TODO

### 3.5 [Baixo] Camada ecat-data sem implementações

- **Arquivos**: `ecat-data/src/{cache,graph,rdbms,search,tsdb}.rs`
- **Problema**: todas as interfaces de acesso a dados têm apenas definições de trait, sem nenhuma implementação (exceto `ecat-data-sqlx`, que fornece uma implementação de RdbmsClient)
- **Sugestão**: documentar no README o status de implementação de cada trait

---

## 4. Cobertura de testes insuficiente

### 4.1 [Médio] Crates com cobertura zero (7)

| Crate | Arquivos-fonte | Descrição |
|-------|--------|------|
| `ecat-data` | 5 arquivos-fonte | Apenas definições de trait, sem testes |
| `ecat-data-sqlx` | 1 arquivo-fonte | Implementação SQLx, sem testes de integração com banco |
| `ecat-middleware` | 4 arquivos-fonte | Layers Logging/Recovery/Timeout/Tracing sem testes |
| `ecat-protos` | 1 arquivo-fonte | Código protobuf gerado, sem testes |
| `ecat-transport-grpc` | 1 arquivo-fonte | Servidor gRPC, sem testes |
| `ecat-transport-http` | 1 arquivo-fonte | Servidor HTTP, sem testes |
| `ecat-cli` | 1 arquivo-fonte | Entrada CLI, sem testes |

**Sugestões**:
- `ecat-middleware`: escrever testes unitários para cada layer com `tower-test`
- `ecat-transport-http`: escrever testes de integração do servidor HTTP com `axum::test`
- `ecat-data-sqlx`: escrever testes de integração de banco com `sqlx::SqlitePool` (in-memory)

---

## 5. Qualidade de código e problemas de design

### 5.1 [Crítico] SecurityLayer detecta ataques mas não bloqueia

- **Arquivo**: `ecat-security/src/lib.rs:100-125`
- **Problema**: `SecurityService::call()` varre os dados da requisição e registra avisos, mas sempre encaminha a requisição para o serviço interno. Mesmo detectando SQL injection e XSS, a requisição é processada normalmente
- **Correção**: ao detectar ataques, retornar `403 Forbidden` ou `400 Bad Request`

```rust
// 当前：总是转发
let fut = self.inner.call(req);
Box::pin(fut)

// 应改为：检测到高危攻击时拒绝
if results.iter().any(|r| r.severity >= Severity::High) {
    // 返回 403 响应
}
```

### 5.2 [Médio] App::run() não coleta JoinHandle

- **Arquivo**: `ecat/src/lib.rs:33-40`
- **Problema**: o `JoinHandle` retornado por `tokio::spawn` é descartado; não é possível detectar panic de servidor nem aguardar shutdown elegante
- **Sugestão**: coletar os JoinHandle em um Vec e aguardar o encerramento de todos os servidores no shutdown

### 5.3 [Médio] Registration::Drop falha silenciosamente quando descartado em runtime

- **Arquivo**: `ecat-registry/src/lib.rs:46-56`
- **Problema**: `Drop` chama `tokio::spawn()` — se o runtime tokio já foi dropado, a task é descartada silenciosamente
- **Sugestão**: usar `tokio::task::block_in_place` + `Handle::block_on`, ou mudar para um método `unregister` explícito

### 5.4 [Médio] Mapeamento de tipos de linhas de consulta do ecat-data-sqlx não confiável

- **Arquivo**: `ecat-data-sqlx/src/lib.rs:55-78`
- **Problema**: os valores de colunas do banco são tentados na ordem `i64 → f64 → String → Null`; alguns drivers podem reportar valores inteiros como tipo incompatível, causando conversão errada (ex.: PostgreSQL retorna INTEGER como `i32` em vez de `i64`)
- **Sugestão**: usar `ValueRef` / `TypeInfo` do SQLx para verificar o tipo real da coluna no banco antes de decidir a estratégia de conversão

### 5.5 [Baixo] Contexto de Metadata sem métodos de definição

- **Arquivo**: `ecat-transport/src/context.rs:18-20`
- **Problema**: `Context` encapsula `Metadata` em um `RwLock` e expõe apenas o método de leitura `trace_id()`; não é possível definir trace_id nem outros metadados
- **Sugestão**: adicionar métodos de escrita como `set_trace_id()` ao `Context`

### 5.6 [Baixo] FileSource do ecat-config descarta silenciosamente YAML/JSON não-objeto

- **Arquivo**: `ecat-config/src/file.rs:30`
- **Problema**: `unwrap_or_default()` mapeia YAML não-objeto (como arrays `[1,2,3]` ou valores escalares) para um HashMap vazio; o usuário pode não perceber por que a configuração não foi carregada
- **Sugestão**: retornar `ConfigError::Other("expected object")`

---

## 6. Problemas de compatibilidade multiplataforma

### 6.1 [Médio] Windows: wait_for_shutdown sem suporte a Ctrl+C

- **Arquivo**: `ecat/src/signal.rs:13-14`
- **Problema**: em plataformas não Unix, `terminate` é definido como `std::future::pending::<()>()`, que nunca resolve. No Windows, Ctrl+C vira sinal SIGINT, mas não é certo se `tokio::signal::ctrl_c()` funciona no Windows
- **Sugestão**: usar `tokio::signal::ctrl_c()` também no Windows (a documentação do tokio diz que suporta Windows), ou usar a família `tokio::signal::windows::ctrl_*`

---

## 7. Sugestões de arquitetura e otimização

### 7.1 [Otimização] ecat-data-sqlx query() clona nomes de colunas repetidamente

- **Arquivo**: `ecat-data-sqlx/src/lib.rs:48-83`
- **Problema**: o vetor de columns é clonado a cada linha. Para consultas que retornam 1000 linhas, columns é clonado 1000 vezes
- **Sugestão**: encapsular columns em `Arc<Vec<String>>`, compartilhando a referência entre todas as linhas

### 7.2 [Otimização] Clonagem desnecessária em MemoryRegistry::discover()

- **Arquivo**: `ecat-registry/src/memory.rs:44-52`
- **Problema**: `.cloned()` clona todos os ServiceInfo correspondentes. Se discover for chamado com alta frequência, gera muitas alocações de memória
- **Sugestão**: se o chamador não precisar de ownership, considerar retornar `Vec<&ServiceInfo>` ou encapsular em `Arc<ServiceInfo>`

### 7.3 [Arquitetura] Sugestão de estrutura de re-export

No crate `ecat-transport`, os parâmetros genéricos `T` de `Request` e `Response` têm padrão `()`; normalmente é preciso especificar o tipo concreto ao usar. Sugere-se fornecer aliases de tipo:
```rust
pub type HttpRequest = Request<hyper::Body>;
pub type JsonRequest<T> = Request<T>;
```

### 7.4 [Segurança] Falta middleware de rate limiting

A camada de middleware atual não tem a funcionalidade de Rate Limiting. Sugere-se adicionar `RateLimitLayer` para prevenir ataques DoS.

---

## 8. Estatísticas de testes

```
Visão geral dos testes:
  Total: 66 tests
  Aprovados: 66
  Falhas: 0
  Ignorados: 0

Distribuição por crate:
  ecat:              4 tests ✅
  ecat-config:       9 tests ✅
  ecat-data:         0 tests ⚠️
  ecat-data-sqlx:    0 tests ⚠️
  ecat-encoding:    15 tests ✅
  ecat-errors:       4 tests ✅
  ecat-logging:      1 test  ✅
  ecat-metadata:     9 tests ✅
  ecat-metrics:      2 tests ✅
  ecat-middleware:   0 tests ⚠️
  ecat-protos:       0 tests ⚠️
  ecat-registry:     5 tests ✅
  ecat-security:     6 tests ✅
  ecat-transport:   11 tests ✅
  ecat-transport-grpc: 0 tests ⚠️
  ecat-transport-http: 0 tests ⚠️
  ecat-cli:          0 tests ⚠️
```

---

## 9. Resumo de prioridades de problemas

| # | Gravidade | Problema | Arquivo |
|---|--------|------|------|
| 1 | 🔴 Crítico | SecurityLayer detecta ataques mas não bloqueia | `ecat-security/src/lib.rs` |
| 2 | 🔴 Crítico | ProtoCodec totalmente inutilizável | `ecat-encoding/src/proto.rs` |
| 3 | 🟠 Médio | HttpServer/GrpcServer stop() é no-op | `ecat-transport-http/src/lib.rs`, `ecat-transport-grpc/src/lib.rs` |
| 4 | 🟠 Médio | 7 crates com cobertura de testes zero | ver tabela 4.1 |
| 5 | 🟠 Médio | App::run() não coleta JoinHandle | `ecat/src/lib.rs` |
| 6 | 🟠 Médio | Transaction não implementado | `ecat-data-sqlx/src/lib.rs` |
| 7 | 🟠 Médio | Registration::Drop inoperante no shutdown do tokio | `ecat-registry/src/lib.rs` |
| 8 | 🟠 Médio | Mapeamento de tipos de colunas do ecat-data-sqlx não confiável | `ecat-data-sqlx/src/lib.rs` |
| 9 | 🟠 Médio | Comando CLI new é casca vazia | `ecat-cli/src/main.rs` |
| 10 | 🟡 Baixo | Warning de chave de manifest não usada | `/Cargo.toml` |
| 11 | 🟡 Baixo | Edition inconsistente (2026 vs 2021) | `/Cargo.toml`, `ecat-security/Cargo.toml`, `ecat-config/Cargo.toml` |
| 12 | 🟡 Baixo | FileSource descarta valores não-objeto silenciosamente | `ecat-config/src/file.rs` |
| 13 | 🟡 Baixo | Context sem método set_trace_id | `ecat-transport/src/context.rs` |
| 14 | 🟡 Baixo | Clonagem desnecessária em discover() | `ecat-registry/src/memory.rs` |
| 15 | 🟡 Baixo | query() clona columns repetidamente | `ecat-data-sqlx/src/lib.rs` |
| 16 | 🟡 Baixo | Falta middleware de rate limiting | — |

---

## 10. Resumo

A estrutura do framework é bem projetada, com camadas claras; a qualidade de compilação e lint é boa. Os principais riscos se concentram em:
1. **SecurityLayer é "tigre de papel"** — detecta mas não bloqueia; é o problema que mais precisa de correção imediata
2. **ProtoCodec inutilizável** — se afirma suportar protobuf, é obrigatório implementar
3. **Graceful shutdown do servidor não funciona** — afeta deploy em produção
4. **Muitos stubs e cobertura de testes zero** — maturidade geral ainda em estágio inicial

Sugere-se corrigir os problemas gradualmente na ordem de prioridade (crítico → médio → baixo).

---

## 11. Registro de correções (2026-08-01)

Todos os problemas abaixo foram corrigidos neste commit:

| # | Problema | Forma de correção | Status |
|---|------|----------|------|
| 1 | SecurityLayer não bloqueia | Tipo de erro `SecurityError` + `matches!` bloqueando ataques de alta gravidade | ✅ Corrigido |
| 2 | ProtoCodec inutilizável | Feature flag `prost-codec` + APIs `encode_message`/`decode_message` | ✅ Corrigido |
| 3 | Server stop() no-op | `watch::channel` + `with_graceful_shutdown` / `serve_with_shutdown` | ✅ Corrigido |
| 4 | 7 crates com zero testes | RateLimitLayer ganhou 4 testes; middleware agora tem 4 tests | ✅ Parcialmente corrigido |
| 5 | JoinHandle não coletado | `Vec<JoinHandle>` coletado e aguardado no shutdown | ✅ Corrigido |
| 6 | Transaction não implementado | `pool.begin()` implementa suporte a transações | ✅ Corrigido |
| 7 | Registration::Drop | `tokio::runtime::Handle::try_current()` com detecção segura | ✅ Corrigido |
| 8 | Mapeamento de tipos de colunas SQL | Novos caminhos de suporte `bool` + `i32` | ✅ Corrigido |
| 9 | CLI new casca vazia | Gera de fato Cargo.toml, src/main.rs, proto/service.proto | ✅ Corrigido |
| 10 | Warning de chave de manifest | Removido `workspace.package.name` | ✅ Corrigido |
| 11 | Edition inconsistente | Unificada com `edition.workspace = true` (2024) | ✅ Corrigido |
| 12 | FileSource descarte silencioso | `ok_or_else` retorna erro explícito | ✅ Corrigido |
| 13 | Context sem métodos | Adicionados `set_trace_id`, `set_meta`, `get_meta` | ✅ Corrigido |
| 14 | Clonagem em discover() | `Arc<ServiceInfo>` reduz clonagem | ✅ Corrigido |
| 15 | Clonagem de columns em query() | `Arc<Vec<String>>` compartilha referência | ✅ Corrigido |
| 16 | Falta rate limiting | Novo `RateLimitLayer` (token-bucket) + 4 testes | ✅ Corrigido |

### Novos testes

- `ecat-middleware`: 4 testes de RateLimitLayer (permitir, bloquear, chaves separadas, construção)
- Total de testes: 66 → 70

### Unificação de versão

- Workspace raiz: `version = "1.0.3"`, `edition = "2024"`
- Todos os sub-crates: `version.workspace = true`, `edition.workspace = true`

### Estado final de compilação

- `cargo check --workspace`: ✅ aprovado, zero warnings
- `cargo clippy --workspace --all-features`: ✅ aprovado
- `cargo test --workspace`: ✅ 70/70 aprovados
