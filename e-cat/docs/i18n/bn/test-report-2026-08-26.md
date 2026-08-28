<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# টেস্ট রিপোর্ট — 2026-08-26

সর্বাঙ্গীণ ইউনিট টেস্ট সম্পূরক (51 crate পূর্ণ কভারেজ), 4 দল সিনিয়র Rust টেস্ট ইঞ্জিনিয়ার সমান্তরালে।

## ওভারভিউ

| দল | crates | পূর্ববর্তী | নতুন যোগ | বর্তমান | গেট |
|---|---|---|---|---|---|
| core/ফ্রেমওয়ার্ক | 12 | 102 | +40 | 142 | ✅ টেস্ট সব সবুজ + clippy 0 warning |
| data | 14 | 87 | +66 | 153 | ✅ একই |
| mq/transport | 12 | 82 | +54 | 136 | ✅ একই |
| app অ্যাপ্লিকেশন স্তর | 13 | ~178 | +46 | ~224 | ✅ একই |
| **মোট** | **51** | **~449** | **+206** | **~655** | ✅ |

নোট: অ্যাপ্লিকেশন স্তরের পূর্ববর্তী সংখ্যায় ecat-auth 24 / ecat-graphql 35 / ecat-bench 11 / ecat-scheduler 6 / ecat-events 7 / ecat-cli 12 / ecat-middleware 34 / ecat-circuit-breaker 10 / ecat-health 4 / ecat-client 7 / ecat-security 12 / ecat-versioning 4 / ecat 4 অন্তর্ভুক্ত। প্রতিটি crate-এর আলাদা `cargo test -p` + `cargo clippy -p --all-targets -- -D warnings` সব পাস, CARGO_TARGET_DIR বিচ্ছিন্ন করে সমান্তরালে চালানো হয়েছে।

## প্রতি crate বিস্তারিত

### core/ফ্রেমওয়ার্ক দল (test-core, +40)

| crate | পূর্ববর্তী→নতুন | কভারেজ পয়েন্ট |
|---|---|---|
| ecat-protos | 4→8 | ErrorCode পূর্ণ এনাম proto-র সাথে তুলনা; ট্রাঙ্কেটেড buffer decode; খালি buffer ডিফল্ট মেসেজ; metadata roundtrip |
| ecat-errors | 4→9 | http_status পূর্ণ ম্যাপিং (409/429/500); from_status; অম্যাপড→Internal; cause source() |
| ecat-metadata | 9→12 | HTTP header থেকে trace_id এক্সট্র্যাকশন; key ছোট হাতেরকরণ; খালি header map |
| ecat-encoding | 18→22 | NaN→null (serde_json ডিফল্ট, ডকুমেন্টেড); খালি বাইট decode; CodecBox অবৈধ JSON; proto roundtrip |
| ecat-lock | 7→9 | লক না ধরে release করলে এরর; খালি key |
| ecat-logging | 1→1 | কম্প্যাটিবিলিটি shim panic করে না |
| ecat-tracing | 9→12 | নন-UTF-8 trace header স্কিপ; canonical header; রেসপন্স ট্রান্সমিশন |
| ecat-tls | 7→12 | basic_auth এক/দুই ফিল্ড; ca ফাইল নেই; is_enabled; ডিফল্ট ক্লায়েন্ট |
| ecat-config | 14→26 | env প্রিফিক্স ফিল্টার+টাইপ পার্স বাউন্ডারি (hex/খালি স্ট্রিং/-0/1e3); একাধিক source মার্জ ওভাররাইড; obfs এরর পাথ; ফাইল অনুপস্থিত/অবৈধ YAML |
| ecat-config-remote | 6→9 | ConsulKvEntry বাউন্ডারি; X-Consul-Index নেই এরর; নেস্টেড key |
| ecat-openapi | 4→11 | components/schema_ref; ডুপ্লিকেট ওভাররাইড; ডিফল্ট 200; tags |
| ecat-metrics | 8→11 | রেজিস্টার্ড মেট্রিক টেক্সট; 404/405 |

### data দল (test-data, +66)

