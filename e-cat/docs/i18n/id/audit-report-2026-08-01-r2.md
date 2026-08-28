# Laporan Audit Framework e-cat R2 — 2026-08-01

**Versi**: 1.0.5
**Ruang lingkup**: Semua 18 sub-crate
**Kesimpulan**: `cargo check` / `cargo clippy --all-features` / `cargo test` semuanya lulus, 70 tests ✅

---

## 1. Tinjauan Perbaikan Sebelumnya (16/16 telah diperbaiki)

Masalah yang ditemukan pada audit sebelumnya (R1) semuanya telah diperbaiki: SecurityLayer memblokir serangan, dukungan prost ProtoCodec, shutdown graceful Server, pengumpulan JoinHandle, implementasi Transaction, deteksi aman Registration Drop, peningkatan pemetaan tipe kolom, pembuatan file CLI new, penyeragaman versi/edition, penanganan error FileSource, metode metadata Context, optimasi Arc discover, optimasi Arc columns query, RateLimitLayer baru.

---

## 2. Masalah Baru yang Ditemukan pada Putaran Ini

### 2.1 [Kritis] Kode template yang dihasilkan CLI `new` tidak dapat dikompilasi

- **File**: `ecat-cli/src/main.rs:79-97`
- **Masalah**: `Cargo.toml` yang dihasilkan menggunakan referensi dependensi `workspace = true` dan path relatif `path = "../ecat"`, tetapi proyek independen yang dibuat `ecat new myapp` tidak berada di dalam workspace e-cat, semua referensi ini akan gagal resolve
- **Dampak**: Proyek yang dibuat `ecat new` sama sekali tidak dapat dikompilasi
- **Perbaikan**: Template harus menggunakan dependensi aktual dengan nomor versi, bukan referensi workspace

```toml
# 当前（错误）：
tokio.workspace = true           # 项目不在 workspace 中，报错
ecat = { path = "../ecat" }      # 相对路径无效

# 应改为：
tokio = { version = "1", features = ["full"] }
ecat = "1.0.5"
```

### 2.2 [Kritis] `transaction()` ecat-data-sqlx membuang handle transaksi database yang sebenarnya

- **File**: `ecat-data-sqlx/src/lib.rs:100-106`
- **Masalah**: `pool.begin()` mengembalikan handle transaksi database yang sebenarnya `Transaction<'_, DB>`, tetapi kode mengikatnya sebagai `_tx` lalu langsung membuangnya. Saat `_tx` di-drop, transaksi database otomatis rollback. `ecat_data::Transaction` yang dikembalikan hanyalah cangkang kosong, metode `commit()/rollback()`-nya tidak berpengaruh
- **Dampak**: Semua kode yang menggunakan `transaction()` berjalan tanpa perlindungan transaksi, konsistensi data tidak dapat dijamin
- **Perbaikan**: Perlu mendesain ulang struct `ecat_data::Transaction` agar menyimpan handle transaksi database yang sebenarnya

### 2.3 [Sedang] SecurityLayer tidak memindai body permintaan

- **File**: `ecat-security/src/lib.rs:117-127`
- **Masalah**: `call()` hanya memindai URI dan header HTTP, sama sekali tidak memeriksa body permintaan. Penyerang dapat menempatkan payload SQL injection/XSS di body POST untuk melewati deteksi dengan mudah
- **Dampak**: Sangat mengurangi cakupan efektif deteksi serangan
- **Perbaikan**: Perlu menambahkan kemampuan pemindaian body, atau menyediakan metode publik `scan_body()` agar pemanggil dapat menggunakannya setelah membaca body

### 2.4 [Sedang] RateLimitLayer menggunakan Mutex sinkron + tanpa pembersihan kedaluwarsa

- **File**: `ecat-middleware/src/ratelimit.rs:10-38`
- **Masalah 1**: `std::sync::Mutex` digunakan dalam konteks async — jika terjadi kontensi lock, akan memblokir seluruh thread worker tokio
- **Masalah 2**: `buckets: HashMap<String, (u32, Instant)>` tidak pernah membersihkan key yang kedaluwarsa, memori server yang berjalan lama tumbuh tanpa batas (setiap IP/key baru menempati memori selamanya)
- **Dampak**: Performa menurun di bawah konkurensi tinggi, kebocoran memori setelah berjalan lama
- **Perbaikan**: Ganti dengan `tokio::sync::Mutex`, dan bersihkan entri kedaluwarsa secara berkala di `allow()`

### 2.5 [Sedang] SQL mentah ecat-data-sqlx tanpa API parameterisasi

- **File**: `ecat-data-sqlx/src/lib.rs:24-29, 32-36`
- **Masalah**: `execute(&self, sql: &str)` dan `query(&self, sql: &str)` hanya menerima string SQL mentah, di tingkat trait tidak ada metode binding parameter. Pemanggil yang menggabungkan input pengguna ke SQL dapat menyebabkan SQL injection
- **Dampak**: Meskipun trait sendiri tidak secara langsung mengekspos celah keamanan, kurangnya API parameterisasi akan menggoda pemanggil menulis kode yang tidak aman
- **Saran**: Tambahkan metode `execute_with` dan `query_with` ke trait `RdbmsClient` untuk menggunakan binding parameter

### 2.6 [Rendah] Arc::clone di query() masih berada di dalam closure

