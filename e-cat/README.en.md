<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Open Travel — Global Travel Platform

<p align="center"><img src="../docs/mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>


[简体中文](README.md) | English | [日本語](../docs/i18n/ja/README.md) | [한국어](../docs/i18n/ko/README.md) | [Русский](../docs/i18n/ru/README.md) | [Deutsch](../docs/i18n/de/README.md) | [Français](../docs/i18n/fr/README.md) | [Español](../docs/i18n/es/README.md) | [Português](../docs/i18n/pt/README.md) | [हिन्दी](../docs/i18n/hi/README.md) | [العربية](../docs/i18n/ar/README.md) | [বাংলা](../docs/i18n/bn/README.md) | [Bahasa Indonesia](../docs/i18n/id/README.md)

> A global travel booking platform: Rust microservices backend (built on the **e-cat** framework) + Flutter / HarmonyOS multi-platform clients, supporting **12+ languages**, international payments, and multilingual search.

## Overview

Open Travel is a global travel platform monorepo. The backend is built on **e-cat** (a.k.a. 一只猫), a Rust microservices framework (v3.0.3 · 51 crates) inspired by [go-kratos/kratos](https://github.com/go-kratos/kratos) v3 — API-first development, pluggable component architecture, and a unified HTTP/gRPC middleware abstraction.

| Dimension | Description |
| :--- | :--- |
| **Backend** | e-cat (Rust): HTTP/axum + gRPC/tonic, 51-crate microservice ecosystem |
| **Services** | user-service (:8001) through payment-service (:8009) — 9 services; business logic in `e-cat/ecat/src/business/`, service entries in `e-cat/ecat/src/bin/` |
| **Gateway** | Nginx (`config/nginx.conf`), prefix-based routing |
| **Clients** | `apps/client/flutter` (iOS / Android / Web / Desktop), `apps/client/harmonyos` (HarmonyOS) |
| **Data** | MySQL + Redis cache + OpenSearch multilingual search |
| **Security** | ecat-security / ecat-auth (JWT) / ecat-tls: auth, audit, rate limiting, injection defense |
| **i18n** | 12+ languages, RTL support, OpenSearch multilingual tokenization |

## Project Structure

```
open-travel/
├── apps/                  # client/ multi-platform clients + admin/ admin console
├── config/                # docker-compose.yml, nginx.conf, schema.sql, opensearch.yml
├── docs/                  # Planning docs, integration/loadtest reports, SVG diagrams, i18n
├── scripts/               # opensearch_init / loadtest / cdn_setup / cdn_upload / release
└── e-cat/                 # e-cat framework + business services (single Cargo workspace)
    ├── ecat*/             # 51 ecat-* framework crates
    ├── ecat/              # Main framework crate: facade + business modules (src/business/) + service entries (src/bin/)
    ├── config/            # Framework config examples
    └── examples/          # Framework examples
```

## Business Services

| Service | Port | Description |
|---------|------|-------------|
| user-service | 8001 | `POST /api/v1/user/register`, `POST /api/v1/user/login` (public), `GET /api/v1/user/profile` (JWT required) |
| booking-service | 8002 | `GET /api/v1/booking/dates?region_id=N` (public), `/api/v1/reviews/` reviews |
| admin-service | 8003 | Admin: login + destination/attraction CRUD (`/api/v1/admin/`) |
| search-service | 8004 | Multilingual search (`/api/v1/search`) |
| line-service | 8005 | Travel lines (`/api/v1/lines`) |
| order-service | 8006 | Orders (`/api/v1/orders`) |
| flight-service | 8007 | Flights (`/api/v1/flights`) |
| hotel-service | 8008 | Hotels (`/api/v1/hotels`) |
| payment-service | 8009 | Payments (`/api/v1/payments`) |
| Nginx gateway | 8082→80 | Prefix routing: `/api/v1/user/`, `/api/v1/booking/`, `/api/v1/admin/`, `/api/v1/search`, `/api/v1/lines`, `/api/v1/orders`, `/api/v1/flights`, `/api/v1/hotels`, `/api/v1/payments` |

All services expose `GET /health` (liveness) and `GET /ready` (readiness, reports degraded data-source state).

> See the [API Reference](../docs/api.md) for request/response examples, auth, and rate-limit details.

## One-Click Install

Requirements: Docker + Docker Compose (v2). (The Rust toolchain is only needed when building from source.)

```bash
git clone <repo-url> && cd open-travel
./scripts/install.sh
```

The script automatically: checks the environment → builds and starts all services (MySQL / Redis / OpenSearch / Kafka / 9 microservices / Nginx gateway) → initializes search indexes → runs a health check, then prints the access URL and the default admin account.

> Usage: check liveness at http://localhost:8082/health; log into the admin console with `admin@travel.local` / `Admin@123`; start the client from `apps/client/flutter` with `flutter run -d chrome`.

## Quick Start

### Prerequisites

- Rust 1.85+ (stable toolchain, required for edition 2024) + [protoc](https://github.com/protocolbuffers/protobuf)
- Docker + Docker Compose

### Build

```bash
cd e-cat
cargo check -p ecat --bin user-service --bin booking-service --bin admin-service \
  --bin search-service --bin line-service --bin order-service --bin flight-service \
  --bin hotel-service --bin payment-service   # compile-check the business services
```

Run locally in development mode (listening on `0.0.0.0:8001` – `0.0.0.0:8009`):

```bash
cd e-cat
cargo run -p ecat --bin user-service &
cargo run -p ecat --bin booking-service
```

Build Docker images (`e-cat/Dockerfile`, builds the nine services with `-p ecat --bin`, binaries at `/usr/local/bin/` in the image):

```bash
docker build -f e-cat/Dockerfile -t travel-services .
```

### Start (Docker Compose)

```bash
docker compose -f config/docker-compose.yml up -d
```

> ⚠️ Do NOT start with `--env-file .env` (it errors out).

### Verify

Business endpoint versions live in the URL prefix: `/api/v1/...` (no version header required).

Curl the services directly:

```bash
curl http://localhost:8002/health                 # OK
curl "http://localhost:8002/api/v1/booking/dates?region_id=1"
# {"code":0,"message":"ok","data":[{"region_id":1,"name_en":"placeholder-destination"}]}
curl -H "Content-Type: application/json" -X POST \
  http://localhost:8001/api/v1/user/register -d '{"email":"a@b.com","password":"secret1"}'
# {"code":0,"message":"ok","data":{"user_id":1,"email":"a@b.com","lang":"en"}}
```

Through the gateway (Nginx, host `8082` → container `80`):

```bash
curl "http://localhost:8082/api/v1/booking/dates?region_id=1"
curl http://localhost:8082/health
```

Authenticated endpoints (`/api/v1/user/profile`) require a JWT in the request header:

```bash
curl -H "Authorization: Bearer <JWT>" http://localhost:8082/api/v1/user/profile
```

### Port Mappings

| Service | Host port → container port |
|---------|---------------------------|
| Nginx gateway | 8082 → 80 |
| user-service | 8001 → 8001 |
| booking-service | 8002 → 8002 |
| admin-service | 8003 → 8003 |
| search-service | 8004 → 8004 |
| line-service | 8005 → 8005 |
| order-service | 8006 → 8006 |
| flight-service | 8007 → 8007 |
| hotel-service | 8008 → 8008 |
| payment-service | 8009 → 8009 |
| MySQL | 3308 → 3306 |
| Redis | 6381 → 6379 |
| OpenSearch | 9201 → 9200 |

> The data-source port mappings are a local temporary arrangement (host 3306/6379/9200 are occupied); see `../docs/integration-report.md`.

### Run Tests

```bash
cd e-cat
cargo test -p ecat --bins                        # business services
cargo test --workspace                          # full workspace
```

### Scripts

| Script | Purpose |
|--------|---------|
| `scripts/opensearch_init.sh` | Idempotently create OpenSearch indexes (cjk analyzer) |
| `scripts/loadtest.sh` | Load testing |
| `scripts/cdn_setup.sh` / `cdn_upload.sh` | CDN setup and asset upload (`--dry-run` by default) |
| `scripts/release.sh` | Release workflow helper |

### Release Workflow

The project version (currently v1.0.0, semver) is **independent** of the e-cat framework version (currently 3.0.3).

1. Add a version section at the top of `CHANGELOG.md`, format `## [x.y.z] — YYYY-MM-DD`, describing the changes
2. Tag it: `git tag -a vX.Y.Z -m "vX.Y.Z"`, then `git push origin vX.Y.Z`
3. Create the release: `gh release create vX.Y.Z --title vX.Y.Z --notes-file <section>`, body from the CHANGELOG section; the newest release is automatically marked Latest

Incremental principle: only create missing tags/releases; skip existing ones.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `mysql://travel:pass@localhost:3306/travel` | MySQL connection string |
| `REDIS_URL` | `redis://localhost:6379` | Redis connection string |
| `JWT_SECRET` | dev placeholder secret | JWT signing key, must be ≥32 bytes; falls back to the placeholder with a warning when unset or too short — **must be configured in production** |

## Architecture (e-cat framework)

```
┌──────────────────────────────────────────────────────────────┐
│                         ecat-cli                             │
│        (new │ proto │ run --watch │ build │ upgrade)         │
├──────────────────────────────────────────────────────────────┤
│                     ecat (App Lifecycle)                     │
│      AppBuilder → App { name, servers, hooks, ... }         │
├────────────────────┬────────────────────┬────────────────────┤
│     transport      │    middleware      │     registry       │
│     ─────────      │    ──────────      │     ────────       │
│     HTTP (axum)    │    RecoveryLayer   │     memory         │
│     gRPC (tonic)   │    TracingLayer    │     consul         │
│     encoding       │    LoggingLayer    │                    │
│                    │    TimeoutLayer    │                    │
│                    │    RateLimitLayer  │                    │
│                    │    SecurityLayer   │                    │
│                    │    CircuitBreaker  │                    │
│                    │    Auth (JWT/API)  │                    │
├────────────────────┼────────────────────┼────────────────────┤
│     config         │     errors         │     metadata       │
│     file / env     │     ErrorCode      │     key-value      │
│     remote source  │     Error          │     HTTP/gRPC      │
├────────────────────┴────────────────────┴────────────────────┤
│                         data layer                            │
│     rdbms:   SQLite / PostgreSQL / MySQL / TiDB              │
│     cache:   Redis ✓ / Memcached (in-memory)                 │
│     search:  OpenSearch / Elasticsearch                      │
│     olap / graph / tsdb / document / storage: 11 more        │
├──────────────────────────────────────────────────────────────┤
│                       ecat-protos                             │
│     (shared .proto definitions: errors, metadata, ...)       │
└──────────────────────────────────────────────────────────────┘
```

### Request Flow

```
Client Request
  │
  ├─ HTTP 0.0.0.0:8000 ──→ axum::Router ──┐
  │                                        │
  └─ gRPC 0.0.0.0:9000 ──→ tonic::Server ─┤
                                           │
                                   ┌───────┴───────┐
                                   │   Middleware   │
                                   │ 1. Recovery    │  catch panics
                                   │ 2. Tracing     │  inject trace_id
                                   │ 3. Logging     │  request logs
                                   │ 4. Security    │  attack detection
                                   │ 5. CircuitBrk  │  circuit breaking
                                   │ 6. Auth        │  authn/authz
                                   └───────┬───────┘
                                           │
                                   ┌───────┴───────┐
                                   │    Handler     │  business logic
                                   │ (tower::Service)│
                                   └───────┬───────┘
                                           │
                                   ┌───────┴───────┐
                                   │   Response     │  JSON/Protobuf encoding
                                   └───────────────┘
```

### Middleware Chains in This Project

Business routes mount the full middleware chain (order: outer → inner):

- **user-service**: `Tracing → CircuitBreaker → Security → RateLimit(Redis) → [profile only] Auth(JWT)`
- **booking-service**: `Tracing → CircuitBreaker → Security → RateLimit` (dates is a public endpoint, no JWT)

Business route versions live in the URL prefix `/api/v1/`; paths without the version prefix 404.

**Rate limiting**: 100 req/60s per service, Redis distributed fixed window; unauthenticated requests also count (preventing resource exhaustion by brute force); returns 429 when exceeded.

## e-cat Framework at a Glance

### Tech Stack

| Component | Choice | Component | Choice |
|-----------|--------|-----------|--------|
| Runtime | **tokio** | RDBMS | **sqlx** |
| HTTP | **axum** | Redis | **redis-rs** |
| gRPC | **tonic** | JWT | **jsonwebtoken** |
| Protobuf | **prost + tonic-build** | HTTP Client | **reqwest** |
| Middleware | **tower::Service / Layer** | CLI | **clap** |
| Logging/Tracing | **tracing + trace_id** | Metrics | **prometheus** |

### Kratos Concept Mapping

| Kratos (Go) | e-cat (Rust) | Notes |
|-------------|-------------|-------|
| `kratos.New()` | `App::builder()` | Builder pattern |
| `http.Handler` | `tower::Service` | Standard Rust ecosystem trait |
| `http.Server` | `axum::Router` | Mainstream HTTP framework |
| `grpc.Server` | `tonic::transport::Server` | Most mature gRPC impl |
| `proto generate` | `prost + tonic-build` | Standard protobuf codegen |
| `registry.Discovery` | `Registry` trait | Pluggable discovery |
| `config.Source` | `ConfigSource` trait | Multi-source config loading |

### Data Backends

All 18 data backends share unified trait abstractions (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`) and provide `XxxConfig` + `from_config()` for loading connection info from JSON/YAML: RDBMS (SQLite/PG/MySQL/TiDB), cache (Redis/Memcached), search (OpenSearch/Elasticsearch), OLAP (ClickHouse), graph (Neo4j/NebulaGraph/ArangoDB), TSDB (InfluxDB/IoTDB/QuestDB/TDengine), document (MongoDB), object storage (S3/MinIO).

### Aggregation Crate (ecat)

`ecat` provides feature-gated re-export entry points: `use ecat::transport_http::HttpServer;` (feature "http"), `use ecat::auth::JwtAuthLayer;` (feature "auth"), etc. Default features = `http+grpc`; full list: `http` `grpc` `middleware` `auth` `client` `events` `metrics` `tracing` `circuit-breaker` `consul` `remote` `redis`.

### Error Handling

`ecat-errors` provides `ErrorCode` + `Error` with compile-time HTTP status mapping; error responses are encoded as JSON by the middleware, carrying `code` / `reason` / `message`:

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

### Implementation Progress & Known Limitations

- Framework Phases 1–16 all complete (see [CHANGELOG](CHANGELOG.md))
- Known limitations: GraphQL rejects aliases/fragments/multiple top-level fields; OAuth2 introspection cache whitelists claims by default; Kafka defaults to `auto_commit=false` (re-reads from the partition end on restart)

## Documentation

- [API Reference](../docs/api.md)
- [Project Planning](../docs/travel-project-planning.md)

## Support

Your support is welcome!

| WeChat Pay | Alipay |
|:---:|:---:|
| <img src="../docs/weixinpay.png" width="130" height="130" alt="WeChat Pay"> | <img src="../docs/alipay.png" width="130" height="130" alt="Alipay"> |

### Global Transfer (Bank Wire)

| Field | Value |
|-------|-------|
| Beneficiary Name | WANG KEXUN |
| Account Number | 881015918251 |
| Bank | ZA Bank Limited |
| SWIFT Code | AABLHKHHXXX |
| Bank Code | 387 |
| Bank Address | Core F, Cyberport 3, 100 Cyberport Road, Hong Kong |

> **Cross-border remittance agent bank (if required)**: this is the agent (intermediary) bank information, NOT the receiving bank. Ask your remitting bank whether it is required.
>
> - For HKD, CNY and USD remittances: **Citibank N.A. Hong Kong** (SWIFT: `CITIHKHXXXX`, Bank Code: 006, Branch: Hong Kong Branch, Branch Code: 391, Address: Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong)
> - For other currencies: **THE BANK OF NEW YORK MELLON** (SWIFT: `IRVTUS3NXXX`, Address: 240 GREENWICH STREET, NEW YORK, United States)

## License

Apache-2.0
