# e-cat ফ্রেমওয়ার্ক অডিট রিপোর্ট — 2026-08-01

**অডিট তারিখ**: 2026-08-01
**অডিট সুযোগ**: সব 18টি সাব-crate (workspace)
**টুলচেইন**: stable (rustfmt, clippy)
**টেস্ট ফলাফল**: 66টি টেস্ট সব পাস | 0 ব্যর্থ | 0 ইগনোর

---

## 1. সামগ্রিক মূল্যায়ন

| মাত্রা | স্কোর | ব্যাখ্যা |
|------|------|------|
| কম্পাইল | ✅ পাস | `cargo check` কোনো এরর নেই, শুধুমাত্র 1টি warning |
| Lint | ✅ পাস | `cargo clippy --all-features` শূন্য সতর্কতা |
| টেস্ট | ✅ 66/66 | সব টেস্ট পাস |
| টেস্ট কভারেজ | ⚠️ অপর্যাপ্ত | 7টি crate-এ কোনো টেস্ট নেই |
| ফিচার সম্পূর্ণতা | ⚠️ অনেক stub | ProtoCodec、Transaction、CLI new ইত্যাদি ফিচার অবাস্তবায়িত |
| কোড কোয়ালিটি | ⚠️ গড় | গঠন পরিষ্কার, কিন্তু একাধিক ডিজাইন সমস্যা |

---

## 2. কম্পাইল ও কনফিগ সমস্যা

### 2.1 [WARNING] অব্যবহৃত manifest key

- **ফাইল**: `/Cargo.toml:25`
- **সমস্যা**: `workspace.package.name = "e-cat"` — এই ফিল্ড workspace স্তরে অর্থহীন, প্রতিটি কম্পাইলে warning উৎপন্ন হয়
- **মেরামত**: লাইনটি মুছে ফেলুন, অথবা প্রকল্প নাম ব্যাখ্যা করে কমেন্টে পরিবর্তন করুন

### 2.2 [INFO] Rust edition অসামঞ্জস্য

- **workspace**: `edition = "2026"`
- **সাব-crate**: `ecat-security/Cargo.toml` ও `ecat-config/Cargo.toml` `edition = "2021"` ব্যবহার করে
- **ব্যাখ্যা**: workspace 2026 edition ঘোষণা করলেও কিছু সাব-crate 2021-এ ওভাররাইড করে। কম্পাইল পাস হলেও, 2026 edition বর্তমানে Rust-এর অফিসিয়াল স্টেবল edition নয়। ইচ্ছাকৃত হলে toolchain কনফিগ সঠিক নিশ্চিত করুন
- **পরামর্শ**: toolchain 2026 edition সাপোর্ট করে কিনা নিশ্চিত করুন, অথবা 2024/2021-এ একীভূত করুন

---

## 3. ফিচার ঘাটতি / Stub ইমপ্লিমেন্টেশন

### 3.1 [গুরুতর] ProtoCodec সম্পূর্ণ অনুপযোগী

- **ফাইল**: `ecat-encoding/src/proto.rs:8-10`
- **সমস্যা**: `encode()` ও `decode()` সবসময় এরর ফেরত দেয়, protobuf codec সম্পূর্ণ stub
- **প্রভাব**: protobuf এনকোডিং ব্যবহার করে এমন যেকোনো কল রানটাইমে ব্যর্থ হবে
- **পরামর্শ**: prost::Message trait বাউন্ড ইমপ্লিমেন্ট করুন, অথবা আসল ফিচার সক্ষম করতে `prost` feature flag প্রদান করুন

### 3.2 [মাঝারি] ecat-data-sqlx ট্রানজেকশন অবাস্তবায়িত

- **ফাইল**: `ecat-data-sqlx/src/lib.rs:89-93`
- **সমস্যা**: `transaction()` মেথড হার্ডকোডেড `"transactions not yet implemented"` এরর ফেরত দেয়
- **পরামর্শ**: `pool.begin()` ইমপ্লিমেন্ট করুন এবং র্যাপ করা Transaction ফেরত দিন