- **File**: `ecat-data-sqlx/src/lib.rs:50-53`
- **Masalah**: `let cols = std::sync::Arc::clone(&columns)` dieksekusi di dalam closure `rows.iter().map()`. Meskipun Arc::clone sangat ringan (hanya increment penghitung referensi atomik), dapat dipindahkan ke luar closure untuk menghindari satu operasi atomik per baris
- **Saran**: Lakukan satu kali clone sebelum `iter()`, closure menangkap clone tersebut

### 2.7 [Rendah] Trait impl ProtoCodec tidak konsisten dengan API baru

- **File**: `ecat-encoding/src/proto.rs`
- **Masalah**: `encode/decode` dari trait `Codec` masih hanya mengembalikan error; `encode_message/decode_message` yang baru adalah jalur yang benar tetapi nama metode tidak cocok dengan trait. Pengguna mungkin mencoba `codec.encode()` terlebih dahulu lalu bingung mengapa gagal
- **Saran**: Jelaskan di dokumentasi/komentar: tipe proto harus menggunakan `encode_message/decode_message` bukan metode trait Codec

---

## 3. Ringkasan Status Saat Ini

| Dimensi | Status |
|------|------|
| `cargo check` | ✅ Nol warning |
| `cargo clippy --all-features` | ✅ Nol peringatan |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 lulus |
| Versi terpadu | ✅ 1.0.5 |
| Edition terpadu | ✅ 2024 |

### Distribusi Pengujian

| Crate | Tests | Keterangan |
|-------|-------|------|
| ecat | 4 | ✅ |
| ecat-config | 9 | ✅ |
| ecat-encoding | 15 | ✅ |
| ecat-errors | 4 | ✅ |
| ecat-logging | 1 | ✅ |
| ecat-metadata | 9 | ✅ |
| ecat-metrics | 2 | ✅ |
| ecat-middleware | 4 | ✅ (termasuk RateLimitLayer) |
| ecat-registry | 5 | ✅ |
| ecat-security | 6 | ✅ |
| ecat-transport | 11 | ✅ |
| ecat-data | 0 | — (murni definisi trait) |
| ecat-data-sqlx | 0 | ⚠️ Tanpa tes integrasi DB |
| ecat-protos | 0 | — (kode hasil generate) |
| ecat-transport-grpc | 0 | ⚠️ |
| ecat-transport-http | 0 | ⚠️ |
| ecat-cli | 0 | ⚠️ |

---

## 4. Prioritas Masalah

| # | Keparahan | Masalah | File | Dampak pengguna |
|---|--------|------|------|----------|
| 1 | 🔴 | Template yang dihasilkan CLI `new` adalah kode yang tidak dapat dikompilasi | `ecat-cli/src/main.rs:79` | Perintah pertama pengguna baru langsung gagal |
| 2 | 🔴 | transaction() membuang handle transaksi DB yang sebenarnya | `ecat-data-sqlx/src/lib.rs:100` | Konsistensi data tanpa jaminan |
| 3 | 🟠 | SecurityLayer tidak memindai body | `ecat-security/src/lib.rs:117` | Penyerang dapat melewati deteksi |
| 4 | 🟠 | Mutex std RateLimitLayer + kebocoran memori | `ecat-middleware/src/ratelimit.rs:10,25` | Performa konkurensi + OOM |
| 5 | 🟠 | SQL mentah tanpa API parameterisasi | `ecat-data-sqlx/src/lib.rs:24` | Risiko SQL injection |
| 6 | 🟡 | Posisi Arc clone di query() | `ecat-data-sqlx/src/lib.rs:53` | Optimasi performa kecil |
| 7 | 🟡 | API ProtoCodec tidak konsisten | `ecat-encoding/src/proto.rs` | Kebingungan pengguna |

---

## 6. Catatan Perbaikan (2026-08-01 R2)

| # | Masalah | Cara perbaikan | Status |
|---|------|----------|------|
| 1 | Template CLI new tidak dapat dikompilasi | Ganti dengan dependensi ber-version (`ecat = "1.0"`, `tokio = "1"`, dll.) | ✅ |
| 2 | transaction() membuang transaksi DB | `Transaction::with_inner()` menyimpan handle yang sebenarnya, sqlx diteruskan melalui `Box<dyn Any>` | ✅ |
| 3 | SecurityLayer tidak memindai body | Menambahkan metode publik `scan_body(&[u8])` | ✅ |
| 4 | Mutex RateLimitLayer + kebocoran | `tokio::sync::Mutex` + membersihkan entri kedaluwarsa setiap 100 key | ✅ |
| 5 | SQL mentah tanpa API parameterisasi | `RdbmsClient` menambahkan metode parameterisasi `execute_with`/`query_with` | ✅ |
| 6 | Posisi Arc clone di query() | `Arc::clone` dipindahkan ke luar `iter()`, semua baris berbagi referensi | ✅ |
| 7 | API ProtoCodec tidak konsisten | Dokumentasi tingkat modul + dokumentasi struct menjelaskan cara penggunaan | ✅ |

### Status Akhir

| Item pemeriksaan | Hasil |
|--------|------|
| `cargo check` | ✅ Nol error / nol warning |
| `cargo clippy --all-features` | ✅ Nol warning |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 lulus |
| Versi | 1.0.5 (semua warisan workspace terpadu) |
| Edition | 2024 |
