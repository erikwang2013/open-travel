# Laporan Peninjauan Mendalam e-cat — 2026-08-01 R6

## Penilaian Keseluruhan

| Dimensi | Status | Keterangan |
|------|------|------|
| Kompilasi | Lulus | 50 crates, nol error |
| Pengujian | Lulus | Semuanya lulus, nol kegagalan |
| Clippy | Lulus | Nol peringatan (`-D warnings`) |
| unsafe | Nol | Tidak ada blok unsafe di codebase |
| Ukuran file | Baik | Hanya `ecat-auth` (540 baris) melebihi nilai saran 500 baris |

## Temuan (15 Item)

### Terkait Keamanan

#### 1. [Kritis] XOR "enkripsi" bukan enkripsi yang sebenarnya
**File:** `ecat-config/src/encrypted.rs:45-56`
**Masalah:** `decrypt()` menggunakan XOR + kunci berulang, ini adalah obfuscation bukan enkripsi, mudah dipecahkan. Kunci digunakan berulang di setiap posisi byte, membuat ciphertext sangat rentan terhadap analisis frekuensi.
**Saran:** Ganti dengan AES-256-GCM (crate `aes-gcm`), atau tandai secara eksplisit sebagai "obfuscation" bukan "enkripsi".

#### 2. [Kritis] Implementasi default `execute_with`/`query_with` membuang parameter secara diam-diam
**File:** `ecat-data/src/rdbms.rs:86-103`
**Masalah:** Implementasi default di trait menerima parameter tetapi mengabaikannya (`let _ = params;`), langsung memanggil `execute(sql)` asli. Semua backend selain `ecat-data-sqlx` (ClickHouse, QuestDB) mewarisi perilaku ini. Jika pengguna mengganti backend dengan metode parameterisasi, parameter akan dibuang diam-diam, menyebabkan celah SQL injection.
**Saran:** Implementasi default harus mengembalikan error "tidak didukung", atau setiap backend mengimplementasikan binding parameter dengan benar.

#### 3. [Tinggi] Kata sandi tertanam plaintext di URL
**File:** `ecat-data-sqlx/src/lib.rs:40`, `ecat-data-redis/src/lib.rs:43`
**Masalah:** `connect_with_auth()` menggunakan `replacen("://", "://user:pass@")` menanamkan kredensial langsung ke URL. URL ini dapat tercatat di log, pesan error, atau output debug.
**Saran:** Gunakan mekanisme autentikasi native masing-masing backend; atau setidaknya lakukan URL-encode username/password sebelum penggabungan.

#### 4. [Sedang] Kegagalan konfigurasi TLS menyebabkan panic
**File:** 8 crate data-* (ClickHouse, QuestDB, Elasticsearch, OpenSearch, ArangoDB, Neo4j, NebulaGraph, InfluxDB, IoTDB)
**Pola:** `.expect("TLS client build failed")` — semua konstruktor `from_config()` panic saat konfigurasi TLS salah.
**Saran:** Ubah `from_config()` mengembalikan `Result`, atau buat pembangunan klien TLS menjadi lazy/toleran kesalahan.

### Kebenaran Fungsional

#### 5. [Tinggi] Routing header `ecat-versioning` tidak efektif
**File:** `ecat-versioning/src/lib.rs:56-64`
**Masalah:** `build_header_router()` menumpuk semua versi di bawah path `/api` yang sama, tetapi tidak memfilter berdasarkan header versi. axum akan mendaftarkan semua route versi ke path yang sama, menyebabkan konflik route dan perilaku tidak dapat diprediksi. Fungsi `extract_version()` ada tetapi tidak pernah digunakan dalam routing.
**Saran:** Gunakan middleware/layer axum untuk memeriksa header Accept dan merutekan ke route versi yang benar, bukan meratakan semua versi ke path yang sama.

#### 6. [Sedang] Pemotongan TTL Redis: kedaluwarsa sub-detik berubah menjadi tidak pernah kedaluwarsa
**File:** `ecat-data-redis/src/lib.rs:76-77`
**Masalah:** `Duration::as_secs()` memotong menuju nol. Mengatur TTL 500ms akan berubah diam-diam menjadi tidak pernah kedaluwarsa saat `secs == 0`, melewati cabang `SET` bukan `SETEX`.
**Saran:** Untuk TTL sub-detik, setidaknya atur 1 detik, atau gunakan `SET ... PX` (mili detik) menggantikan `SETEX`.

