<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# E-CAT অডিট রিপোর্ট — r5

**তারিখ**: 2026-08-01  
**ব্রাঞ্চ**: main  
**সংস্করণ**: 2.1.7  
**Crate সংখ্যা**: 47 (workspace members)
**অবস্থা**: ✅ সব মেরামতযোগ্য সমস্যা সমাধান হয়েছে + ডেটা ব্যাকএন্ডে কনফিগ ফাইলের পূর্ণ সমর্থন

---

## 0. মেরামত রেকর্ড (2026-08-01)

| # | সমস্যা | ফাইল | মেরামত |
|---|------|------|------|
| 1 | unused import `axum::routing::get` | `ecat-versioning/src/lib.rs:3` | টপ-লেভেল import অপসারণ, `#[cfg(test)]`-এর ভিতরে স্থানান্তর |
| 2 | unused variable `version` | `ecat-versioning/src/lib.rs:61` | `_version` করা হয়েছে |
| 3 | dead code `extract_version` | `ecat-versioning/src/lib.rs:68` | `pub fn` করা হয়েছে |
| 4 | `useless_format!` | `ecat-versioning/src/lib.rs:62` | সরাসরি `"/api"` ব্যবহার |
| 5 | `unnecessary_to_owned` | `ecat-data-questdb/src/lib.rs:39` | `"true".to_string()` → `"true"` |
| 6 | এরর মেসেজ গিলে ফেলা | `ecat-data-questdb/src/lib.rs:30` | `unwrap_or_default()` → `unwrap_or_else(...)` |
| 7 | `derivable_impls` | `ecat-client/src/lib.rs:249` | `GrpcClientBuilder`-তে `#[derive(Default)]` ব্যবহার |
| 8 | `manual_is_multiple_of` | `ecat-config/src/encrypted.rs:60` | `s.len() % 2 != 0` → `!s.len().is_multiple_of(2)` |
| 9 | `collapsible_if` | `ecat-registry-etcd/src/lib.rs:92` | নেস্টেড `if let` একীভূত |
| 10 | `collapsible_if` | `ecat-data-clickhouse/src/lib.rs:56` | নেস্টেড `if let` একীভূত |
| 11 | `type_complexity` | `ecat-data-memcached/src/lib.rs:9` | `type CacheEntry` অ্যালিয়াস যোগ |

**চূড়ান্ত ফলাফল**: `cargo build` শূন্য warning, `cargo clippy --all-targets` শূন্য warning, `cargo test` সব পাস (0 ব্যর্থ)।

### 12 ─ ডেটা ব্যাকএন্ডে কনফিগ ফাইলের পূর্ণ সমর্থন (Cargo + lib.rs)

12টি ডেটা ব্যাকএন্ড crate-এ নতুন `Config` স্ট্রাক্ট (`#[derive(Deserialize)]`) এবং `from_config()` কনস্ট্রাক্টর যোগ করা হয়েছে, JSON/YAML কনফিগ ফাইল থেকে সংযোগ তথ্য লোড করা যায়, হার্ডকোডিংয়ের প্রয়োজন নেই।

| Crate | Config স্ট্রাক্ট | ফিল্ড |
|-------|--------------|------|
| `ecat-data-redis` | `RedisConfig` | `url` |
| `ecat-data-sqlx` | `SqlxConfig` | `url` |
| `ecat-data-clickhouse` | `ClickhouseConfig` | `base_url`, `database` (ডিফল্ট "default") |
| `ecat-data-questdb` | `QuestdbConfig` | `base_url` |
| `ecat-data-elasticsearch` | `ElasticsearchConfig` | `base_url` |
| `ecat-data-opensearch` | `OpenSearchConfig` | `base_url` |
| `ecat-data-influxdb` | `InfluxConfig` | `base_url`, `org`, `bucket`, `token` |
| `ecat-data-memcached` | `MemcachedConfig` | (খালি — মেমরি ইমপ্লিমেন্টেশন) |
| `ecat-data-neo4j` | `Neo4jConfig` | `base_url`, `username`, `password` |
| `ecat-data-nebulagraph` | `NebulaGraphConfig` | `base_url`, `space` |
| `ecat-data-arangodb` | `ArangoConfig` | `base_url`, `db`, `username`, `password` |
| `ecat-data-iotdb` | `IotdbConfig` | `base_url`, `username`, `password` |

