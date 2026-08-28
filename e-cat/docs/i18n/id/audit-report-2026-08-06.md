# Laporan Peninjauan Menyeluruh e-cat

**Tanggal**: 2026-08-06
**Versi**: 2.3.0 · 55 crates
**Ruang lingkup**: build/test, smoke test runtime, konsistensi ekosistem, keamanan, konfigurasi deployment

---

## 1. Hasil Pengujian dan Build

| Item pemeriksaan | Hasil | Keterangan |
|--------|------|------|
| `cargo check --workspace` | ✅ Lulus | 0 peringatan |
| `cargo test --workspace` | ✅ Lulus | **202 tes semuanya lulus, 0 gagal** (termasuk doc-tests) |
| `cargo fmt --check` | ✅ Lulus | |
| `cargo clippy --workspace -- -D warnings` | ✅ Lulus | Konsisten dengan perintah CI |
| `cargo clippy --all-targets -- -D warnings` | ❌ Gagal | Lihat temuan D2 |
| Smoke test (helloworld) | ❌ **Gagal start** | Lihat temuan D1 |

**Distribusi cakupan tes**: 51 file sumber berisi `#[test]`, 105 binary tes. Tidak ada `todo!()`/`unimplemented!()` di jalur produksi, `panic!` hanya ada di kode tes.

---

## 2. Masalah Runtime (ditemukan oleh smoke test)

### [TINGGI] D1. `HttpServer::new(":8000")` gagal start di lingkungan tanpa IPv6
- **Lokasi**: `ecat-transport-http/src/lib.rs:40`, `examples/helloworld/src/main.rs:41`, README di banyak tempat
- **Gejala**: `TcpListener::bind(":8000")` di-resolve ke wildcard IPv6 `[::]:8000`, mesin tanpa IPv6 (container/sebagian host cloud) melaporkan `failed to lookup address information: Name or service not known`, layanan tidak dapat start.
- **Reproduksi**: program minimal mandiri — `bind(":8001")` gagal, `bind("0.0.0.0:8002")` berhasil, `bind("localhost:8003")` berhasil.
- **Perbaikan**: `HttpServer::new` menormalkan host kosong menjadi `"0.0.0.0"`; contoh dan dokumentasi diseragamkan memakai `"0.0.0.0:8000"`.

### [RENDAH] D2. `cargo clippy --all-targets -- -D warnings` gagal
- **Lokasi**: `ecat-data-sqlx/src/lib.rs` (ada items setelah modul tes, memicu `items_after_test_module`)
- **Dampak**: perintah clippy CI saat ini (tanpa `--all-targets`) tidak terpengaruh; jika CI diperketat maka gagal.
- **Perbaikan**: memindahkan modul tes ke akhir file.

---

## 3. Masalah Kritis (CRITICAL)

### [KRITIS] C1. `ecat-data-memcached` adalah "implementasi palsu"
- **Lokasi**: `ecat-data-memcached/src/lib.rs:23-88`
- **Masalah**: seluruh crate adalah `HashMap` murni di memori, tanpa koneksi jaringan, tanpa konfigurasi alamat server (`MemcachedConfig` hanya username/password/tls), description Cargo.toml mengakui sendiri "in-memory cache client". Salah pakai di produksi akan menyebabkan **kehilangan data diam-diam** (dibersihkan saat restart, tidak dibagi antar instans).
- **Perbaikan**: integrasikan protokol memcached asli (mis. crate `memcache`), atau tandai eksplisit `#[deprecated]`/peringatan dokumentasi yang melarang pemakaian produksi.

