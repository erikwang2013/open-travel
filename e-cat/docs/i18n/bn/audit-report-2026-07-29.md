<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat কোড রিভিউ ও TDD টেস্ট রিপোর্ট

**তারিখ**: 2026-07-29  
**ব্রাঞ্চ**: main  
**প্রকল্প**: e-cat (Rust workspace, 17টি crate)

---

## এক、রিভিউ সুযোগ

workspace-এর 17টি crate-এর সব Rust সোর্স (38টি `.rs` ফাইল) রিভিউ করা হয়েছে।

| Crate | ব্যাখ্যা | ফাইল সংখ্যা |
|-------|------|--------|
| `ecat-protos` | Protobuf সংজ্ঞা ও কোড জেনারেশন | 2 |
| `ecat-errors` | ইউনিফাইড এরর টাইপ | 2 |
| `ecat-metadata` | রিকোয়েস্ট মেটাডেটা অ্যাবস্ট্রাকশন | 1 |
| `ecat-encoding` | JSON/Protobuf এনকোড-ডিকোড | 3 |
| `ecat-logging` | লগ/Tracing ইনিশিয়ালাইজেশন | 1 |
| `ecat-config` | কনফিগ লোডিং (ফাইল/এনভায়রনমেন্ট ভেরিয়েবল) | 3 |
| `ecat-data` | ডেটা স্তরের trait অ্যাবস্ট্রাকশন | 5 |
| `ecat-data-sqlx` | SQLx RDBMS ইমপ্লিমেন্টেশন | 1 |
| `ecat-registry` | সার্ভিস রেজিস্ট্রি ও ডিসকভারি | 2 |
| `ecat-metrics` | Prometheus মেট্রিক | 1 |
| `ecat-middleware` | Tower মিডলওয়্যার স্তর | 4 |
| `ecat-transport` | ট্রান্সপোর্ট স্তর অ্যাবস্ট্রাকশন | 4 |
| `ecat-transport-http` | HTTP/Axum ট্রান্সপোর্ট ইমপ্লিমেন্টেশন | 1 |
| `ecat-transport-grpc` | gRPC/Tonic ট্রান্সপোর্ট ইমপ্লিমেন্টেশন | 1 |
| `ecat` | অ্যাপ্লিকেশন ফ্রেমওয়ার্ক কোর | 3 |
| `ecat-cli` | CLI টুল | 1 |
| `examples/helloworld` | উদাহরণ প্রকল্প | 1 |

---

## দুই、আবিষ্কৃত সমস্যা ও মেরামত

### সমস্যা 1：[Clippy] `map_identity` — অর্থহীন identity map

- **ফাইল**: `ecat-config/src/file.rs:30`
- **গুরুতরতা**: কম
- **সমস্যা**: `map(|(k, v)| (k, v))` কোনো রূপান্তর করে না, এটি অকার্যকর কোড
- **মেরামত**: অপ্রয়োজনীয় `.map()` কল সরানো হয়েছে

### সমস্যা 2：[Clippy] `new_without_default` — Config-এ Default ইমপ্লিমেন্টেশন নেই

- **ফাইল**: `ecat-config/src/lib.rs:27`
- **গুরুতরতা**: কম
- **সমস্যা**: `Config`-এ `new()` মেথড আছে কিন্তু `Default` trait ইমপ্লিমেন্ট করা হয়নি
- **মেরামত**: ম্যানুয়াল ইমপ্লিমেন্টেশনের বদলে `#[derive(Default)]` ব্যবহার

### সমস্যা 3：[Clippy] `io_other_error` — পুরনো ধরনের Error কনস্ট্রাকশন

- **ফাইল**: `ecat-middleware/src/recovery.rs:42`
- **গুরুতরতা**: কম
- **সমস্যা**: `std::io::Error::new(std::io::ErrorKind::Other, ...)`-এর সহজ বিকল্প আছে
- **মেরামত**: `std::io::Error::other("task panicked")` ব্যবহার

### সমস্যা 4：[Clippy] `redundant_async_block` — অপ্রয়োজনীয় async ব্লক

- **ফাইল**: `ecat-middleware/src/tracing.rs:38`
- **গুরুতরতা**: কম
- **সমস্যা**: `Box::pin(async move { fut.await })`-এর async ব্লক অপ্রয়োজনীয়
- **মেরামত**: `Box::pin(fut)`-এ সরলীকরণ

### সমস্যা 5：[Clippy] `redundant_closure` — অপ্রয়োজনীয় ক্লোজার

- **ফাইল**: `ecat-data-sqlx/src/lib.rs:63`
- **গুরুতরতা**: কম
- **সমস্যা**: `.and_then(|f| serde_json::Number::from_f64(f))` ক্লোজারটি বাদ দেওয়া যায়
- **মেরামত**: সরাসরি `.and_then(serde_json::Number::from_f64)` ব্যবহার

