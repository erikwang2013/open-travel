<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat Relatório de revisão de código e testes TDD

**Data**: 2026-07-29  
**Branch**: main  
**Projeto**: e-cat (workspace Rust, 17 crates)

---

## 1. Escopo da revisão

Todo o código-fonte Rust dos 17 crates do workspace foi revisado (38 arquivos `.rs`).

| Crate | Descrição | Nº de arquivos |
|-------|------|--------|
| `ecat-protos` | Definições Protobuf e geração de código | 2 |
| `ecat-errors` | Tipos de erro unificados | 2 |
| `ecat-metadata` | Abstração de metadados de requisição | 1 |
| `ecat-encoding` | Codificação/decodificação JSON/Protobuf | 3 |
| `ecat-logging` | Inicialização de logs/Tracing | 1 |
| `ecat-config` | Carregamento de configuração (arquivo/variáveis de ambiente) | 3 |
| `ecat-data` | Abstrações de trait da camada de dados | 5 |
| `ecat-data-sqlx` | Implementação RDBMS com SQLx | 1 |
| `ecat-registry` | Registro/descoberta de serviço | 2 |
| `ecat-metrics` | Métricas Prometheus | 1 |
| `ecat-middleware` | Camadas de middleware Tower | 4 |
| `ecat-transport` | Abstração da camada de transporte | 4 |
| `ecat-transport-http` | Implementação de transporte HTTP/Axum | 1 |
| `ecat-transport-grpc` | Implementação de transporte gRPC/Tonic | 1 |
| `ecat` | Núcleo do framework de aplicação | 3 |
| `ecat-cli` | Ferramenta CLI | 1 |
| `examples/helloworld` | Projeto de exemplo | 1 |

---

## 2. Problemas encontrados e correções

### Problema 1: [Clippy] `map_identity` — map de identidade sem sentido

- **Arquivo**: `ecat-config/src/file.rs:30`
- **Gravidade**: baixa
- **Problema**: `map(|(k, v)| (k, v))` não faz nenhuma transformação; é código morto
- **Correção**: remover a chamada `.map()` redundante

### Problema 2: [Clippy] `new_without_default` — Config sem implementação de Default

- **Arquivo**: `ecat-config/src/lib.rs:27`
- **Gravidade**: baixa
- **Problema**: `Config` tem o método `new()` mas não implementa o trait `Default`
- **Correção**: usar `#[derive(Default)]` em vez da implementação manual

### Problema 3: [Clippy] `io_other_error` — construção de Error no estilo antigo

- **Arquivo**: `ecat-middleware/src/recovery.rs:42`
- **Gravidade**: baixa
- **Problema**: `std::io::Error::new(std::io::ErrorKind::Other, ...)` já tem alternativa mais concisa
- **Correção**: usar `std::io::Error::other("task panicked")`

### Problema 4: [Clippy] `redundant_async_block` — bloco async redundante

- **Arquivo**: `ecat-middleware/src/tracing.rs:38`
- **Gravidade**: baixa
- **Problema**: o bloco async em `Box::pin(async move { fut.await })` é desnecessário
- **Correção**: simplificar para `Box::pin(fut)`

### Problema 5: [Clippy] `redundant_closure` — closure redundante

- **Arquivo**: `ecat-data-sqlx/src/lib.rs:63`
- **Gravidade**: baixa
- **Problema**: a closure em `.and_then(|f| serde_json::Number::from_f64(f))` pode ser omitida
- **Correção**: usar diretamente `.and_then(serde_json::Number::from_f64)`

### Problema 6: [Clippy] `unwrap_or_default` — pode ser simplificado com unwrap_or_default

- **Arquivo**: `ecat-transport-http/src/lib.rs:27`
- **Gravidade**: baixa
- **Problema**: `unwrap_or_else(Router::new)` equivale a `unwrap_or_default()`
- **Correção**: usar `unwrap_or_default()`

---

## 3. Situação da cobertura de testes

### Antes da correção

| Crate | Nº de testes |
|-------|--------|
| `ecat-errors` | 4 |
| `ecat-transport` | 11 |
| Outros 15 crates | **0** |
| **Total** | **15** |

### Depois da correção

| Crate | Nº de testes | Novos | Conteúdo dos testes |
|-------|--------|------|----------|
| `ecat-encoding` | 15 | +15 | Roundtrip de codificação/decodificação do JsonCodec, decodificação inválida, content_type; despacho do CodecBox; caminhos normal/erro de codec_from_content_type; variantes de Encoding |
| `ecat-errors` | 4 | — | Mapeamento de status HTTP, conversão de status gRPC, acumulação de metadata, formato Display |
| `ecat-metadata` | 9 | +9 | Leitura/escrita de chave-valor, trace_id, From\<HeaderMap\> (inclui pular valores não UTF-8), From\<MetadataMap\> (pular ASCII e binário), IntoIterator |
| `ecat-logging` | 1 | +1 | Teste de fumaça do init |
| `ecat-config` | 4 | +4 | Novo/valores padrão, leitura tipada, carregamento de ConfigSource |
| `ecat-registry` | 5 | +5 | Registro/descoberta, deregistro/remoção, erro de inexistente, listagem de serviços, filtro por nome |
| `ecat-metrics` | 2 | +2 | Registry singleton, metrics_text sem panic |
| `ecat` | 4 | +4 | Valores padrão do Builder, nome/versão customizados, registro de server, lifecycle hook |
| `ecat-transport` | 11 | — | Criação de Context/Request/Response e valores padrão, trait Server |
| **Total** | **55** | **+40** | |

### Crates sem necessidade de testes unitários

- `ecat-protos` — apenas geração de código protobuf
- `ecat-data` — apenas definições de trait, sem lógica de implementação
- `ecat-data-sqlx` — exige conexão com banco de dados; pertence ao escopo de testes de integração
- `ecat-middleware` — implementações de Tower Service, exigem testes de integração
- `ecat-transport-http` / `ecat-transport-grpc` — exigem escuta de rede; pertencem ao escopo de testes de integração
- `ecat-cli` — apenas saída de impressão, sem lógica

---

## 4. Resultados da verificação

```
cargo test   → 55 passed, 0 failed
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
```

---

## 5. Lista de arquivos modificados

| Arquivo | Alteração |
|------|------|
| `ecat-config/src/file.rs` | Removido o identity map |
| `ecat-config/src/lib.rs` | `#[derive(Default)]` + 4 testes |
| `ecat-data-sqlx/src/lib.rs` | Simplificada a closure redundante |
| `ecat-middleware/src/recovery.rs` | Uso de `std::io::Error::other()` |
| `ecat-middleware/src/tracing.rs` | Removido o bloco async redundante |
| `ecat-transport-http/src/lib.rs` | `unwrap_or_else` → `unwrap_or_default` |
| `ecat-metrics/src/lib.rs` | 2 testes |
| `ecat-registry/src/memory.rs` | 5 testes |
| `ecat/src/lib.rs` | 4 testes |
