<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# مرجع API الخاص بـ Ecat

تصف هذه الصفحة سطح واجهات (API) إطار عمل Ecat: اصطلاحات المنافذ، والنقاط النهائية المدمجة، وتنسيق الأخطاء، والواجهات الموسّعة. يتم تسجيل مسارات الأعمال من قبل كل خدمة بنفسها.

## اصطلاحات المنافذ

| البروتوكول | عنوان الاستماع | الوصف |
|------|----------|------|
| HTTP | `0.0.0.0:8000` | توجيه axum، منفذ المثال الافتراضي |
| gRPC | `0.0.0.0:9000` | خادم tonic، منفذ المثال الافتراضي |

## النقاط النهائية المدمجة

توفر النقاط النهائية التالية من crates النظام البيئي، وتُركَّب مع الخدمة:

| النقطة النهائية | المصدر | الوصف |
|------|------|------|
| `/health` | ecat-health | فحص البقاء على قيد الحياة (يُرجع اسم الخدمة والإصدار ووقت البدء) |
| `/ready` | ecat-health | فحص الجاهزية (يُرجع 200 بعد جاهزية التبعيات) |
| `/metrics` | ecat-metrics | كشف مقاييس Prometheus (`ecat_http_requests_total` / `ecat_http_request_duration_seconds`) |
| `/{service}/{method}` | مسارات المستخدم | مثال: `/helloworld/ecat` |

> في السيناريوهات عالية الكاردينالية مثل مسارات تحتوي على معرّفات، استخدم `MetricsLayer::new().with_path_fn(...)` لتطبيع مسار المقاييس وتجنب انفجار الكاردينالية.

## تدفق معالجة الطلبات

```
طلب العميل
  ├─ HTTP :8000 ──→ axum::Router ─┐
  └─ gRPC :9000 ──→ tonic::Server ─┤
                              ┌─────┴──────┐
                              │ Middleware │  Recovery→Tracing→Logging→Auth→Metrics→Security→CircuitBreaker
                              └─────┬──────┘
                                    ▼
                               Handler (tower::Service)
                                    ▼
                               Response (ترميز JSON/Protobuf)
```

## تنسيق الأخطاء

يوفر `ecat-errors` `ErrorCode` + `Error`، مع ربط أكواد حالة HTTP في وقت الترجمة:

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

تُرمَّز استجابات الأخطاء عبر middleware إلى JSON (أو Protobuf)، وتحمل code / reason / message.

## الواجهات الموسّعة

| القدرة | Crate | الواجهة |
|------|-------|------|
| GraphQL | ecat-graphql | نقطة `/graphql`؛ تدعم معاملات الحقول وselection المتداخلة، ولا تدعم aliases ولا fragments ولا حقولًا متعددة على المستوى الأعلى |
| OpenAPI | ecat-openapi | توليد مواصفات OpenAPI من المسارات |
| WebSocket | ecat-transport-ws | نقل WS مُرقّى |
| توجيه إصدارات API | ecat-versioning | توجيه الإصدارات ببادئة `/v1/...` |
| المصادقة | ecat-auth | وسائط JWT / API Key؛ يجب أن يكون مفتاح JWT ≥32 بايت، مع دعم `required_issuer`/`required_audience` المتسلسل |
| عميل gRPC | ecat-transport-grpc | تكامل اكتشاف الخدمات وموازنة الحمل |

## التواصل بين الخدمات

- `HttpClient` (ecat-client): يدمج اكتشاف الخدمات وموازنة الحمل، مع حماية عبر CircuitBreaker
- `GrpcClient` (ecat-transport-grpc): كما سبق، عبر بروتوكول gRPC
- تُركَّب الوسائط بشكل موحد عبر `tower::ServiceBuilder` (Recovery / Tracing / Logging / Timeout / RateLimit / Security / CircuitBreaker / Metrics / Retry / Validate / CORS)

## واجهات خلفيات البيانات

جميع خلفيات البيانات (`ecat-data-*`) مُجرّدة عبر traits موحّدة (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`)؛ تصل خلفيات نمط REST (Neo4j / NebulaGraph / ArangoDB / InfluxDB / IoTDB / QuestDB / TDengine / OpenSearch / Elasticsearch / S3) إلى واجهات HTTP المقابلة عبر `base_url`. راجع [برنامج تعليمي لإعداد قاعدة البيانات](database-config-tutorial.md) لإعدادات الاتصال.
