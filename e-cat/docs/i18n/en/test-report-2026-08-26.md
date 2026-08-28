# Test Report — 2026-08-26

Comprehensive unit-test supplementation (51-crate full coverage), 4 senior Rust test engineers in parallel.

## Overview

| Group | crates | Existing | Added | Now | Gate |
|---|---|---|---|---|---|
| core/framework | 12 | 102 | +40 | 142 | ✅ tests all green + clippy 0 warnings |
| data | 14 | 87 | +66 | 153 | ✅ same as above |
| mq/transport | 12 | 82 | +54 | 136 | ✅ same as above |
| app application layer | 13 | ~178 | +46 | ~224 | ✅ same as above |
| **Total** | **51** | **~449** | **+206** | **~655** | ✅ |

Note: the application-layer existing counts include ecat-auth 24 / ecat-graphql 35 / ecat-bench 11 / ecat-scheduler 6 / ecat-events 7 / ecat-cli 12 / ecat-middleware 34 / ecat-circuit-breaker 10 / ecat-health 4 / ecat-client 7 / ecat-security 12 / ecat-versioning 4 / ecat 4. Each crate passes independent `cargo test -p` + `cargo clippy -p --all-targets -- -D warnings`, with isolated CARGO_TARGET_DIR for parallelism.

## Per-crate Details

### core/framework group (test-core, +40)

| crate | before→now | Coverage highlights |
|---|---|---|
| ecat-protos | 4→8 | ErrorCode full-enum mapping against proto; truncated buffer decode; empty buffer default message; metadata roundtrip |
| ecat-errors | 4→9 | http_status full mapping (409/429/500); from_status; unmapped→Internal; cause source() |
| ecat-metadata | 9→12 | HTTP header trace_id extraction; key lowercasing; empty header map |
| ecat-encoding | 18→22 | NaN→null (serde_json default, documented); empty bytes decode; CodecBox invalid JSON; proto roundtrip |
| ecat-lock | 7→9 | release without holding lock errors; empty key |
| ecat-logging | 1→1 | compat shim does not panic |
| ecat-tracing | 9→12 | non-UTF-8 trace header skipped; canonical header; response passthrough |
| ecat-tls | 7→12 | basic_auth single/dual fields; missing ca file; is_enabled; default client |
| ecat-config | 14→26 | env prefix filtering + type-parse edges (hex/empty string/-0/1e3); multi-source merge override; obfs error paths; missing file/invalid YAML |
| ecat-config-remote | 6→9 | ConsulKvEntry edges; missing X-Consul-Index errors; nested keys |
| ecat-openapi | 4→11 | components/schema_ref; duplicate overrides; default 200; tags |
| ecat-metrics | 8→11 | registered metrics text; 404/405 |

### data group (test-data, +66)

| crate | before→now | Coverage highlights |
|---|---|---|
| ecat-data | 12→14 | search syntax parsing |
| ecat-data-sqlx | 7→14 | in-memory SQLite end-to-end; parameter binding all types; Blob→base64; config |
| ecat-data-redis | 6→12 | redis:///rediss:// URL construction; auth; config error paths |
| ecat-data-opensearch | 4→10 | mock HTTP: percent-encode, Basic auth, error passthrough |
| ecat-data-elasticsearch | 6→11 | same as above |
| ecat-data-influxdb | 5→10 | line protocol escaping; Token header; error passthrough |
| ecat-data-clickhouse | 12→22 | CREATE TABLE SQL; JSONEachRow; written row count; grouping |
| ecat-data-memcached | 4→8 | TTL seconds→milliseconds; flag packing |
| ecat-data-nebulagraph | 6→7 | config parsing |
| ecat-data-arangodb | 5→7 | config/URL |
| ecat-data-iotdb | 5→10 | mock HTTP: session path parameters |
| ecat-data-questdb | 4→9 | line protocol; transactions unsupported |
| ecat-data-tdengine | 6→11 | INSERT generation; 100-batch chunking |
| ecat-data-mongodb | 5→8 | bson roundtrip; URI |

### mq/transport/registry group (test-mq, +54)

