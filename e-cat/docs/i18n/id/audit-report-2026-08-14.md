# Laporan Audit Khusus (Keamanan dan Kinerja) — 2026-08-14

Ruang lingkup audit: workspace 55 crate (v2.3.5). Metode: pemeriksaan manual Cargo.lock (cargo-audit tidak diinstal), audit sumber jalur autentikasi/TLS, pemeriksaan siklus hidup konkurensi dan sumber daya. Tidak ada kode yang dikomit.

## Pemeriksaan CVE Dependensi

- Versi dependensi inti semuanya relatif baru dan tanpa CVE tidak diperbaiki yang diketahui: rustls 0.23.43, ring 0.17.14, aws-lc-rs 1.17.3, jsonwebtoken 9.3.1, tokio 1.53.1, h2 0.4.15, quinn 0.11.11, sqlx 0.8.6, zerocopy 0.8.55, time 0.3.54, openssl 0.10.81.
- hyper 0.14.32 (hanya dari rust-s3 0.35.1, melalui hyper-tls 0.5) sudah di atas garis perbaikan 0.14.28.
- Catatan: CI tidak menginstal cargo-audit, disarankan menambahkan ke alur kerja untuk pemeriksaan otomatis.

## Temuan (diurutkan berdasarkan keparahan)

### S1 [Sedang] Handshake TLS HTTP diserialkan → DoS handshake lambat
- Lokasi: `ecat-transport-http/src/lib.rs:134-150` (TlsListener::accept)
- Gejala: handshake TLS diselesaikan secara sinkron di dalam `accept()`, axum::serve memanggil accept secara serial — satu koneksi yang tidak menyelesaikan handshake memblokir seluruh loop accept.
- Dampak: penyerang membuat banyak koneksi TCP lambat/zombie secara massal sehingga layanan sepenuhnya berhenti menerima koneksi baru (di sisi gRPC tonic melakukan spawn handshake per koneksi, tidak terpengaruh).
- Saran: setelah accept `tokio::spawn` handshake dan tambahkan `tokio::time::timeout(10s)`, tutup koneksi saat gagal.

### S2 [Sedang] Cache introspection OAuth2 tumbuh tanpa batas → DoS memori
- Lokasi: `ecat-auth/src/oauth2.rs:45,84-92`
- Gejala: `HashMap<String,(String,Instant)>` dengan token sebagai kunci, TTL hanya mengontrol kesegaran, tanpa batas kapasitas, tanpa eviction.
- Dampak: permintaan token unik dalam jumlah besar dapat menumbuhkan memori tanpa batas (setiap miss juga memicu introspection upstream).
- Saran: tambahkan batas kapasitas (mis. 10k) + pembersihan berkala, atau ganti moka/LRU dengan eviction kapasitas dan TTL.

### S3 [Rendah-sedang] ecat-data-s3 menggunakan rust-s3 0.35.1 lama (hyper 0.14 + native-tls/openssl)
- Lokasi: `ecat-data-s3/Cargo.toml` → rust-s3 0.35.1
- Gejala: klien S3 secara independen menggunakan stack hyper-tls/openssl, `ecat-tls::TlsClientConfig` (CA kustom, sertifikat klien, skip_verify) tidak berlaku untuk S3; permukaan konfigurasi TLS tidak konsisten.
- Dampak: CA privat S3/mTLS di lingkungan enterprise tidak dapat dikonfigurasi; pemeliharaan lambat setelah 2023.
- Saran: evaluasi upgrade rust-s3 atau ganti ke klien reqwest/rustls yang seragam.

### S4 [Rendah] Validasi default JWT tidak menyertakan iss/aud
- Lokasi: `ecat-auth/src/jwt.rs:125` — `Validation::new(HS256)` hanya signature+exp.
- Dampak: dengan kunci bersama HS256, token satu layanan dapat diterima layanan lain (tanpa isolasi issuer).
- Saran: dokumentasikan eksplisit bahwa produksi wajib mengonfigurasi issuer/audience; atau tambahkan entri validasi iss secara default.

### S5 [Rendah] TlsClientConfig.skip_verify sendirian membuat is_enabled() bernilai benar
- Lokasi: `ecat-tls/src/lib.rs:23-29`
- Gejala: hanya mengonfigurasi `skip_verify: true` membuat TLS dianggap "aktif" dan tidak memverifikasi sertifikat, mematikan verifikasi secara diam-diam.
- Saran: skip_verify dan ca_cert validasi saling eksklusif, atau wajibkan konfirmasi ganda eksplisit.

