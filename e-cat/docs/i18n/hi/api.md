<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat API संदर्भ

यह पृष्ठ Ecat फ्रेमवर्क के API सतह का सारांश प्रस्तुत करता है: पोर्ट सम्मेलन, अंतर्निहित एंडपॉइंट, त्रुटि प्रारूप और विस्तार इंटरफ़ेस। व्यावसायिक रूट प्रत्येक सेवा द्वारा स्वयं पंजीकृत किए जाते हैं।

## पोर्ट सम्मेलन

| प्रोटोकॉल | सुनने का पता | स्पष्टीकरण |
|------|----------|------|
| HTTP | `0.0.0.0:8000` | axum रूट, डिफ़ॉल्ट उदाहरण पोर्ट |
| gRPC | `0.0.0.0:9000` | tonic Server, डिफ़ॉल्ट उदाहरण पोर्ट |

## अंतर्निहित एंडपॉइंट

निम्न एंडपॉइंट इकोसिस्टम crates द्वारा प्रदान किए जाते हैं, सेवा के साथ माउंट होते हैं:

| एंडपॉइंट | स्रोत | स्पष्टीकरण |
|------|------|------|
| `/health` | ecat-health | लिवनेस चेक (सेवा नाम, संस्करण, प्रारंभ समय लौटाता है) |
| `/ready` | ecat-health | रेडिनेस चेक (निर्भरताएँ तैयार होने पर 200 लौटाता है) |
| `/metrics` | ecat-metrics | Prometheus मेट्रिक्स एक्सपोज़र (`ecat_http_requests_total` / `ecat_http_request_duration_seconds`) |
| `/{service}/{method}` | उपयोगकर्ता रूट | उदाहरण: `/helloworld/ecat` |

> मेट्रिक्स एंडपॉइंट पथ में ID जैसे उच्च-कार्डिनैलिटी परिदृश्यों के लिए `MetricsLayer::new().with_path_fn(...)` से नॉर्मलाइज़ करें, मेट्रिक्स कार्डिनैलिटी विस्फोट से बचें।

## अनुरोध प्रोसेसिंग प्रवाह

```
क्लाइंट अनुरोध
  ├─ HTTP :8000 ──→ axum::Router ─┐
  └─ gRPC :9000 ──→ tonic::Server ─┤
                              ┌─────┴──────┐
                              │ Middleware │  Recovery→Tracing→Logging→Auth→Metrics→Security→CircuitBreaker
                              └─────┬──────┘
                                    ▼
                               Handler (tower::Service)
                                    ▼
                               Response (JSON/Protobuf एन्कोडिंग)
```

## त्रुटि प्रारूप

`ecat-errors` `ErrorCode` + `Error` प्रदान करता है, कंपाइल-टाइम पर HTTP स्टेटस कोड मैप करता है:

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

त्रुटि प्रतिक्रिया middleware द्वारा JSON (या Protobuf) में एन्कोड होती है, जिसमें code / reason / message होता है।

## विस्तार इंटरफ़ेस

| क्षमता | Crate | इंटरफ़ेस |
|------|-------|------|
| GraphQL | ecat-graphql | `/graphql` एंडपॉइंट; फ़ील्ड पैरामीटर और नेस्टेड selection का समर्थन करता है, alias, fragment और मल्टी-टॉप-लेवल फ़ील्ड का नहीं |
| OpenAPI | ecat-openapi | रूट से OpenAPI spec जनरेट करता है |
| WebSocket | ecat-transport-ws | अपग्रेडेड WS ट्रांसपोर्ट |
| API संस्करण रूटिंग | ecat-versioning | `/v1/...` उपसर्ग संस्करण रूटिंग |
| प्रमाणीकरण | ecat-auth | JWT / API Key मिडलवेयर; JWT कुंजी ≥32 बाइट्स होनी चाहिए, चेन किया जा सकता है `required_issuer`/`required_audience` |
| gRPC क्लाइंट | ecat-transport-grpc | सेवा खोज और लोड बैलेंसिंग के साथ एकीकृत |

## सेवा-दर-सेवा संचार

- `HttpClient` (ecat-client)：सेवा खोज और लोड बैलेंसिंग के साथ एकीकृत, CircuitBreaker सर्किट ब्रेकर सुरक्षा
- `GrpcClient` (ecat-transport-grpc)：वही, gRPC प्रोटोकॉल
- मिडलवेयर एकीकृत रूप से `tower::ServiceBuilder` से संयोजित होता है (Recovery / Tracing / Logging / Timeout / RateLimit / Security / CircuitBreaker / Metrics / Retry / Validate / CORS)

## डेटा बैकएंड इंटरफ़ेस

सभी डेटा बैकएंड (`ecat-data-*`) एकीकृत traits (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`) के माध्यम से एब्स्ट्रैक्ट किए गए हैं; REST प्रकार के बैकएंड (Neo4j / NebulaGraph / ArangoDB / InfluxDB / IoTDB / QuestDB / TDengine / OpenSearch / Elasticsearch / S3) `base_url` के आधार पर संबंधित HTTP इंटरफ़ेस तक पहुँचते हैं। कनेक्शन कॉन्फ़िगरेशन के लिए देखें [डेटाबेस कॉन्फ़िगरेशन ट्यूटोरियल](database-config-tutorial.md)।
