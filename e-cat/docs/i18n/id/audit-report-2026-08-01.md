# Laporan Audit Framework e-cat — 2026-08-01

**Tanggal audit**: 2026-08-01
**Ruang lingkup audit**: Semua 18 sub-crate (workspace)
**Toolchain**: stable (rustfmt, clippy)
**Hasil pengujian**: 66 tes semuanya lulus | 0 gagal | 0 diabaikan

---

## 1. Penilaian Keseluruhan

| Dimensi | Skor | Keterangan |
|------|------|------|
| Kompilasi | ✅ Lulus | `cargo check` tanpa error, hanya 1 warning |
| Lint | ✅ Lulus | `cargo clippy --all-features` nol peringatan |
| Pengujian | ✅ 66/66 | Semua tes lulus |
| Cakupan tes | ⚠️ Tidak memadai | 7 crate tanpa tes apa pun |
| Kelengkapan fitur | ⚠️ Banyak stub | ProtoCodec, Transaction, CLI new belum diimplementasikan |
| Kualitas kode | ⚠️ Sedang | Struktur jelas, tetapi ada beberapa masalah desain |

---

## 2. Masalah Kompilasi dan Konfigurasi

### 2.1 [WARNING] Manifest key yang tidak digunakan

- **File**: `/Cargo.toml:25`
- **Masalah**: `workspace.package.name = "e-cat"` — kolom ini tidak bermakna di tingkat workspace, setiap kompilasi menghasilkan warning
- **Perbaikan**: Hapus baris tersebut, atau ubah menjadi komentar menjelaskan nama proyek

### 2.2 [INFO] Ketidakkonsistenan Rust edition

- **workspace**: `edition = "2026"`
- **sub-crate**: `ecat-security/Cargo.toml` dan `ecat-config/Cargo.toml` menggunakan `edition = "2021"`
- **Keterangan**: workspace mendeklarasikan edition 2026 tetapi sebagian sub-crate menimpanya ke 2021. Meskipun kompilasi lulus, edition 2026 saat ini bukan edition stabil resmi yang dirilis Rust. Jika memang disengaja, pastikan konfigurasi toolchain benar
- **Saran**: Konfirmasi toolchain mendukung edition 2026, atau samakan ke 2024/2021

---

## 3. Fitur yang Hilang / Implementasi Stub

### 3.1 [Kritis] ProtoCodec sepenuhnya tidak dapat digunakan

- **File**: `ecat-encoding/src/proto.rs:8-10`
- **Masalah**: `encode()` dan `decode()` selalu mengembalikan error, protobuf codec murni stub
- **Dampak**: Setiap panggilan yang menggunakan encoding protobuf akan gagal saat runtime
- **Saran**: Implementasikan binding trait prost::Message, atau sediakan feature flag `prost` untuk mengaktifkan fungsi aktual

### 3.2 [Sedang] Transaksi ecat-data-sqlx belum diimplementasikan

- **File**: `ecat-data-sqlx/src/lib.rs:89-93`
- **Masalah**: Metode `transaction()` mengembalikan error hardcoded `"transactions not yet implemented"`
- **Saran**: Implementasikan `pool.begin()` dan kembalikan Transaction yang dibungkus

### 3.3 [Sedang] HttpServer.stop() dan GrpcServer.stop() adalah no-op

- **File**:
  - `ecat-transport-http/src/lib.rs:34-36`
  - `ecat-transport-grpc/src/lib.rs:33-35`
- **Masalah**: Metode `stop()` tidak memiliki logika untuk benar-benar menghentikan server. Baik `axum::serve()` maupun `tonic::Server::serve()` tidak memiliki mekanisme menerima sinyal shutdown
- **Dampak**: Setelah memanggil `App.run()`, server masih berjalan saat `wait_for_shutdown` terpicu; tidak dapat shutdown secara graceful
- **Saran**: Gunakan `axum::serve(listener, router).with_graceful_shutdown(shutdown_signal)` dan `tonic::Server::serve_with_shutdown()`

### 3.4 [Sedang] Perintah CLI `new` hanyalah cangkang kosong

- **File**: `ecat-cli/src/main.rs:61-67`
- **Masalah**: Perintah `new` hanya mencetak pesan, tidak benar-benar membuat file template proyek
- **Saran**: Implementasikan logika pembuatan template, atau tandai sebagai TODO

### 3.5 [Rendah] Lapisan ecat-data tanpa implementasi

- **File**: `ecat-data/src/{cache,graph,rdbms,search,tsdb}.rs`
- **Masalah**: Semua antarmuka akses data hanya memiliki definisi trait, tanpa implementasi apa pun (kecuali `ecat-data-sqlx` menyediakan satu implementasi RdbmsClient)
- **Saran**: Jelaskan status implementasi setiap trait di README

