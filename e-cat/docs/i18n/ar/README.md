<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat

[简体中文](../../../README.md) | [English](../../../README.en.md) | [日本語](../ja/README.md) | [한국어](../ko/README.md) | [Русский](../ru/README.md) | [Deutsch](../de/README.md) | [Français](../fr/README.md) | [Español](../es/README.md) | [Português](../pt/README.md) | [हिन्दी](../hi/README.md) | **[العربية](../ar/README.md)** | [বাংলা](../bn/README.md) | [Bahasa Indonesia](../id/README.md)

الاسم الصيني لـ Ecat: 一只猫 (تُنطق "إي تشي ماو"، وتعني حرفيًا "قطة")

**一只猫 (Ecat)** هو إطار عمل للخدمات المصغرة بلغة Rust، يهدف إلى منافسة [go-kratos/kratos](https://github.com/go-kratos/kratos) الإصدار v3 (v3.0.2 · 51 crate).

يوفر تجربة تطوير API-first، وبنية مكونات قابلة للتركيب، وتجريدًا موحدًا للوسائط الوسيطة (middleware) بين HTTP/gRPC، وسلسلة أدوات CLI متكاملة. يمكن للمطوّرين الملمّين بـ Kratos البدء فورًا دون عناء، مع الاستفادة الكاملة من أمان الأنواع في Rust، وتجريدات التكلفة الصفرية، والأداء الفائق.

<p align="center">
  <img src="e-cat.svg" alt="تميمة مشروع Ecat (متحركة)" width="220" />
</p>

## البنية التصميمية

```
┌──────────────────────────────────────────────────────────────┐
│                         ecat-cli                             │
│        (new │ proto │ run --watch │ build │ upgrade)         │
├──────────────────────────────────────────────────────────────┤
│                     ecat (دورة حياة التطبيق)                  │
│      AppBuilder → App { name, servers, hooks, ... }         │
├────────────────────┬────────────────────┬────────────────────┤
│     transport      │    middleware      │     registry       │
│     ─────────      │    ──────────      │     ────────       │
│     HTTP (axum)    │    RecoveryLayer   │     memory         │
│     gRPC (tonic)   │    TracingLayer    │     consul         │
│     encoding       │    LoggingLayer    │                    │
│                    │    TimeoutLayer    │                    │
│                    │    RateLimitLayer  │                    │
│                    │    SecurityLayer   │                    │
│                    │    CircuitBreaker  │                    │
│                    │    Auth (JWT/API)  │                    │
├────────────────────┼────────────────────┼────────────────────┤
│     config         │     errors         │     metadata       │
│     ──────         │     ──────         │     ────────       │
│     file / env     │     ErrorCode      │     key-value      │
│     remote source  │     Error          │     HTTP/gRPC      │
├────────────────────┴────────────────────┴────────────────────┤
│                         طبقة البيانات                         │
│     ────────────────────────────────────────────────          │
│     rdbms:   SQLite / PostgreSQL / MySQL / TiDB              │
│     cache:   Redis ✓                                         │
│     config:  remote (Consul KV)                              │
│     registry: consul                                         │
├──────────────────────────────────────────────────────────────┤
│                       ecat-protos                             │
│     (تعريفات .proto مشتركة: errors, metadata, ...)           │
└──────────────────────────────────────────────────────────────┘
```

### تدفق معالجة الطلبات

```
طلب العميل
  │
  ├─ HTTP 0.0.0.0:8000 ──→ axum::Router ──┐
  │                                        │
  └─ gRPC 0.0.0.0:9000 ──→ tonic::Server ─┤
                                      │
                              ┌───────┴───────┐
                              │   Middleware   │
                              │   ──────────   │
                              │ 1. Recovery    │   التقاط panic
                              │ 2. Tracing     │   حقن trace_id
                              │ 3. Logging     │   سجلات الطلبات
                              │ 4. Auth        │   المصادقة والتفويض
                              │ 5. Metrics     │   جمع المقاييس
│ 6. Security    │   كشف الهجمات
│ 7. CircuitBrk  │   حماية قاطع الدائرة
                              └───────┬───────┘
                                      │
                              ┌───────┴───────┐
                              │    Handler     │   منطق الأعمال
                              │ (tower::Service)│
                              └───────┬───────┘
                                      │
                              ┌───────┴───────┐
                              │   Response     │   الترميز والتسلسل
                              │ JSON/Protobuf  │
                              └───────────────┘
```

## الميزات

- **API-first**: تعريف API وأكواد الأخطاء والبيانات الوصفية في Protobuf؛ توليد الكود عبر prost + tonic-build
- **دعم بروتوكول مزدوج**: HTTP (axum) وgRPC (tonic) يتشاركان نفس مجموعة وسائط tower::Layer
- **بنية قابلة للتركيب**: Registry وConfig وLogging وEncoding جميعها مُجرّدة عبر traits، مع تنفيذات افتراضية جاهزة للإنتاج
- **نظام الوسائط**: Recovery وTracing وLogging وTimeout وRateLimit وSecurity وCircuitBreaker وMetricsLayer وRetryLayer وValidateLayer وCORS (ميزة cors) مدمجة؛ وتُركَّب عبر tower::ServiceBuilder
- **دورة حياة التطبيق**: نمط Builder لبناء App، تشغيل خوادم متعددة بالتوازي، معالجة إشارات SIGTERM/SIGINT، خطافات دورة الحياة start/stop
- **أمان الأنواع**: نظام أكواد أخطاء قائم على protobuf مع ربط أكواد حالة HTTP في وقت الترجمة
- **المراقبة**: tracing + Prometheus + نقاط Health (/health و/ready)
- **كشف الهجمات**: يكتشف SecurityLayer تلقائيًا أنماط الهجمات مثل SQL injection وXSS وSSRF، ويعيق الطلبات عالية الخطورة
- **التواصل بين الخدمات**: يدمج HttpClient اكتشاف الخدمات وموازنة الحمل، مع حماية عبر CircuitBreaker
- **المصادقة والتفويض**: وسيط مصادقة JWT / API Key، مع تمرير Claims إلى سياق الطلب
- **الرسائل والأحداث**: trait MessageQueue + EventBus للنشر/الاشتراك المحلي والبعيد
- **التتبع الموزع**: spans الطلبات، حقن/استخراج trace_id
- **عميل gRPC**: يدمج GrpcClient اكتشاف الخدمات وموازنة الحمل
- **متعدد البروتوكولات**: توجيه موحد لـ HTTP وgRPC وWebSocket وGraphQL
- **مصادر بيانات متعددة**: RDBMS (SQLite/PG/MySQL/TiDB)، التخزين المؤقت (Redis/Memcached)، البحث (OpenSearch/Elasticsearch)، الرسوم البيانية (Neo4j/NebulaGraph/ArangoDB)، السلاسل الزمنية (InfluxDB/IoTDB/QuestDB/TDengine)، المستندات (MongoDB)، التخزين الكائني (S3/MinIO)

### مقارنة مفاهيم Kratos

| Kratos (Go) | e-cat (Rust) | ملاحظات |
|-------------|-------------|------|
| `kratos.New()` | `App::builder()` | نمط Builder |
| `http.Handler` | `tower::Service` | trait قياسي في نظام Rust البيئي |
| `http.Server` | `axum::Router` | إطار HTTP السائد في المجتمع |
| `grpc.Server` | `tonic::transport::Server` | أنضج تنفيذ gRPC |
| `proto generate` | `prost + tonic-build` | protobuf القياسي في المجتمع |
| `registry.Discovery` | `Registry` trait | تسجيل واكتشاف قابل للتركيب |
| `config.Source` | `ConfigSource` trait | تحميل إعدادات متعدد المصادر |

## الرصيد التقني

| المكوّن | الاختيار |
|------|------|
| وقت تشغيل غير متزامن | **tokio** |
| HTTP | **axum** |
| gRPC | **tonic** |
| Protobuf | **prost + tonic-build** |
| الوسائط | **tower::Service / Layer** |
| التسجيل/التتبع | **tracing + trace_id propagation** |
| المقاييس | **prometheus** |
| التسلسل | **serde + prost** |
| كشف الهجمات | **security-rust** |
| RDBMS | **sqlx** |
| Redis | **redis-rs** |
| JWT | **jsonwebtoken** |
| عميل HTTP | **reqwest** |
| CLI | **clap** |

## قواعد البيانات المدعومة

| الفئة | قاعدة البيانات | Crate | الحالة |
|------|--------|-------|------|
| RDBMS | SQLite | `ecat-data-sqlx` | ✅ مُنفَّذ |
| RDBMS | PostgreSQL | `ecat-data-sqlx` | ✅ مُنفَّذ |
| RDBMS | MySQL | `ecat-data-sqlx` | ✅ مُنفَّذ |
| RDBMS | TiDB | `ecat-data-sqlx` | ✅ مُنفَّذ |
| التخزين المؤقت | Redis | `ecat-data-redis` | ✅ مُنفَّذ |
| البحث | OpenSearch | `ecat-data-opensearch` | ✅ مُنفَّذ |
| البحث | Elasticsearch | `ecat-data-elasticsearch` | ✅ مُنفَّذ |
| التخزين المؤقت | Memcached | `ecat-data-memcached` | ⚠️ تنفيذ في الذاكرة (غير مناسب للإنتاج، لا تستخدمه للتخزين المؤقت الدائم) |
| OLAP | ClickHouse | `ecat-data-clickhouse` | ✅ مُنفَّذ |
| الرسوم البيانية | Neo4j | `ecat-data-neo4j` | ✅ REST API |
| الرسوم البيانية | NebulaGraph | `ecat-data-nebulagraph` | ✅ REST API |
| الرسوم البيانية | ArangoDB | `ecat-data-arangodb` | ✅ REST API |
| السلاسل الزمنية | InfluxDB | `ecat-data-influxdb` | ✅ HTTP API |
| السلاسل الزمنية | Apache IoTDB | `ecat-data-iotdb` | ✅ REST API |
| السلاسل الزمنية | QuestDB | `ecat-data-questdb` | ✅ HTTP API |
| السلاسل الزمنية | TDengine | `ecat-data-tdengine` | ✅ REST API |
| المستندات | MongoDB | `ecat-data-mongodb` | ✅ مشغّل أصلي |
| التخزين الكائني | S3 / MinIO | `ecat-data-s3` | ✅ reqwest+rustls |

> جميع الخلفيات البياناتية مُجرّدة عبر traits موحّدة (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`)؛ استورد crate المساهمة المناسبة حسب الحاجة. يوفر كل خلفية بنية `XxxConfig` (`#[derive(Deserialize)]`) تدعم تحميل معلومات الاتصال من ملفات إعدادات JSON/YAML.

> **اصطلاح تسمية البنّاءين**: crates قوائم الرسائل (`ecat-mq-*`) تستخدم `connect` كبنّاء أساسي موحد (مثل `KafkaMq::connect(brokers)` و`MqttMq::connect(url)`)، وتوفر أيضًا `from_config` للتحميل من الإعدادات؛ معظم crates خلفيات البيانات (`ecat-data-*`) تستخدم `new` كبنّاء أساسي، مع استثناءات: `ecat-data-redis` / `ecat-data-sqlx` تحافظان على `connect`، و`ecat-data-mongodb` / `ecat-data-s3` توفران `from_config` فقط. هذا اصطلاح قائم ولا يُفرض توحيده (لتجنب التغييرات الكاسرة)؛ يمكن تقييم التوحيد في نافذة 3.0.

### مثال على إعداد قاعدة البيانات

يوفر كل خلفية بيانات بنية `XxxConfig` وطريقة `from_config()` لفصل معلومات الاتصال عن الكود إلى ملفات إعدادات:

```rust
use ecat_data_redis::{RedisCache, RedisConfig};
use ecat_data_sqlx::{SqlxClient, SqlxConfig};
use ecat_data_clickhouse::{ClickhouseClient, ClickhouseConfig};

// التحميل من ملف الإعدادات (JSON أو YAML)
let config: serde_json::Value = serde_json::from_str(r#"{
    "redis":     {"url": "redis://localhost:6379"},
    "sql":       {"url": "postgres://user:pass@localhost/db"},
    "clickhouse":{"base_url": "http://localhost:8123", "database": "mydb"}
}"#)?;

// Redis
let redis_cfg: RedisConfig = serde_json::from_value(config["redis"].clone())?;
let cache = RedisCache::from_config(redis_cfg).await?;
cache.set("key", b"value", Duration::from_secs(60)).await?;

// RDBMS
let sql_cfg: SqlxConfig = serde_json::from_value(config["sql"].clone())?;
let db = SqlxClient::from_config(sql_cfg).await?;
let rows = db.query("SELECT * FROM users").await?;

// ClickHouse
let ch_cfg: ClickhouseConfig = serde_json::from_value(config["clickhouse"].clone())?;
let ch = ClickhouseClient::from_config(ch_cfg);
ch.execute("INSERT INTO events VALUES (1, 'start')").await?;
```

**مرجع حقول الإعدادات**:

| الخلفية | Config | الحقول | أمثلة على القيم |
|------|--------|------|--------|
| Redis | `RedisConfig` | `url`, `password`? | `redis://localhost:6379` |
| RDBMS | `SqlxConfig` | `url`, `username`?, `password`? | `postgres://localhost/db` |
| ClickHouse | `ClickhouseConfig` | `base_url`, `database`, `username`?, `password`? | `http://localhost:8123`, `default` |
| QuestDB | `QuestdbConfig` | `base_url`, `username`?, `password`? | `http://localhost:9000` |
| Elasticsearch | `ElasticsearchConfig` | `base_url`, `username`?, `password`? | `http://localhost:9200` |
| OpenSearch | `OpenSearchConfig` | `base_url`, `username`?, `password`? | `http://localhost:9200` |
| InfluxDB | `InfluxConfig` | `base_url`, `org`, `bucket`, `token` | — |
| Neo4j | `Neo4jConfig` | `base_url`, `username`, `password` | — |
| NebulaGraph | `NebulaGraphConfig` | `base_url`, `space`, `username`?, `password`? | — |
| ArangoDB | `ArangoConfig` | `base_url`, `db`, `username`, `password` | — |
| IoTDB | `IotdbConfig` | `base_url`, `username`, `password` | — |
| Memcached | `MemcachedConfig` | `username`?, `password`? (حقول محجوزة) | — |
| TDengine | `TdengineConfig` | `base_url`, `username`, `password`, `database`? | `http://localhost:6041` |
| MongoDB | `MongoConfig` | `url`, `database`, `tls`? | `mongodb://localhost:27017`, `app` |
| S3 | `S3Config` | `endpoint`, `region`, `access_key`, `secret_key`, `tls`? | `http://localhost:9000`, `us-east-1` |

> تدعم جميع Configs الخلفية حقل `tls` اختياريًا (`TlsClientConfig`) لتكوين مصادقة شهادة عميل TLS. انظر [برنامج تعليمي لإعداد قاعدة البيانات](database-config-tutorial.md).

## بنية المشروع

```
e-cat/
├── ecat/                       # النواة: دورة حياة التطبيق
├── ecat-transport/             # تجريد النقل (trait Server)
├── ecat-transport-http/        # تنفيذ axum
├── ecat-transport-grpc/        # تنفيذ tonic
├── ecat-middleware/            # وسائط tower::Layer
├── ecat-protos/                # تعريفات Protobuf
├── ecat-errors/                # نظام أكواد الأخطاء
├── ecat-metadata/              # تمرير البيانات الوصفية
├── ecat-encoding/              # تجريد التسلسل
├── ecat-logging/               # تكامل tracing
├── ecat-registry/              # تسجيل واكتشاف الخدمات
├── ecat-config/                # إدارة الإعدادات
├── ecat-metrics/               # تكامل Prometheus
├── ecat-data/                  # traits الوصول إلى البيانات
├── ecat-security/              # كشف الهجمات (security-rust)
├── ecat-cli/                   # أداة CLI
├── ecat-health/                # فحوصات الصحة (/health /ready)
├── ecat-auth/                  # وسيط المصادقة (JWT / API Key)
├── ecat-client/                # عميل HTTP بين الخدمات
├── ecat-circuit-breaker/       # قاطع الدائرة (Tower Layer)
├── ecat-registry-consul/       # تسجيل الخدمات في Consul
├── ecat-config-remote/         # الإعدادات البعيدة Consul KV
├── ecat-data-redis/            # تنفيذ التخزين المؤقت Redis
├── ecat-mq/                    # تجريد قوائم الرسائل
├── ecat-events/                # ناقل الأحداث (محلي + بعيد)
├── ecat-testing/               # أدوات اختبار التكامل
├── ecat-openapi/               # توليد مواصفات OpenAPI
├── ecat-bench/                 # معايير الأداء
├── ecat-tracing/               # التتبع الموزع (حقن/استخراج trace_id)
├── ecat-registry-etcd/         # تسجيل الخدمات في etcd
├── ecat-mq-kafka/              # محول قائمة رسائل Kafka
├── ecat-data-opensearch/       # خلفية بحث OpenSearch
├── ecat-data-influxdb/         # خلفية السلاسل الزمنية InfluxDB
├── ecat-graphql/               # نقطة GraphQL
├── ecat-data-elasticsearch/    # خلفية بحث Elasticsearch
├── ecat-data-clickhouse/       # خلفية OLAP ClickHouse
├── ecat-data-sqlx/             # خلفية RDBMS (SQLite/PG/MySQL/TiDB)
├── ecat-data-memcached/        # خلفية التخزين المؤقت Memcached (تنفيذ في الذاكرة)
├── ecat-data-neo4j/            # خلفية الرسوم البيانية Neo4j
├── ecat-data-nebulagraph/      # خلفية الرسوم البيانية NebulaGraph
├── ecat-data-arangodb/         # خلفية الرسوم البيانية ArangoDB
├── ecat-data-iotdb/            # خلفية السلاسل الزمنية IoTDB
├── ecat-data-questdb/          # خلفية السلاسل الزمنية QuestDB
├── ecat-transport-ws/          # نقل WebSocket
├── ecat-versioning/            # توجيه إصدارات API
├── ecat-tls/                   # إعداد شهادات TLS وتوليدها تلقائيًا
├── ecat-deploy/                # Docker / K8s / Helm / CI/CD
├── ecat-lock/                  # تجريد الأقفال الموزعة (تنفيذ Redis)
├── ecat-scheduler/             # جدولة المهام التوقيتية tokio
├── ecat-tracing-otlp/          # تصدير تتبع OpenTelemetry OTLP
├── ecat-data-tdengine/         # خلفية السلاسل الزمنية TDengine
├── ecat-data-mongodb/          # خلفية المستندات MongoDB
├── ecat-data-s3/               # خلفية التخزين الكائني S3 / MinIO
├── ecat-mq-rabbitmq/           # خلفية رسائل RabbitMQ
├── ecat-mq-mqtt/               # خلفية رسائل MQTT
├── ecat-mq-nats/               # خلفية رسائل NATS
├── config/                     # ملفات أمثلة الإعدادات
├── docs/                       # مستندات التصميم وخطط النظام البيئي
└── examples/                   # مشاريع أمثلة
```

## البدء السريع

### المتطلبات المسبقة

- Rust 1.85+ (سلسلة أدوات stable، يتطلب edition 2024)
- [protoc](https://github.com/protocolbuffers/protobuf) (مجمّع Protocol Buffers)

### تثبيت CLI

```bash
cargo install ecat-cli
```

### إنشاء خدمة

```bash
# توليد مشروع من القالب
ecat new helloworld
cd helloworld

# إضافة تعريف proto
ecat proto add proto/service.proto

# توليد كود العميل والخادم (tonic-build build.rs، إكمال تبعيات Cargo.toml تلقائيًا)
ecat proto client proto/service.proto
ecat proto server proto/service.proto -t internal/service

# التشغيل في وضع التطوير
ecat run

# مراقبة تغييرات src/ وإعادة التشغيل تلقائيًا
ecat run --watch

# تحديث جميع تبعيات ecat-*
ecat upgrade
```

قم بزيارة `http://localhost:8000/helloworld/ecat`.

### مثال على الكود

```rust
use ecat::App;
use ecat_transport_http::HttpServer;
use ecat_transport_grpc::GrpcServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let http_srv = HttpServer::new("0.0.0.0:8000");
    let grpc_srv = GrpcServer::new("0.0.0.0:9000");

    let app = App::builder()
        .name("my-service")
        .version("v1.0.0")
        .server(http_srv)
        .server(grpc_srv)
        .on_start(|| async {
            tracing::info!("service started");
            Ok(())
        })
        .on_stop(|| async {
            tracing::info!("service stopped");
            Ok(())
        })
        .build()?;

    app.run().await?; // الحظر حتى SIGTERM/SIGINT
    Ok(())
}
```

### الـ Crate الجامع (ecat)

يوفر `ecat` نقطة دخول re-export مُفعّلة عبر الميزات — فعّل فقط المكونات التي تحتاجها:

```rust
use ecat::transport_http::HttpServer;   // feature "http" (افتراضي)
use ecat::middleware::RecoveryLayer;     // feature "middleware"
use ecat::auth::JwtAuthLayer;            // feature "auth"
use ecat::data::redis::RedisCache;       // feature "redis"
```

الميزات الافتراضية = `http+grpc`؛ استخدم `--no-default-features --features <component>` لتقليص شجرة التبعيات. قائمة الميزات الكاملة: `http` `grpc` `middleware` `auth` `client` `events` `metrics` `tracing` `circuit-breaker` `consul` `remote` `redis`.

### الوسائط

```rust
use tower::ServiceBuilder;
use ecat_middleware::{RecoveryLayer, TracingLayer, LoggingLayer, TimeoutLayer};
use ecat_circuit_breaker::CircuitBreakerLayer;
use ecat_security::SecurityLayer;
use ecat_auth::JwtAuthLayer;
use std::time::Duration;

// يجب أن يكون مفتاح JWT ≥32 بايت؛ يمكن فرض التحقق من ادعاءات iss/aud بشكل تسلسلي (اختياري، لا يُتحقق افتراضيًا):
// JwtAuthLayer::new(secret)?.required_issuer("my-issuer").required_audience("my-api")
let jwt = JwtAuthLayer::new("change-me-32-bytes-minimum-secret").expect("valid jwt secret");

let layer = ServiceBuilder::new()
    .layer(RecoveryLayer)
    .layer(TracingLayer)
    .layer(LoggingLayer)
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    .layer(CircuitBreakerLayer::new())
    .layer(jwt)
    .layer(SecurityLayer::new());
```

> ملاحظة: `ecat_middleware::TracingLayer` لا يحقن trace_id؛ لحقن trace_id على مستوى الطلب، استخدم `ecat_tracing::TracingLayer::new()`.

```rust
// المقاييس: تسجيل عدد الطلبات وزمن الاستجابة في registry عام (مشترك مع نقطة /metrics)
use ecat_metrics::MetricsLayer;
let app = Router::new().route("/hello", get(hello)).layer(MetricsLayer::new());
// أسماء المقاييس: ecat_http_requests_total / ecat_http_request_duration_seconds
// (الوسوم method/path/status). في السيناريوهات عالية الكاردينالية مثل المسارات التي تحتوي
// على معرّفات، استخدم MetricsLayer::new().with_path_fn(...) للتطبيع وتجنب انفجار كاردينالية المقاييس.

// إعادة المحاولة: تراجع أسي؛ ⚠️ آمنة فقط للطلبات متطابقة الأثر (GET/HEAD/PUT/DELETE)
use ecat_middleware::RetryLayer;
let retry = RetryLayer::new(3, Duration::from_secs(1), Duration::from_secs(30)); // إجمالي 3 محاولات شاملة الأولى
// قواعد إعادة محاولة مخصصة: RetryLayer::new(3, ...).with_rule(MyRule)  // التحقق حسب رمز الحالة/محتوى الاستجابة

// التحقق: التحقق من header/المعلمات قبل التوجيه، وإرجاع خطأ JSON عند الفشل (افتراضي 400، مع with_status يمكن تغييره إلى 422 إلخ)
use ecat_middleware::{ValidateLayer, ValidateError};
let validate = ValidateLayer::from_fn(|req: &http::Request<axum::body::Body>| {
    if req.headers().contains_key("x-api-key") {
        Ok(())
    } else {
        Err(ValidateError::new("missing x-api-key").with_status(422))
    }
});

// CORS: يتطلب تفعيل ميزة "cors" في ecat-middleware
use ecat_middleware::{CorsLayer, AllowOrigin};
let cors = CorsLayer::new().allow_origin(AllowOrigin::any());
```

### معالجة الأخطاء

```rust
use ecat_errors::{Error, ErrorCode};

fn get_user(id: u64) -> Result<User, Error> {
    if id == 0 {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "bad_request",
            "user id must be positive",
        ));
    }
    // ...
}
```

## مراحل التنفيذ

| المرحلة | الحالة | المحتوى |
|------|------|------|
| Phase 1 | ✅ مكتملة | هيكل المشروع، protos، errors، metadata، encoding، logging |
| Phase 2 | ✅ مكتملة | طبقة النقل (HTTP + gRPC) |
| Phase 3 | ✅ مكتملة | نظام الوسائط (Recovery/Tracing/Logging/Timeout) |
| Phase 4 | ✅ مكتملة | إدارة دورة حياة التطبيق |
| Phase 5 | ✅ مكتملة | Registry وConfig وMetrics |
| Phase 5.5 | ✅ مكتملة | طبقة الوصول إلى البيانات (traits + خلفية sqlx) |
| Phase 6 | ✅ مكتملة | سلسلة أدوات CLI (new/proto/run/build) |
| Phase 7 | ✅ مكتملة | README وأمثلة (helloworld) ومستندات التصميم |
| Phase 8 | ✅ مكتملة | تكامل كشف الهجمات (security-rust، ecat-security) |
| Phase 9 | ✅ مكتملة | المرحلة الأولى من النظام البيئي (health / client / circuit-breaker / auth / registry-consul) |
| Phase 10 | ✅ مكتملة | المرحلة الثانية من النظام البيئي (redis / mq / events / config-remote) |
| Phase 11 | ✅ مكتملة | المرحلة الثالثة من النظام البيئي (testing / deploy / bench / openapi) |
| Phase 12 | ✅ مكتملة | تقوية الاتصالات والأمان (عميل gRPC / OAuth2 / mTLS / التتبع الموزع) |
| Phase 13 | ✅ مكتملة | استكمال خلفيات البيانات (etcd / Kafka / OpenSearch / InfluxDB) |
| Phase 14 | ✅ مكتملة | التشغيل والتجربة (WebSocket / إدارة إصدارات API / Helm / CI/CD) |
| Phase 15 | ✅ مكتملة | توسيع النظام البيئي v2 (Kafka حقيقي / RabbitMQ / MQTT / NATS / MongoDB / S3 / TDengine / OTLP / الأقفال الموزعة / الجدولة / CLI watch+upgrade) |
| Phase 16 | ✅ مكتملة | تقوية الصيانة v2.4 (M1 MetricsLayer / M2 RetryLayer / M3 ValidateLayer / M4 CORS / U1 crate الجامع ecat / U2 examples / OAuth2 token hash / تتبع CVE) |

## القيود المعروفة

- **تحليل GraphQL (ecat-graphql)**: يدعم معاملات الحقول وselection المتداخلة (يمكن لـ resolvers الغنية `query_field`/`mutation_field` الوصول إلى `args`/`variables`/`selection`)؛ لا يدعم بعد aliases ولا fragments ولا حقولًا متعددة على المستوى الأعلى — لا تعرّضه كنقطة نهاية GraphQL عامة.
- **ذاكرة التخزين المؤقت لفحص OAuth2 (ecat-auth)**: مفتاح التخزين المؤقت هو SHA-256 hash للرمز (لا تُخزَّن قيمة الرمز نفسه)؛ القيم المخزنة تُصفّى عبر قائمة بيضاء (افتراضيًا تحتفظ بـ sub/exp/iat/role بالإضافة إلى iss/aud/scope/roles من extra، ويمكن تكوينها عبر `cache_claims_whitelist`؛ عند عدم الإصابة (miss) تُعاد الادعاءات الكاملة، فقط القيم المخزنة مؤقتًا تُصفّى)؛ الإدخالات منتهية TTL تُنقّى بنشاط عند الكتابة (TTL افتراضي 300s).
- **إزاحة Kafka (ecat-mq-kafka)**: افتراضيًا `enable.auto.commit=false` دون commit يدوي — بعد إعادة تشغيل العملية تُقرأ الرسائل من نهاية القسم (latest)، فتتخطى الرسائل المنتجة أثناء التوقف؛ لضمان دلالات at-least-once (الاستئناف من آخر نقطة مُلتزمة بعد إعادة التشغيل) يجب تكوين `auto_commit=true` صراحةً.

## أهداف التصميم

| # | الهدف | ملاحظات |
|---|------|------|
| 1 | **محاذاة Kratos** | الحفاظ على فلسفة Kratos في API-first والتركيب والتجريد الموحد |
| 2 | **أسلوب Rust الأصيل** | إعادة استخدام tower::Service وtraits العامة والتجريدات ذات التكلفة الصفرية؛ لا "Go in Rust" |
| 3 | **أمان الأنواع** | التقاط الأخطاء في وقت الترجمة؛ تعريفات Protobuf شديدة التحديد بالأنواع |
| 4 | **قابلية التركيب** | Registry وConfig وLogging وEncoding جميعها مُجرّدة عبر traits |
| 5 | **سلسلة أدوات متكاملة** | يدعم CLI توليد المشاريع من القوالب وتوليد كود proto والتشغيل التطويري |
| 6 | **الأداء أولًا** | تجريدات التكلفة الصفرية + وقت تشغيل غير متزامن |
| 7 | **المراقبة** | tracing + Prometheus جاهزان خارج الصندوق |
| 8 | **نظام بيئي متكامل** | عملاء وقاطع دائرة ومصادقة وفحوصات صحة وخلفيات سجل خدمات |

## ملاحظات تقنية

### لماذا tower::Service

[`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) هو المعادل لـ `http.Handler` في النظام البيئي غير المتزامن لـ Rust. كل من axum وtonic مبنيان على tower، لذا لا يحتاج e-cat إلى trait وسيط مخصص — توفير تنفيذات tower::Layer مباشرة يحقق نفس تأثير وسائط Kratos دون أي عبء محولات.

### لماذا Cargo Workspace

اتساقًا مع التصميم المعياري لـ Kratos. تُنشر جميع crates `ecat-*` بإصدارات متزامنة داخل workspace (حاليًا 3.0.2)، وتُترجم كل منها بشكل مستقل، ويستوردها المستخدمون حسب الحاجة. تحافظ crates النواة على الحد الأدنى من التبعيات، بينما توفر crates المساهمة تكاملات اختيارية.

### لماذا prost (بدلًا من protobuf-rs)

prost هو تنفيذ protobuf الأكثر استخدامًا في مجتمع Rust، ويولّد كودًا آمن الأنواع في وقت الترجمة ويتكامل بعمق مع tonic.

## مستندات التصميم

- [مواصفات التصميم](../../../docs/superpowers/specs/2026-07-29-ecat-framework-design.md)
- [خطة التنفيذ](../../../docs/superpowers/plans/2026-07-29-ecat-framework.md)
- [الخطة البيئية v1](ecosystem-plan.md) (مكتملة)
- [الخطة البيئية v2](ecosystem-plan-v2.md) (مكتملة)
- [الخطة البيئية v3](ecosystem-plan-v3.md) (التقييم النهائي)
- [مرجع API](api.md)
- [تقرير التدقيق r5](audit-report-2026-08-01-r5.md) (2026-08-01)
- [برنامج تعليمي لإعداد قاعدة البيانات](database-config-tutorial.md)
- [تتبع ثغرات CVE في التبعيات](dependency-cve-tracking.md)
- [برنامج تعليمي لمصادقة شهادات TLS](tls-certificate-tutorial.md)
- [ملفات أمثلة الإعدادات](../../../config/databases.example.yaml)

## الدعم

نرحب بدعمك للمشروع!

| WeChat Pay | Alipay |
|:---:|:---:|
| <img src="weixinpay.png" width="130" height="130" alt="WeChat Pay"> | <img src="alipay.png" width="130" height="130" alt="Alipay"> |

### التحويل العالمي (حوالة مصرفية)

| العنصر | التفاصيل |
|------|------|
| اسم المستلم | WANG KEXUN |
| رقم حساب المستلم | 881015918251 |
| البنك المستلم | ZA Bank Limited |
| رمز SWIFT | AABLHKHHXXX |
| رمز البنك | 387 |
| عنوان البنك | Core F, Cyberport 3, 100 Cyberport Road, Hong Kong |

> **البنك المراسل للتحويلات عبر الحدود (إذا لزم الأمر)**: هذه معلومات البنك المراسل (الوسيط)، وليست معلومات البنك المستلم. يُرجى الاستفسار من بنكك المُرسِل عما إذا كان تقديمها مطلوبًا.
>
> - للتحويلات بالدولار الهونغ كونغي واليوان الصيني والدولار الأمريكي: **Citibank N.A. Hong Kong** (SWIFT: `CITIHKHXXXX`، رمز البنك: 006، الفرع: Hong Kong Branch، رمز الفرع: 391، العنوان: Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong)
> - للعملات الأخرى: **THE BANK OF NEW YORK MELLON** (SWIFT: `IRVTUS3NXXX`، العنوان: 240 GREENWICH STREET, NEW YORK, United States)

## الترخيص

Apache-2.0