## Kinerja dan Sumber Daya

### P1 [Rendah] Jalur hit cache OAuth2 melakukan deserialisasi JSON setiap permintaan
- Lokasi: `ecat-auth/src/oauth2.rs:87` — cache menyimpan string serialisasi, setelah hit tetap `serde_json::from_str`.
- Saran: cache langsung menyimpan struct `AuthClaims`, menghemat parse per permintaan.

### P2 [Rendah] ecat-bench tanpa warmup dan penilaian steady-state
- Lokasi: `ecat-bench/src/lib.rs:run_bench` — langsung timing, tanpa warmup, cold start/alokasi pertama connection pool tercampur ke p99.
- Saran: tambahkan putaran warmup dan penilaian konvergensi steady-state agar hasil lebih dapat dipercaya.

### P3 [Rendah] Konsumen Kafka poll 100ms + sleep 100ms serial
- Lokasi: `ecat-mq-kafka/src/lib.rs:84-92` — batas atas latensi end-to-end pesan sekitar 200ms.
- Saran: setelah poll tidak perlu sleep lagi; skenario throughput rendah dapat memperpendek interval poll.

## Konfirmasi Praktik Baik

- Jalur produksi tanpa panic unwrap/expect (transport/auth/middleware hanya dalam tes).
- Fallback parameter query API key disertai log peringatan kebocoran; HashMap menggunakan SipHash untuk mencegah tabrakan.
- Lapisan SQL meneruskan SQL pemanggil (sifat framework), user:pass string koneksi di-encode persen dengan benar.
- Saat channel konsumsi Kafka penuh memblokir backpressure alih-alih membuang; setelah rx drop task poll keluar normal.
- Penarikan config-remote membawa timeout (5s/30s), kueri blocking tanpa index melaporkan error mencegah busy-wait.

---

## Audit Kebenaran Domain Inti (pelengkap, saling melengkapi dengan audit khusus keamanan/kinerja di atas)

Metode audit: pemindaian kode produksi seluruh workspace (lokasi unwrap/expect/panic, penelan error diam-diam, penghentian async, state konkurensi) + verifikasi ulang penuh `cargo test --workspace` (putaran pertama semua hijau; perbaikan S1 sedang berjalan menyebabkan peringatan kompilasi di transport-http, setelah selesai perlu dijalankan ulang). Tidak ada kode yang dikomit.

### N1 [Sedang] Handle bocor setelah task konsumsi ecat-events keluar → event hilang diam-diam
- Lokasi: `ecat-events/src/lib.rs:97-101` (loop konsumsi 89-95 baris `None => break`)
- Gejala: mq stream mengembalikan None (mis. channel broadcast kafka ditutup) atau task panic menyebabkan loop konsumsi keluar, tetapi JoinHandle di map `consumers` tertinggal; setelah itu `subscribe()` untuk tipe event yang sama karena `contains_key` di baris 68 selalu benar tidak lagi me-restart task konsumsi → event tipe tersebut hilang diam-diam selamanya.
- Dampak: setelah aliran event remote terputus tidak dapat self-heal, pemulihan memerlukan restart proses.
- Saran: jalur keluar task menghapus handle dari map (spawn watcher atau pembersihan lazy `handle.is_finished()`).

### N2 [Sedang] Semantik group_id `subscribe` ecat-mq-kafka salah
- Lokasi: `ecat-mq-kafka/src/lib.rs:71-84`
- a. Saat `group_id` default None, rdkafka `consumer.subscribe()` mensyaratkan group.id (librdkafka melaporkan INVALID_ARG), dengan konfigurasi default subscribe besar kemungkinan langsung gagal (perlu verifikasi di perangkat nyata).
- b. Saat group_id dikonfigurasi (ecat-events melakukan subscribe sekali per tipe event, group sama), Kafka membagi topic antar konsumen multi dalam group yang sama berdasarkan partisi → satu tipe event dapat jatuh ke task konsumsi tipe lain dan dibuang diam-diam (auto.offset.reset=latest dan tanpa commit).
- Dampak: event bus di bawah backend kafka kehilangan event.
- Saran: saat tanpa group_id buat group.id acak unik; atau sisi konsumen gunakan assign() untuk menetapkan partisi eksplisit; dokumentasikan eksplisit bahwa multi-subscribe wajib group terpisah.