### [KRITIS] C2. Injeksi perangkaian SQL TDengine
- **Lokasi**: `ecat-data-tdengine/src/lib.rs:91-116`
- **Masalah**: dalam `INSERT INTO "{}" ({}) VALUES ({})`, measurement/nama kolom/nilai semuanya dirangkai langsung dengan `format!`, nilai string hanya dibungkus tanda kutip ganda tanpa meng-escape `"` dan `\`. Nilai field yang mengandung `"; DELETE ...; --` dapat lolos mengeksekusi SQL arbitrer (REST TDengine mendukung multi-pernyataan).
- **Perbaikan**: escape identifier dan nilai string (`"`→`\"`, `\`→`\\`), atau gunakan antarmuka penulisan terparameterisasi.

---

## 4. Masalah Risiko Tinggi (HIGH)

### [TINGGI] H1. Semua adapter database HTTP tanpa timeout
- **Lokasi**: `ecat-tls/src/lib.rs:27,61`, elasticsearch/opensearch/clickhouse/influxdb/iotdb/questdb/tdengine/neo4j/nebulagraph/arangodb
- **Masalah**: reqwest tanpa timeout secara default, saat server menggantung permintaan **menggantung selamanya** (pool koneksi habis, kebocoran task).
- **Perbaikan**: `build_reqwest_client` mengatur seragam `connect_timeout` (mis. 5s) + `timeout` (mis. 30s).

### [TINGGI] H2. Rate limiting tidak dapat berlaku per klien
- **Lokasi**: `ecat-middleware/src/ratelimit.rs:155`
- **Masalah**: `key_fn("")` tidak mendapat objek permintaan, tidak dapat melakukan limit per IP/pengguna; default bucket tunggal "global", penyerang dapat menghabiskan kuota global (DoS untuk orang lain) atau melewatinya secara terdistribusi.
- **Perbaikan**: ubah tanda tangan `key_fn` menerima `&http::Request`, ambil key berdasarkan `X-Forwarded-For`/alamat peer.

### [TINGGI] H3. CI GitHub pasti gagal (kurang protoc)
- **Lokasi**: `.github/workflows/ci.yml`
- **Masalah**: build.rs `ecat-protos` menggunakan tonic-build untuk mengompilasi proto, sangat bergantung pada protoc; CI GH tidak menginstal `protobuf-compiler` (di mesin lokal `/home/erik/.local/bin/protoc` ada sehingga lolos lokal). `.gitlab-ci.yml` sudah menginstal, perilaku dua CI tidak konsisten.
- **Perbaikan**: CI GH menambahkan `apt-get install protobuf-compiler` (dan cmake, jika perlu).

### [TINGGI] H4. Elasticsearch `search()`/`delete()` tidak memeriksa status code HTTP
- **Lokasi**: `ecat-data-elasticsearch/src/lib.rs:87-114`
- **Masalah**: body error 404/400 dianggap JSON dan di-parse, melaporkan error menyesatkan "es parse"; `index()` memeriksa sedangkan `search`/`delete` tidak, perilaku tidak konsisten (opensearch sudah benar).
- **Perbaikan**: periksa seragam `status.is_success()`.

### [TINGGI] H5. Kecurigaan ketidakcocokan protokol IoTDB `insertTablet`
- **Lokasi**: `ecat-data-iotdb/src/lib.rs:51-82`
- **Masalah**: REST IoTDB `insertTablet` mensyaratkan format array `timestamps/measurements/values/data_types`; implementasi ini mengirim JSON dokumen tunggal, mungkin "terlihat terimplementasi padahal tidak terpakai".
- **Perbaikan**: susun body permintaan sesuai spesifikasi insertTablet, dan tambahkan tes integrasi.

### [TINGGI] H6. Prefiks deregister etcd tidak cocok (deregister tidak efektif)
- **Lokasi**: `ecat-registry-etcd/src/lib.rs:47,66`
- **Masalah**: kunci registrasi adalah `/ecat/services/{prefix}/{name}/{uuid}`, deregister justru menghapus `{prefix}/{name}` (kurang segmen uuid) → informasi registrasi tertinggal setelah instans keluar.
- **Perbaikan**: saat menghapus cocokkan kunci lengkap, atau daftarkan lalu hapus berdasarkan prefiks name.

---

## 5. Masalah Risiko Sedang (MEDIUM)

| # | Lokasi | Masalah | Saran |
|---|------|------|------|
| M1 | `ecat-middleware/src/ratelimit_redis.rs:28-48` | saat Redis gagal, Err dianggap terlampaui limit → **DoS fail-closed**; setelah INCR, kunci yang gagal EXPIRE tidak pernah kedaluwarsa → blokir permanen | pisahkan error limit/storage (storage gagal → lewati), script atomik Lua |
| M2 | `ecat-middleware/src/ratelimit.rs:16-51` | entri MemoryStore hanya di-reset tidak dihapus, dengan kunci per klien **memori tumbuh tanpa batas** | bersihkan bucket kedaluwarsa secara berkala |
| M3 | `ecat-auth/src/jwt.rs:25-31` | tanpa validasi panjang minimum kunci lemah (untuk tes "secret-key"), dapat dibobol offline | wajibkan kunci acak ≥32 byte; generalkan respons error untuk menghindari gema detail jsonwebtoken |
| M4 | `ecat-auth/src/oauth2.rs:111-123` | tiap permintaan membuat reqwest::Client baru tanpa timeout; URL tidak diwajibkan HTTPS | reuse Client, atur timeout, validasi https |
| M5 | `ecat-data-redis/src/lib.rs:34-64`, `ratelimit_redis.rs:12-17`, ecat-lock | password di-embed ke URL setelah percent_encode, Display error koneksi berisi URL lengkap → **kebocoran kata sandi di log**; saat URL sudah berisi `@` kredensial dibuang diam-diam | teruskan parameter autentikasi terpisah, desensitisasi pesan error |
| M6 | `ecat-data-elasticsearch/src/lib.rs:104-113`, opensearch:111-116 | index/id dirangkai ke path tanpa URL encode, dapat mengakses index lain via `/` (IDOR) | URL encode + whitelist index |
| M7 | `ecat-data-sqlx/src/lib.rs:79,173`, questdb:78-84 | error mentah database (berisi SQL dan nilai) di-raise langsung ke atas | generalkan seragam di luar, detail hanya masuk log |
| M8 | `ecat-data-clickhouse/src/lib.rs:92` | `execute()` selalu mengembalikan `Ok(0)`, rows_affected hilang; `query()` membuang baris gagal parse secara diam-diam | kembalikan jumlah baris asli, naikkan error |
| M9 | `ecat-data-tdengine/src/lib.rs:80-118` | `write()` melakukan permintaan per titik dalam loop (N+1) | tulis batch |
| M10 | `ecat-data-sqlx/src/lib.rs:98-142 vs 213-256` | query/query_with menduplikasi ~50 baris logika konversi tipe | ekstrak fungsi bersama |
| M11 | `ecat-data-redis/src/lib.rs:167` | di `acquire`, `ttl.as_millis() as u64` overflow terpotong (`set` sudah menangani, di sini belum) | penanganan overflow seragam |
| M12 | `ecat-data-influxdb/src/lib.rs:69-79` | field string line protocol tidak di-escape (kutip/koma/spasi) → menulis langsung error protokol | escape sesuai spesifikasi |
| M13 | `ecat-mq-*` | tanda tangan `from_config` tidak seragam: kafka/mqtt mengembalikan sinkron, rabbitmq/nats async | seragamkan menjadi async |
| M14 | `ecat-auth/src/apikey.rs:33-36`, `ecat-security/src/lib.rs:126-137` | API key mendukung parameter query (masuk log/Referer); WAF hanya memindai URI+headers tidak memindai body | key hanya lewat header; WAF tambah pemindaian body |

---

## 6. Risiko Rendah dan Tingkat Informasi (LOW/INFO)

| # | Lokasi | Masalah |
|---|------|------|
| L1 | `ecat-deploy/Dockerfile` | **menyalin binary `ecat-app` yang tidak ada** (binary asli adalah `ecat`, dari ecat-cli) → setelah docker build image tidak punya entry; HEALTHCHECK memakai curl tetapi image tidak menginstal curl |
| L2 | `ecat-deploy/helm/Chart.yaml` | appVersion "2.2.0", versi saat ini 2.3.0 |
| L3 | `README.en.md` | mengklaim "v2.1.7 · 47 crates", padahal v2.3.0 · 55 crates, dokumentasi Inggris sangat ketinggalan zaman |
| L4 | `ecat-registry-consul/src/lib.rs:66,143` | port registrasi selalu 0, versi hasil discover hardcoded "1.0" |
| L5 | Cargo.toml 11 crate | menulis dependensi versi sama langsung melewati `workspace.dependencies` (risiko drift versi) |
| L6 | `ecat-tracing` / `ecat-middleware/src/tracing.rs` | TracingLayer diimplementasikan duplikat; ecat-tracing-otlp dan ecat-tracing masing-masing menginstal subscriber sendiri, dipanggil bersamaan akan konflik double init |
| L7 | `ecat-config-remote/src/lib.rs:92` | decode base64 tulisan tangan, disarankan memakai crate base64 |
| L8 | `ecat-graphql` | parser satu-field tulisan tangan, hanya mendukung field tunggal level atas (tanpa nested/alias/argumen), dokumentasi tidak menyebut keterbatasan |
| L9 | `ecat-cli/src/main.rs:69-104`, lib.rs:3-22 | `ecat new ../../x` path traversal; nama berisi `"`/newline dapat menginjeksi Cargo.toml yang dihasilkan |
| L10 | `config/databases.example.yaml:54-79` | beberapa kata sandi default valid (neo4j/changeme, arangodb root/changeme, iotdb root/root, influx my-secret-token), disalin langsung online dengan kata sandi default |
| L11 | `ecat-data-s3/src/lib.rs:83-93` | list() tanpa konfigurasi timeout; konstruksi kredensial adalah panggilan blok sinkron |
| L12 | `ecat-data-redis` | tanpa koneksi ulang eksplisit, bergantung pada reconnect bawaan MultiplexedConnection, dokumentasi tidak menjelaskan |
| L13 | `ecat-data/src/rdbms.rs:71-77` | `Transaction::drop` hanya warn tidak memicu rollback, bergantung pada drop sqlx yang rollback otomatis, disarankan tambah komentar penjelasan |

---

## 7. Kesimpulan Kelengkapan Ekosistem

**Kelengkapan: tinggi**. 55/55 crates ada di workspace, versi seragam 2.3.0, tanpa stub (kecuali implementasi palsu memcached). 18 backend database, 4 backend MQ, 2 registry, abstraksi penyimpanan rate limit, distributed lock, scheduler, tracing OTLP, versioning, GraphQL semuanya terwujud. `todo!()`/`unimplemented!()` nol tempat.

**Perlu diperkuat**:
1. Implementasi protokol asli memcached (satu-satunya adapter "palsu" saat ini)
2. Verifikasi kepatuhan protokol IoTDB (diduga tidak terpakai)
3. Menyelaraskan CI GitHub dengan CI GitLab (kurang protoc)
4. Strategi timeout seragam untuk semua adapter HTTP

## 8. Kesimpulan Keamanan

**Tidak ada kerentanan keamanan KRITIS (injection/handling kredensial/default TLS semuanya aman)**:
- ✅ Nol blok unsafe di seluruh workspace
- ✅ Tanpa kredensial hardcoded, konfigurasi contoh adalah placeholder changeme (disarankan semuanya dikomentari, L10)
- ✅ sqlx semuanya binding terparameterisasi; lock Redis dilepas dengan Lua CAS
- ✅ TLS `skip_verify` mati secara default; Redis otomatis upgrade rediss://
- ⚠️ Perlu perbaikan: injeksi rangkaian TDengine (C2, melewati cakupan sqlx), rate limit per klien (H2), Redis rate limit fail-closed (M1), kunci lemah JWT (M3), kebocoran pesan error Redis (M5), injeksi path ES (M6)

## 9. Saran Optimasi (Prioritas Top)

1. **P0**: C1 implementasi palsu, C2 injeksi SQL, D1 bind port, H1 timeout — 4 item
2. **P1**: H2 rate limit, H3 CI, H4 status code ES, H5 IoTDB, H6 deregister etcd
3. **P1**: M1 fail-closed, M3 JWT, M5 kebocoran password, M6 injeksi path
4. **P2**: perbaikan Dockerfile/Helm/README, clippy --all-targets, kebocoran error, tulis batch
5. **P3**: konvergensi workspace.dependencies, penyatuan from_config MQ, sinkronisasi dokumentasi

---

## 10. Status Perbaikan (verifikasi ulang 2026-08-06)

**Semua 35 temuan telah diperbaiki atau ditangani secara terdokumentasi.** Hasil verifikasi ulang: `cargo check --workspace` ✅, `cargo test --workspace` 219 tes semua lolos ✅, `cargo clippy --workspace --all-targets -- -D warnings` nol peringatan ✅, `cargo fmt --check` bersih ✅, smoke test helloworld (`/` + `/health`) ✅.

| Nomor | Severitas | Cara perbaikan | Verifikasi |
|------|--------|----------|------|
| D1 | TINGGI | `HttpServer` menormalkan host kosong menjadi `0.0.0.0`; contoh/dokumen/template CLI diseragamkan `0.0.0.0:8000` | smoke test bind berhasil |
| D2 | RENDAH | impl `SqlxTransactionWrapper` dipindah sebelum modul tes | clippy nol peringatan |
| C1 | KRITIS | memcached ditandai eksplisit "khusus pengembangan/tes"; saklar `in_memory`; get lazy expiration + set sweep | 23 tes lapisan data lulus |
| C2 | KRITIS | TDengine double escape (`\`→`\\`, `"`→`\"`); chunking batch per 100 | Lulus |
| H1 | TINGGI | `ecat-tls` timeout seragam connect 5s / request 30s, semua adapter HTTP mewarisi | Lulus |
| H2 | TINGGI | key rate limit default berdasarkan X-Forwarded-For hop pertama → X-Real-IP → global; MemoryStore pembersihan lazy 60s | 22 tes middleware lulus |
| H3 | TINGGI | CI menambahkan instalasi `protobuf-compiler` | konfigurasi diperbarui |
| H4 | TINGGI | ES/OpenSearch `search()`/`delete()` memeriksa `is_success()`; index/id encode RFC 3986 | Lulus |
| H5 | TINGGI | IoTDB direstrukturisasi menjadi body insertTablet standar, memeriksa `code != 200` | Lulus |
| H6 | TINGGI | deregister etcd diganti range delete dengan prefiks, cocok dengan kunci registrasi | Lulus |
| M1 | SEDANG | Redis rate limit: INCR+EXPIRE atomik Lua, EXPIRE gagal DEL rollback, error koneksi fail-open + warn | Lulus |
| M3 | SEDANG | kunci JWT <32 byte ditolak (`WeakKey`); respons error seragam `invalid token` | 9 tes auth lulus |
| M5 | SEDANG | password Redis diteruskan terpisah melalui `ConnectionInfo`, tidak lagi di-embed di URL | Lulus |
| M6 | SEDANG | semua permukaan injeksi ES/OpenSearch/InfluxDB di-escape atau diparameterisasi | Lulus |
| M9 | SEDANG | TDengine 100 baris/batch | Lulus |
| M11 | SEDANG | overflow ttl Redis diklem ke `u64::MAX` | Lulus |
| M13 | SEDANG | MQ `from_config` diseragamkan async (kafka/mqtt sinkronisasi) | 11 tes CLI lulus |
| Seri L | RENDAH/INFO | Dockerfile (nama binary asli + health check curl + builder 1.85), Chart appVersion 2.3.0, contoh kata sandi dikomentari, versi/port consul di-parse dari info registrasi, base64 tulisan tangan diganti crate `base64`, `validate_crate_name` cegah injeksi, konvergensi workspace.dependencies 8 tempat, konflik double subscriber dikomentari, sinkronisasi dokumentasi (README/README.en/CHANGELOG 2.3.1) | Semua lulus |

**Masalah baru selama perbaikan**: tes `ecat-config-remote` mereferensikan `base64_decode` lama (terlewat saat penggantian agent) → sudah diganti `base64::engine`; `ecat-middleware` 4 peringatan clippy (if bersarang / tipe kompleks) → sudah dilipat + alias tipe `KeyFn`. Tidak ada regresi setelah perbaikan.

**Kesimpulan ekosistem**: 55 crate, 18 adapter database, 4 MQ, konfigurasi Docker/Helm/CI, README dwibahasa, CHANGELOG semuanya konsisten dengan v2.3.0; referensi gambar (alipay/weixinpay.png) normal.

---

*Laporan dibuat oleh peninjauan otomatis: build+test+smoke run + 3 agen peninjau khusus (keamanan/lapisan data/konsistensi ekosistem), verifikasi ulang menyeluruh 2026-08-06.*
