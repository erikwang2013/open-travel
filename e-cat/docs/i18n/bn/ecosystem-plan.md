# e-cat ইকোসিস্টেম পরিকল্পনা

**ভার্সন:** 2.1.7  
**তারিখ:** 2026-08-01  
**অবস্থা:** সব সম্পন্ন · 47 crates

| ক্ষেত্র | কভারেজ | অবস্থা |
|------|--------|------|
| ট্রান্সপোর্ট স্তর | HTTP (axum), gRPC (tonic), WebSocket | ✅ |
| এনকোডিং | JSON, Protobuf | ✅ |
| মিডলওয়্যার | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth (JWT/API Key/OAuth2) | ✅ |
| কনফিগ | env, file (JSON/YAML), Consul KV রিমোট, এনক্রিপশন | ✅ |
| রেজিস্ট্রি | memory, Consul, etcd | ✅ |
| সিকিউরিটি | আক্রমণ সনাক্তকরণ, JWT, API Key, OAuth2, TlsConfig | ✅ |
| ডেটা | RDBMS (sqlx), Redis, Memcached, OpenSearch, Elasticsearch, ClickHouse, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB | ✅ |
| অবজারভেবিলিটি | tracing, Prometheus, Health, ডিস্ট্রিবিউটেড ট্রেসিং | ✅ |
| কমিউনিকেশন | HTTP/gRPC Client, MessageQueue (InMemory/Kafka), EventBus | ✅ |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | ✅ |
| API টুল | OpenAPI, Versioning, GraphQL | ✅ |

## অবশিষ্ট ফাঁক (৩টি ছোট অপটিমাইজেশন)

1. **mTLS ট্রান্সপোর্টে যুক্ত করা** — TlsConfig আছে, HttpServer/GrpcServer-এ যুক্ত হয়নি
2. **Redis রেট-লিমিট ব্যাকএন্ড** — RateLimitLayer শুধুমাত্র মেমরি, মাল্টি-ইনস্ট্যান্সে শেয়ার দরকার
3. **GitLab CI টেমপ্লেট** — বর্তমানে শুধুমাত্র GitHub Actions

## ভার্সন বিবর্তন

```
v1.0.x  কোর স্কেলেটন (18 crates)                    ✅
v2.0.x  ইকোসিস্টেম পর্ব 1~3 (+13 crates)             ✅
v2.1.x  কমিউনিকেশন ও সিকিউরিটি শক্তিশালীকরণ + ডেটা ব্যাকএন্ড + অপারেশন অভিজ্ঞতা   ✅ (বর্তমান)
```

## ইকোসিস্টেমে অন্তর্ভুক্ত নয়

| প্রয়োজন | সমাধান | কারণ |
|------|------|------|
| API গেটওয়ে | Kong / Envoy | ভাষা-নিরপেক্ষ |
| সার্ভিস মেশ | Linkerd | Rust-এ পরিণত সমাধান নেই |
| কনটেইনার অর্কেস্ট্রেশন | Kubernetes | ইন্ডাস্ট্রি স্ট্যান্ডার্ড |
| লগ সংগ্রহ | Vector | Rust নেটিভ |