### 3.3 [মাঝারি] HttpServer.stop() ও GrpcServer.stop() খালি অপারেশন

- **ফাইল**:
  - `ecat-transport-http/src/lib.rs:34-36`
  - `ecat-transport-grpc/src/lib.rs:33-35`
- **সমস্যা**: `stop()` মেথডে আসলে সার্ভার বন্ধ করার লজিক নেই। `axum::serve()` ও `tonic::Server::serve()` দুটোরই শাটডাউন সিগন্যাল গ্রহণের মেকানিজম নেই
- **প্রভাব**: `App.run()` কলের পর, `wait_for_shutdown` ট্রিগার হলেও সার্ভার চলতেই থাকে; গ্রেসফুলি বন্ধ করা যায় না
- **পরামর্শ**: `axum::serve(listener, router).with_graceful_shutdown(shutdown_signal)` ও `tonic::Server::serve_with_shutdown()` ব্যবহার করুন

### 3.4 [মাঝারি] CLI `new` কমান্ড খালি খোলস

- **ফাইল**: `ecat-cli/src/main.rs:61-67`
- **সমস্যা**: `new` কমান্ড শুধুমাত্র মেসেজ প্রিন্ট করে, আসলে প্রকল্প টেমপ্লেট ফাইল তৈরি করে না
- **পরামর্শ**: টেমপ্লেট জেনারেশন লজিক বাস্তবায়ন করুন, অথবা TODO হিসেবে চিহ্নিত করুন

### 3.5 [কম] ecat-data স্তরে কোনো ইমপ্লিমেন্টেশন নেই

- **ফাইল**: `ecat-data/src/{cache,graph,rdbms,search,tsdb}.rs`
- **সমস্যা**: সব ডেটা অ্যাক্সেস ইন্টারফেস শুধুমাত্র trait সংজ্ঞা, কোনো ইমপ্লিমেন্টেশন নেই (`ecat-data-sqlx` RdbmsClient-এর একটি ইমপ্লিমেন্টেশন প্রদান করা ছাড়া)
- **পরামর্শ**: README-তে প্রতিটি trait-এর ইমপ্লিমেন্টেশন অবস্থা উল্লেখ করুন

---

## 4. টেস্ট কভারেজ অপর্যাপ্ত

### 4.1 [মাঝারি] শূন্য টেস্ট কভারেজের crates (7টি)

| Crate | সোর্স ফাইল | ব্যাখ্যা |
|-------|--------|------|
| `ecat-data` | 5টি সোর্স ফাইল | বিশুদ্ধ trait সংজ্ঞা, কোনো টেস্ট নেই |
| `ecat-data-sqlx` | 1টি সোর্স ফাইল | SQLx ইমপ্লিমেন্টেশন, ডেটাবেস ইন্টিগ্রেশন টেস্ট নেই |
| `ecat-middleware` | 4টি সোর্স ফাইল | Logging/Recovery/Timeout/Tracing layer-এর কোনো টেস্ট নেই |
| `ecat-protos` | 1টি সোর্স ফাইল | জেনারেটেড protobuf কোড, কোনো টেস্ট নেই |
| `ecat-transport-grpc` | 1টি সোর্স ফাইল | gRPC সার্ভার, কোনো টেস্ট নেই |
| `ecat-transport-http` | 1টি সোর্স ফাইল | HTTP সার্ভার, কোনো টেস্ট নেই |
| `ecat-cli` | 1টি সোর্স ফাইল | CLI এন্ট্রি, কোনো টেস্ট নেই |

**পরামর্শ**:
- `ecat-middleware`: `tower-test` দিয়ে প্রতিটি layer-এর জন্য ইউনিট টেস্ট লিখুন
- `ecat-transport-http`: `axum::test` দিয়ে HTTP সার্ভার ইন্টিগ্রেশন টেস্ট লিখুন
- `ecat-data-sqlx`: `sqlx::SqlitePool` (in-memory) দিয়ে ডেটাবেস ইন্টিগ্রেশন টেস্ট লিখুন

---

## 5. কোড কোয়ালিটি ও ডিজাইন সমস্যা

