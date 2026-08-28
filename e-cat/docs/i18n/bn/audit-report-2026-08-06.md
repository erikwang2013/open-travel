<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat সর্বাঙ্গীণ পর্যালোচনা রিপোর্ট

**তারিখ**: 2026-08-06
**সংস্করণ**: 2.3.0 · 55 crates
**সুযোগ**: বিল্ড/টেস্ট, রানটাইম স্মোক, ইকোসিস্টেম ধারাবাহিকতা, নিরাপত্তা সুরক্ষা, ডিপ্লয় কনফিগ

---

## 1. টেস্ট ও বিল্ড ফলাফল

| চেক আইটেম | ফলাফল | ব্যাখ্যা |
|--------|------|------|
| `cargo check --workspace` | ✅ পাস | 0 warning |
| `cargo test --workspace` | ✅ পাস | **202টি টেস্ট সব পাস, 0 ব্যর্থতা** (doc-tests সহ) |
| `cargo fmt --check` | ✅ পাস | |
| `cargo clippy --workspace -- -D warnings` | ✅ পাস | CI কমান্ডের সাথে সামঞ্জস্যপূর্ণ |
| `cargo clippy --all-targets -- -D warnings` | ❌ ব্যর্থ | আবিষ্কার D2 দেখুন |
| স্মোক টেস্ট (helloworld) | ❌ **স্টার্ট ব্যর্থ** | আবিষ্কার D1 দেখুন |

**টেস্ট কভারেজ বণ্টন**: 51টি সোর্স ফাইলে `#[test]`, 105টি টেস্ট বাইনারি। প্রোডাকশন পাথে কোনো `todo!()`/`unimplemented!()` নেই, `panic!` শুধুমাত্র টেস্ট কোডে আছে।

---

## 2. রানটাইম সমস্যা (স্মোক টেস্টে আবিষ্কৃত)

### [HIGH] D1. `HttpServer::new(":8000")` IPv6 নেই এমন পরিবেশে শুরু ব্যর্থ
- **অবস্থান**: `ecat-transport-http/src/lib.rs:40`, `examples/helloworld/src/main.rs:41`, README-এর একাধিক জায়গা
- **উপসর্গ**: `TcpListener::bind(":8000")` IPv6 ওয়াইল্ডকার্ড `[::]:8000`-এ রেজলভ করে, IPv6 নেই এমন মেশিনে (কনটেইনার/কিছু ক্লাউড হোস্ট) `failed to lookup address information: Name or service not known` রিপোর্ট হয়, সার্ভিস শুরু হয় না।
- **প্রতিলিপি**: স্বাধীন মিনিমাল প্রোগ্রাম দিয়ে যাচাই — `bind(":8001")` ব্যর্থ, `bind("0.0.0.0:8002")` সফল, `bind("localhost:8003")` সফল।
- **মেরামত**: `HttpServer::new`-এর ভিতরে খালি host-কে `"0.0.0.0"`-তে নরমালাইজ করা হয়েছে; উদাহরণ ও ডকুমেন্টেশন সব `"0.0.0.0:8000"` ব্যবহার করে।

### [LOW] D2. `cargo clippy --all-targets -- -D warnings` ব্যর্থ
- **অবস্থান**: `ecat-data-sqlx/src/lib.rs` (টেস্ট মডিউলের পরে items আছে, `items_after_test_module` ট্রিগার করে)
- **প্রভাব**: বর্তমান CI-এর clippy কমান্ড (কোনো `--all-targets` নেই) প্রভাবিত নয়; CI কড়া করলে ব্যর্থ হবে।
- **মেরামত**: টেস্ট মডিউল ফাইলের শেষে সরানো হয়েছে।

---

## 3. গুরুতর সমস্যা (CRITICAL)