| crate | পূর্ববর্তী→নতুন | কভারেজ পয়েন্ট |
|---|---|---|
| ecat-data | 12→14 | সার্চ সিনট্যাক্স পার্সিং |
| ecat-data-sqlx | 7→14 | মেমরি SQLite এন্ড-টু-এন্ড; প্যারামিটার বাইন্ডিং পূর্ণ টাইপ; Blob→base64; config |
| ecat-data-redis | 6→12 | redis:///rediss:// URL নির্মাণ; auth; config এরর পাথ |
| ecat-data-opensearch | 4→10 | mock HTTP: percent-encode, Basic auth, এরর ট্রান্সমিশন |
| ecat-data-elasticsearch | 6→11 | একই |
| ecat-data-influxdb | 5→10 | line protocol এস্কেপ; Token header; এরর ট্রান্সমিশন |
| ecat-data-clickhouse | 12→22 | টেবিল তৈরি SQL; JSONEachRow; রাইট সারি সংখ্যা; গ্রুপিং |
| ecat-data-memcached | 4→8 | TTL সেকেন্ড→মিলিসেকেন্ড; flag প্যাকিং |
| ecat-data-nebulagraph | 6→7 | config পার্স |
| ecat-data-arangodb | 5→7 | config/URL |
| ecat-data-iotdb | 5→10 | mock HTTP: session পাথ প্যারামিটার |
| ecat-data-questdb | 4→9 | line protocol; ট্রানজেকশন সাপোর্ট নেই |
| ecat-data-tdengine | 6→11 | INSERT জেনারেশন; 100টি ব্যাচ চাঙ্ক |
| ecat-data-mongodb | 5→8 | bson রাউন্ডট্রিপ; URI |

### mq/transport/registry দল (test-mq, +54)

| crate | পূর্ববর্তী→নতুন | কভারেজ পয়েন্ট |
|---|---|---|
| ecat-mq | 5→9 | পূর্ণ বাফার ল্যাগ এরর ফ্রেম; সব drop হলে স্ট্রিম বন্ধ; একাধিক সাবস্ক্রাইবার; সাবস্ক্রাইবার ছাড়া publish |
| ecat-mq-kafka | 12→14 | config ডিফল্ট; SASL ফিল্ড আলাদাভাবে কার্যকর |
| ecat-mq-rabbitmq | 2→5 | exchange ডিফল্ট; url এরর পাথ |
| ecat-mq-mqtt | 5→9 | cert/key পেয়ারিং যাচাই; ফাইল অনুপস্থিত; পোর্ট ডিফল্ট 1883/8883; অবৈধ পোর্ট ফলব্যাক |
| ecat-mq-nats | 6→9 | প্লেইনটেক্সট ডিফল্ট; ca/cert অনুপস্থিত এরর পাথ |
| ecat-transport | 4→7 | TlsConfig ডিফল্ট/with_client_auth; normalize_addr বাউন্ডারি |
| ecat-transport-http | 17→20 | ইন্টিগ্রেশন টেস্ট: stop খালি অপারেশন, পোর্ট দখল ব্যর্থতা, প্রকৃত পাঠানো/গ্রহণ |
| ecat-transport-grpc | 7→13 | TLS ফাইল অনুপস্থিত; প্লেইনটেক্সট লাইফসাইকেল; mTLS প্রত্যাখ্যান |
| ecat-transport-ws | 4→8 | handler ছাড়া ব্যর্থতা; পোর্ট দখল; RFC 6455 masked ফ্রেম ইকো |
| ecat-registry | 5→8 | মাল্টি-ইন্সট্যান্স discover; drop-এ স্বয়ংক্রিয় ডিরেজিস্টার; builder ডিফল্ট |
| ecat-registry-consul | 10→24 | percent-encode; রেজিস্ট্রেশন ভেরিয়েন্ট; এরর রেসপন্স; X-Consul-Token; agent/services পার্স; node ফলব্যাক |
| ecat-registry-etcd | 5→10 | discover খারাপ মান স্কিপ; kv রিকোয়েস্ট বডি; lease grant; keepalive |

### app অ্যাপ্লিকেশন স্তর দল (test-app, +46)