### সমস্যা 6：[Clippy] `unwrap_or_default` — unwrap_or_default দিয়ে সরলীকরণ

- **ফাইল**: `ecat-transport-http/src/lib.rs:27`
- **গুরুতরতা**: কম
- **সমস্যা**: `unwrap_or_else(Router::new)` `unwrap_or_default()`-এর সমতুল্য
- **মেরামত**: `unwrap_or_default()` ব্যবহার

---

## তিন、টেস্ট কভারেজ পরিস্থিতি

### মেরামতের আগে

| Crate | টেস্ট সংখ্যা |
|-------|--------|
| `ecat-errors` | 4 |
| `ecat-transport` | 11 |
| অন্য 15টি crate | **0** |
| **মোট** | **15** |

### মেরামতের পরে

| Crate | টেস্ট সংখ্যা | নতুন | টেস্ট বিষয়বস্তু |
|-------|--------|------|----------|
| `ecat-encoding` | 15 | +15 | JsonCodec এনকোড-ডিকোড রাউন্ডট্রিপ、অবৈধ ডিকোড、content_type；CodecBox ডিসপ্যাচ；codec_from_content_type স্বাভাবিক/এরর পাথ；Encoding ভেরিয়েন্ট |
| `ecat-errors` | 4 | — | HTTP স্ট্যাটাস কোড ম্যাপিং、gRPC স্ট্যাটাস রূপান্তর、metadata জমা、Display ফরম্যাট |
| `ecat-metadata` | 9 | +9 | কী-ভ্যালু存取、trace_id、From\<HeaderMap\>（অ-UTF8 মান স্কিপ সহ）、From\<MetadataMap\>（ASCII ও বাইনারি স্কিপ）、IntoIterator |
| `ecat-logging` | 1 | +1 | init স্মোক টেস্ট |
| `ecat-config` | 4 | +4 | নতুন/ডিফল্ট মান、টাইপড রিডিং、ConfigSource থেকে লোড |
| `ecat-registry` | 5 | +5 | রেজিস্ট্রি/ডিসকভারি、ডিরেজিস্টার/ডিলিট、অস্তিত্বহীন এরর、সার্ভিস তালিকা、নাম ফিল্টার |
| `ecat-metrics` | 2 | +2 | সিঙ্গেলটন registry、metrics_text panic হয় না |
| `ecat` | 4 | +4 | Builder ডিফল্ট মান、কাস্টম নাম/ভার্সন、server রেজিস্ট্রেশন、lifecycle hook |
| `ecat-transport` | 11 | — | Context/Request/Response তৈরি ও ডিফল্ট মান、Server trait |
| **মোট** | **55** | **+40** | |

### ইউনিট টেস্টের প্রয়োজন নেই এমন crates

- `ecat-protos` — শুধুমাত্র protobuf কোড জেনারেশন
- `ecat-data` — বিশুদ্ধ trait সংজ্ঞা, ইমপ্লিমেন্টেশন লজিক নেই
- `ecat-data-sqlx` — ডেটাবেস সংযোগ প্রয়োজন, ইন্টিগ্রেশন টেস্টের আওতায়
- `ecat-middleware` — Tower Service ইমপ্লিমেন্টেশন, ইন্টিগ্রেশন টেস্ট প্রয়োজন
- `ecat-transport-http` / `ecat-transport-grpc` — নেটওয়ার্ক লিসেনিং প্রয়োজন, ইন্টিগ্রেশন টেস্টের আওতায়
- `ecat-cli` — শুধুমাত্র প্রিন্ট আউটপুট, কোনো লজিক নেই

---

## চার、ভেরিফিকেশন ফলাফল

```
cargo test   → 55 passed, 0 failed
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
```

---

## পাঁচ、পরিবর্তিত ফাইল তালিকা

| ফাইল | পরিবর্তন |
|------|------|
| `ecat-config/src/file.rs` | identity map সরানো |
| `ecat-config/src/lib.rs` | `#[derive(Default)]` + 4টি টেস্ট |
| `ecat-data-sqlx/src/lib.rs` | অপ্রয়োজনীয় ক্লোজার সরলীকরণ |
| `ecat-middleware/src/recovery.rs` | `std::io::Error::other()` ব্যবহার |
| `ecat-middleware/src/tracing.rs` | অপ্রয়োজনীয় async ব্লক সরানো |
| `ecat-transport-http/src/lib.rs` | `unwrap_or_else` → `unwrap_or_default` |
| `ecat-metrics/src/lib.rs` | 2টি টেস্ট |
| `ecat-registry/src/memory.rs` | 5টি টেস্ট |
| `ecat/src/lib.rs` | 4টি টেস্ট |