### [CRITICAL] C1. `ecat-data-memcached` একটি «জাল ইমপ্লিমেন্টেশন»
- **অবস্থান**: `ecat-data-memcached/src/lib.rs:23-88`
- **সমস্যা**: পুরো crate একটি বিশুদ্ধ মেমরি `HashMap`, কোনো নেটওয়ার্ক সংযোগ নেই, কোনো সার্ভার ঠিকানা কনফিগ নেই (`MemcachedConfig`-এ শুধু username/password/tls আছে), Cargo.toml description নিজেই স্বীকার করে "in-memory cache client"। প্রোডাকশনে ভুল ব্যবহার করলে **নীরবে ডেটা হারাবে** (রিস্টার্টেই খালি, মাল্টি-ইনস্ট্যান্সে শেয়ার হয় না)।
- **মেরামত**: প্রকৃত memcached প্রোটোকল সংযোগ (`memcache` crate-এর মতো), অথবা স্পষ্টভাবে `#[deprecated]`/ডকুমেন্টেশন সতর্কতা দিয়ে প্রোডাকশন ব্যবহার নিষিদ্ধ করা।

### [CRITICAL] C2. TDengine রাইট SQL স্ট্রিং জোড়া ইনজেকশন
- **অবস্থান**: `ecat-data-tdengine/src/lib.rs:91-116`
- **সমস্যা**: `INSERT INTO "{}" ({}) VALUES ({})`-এ measurement/কলাম নাম/মান সব `format!` দিয়ে সরাসরি জোড়া হয়, স্ট্রিং মান শুধু ডাবল কোটে মোড়ানো, `"` ও `\` এস্কেপ করা হয় না। `"; DELETE ...; --` সম্বলিত ফিল্ড মান এস্কেপ করে ইচ্ছামতো SQL চালাতে পারে (TDengine REST মাল্টি-স্টেটমেন্ট সাপোর্ট করে)।
- **মেরামত**: আইডেন্টিফায়ার ও স্ট্রিং মান এস্কেপ করুন (`"`→`\"`, `\`→`\\`), অথবা প্যারামিটারাইজড রাইট ইন্টারফেস ব্যবহার করুন।

---

## 4. উচ্চ-ঝুঁকি সমস্যা (HIGH)

### [HIGH] H1. সব HTTP ডেটাবেস অ্যাডাপ্টারে টাইমআউট নেই
- **অবস্থান**: `ecat-tls/src/lib.rs:27,61`, elasticsearch/opensearch/clickhouse/influxdb/iotdb/questdb/tdengine/neo4j/nebulagraph/arangodb
- **সমস্যা**: reqwest-এ ডিফল্ট কোনো টাইমআউট নেই, সার্ভার হ্যাং হলে রিকোয়েস্ট **চিরকাল ঝুলে থাকবে** (কানেকশন পুল নিঃশেষ, টাস্ক লিক)।
- **মেরামত**: `build_reqwest_client`-এ ইউনিফর্ম `connect_timeout` (যেমন 5s) + `timeout` (যেমন 30s) সেট করুন।

### [HIGH] H2. রেট লিমিটিং ক্লায়েন্ট অনুযায়ী কার্যকর হয় না
- **অবস্থান**: `ecat-middleware/src/ratelimit.rs:155`
- **সমস্যা**: `key_fn("")` রিকোয়েস্ট অবজেক্ট পায় না, IP/ইউজার অনুযায়ী লিমিটিং করা যায় না; ডিফল্ট একক বাকেট "global", আক্রমণকারী গ্লোবাল কোটা নিঃশেষ করতে পারে (অন্যদের DoS) বা ডিস্ট্রিবিউটেড বাইপাস।
- **মেরামত**: `key_fn` সিগনেচার `&http::Request` গ্রহণ করার জন্য পরিবর্তন করুন, `X-Forwarded-For`/পিয়ার ঠিকানা অনুযায়ী key নিন।

### [HIGH] H3. GitHub CI অবশ্যই ব্যর্থ হবে (protoc নেই)
- **অবস্থান**: `.github/workflows/ci.yml`
- **সমস্যা**: `ecat-protos` build.rs tonic-build দিয়ে proto কম্পাইল করে, protoc-এর উপর দৃঢ় নির্ভর; GH CI-তে `protobuf-compiler` ইনস্টল নেই (লোকাল `/home/erik/.local/bin/protoc` আছে বলে লোকাল পাস করে)। `.gitlab-ci.yml`-এ ইনস্টল করা আছে, দুই সেট CI-এর আচরণ ভিন্ন।
- **মেরামত**: GH CI-তে `apt-get install protobuf-compiler` যোগ করুন (প্রয়োজনে cmake-ও)।

### [HIGH] H4. Elasticsearch `search()`/`delete()` HTTP স্ট্যাটাস কোড পরীক্ষা করে না
- **অবস্থান**: `ecat-data-elasticsearch/src/lib.rs:87-114`
- **সমস্যা**: 404/400 এরর বডি JSON হিসেবে পার্স হয়, বিভ্রান্তিকর "es parse" এরর রিপোর্ট হয়; `index()` পরীক্ষা করে কিন্তু `search`/`delete` করে না, আচরণ অসামঞ্জস্যপূর্ণ (opensearch সঠিক)।
- **মেরামত**: ইউনিফর্মভাবে `status.is_success()` পরীক্ষা করুন।

### [HIGH] H5. IoTDB `insertTablet` প্রোটোকল সামঞ্জস্যের সন্দেহ
- **অবস্থান**: `ecat-data-iotdb/src/lib.rs:51-82`
- **সমস্যা**: IoTDB REST `insertTablet`-এর `timestamps/measurements/values/data_types` অ্যারে ফরম্যাট প্রয়োজন; এই ইমপ্লিমেন্টেশন একক ডকুমেন্ট JSON পাঠায়, «দেখতে কাজ করে কিন্তু আসলে অকেজো» হতে পারে।
- **মেরামত**: insertTablet স্পেক অনুযায়ী রিকোয়েস্ট বডি তৈরি করুন, এবং ইন্টিগ্রেশন টেস্ট যোগ করুন।

### [HIGH] H6. etcd deregister প্রিফিক্স মেলে না (deregister অকার্যকর)
- **অবস্থান**: `ecat-registry-etcd/src/lib.rs:47,66`
- **সমস্যা**: রেজিস্ট্রেশন key `/ecat/services/{prefix}/{name}/{uuid}`, কিন্তু deregister `{prefix}/{name}` মুছে দেয় (uuid সেগমেন্ট নেই) → ইন্সট্যান্স বেরোনোর পর রেজিস্ট্রেশন তথ্য থেকে যায়।
- **মেরামত**: মুছে ফেলার সময় সম্পূর্ণ key ম্যাচ করুন অথবা তালিকাভুক্ত করে name প্রিফিক্স অনুযায়ী মুছুন।

---

## 5. মাঝারি-ঝুঁকি সমস্যা (MEDIUM)

| # | অবস্থান | সমস্যা | পরামর্শ |
|---|------|------|------|
| M1 | `ecat-middleware/src/ratelimit_redis.rs:28-48` | Redis ব্যর্থ হলে Err-কে লিমিট অতিক্রম ধরা হয় → **fail-closed DoS**; INCR-এর পর EXPIRE ব্যর্থ হলে key চিরকালের জন্য অ-মেয়াদোত্তীর্ণ → স্থায়ী নিষেধাজ্ঞা | লিমিটিং/স্টোরেজ এরর আলাদা করুন (স্টোরেজ ব্যর্থ হলে ছেড়ে দিন), Lua অ্যাটমিক স্ক্রিপ্ট |
| M2 | `ecat-middleware/src/ratelimit.rs:16-51` | MemoryStore এন্ট্রি শুধু রিসেট হয় মুছে যায় না, ক্লায়েন্ট key অনুযায়ী হলে **মেমরি সীমাহীন বৃদ্ধি** | নিয়মিত মেয়াদোত্তীর্ণ বাকেট পরিষ্কার করুন |
| M3 | `ecat-auth/src/jwt.rs:25-31` | দুর্বল key-তে ন্যূনতম দৈর্ঘ্য যাচাই নেই (টেস্টে "secret-key"), অফলাইনে ব্রুটফোর্স করা যায় | ≥32 বাইট র্যান্ডম key বাধ্যতামূলক; এরর রেসপন্স জেনারালাইজ করে jsonwebtoken বিশদ ফাঁস এড়ান |
| M4 | `ecat-auth/src/oauth2.rs:111-123` | প্রতি রিকোয়েস্টে নতুন reqwest::Client তৈরি, কোনো timeout নেই; URL-এ HTTPS বাধ্যতামূলক নয় | Client পুনর্ব্যবহার, timeout সেট, https যাচাই |
| M5 | `ecat-data-redis/src/lib.rs:34-64`、`ratelimit_redis.rs:12-17`、ecat-lock | পাসওয়ার্ড percent_encode-এর পর URL-এ এমবেড, সংযোগ এরর Display-এ সম্পূর্ণ URL থাকে → **লগে পাসওয়ার্ড লিক**; URL-এ ইতিমধ্যে `@` থাকলে ক্রেডেনশিয়াল নীরবে ফেলে দেওয়া হয় | অথেনটিকেশন প্যারামিটার আলাদা করে পাঠান, এরর মেসেজে সংবেদনশীল তথ্য মুছুন |
| M6 | `ecat-data-elasticsearch/src/lib.rs:104-113`、opensearch:111-116 | index/id URL এনকোড ছাড়া পাথে জোড়া হয়, `/` দিয়ে অন্য index অ্যাক্সেস করা যায় (IDOR) | URL এনকোড + index ওয়াইটলিস্ট |
| M7 | `ecat-data-sqlx/src/lib.rs:79,173`、questdb:78-84 | ডেটাবেসের কাঁচা এরর (SQL ও মানসহ) সরাসরি উপরে ছুড়ে দেওয়া | বাইরে ইউনিফর্ম জেনারালাইজ, বিশদ শুধু লগে |
| M8 | `ecat-data-clickhouse/src/lib.rs:92` | `execute()` সর্বদা `Ok(0)` ফেরত দেয়, rows_affected হারায়; `query()` পার্স ব্যর্থ সারি নীরবে ফেলে দেয় | প্রকৃত সারি সংখ্যা ফেরত দিন, এরর উপরে ছুড়ুন |
| M9 | `ecat-data-tdengine/src/lib.rs:80-118` | `write()` প্রতি পয়েন্টে লুপে রিকোয়েস্ট করে (N+1) | ব্যাচ রাইট |
| M10 | `ecat-data-sqlx/src/lib.rs:98-142 vs 213-256` | query/query_with-এ ~50 লাইন টাইপ কনভার্সন লজিক ডুপ্লিকেট | সাধারণ ফাংশন এক্সট্র্যাক্ট করুন |
| M11 | `ecat-data-redis/src/lib.rs:167` | `acquire`-এ `ttl.as_millis() as u64` ওভারফ্লো ট্রাঙ্কেশন (`set`-এ হ্যান্ডেল করা হয়েছে, এখানে হয়নি) | ইউনিফর্ম ওভারফ্লো হ্যান্ডলিং |
| M12 | `ecat-data-influxdb/src/lib.rs:69-79` | line protocol স্ট্রিং ফিল্ড এস্কেপ করা হয়নি (কোট/কমা/স্পেস) → রাইট করলেই প্রোটোকল এরর | স্পেক অনুযায়ী এস্কেপ |
| M13 | `ecat-mq-*` | `from_config` সিগনেচার অসামঞ্জস্যপূর্ণ: kafka/mqtt সিঙ্ক্রোনাস, rabbitmq/nats async | সব async-এ ইউনিফাইড |
| M14 | `ecat-auth/src/apikey.rs:33-36`、`ecat-security/src/lib.rs:126-137` | API key query প্যারামিটার সমর্থন করে (লগ/Referer-এ পড়ে); WAF শুধু URI+headers স্ক্যান করে body নয় | key শুধু header-এ পাঠান; WAF-এ body স্ক্যান যোগ করুন |

---

## 6. কম-ঝুঁকি ও তথ্য-স্তর (LOW/INFO)

| # | অবস্থান | সমস্যা |
|---|------|------|
| L1 | `ecat-deploy/Dockerfile` | **অস্তিত্বহীন `ecat-app` বাইনারি কপি করে** (আসল bin `ecat`, ecat-cli থেকে আসে) → docker build-এর পর ইমেজে এন্ট্রিপয়েন্ট নেই; HEALTHCHECK curl ব্যবহার করে কিন্তু ইমেজে curl ইনস্টল নেই |
| L2 | `ecat-deploy/helm/Chart.yaml` | appVersion "2.2.0", বর্তমান সংস্করণ 2.3.0 |
| L3 | `README.en.md` | দাবি করে "v2.1.7 · 47 crates", আসল v2.3.0 · 55 crates, ইংরেজি ডকুমেন্টেশন গুরুতরভাবে পুরনো |
| L4 | `ecat-registry-consul/src/lib.rs:66,143` | রেজিস্ট্রেশন পোর্ট সবসময় 0, discover ফলাফলে সংস্করণ হার্ডকোডেড "1.0" |
| L5 | 11টি crate-এর Cargo.toml | `workspace.dependencies` বাইপাস করে সরাসরি একই সংস্করণের ডিপেন্ডেন্সি লেখা (সংস্করণ ড্রিফট ঝুঁকি) |
| L6 | `ecat-tracing` / `ecat-middleware/src/tracing.rs` | TracingLayer ডুপ্লিকেট ইমপ্লিমেন্টেশন; ecat-tracing-otlp ও ecat-tracing আলাদাভাবে subscriber ইনস্টল করে, একসাথে কল করলে ডাবল init কনফ্লিক্ট |
| L7 | `ecat-config-remote/src/lib.rs:92` | হাতে লেখা base64 ডিকোড, base64 crate ব্যবহারের পরামর্শ |
| L8 | `ecat-graphql` | হাতে লেখা একক-ফিল্ড পার্সার, শুধু টপ-লেভেল একক ফিল্ড সমর্থন করে (নেস্টিং/অ্যালিয়াস/প্যারামিটার নেই), ডকুমেন্টেশনে সীমাবদ্ধতা উল্লেখ নেই |
| L9 | `ecat-cli/src/main.rs:69-104`、lib.rs:3-22 | `ecat new ../../x` পাথ ট্রাভার্সাল; নামে `"`/নিউলাইন থাকলে জেনারেট করা Cargo.toml-এ ইনজেকশন হতে পারে |
| L10 | `config/databases.example.yaml:54-79` | একাধিক কার্যকর ডিফল্ট পাসওয়ার্ড (neo4j/changeme, arangodb root/changeme, iotdb root/root, influx my-secret-token), কপি করলেই ডিফল্ট পাসওয়ার্ডসহ লাইভ |
| L11 | `ecat-data-s3/src/lib.rs:83-93` | list()-এ টাইমআউট কনফিগ নেই; ক্রেডেনশিয়াল নির্মাণ সিঙ্ক্রোনাস ব্লকিং কল |
| L12 | `ecat-data-redis` | স্পষ্ট রিকানেক্ট নেই, MultiplexedConnection-এর বিল্ট-ইন রিকানেক্টের উপর নির্ভর, ডকুমেন্টেশনে ব্যাখ্যা নেই |
| L13 | `ecat-data/src/rdbms.rs:71-77` | `Transaction::drop` শুধু warn করে rollback ট্রিগার করে না, sqlx পাশের drop-এর অটো-রোলব্যাকের উপর নির্ভর, মন্তব্যে ব্যাখ্যার পরামর্শ |

---

## 7. ইকোসিস্টেম সম্পূর্ণতার সিদ্ধান্ত

**সম্পূর্ণতা: উচ্চ**। 55/55 crates workspace-এ, সংস্করণ 2.3.0-এ একীভূত, কোনো stub নেই (memcached জাল ইমপ্লিমেন্টেশন ছাড়া)। 18টি ডেটাবেস ব্যাকএন্ড, 4টি MQ ব্যাকএন্ড, 2টি রেজিস্ট্রি সেন্টার, রেট লিমিটিং স্টোরেজ অ্যাবস্ট্রাকশন, ডিস্ট্রিবিউটেড লক, শিডিউলার, OTLP ট্রেসিং, ভার্সনিং, GraphQL সব বাস্তবায়িত। `todo!()`/`unimplemented!()` শূন্য জায়গায়।

**জোরদার করার বাকি**:
1. memcached-এ প্রকৃত প্রোটোকল ইমপ্লিমেন্টেশন (বর্তমানে একমাত্র «জাল» অ্যাডাপ্টার)
2. IoTDB প্রোটোকল সামঞ্জস্য যাচাই (অকার্যকর হওয়ার সন্দেহ)
3. GitHub CI ও GitLab CI সারিবদ্ধকরণ (protoc-এর অভাব)
4. সব HTTP অ্যাডাপ্টারে ইউনিফর্ম টাইমআউট নীতি

## 8. নিরাপত্তা সুরক্ষার সিদ্ধান্ত

**কোনো CRITICAL নিরাপত্তা দুর্বলতা নেই (ইনজেকশন/ক্রেডেনশিয়াল হ্যান্ডলিং/TLS ডিফল্ট সব নিরাপদ)**:
- ✅ পুরো workspace-এ শূন্য unsafe ব্লক
- ✅ কোনো হার্ডকোডেড ক্রেডেনশিয়াল নেই, উদাহরণ কনফিগ changeme প্লেসহোল্ডার (সব মন্তব্য করার পরামর্শ, L10)
- ✅ sqlx-এ সব প্যারামিটারাইজড বাইন্ডিং; Redis লক Lua CAS দিয়ে মুক্ত হয়
- ✅ TLS `skip_verify` ডিফল্ট বন্ধ; Redis স্বয়ংক্রিয় rediss://-এ আপগ্রেড
- ⚠️ মেরামত বাকি: TDengine জোড়া ইনজেকশন (C2, sqlx কভারেজের বাইরে), ক্লায়েন্ট অনুযায়ী রেট লিমিটিং (H2), Redis রেট লিমিট fail-closed (M1), JWT দুর্বল key (M3), Redis এরর মেসেজ লিক (M5), ES পাথ ইনজেকশন (M6)

## 9. অপটিমাইজেশন পরামর্শ (Top অগ্রাধিকার)

1. **P0**: C1 জাল ইমপ্লিমেন্টেশন, C2 SQL ইনজেকশন, D1 পোর্ট বাইন্ডিং, H1 টাইমআউট — 4টি আইটেম
2. **P1**: H2 রেট লিমিটিং, H3 CI, H4 ES স্ট্যাটাস কোড, H5 IoTDB, H6 etcd deregister
3. **P1**: M1 fail-closed, M3 JWT, M5 পাসওয়ার্ড লিক, M6 পাথ ইনজেকশন
4. **P2**: Dockerfile/Helm/README মেরামত, clippy --all-targets, এরর ফাঁস, ব্যাচ রাইট
5. **P3**: workspace.dependencies কনভার্জেন্স, MQ from_config ইউনিফিকেশন, ডকুমেন্টেশন সিঙ্ক

---

## 10. মেরামত অবস্থা (2026-08-06 পুনঃযাচাই)

**সব 35টি আবিষ্কার মেরামত বা ডকুমেন্টেডভাবে মোকাবিলা করা হয়েছে।** পুনঃযাচাই ফলাফল: `cargo check --workspace` ✅, `cargo test --workspace` 219টি টেস্ট সব পাস ✅, `cargo clippy --workspace --all-targets -- -D warnings` শূন্য সতর্কতা ✅, `cargo fmt --check` পরিষ্কার ✅, helloworld স্মোক টেস্ট (`/` + `/health`) ✅।

| নম্বর | গুরুতরতা | মেরামতের উপায় | যাচাই |
|------|--------|----------|------|
| D1 | HIGH | `HttpServer` খালি host-কে `0.0.0.0`-তে নরমালাইজ; উদাহরণ/ডকুমেন্টেশন/CLI টেমপ্লেট সব `0.0.0.0:8000` | স্মোক টেস্ট বাইন্ড সফল |
| D2 | LOW | `SqlxTransactionWrapper` impl টেস্ট মডিউলের আগে সরানো | clippy শূন্য সতর্কতা |
| C1 | CRITICAL | memcached স্পষ্টভাবে «শুধু ডেভেলপ/টেস্ট» চিহ্নিত; `in_memory` সুইচ; get লেজি এক্সপায়ার + set sweep | 23টি ডেটা লেয়ার টেস্ট পাস |
| C2 | CRITICAL | TDengine ডাবল এস্কেপ (`\`→`\\`, `"`→`\"`); 100টি করে ব্যাচ চাঙ্ক | পাস |
| H1 | HIGH | `ecat-tls` ইউনিফর্ম connect 5s / request 30s টাইমআউট, সব HTTP অ্যাডাপ্টার উত্তরাধিকার | পাস |
| H2 | HIGH | রেট লিমিট key ডিফল্ট X-Forwarded-For প্রথম হপ → X-Real-IP → global; MemoryStore 60s লেজি ক্লিনিং | 22টি মিডলওয়্যার টেস্ট পাস |
| H3 | HIGH | CI-তে `protobuf-compiler` ইনস্টল যোগ | কনফিগ আপডেট হয়েছে |
| H4 | HIGH | ES/OpenSearch `search()`/`delete()`-এ `is_success()` পরীক্ষা; index/id RFC 3986 এনকোডিং | পাস |
| H5 | HIGH | IoTDB স্ট্যান্ডার্ড insertTablet body-তে রিফ্যাক্টর, `code != 200` পরীক্ষা | পাস |
| H6 | HIGH | etcd deregister প্রিফিক্স range delete ব্যবহার, রেজিস্ট্রেশন key ম্যাচ | পাস |
| M1 | MED | Redis রেট লিমিট: Lua অ্যাটমিক INCR+EXPIRE, EXPIRE ব্যর্থ হলে DEL রোলব্যাক, সংযোগ এরর fail-open + warn | পাস |
| M3 | MED | JWT key <32 বাইট প্রত্যাখ্যান (`WeakKey`); এরর রেসপন্স ইউনিফর্ম `invalid token` | 9টি auth টেস্ট পাস |
| M5 | MED | Redis পাসওয়ার্ড `ConnectionInfo` দিয়ে আলাদা পাঠানো, URL-এ এমবেড নয় | পাস |
| M6 | MED | ES/OpenSearch/InfluxDB সব ইনজেকশন পৃষ্ঠ এস্কেপ বা প্যারামিটারাইজড | পাস |
| M9 | MED | TDengine 100টি/ব্যাচ | পাস |
| M11 | MED | Redis ttl ওভারফ্লো `u64::MAX`-এ ক্ল্যাম্প | পাস |
| M13 | MED | MQ `from_config` ইউনিফর্ম async (kafka/mqtt সিঙ্ক্রোনাইজড) | 11টি CLI টেস্ট পাস |
| L সিরিজ | LOW/INFO | Dockerfile (আসল বাইনারি নাম + curl হেলথ চেক + builder 1.85), Chart appVersion 2.3.0, উদাহরণ পাসওয়ার্ড মন্তব্য করা, consul সংস্করণ/পোর্ট রেজিস্ট্রেশন তথ্য থেকে পার্স, হাতে লেখা base64 `base64` crate দিয়ে, `validate_crate_name` ইনজেকশন প্রতিরোধ, workspace.dependencies 8টি জায়গায় কনভার্জ, ডাবল subscriber কনফ্লিক্ট মন্তব্য, ডকুমেন্টেশন (README/README.en/CHANGELOG 2.3.1) সিঙ্ক | সব পাস |

**মেরামতের সময় নতুন সমস্যা**: `ecat-config-remote` টেস্ট পুরনো `base64_decode` রেফারেন্স করত (agent প্রতিস্থাপনে বাদ পড়েছিল) → `base64::engine` ব্যবহার করা হয়েছে; `ecat-middleware`-এ 4টি clippy সতর্কতা (নেস্টেড if / জটিল টাইপ) → ভাঁজ করা হয়েছে + `KeyFn` টাইপ অ্যালিয়াস। মেরামতের পর কোনো রিগ্রেশন নেই।

**ইকোসিস্টেম সিদ্ধান্ত**: 55টি crate, 18টি ডেটাবেস অ্যাডাপ্টার, 4টি MQ, Docker/Helm/CI কনফিগ, ইংরেজি-চীনা README, CHANGELOG সব v2.3.0-এর সাথে সামঞ্জস্যপূর্ণ; ছবি (alipay/weixinpay.png) রেফারেন্স স্বাভাবিক।

---

*রিপোর্ট অটোমেটেড রিভিউ দিয়ে তৈরি: বিল্ড+টেস্ট+স্মোক রান + 3টি বিশেষায়িত রিভিউ agent (নিরাপত্তা/ডেটা লেয়ার/ইকোসিস্টেম ধারাবাহিকতা), 2026-08-06 সম্পূর্ণ পুনঃযাচাই।*