**ব্যবহারের উদাহরণ**:
```rust
// YAML কনফিগ ফাইল থেকে লোড
let cfg: ClickhouseConfig = serde_json::from_str(r#"{"base_url":"http://localhost:8123"}"#)?;
let client = ClickhouseClient::from_config(cfg);
```

### 13 ─ HTTP ব্যাকএন্ডে ঐচ্ছিক অথেনটিকেশন সাপোর্ট (5টি crate)

5টি বিশুদ্ধ HTTP ব্যাকএন্ডে ঐচ্ছিক `username` / `password` ফিল্ড এবং `with_auth()` কনস্ট্রাক্টর যোগ করা হয়েছে। সব `Option<String>` (`#[serde(default)]`), কনফিগ না করলে কোনো অথেনটিকেশন নেই।

| Crate | নতুন Config ফিল্ড | নতুন কনস্ট্রাক্টর |
|-------|-----------------|-------------|
| `ecat-data-elasticsearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-opensearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-clickhouse` | `username?`, `password?` | `with_auth()` |
| `ecat-data-questdb` | `username?`, `password?` | `with_auth()` |
| `ecat-data-nebulagraph` | `username?`, `password?` | `with_auth()` |

সব HTTP রিকোয়েস্ট `apply_auth()` হেল্পার মেথডের মাধ্যমে স্বয়ংক্রিয়ভাবে Basic Auth সংযুক্ত করে (শুধুমাত্র দুটোই None না হলে)।

### 14 ─ Redis / RDBMS / Memcached-এ ঐচ্ছিক অথেনটিকেশন ফিল্ড (3টি crate)

| Crate | নতুন Config ফিল্ড | নতুন কনস্ট্রাক্টর | অথেনটিকেশন পদ্ধতি |
|-------|-----------------|-------------|----------|
| `ecat-data-redis` | `password?` | `connect_with_password()` | URL-এ এমবেড করা পাসওয়ার্ড |
| `ecat-data-sqlx` | `username?`, `password?` | `connect_with_auth()` | URL-এ এমবেড করা অথেনটিকেশন |
| `ecat-data-memcached` | `username?`, `password?` | `with_auth()` | ফিল্ড সংরক্ষিত (মেমরি ইমপ্লিমেন্টেশন) |

Sqlx চার ধরনের RDBMS কভার করে: SQLite / PostgreSQL / MySQL / TiDB। Auth ফিল্ড `replacen("://", "://user:pass@")` দিয়ে কানেকশন URL-এ এমবেড হয়, শুধুমাত্র URL-এ `@` না থাকলে কার্যকর হয়।

### 15 ─ TLS সার্টিফিকেট অথেনটিকেশন সাপোর্ট + ecat-tls crate (সব 12টি ব্যাকএন্ড)

নতুন `ecat-tls` crate, যা প্রদান করে:
- `TlsClientConfig` — ঐচ্ছিক TLS কনফিগ (ca_cert, client_cert, client_key, skip_verify)
- `generate_ca()` — স্বাক্ষরিত CA সার্টিফিকেট জেনারেশন
- `generate_server_cert()` — সার্ভার সার্টিফিকেট জেনারেশন
- `generate_client_cert()` — ক্লায়েন্ট সার্টিফিকেট জেনারেশন (mTLS)

সব 12টি ডেটা ব্যাকএন্ড Config-এ নতুন `#[serde(default)] tls: Option<TlsClientConfig>` ফিল্ড।

| ব্যাকএন্ড টাইপ | TLS পদ্ধতি |
|----------|----------|
| 9টি HTTP ব্যাকএন্ড | `tls.build_reqwest_client()` দিয়ে TLS reqwest Client নির্মাণ |
| Redis | URL scheme সুইচ `redis://` → `rediss://` |
| Sqlx | ফিল্ড সংরক্ষিত (TLS URL প্যারামিটার `?sslmode=require`-এর মাধ্যমে) |
| Memcached | ফিল্ড সংরক্ষিত (নেটওয়ার্ক ইমপ্লিমেন্টেশনের জন্য রিজার্ভ) |

---

## 1. ওভারভিউ

| আইটেম | অবস্থা | বিস্তারিত |
|------|------|------|
| `cargo build` | ✅ পাস | 3টি কম্পাইলার warnings, 19.85s |
| `cargo test` | ✅ পাস | ~137টি ইউনিট টেস্ট সব পাস, 0 ব্যর্থ, 1 ignored |
| `cargo clippy` | ⚠️ warning আছে | 3টি crate-এ মোট 5টি lint warnings |
| `cargo fmt` | ✅ পাস | কোনো ফরম্যাট সমস্যা নেই |
| `cargo audit` | ❌ ইনস্টল করা নেই | পরিচিত CVE স্ক্যান করা যায়নি |

