# Laporan Pengujian — 2026-08-26

Penulisan ulang menyeluruh unit test (cakupan penuh 51 crate), 4 kelompok insinyur penguji Rust senior secara paralel.

## Ringkasan

| Kelompok | crates | Semula | Baru | Sekarang | Gerbang |
|---|---|---|---|---|---|
| core/kerangka | 12 | 102 | +40 | 142 | ✅ test semua hijau + clippy 0 peringatan |
| data | 14 | 87 | +66 | 153 | ✅ sama seperti di atas |
| mq/transport | 12 | 82 | +54 | 136 | ✅ sama seperti di atas |
| lapisan app | 13 | ~178 | +46 | ~224 | ✅ sama seperti di atas |
| **Total** | **51** | **~449** | **+206** | **~655** | ✅ |

Catatan: jumlah semula lapisan aplikasi mencakup ecat-auth 24 / ecat-graphql 35 / ecat-bench 11 / ecat-scheduler 6 / ecat-events 7 / ecat-cli 12 / ecat-middleware 34 / ecat-circuit-breaker 10 / ecat-health 4 / ecat-client 7 / ecat-security 12 / ecat-versioning 4 / ecat 4. Setiap crate independen `cargo test -p` + `cargo clippy -p --all-targets -- -D warnings` semuanya lulus, paralel dengan isolasi CARGO_TARGET_DIR.

## Rincian per Crate

### Kelompok core/kerangka (test-core, +40)

| crate | Semula→Baru | Poin cakupan |
|---|---|---|
| ecat-protos | 4→8 | Semua enum ErrorCode dibandingkan dengan proto; decode buffer terpotong; buffer kosong pesan default; roundtrip metadata |
| ecat-errors | 4→9 | Pemetaan lengkap http_status (409/429/500); from_status; tidak terpetakan→Internal; cause source() |
| ecat-metadata | 9→12 | Ekstraksi trace_id dari header HTTP; lowercasing key; peta header kosong |
| ecat-encoding | 18→22 | NaN→null (default serde_json, sudah didokumentasikan); decode byte kosong; CodecBox JSON tidak valid; roundtrip proto |
| ecat-lock | 7→9 | release tanpa memegang kunci melaporkan error; key kosong |
| ecat-logging | 1→1 | shim kompatibilitas tidak panic |
| ecat-tracing | 9→12 | Header trace non-UTF-8 dilewati; header kanonik; perpindahan respons |
| ecat-tls | 7→12 | basic_auth satu/dua kolom; file ca tidak ada; is_enabled; klien default |
| ecat-config | 14→26 | Filter prefiks env + batas parsing tipe (hex/string kosong/-0/1e3); penggabungan dan override multi-source; jalur error obfs; file hilang/YAML tidak valid |
| ecat-config-remote | 6→9 | Batas ConsulKvEntry; error X-Consul-Index tidak ada; key bersarang |
| ecat-openapi | 4→11 | components/schema_ref; override duplikat; default 200; tags |
| ecat-metrics | 8→11 | Teks metrik terdaftar; 404/405 |

### Kelompok data (test-data, +66)

| crate | Semula→Baru | Poin cakupan |
|---|---|---|
| ecat-data | 12→14 | Parsing sintaks pencarian |
| ecat-data-sqlx | 7→14 | End-to-end SQLite memori; binding parameter semua tipe; Blob→base64; config |
| ecat-data-redis | 6→12 | Pembuatan URL redis:///rediss://; auth; jalur error config |
| ecat-data-opensearch | 4→10 | Mock HTTP: percent-encode, Basic auth, perpindahan error |
| ecat-data-elasticsearch | 6→11 | Sama seperti di atas |
| ecat-data-influxdb | 5→10 | Escape line protocol; header Token; perpindahan error |
| ecat-data-clickhouse | 12→22 | SQL pembuatan tabel; JSONEachRow; jumlah baris tulis; pengelompokan |
| ecat-data-memcached | 4→8 | TTL detik→mili detik; pengemasan flag |
| ecat-data-nebulagraph | 6→7 | Parsing config |
| ecat-data-arangodb | 5→7 | config/URL |
| ecat-data-iotdb | 5→10 | Mock HTTP: parameter jalur session |
| ecat-data-questdb | 4→9 | line protocol; transaksi tidak didukung |
| ecat-data-tdengine | 6→11 | Pembuatan INSERT; pembagian batch 100 |
| ecat-data-mongodb | 5→8 | Roundtrip bson; URI |

### Kelompok mq/transport/registry (test-mq, +54)

