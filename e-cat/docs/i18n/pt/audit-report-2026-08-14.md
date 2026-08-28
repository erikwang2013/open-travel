# Relatório de auditoria especializada (segurança e desempenho) — 2026-08-14

Escopo da auditoria: workspace de 55 crates (v2.3.5). Método: verificação manual do Cargo.lock (cargo-audit não instalado), auditoria de código-fonte dos caminhos de autenticação/TLS, verificação de concorrência e ciclo de vida de recursos. Nenhum código commitado.

## Verificação de CVEs nas dependências

- As versões das dependências centrais são relativamente novas e sem CVEs conhecidos não corrigidos: rustls 0.23.43, ring 0.17.14, aws-lc-rs 1.17.3, jsonwebtoken 9.3.1, tokio 1.53.1, h2 0.4.15, quinn 0.11.11, sqlx 0.8.6, zerocopy 0.8.55, time 0.3.54, openssl 0.10.81.
- hyper 0.14.32 (vem apenas do rust-s3 0.35.1, via hyper-tls 0.5) já está acima da linha de correção 0.14.28.
- Observação: o CI não instala cargo-audit; sugere-se adicionar a verificação automatizada ao workflow.

## Descobertas (ordenadas por gravidade)

### S1 [Médio] Handshake TLS do HTTP serializado → DoS de handshake lento
- Local: `ecat-transport-http/src/lib.rs:134-150` (TlsListener::accept)
- Sintoma: o handshake TLS é concluído de forma síncrona dentro de `accept()`; o axum::serve chama accept serialmente — uma conexão que não completa o handshake bloqueia todo o loop de accept.
- Impacto: um atacante abrindo em massa conexões TCP lentas/zumbis pode fazer o serviço parar completamente de aceitar novas conexões (no lado gRPC, o tonic faz spawn do handshake por conexão, não é afetado).
- Sugestão: após accept, `tokio::spawn` o handshake com `tokio::time::timeout(10s)`, fechando a conexão em caso de falha.

### S2 [Médio] Cache de introspecção OAuth2 cresce sem limite → DoS de memória
- Local: `ecat-auth/src/oauth2.rs:45,84-92`
- Sintoma: `HashMap<String,(String,Instant)>` com token como chave; o TTL controla apenas a frescura, sem limite de capacidade, sem evicção.
- Impacto: requisições em massa com tokens únicos podem crescer a memória indefinidamente (cada miss também dispara introspection upstream).
- Sugestão: adicionar limite de capacidade (ex.: 10k) + limpeza periódica, ou trocar para moka/LRU com capacidade e evicção por TTL.

### S3 [Baixo-médio] ecat-data-s3 usa rust-s3 0.35.1 antigo (hyper 0.14 + native-tls/openssl)
- Local: `ecat-data-s3/Cargo.toml` → rust-s3 0.35.1
- Sintoma: o cliente S3 usa de forma independente a stack hyper-tls/openssl; `ecat-tls::TlsClientConfig` (CA customizada, certificado de cliente, skip_verify) não tem efeito no S3; a superfície de configuração TLS fica inconsistente.
- Impacto: CA privada/mTLS do S3 em ambientes corporativos não configurável; dependência com manutenção lenta desde 2023.
- Sugestão: avaliar upgrade do rust-s3 ou unificar com o cliente reqwest/rustls.

### S4 [Baixo] Validação padrão do JWT não inclui iss/aud
- Local: `ecat-auth/src/jwt.rs:125` — `Validation::new(HS256)` apenas assinatura + exp.
- Impacto: com chave compartilhada HS256, o token de um serviço pode ser aceito por outro serviço (sem isolamento por emissor).
- Sugestão: documentar explicitamente que produção deve configurar issuer/audience; ou adicionar por padrão ponto de validação de iss.

### S5 [Baixo] `TlsClientConfig.skip_verify` sozinho já torna `is_enabled()` verdadeiro
- Local: `ecat-tls/src/lib.rs:23-29`
- Sintoma: com apenas `skip_verify: true` configurado, o TLS é considerado "habilitado" e sem verificação de certificado, desligando silenciosamente a validação.
- Sugestão: validação mútua de skip_verify com ca_cert, ou exigir dupla confirmação explícita.