#### 7. [Sedang] `StaticResolver::add_service` panic saat kontensi lock
**File:** `ecat-client/src/lib.rs:27-29`
**Masalah:** Menggunakan `try_write()` dengan expect, panic jika ada pemegang write lock lain. Pola builder membuat masalah ini sulit terpicu, tetapi merupakan bom waktu di kode konkuren.
**Saran:** Gunakan `blocking_write()` (jika dalam konteks sinkron) atau ubah menerima `&mut self` untuk menghindari kebutuhan lock.

### Kualitas Kode

#### 8. [Sedang] Penggunaan `std::sync::Mutex` dalam konteks async
**File:** `ecat-data-memcached/src/lib.rs:7,24`
**Masalah:** Menggunakan `std::sync::Mutex` di implementasi trait async. Meskipun waktu memegang lock sangat singkat (hanya operasi HashMap), secara teoretis dapat memblokir runtime async di bawah kontensi tinggi.
**Saran:** Untuk skenario penggunaan cache memori khusus ini, karena critical section sangat pendek dan tanpa titik `.await`, penggunaan `std::sync::Mutex` sebenarnya dapat diterima. Namun jika ke depan perlu melakukan operasi I/O di dalam lock, harus diganti `tokio::sync::Mutex`.

#### 9. [Rendah] Implementasi base64 tulisan tangan
**File:** `ecat-registry-etcd/src/lib.rs:148-193`
**Masalah:** ~45 baris codec base64 tulisan tangan, berpotensi bug kasus batas. Ada alternatif yang telah diaudit baik seperti crate `base64` di ekosistem Rust.
**Saran:** Ganti dengan crate `base64`, mengurangi beban pemeliharaan dan potensi bug.

#### 10. [Rendah] `RandomBalancer` tidak acak
**File:** `ecat-client/src/lib.rs:91-105`
**Masalah:** Menggunakan hash `Instant::now()` sebagai sumber acak. Panggilan simultan dalam instance yang sama akan mendapatkan pilihan "acak" yang sama. `checked_add(0)` adalah operasi yang berlebihan.
**Saran:** Gunakan crate `rand` atau setidaknya `std::collections::hash_map::RandomState`.

#### 11. [Rendah] `Arc<Vec<String>>` yang tidak perlu di `ecat-data-sqlx`
**File:** `ecat-data-sqlx/src/lib.rs:79-87, 197-203`
**Masalah:** Nama kolom dibungkus dalam `Arc<Vec<String>>`, tetapi setiap konstruktor `Row` mengklon seluruh daftar nama kolom (`(*cols).clone()`). `Arc` hanya digunakan sekali selama iterasi, cukup gunakan `Rc` atau langsung `clone()`.
**Saran:** Di `query()` dan `query_with()`, ganti `Arc<Vec<String>>` dengan `Vec<String>` biasa. Biaya klon per baris sama dengan dereference melalui Arc + klon.

### Desain/Arsitektur

#### 12. [Informasi] QuestDB menggunakan GET + parameter kueri
**File:** `ecat-data-questdb/src/lib.rs:76, 91`
**Masalah:** SQL dikirim melalui parameter kueri GET, dibatasi panjang URL (biasanya ~2000-8000 karakter). Kueri besar akan terpotong.
**Saran:** Ubah menjadi POST + body, atau pertahankan GET untuk kueri sederhana, gunakan POST untuk kueri kompleks.

#### 13. [Informasi] `#[allow(dead_code)]` tersebar di berbagai tempat
**File:** `ecat-registry-consul/src/lib.rs:225`, `ecat-data-memcached/src/lib.rs:25-28`, `ecat-auth/src/lib.rs:52`
**Masalah:** Kolom username/password disimpan di memori tetapi ditandai dead_code (tidak diperlukan di memcached in-memory; varian RSA di auth belum diimplementasikan).
**Saran:** Implementasikan jalur fungsi yang hilang, atau hapus kolom tersebut, atau tambahkan dokumentasi menjelaskan mengapa dipertahankan.