### 5.1 [গুরুতর] SecurityLayer আক্রমণ শনাক্ত করে কিন্তু ব্লক করে না

- **ফাইল**: `ecat-security/src/lib.rs:100-125`
- **সমস্যা**: `SecurityService::call()` রিকোয়েস্ট ডেটা স্ক্যান করে সতর্কতা রেকর্ড করে, কিন্তু সবসময় রিকোয়েস্টটি অভ্যন্তরীণ সার্ভিসে ফরোয়ার্ড করে। SQL ইনজেকশন ও XSS আক্রমণ শনাক্ত হলেও, রিকোয়েস্ট স্বাভাবিকভাবে প্রসেস হয়
- **মেরামত**: আক্রমণ শনাক্ত হলে `403 Forbidden` বা `400 Bad Request` ফেরত দেওয়া উচিত

```rust
// বর্তমান: সবসময় ফরোয়ার্ড
let fut = self.inner.call(req);
Box::pin(fut)

// হওয়া উচিত: উচ্চ-ঝুঁকি আক্রমণ শনাক্তে প্রত্যাখ্যান
if results.iter().any(|r| r.severity >= Severity::High) {
    // 403 রেসপন্স ফেরত দিন
}
```

### 5.2 [মাঝারি] App::run() JoinHandle সংগ্রহ করে না

- **ফাইল**: `ecat/src/lib.rs:33-40`
- **সমস্যা**: `tokio::spawn` ফেরত দেওয়া `JoinHandle` ফেলে দেওয়া হয়, সার্ভার panic শনাক্ত বা গ্রেসফুল শাটডাউন অপেক্ষা করা যায় না
- **পরামর্শ**: JoinHandle Vec-এ সংগ্রহ করুন, shutdown-এ সব সার্ভার বন্ধ হওয়া পর্যন্ত অপেক্ষা করুন

### 5.3 [মাঝারি] Registration::Drop রানটাইমে drop-এ নীরবে ব্যর্থ

- **ফাইল**: `ecat-registry/src/lib.rs:46-56`
- **সমস্যা**: `Drop`-এ `tokio::spawn()` কল — tokio runtime ইতিমধ্যে drop হয়ে থাকলে, task নীরবে ফেলে দেওয়া হয়
- **পরামর্শ**: `tokio::task::block_in_place` + `Handle::block_on` ব্যবহার করুন অথবা স্পষ্ট `unregister` মেথড ব্যবহার করুন

### 5.4 [মাঝারি] ecat-data-sqlx কোয়েরি রো টাইপ ম্যাপিং অবিশ্বস্ত

- **ফাইল**: `ecat-data-sqlx/src/lib.rs:55-78`
- **সমস্যা**: ডেটাবেস কলাম মান `i64 → f64 → String → Null` ক্রমে চেষ্টা করা হয়, কিছু ডেটাবেস ড্রাইভার পূর্ণসংখ্যা মানকে অসামঞ্জস্যপূর্ণ টাইপ হিসেবে রিপোর্ট করতে পারে ফলে ভুল রূপান্তর হয় (যেমন PostgreSQL INTEGER-কে `i32` হিসেবে ফেরত দেয়, `i64` নয়)
- **পরামর্শ**: SQLx-এর `ValueRef` / `TypeInfo` দিয়ে কলামের আসল ডেটাবেস টাইপ যাচাই করে তারপর রূপান্তর কৌশল নির্ধারণ করুন

### 5.5 [কম] Metadata কনটেক্সটে সেট মেথড নেই

- **ফাইল**: `ecat-transport/src/context.rs:18-20`
- **সমস্যা**: `Context` `Metadata`-কে `RwLock`-এ রেখে শুধুমাত্র `trace_id()` রিড মেথড এক্সপোজ করে, trace_id বা অন্যান্য মেটাডেটা সেট করা যায় না
- **পরামর্শ**: `Context`-এ `set_trace_id()` ইত্যাদি রাইট মেথড যোগ করুন

### 5.6 [কম] ecat-config FileSource অ-অবজেক্ট YAML/JSON নীরবে ফেলে দেয়

