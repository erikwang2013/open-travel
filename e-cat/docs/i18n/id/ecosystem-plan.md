# Rencana Ekosistem e-cat

**Versi:** 2.1.7  
**Tanggal:** 2026-08-01  
**Status:** Semua selesai · 47 crates

| Bidang | Cakupan | Status |
|------|--------|------|
| Transport | HTTP (axum), gRPC (tonic), WebSocket | ✅ |
| Encoding | JSON, Protobuf | ✅ |
| Middleware | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth (JWT/API Key/OAuth2) | ✅ |
| Konfigurasi | env, file (JSON/YAML), Consul KV jarak jauh, enkripsi | ✅ |
| Registri | memory, Consul, etcd | ✅ |
| Keamanan | Deteksi serangan, JWT, API Key, OAuth2, TlsConfig | ✅ |
| Data | RDBMS (sqlx), Redis, Memcached, OpenSearch, Elasticsearch, ClickHouse, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB | ✅ |
| Observabilitas | tracing, Prometheus, Health, pelacakan terdistribusi | ✅ |
| Komunikasi | HTTP/gRPC Client, MessageQueue (InMemory/Kafka), EventBus | ✅ |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | ✅ |
| Alat API | OpenAPI, Versioning, GraphQL | ✅ |

## Sisa Kesenjangan (3 Optimasi Kecil)

1. **mTLS terhubung ke transport** — TlsConfig sudah ada, belum terhubung ke HttpServer/GrpcServer
2. **Backend rate limit Redis** — RateLimitLayer hanya memori, multi-instance perlu berbagi
3. **Template CI GitLab** — saat ini hanya GitHub Actions

## Evolusi Versi

```
v1.0.x  Kerangka inti (18 crates)                       ✅
v2.0.x  Ekosistem tahap 1~3 (+13 crates)                ✅
v2.1.x  Penguatan komunikasi & keamanan + kelengkapan backend data + pengalaman operasional   ✅ (saat ini)
```

## Tidak Dimasukkan ke Ekosistem

| Kebutuhan | Solusi | Alasan |
|------|------|------|
| API Gateway | Kong / Envoy | Tidak tergantung bahasa |
| Service mesh | Linkerd | Rust belum ada solusi matang |
| Orkestrasi kontainer | Kubernetes | Standar industri |
| Pengumpulan log | Vector | Native Rust |
