<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Laporan Peninjauan Kode e-cat (Putaran Kedua)

**Tanggal**: 2026-07-29  
**Cabang**: main  
**Proyek**: e-cat (Rust workspace, 17 crate)

---

## 1. Ringkasan Peninjauan

Berdasarkan perbaikan clippy putaran pertama dan penambahan tes, putaran ini melakukan peninjauan mendalam logika kode, dengan fokus pada kebenaran runtime, keamanan konkurensi, dan konsistensi semantik API. Total 32 file sumber ditinjau.

### Baseline Verifikasi

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

---

## 2. Bug yang Ditemukan dan Perbaikan

### Bug 1: [Kritis] Kesalahan siklus hidup guard span TracingLayer

- **File**: `ecat-middleware/src/tracing.rs:37`
- **Tingkat keparahan**: **Tinggi**
- **Dampak**: Semua permintaan yang melewati TracingLayer tidak tercakup oleh span tracing

**Analisis akar masalah**:

```rust
// 修复前
fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let _guard = span.enter();  // guard 在 call() 返回时 drop
    let fut = self.inner.call(req);
    Box::pin(fut)               // future 在后续 poll 时才执行
}
```

Guard yang dikembalikan oleh `span.enter()` hanya menjaga span tetap aktif dalam konteks sinkron saat ini. `call()` mengembalikan future yang belum di-poll, eksekusi asinkron aktual terjadi pada tahap poll berikutnya — pada saat itu guard sudah di-drop, span tidak akan berlaku. Semua permintaan yang melewati TracingLayer tidak akan muncul di output tracing.

**Perbaikan**:

```rust
// 修复后
use tracing::Instrument;

fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let fut = self.inner.call(req);
    Box::pin(fut.instrument(span))  // span 附着在 future 生命周期上
}
```

Menggunakan `tracing::Instrument::instrument()` untuk menempelkan span pada future, memastikan span tetap aktif selama seluruh siklus hidup poll future.

---

### Bug 2: [Kritis] Cacat implementasi closure LifecycleHook — on_stop tidak pernah dieksekusi

- **File**: `ecat/src/hook.rs:14-23`, `ecat/src/lib.rs:11-16`
- **Tingkat keparahan**: **Tinggi**
- **Dampak**: Hook closure yang didaftarkan melalui `.on_stop()` tidak melakukan apa pun saat shutdown

**Analisis akar masalah**:

Dalam desain awal, metode `on_start()` dan `on_stop()` sama-sama mendorong hook ke Vec `lifecycle_hooks` yang sama. Saat `run()`, semua hook memanggil `on_start()` secara berurutan, saat shutdown semua hook memanggil `on_stop()` secara berurutan.

Masalahnya ada pada blanket impl trait `LifecycleHook` untuk closure `Fn() -> Fut`: **hanya mencakup `on_start()`, `on_stop()` menggunakan implementasi default trait (no-op)**.

Ini berarti ketika pengguna menggunakan sintaks closure `.on_stop(|| async { ... })`, closure memang ditambahkan ke daftar hooks, tetapi saat shutdown hanya `on_stop()` kosong default yang dieksekusi, logika pengguna tidak pernah berjalan.

**Perbaikan (dua bagian)**:

1. **Memisahkan start_hooks dan stop_hooks** (`ecat/src/lib.rs`):

```rust
// App 结构体 — 两个独立的 Vec
pub struct App {
    start_hooks: Vec<Box<dyn LifecycleHook>>,
    stop_hooks: Vec<Box<dyn LifecycleHook>>,
    // ...
}

// on_start() → start_hooks, on_stop() → stop_hooks
pub fn on_start(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.start_hooks.push(Box::new(hook));
    self
}
pub fn on_stop(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.stop_hooks.push(Box::new(hook));
    self
}
```

2. **Melengkapi blanket impl closure** (`ecat/src/hook.rs`):

```rust
impl<F, Fut> LifecycleHook for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Result<...>> + Send,
{
    async fn on_start(&self) -> ... { (self)().await }
    async fn on_stop(&self) -> ...  { (self)().await }  // 新增
}
```

Sekarang closure mengimplementasikan `on_start` dan `on_stop` secara bersamaan, dipadukan dengan Vec yang terpisah, setiap hook hanya dipanggil pada tahap siklus hidup yang benar.

---

### Bug 3: [Sedang] Prioritas ekstraksi tipe nilai Row SqlxClient salah