## Desempenho e recursos

### P1 [Baixo] Caminho de cache hit do OAuth2 desserializa JSON em cada requisição
- Local: `ecat-auth/src/oauth2.rs:87` — o cache armazena string serializada; mesmo no hit, ainda faz `serde_json::from_str`.
- Sugestão: armazenar diretamente a struct `AuthClaims` no cache, economizando o parse por requisição.

### P2 [Baixo] ecat-bench sem warmup e sem julgamento de estado estável
- Local: `ecat-bench/src/lib.rs:run_bench` — cronometra direto, sem warmup; o cold start/alocação inicial do pool de conexões entra no p99.
- Sugestão: adicionar rodadas de warmup e verificação de convergência de estado estável para resultados mais confiáveis.

### P3 [Baixo] Consumer Kafka com poll de 100ms + sleep de 100ms em série
- Local: `ecat-mq-kafka/src/lib.rs:84-92` — a latência de ponta a ponta da mensagem fica limitada a ~200ms.
- Sugestão: não precisa de sleep adicional após o poll; em cenários de baixa taxa, pode-se encurtar o intervalo de poll.

## Confirmação de boas práticas

- Caminhos de produção sem panic de unwrap/expect (transport/auth/middleware apenas em testes).
- Fallback de API key via parâmetro de query com log de aviso de vazamento; HashMap usa SipHash contra colisões.
- A camada SQL repassa o SQL do chamador (natureza de framework); user:pass na connection string com percent-encoding correto.
- Canal de consumo Kafka bloqueia por backpressure quando cheio, em vez de descartar; após drop do rx, a task de poll sai normalmente.
- Fetch do config-remote com timeout (5s/30s); consulta bloqueante sem index retorna erro para evitar busy-wait.

---

## Auditoria de corretude do domínio central (complementar, cobre o especializado de segurança/desempenho acima)

Método de auditoria: varredura do código de produção de todo o workspace (localização de unwrap/expect/panic, erros engolidos silenciosamente, parada assíncrona, estado concorrente) + reverificação completa com `cargo test --workspace` (primeira rodada toda verde; a correção de S1 em andamento causou avisos de compilação intermediários no transport-http; após o fechamento, é preciso re-executar). Nenhum código commitado.

### N1 [Médio] Após a saída da task de consumo do ecat-events, o handle vaza → eventos perdidos silenciosamente
- Local: `ecat-events/src/lib.rs:97-101` (loop de consumo nas linhas 89-95 com `None => break`)
- Sintoma: quando o stream do mq retorna None (ex.: fechamento do canal broadcast do kafka) ou a task entra em panic, o loop de consumo sai, mas o JoinHandle permanece no map `consumers`; depois, um novo `subscribe()` do mesmo tipo de evento não reinicia a task de consumo porque `contains_key` na linha 68 é sempre verdadeiro → eventos desse tipo são perdidos silenciosamente para sempre.
- Impacto: após interrupção do stream de eventos remoto, não há auto-recuperação; a recuperação exige reiniciar o processo.
- Sugestão: remover o handle do map no caminho de saída da task (spawn de watcher ou limpeza lazy com `handle.is_finished()`).

### N2 [Médio] Semântica de group_id errada no subscribe do ecat-mq-kafka
- Local: `ecat-mq-kafka/src/lib.rs:71-84`
- a. Com `group_id` padrão None, o `consumer.subscribe()` do rdkafka exige group.id (librdkafka reporta INVALID_ARG); com a configuração padrão, o subscribe provavelmente falha direto (requer validação em máquina real).
- b. Com group_id configurado (o ecat-events faz subscribe uma vez por tipo de evento, mesmo grupo), o Kafka divide o tópico por partições entre os consumidores do mesmo grupo → um tipo de evento pode cair na task de consumo de outro tipo e ser descartado silenciosamente (auto.offset.reset=latest e sem commit).
- Impacto: o barramento de eventos perde eventos no backend kafka.
- Sugestão: gerar group.id aleatório único quando não houver group_id; ou usar assign() no lado do consumidor para atribuir partições explicitamente; documentar que múltiplos subscribes exigem grupos independentes.