#### 14. [Informasi] Sebagian klien HTTP kekurangan header Content-Type
**File:** `ecat-data-influxdb/src/lib.rs:96-103`, `ecat-data-clickhouse/src/lib.rs:87-89`
**Masalah:** Sebagian permintaan POST tidak mengatur header `Content-Type`, bergantung pada deteksi otomatis server.
**Saran:** Selalu atur Content-Type eksplisit untuk memastikan kompatibilitas.

#### 15. [Informasi] `ecat-auth` melebihi 500 baris
**File:** `ecat-auth/src/lib.rs` (540 baris)
**Masalah:** CLAUDE.md mensyaratkan file tetap di bawah 500 baris. Crate auth adalah satu-satunya file yang melebihi batas ini.
**Saran:** Pecah logika validasi JWT ke `ecat-auth/src/jwt.rs`, atau pecah berdasarkan fungsi.

## Peluang Optimasi (Bukan Bug)

| # | Lokasi | Saran |
|---|------|------|
| O1 | Semua crate data-* | Pola pembangunan klien TLS berulang di semua `from_config()` dapat diekstrak ke makro atau fungsi bersama |
| O2 | `ecat-data-sqlx` | Logika konversi tipe baris di `query()` dan `query_with()` (117 baris duplikat) dapat diekstrak ke fungsi bantu |
| O3 | `ecat-client` | `HttpClient::get()` dan `post()` berbagi pipeline "resolve → pick → build URL" yang sama — dapat diekstrak |
| O4 | `ecat-data` | Tipe error kustom dari semua 5 traits (Rdbms/Cache/Graph/Search/Tsdb) dapat disatukan menjadi satu enum `DataError` |
| O5 | `ecat-data-redis` | `self.conn.clone()` di setiap metode tidak perlu — `MultiplexedConnection` memang dirancang untuk `Clone` agar mendukung berbagi |

## Ringkasan Metrik

| Metrik | Nilai |
|------|------|
| Total crate | 50 |
| Total baris file sumber Rust | 7,968 |
| `expect()` di kode non-tes | 12 |
| `unwrap()` di kode non-tes | 0 |
| Blok `unsafe` | 0 |
| `panic!` di kode non-tes | 0 |
| `#[allow(dead_code)]` | 4 |
| TODO/FIXME/HACK | 0 |
| Mutex std di kode async | 1 (memcached) |

## Kesimpulan

Codebase berada dalam kondisi baik — kompilasi, pengujian, dan clippy semuanya lulus, tanpa kode unsafe, tanpa makro panic. Dua masalah paling kritis adalah **XOR "enkripsi"** (keamanan palsu) dan **implementasi default kueri parameterisasi membuang parameter secara diam-diam** (celah keamanan). Fungsionalitas routing header juga sepenuhnya tidak dapat digunakan. Masalah lainnya relatif kecil, termasuk dalam level pemeliharaan.

**Urutan perbaikan yang direkomendasikan:**
1. Implementasi default `execute_with`/`query_with` → kembalikan error daripada membuang parameter diam-diam
2. Enkripsi XOR → AEAD yang sebenarnya, atau ganti nama menjadi "obfuscation"
3. Routing versi header → implementasikan routing header yang sebenarnya
4. `from_config()` → kembalikan Result daripada expect-panic
5. Pemotongan TTL Redis → TTL sub-detik minimal gunakan 1 detik

## Status Perbaikan (R6 → R6.1)