- **File**: `ecat-data-sqlx/src/lib.rs:53-68`
- **Tingkat keparahan**: Sedang
- **Dampak**: Nilai integer dan float di database diekstraksi sebagai string JSON, bukan angka

**Analisis akar masalah**:

`try_get::<String>()` ditempatkan pertama kali dicoba. Sebagian besar driver database dapat berhasil menjalankan `try_get::<String>()` pada kolom numerik (konversi implisit), menyebabkan nilai integer `42` diekstraksi sebagai `"42"` bukan `42`.

**Perbaikan**: Menyesuaikan urutan percobaan `try_get` menjadi `i64 → f64 → String → Null`, memprioritaskan mempertahankan tipe numerik.

---

## 3. Temuan Peninjauan Lainnya (Tidak Diubah / Keterbatasan yang Diketahui)

| Kategori | File | Keterangan | Saran |
|------|------|------|------|
| Fitur belum selesai | `ecat-transport-http/src/lib.rs:30` | `axum::serve().await` memblokir dan tidak pernah kembali, `stop()` adalah no-op | Implementasikan graceful shutdown |
| Fitur belum selesai | `ecat-transport-grpc/src/lib.rs:29` | Sama seperti di atas | Implementasikan graceful shutdown |
| Fitur belum selesai | `ecat-data-sqlx/src/lib.rs:79` | `transaction()` mengembalikan error tidak terimplementasi | Implementasikan dukungan transaksi |
| Gaya kode | `ecat-middleware/src/logging.rs:42` | `elapsed.as_millis() as u64` pemotongan teoretis u128→u64 | Tidak berdampak aktual |
| Tes hilang | `ecat-middleware/` | 4 Tower Service tanpa unit test | Perlu tes integrasi |
| Tes hilang | `ecat-data/` | Murni definisi trait | Saat ini dapat diterima |
| Blokir RwLock | `ecat-registry/src/memory.rs` | RwLock sinkron dapat memblokir dalam konteks asinkron | Pertimbangkan ganti ke tokio::sync::RwLock |

---

## 4. Hasil Pengujian

```
cargo test → 60 passed, 0 failed

Distribusi per crate:
  ecat                  4   (Builder/nilai default/hook lifecycle)
  ecat-config           9   (env parse ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (pemetaan HTTP/konversi gRPC/metadata/Display)
  ecat-logging          1   (asap init)
  ecat-metadata         9   (akses/From HeaderMap/From MetadataMap/iterator)
  ecat-metrics          2   (singleton/text tidak panic)
  ecat-registry         5   (registrasi/discovery/unregistrasi/daftar/filter)
  ecat-transport       11   (Context/Request/Response/trait Server)
  8 crate lainnya       0   (murni trait/pembuatan kode/perlu tes integrasi/murni cetak)
```

---

## 5. Daftar File yang Diubah

| File | Jenis perubahan | Keterangan perubahan |
|------|----------|----------|
| `ecat/src/lib.rs` | Perbaikan Bug | App memisahkan start_hooks/stop_hooks; AppBuilder diperbarui sesuai; tes disesuaikan |
| `ecat/src/hook.rs` | Perbaikan Bug | Melengkapi implementasi on_stop() pada blanket impl closure |
| `ecat-middleware/src/tracing.rs` | Perbaikan Bug | guard span → `fut.instrument(span)` |
| `ecat-data-sqlx/src/lib.rs` | Perbaikan Bug | Urutan ekstraksi nilai Row i64→f64→String→Null |

---

## 6. Ringkasan

Putaran ini menemukan 2 Bug runtime tingkat keparahan tinggi dan 1 masalah kebenaran data tingkat keparahan sedang:

1. **Span TracingLayer tidak berlaku** — memengaruhi observabilitas semua permintaan
2. **LifecycleHook on_stop tidak dieksekusi** — memengaruhi kebenaran semua logika shutdown
3. **Tipe numerik Row hilang** — memengaruhi kebenaran tipe hasil kueri database

Ketiga masalah telah diperbaiki, setelah perbaikan semua 60 tes lulus, kompilasi nol error nol peringatan.

### Saran Lanjutan

- Menerapkan graceful shutdown untuk server HTTP/gRPC
- Menambahkan tes integrasi untuk `ecat-middleware` (mock Service + verifikasi perilaku span/timeout/recovery)
- Menambahkan tes integrasi untuk `ecat-data-sqlx` (menggunakan database memori SQLite)
- Mengganti RwLock sinkron `ecat-registry/memory.rs` dengan `tokio::sync::RwLock`