### N3 [Rendah] Host kosong GrpcServer/WsServer tidak dinormalkan (perbaikan D1 tidak lengkap)
- Lokasi: `ecat-transport-grpc/src/lib.rs:52`, `ecat-transport-ws/src/lib.rs:58`
- Gejala: `addr.parse::<SocketAddr>()` pada `GrpcServer::new(":8000")` mengembalikan AddrParseError (sudah diverifikasi diuji); `TcpListener::bind(":8000")` pada WsServer di-resolve ke wildcard IPv6, lingkungan tanpa IPv6 gagal start. HttpServer sudah menormalkan 0.0.0.0, tiga server API berperilaku tidak konsisten.
- Saran: normalkan host kosong seragam di dalam new.

### N4 [Rendah] TracingLayer tidak menginjeksi trace_id, tidak sesuai deklarasi CHANGELOG 2.3.3
- Lokasi: `ecat-tracing/src/lib.rs:72-84` (span hanya berisi field service, komentar kode mengakui Req generik tidak dapat mengambil header); `inject_trace_id()` setiap kali membuat UUID baru, tidak melanjutkan trace_id yang diekstrak upstream.
- Dampak: tracing terdistribusi yang dikonfigurasi sesuai dokumentasi tidak dapat menghubungkan antar layanan.
- Saran: field span binding tertunda atau spesialisasi http::Request<B>; inject mendukung membawa id upstream.

### N5 [Rendah] panic job ecat-scheduler berhenti diam-diam
- Lokasi: `ecat-scheduler/src/lib.rs:53-57,83` (`let _ = handle.await` di `run()`)
- Gejala: setelah task terjadwal panic task mati, tanpa restart, tanpa log; `run()` membuang error JoinHandle.
- Saran: tangkap panic tulis log + strategi restart opsional.

### N6 [Rendah] unwrap tersisa di kode produksi (jalur poison/panic)
- `ecat-events/src/lib.rs:68,98` std `Mutex::lock().unwrap()` (poison langsung panic); `ecat-versioning/src/lib.rs:86` unwrap Response builder (tidak dapat gagal tetapi merupakan jalur panic); `ecat-mq/src/lib.rs:110` expect sudah dijaga is_none (aman).
- Saran: dua tempat events ganti `unwrap_or_else(|e| e.into_inner())`.

### N7 [Informasi] WsServer::stop() tidak menunggu koneksi WebSocket yang sudah di-upgrade
- Lokasi: `ecat-transport-ws/src/lib.rs:63-87`
- Koneksi on_upgrade axum berjalan di task terpisah, graceful shutdown tidak mencakupnya; handler koneksi panjang tetap bertahan setelah stop(), proses keluar tidak bersih (semantik App::stop tidak lengkap).

### N8 [Informasi] Crate nol tes: ecat-data / ecat-lock / ecat-protos
- Semuanya crate bertipe trait/definisi; sudah diverifikasi metode default fail-loud (mengembalikan error alih-alih diam), tetapi kontrak trait (semantik rollback drop Transaction, validasi token lock) tanpa unit test apa pun.
- Saran: tambahkan unit test minimal untuk semantik RdbmsError/Transaction dan DistributedLock.

### N9 [Informasi] Parameter graphql dan field nested masih dibuang
- `ecat-graphql/src/lib.rs` execute hanya meneruskan `variables` ke resolver, argumen field `{ hello(name: "x") }`, selection nested tidak diteruskan; README tidak menyebut keterbatasan ini (L8 laporan lama mewajibkan didokumentasikan, setelah penulisan ulang 2.3.3 masih belum dilengkapi).

### N10 [Informasi] circuit-breaker hanya menghitung error lapisan transport
- `ecat-circuit-breaker/src/lib.rs:203-209` hanya mencatat inner Err sebagai kegagalan, HTTP 5xx dianggap sukses → circuit breaker tidak efektif untuk layanan tidak tersedia (badai 5xx); dokumentasi tidak menjelaskan.

**Status verifikasi**: putaran pertama `cargo test --workspace` semua hijau (termasuk doc-tests, output ekor tidak melihat kegagalan apa pun); selama edit agent perbaikan S1 transport-http sempat muncul error kompilasi dan 2 peringatan (unused import `ensure_crypto_provider`, `shutdown_tx` tidak dibaca) — ini keadaan antara, setelah S1 selesai perlu menjalankan ulang penuh tes dan `clippy --all-targets -D warnings`.