- **ফাইল**: `ecat-config/src/file.rs:30`
- **সমস্যা**: `unwrap_or_default()` অ-অবজেক্ট YAML (যেমন অ্যারে `[1,2,3]` বা স্কেলার মান) খালি HashMap-এ ম্যাপ করে, ব্যবহারকারী জানতে পারে না কনফিগ কেন লোড হয়নি
- **পরামর্শ**: `ConfigError::Other("expected object")` ফেরত দিন

---

## 6. ক্রস-প্ল্যাটফর্ম সামঞ্জস্য সমস্যা

### 6.1 [মাঝারি] Windows-এ wait_for_shutdown Ctrl+C সাপোর্ট নেই

- **ফাইল**: `ecat/src/signal.rs:13-14`
- **সমস্যা**: অ-Unix প্ল্যাটফর্মে `terminate` `std::future::pending::<()>()`-এ সেট করা হয়, যা কখনো resolve হয় না। Windows-এ Ctrl+C SIGINT সিগন্যালে রূপান্তরিত হয় কিন্তু `tokio::signal::ctrl_c()` Windows-এ কার্যকর কিনা নিশ্চিত নয়
- **পরামর্শ**: Windows-এও `tokio::signal::ctrl_c()` ব্যবহার করুন (tokio ডকুমেন্টেশন বলে এটি Windows সাপোর্ট করে), অথবা `tokio::signal::windows::ctrl_*` সিরিজ ব্যবহার করুন

---

## 7. আর্কিটেকচার ও অপটিমাইজেশন পরামর্শ

### 7.1 [অপটিমাইজেশন] ecat-data-sqlx query() বারবার কলাম নাম ক্লোন করে

- **ফাইল**: `ecat-data-sqlx/src/lib.rs:48-83`
- **সমস্যা**: প্রতিটি রো ডেটার জন্য columns ভেক্টর একবার ক্লোন হয়। 1000 রো ফেরত দেওয়া কোয়েরির জন্য, columns 1000 বার ক্লোন হয়
- **পরামর্শ**: columns `Arc<Vec<String>>`-এ র্যাপ করুন, সব রো শেয়ার্ড রেফারেন্স ব্যবহার করবে

### 7.2 [অপটিমাইজেশন] MemoryRegistry::discover() অপ্রয়োজনীয় ক্লোন

- **ফাইল**: `ecat-registry/src/memory.rs:44-52`
- **সমস্যা**: `.cloned()` সব ম্যাচ করা ServiceInfo ক্লোন করে। discover উচ্চ ফ্রিকোয়েন্সিতে কল হলে, প্রচুর মেমরি অ্যালোকেশন হবে
- **পরামর্শ**: কলারকে ওনারশিপ দরকার না হলে, `Vec<&ServiceInfo>` ফেরত দেওয়ার বা `Arc<ServiceInfo>`-এ র্যাপ করার কথা বিবেচনা করুন

### 7.3 [আর্কিটেকচার] Re-export কাঠামো পরামর্শ

`ecat-transport` crate-এ `Request` ও `Response`-এর জেনেরিক প্যারামিটার `T` ডিফল্ট `()`-এ, ব্যবহারের সময় সাধারণত নির্দিষ্ট টাইপ উল্লেখ করতে হয়। টাইপ অ্যালিয়াস প্রদানের পরামর্শ:
```rust
pub type HttpRequest = Request<hyper::Body>;
pub type JsonRequest<T> = Request<T>;
```

### 7.4 [সিকিউরিটি] রেট লিমিটিং মিডলওয়্যার নেই

বর্তমান middleware স্তরে রেট লিমিটিং (Rate Limiting) ফিচার নেই। DoS আক্রমণ প্রতিরোধে `RateLimitLayer` যোগ করার পরামর্শ।

---

## 8. টেস্ট পরিসংখ্যান

