<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat Relatório de revisão de código (segunda rodada)

**Data**: 2026-07-29  
**Branch**: main  
**Projeto**: e-cat (workspace Rust, 17 crates)

---

## 1. Resumo da revisão

Com base nas correções de clippy e no complemento de testes da primeira rodada, esta rodada realizou uma revisão profunda da lógica de código, com foco em correção em tempo de execução, segurança de concorrência e consistência semântica da API. Foram revisados 32 arquivos-fonte.

### Linha de base de verificação

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

---

## 2. Bugs encontrados e correções

### Bug 1: [Crítico] Erro de ciclo de vida do guard de span do TracingLayer

- **Arquivo**: `ecat-middleware/src/tracing.rs:37`
- **Gravidade**: **alta**
- **Impacto**: todas as requisições que passam pelo TracingLayer não são cobertas pelo span de tracing

**Análise de causa raiz**:

```rust
// 修复前
fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let _guard = span.enter();  // guard 在 call() 返回时 drop
    let fut = self.inner.call(req);
    Box::pin(fut)               // future 在后续 poll 时才执行
}
```

O guard retornado por `span.enter()` mantém o span ativo apenas no contexto síncrono atual. `call()` retorna um future que ainda não foi polled; a execução assíncrona real ocorre na fase de poll posterior — nesse momento o guard já foi dropado e o span não tem efeito. Nenhuma requisição que passa pelo TracingLayer aparece na saída do tracing.

**Correção**:

```rust
// 修复后
use tracing::Instrument;

fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let fut = self.inner.call(req);
    Box::pin(fut.instrument(span))  // span 附着在 future 生命周期上
}
```

Usar `tracing::Instrument::instrument()` para anexar o span ao future garante que o span permaneça ativo durante todo o ciclo de vida de poll do future.

---

### Bug 2: [Crítico] Defeito na implementação de closure do LifecycleHook — on_stop nunca executa

- **Arquivo**: `ecat/src/hook.rs:14-23`, `ecat/src/lib.rs:11-16`
- **Gravidade**: **alta**
- **Impacto**: hooks de closure registrados via `.on_stop()` não fazem nada no shutdown

**Análise de causa raiz**:

No design original, tanto `on_start()` quanto `on_stop()` empurram os hooks para o mesmo Vec `lifecycle_hooks`. Em `run()`, todos os hooks chamam `on_start()` em sequência; no shutdown, todos os hooks chamam `on_stop()` em sequência.

O problema está no blanket impl do trait `LifecycleHook` para closures `Fn() -> Fut`: **ele cobre apenas `on_start()`; `on_stop()` usa a implementação padrão do trait (no-op)**.

Isso significa que, ao usar a sintaxe de closure `.on_stop(|| async { ... })`, a closure é adicionada à lista de hooks, mas no shutdown apenas o `on_stop()` vazio padrão é executado — a lógica do usuário nunca roda.

**Correção (duas partes)**:

1. **Separar start_hooks e stop_hooks** (`ecat/src/lib.rs`):

```rust
// App 结构体 — 两个独立的 Vec
pub struct App {
    start_hooks: Vec<Box<dyn LifecycleHook>>,
    stop_hooks: Vec<Box<dyn LifecycleHook>>,
    // ...
}

// on_start() → start_hooks, on_stop() → stop_hooks
pub fn on_start(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.start_hooks.push(Box::new(hook));
    self
}
pub fn on_stop(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.stop_hooks.push(Box::new(hook));
    self
}
```

2. **Completar o blanket impl de closures** (`ecat/src/hook.rs`):

```rust
impl<F, Fut> LifecycleHook for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Result<...>> + Send,
{
    async fn on_start(&self) -> ... { (self)().await }
    async fn on_stop(&self) -> ...  { (self)().await }  // 新增
}
```

Agora as closures implementam `on_start` e `on_stop`; combinado com os Vecs separados, cada hook é chamado apenas na fase correta do ciclo de vida.

---

### Bug 3: [Médio] Prioridade errada na extração de tipos de valores de Row do SqlxClient