| # | Masalah | Status | Perubahan |
|---|------|------|------|
| 1 | XOR "enkripsi" | Diperbaiki | `EncryptedSource` → `ObfuscatedSource`, `decrypt` → `deobfuscate`, prefiks `enc:` → `obfs:`, menambahkan dokumentasi menjelaskan ini obfuscation bukan enkripsi |
| 2 | `execute_with`/`query_with` membuang parameter diam-diam | Diperbaiki | Implementasi default diubah mengembalikan error `"parameterized ... not supported by this backend"` |
| 3 | Kata sandi plaintext di URL | Diperbaiki | Menggunakan `percent_encode()` untuk meng-encode kredensial di metode `connect_with_auth` |
| 4 | panic `expect()` TLS | Diperbaiki | `from_config()` dari 9 crate diubah mengembalikan `Result`, `RdbmsError` menambahkan varian `Config` |
| 5 | Routing header tidak efektif | Diperbaiki | Mengimplementasikan validasi versi dengan middleware `from_fn_with_state`, menambahkan tes `header_versioned_router_builds` |
| 6 | Pemotongan TTL Redis | Diperbaiki | `set_ex` → `pset_ex`, menggunakan presisi mili detik menghindari TTL sub-detik terpotong menjadi tidak pernah kedaluwarsa |
| 7 | Panic kontensi lock `StaticResolver` | Diperbaiki | `try_write()` → `blocking_write()` |
| 8 | `RandomBalancer` tidak acak | Diperbaiki | Mengganti hash `Instant::now()` dengan `RandomState::new().build_hasher()` |
| 9 | `std::sync::Mutex` dalam konteks async | Diperbaiki | Diganti `tokio::sync::Mutex` |
| 10 | base64 tulisan tangan | Diperbaiki | Diganti crate `base64` 0.22 |
| 11 | Overhead `Arc<Vec<String>>` | Diperbaiki | Diganti `Vec<String>` biasa, menghapus pembungkus Arc yang tidak perlu |
| 12 | QuestDB mengirim SQL dengan GET | Diperbaiki | Diubah POST + body, menambahkan header Content-Type |
| 13 | `#[allow(dead_code)]` | Diperbaiki | Kolom memcached ditambah prefiks `_`; kolom consul ditambah prefiks `_` dan menghapus allow; di auth `Rsa` → `RsaReserved` |
| 14 | Kekurangan Content-Type | Diperbaiki | Permintaan InfluxDB, ClickHouse, IoTDB menambahkan Content-Type eksplisit |
| 15 | `ecat-auth` melebihi 500 baris | Diperbaiki | Dipecah menjadi `claims.rs`(31) + `jwt.rs`(139) + `apikey.rs`(96) + `oauth2.rs`(173) + `helpers.rs`(28) + `lib.rs`(98) |

### Crate yang Terpengaruh

| Crate | Jenis perubahan |
|-------|----------|
| `ecat-data` | Implementasi default trait, varian `RdbmsError::Config` |
| `ecat-config` | `EncryptedSource` → `ObfuscatedSource` |
| `ecat-versioning` | Implementasi middleware routing header |
| `ecat-data-redis` | Presisi mili detik TTL, URL-encode kredensial |
| `ecat-data-sqlx` | URL-encode kredensial, menghapus overhead Arc |
| `ecat-data-clickhouse` | `from_config` → `Result`, header Content-Type |
| `ecat-data-questdb` | `from_config` → `Result`, GET → POST |
| `ecat-data-elasticsearch` | `from_config` → `Result` |
| `ecat-data-opensearch` | `from_config` → `Result` |
| `ecat-data-arangodb` | `from_config` → `Result` |
| `ecat-data-neo4j` | `from_config` → `Result` |
| `ecat-data-nebulagraph` | `from_config` → `Result` |
| `ecat-data-influxdb` | `from_config` → `Result`, header Content-Type |
| `ecat-data-iotdb` | `from_config` → `Result`, header Content-Type |
| `ecat-data-memcached` | `std::sync::Mutex` → `tokio::sync::Mutex`, pembersihan dead_code |
| `ecat-client` | Perbaikan `StaticResolver`, `RandomBalancer` |
| `ecat-registry-etcd` | base64 diganti crate |
| `ecat-registry-consul` | Pembersihan dead_code |
| `ecat-auth` | Dipecah menjadi 6 modul, pembersihan dead_code |

### Verifikasi Akhir (R6.2)

| Dimensi | Status |
|------|------|
| Build | Lulus, nol error nol warning |
| Test | Semuanya lulus, nol kegagalan |
| Clippy (`-D warnings`) | Lulus, nol peringatan |
| Ukuran file | Semua ≤ 300 baris |