---

## 2. কম্পাইলার Warnings (মেরামত প্রয়োজন)

### 2.1 ecat-versioning (3টি warning)

**ফাইল**: `ecat-versioning/src/lib.rs`

| # | Warning | লাইন | গুরুতরতা |
|---|---------|------|----------|
| 1 | `unused import: axum::routing::get` | 3 | কম |
| 2 | `unused variable: version` | 61 | কম |
| 3 | `function extract_version is never used` | 68 | কম |

**পরামর্শ**: অব্যবহৃত import মুছুন, `version`-কে `_version` করুন, `extract_version`-কে `pub` করুন বা `#[allow(dead_code)]` চিহ্নিত করুন।

### 2.2 ecat-data-questdb (1টি clippy warning)

**ফাইল**: `ecat-data-questdb/src/lib.rs:39`

```rust
// বর্তমান:
.query(&[("query", sql), ("count", &"true".to_string())])

// বদলে হওয়া উচিত:
.query(&[("query", sql), ("count", &"true")])
```

### 2.3 ecat-client (1টি clippy warning)

**ফাইল**: `ecat-client/src/lib.rs:249`

`GrpcClientBuilder` ম্যানুয়ালি `Default` ইমপ্লিমেন্ট করেছে, সরাসরি `#[derive(Default)]` দিয়ে প্রতিস্থাপন করা যায়।

---

## 3. Clippy Lint Warnings সারসংক্ষেপ

| Crate | Warning | টাইপ |
|-------|---------|------|
| ecat-versioning | `useless_format!` — `"/api".to_string()` ব্যবহার | পারফরম্যান্স |
| ecat-versioning | unused import / dead code | পরিষ্করণ |
| ecat-data-questdb | `unnecessary_to_owned` | পারফরম্যান্স |
| ecat-client | `derivable_impls` — derive Default ব্যবহার | সরলীকরণ |

---

## 4. টেস্ট কভারেজ বিশ্লেষণ

### 4.1 পরিসংখ্যান

| মেট্রিক | মান |
|------|------|
| মোট ইউনিট টেস্ট | ~137 |
| ব্যর্থ | 0 |
| Ignored | 1 |
| টেস্ট আছে এমন crate | ~24 / 48 |
| **0 টেস্টের crate** | **~24 / 48 (50%)** |

### 4.2 টেস্টের অভাবযুক্ত Crate (0 বা শুধু কনস্ট্রাকশন টেস্ট)

নিম্নলিখিত crates-এর টেস্ট দুর্বল:

- ecat-data-arangodb, ecat-data-clickhouse, ecat-data-elasticsearch
- ecat-data-influxdb, ecat-data-iotdb, ecat-data-nebulagraph
- ecat-data-neo4j, ecat-data-opensearch, ecat-data-questdb
- ecat-data-redis, ecat-data-sqlx, ecat-data-memcached
- ecat-mq, ecat-mq-kafka, ecat-graphql, ecat-openapi
- ecat-transport, ecat-transport-grpc, ecat-transport-http
- ecat-transport-ws, ecat-tracing, ecat-logging
- ecat-middleware, ecat-registry-consul, ecat-registry-etcd

### 4.3 Doc-tests

সব **48টি crate-এর doc-tests 0**। কোডে কোনো `/// ````rust` ডকুমেন্টেশন উদাহরণ নেই।

---

## 5. ডিপেন্ডেন্সি সমস্যা

### 5.1 ⚠️ yaml_serde বনাম serde_yaml (মাঝারি ঝুঁকি)

**ফাইল**: `ecat-config/Cargo.toml:9`

```toml
yaml_serde = "0.10"
```

Rust ইকোসিস্টেমের স্ট্যান্ডার্ড YAML লাইব্রেরি হল `serde_yaml` (সর্বশেষ `0.9.34+`), আর `yaml_serde` একটি **ভিন্ন এবং কম রক্ষণাবেক্ষণকৃত** crate।

**পরামর্শ**: নিশ্চিত করুন `yaml_serde` প্রত্যাশিত ডিপেন্ডেন্সি কিনা। যদি উদ্দেশ্য `serde_yaml` হয়, তবে প্রতিস্থাপন করুন।

