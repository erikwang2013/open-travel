<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat API রেফারেন্স

এই পৃষ্ঠায় Ecat ফ্রেমওয়ার্কের ইন্টারফেস (API) সারফেসের সারসংক্ষেপ দেওয়া হয়েছে: পোর্ট কনভেনশন, বিল্ট-ইন এন্ডপয়েন্ট, এরর ফরম্যাট ও এক্সটেনশন ইন্টারফেস। ব্যবসায়িক রাউটিং প্রতিটি সার্ভিস নিজে রেজিস্টার করে।

## পোর্ট কনভেনশন

| প্রোটোকল | লিসেনিং ঠিকানা | ব্যাখ্যা |
|------|----------|------|
| HTTP | `0.0.0.0:8000` | axum রাউটিং, ডিফল্ট উদাহরণ পোর্ট |
| gRPC | `0.0.0.0:9000` | tonic Server, ডিফল্ট উদাহরণ পোর্ট |

## বিল্ট-ইন এন্ডপয়েন্ট

নিচের এন্ডপয়েন্টগুলো ইকোসিস্টেম crates থেকে আসে, সার্ভিসের সাথে মাউন্ট হয়:

| এন্ডপয়েন্ট | উৎস | ব্যাখ্যা |
|------|------|------|
| `/health` | ecat-health | লিভনেস চেক (সার্ভিস নাম, ভার্সন, স্টার্ট টাইম রিটার্ন করে) |
| `/ready` | ecat-health | রেডিনেস চেক (ডিপেন্ডেন্সি প্রস্তুত হলে 200 রিটার্ন) |
| `/metrics` | ecat-metrics | Prometheus মেট্রিক এক্সপোজ (`ecat_http_requests_total` / `ecat_http_request_duration_seconds`) |
| `/{service}/{method}` | ব্যবহারকারী রাউট | উদাহরণ: `/helloworld/ecat` |

> মেট্রিক এন্ডপয়েন্ট পাথে ID-এর মতো উচ্চ-কার্ডিনালিটি হলে `MetricsLayer::new().with_path_fn(...)` দিয়ে নরমালাইজ করুন, মেট্রিক কার্ডিনালিটি বিস্ফোরণ এড়াতে।

## রিকোয়েস্ট প্রসেসিং ফ্লো

```
ক্লায়েন্ট রিকোয়েস্ট
  ├─ HTTP :8000 ──→ axum::Router ─┐
  └─ gRPC :9000 ──→ tonic::Server ─┤
                              ┌─────┴──────┐
                              │ Middleware │  Recovery→Tracing→Logging→Auth→Metrics→Security→CircuitBreaker
                              └─────┬──────┘
                                    ▼
                               Handler（tower::Service）
                                    ▼
                               Response（JSON/Protobuf এনকোডিং）
```

## এরর ফরম্যাট

`ecat-errors` `ErrorCode` + `Error` প্রদান করে, কম্পাইল-টাইমে HTTP স্ট্যাটাস কোড ম্যাপিং:

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

এরর রেসপন্স middleware-এর মাধ্যমে JSON (বা Protobuf) এ এনকোড হয়, code / reason / message বহন করে।

## এক্সটেনশন ইন্টারফেস

| ক্ষমতা | Crate | ইন্টারফেস |
|------|-------|------|
| GraphQL | ecat-graphql | `/graphql` এন্ডপয়েন্ট; ফিল্ড প্যারামিটার ও নেস্টেড selection সমর্থন করে, alias, fragment ও মাল্টি-টপ-লেভেল ফিল্ড সমর্থন করে না |
| OpenAPI | ecat-openapi | রাউট থেকে OpenAPI spec জেনারেট |
| WebSocket | ecat-transport-ws | আপগ্রেডেড WS ট্রান্সপোর্ট |
| API ভার্সন রাউটিং | ecat-versioning | `/v1/...` প্রিফিক্স ভার্সন রাউটিং |
| অথেনটিকেশন | ecat-auth | JWT / API Key মিডলওয়্যার; JWT সিক্রেট ≥32 বাইট, চেইনযোগ্য `required_issuer`/`required_audience` |
| gRPC ক্লায়েন্ট | ecat-transport-grpc | সার্ভিস ডিসকভারি ও লোড ব্যালেন্সিং একীভূত |

## সার্ভিস-টু-সার্ভিস কমিউনিকেশন

- `HttpClient`（ecat-client）：সার্ভিস ডিসকভারি ও লোড ব্যালেন্সিং একীভূত, CircuitBreaker সার্কিট ব্রেকার সুরক্ষা
- `GrpcClient`（ecat-transport-grpc）：একই, gRPC প্রোটোকলে
- মিডলওয়্যার ইউনিফাইড `tower::ServiceBuilder` দিয়ে কম্পোজ (Recovery / Tracing / Logging / Timeout / RateLimit / Security / CircuitBreaker / Metrics / Retry / Validate / CORS)

## ডেটা ব্যাকএন্ড ইন্টারফেস

সব ডেটা ব্যাকএন্ড (`ecat-data-*`) ইউনিফাইড trait (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`) অ্যাবস্ট্রাকশনের মাধ্যমে; REST-টাইপ ব্যাকএন্ডগুলো (Neo4j / NebulaGraph / ArangoDB / InfluxDB / IoTDB / QuestDB / TDengine / OpenSearch / Elasticsearch / S3) `base_url`-এর মাধ্যমে সংশ্লিষ্ট HTTP ইন্টারফেস অ্যাক্সেস করে। সংযোগ কনফিগ দেখুন [ডেটাবেস কনফিগ টিউটোরিয়াল](database-config-tutorial.md)。
