# LAPORAN AUDIT E-CAT — r5

**Tanggal**: 2026-08-01  
**Cabang**: main  
**Versi**: 2.1.7  
**Jumlah crate**: 47 (anggota workspace)
**Status**: ✅ Semua masalah yang dapat diperbaiki telah diselesaikan + backend data mendukung penuh file konfigurasi

---

## 0. Catatan Perbaikan (2026-08-01)

| # | Masalah | File | Perbaikan |
|---|------|------|------|
| 1 | unused import `axum::routing::get` | `ecat-versioning/src/lib.rs:3` | Menghapus import tingkat atas, memindahkan ke dalam `#[cfg(test)]` |
| 2 | unused variable `version` | `ecat-versioning/src/lib.rs:61` | Diubah menjadi `_version` |
| 3 | dead code `extract_version` | `ecat-versioning/src/lib.rs:68` | Diubah menjadi `pub fn` |
| 4 | `useless_format!` | `ecat-versioning/src/lib.rs:62` | Diubah langsung menjadi `"/api"` |
| 5 | `unnecessary_to_owned` | `ecat-data-questdb/src/lib.rs:39` | `"true".to_string()` → `"true"` |
| 6 | Pesan error ditelan | `ecat-data-questdb/src/lib.rs:30` | `unwrap_or_default()` → `unwrap_or_else(...)` |
| 7 | `derivable_impls` | `ecat-client/src/lib.rs:249` | `GrpcClientBuilder` diganti `#[derive(Default)]` |
| 8 | `manual_is_multiple_of` | `ecat-config/src/encrypted.rs:60` | `s.len() % 2 != 0` → `!s.len().is_multiple_of(2)` |
| 9 | `collapsible_if` | `ecat-registry-etcd/src/lib.rs:92` | Menggabungkan `if let` bersarang |
| 10 | `collapsible_if` | `ecat-data-clickhouse/src/lib.rs:56` | Menggabungkan `if let` bersarang |
| 11 | `type_complexity` | `ecat-data-memcached/src/lib.rs:9` | Menambahkan alias `type CacheEntry` |

**Hasil akhir**: `cargo build` nol warning, `cargo clippy --all-targets` nol warning, `cargo test` semuanya lulus (0 gagal).

### 12 — Backend data mendukung penuh file konfigurasi (Cargo + lib.rs)

Untuk 12 crate backend data menambahkan struct `Config` (`#[derive(Deserialize)]`) dan konstruktor `from_config()`, mendukung pemuatan info koneksi dari file konfigurasi JSON/YAML, tanpa hardcode.

| Crate | Struct Config | Kolom |
|-------|--------------|------|
| `ecat-data-redis` | `RedisConfig` | `url` |
| `ecat-data-sqlx` | `SqlxConfig` | `url` |
| `ecat-data-clickhouse` | `ClickhouseConfig` | `base_url`, `database` (default "default") |
| `ecat-data-questdb` | `QuestdbConfig` | `base_url` |
| `ecat-data-elasticsearch` | `ElasticsearchConfig` | `base_url` |
| `ecat-data-opensearch` | `OpenSearchConfig` | `base_url` |
| `ecat-data-influxdb` | `InfluxConfig` | `base_url`, `org`, `bucket`, `token` |
| `ecat-data-memcached` | `MemcachedConfig` | (kosong — implementasi memori) |
| `ecat-data-neo4j` | `Neo4jConfig` | `base_url`, `username`, `password` |
| `ecat-data-nebulagraph` | `NebulaGraphConfig` | `base_url`, `space` |
| `ecat-data-arangodb` | `ArangoConfig` | `base_url`, `db`, `username`, `password` |
| `ecat-data-iotdb` | `IotdbConfig` | `base_url`, `username`, `password` |

**Contoh penggunaan**:
```rust
// 从 YAML 配置文件加载
let cfg: ClickhouseConfig = serde_json::from_str(r#"{"base_url":"http://localhost:8123"}"#)?;
let client = ClickhouseClient::from_config(cfg);
```

### 13 — Backend HTTP menambahkan dukungan autentikasi opsional (5 crate)

Untuk 5 backend HTTP murni menambahkan kolom opsional `username` / `password` dan konstruktor `with_auth()`. Semuanya `Option<String>` (`#[serde(default)]`), tanpa konfigurasi maka tanpa autentikasi.

| Crate | Kolom Config baru | Konstruktor baru |
|-------|-----------------|-------------|
| `ecat-data-elasticsearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-opensearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-clickhouse` | `username?`, `password?` | `with_auth()` |
| `ecat-data-questdb` | `username?`, `password?` | `with_auth()` |
| `ecat-data-nebulagraph` | `username?`, `password?` | `with_auth()` |

