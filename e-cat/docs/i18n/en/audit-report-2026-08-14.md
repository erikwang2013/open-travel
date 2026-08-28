# Specialized Audit Report (Security & Performance) — 2026-08-14

Audit scope: 55-crate workspace (v2.3.5). Method: manual review of Cargo.lock (cargo-audit not installed), source audit of auth/TLS paths, concurrency and resource lifecycle checks. No code committed.

## Dependency CVE Review

- Core dependency versions are all recent with no known unfixed CVEs: rustls 0.23.43, ring 0.17.14, aws-lc-rs 1.17.3, jsonwebtoken 9.3.1, tokio 1.53.1, h2 0.4.15, quinn 0.11.11, sqlx 0.8.6, zerocopy 0.8.55, time 0.3.54, openssl 0.10.81.
- hyper 0.14.32 (only from rust-s3 0.35.1, via hyper-tls 0.5) is above the 0.14.28 fix line.
- Note: CI does not install cargo-audit; it is recommended to add it to the workflow for automated checks.

## Findings (sorted by severity)

### S1 [MEDIUM] HTTP TLS handshake serialized → slow-handshake DoS
- Location: `ecat-transport-http/src/lib.rs:134-150` (TlsListener::accept)
- Symptom: the TLS handshake completes synchronously inside `accept()`; axum::serve calls accept serially — one connection that never completes its handshake blocks the entire accept loop.
- Impact: an attacker can open slow/zombie TCP connections in bulk to completely stop the service from accepting new connections (the gRPC side spawns a handshake per connection via tonic and is unaffected).
- Suggestion: after accept, `tokio::spawn` the handshake with a `tokio::time::timeout(10s)` and close the connection on failure.

### S2 [MEDIUM] OAuth2 introspection cache grows unboundedly → memory DoS
- Location: `ecat-auth/src/oauth2.rs:45,84-92`
- Symptom: the `HashMap<String,(String,Instant)>` is keyed by token; the TTL only governs freshness — no capacity cap, no eviction.
- Impact: a flood of unique-token requests can grow memory without limit (each miss also triggers an upstream introspection).
- Suggestion: add a capacity cap (e.g. 10k) plus periodic cleanup, or switch to moka/LRU with capacity and TTL eviction.

### S3 [LOW-MEDIUM] ecat-data-s3 uses old rust-s3 0.35.1 (hyper 0.14 + native-tls/openssl)
- Location: `ecat-data-s3/Cargo.toml` → rust-s3 0.35.1
- Symptom: the S3 client independently uses the hyper-tls/openssl stack; ecat-tls::TlsClientConfig (custom CA, client certificates, skip_verify) has no effect on S3; the TLS configuration surface is inconsistent.
- Impact: private CA/mTLS for S3 cannot be configured in enterprise environments; the dependency has been maintained slowly since 2023.
- Suggestion: evaluate upgrading rust-s3 or switching to the unified reqwest/rustls client.

### S4 [LOW] JWT default validation does not include iss/aud
- Location: `ecat-auth/src/jwt.rs:125` — `Validation::new(HS256)` only checks signature + exp.
- Impact: with an HS256 shared secret, a token issued by one service can be accepted by another (no issuer isolation).
- Suggestion: document that production configuration must set issuer/audience; or add an iss validation entry by default.

### S5 [LOW] TlsClientConfig.skip_verify alone makes is_enabled() true
- Location: `ecat-tls/src/lib.rs:23-29`
- Symptom: configuring only `skip_verify: true` marks TLS as "enabled" while skipping certificate verification, silently disabling validation.
- Suggestion: make skip_verify mutually exclusive with ca_cert, or require explicit double confirmation.

## Performance and Resources

### P1 [LOW] OAuth2 cache hit path deserializes JSON per request
- Location: `ecat-auth/src/oauth2.rs:87` — the cache stores a serialized string; hits still run `serde_json::from_str`.
- Suggestion: cache the `AuthClaims` struct directly, saving a parse per request.

### P2 [LOW] ecat-bench has no warmup or steady-state detection
- Location: `ecat-bench/src/lib.rs:run_bench` — timing starts directly, no warmup; cold-start/pool first allocation skews p99.
- Suggestion: add warmup rounds and steady-state convergence detection for more trustworthy results.

### P3 [LOW] Kafka consumer 100ms poll + 100ms sleep in series
- Location: `ecat-mq-kafka/src/lib.rs:84-92` — end-to-end message latency is capped at about 200ms.
- Suggestion: no need to sleep after poll; the poll interval can be shortened for low-throughput scenarios.

## Confirmed Good Practices

