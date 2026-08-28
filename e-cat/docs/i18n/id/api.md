<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Referensi API Ecat

Halaman ini merangkum permukaan antarmuka (API) framework Ecat: konvensi port, endpoint bawaan, format error, dan antarmuka ekstensi. Routing bisnis didaftarkan oleh masing-masing layanan.

## Konvensi Port

| Protokol | Alamat listen | Keterangan |
|------|----------|------|
| HTTP | `0.0.0.0:8000` | Routing axum, port contoh default |
| gRPC | `0.0.0.0:9000` | tonic Server, port contoh default |

## Endpoint Bawaan

Endpoint berikut disediakan oleh crate ekosistem, dipasang bersama layanan:

| Endpoint | Sumber | Keterangan |
|------|------|------|
| `/health` | ecat-health | Pemeriksaan kelangsungan hidup (mengembalikan nama layanan, versi, waktu mulai) |
| `/ready` | ecat-health | Pemeriksaan kesiapan (mengembalikan 200 setelah dependensi siap) |
| `/metrics` | ecat-metrics | Ekspos metrik Prometheus (`ecat_http_requests_total` / `ecat_http_request_duration_seconds`) |
| `/{service}/{method}` | Routing pengguna | Contoh: `/helloworld/ecat` |

> Untuk skenario kardinalitas tinggi seperti path endpoint metrik yang berisi ID, gunakan `MetricsLayer::new().with_path_fn(...)` untuk normalisasi, hindari ledakan kardinalitas metrik.

## Alur Pemrosesan Permintaan

```
客户端请求
  ├─ HTTP :8000 ──→ axum::Router ─┐
  └─ gRPC :9000 ──→ tonic::Server ─┤
                              ┌─────┴──────┐
                              │ Middleware │  Recovery→Tracing→Logging→Auth→Metrics→Security→CircuitBreaker
                              └─────┬──────┘
                                    ▼
                               Handler（tower::Service）
                                    ▼
                               Response（JSON/Protobuf 编码）
```

## Format Error

`ecat-errors` menyediakan `ErrorCode` + `Error`, memetakan status HTTP pada waktu kompilasi:

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

Respons error dienkode oleh middleware menjadi JSON (atau Protobuf), membawa code / reason / message.

## Antarmuka Ekstensi

| Kemampuan | Crate | Antarmuka |
|------|-------|------|
| GraphQL | ecat-graphql | Endpoint `/graphql`; mendukung parameter kolom dan selection bersarang, tidak mendukung alias, fragment, dan beberapa kolom tingkat atas |
| OpenAPI | ecat-openapi | Membuat spec OpenAPI dari routing |
| WebSocket | ecat-transport-ws | Transport WS yang di-upgrade |
| Routing versi API | ecat-versioning | Routing versi dengan prefiks `/v1/...` |
| Autentikasi | ecat-auth | Middleware JWT / API Key; kunci JWT harus ≥32 byte, dapat dirantai `required_issuer`/`required_audience` |
| Klien gRPC | ecat-transport-grpc | Terintegrasi dengan service discovery dan load balancing |

## Komunikasi Antar-layanan

- `HttpClient` (ecat-client): terintegrasi dengan service discovery dan load balancing, perlindungan circuit breaker dengan CircuitBreaker
- `GrpcClient` (ecat-transport-grpc): sama seperti di atas, protokol gRPC
- Middleware dikombinasikan secara terpadu dengan `tower::ServiceBuilder` (Recovery / Tracing / Logging / Timeout / RateLimit / Security / CircuitBreaker / Metrics / Retry / Validate / CORS)

## Antarmuka Backend Data

Semua backend data (`ecat-data-*`) diabstraksikan melalui trait terpadu (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`); backend bergaya REST (Neo4j / NebulaGraph / ArangoDB / InfluxDB / IoTDB / QuestDB / TDengine / OpenSearch / Elasticsearch / S3) mengakses antarmuka HTTP terkait berdasarkan `base_url`. Konfigurasi koneksi lihat [Tutorial Konfigurasi Database](database-config-tutorial.md).