### N3 [Baixo] Host vazio não normalizado no GrpcServer/WsServer (correção de D1 incompleta)
- Local: `ecat-transport-grpc/src/lib.rs:52`, `ecat-transport-ws/src/lib.rs:58`
- Sintoma: `addr.parse::<SocketAddr>()` de `GrpcServer::new(":8000")` retorna AddrParseError (verificado empiricamente); o `TcpListener::bind(":8000")` do WsServer resolve para o wildcard IPv6, e em ambientes sem IPv6 a inicialização falha. O HttpServer já normaliza para 0.0.0.0; o comportamento dos três servidores é inconsistente.
- Sugestão: normalizar host vazio dentro de `new` de forma unificada.

### N4 [Baixo] TracingLayer não injeta trace_id, inconsistente com a declaração do CHANGELOG 2.3.3
- Local: `ecat-tracing/src/lib.rs:72-84` (o span contém apenas o campo service; o comentário no código admite que o Req genérico não permite obter headers); `inject_trace_id()` gera novo UUID a cada vez, sem reaproveitar o trace_id extraído upstream.
- Impacto: o tracing distribuído configurado conforme a documentação não consegue correlacionar entre serviços.
- Sugestão: binding tardio dos campos do span ou especialização para `http::Request<B>`; o inject deve suportar carregar o id upstream.

### N5 [Baixo] Panic de job do ecat-scheduler para silenciosamente
- Local: `ecat-scheduler/src/lib.rs:53-57,83` (`let _ = handle.await` em `run()`)
- Sintoma: após panic de uma tarefa agendada, a task morre sem reinício, sem log; `run()` descarta o erro do JoinHandle.
- Sugestão: capturar o panic com log + política de reinício opcional.

### N6 [Baixo] unwrap remanescente em código de produção (caminhos de poison/panic)
- `ecat-events/src/lib.rs:68,98` `Mutex::lock().unwrap()` do std (panic se envenenado); `ecat-versioning/src/lib.rs:86` unwrap do Response builder (não falha, mas é caminho de panic); `ecat-mq/src/lib.rs:110` o expect já é protegido por guarda is_none (seguro).
- Sugestão: os dois pontos de events devem usar `unwrap_or_else(|e| e.into_inner())`.

### N7 [Informação] `WsServer::stop()` não espera conexões WebSocket já atualizadas
- Local: `ecat-transport-ws/src/lib.rs:63-87`
- As conexões on_upgrade do axum rodam em tasks independentes; o graceful shutdown não as cobre; handlers de conexão longa permanecem após `stop()`, o processo não sai limpo (semântica incompleta do App::stop).

### N8 [Informação] Crates com zero testes: ecat-data / ecat-lock / ecat-protos
- Todos são crates de trait/definição; foi verificado que os métodos padrão fazem fail-loud (retornam erro em vez de silêncio), mas os contratos dos traits (semântica de rollback no drop do Transaction, validação de token do lock) não têm nenhum teste unitário.
- Sugestão: adicionar testes mínimos unitários para a semântica de RdbmsError/Transaction e DistributedLock.

### N9 [Informação] Parâmetros e campos aninhados do graphql continuam descartados
- O execute de `ecat-graphql/src/lib.rs` passa apenas `variables` para o resolver; parâmetros de campo de `{ hello(name: "x") }` e selection aninhada não são repassados; o README não menciona essa limitação (o L8 de relatórios antigos pedia documentação; após a reescrita 2.3.3 ainda não foi complementada).

### N10 [Informação] O circuit-breaker conta apenas erros da camada de transporte
- `ecat-circuit-breaker/src/lib.rs:203-209` registra apenas Err do inner como falha; HTTP 5xx é tratado como sucesso → o circuit breaker é ineficaz contra indisponibilidade de serviço (tempestade de 5xx); a documentação não explica.

**Estado da validação**: primeira rodada de `cargo test --workspace` toda verde (incluindo doc-tests, nenhuma falha na saída final); durante a edição da correção de S1 pelo agent, o transport-http apresentou erro de compilação e 2 avisos (unused import `ensure_crypto_provider`, `shutdown_tx` não lido) — estado intermediário; após o fechamento de S1, é necessário re-executar a suíte completa e `clippy --all-targets -D warnings`.