- No unwrap/expect panics on production paths (transport/auth/middleware only in tests).
- API key query-parameter fallback carries a leak-warning log; HashMap uses SipHash against collision attacks.
- The SQL layer passes through caller SQL (framework nature), and user:pass in connection strings is percent-encoded correctly.
- Kafka consumption applies blocking backpressure when the channel is full rather than dropping; after rx is dropped the poll task exits normally.
- config-remote fetches with timeouts (5s/30s); blocking queries fail with a missing-index error instead of busy-waiting.

---

## Core Domain Correctness Audit (supplementary, complements the above security/performance specials)

Audit method: full workspace production-code scan (unwrap/expect/panic localization, silent error swallowing, async shutdown, concurrent state) + full `cargo test --workspace` re-verification (first round all green; the in-progress S1 fix caused a mid-build warning in transport-http, needs a re-run after wrap-up). No code committed.

### N1 [MEDIUM] ecat-events consumer task handle leaks after exit → events silently lost
- Location: `ecat-events/src/lib.rs:97-101` (consume loop lines 89-95 `None => break`)
- Symptom: when the mq stream returns None (e.g. kafka broadcast channel closed) or the task panics, the consume loop exits but the JoinHandle stays in the `consumers` map; later `subscribe()` for the same event type never restarts the consumer because `contains_key` at line 68 is always true → events of that type are silently lost forever.
- Impact: after a remote event-stream interruption the system cannot self-heal; recovery requires a process restart.
- Suggestion: remove the handle from the map on task exit (spawn a watcher or lazily clean up via `handle.is_finished()`).

### N2 [MEDIUM] ecat-mq-kafka subscribe group_id semantics wrong
- Location: `ecat-mq-kafka/src/lib.rs:71-84`
- a. When `group_id` defaults to None, rdkafka `consumer.subscribe()` requires group.id (librdkafka reports INVALID_ARG); with default config the subscription likely fails outright (needs real-hardware verification).
- b. When a group_id is configured (ecat-events subscribes once per event type in the same group), Kafka splits the topic among consumers of the same group by partition → an event type may land in another type's consume task and be silently discarded (auto.offset.reset=latest and no commits).
- Impact: the event bus drops events under the kafka backend.
- Suggestion: generate a random unique group.id when none is provided; or use assign() to explicitly assign partitions on the consumer side; document that multiple subscriptions require separate groups.

### N3 [LOW] GrpcServer/WsServer empty host not normalized (D1 fix incomplete)
- Location: `ecat-transport-grpc/src/lib.rs:52`, `ecat-transport-ws/src/lib.rs:58`
- Symptom: `GrpcServer::new(":8000")` — `addr.parse::<SocketAddr>()` returns AddrParseError (verified by testing); WsServer `TcpListener::bind(":8000")` resolves to the IPv6 wildcard and fails to start on IPv6-less environments. HttpServer already normalizes to 0.0.0.0 — the three server APIs behave inconsistently.
- Suggestion: normalize the empty host uniformly inside `new`.

### N4 [LOW] TracingLayer does not inject trace_id, inconsistent with CHANGELOG 2.3.3 claim
- Location: `ecat-tracing/src/lib.rs:72-84` (span only contains the service field; the code comment admits the generic Req cannot extract headers); `inject_trace_id()` generates a new UUID each time, not reusing an upstream-extracted trace_id.
- Impact: distributed tracing configured per the docs cannot correlate across services.
- Suggestion: lazily bind span fields or specialize on http::Request<B>; inject should carry the upstream id.

### N5 [LOW] ecat-scheduler job panic silently stops the scheduler
- Location: `ecat-scheduler/src/lib.rs:53-57,83` (`let _ = handle.await` in `run()`)
- Symptom: after a scheduled task panics, the task dies with no restart and no log; `run()` discards the JoinHandle error.
- Suggestion: catch the panic, log it, and add an optional restart policy.

### N6 [LOW] Residual unwraps in production code (poison/panic paths)
- `ecat-events/src/lib.rs:68,98` std `Mutex::lock().unwrap()` (panics on poisoning); `ecat-versioning/src/lib.rs:86` Response builder unwrap (infallible but still a panic path); `ecat-mq/src/lib.rs:110` expect is guarded by is_none (safe).
- Suggestion: change the two events spots to `unwrap_or_else(|e| e.into_inner())`.

### N7 [INFO] WsServer::stop() does not wait for upgraded WebSocket connections
- Location: `ecat-transport-ws/src/lib.rs:63-87`
- axum on_upgrade connections run in separate tasks that graceful shutdown does not cover; long-lived connection handlers linger after stop(), so process exit is not clean (App::stop semantics incomplete).