Semua permintaan HTTP secara otomatis menambahkan Basic Auth melalui metode bantu `apply_auth()` (hanya jika keduanya bukan None).

### 14 — Redis / RDBMS / Memcached menambahkan kolom autentikasi opsional (3 crate)

| Crate | Kolom Config baru | Konstruktor baru | Cara autentikasi |
|-------|-----------------|-------------|----------|
| `ecat-data-redis` | `password?` | `connect_with_password()` | Kata sandi tertanam di URL |
| `ecat-data-sqlx` | `username?`, `password?` | `connect_with_auth()` | Autentikasi tertanam di URL |
| `ecat-data-memcached` | `username?`, `password?` | `with_auth()` | Kolom cadangan (implementasi memori) |

Sqlx mencakup empat RDBMS SQLite / PostgreSQL / MySQL / TiDB. Kolom Auth ditanamkan ke URL koneksi melalui `replacen("://", "://user:pass@")`, hanya berlaku saat URL tidak mengandung `@`.

### 15 — Dukungan autentikasi sertifikat TLS + crate ecat-tls (semua 12 backend)

Menambahkan crate `ecat-tls` baru, menyediakan:
- `TlsClientConfig` — konfigurasi TLS opsional (ca_cert, client_cert, client_key, skip_verify)
- `generate_ca()` — pembuatan sertifikat CA self-signed
- `generate_server_cert()` — pembuatan sertifikat server
- `generate_client_cert()` — pembuatan sertifikat klien (mTLS)

Semua 12 Config backend data menambahkan kolom `#[serde(default)] tls: Option<TlsClientConfig>`.

| Jenis backend | Cara TLS |
|----------|----------|
| 9 backend HTTP | `tls.build_reqwest_client()` membangun Client reqwest TLS |
| Redis | Peralihan URL scheme `redis://` → `rediss://` |
| Sqlx | Kolom cadangan (TLS melalui parameter URL `?sslmode=require`) |
| Memcached | Kolom cadangan (cadangan implementasi jaringan) |

---

## 1. Ringkasan

| Item | Status | Detail |
|------|------|------|
| `cargo build` | ✅ Lulus | 3 warning kompiler, 19.85s |
| `cargo test` | ✅ Lulus | ~137 unit test semuanya lulus, 0 gagal, 1 ignored |
| `cargo clippy` | ⚠️ Ada warning | 3 crate total 5 lint warnings |
| `cargo fmt` | ✅ Lulus | Tidak ada masalah format |
| `cargo audit` | ❌ Belum diinstal | Tidak dapat memindai CVE yang diketahui |

---

## 2. Warning Kompiler (Perlu Diperbaiki)

### 2.1 ecat-versioning (3 warning)

**File**: `ecat-versioning/src/lib.rs`

| # | Warning | Baris | Tingkat keparahan |
|---|---------|------|----------|
| 1 | `unused import: axum::routing::get` | 3 | Rendah |
| 2 | `unused variable: version` | 61 | Rendah |
| 3 | `function extract_version is never used` | 68 | Rendah |

**Saran**: Hapus import yang tidak digunakan, ubah `version` menjadi `_version`, ubah `extract_version` menjadi `pub` atau tandai `#[allow(dead_code)]`.

### 2.2 ecat-data-questdb (1 clippy warning)

**File**: `ecat-data-questdb/src/lib.rs:39`

```rust
// 当前:
.query(&[("query", sql), ("count", &"true".to_string())])

// 应改为:
.query(&[("query", sql), ("count", &"true")])
```

### 2.3 ecat-client (1 clippy warning)

**File**: `ecat-client/src/lib.rs:249`

`GrpcClientBuilder` mengimplementasikan `Default` manual, dapat langsung diganti dengan `#[derive(Default)]`.

---

## 3. Ringkasan Warning Lint Clippy

| Crate | Warning | Jenis |
|-------|---------|------|
| ecat-versioning | `useless_format!` — menggunakan `"/api".to_string()` | Performa |
| ecat-versioning | unused import / dead code | Pembersihan |
| ecat-data-questdb | `unnecessary_to_owned` | Performa |
| ecat-client | `derivable_impls` — pakai derive Default | Penyederhanaan |

---

## 4. Analisis Cakupan Pengujian

### 4.1 Data Statistik

| Metrik | Nilai |
|------|------|
| Total unit test | ~137 |
| Gagal | 0 |
| Ignored | 1 |
| Crate dengan tes | ~24 / 48 |
| **Crate dengan 0 tes** | **~24 / 48 (50%)** |

### 4.2 Crate yang Kurang Tes (0 atau hanya tes konstruksi)

Crate berikut tesnya lemah:

- ecat-data-arangodb, ecat-data-clickhouse, ecat-data-elasticsearch
- ecat-data-influxdb, ecat-data-iotdb, ecat-data-nebulagraph
- ecat-data-neo4j, ecat-data-opensearch, ecat-data-questdb
- ecat-data-redis, ecat-data-sqlx, ecat-data-memcached
- ecat-mq, ecat-mq-kafka, ecat-graphql, ecat-openapi
- ecat-transport, ecat-transport-grpc, ecat-transport-http
- ecat-transport-ws, ecat-tracing, ecat-logging
- ecat-middleware, ecat-registry-consul, ecat-registry-etcd

### 4.3 Doc-tests

Semua **doc-tests dari 48 crate adalah 0**. Tidak ada contoh dokumentasi `/// ````rust` di kode.

---

## 5. Masalah Dependensi

### 5.1 ⚠️ yaml_serde vs serde_yaml (risiko sedang)

**File**: `ecat-config/Cargo.toml:9`

```toml
yaml_serde = "0.10"
```

Library YAML standar di ekosistem Rust adalah `serde_yaml` (versi terbaru `0.9.34+`), sedangkan `yaml_serde` adalah crate yang **berbeda dan kurang dipelihara**.

**Saran**: Konfirmasi apakah `yaml_serde` adalah dependensi yang dimaksud. Jika yang dimaksud adalah `serde_yaml`, silakan ganti.

### 5.2 Kekurangan cargo-audit

`cargo audit` belum diinstal. Disarankan `cargo install cargo-audit` dan tambahkan ke CI.

### 5.3 Kekurangan kolom description

`[workspace.package]` tidak memiliki `description`, semua sub-crate juga tidak mendefinisikan description.

---

## 6. Masalah Kualitas Kode

### 6.1 unwrap/expect di kode produksi

| File | Baris | Panggilan | Risiko |
|------|------|------|------|
| `ecat-client/src/lib.rs` | 28 | `.expect("StaticResolver poisoned")` | Rendah — masuk akal |
| `ecat/src/signal.rs` | 8 | `.expect("failed to install SIGTERM handler")` | Sedang — panic saat start |
| `ecat-protos/build.rs` | 5 | `.unwrap()` | Rendah — build script |

### 6.2 extract_version di ecat-versioning

Fungsi `extract_version` (baris 68) mengimplementasikan ekstraksi nomor versi dari header Accept, tetapi tidak dipanggil oleh `build_header_router()`.

### 6.3 Penanganan error ecat-data-questdb

```rust
// 第 30 行: 网络响应体读取使用 unwrap_or_default
Err(RdbmsError::Database(resp.text().await.unwrap_or_default()))
```

Saat `resp.text()` gagal, pesan error ditelan diam-diam. Disarankan diubah menjadi `unwrap_or_else(|e| format!("questdb parse: {e}"))`.

---

## 7. Evaluasi Arsitektur

### Kelebihan

- 48 crate memiliki pemisahan tanggung jawab yang jelas
- Workspace versi terpadu `version.workspace = true`
- Dependensi ringkas, tanpa framework besar
- Tidak ada TODO/FIXME/HACK

### Perlu Ditingkatkan

| Masalah | Prioritas |
|------|--------|
| 50% crate tanpa tes | Tinggi |
| Kebingungan yaml_serde vs serde_yaml | Sedang |
| Kekurangan cargo-audit | Sedang |
| Kode mati ecat-versioning | Rendah |
| Tanpa doc-tests | Rendah |

---

## 8. Ringkasan Keamanan

| Item pemeriksaan | Hasil |
|--------|------|
| Kunci hardcoded | Tidak ditemukan |
| Kebocoran file .env | Tidak ditemukan |
| unwrap berbahaya (kode produksi) | 2 tempat (signal.rs, client.rs) |
| Pemindaian CVE | Belum dilakukan (perlu menginstal cargo-audit) |

---

## 9. Rencana Aksi

### P0 — Perbaikan Segera
1. Membersihkan 3 warning kompiler ecat-versioning
2. Memperbaiki clippy ecat-data-questdb
3. Memperbaiki derivable_impls ecat-client

### P1 — Jangka Pendek
4. Menginstal `cargo-audit` untuk memindai kerentanan dependensi
5. Mengonfirmasi pilihan `yaml_serde` vs `serde_yaml`
6. Melengkapi doc-tests untuk crate inti

### P2 — Jangka Menengah
7. Melengkapi tes untuk crate transport/data/security
8. Menambahkan kolom `description` untuk semua crate
9. Mengintegrasikan atau menghapus `extract_version`

### P3 — Jangka Panjang
10. Membangun CI: build → test → clippy → audit → coverage

---

*Laporan dibuat pada 2026-08-01. Toolchain: cargo 1.92.0, rustc 1.92.0, clippy 1.92.0*