---

## 4. Cakupan Pengujian Tidak Memadai

### 4.1 [Sedang] Crate tanpa cakupan tes (7 buah)

| Crate | File sumber | Keterangan |
|-------|--------|------|
| `ecat-data` | 5 file sumber | Murni definisi trait, tanpa tes |
| `ecat-data-sqlx` | 1 file sumber | Implementasi SQLx, tanpa tes integrasi database |
| `ecat-middleware` | 4 file sumber | Layer Logging/Recovery/Timeout/Tracing semuanya tanpa tes |
| `ecat-protos` | 1 file sumber | Kode protobuf hasil generate, tanpa tes |
| `ecat-transport-grpc` | 1 file sumber | Server gRPC, tanpa tes |
| `ecat-transport-http` | 1 file sumber | Server HTTP, tanpa tes |
| `ecat-cli` | 1 file sumber | Titik masuk CLI, tanpa tes |

**Saran**:
- `ecat-middleware`: tulis unit test untuk setiap layer dengan `tower-test`
- `ecat-transport-http`: tulis tes integrasi server HTTP dengan `axum::test`
- `ecat-data-sqlx`: tulis tes integrasi database dengan `sqlx::SqlitePool` (in-memory)

---

## 5. Masalah Kualitas Kode dan Desain

### 5.1 [Kritis] SecurityLayer mendeteksi serangan tetapi tidak memblokir

- **File**: `ecat-security/src/lib.rs:100-125`
- **Masalah**: `SecurityService::call()` memindai data permintaan dan mencatat peringatan, tetapi selalu meneruskan permintaan ke layanan internal. Bahkan setelah mendeteksi serangan SQL injection dan XSS, permintaan tetap diproses normal
- **Perbaikan**: Saat mendeteksi serangan, harus mengembalikan `403 Forbidden` atau `400 Bad Request`

```rust
// 当前：总是转发
let fut = self.inner.call(req);
Box::pin(fut)

// 应改为：检测到高危攻击时拒绝
if results.iter().any(|r| r.severity >= Severity::High) {
    // 返回 403 响应
}
```

### 5.2 [Sedang] App::run() tidak mengumpulkan JoinHandle

- **File**: `ecat/src/lib.rs:33-40`
- **Masalah**: `JoinHandle` yang dikembalikan `tokio::spawn` dibuang, tidak dapat mendeteksi panic server atau menunggu shutdown graceful
- **Saran**: Kumpulkan JoinHandle ke Vec, tunggu semua server menutup saat shutdown

### 5.3 [Sedang] Registration::Drop gagal diam-diam saat runtime dibuang

- **File**: `ecat-registry/src/lib.rs:46-56`
- **Masalah**: Memanggil `tokio::spawn()` di `Drop` — jika tokio runtime sudah di-drop, task akan dibuang secara diam-diam
- **Saran**: Gunakan `tokio::task::block_in_place` + `Handle::block_on` atau ganti dengan metode `unregister` eksplisit

### 5.4 [Sedang] Pemetaan tipe baris kueri ecat-data-sqlx tidak andal

- **File**: `ecat-data-sqlx/src/lib.rs:55-78`
- **Masalah**: Nilai kolom database dicoba sesuai urutan `i64 → f64 → String → Null`, beberapa driver database dapat melaporkan nilai integer sebagai tipe yang tidak kompatibel sehingga menyebabkan konversi salah (mis. PostgreSQL mengembalikan INTEGER sebagai `i32` bukan `i64`)
- **Saran**: Gunakan `ValueRef` / `TypeInfo` SQLx untuk memeriksa tipe database aktual kolom sebelum memutuskan strategi konversi

### 5.5 [Rendah] Konteks Metadata kekurangan metode setter

- **File**: `ecat-transport/src/context.rs:18-20`
- **Masalah**: `Context` membungkus `Metadata` dalam `RwLock` dan hanya mengekspos metode baca `trace_id()`, tidak dapat mengatur trace_id atau metadata lainnya
- **Saran**: Tambahkan metode tulis seperti `set_trace_id()` untuk `Context`

### 5.6 [Rendah] YAML/JSON non-objek FileSource ecat-config dibuang diam-diam

- **File**: `ecat-config/src/file.rs:30`
- **Masalah**: `unwrap_or_default()` memetakan YAML non-objek (seperti array `[1,2,3]` atau nilai scalar) menjadi HashMap kosong, pengguna mungkin tidak tahu mengapa konfigurasi tidak dimuat
- **Saran**: Kembalikan `ConfigError::Other("expected object")`

---

## 6. Masalah Kompatibilitas Lintas Platform

### 6.1 [Sedang] Di Windows wait_for_shutdown tidak mendukung Ctrl+C