---

## Putaran Ketiga: Validasi Dinamis + Pemeriksaan ulang CVE + permukaan panic (khusus, 2026-08-14)

### Pemeriksaan ulang CVE (temuan baru, diurutkan berdasarkan keparahan)

1. **[Sedang] rustls-webpki 0.102.8 tersisa di pohon dependensi** (RUSTSEC-2026-0049/0098/0099/0104: bypass distributionPoint CRL, URI/wildcard name-constraints, versi perbaikan 0.103.10). Rantai utama 0.103.13 (melalui rustls 0.23.43, aman); 0.102.8 diperkenalkan melalui async-nats 0.38.0 / rumqttc 0.25.1, mencakup rantai klien TLS NATS/MQTT. Upstream belum migrasi rustls 0.23, tanpa versi perbaikan — risiko terkendali, disarankan komentar pelacakan.
2. **[Sedang-rendah] rdkafka 0.36.2 librdkafka tertanam membawa cJSON 1.7.14** (CVE-2023-53154 dan seri cJSON; CVE-2025-57052 berlabel CVSS 9.8 tetapi file yang terpengaruh cJSON_utils.c tidak digunakan librdkafka, kesesuaian diragukan). Perbaikan upstream di librdkafka 2.10+ (2026-03 PR #5346). ecat-mq-kafka link statis, perlu mencocokkan versi paket librdkafka-sys dan melacak upgrade.
3. **[Rendah] rustls-pemfile 2.2.0 tidak terpelihara** (RUSTSEC-2025-0134) — ecat-transport-http mem-parse file lokal saat start, bukan input penyerang.
4. **[Rendah] rsa 0.9.10** (RUSTSEC-2023-0071 side-channel timing Marvin) — diperkenalkan melalui TLS sqlx-mysql, hanya relevan untuk skenario MySQL + pertukaran kunci RSA.
5. async-nats 0.38.0 sudah di atas garis perbaikan RUSTSEC-2023-0027 (bypass validasi CN), tidak ada masalah.

### Validasi dinamis (examples/helloworld, build debug, port sementara 18080, sudah dibersihkan)

- /health 200, / (serialisasi JSON) 200 (27B), 404 normal; middleware Logging mencatat permintaan normal.
- **/metrics terpasang tetapi mengembalikan 200 + body kosong (0 byte)**: tanpa registrasi metrik tidak ada output apa pun, sisi monitoring tidak dapat membedakan "sehat/tanpa metrik". Disarankan registry kosong mengeluarkan baris komentar atau 503.
- Permintaan malformed (header berisi 0x01/0x02) → 400 Bad Request, layanan tetap hidup, /health setelahnya tetap 200, tanpa panic.
- Jalur TLS/mTLS dan middleware circuit breaker/rate limit: dicakup oleh tes ecat-transport-http/grpc, ecat-middleware (setelah perbaikan race mTLS semua hijau, kasus menolak sertifikat klien anonim/salah lulus).

### Baseline bench

- ecat-bench tanpa target [[bench]]/bin, tanpa entri cargo bench; run_bench_with_warmup sudah membawa warmup (perbaikan P2 terwujud), tes harness semua hijau.
- Pengukuran nyata hanya smoke debug build: / sekitar 1.3ms, /health sekitar 1.8ms (termasuk overhead proses curl, tanpa makna baseline). Disarankan build release + uji beban wrk/hey untuk baseline nyata.

### Pemeriksaan ulang permukaan panic (seluruh workspace, tidak termasuk modul tes)

- Total 31 tempat unwrap/expect/panic, semuanya risiko rendah: `Response::builder().body().unwrap()` (cabang jwt/apikey/oauth2 tidak dapat gagal), fallback lock poison (etcd/testing), `serde_json::to_string().unwrap()` clickhouse (input NaN/inf ekstrem berpotensi panic teoretis).
- **1 tempat perlu diperhatikan**: `ecat-transport-http/src/tls_listener.rs:234` — saat loop accept latar belakang keluar abnormal `panic!` di dalam `accept()`, thread layanan mati (kondisi pemicu ketat: hanya error fatal listener), disarankan diturunkan menjadi pengembalian error dan tulis log.