### N8 [INFO] Zero-test crates: ecat-data / ecat-lock / ecat-protos
- All are trait/definition-type crates; the default methods are verified fail-loud (return errors rather than silently succeeding), but the trait contracts (Transaction drop rollback semantics, lock token validation) have no unit tests at all.
- Suggestion: add minimal unit tests for RdbmsError/Transaction and DistributedLock semantics.

### N9 [INFO] GraphQL arguments and nested fields still discarded
- `ecat-graphql/src/lib.rs` execute only passes `variables` to the resolver; field arguments and nested selections of `{ hello(name: "x") }` are not propagated; the README does not document this limitation (old report L8 required documentation; still missing after the 2.3.3 rewrite).

### N10 [INFO] circuit-breaker only counts transport-layer errors
- `ecat-circuit-breaker/src/lib.rs:203-209` records only inner Err as failure; HTTP 5xx counts as success → the breaker does nothing for service unavailability (5xx storms); undocumented.

**Verification status**: first-round `cargo test --workspace` all green (incl. doc-tests, no failures at the tail of the output); during the S1 fix agent's edits, transport-http showed a compilation error and 2 warnings (unused import `ensure_crypto_provider`, `shutdown_tx` unread) — an intermediate state; after S1 wrap-up, tests and `clippy --all-targets -D warnings` need a full re-run.

---

## Third Round: Dynamic Verification + CVE Recheck + Panic Surface (special, 2026-08-14)

### CVE Recheck (new findings, by severity)

1. **[MEDIUM] rustls-webpki 0.102.8 remains in the dependency tree** (RUSTSEC-2026-0049/0098/0099/0104: CRL distributionPoint bypass, URI/wildcard name-constraints; fixed in 0.103.10). The main chain is 0.103.13 (via rustls 0.23.43, safe); 0.102.8 is pulled in via async-nats 0.38.0 / rumqttc 0.25.1, covering the NATS/MQTT TLS client chains. Upstream has not migrated to rustls 0.23, no fixed version — controlled risk, recommend a tracking comment.
2. **[MEDIUM-LOW] rdkafka 0.36.2's embedded librdkafka carries cJSON 1.7.14** (CVE-2023-53154 and the cJSON series; CVE-2025-57052 is rated CVSS 9.8 but the affected file cJSON_utils.c is not used by librdkafka, applicability questionable). Upstream fix is in librdkafka 2.10+ (2026-03 PR #5346). ecat-mq-kafka links statically; the packaged librdkafka-sys version must be checked and the upgrade tracked.
3. **[LOW] rustls-pemfile 2.2.0 unmaintained** (RUSTSEC-2025-0134) — ecat-transport-http parses local files at startup; not attacker input.
4. **[LOW] rsa 0.9.10** (RUSTSEC-2023-0071 Marvin timing side-channel) — pulled in via sqlx-mysql TLS; only relevant to MySQL + RSA key-exchange scenarios.
5. async-nats 0.38.0 is above the RUSTSEC-2023-0027 (CN validation bypass) fix line, no issue.

### Dynamic Verification (examples/helloworld, debug build, temporary port 18080, cleaned up)

- /health 200, / (JSON serialization) 200 (27B), 404 works; Logging middleware records requests normally.
- **/metrics is mounted but returns 200 + empty body (0 bytes)**: with no metrics registered there is no output at all; the monitoring side cannot distinguish "healthy/no metrics". Suggest a comment line or 503 for an empty registry.
- Malformed requests (headers containing 0x01/0x02) → 400 Bad Request; the service stays alive and subsequent /health still returns 200, no panic.
- TLS/mTLS paths and breaker/rate-limit middleware: covered by ecat-transport-http/grpc and ecat-middleware tests (all green after the mTLS race fix; anonymous/wrong-client-cert rejection cases pass).

### bench baseline

- ecat-bench has no [[bench]]/bin target, no cargo bench entry; run_bench_with_warmup already includes warmup (P2 fix landed), harness tests all green.
- Measured as a debug-build smoke: / about 1.3ms, /health about 1.8ms (includes curl process overhead, no baseline significance). Suggest release build + wrk/hey load testing for a real baseline.

### Panic Surface Recheck (full workspace, excluding test modules)

- 31 unwrap/expect/panic occurrences total, all low risk: `Response::builder().body().unwrap()` (infallible branches in jwt/apikey/oauth2), lock-poison fallbacks (etcd/testing), clickhouse `serde_json::to_string().unwrap()` (theoretical panic on extreme NaN/inf input).
- **1 spot to note**: `ecat-transport-http/src/tls_listener.rs:234` — when the background accept loop exits abnormally, `accept()` panics internally and the service thread dies (hard to trigger: only fatal listener errors); suggest downgrading to an error return plus logging.