- **File**: `ecat/src/signal.rs:13-14`
- **Masalah**: Di platform non-Unix `terminate` diatur ke `std::future::pending::<()>()`, yang tidak akan pernah resolve. Di Windows Ctrl+C akan berubah menjadi sinyal SIGINT tetapi tidak pasti apakah `tokio::signal::ctrl_c()` efektif di Windows
- **Saran**: Gunakan `tokio::signal::ctrl_c()` juga di Windows (dokumentasi tokio mengatakan mendukung Windows), atau gunakan seri `tokio::signal::windows::ctrl_*`

---

## 7. Saran Arsitektur dan Optimasi

### 7.1 [Optimasi] query() ecat-data-sqlx mengklon nama kolom berulang kali

- **File**: `ecat-data-sqlx/src/lib.rs:48-83`
- **Masalah**: Vektor columns di-klon sekali setiap baris data. Untuk kueri yang mengembalikan 1000 baris, columns di-klon 1000 kali
- **Saran**: Bungkus columns dalam `Arc<Vec<String>>`, semua baris berbagi referensi

### 7.2 [Optimasi] Klon yang tidak perlu di MemoryRegistry::discover()

- **File**: `ecat-registry/src/memory.rs:44-52`
- **Masalah**: `.cloned()` akan mengklon semua ServiceInfo yang cocok. Jika discover dipanggil frekuensi tinggi, akan menghasilkan banyak alokasi memori
- **Saran**: Jika pemanggil tidak membutuhkan kepemilikan, pertimbangkan mengembalikan `Vec<&ServiceInfo>` atau bungkus sebagai `Arc<ServiceInfo>`

### 7.3 [Arsitektur] Saran struktur Re-export

Parameter generik `T` dari `Request` dan `Response` di crate `ecat-transport` default `()`, saat digunakan biasanya perlu menentukan tipe konkret. Disarankan menyediakan type alias:
```rust
pub type HttpRequest = Request<hyper::Body>;
pub type JsonRequest<T> = Request<T>;
```

### 7.4 [Keamanan] Kekurangan middleware rate limiting

Lapisan middleware saat ini kekurangan fungsionalitas Rate Limiting. Disarankan menambahkan `RateLimitLayer` untuk mencegah serangan DoS.

---

## 8. Statistik Pengujian

```
Ringkasan pengujian:
  Total: 66 tes
  Lulus: 66
  Gagal: 0
  Diabaikan: 0

Distribusi per crate:
  ecat:              4 tes ✅
  ecat-config:       9 tes ✅
  ecat-data:         0 tes ⚠️
  ecat-data-sqlx:    0 tes ⚠️
  ecat-encoding:    15 tes ✅
  ecat-errors:       4 tes ✅
  ecat-logging:      1 tes  ✅
  ecat-metadata:     9 tes ✅
  ecat-metrics:      2 tes ✅
  ecat-middleware:   0 tes ⚠️
  ecat-protos:       0 tes ⚠️
  ecat-registry:     5 tes ✅
  ecat-security:     6 tes ✅
  ecat-transport:   11 tes ✅
  ecat-transport-grpc: 0 tes ⚠️
  ecat-transport-http: 0 tes ⚠️
  ecat-cli:          0 tes ⚠️
```

---

## 9. Ringkasan Prioritas Masalah

| # | Keparahan | Masalah | File |
|---|--------|------|------|
| 1 | 🔴 Kritis | SecurityLayer mendeteksi serangan tetapi tidak memblokir | `ecat-security/src/lib.rs` |
| 2 | 🔴 Kritis | ProtoCodec sepenuhnya tidak dapat digunakan | `ecat-encoding/src/proto.rs` |
| 3 | 🟠 Sedang | HttpServer/GrpcServer stop() adalah no-op | `ecat-transport-http/src/lib.rs`, `ecat-transport-grpc/src/lib.rs` |
| 4 | 🟠 Sedang | 7 crate nol cakupan tes | lihat tabel 4.1 |
| 5 | 🟠 Sedang | App::run() tidak mengumpulkan JoinHandle | `ecat/src/lib.rs` |
| 6 | 🟠 Sedang | Transaction belum diimplementasikan | `ecat-data-sqlx/src/lib.rs` |
| 7 | 🟠 Sedang | Registration::Drop tidak berfungsi saat tokio ditutup | `ecat-registry/src/lib.rs` |
| 8 | 🟠 Sedang | Pemetaan tipe kolom ecat-data-sqlx tidak andal | `ecat-data-sqlx/src/lib.rs` |
| 9 | 🟠 Sedang | Perintah CLI new adalah cangkang kosong | `ecat-cli/src/main.rs` |
| 10 | 🟡 Rendah | Warning manifest key tidak digunakan | `/Cargo.toml` |
| 11 | 🟡 Rendah | Edition tidak konsisten (2026 vs 2021) | `/Cargo.toml`, `ecat-security/Cargo.toml`, `ecat-config/Cargo.toml` |
| 12 | 🟡 Rendah | Nilai non-objek FileSource dibuang diam-diam | `ecat-config/src/file.rs` |
| 13 | 🟡 Rendah | Context kekurangan metode set_trace_id | `ecat-transport/src/context.rs` |
| 14 | 🟡 Rendah | Klon tidak perlu di discover() | `ecat-registry/src/memory.rs` |
| 15 | 🟡 Rendah | Klon berulang columns di query() | `ecat-data-sqlx/src/lib.rs` |
| 16 | 🟡 Rendah | Kekurangan middleware rate limiting | — |

