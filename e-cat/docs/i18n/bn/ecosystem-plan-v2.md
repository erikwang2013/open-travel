# e-cat ইকোসিস্টেম পরিকল্পনা v2 — সম্পন্ন ও পরবর্তী

**ভার্সন:** 2.1.7  
**তারিখ:** 2026-08-01  
**অবস্থা:** সব পরিকল্পনা সম্পন্ন, 47 crates

---

## এক、সম্পন্ন (সব ডেলিভারি)

| পর্ব | Crate | ক্ষমতা | টেস্ট |
|------|-------|------|------|
| পর্ব 1 | `ecat-health` | হেলথ চেক (/health、/ready) | 4 |
| পর্ব 1 | `ecat-client` | HTTP/gRPC ক্লায়েন্ট + সার্ভিস ডিসকভারি + লোড ব্যালেন্সিং | 7 |
| পর্ব 1 | `ecat-circuit-breaker` | থ্রি-স্টেট সার্কিট ব্রেকার (Tower Layer) | 4 |
| পর্ব 1 | `ecat-auth` | JWT + API Key + OAuth2 অথেনটিকেশন মিডলওয়্যার | 8 |
| পর্ব 1 | `ecat-registry-consul` | Consul সার্ভিস রেজিস্ট্রি | 2 |
| পর্ব 2 | `ecat-data-redis` | Redis ক্যাশ (Cache trait) | 1 |
| পর্ব 2 | `ecat-mq` | মেসেজ কিউ অ্যাবস্ট্রাকশন + InMemoryMq | 2 |
| পর্ব 2 | `ecat-events` | লোকাল + রিমোট ইভেন্ট বাস | 2 |
| পর্ব 2 | `ecat-config-remote` | Consul KV রিমোট কনফিগ | 2 |
| পর্ব 3 | `ecat-testing` | MockServer + ChaosConfig | 5 |
| পর্ব 3 | `ecat-openapi` | OpenAPI 3.0 spec জেনারেশন | 2 |
| পর্ব 3 | `ecat-bench` | কনকারেন্সি পারফরম্যান্স বেঞ্চমার্ক | 2 |
| পর্ব 3 | `ecat-deploy` | Dockerfile + K8s + Helm | — |
| পর্ব 4 | `ecat-tracing` | ডিস্ট্রিবিউটেড ট্রেসিং (span + trace_id) | 2 |
| পর্ব 4 | `ecat-client` এক্সটেনশন | GrpcClient + TlsConfig | — |
| পর্ব 4 | `ecat-auth` এক্সটেনশন | OAuth2Layer | — |
| পর্ব 5 | `ecat-registry-etcd` | etcd সার্ভিস রেজিস্ট্রি | 4 |
| পর্ব 5 | `ecat-mq-kafka` | Kafka মেসেজ কিউ | 1 |
| পর্ব 5 | `ecat-data-opensearch` | OpenSearch সার্চ | 1 |
| পর্ব 5 | `ecat-data-influxdb` | InfluxDB টাইম-সিরিজ | 2 |
| পর্ব 5 | `ecat-data-elasticsearch` | Elasticsearch সার্চ | 2 |
| পর্ব 5 | `ecat-data-clickhouse` | ClickHouse OLAP | 1 |
| পর্ব 5 | `ecat-data-memcached` | Memcached ক্যাশ | 3 |
| পর্ব 5 | `ecat-data-neo4j` | Neo4j গ্রাফ ডেটাবেস | 1 |
| পর্ব 5 | `ecat-data-nebulagraph` | NebulaGraph গ্রাফ ডেটাবেস | 1 |
| পর্ব 5 | `ecat-data-arangodb` | ArangoDB গ্রাফ ডেটাবেস | 1 |
| পর্ব 5 | `ecat-data-iotdb` | IoTDB টাইম-সিরিজ | 1 |
| পর্ব 5 | `ecat-data-questdb` | QuestDB টাইম-সিরিজ | 1 |
| পর্ব 6 | `ecat-transport-ws` | WebSocket সাপোর্ট | 2 |
| পর্ব 6 | `ecat-versioning` | API ভার্সন রাউটিং | 2 |
| পর্ব 6 | `ecat-graphql` | GraphQL endpoint | 9 |
| পর্ব 6 | CI/CD টেমপ্লেট | GitHub Actions | — |

---

## দুই、অবশিষ্ট ফাঁক (৩টি)

| # | ফাঁক | কাজের পরিমাণ |
|---|------|--------|
| 1 | **mTLS ট্রান্সপোর্টে যুক্ত করা** | ছোট |
| 2 | **Redis রেট-লিমিট ব্যাকএন্ড** | ছোট |
| 3 | **GitLab CI টেমপ্লেট** | ছোট |

---

## তিন、ভার্সন রোডম্যাপ

```
v1.0.x  কোর স্কেলেটন (18 crates)                    ✅ সম্পন্ন
v2.0.x  ইকোসিস্টেম পর্ব 1~3 (+13 crates = 31 total)   ✅ সম্পন্ন
v2.1.x  কমিউনিকেশন ও সিকিউরিটি + ডেটা ব্যাকএন্ড + অপারেশন অভিজ্ঞতা             ✅ সম্পন্ন (বর্তমান 47 crates)
```

## চার、ইকোসিস্টেমে অন্তর্ভুক্ত নয়

| প্রয়োজন | সমাধান | কারণ |
|------|------|------|
| API গেটওয়ে | Kong / Envoy | ভাষা-নিরপেক্ষ |
| সার্ভিস মেশ | Linkerd | Rust-এ পরিণত সমাধান নেই |
| কনটেইনার অর্কেস্ট্রেশন | Kubernetes | ইন্ডাস্ট্রি স্ট্যান্ডার্ড |
| লগ সংগ্রহ | Vector | Rust নেটিভ |