---

## Terceira rodada: validação dinâmica + recheck de CVE + superfície de panic (especializada, 2026-08-14)

### Recheck de CVE (novas descobertas, por gravidade)

1. **[Médio] rustls-webpki 0.102.8 remanescente na árvore de dependências** (RUSTSEC-2026-0049/0098/0099/0104: bypass de CRL distributionPoint, name-constraints de URI/wildcard; versão corrigida 0.103.10). A cadeia principal é 0.103.13 (via rustls 0.23.43, segura); a 0.102.8 entra via async-nats 0.38.0 / rumqttc 0.25.1, cobrindo as cadeias de clientes TLS NATS/MQTT. O upstream não migrou para rustls 0.23 e não há versão corrigida — risco controlado, sugere-se acompanhar com comentário.
2. **[Médio-baixo] rdkafka 0.36.2 embute librdkafka com cJSON 1.7.14** (CVE-2023-53154 e a série cJSON; CVE-2025-57052 marcado CVSS 9.8 mas o arquivo afetado cJSON_utils.c não é usado pelo librdkafka, aplicabilidade duvidosa). A correção upstream está no librdkafka 2.10+ (PR #5346 de 2026-03). O ecat-mq-kafka faz link estático; é preciso conferir a versão empacotada do librdkafka-sys e acompanhar o upgrade.
3. **[Baixo] rustls-pemfile 2.2.0 sem manutenção** (RUSTSEC-2025-0134) — usado pelo ecat-transport-http no parsing de arquivos locais na inicialização, não é entrada de atacante.
4. **[Baixo] rsa 0.9.10** (RUSTSEC-2023-0071 side channel de temporização Marvin) — introduzido via TLS do sqlx-mysql, relevante apenas para cenário MySQL + troca de chave RSA.
5. async-nats 0.38.0 já está acima da linha de correção do RUSTSEC-2023-0027 (bypass de validação de CN), sem problemas.

### Validação dinâmica (examples/helloworld, build debug, porta temporária 18080, já limpo)

- /health 200, / (serialização JSON) 200 (27B), 404 normal; o middleware Logging registra as requisições normalmente.
- **/metrics montado mas retorna 200 + body vazio (0 bytes)**: sem métricas registradas não há saída alguma; o lado de monitoramento não consegue distinguir "saudável/sem métricas". Sugestão: registry vazio deve emitir linha de comentário ou 503.
- Requisição malformada (header com 0x01/0x02) → 400 Bad Request, serviço permanece vivo, /health posterior ainda 200, sem panic.
- Caminhos TLS/mTLS e middlewares de circuit breaker/rate limit: cobertos pelos testes de ecat-transport-http/grpc e ecat-middleware (após a correção da corrida mTLS, tudo verde; casos de rejeição de certificados anônimos/incorretos passam).

### Baseline de benchmark

- ecat-bench não tem alvo [[bench]]/bin, sem entrypoint de cargo bench; run_bench_with_warmup já vem com warmup (correção de P2 implementada), testes do harness todos verdes.
- Medição real foi smoke em build debug: / ~1.3ms, /health ~1.8ms (inclui overhead do processo curl, sem significado de baseline). Sugestão: build release + load test com wrk/hey para obter baseline real.

### Recheck da superfície de panic (todo o workspace, módulos de teste excluídos)

- Total de 31 ocorrências de unwrap/expect/panic, todas de baixo risco: `Response::builder().body().unwrap()` (ramos infalíveis de jwt/apikey/oauth2), fallback de poison de lock (etcd/testing), `clickhouse serde_json::to_string().unwrap()` (panic teórico apenas com entrada extremamente NaN/inf).
- **1 ponto que merece atenção**: `ecat-transport-http/src/tls_listener.rs:234` — quando o loop de accept em segundo plano sai por exceção, há `panic!` dentro de `accept()`, matando a thread do serviço (condição de disparo rigorosa: apenas erro fatal do listener); sugere-se rebaixar para retorno de erro + log.