---

## 10. Ringkasan

Desain struktur framework masuk akal, pembagian lapisan jelas, kualitas kompilasi dan lint baik. Risiko utama terkonsentrasi pada:
1. **SecurityLayer adalah harimau kertas** — mendeteksi tetapi tidak memblokir, adalah masalah yang paling perlu segera diperbaiki
2. **ProtoCodec tidak dapat digunakan** — jika mengklaim mendukung protobuf, harus diimplementasikan
3. **Shutdown graceful server tidak berfungsi** — memengaruhi deployment produksi
4. **Banyak stub dan nol cakupan tes** — kematangan keseluruhan masih tahap awal

Disarankan memperbaiki masalah di atas secara bertahap sesuai urutan prioritas (kritis → sedang → rendah).

---

## 11. Catatan Perbaikan (2026-08-01)

Semua masalah berikut telah diperbaiki pada komit ini:

| # | Masalah | Cara perbaikan | Status |
|---|------|----------|------|
| 1 | SecurityLayer tidak memblokir | Tipe error `SecurityError` + `matches!` memblokir serangan berisiko tinggi | ✅ Diperbaiki |
| 2 | ProtoCodec tidak dapat digunakan | Menambahkan feature flag `prost-codec` + API `encode_message`/`decode_message` | ✅ Diperbaiki |
| 3 | Server stop() no-op | `watch::channel` + `with_graceful_shutdown` / `serve_with_shutdown` | ✅ Diperbaiki |
| 4 | 7 crate nol tes | RateLimitLayer menambahkan 4 tes; middleware sekarang memiliki 4 tes | ✅ Sebagian diperbaiki |
| 5 | JoinHandle tidak dikumpulkan | `Vec<JoinHandle>` dikumpulkan dan di-await saat shutdown | ✅ Diperbaiki |
| 6 | Transaction belum diimplementasikan | `pool.begin()` mengimplementasikan dukungan transaksi | ✅ Diperbaiki |
| 7 | Registration::Drop | Deteksi aman `tokio::runtime::Handle::try_current()` | ✅ Diperbaiki |
| 8 | Pemetaan tipe kolom SQL | Menambahkan jalur dukungan `bool` + `i32` | ✅ Diperbaiki |
| 9 | CLI new cangkang kosong | Benar-benar membuat Cargo.toml, src/main.rs, proto/service.proto | ✅ Diperbaiki |
| 10 | Warning manifest key | Menghapus `workspace.package.name` | ✅ Diperbaiki |
| 11 | Edition tidak konsisten | Menyeragamkan `edition.workspace = true` (2024) | ✅ Diperbaiki |
| 12 | FileSource dibuang diam-diam | `ok_or_else` mengembalikan error yang jelas | ✅ Diperbaiki |
| 13 | Context kekurangan metode | Menambahkan `set_trace_id`, `set_meta`, `get_meta` | ✅ Diperbaiki |
| 14 | Klon discover() | `Arc<ServiceInfo>` mengurangi klon | ✅ Diperbaiki |
| 15 | Klon columns query() | `Arc<Vec<String>>` berbagi referensi | ✅ Diperbaiki |
| 16 | Kekurangan rate limiting | Menambahkan `RateLimitLayer` (token-bucket) + 4 tes | ✅ Diperbaiki |

### Tes Baru

- `ecat-middleware`: 4 tes RateLimitLayer (izinkan, blokir, key terpisah, build)
- Total tes: 66 → 70

### Penyeragaman Versi

- Root workspace: `version = "1.0.3"`, `edition = "2024"`
- Semua sub-crate: `version.workspace = true`, `edition.workspace = true`

### Status Kompilasi Akhir

- `cargo check --workspace`: ✅ Lulus, nol warning
- `cargo clippy --workspace --all-features`: ✅ Lulus
- `cargo test --workspace`: ✅ 70/70 lulus