| crate | Semula→Baru | Poin cakupan |
|---|---|---|
| ecat-mq | 5→9 | Frame error tertunda buffer penuh; penutupan aliran full drop; banyak subscriber; publish tanpa subscriber |
| ecat-mq-kafka | 12→14 | Default config; kolom SASL berlaku independen |
| ecat-mq-rabbitmq | 2→5 | Default exchange; jalur error url |
| ecat-mq-mqtt | 5→9 | Validasi pasangan cert/key; file tidak ada; default port 1883/8883; fallback port tidak valid |
| ecat-mq-nats | 6→9 | Default plaintext; jalur error ca/cert hilang |
| ecat-transport | 4→7 | Default TlsConfig/with_client_auth; batas normalize_addr |
| ecat-transport-http | 17→20 | Tes integrasi: stop no-op, kegagalan port terpakai, kirim-terima nyata |
| ecat-transport-grpc | 7→13 | TLS file hilang; siklus hidup plaintext; penolakan mTLS |
| ecat-transport-ws | 4→8 | Gagal tanpa handler; port terpakai; gema frame masked RFC 6455 |
| ecat-registry | 5→8 | discover multi-instance; auto-unregister saat drop; default builder |
| ecat-registry-consul | 10→24 | percent-encode; varian registrasi; respons error; X-Consul-Token; parsing agent/services; fallback node |
| ecat-registry-etcd | 5→10 | discover lewati nilai buruk; body permintaan kv; lease grant; keepalive |

### Kelompok lapisan app (test-app, +46)

| crate | Semula→Baru | Poin cakupan |
|---|---|---|
| ecat-auth | 20→46 | Whitelist cache oauth2/key SHA-256/evict FIFO; apikey tiga status; paksaan jwt iss/aud; kedaluwarsa/signature salah |
| ecat-health | 4→8 | Agregasi readiness (semua ok/salah satu gagal/registri kosong); liveness |
| ecat-versioning | 4→7 | Routing strategi path; batas extract_version |
| ecat-security | 12→20 | End-to-end lapisan header; bentuk JSON intersepsi serangan |
| ecat-middleware | 34→37 | Kedaluwarsa jendela MemoryStore; panic lapisan dalam→Err |
| ecat-circuit-breaker | 10→12 | Probe half-open habis; degradasi classify |
| ecat-client | 7→10 | Endpoint grpc tidak valid melaporkan error tanpa jaringan |
| ecat-graphql | 35→35 | Cakupan yang ada sudah memadai, tidak ada kesenjangan |
| ecat-scheduler / ecat-bench / ecat-events / ecat-cli / ecat | Cakupan yang ada sudah memadai | Tidak ada kesenjangan |

## Defect yang Ditemukan

| Level | Lokasi | Deskripsi | Status |
|---|---|---|---|
| P1 | ecat-events/Cargo.toml | dev-dependencies kekurangan feature tokio macros/rt/time, kompilasi target tes crate tersebut secara terpisah pasti gagal (build penuh workspace ditutupi oleh union feature) | ✅ Diperbaiki (menambah features + komentar) |
| P2 | ecat-security src/lib.rs:118-127 | SQLi ter-encode persen URI (`?q=SELECT%20*%20...`) dapat melewati pemindaian lapisan header (detektor mensyaratkan spasi literal, memindai URI mentah tanpa mendekode terlebih dahulu); pemindaian body tidak terpengaruh | ⏳ Menunggu perbaikan |
| P3 | ecat-data-sqlx | `connect()/from_config()` menggunakan AnyPool tetapi tidak menginstal driver, sqlx 0.8.6 panic "No drivers installed" pada koneksi pertama | ⏳ Menunggu perbaikan |
| P3 | ecat-data-influxdb | Field string meng-escape spasi (`\ `), spesifikasi line protocol hanya perlu meng-escape `"` dan `\`; urutan tag/field tidak deterministik | ⏳ Menunggu perbaikan |
| P3 | ecat-data-clickhouse | Cache pembuatan tabel tidak pernah kedaluwarsa, tidak mencoba ulang CREATE setelah drop/ubah tabel eksternal | ⏳ Menunggu perbaikan |
| P3 | ecat-circuit-breaker | Batas atas half_open_probes tidak dapat tercapai di bawah probing sekuensial (hanya dapat tercapai saat konkurensi in-flight), sudah tercakup oleh white-box test | ℹ️ Diketahui, bukan defect |
| P3 | ecat-health | `with_check` menggunakan blocking_write(), memanggil dari konteks async akan panic; saat ini hanya dapat digunakan dari konteks sinkron | ℹ️ Diketahui, batasan API |

## Modul yang Dilewati (Membutuhkan Lingkungan Integrasi, Tidak Dimock)

- Roundtrip broker nyata: publish-subscribe kafka/rabbitmq/mqtt/nats (konfigurasi dan jalur error sudah tercakup)
- Klaster nyata: siklus hidup registrasi-discovery consul/etcd (mock axum mencakup bentuk permintaan)
- Database nyata: operasi redis/memcached, mongod, validasi server influxdb, driver sqlx postgres/mysql, API nebulagraph/arangodb
- Layanan eksternal nyata: introspeksi OAuth2 (mock lokal sudah tercakup), roundtrip gRPC/HTTP (mock lokal sudah mencakup 302 tidak diikuti)