- **Arquivo**: `ecat-data-sqlx/src/lib.rs:53-68`
- **Gravidade**: média
- **Impacto**: valores inteiros e de ponto flutuante no banco são extraídos como strings JSON em vez de números

**Análise de causa raiz**:

`try_get::<String>()` é tentado primeiro. A maioria dos drivers de banco consegue executar `try_get::<String>()` em colunas numéricas (conversão implícita), fazendo com que o inteiro `42` seja extraído como `"42"` em vez de `42`.

**Correção**: ajustar a ordem de tentativa do `try_get` para `i64 → f64 → String → Null`, preservando os tipos numéricos com prioridade.

---

## 3. Outras descobertas da revisão (não modificadas / limitações conhecidas)

| Categoria | Arquivo | Descrição | Sugestão |
|------|------|------|------|
| Funcionalidade incompleta | `ecat-transport-http/src/lib.rs:30` | `axum::serve().await` bloqueia e nunca retorna; `stop()` é no-op | Implementar graceful shutdown |
| Funcionalidade incompleta | `ecat-transport-grpc/src/lib.rs:29` | Idem | Implementar graceful shutdown |
| Funcionalidade incompleta | `ecat-data-sqlx/src/lib.rs:79` | `transaction()` retorna erro "não implementado" | Implementar suporte a transações |
| Estilo de código | `ecat-middleware/src/logging.rs:42` | `elapsed.as_millis() as u64` truncamento teórico u128→u64 | Sem impacto real |
| Testes ausentes | `ecat-middleware/` | 4 Tower Services sem testes unitários | Exigem testes de integração |
| Testes ausentes | `ecat-data/` | Apenas definições de trait | Aceitável no momento |
| Bloqueio RwLock | `ecat-registry/src/memory.rs` | RwLock síncrono pode bloquear em contexto assíncrono | Considerar `tokio::sync::RwLock` |

---

## 4. Resultados dos testes

```
cargo test → 60 passed, 0 failed

Distribuição por crate:
  ecat                  4   (Builder/valores padrão/hooks de ciclo de vida)
  ecat-config           9   (env parse ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (mapeamento HTTP/conversão gRPC/metadata/Display)
  ecat-logging          1   (fumaça do init)
  ecat-metadata         9   (leitura/escrita/From HeaderMap/From MetadataMap/iterador)
  ecat-metrics          2   (singleton/text sem panic)
  ecat-registry         5   (registro/descoberta/deregistro/lista/filtro)
  ecat-transport       11   (Context/Request/Response/trait Server)
  outros 8 crates       0   (trait puro/geração de código/exige teste de integração/impressão pura)
```

---

## 5. Lista de arquivos modificados

| Arquivo | Tipo de alteração | Descrição da alteração |
|------|----------|----------|
| `ecat/src/lib.rs` | Correção de bug | App separa start_hooks/stop_hooks; AppBuilder atualizado; testes adaptados |
| `ecat/src/hook.rs` | Correção de bug | Blanket impl de closure completa a implementação de on_stop() |
| `ecat-middleware/src/tracing.rs` | Correção de bug | guard de span → `fut.instrument(span)` |
| `ecat-data-sqlx/src/lib.rs` | Correção de bug | Ordem de extração de valores de Row i64→f64→String→Null |

---

## 6. Resumo

Esta rodada encontrou 2 bugs de runtime de alta gravidade e 1 problema de correção de dados de média gravidade:

1. **TracingLayer span inoperante** — afeta a observabilidade de todas as requisições
2. **LifecycleHook on_stop não executa** — afeta a correção de toda a lógica de shutdown
3. **Perda de tipos numéricos de Row** — afeta a correção de tipos dos resultados de consultas

Os três problemas foram corrigidos; após a correção, os 60 testes passam, com zero erros e zero warnings de compilação.

### Recomendações futuras

- Implementar graceful shutdown para os servidores HTTP/gRPC
- Adicionar testes de integração para `ecat-middleware` (mock Service + verificar comportamento de span/timeout/recuperação)
- Adicionar testes de integração para `ecat-data-sqlx` (usando SQLite em memória)
- Substituir o RwLock síncrono de `ecat-registry/memory.rs` por `tokio::sync::RwLock`