| crate | পূর্ববর্তী→নতুন | কভারেজ পয়েন্ট |
|---|---|---|
| ecat-auth | 20→46 | oauth2 ক্যাশ ওয়াইটলিস্ট/SHA-256 key/FIFO ইভিকশন; apikey ত্রি-অবস্থা; jwt iss/aud বাধ্যতামূলক; মেয়াদোত্তীর্ণ/ভুল সিগনেচার |
| ecat-health | 4→8 | readiness অ্যাগ্রিগেশন (সব ok/যেকোনো fail/খালি রেজিস্ট্রি); liveness |
| ecat-versioning | 4→7 | path নীতি রাউটিং; extract_version বাউন্ডারি |
| ecat-security | 12→20 | header স্তর এন্ড-টু-এন্ড; আক্রমণ ইন্টারসেপ্ট JSON আকৃতি |
| ecat-middleware | 34→37 | MemoryStore উইন্ডো মেয়াদোত্তীর্ণ; ভিতরের panic→Err |
| ecat-circuit-breaker | 10→12 | half-open প্রোব নিঃশেষ; classify ডাউনগ্রেড |
| ecat-client | 7→10 | grpc অবৈধ এন্ডপয়েন্ট এরর, নেটওয়ার্কে যায় না |
| ecat-graphql | 35→35 | বিদ্যমান কভারেজ পর্যাপ্ত, কোনো ফাঁক নেই |
| ecat-scheduler / ecat-bench / ecat-events / ecat-cli / ecat | বিদ্যমান কভারেজ পর্যাপ্ত | কোনো ফাঁক নেই |

## আবিষ্কৃত ত্রুটি

| স্তর | অবস্থান | বর্ণনা | অবস্থা |
|---|---|---|---|
| P1 | ecat-events/Cargo.toml | dev-dependencies-এ tokio macros/rt/time features নেই, আলাদাভাবে সেই crate-এর টেস্ট টার্গেট কম্পাইল করলেই ব্যর্থ (workspace পূর্ণ বিল্ডে feature যুক্ত করে ঢাকা পড়ে) | ✅ মেরামতকৃত (features + মন্তব্য যোগ) |
| P2 | ecat-security src/lib.rs:118-127 | URI পার্সেন্ট-এনকোডেড SQLi (`?q=SELECT%20*%20...`) header স্তরের স্ক্যান বাইপাস করতে পারে (ডিটেক্টরের আক্ষরিক স্পেস দরকার, কাঁচা URI স্ক্যান করে আগে ডিকোড করে না); বডি স্ক্যান প্রভাবিত নয় | ⏳ মেরামত বাকি |
| P3 | ecat-data-sqlx | `connect()/from_config()` AnyPool ব্যবহার করে কিন্তু ড্রাইভার ইনস্টল নেই, sqlx 0.8.6-এ প্রথম সংযোগেই panic "No drivers installed" | ⏳ মেরামত বাকি |
| P3 | ecat-data-influxdb | স্ট্রিং field-এ স্পেস এস্কেপ করা হয়েছে (`\ `), line protocol স্পেক-এ শুধু `"` ও `\` এস্কেপ দরকার; tag/field ক্রম নন-ডিটারমিনিস্টিক | ⏳ মেরামত বাকি |
| P3 | ecat-data-clickhouse | টেবিল তৈরি ক্যাশ কখনো অমূল্য হয় না, বাইরে drop/টেবিল পরিবর্তনের পর CREATE পুনরায় চেষ্টা হয় না | ⏳ মেরামত বাকি |
| P3 | ecat-circuit-breaker | half_open_probes সীমা সিকোয়েন্সিয়াল প্রোবের অধীনে অপ্রাপ্য (শুধু কনকারেন্ট ইন-ফ্লাইটে প্রাপ্য), হোয়াইট-বক্স টেস্ট দিয়ে কভার করা হয়েছে | ℹ️ পরিচিত, ত্রুটি নয় |
| P3 | ecat-health | `with_check` blocking_write() ব্যবহার করে, async কনটেক্সটে কল করলে panic; বর্তমানে শুধু সিঙ্ক্রোনাস কনটেক্সটে ব্যবহারযোগ্য | ℹ️ পরিচিত, API সীমাবদ্ধতা |

## স্কিপ করা মডিউল (ইন্টিগ্রেশন পরিবেশ দরকার, mock করা হয়নি)

- প্রকৃত broker রাউন্ডট্রিপ: kafka/rabbitmq/mqtt/nats publish-subscribe (কনফিগ ও এরর পাথ কভার করা হয়েছে)
- প্রকৃত ক্লাস্টার: consul/etcd রেজিস্ট্রেশন-ডিসকভারি লাইফসাইকেল (axum mock রিকোয়েস্ট আকৃতি কভার করে)
- প্রকৃত ডেটাবেস: redis/memcached অপারেশন, mongod, influxdb সার্ভার-সাইড যাচাই, sqlx postgres/mysql ড্রাইভার, nebulagraph/arangodb API
- প্রকৃত বাহ্যিক সার্ভিস: OAuth2 introspection (লোকাল mock কভার), gRPC/HTTP রাউন্ডট্রিপ (লোকাল mock 302 ফলো না করা কভার)