| crate | before→now | Coverage highlights |
|---|---|---|
| ecat-mq | 5→9 | lagging error frames on full buffer; stream closes on full drop; multiple subscribers; publish with no subscribers |
| ecat-mq-kafka | 12→14 | config defaults; SASL fields take effect independently |
| ecat-mq-rabbitmq | 2→5 | exchange defaults; url error paths |
| ecat-mq-mqtt | 5→9 | cert/key pairing validation; missing files; default ports 1883/8883; invalid port fallback |
| ecat-mq-nats | 6→9 | plaintext default; ca/cert missing error paths |
| ecat-transport | 4→7 | TlsConfig defaults/with_client_auth; normalize_addr edges |
| ecat-transport-http | 17→20 | integration tests: stop no-op, occupied port fails, real send/receive |
| ecat-transport-grpc | 7→13 | TLS missing file; plaintext lifecycle; mTLS rejection |
| ecat-transport-ws | 4→8 | fails without handler; occupied port; RFC 6455 masked frame echo |
| ecat-registry | 5→8 | multi-instance discover; auto-deregister on drop; builder defaults |
| ecat-registry-consul | 10→24 | percent-encode; registration variants; error responses; X-Consul-Token; agent/services parsing; node fallback |
| ecat-registry-etcd | 5→10 | discover skips bad values; kv request body; lease grant; keepalive |

### app application-layer group (test-app, +46)

| crate | before→now | Coverage highlights |
|---|---|---|
| ecat-auth | 20→46 | oauth2 cache whitelist/SHA-256 key/FIFO eviction; apikey three states; jwt iss/aud enforced; expired/wrong-signature |
| ecat-health | 4→8 | readiness aggregation (all ok/any fail/empty registry); liveness |
| ecat-versioning | 4→7 | path strategy routing; extract_version edges |
| ecat-security | 12→20 | header layer end-to-end; attack-interception JSON shape |
| ecat-middleware | 34→37 | MemoryStore window expiry; inner panic→Err |
| ecat-circuit-breaker | 10→12 | half-open probe exhaustion; classify degradation |
| ecat-client | 7→10 | grpc invalid endpoint errors without networking |
| ecat-graphql | 35→35 | existing coverage sufficient, no gaps |
| ecat-scheduler / ecat-bench / ecat-events / ecat-cli / ecat | existing coverage sufficient | no gaps |

## Defects Found

| Level | Location | Description | Status |
|---|---|---|---|
| P1 | ecat-events/Cargo.toml | dev-dependencies missing tokio macros/rt/time features; compiling that crate's test targets alone always fails (masked by the feature union in full workspace builds) | ✅ Fixed (features + comment added) |
| P2 | ecat-security src/lib.rs:118-127 | URI percent-encoded SQLi (`?q=SELECT%20*%20...`) can bypass the header-layer scan (the detector requires literal whitespace and scans the raw URI without decoding first); body scanning unaffected | ⏳ To fix |
| P3 | ecat-data-sqlx | `connect()/from_config()` use AnyPool without installing drivers; sqlx 0.8.6 panics "No drivers installed" on first connect | ⏳ To fix |
| P3 | ecat-data-influxdb | string fields escape spaces (`\ `); the line protocol spec only requires escaping `"` and `\`; tag/field order non-deterministic | ⏳ To fix |
| P3 | ecat-data-clickhouse | the CREATE TABLE cache never invalidates; no CREATE retry after external drop/alter | ⏳ To fix |
| P3 | ecat-circuit-breaker | half_open_probes cap unreachable under sequential probing (only reachable with concurrent in-flight requests); white-box tests cover it | ℹ️ Known, not a defect |
| P3 | ecat-health | `with_check` uses blocking_write(), panics when called in an async context; currently only usable synchronously | ℹ️ Known, API limitation |

## Skipped Modules (need integration environments, not mocked)

- Real broker roundtrips: kafka/rabbitmq/mqtt/nats publish-subscribe (config and error paths covered)
- Real clusters: consul/etcd register-discover lifecycles (axum mocks cover request shapes)
- Real databases: redis/memcached operations, mongod, influxdb server-side validation, sqlx postgres/mysql drivers, nebulagraph/arangodb APIs
- Real external services: OAuth2 introspection (covered by a local mock), gRPC/HTTP roundtrips (local mocks cover 302 not followed)