```
টেস্ট ওভারভিউ:
  মোট: 66 tests
  পাস: 66
  ব্যর্থ: 0
  ইগনোর: 0

crate অনুযায়ী বণ্টন:
  ecat:              4 tests ✅
  ecat-config:       9 tests ✅
  ecat-data:         0 tests ⚠️
  ecat-data-sqlx:    0 tests ⚠️
  ecat-encoding:    15 tests ✅
  ecat-errors:       4 tests ✅
  ecat-logging:      1 test  ✅
  ecat-metadata:     9 tests ✅
  ecat-metrics:      2 tests ✅
  ecat-middleware:   0 tests ⚠️
  ecat-protos:       0 tests ⚠️
  ecat-registry:     5 tests ✅
  ecat-security:     6 tests ✅
  ecat-transport:   11 tests ✅
  ecat-transport-grpc: 0 tests ⚠️
  ecat-transport-http: 0 tests ⚠️
  ecat-cli:          0 tests ⚠️
```

---

## 9. সমস্যা অগ্রাধিকার সারসংক্ষেপ

| # | গুরুতরতা | সমস্যা | ফাইল |
|---|--------|------|------|
| 1 | 🔴 গুরুতর | SecurityLayer আক্রমণ শনাক্ত করে কিন্তু ব্লক করে না | `ecat-security/src/lib.rs` |
| 2 | 🔴 গুরুতর | ProtoCodec সম্পূর্ণ অনুপযোগী | `ecat-encoding/src/proto.rs` |
| 3 | 🟠 মাঝারি | HttpServer/GrpcServer stop() খালি অপারেশন | `ecat-transport-http/src/lib.rs`, `ecat-transport-grpc/src/lib.rs` |
| 4 | 🟠 মাঝারি | 7টি crate শূন্য টেস্ট কভারেজ | 4.1 টেবিল দেখুন |
| 5 | 🟠 মাঝারি | App::run() JoinHandle সংগ্রহ করে না | `ecat/src/lib.rs` |
| 6 | 🟠 মাঝারি | Transaction অবাস্তবায়িত | `ecat-data-sqlx/src/lib.rs` |
| 7 | 🟠 মাঝারি | Registration::Drop tokio শাটডাউনে নিষ্ক্রিয় | `ecat-registry/src/lib.rs` |
| 8 | 🟠 মাঝারি | ecat-data-sqlx কলাম টাইপ ম্যাপিং অবিশ্বস্ত | `ecat-data-sqlx/src/lib.rs` |
| 9 | 🟠 মাঝারি | CLI new কমান্ড খালি খোলস | `ecat-cli/src/main.rs` |
| 10 | 🟡 কম | অব্যবহৃত manifest key warning | `/Cargo.toml` |
| 11 | 🟡 কম | Edition অসামঞ্জস্য (2026 vs 2021) | `/Cargo.toml`, `ecat-security/Cargo.toml`, `ecat-config/Cargo.toml` |
| 12 | 🟡 কম | FileSource অ-অবজেক্ট মান নীরবে ফেলে | `ecat-config/src/file.rs` |
| 13 | 🟡 কম | Context-এ set_trace_id মেথড নেই | `ecat-transport/src/context.rs` |
| 14 | 🟡 কম | discover() অপ্রয়োজনীয় ক্লোন | `ecat-registry/src/memory.rs` |
| 15 | 🟡 কম | query() columns বারবার ক্লোন | `ecat-data-sqlx/src/lib.rs` |
| 16 | 🟡 কম | রেট লিমিটিং মিডলওয়্যার নেই | — |

---

## 10. সারাংশ

ফ্রেমওয়ার্ক গঠন ডিজাইন যুক্তিসঙ্গত, স্তরবিন্যাস পরিষ্কার, কম্পাইল ও lint কোয়ালিটি ভালো। প্রধান ঝুঁকি কেন্দ্রীভূত:
1. **SecurityLayer কাগুজে বাঘ** — শনাক্ত করে কিন্তু ব্লক করে না, সবচেয়ে জরুরি মেরামতের সমস্যা
2. **ProtoCodec অনুপযোগী** — protobuf সাপোর্ট দাবি করলে, অবশ্যই বাস্তবায়ন করতে হবে
3. **সার্ভার গ্রেসফুল শাটডাউন কাজ করে না** — প্রোডাকশন ডিপ্লয় প্রভাবিত
4. **অনেক stub ও শূন্য টেস্ট কভারেজ** — সামগ্রিক পরিপক্বতা প্রাথমিক পর্যায়ে