### 5.2 cargo-audit-এর অভাব

`cargo audit` ইনস্টল করা নেই। `cargo install cargo-audit` করার এবং CI-তে যোগ করার পরামর্শ।

### 5.3 description ফিল্ডের অভাব

`[workspace.package]`-এ `description` নেই, সব সাব-ক্রেটেও description সংজ্ঞায়িত নেই।

---

## 6. কোড কোয়ালিটি সমস্যা

### 6.1 প্রোডাকশন কোডে unwrap/expect

| ফাইল | লাইন | কল | ঝুঁকি |
|------|------|------|------|
| `ecat-client/src/lib.rs` | 28 | `.expect("StaticResolver poisoned")` | কম — যুক্তিসঙ্গত |
| `ecat/src/signal.rs` | 8 | `.expect("failed to install SIGTERM handler")` | মাঝারি — স্টার্টআপে panic |
| `ecat-protos/build.rs` | 5 | `.unwrap()` | কম — build script |

### 6.2 ecat-versioning-এর extract_version

`extract_version` ফাংশন (৬৮ নম্বর লাইন) Accept header থেকে সংস্করণ নম্বর এক্সট্র্যাক্ট করে, কিন্তু `build_header_router()` এটিকে কল করে না।

### 6.3 ecat-data-questdb এরর হ্যান্ডলিং

```rust
// ৩০ নম্বর লাইন: নেটওয়ার্ক রেসপন্স বডি পড়ায় unwrap_or_default ব্যবহৃত
Err(RdbmsError::Database(resp.text().await.unwrap_or_default()))
```

`resp.text()` ব্যর্থ হলে এরর মেসেজ নীরবে গিলে ফেলা হয়। `unwrap_or_else(|e| format!("questdb parse: {e}"))` ব্যবহারের পরামর্শ।

---

## 7. আর্কিটেকচার মূল্যায়ন

### সুবিধা

- 48টি crate-এর দায়িত্ব বিভাজন স্পষ্ট
- workspace ইউনিফাইড সংস্করণ `version.workspace = true`
- ডিপেন্ডেন্সি সংক্ষিপ্ত, কোনো বড় ফ্রেমওয়ার্ক নেই
- কোনো TODO/FIXME/HACK নেই

### উন্নতির প্রয়োজন

| সমস্যা | অগ্রাধিকার |
|------|--------|
| 50% crate-এ টেস্ট নেই | উচ্চ |
| yaml_serde বনাম serde_yaml বিভ্রান্তি | মাঝারি |
| cargo-audit-এর অভাব | মাঝারি |
| ecat-versioning ডেড কোড | কম |
| doc-tests নেই | কম |

---

## 8. নিরাপত্তা ওভারভিউ

| চেক আইটেম | ফলাফল |
|--------|------|
| হার্ডকোডেড সিক্রেট | পাওয়া যায়নি |
| .env ফাইল লিক | পাওয়া যায়নি |
| বিপজ্জনক unwrap (প্রোডাকশন কোড) | 2টি জায়গায় (signal.rs, client.rs) |
| CVE স্ক্যান | সম্পন্ন হয়নি (cargo-audit ইনস্টল প্রয়োজন) |

---

## 9. অ্যাকশন প্ল্যান

### P0 — এখনই মেরামত
1. ecat-versioning-এর 3টি compiler warnings পরিষ্কার করুন
2. ecat-data-questdb clippy মেরামত করুন
3. ecat-client derivable_impls মেরামত করুন

### P1 — স্বল্পমেয়াদী
4. ডিপেন্ডেন্সি দুর্বলতা স্ক্যানের জন্য `cargo-audit` ইনস্টল করুন
5. `yaml_serde` বনাম `serde_yaml` নির্বাচন নিশ্চিত করুন
6. কোর crates-এর জন্য doc-tests যোগ করুন

### P2 — মাঝারি মেয়াদী
7. transport/data/security crates-এর জন্য টেস্ট যোগ করুন
8. সব crates-এ `description` ফিল্ড যোগ করুন
9. `extract_version` একীভূত করুন বা অপসারণ করুন

### P3 — দীর্ঘমেয়াদী
10. CI প্রতিষ্ঠা করুন: build → test → clippy → audit → coverage

---

*রিপোর্ট তৈরি হয়েছে 2026-08-01। টুলচেইন: cargo 1.92.0, rustc 1.92.0, clippy 1.92.0*
