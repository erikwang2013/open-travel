<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat কোড রিভিউ রিপোর্ট — 2026-08-01 (৪র্থ রাউন্ড · সব মেরামতকৃত)

**প্রজেক্ট সংস্করণ:** 2.1.0  
**চূড়ান্ত অবস্থা:** 0 warnings, ~116 tests, clippy clean, fmt clean

**৫ম রাউন্ডের পরিষ্করণ:** 12টি অব্যবহৃত ডিপেন্ডেন্সি অপসারণ (ecat-health/reqwest, ecat-circuit-breaker/tokio, ecat-bench/tracing, ecat-mq/serde+serde_json, ecat-events/async-trait, ecat-config-remote/tracing, ecat-testing/transport-http+axum, ecat-client/serde+serde_json)
**রিভিউ সুযোগ:** সব 18টি crate

## চূড়ান্ত অবস্থা

| টুল | অবস্থা |
|------|------|
| `cargo build` | পাস (0 warnings) |
| `cargo test` | 77 passed, 0 failed, 1 ignored |
| `cargo clippy` | পাস (0 warnings) |
| `cargo fmt` | পাস |

---

## মেরামত তালিকা (সব)

### মাঝারি ঝুঁকি

1. **[মেরামতকৃত]** `Mutex::lock().unwrap()` → `ecat-transport-http/lib.rs`, `ecat-transport-grpc/lib.rs`
2. **[মেরামতকৃত]** CLI `fs::write().unwrap()` → `ecat-cli/src/main.rs`

### কম ঝুঁকি

3. **[মেরামতকৃত]** ProtoCodec doc-test → `ecat-encoding/src/proto.rs`
4. **[মেরামতকৃত]** শূন্য ইউনিট টেস্টের crate → transport-http/grpc-তে প্রতিটিতে 3টি টেস্ট যোগ
5. **[মেরামতকৃত]** `Transaction::commit()` খালি অপারেশন → নতুন `TransactionInner` trait
6. **[মেরামতকৃত]** `SecurityScanner::new()` মন্তব্য সংশোধন
7. **[মেরামতকৃত]** অব্যবহৃত `opentelemetry` ডিপেন্ডেন্সি → `ecat-logging` এবং workspace রুট Cargo.toml
8. **[মেরামতকৃত]** Doc-test ফরম্যাট

### অপটিমাইজেশন

9. **[মেরামতকৃত]** `scan_parts` প্রি-অ্যালোকেশন → `Vec::with_capacity`
10. **[মেরামতকৃত]** `serde_yaml` 0.9 অবচিত → `yaml_serde` 0.10-এ স্থানান্তর
11. **[মেরামতকৃত]** `Transaction::commit()` আর খালি অপারেশন নয় → `SqlxTransactionWrapper`-এর মাধ্যমে প্রকৃত commit/rollback বাস্তবায়িত

### মেরামতের প্রয়োজন নেই (ডিজাইন সিদ্ধান্ত)

- **`ecat` crate অতিরিক্ত ডিপেন্ডেন্সি** — ইচ্ছাকৃত «meta crate» প্যাটার্ন, ডাউনস্ট্রিমের জন্য সুবিধাজনক ট্রানজিটিভ ডিপেন্ডেন্সি প্রদান করে
- **ProtoCodec Codec trait এরর ফেরত দেয়** — serde ও prost::Message-এর মৌলিক টাইপ পার্থক্য, `encode_message()`/`decode_message()` আলাদা API এবং স্পষ্ট ডকুমেন্টেশনের মাধ্যমে ব্যাখ্যা করা হয়েছে
- **`ecat-data`-তে কংক্রিট ইমপ্লিমেন্টেশন নেই** — trait ইন্টারফেস ডিজাইন, ইমপ্লিমেন্টেশন `ecat-data-sqlx`-এ

---

## পরিবর্তিত ফাইল সারসংক্ষেপ

| ফাইল | পরিবর্তন |
|------|------|
| `ecat-transport-http/src/lib.rs` | Mutex পয়জনিং সুরক্ষা + 3টি নতুন টেস্ট |
| `ecat-transport-grpc/src/lib.rs` | Mutex পয়জনিং সুরক্ষা + 3টি নতুন টেস্ট |
| `ecat-cli/src/main.rs` | ইউনিফাইড এরর হ্যান্ডলিং |
| `ecat-security/src/lib.rs` | মন্তব্য সংশোধন + প্রি-অ্যালোকেশন অপটিমাইজেশন |
| `ecat-logging/Cargo.toml` | অব্যবহৃত opentelemetry অপসারণ |
| `ecat-encoding/src/proto.rs` | doc-test উন্নতকরণ |
| `ecat-data/src/lib.rs` | TransactionInner এক্সপোর্ট |
| `ecat-data/src/rdbms.rs` | নতুন TransactionInner trait |
| `ecat-data-sqlx/src/lib.rs` | SqlxTransactionWrapper TransactionInner বাস্তবায়ন করে |
| `ecat-config/Cargo.toml` | serde_yaml → yaml_serde |
| `ecat-config/src/file.rs` | serde_yaml → yaml_serde |
| `Cargo.toml` | orphaned opentelemetry workspace ডিপেন্ডেন্সি অপসারণ |
| `README.md` | সংস্করণ নম্বর আপডেট, অবজারভেবিলিটি বর্ণনা সংশোধন, ইকোসিস্টেম পরিকল্পনা লিংক যোগ |
| `docs/ecosystem-plan.md` | নতুন ইকোসিস্টেম পরিকল্পনা ডকুমেন্টেশন (তিন ধাপে 15টি crate) |