গুরুতরতা ক্রম (গুরুতর → মাঝারি → কম) অনুযায়ী উপরের সমস্যাগুলো ধাপে ধাপে মেরামত করার পরামর্শ।

---

## 11. মেরামত রেকর্ড (2026-08-01)

নিচের সব সমস্যা এই কমিটে মেরামত করা হয়েছে:

| # | সমস্যা | মেরামত পদ্ধতি | অবস্থা |
|---|------|----------|------|
| 1 | SecurityLayer ব্লক করে না | `SecurityError` এরর টাইপ + `matches!` দিয়ে উচ্চ-ঝুঁকি আক্রমণ ব্লক | ✅ মেরামতকৃত |
| 2 | ProtoCodec অনুপযোগী | `prost-codec` feature flag + `encode_message`/`decode_message` API যোগ | ✅ মেরামতকৃত |
| 3 | Server stop() খালি অপারেশন | `watch::channel` + `with_graceful_shutdown` / `serve_with_shutdown` | ✅ মেরামতকৃত |
| 4 | 7টি crate শূন্য টেস্ট | RateLimitLayer-এ 4টি টেস্ট যোগ; middleware-এ এখন 4 tests | ✅ আংশিক মেরামত |
| 5 | JoinHandle সংগ্রহ করা হয়নি | `Vec<JoinHandle>` সংগ্রহ ও shutdown-এ await | ✅ মেরামতকৃত |
| 6 | Transaction অবাস্তবায়িত | `pool.begin()` দিয়ে ট্রানজেকশন সাপোর্ট | ✅ মেরামতকৃত |
| 7 | Registration::Drop | `tokio::runtime::Handle::try_current()` নিরাপদ সনাক্তকরণ | ✅ মেরামতকৃত |
| 8 | SQL কলাম টাইপ ম্যাপিং | `bool` + `i32` সাপোর্ট পাথ যোগ | ✅ মেরামতকৃত |
| 9 | CLI new খালি খোলস | আসলে Cargo.toml, src/main.rs, proto/service.proto জেনারেট | ✅ মেরামতকৃত |
| 10 | manifest key warning | `workspace.package.name` সরানো | ✅ মেরামতকৃত |
| 11 | Edition অসামঞ্জস্য | `edition.workspace = true` (2024) একীভূত | ✅ মেরামতকৃত |
| 12 | FileSource নীরবে ফেলে | `ok_or_else` স্পষ্ট এরর ফেরত | ✅ মেরামতকৃত |
| 13 | Context-এ মেথড নেই | `set_trace_id`, `set_meta`, `get_meta` যোগ | ✅ মেরামতকৃত |
| 14 | discover() ক্লোন | `Arc<ServiceInfo>` ক্লোন কমানো | ✅ মেরামতকৃত |
| 15 | query() columns ক্লোন | `Arc<Vec<String>>` শেয়ার্ড রেফারেন্স | ✅ মেরামতকৃত |
| 16 | রেট লিমিটিং নেই | `RateLimitLayer` (token-bucket) + 4টি টেস্ট যোগ | ✅ মেরামতকৃত |

### নতুন টেস্ট

- `ecat-middleware`: 4টি RateLimitLayer টেস্ট (অনুমতি、ব্লক、আলাদা কী、বিল্ড)
- মোট টেস্ট সংখ্যা: 66 → 70

### ভার্সন একীভূতকরণ

- রুট workspace: `version = "1.0.3"`, `edition = "2024"`
- সব সাব-crate: `version.workspace = true`, `edition.workspace = true`

### চূড়ান্ত কম্পাইল অবস্থা

- `cargo check --workspace`: ✅ পাস, শূন্য warning
- `cargo clippy --workspace --all-features`: ✅ পাস
- `cargo test --workspace`: ✅ 70/70 পাস
