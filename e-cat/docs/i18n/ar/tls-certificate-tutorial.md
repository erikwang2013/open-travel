# برنامج تعليمي لإعداد ومصادقة شهادات TLS

**الإصدار:** 2.4.2 · **التاريخ:** 2026-08-01

تدعم خلفيات البيانات الأربع عشرة في e-cat جميعًا مصادقة شهادة عميل TLS (mTLS). يغطي هذا البرنامج التعليمي التدفق الكامل لتوليد الشهادات وتكوينها والاتصال بها عبر جميع خلفيات قواعد البيانات.

---

## ١. توليد الشهادات

### الطريقة 1: التوليد التلقائي عبر ecat-tls (موصى بها)

```rust
use ecat_tls::{generate_ca, generate_server_cert, generate_client_cert};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("certs")?;

    // 1. توليد شهادة CA
    let ca = generate_ca("MyOrganization")?;
    fs::write("certs/ca.pem", &ca.cert_pem)?;
    fs::write("certs/ca-key.pem", &ca.key_pem)?;

    // 2. توليد شهادة الخادم (تُنشر على خادم قاعدة البيانات)
    let server = generate_server_cert("db.internal")?;
    fs::write("certs/server.pem", &server.cert_pem)?;
    fs::write("certs/server-key.pem", &server.key_pem)?;

    // 3. توليد شهادة العميل (يستخدمها التطبيق، mTLS)
    let client = generate_client_cert("myapp")?;
    fs::write("certs/client.pem", &client.cert_pem)?;
    fs::write("certs/client-key.pem", &client.key_pem)?;

    Ok(())
}
```

### الطريقة 2: التوليد اليدوي عبر OpenSSL

```bash
mkdir -p certs && cd certs

# توليد CA
openssl req -x509 -newkey rsa:4096 \
  -keyout ca-key.pem -out ca.pem -days 3650 -nodes \
  -subj "/O=MyOrg/CN=MyOrg CA"

# توليد شهادة الخادم
openssl req -new -newkey rsa:4096 \
  -keyout server-key.pem -out server.csr -nodes \
  -subj "/CN=db.internal"
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca-key.pem \
  -out server.pem -days 365

# توليد شهادة العميل (mTLS)
openssl req -new -newkey rsa:4096 \
  -keyout client-key.pem -out client.csr -nodes \
  -subj "/CN=myapp"
openssl x509 -req -in client.csr -CA ca.pem -CAkey ca-key.pem \
  -out client.pem -days 365

rm -f *.csr
```

---

## ٢. إعداد TLS

### حقول TLS العامة

تدعم جميع Configs الخلفيات الحقول الاختيارية التالية (`#[serde(default)]`):

| الحقل | النوع | الوصف |
|------|------|------|
| `tls.ca_cert` | `Option<String>` | مسار PEM لشهادة CA (التحقق من شهادة الخادم) |
| `tls.client_cert` | `Option<String>` | مسار PEM لشهادة العميل (mTLS) |
| `tls.client_key` | `Option<String>` | مسار PEM للمفتاح الخاص للعميل (mTLS) |
| `tls.skip_verify` | `Option<bool>` | تخطي التحقق من الشهادة (بيئات الاختبار فقط) |

> ⚠️ استبعاد متبادل: تكوين `skip_verify=true` مع `ca_cert` معًا يفشل مباشرة عند البناء (`ecat-tls` يرفض الإعدادات المتناقضة — تخطي التحقق مع تكوين نقطة ثقة، لمنع إيقاف التحقق من الشهادة صامتًا بسبب إعداد خاطئ).

### مثال على إعداد YAML

```yaml
# التحقق من شهادة الخادم فقط
elasticsearch:
  base_url: "https://es.internal:9200"
  tls:
    ca_cert: "/etc/ecat/ca.pem"

# mTLS (مصادقة ثنائية الاتجاه)
clickhouse:
  base_url: "https://ch.internal:8443"
  database: "analytics"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"

# بيئة الاختبار (تخطي التحقق)
questdb:
  base_url: "https://localhost:9000"
  tls:
    skip_verify: true
```

---

## ٣. إعداد TLS لكل خلفية

### خلفيات HTTP (9)

Elasticsearch, OpenSearch, ClickHouse, QuestDB, InfluxDB, Neo4j, NebulaGraph, ArangoDB, IoTDB — تبني جميعًا عميل TLS عبر `TlsClientConfig::build_reqwest_client()`.

```yaml
# جميع خلفيات HTTP تستخدم نفس التنسيق
backend:
  base_url: "https://host:port"
  tls:
    ca_cert: "/path/to/ca.pem"
    client_cert: "/path/to/client.pem"   # يتطلبها mTLS
    client_key: "/path/to/client-key.pem" # يتطلبها mTLS
```

### Redis — تبديل تلقائي لمخطط URL

```yaml
redis:
  url: "redis://cache.internal:6379"    # تفعيل TLS → تبديل تلقائي إلى rediss://
  tls:
    ca_cert: "/etc/ecat/ca.pem"
```

### RDBMS (Sqlx) — إعداد عبر معاملات URL

```yaml
sql:
  url: "postgres://db.internal:5432/mydb?sslmode=require"
  tls: {}  # حقل محجوز
```

| قاعدة البيانات | معاملات TLS في URL |
|--------|------------|
| PostgreSQL | `?sslmode=require` أو `?sslmode=verify-full` |
| MySQL | `?ssl-mode=VERIFY_CA&ssl-ca=/path/to/ca.pem` |
| TiDB | `?ssl-mode=VERIFY_IDENTITY&ssl-ca=/path/to/ca.pem` |
| SQLite | لا يتطلب TLS |

---

## ٤. التحميل في كود Rust

```rust
use serde::Deserialize;
use ecat_data_elasticsearch::{ElasticsearchClient, ElasticsearchConfig};
use ecat_data_clickhouse::{ClickhouseClient, ClickhouseConfig};

#[derive(Deserialize)]
struct AppConfig {
    elasticsearch: ElasticsearchConfig,
    clickhouse: ClickhouseConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = std::fs::read_to_string("databases.yaml")?;
    let cfg: AppConfig = serde_yaml::from_str(&yaml)?;

    // from_config يستدعي داخليًا tls.build_reqwest_client() — يُطبَّق TLS تلقائيًا
    let es = ElasticsearchClient::from_config(cfg.elasticsearch);
    let ch = ClickhouseClient::from_config(cfg.clickhouse);

    let results = es.search("logs", &serde_json::json!({"match_all": {}})).await?;
    Ok(())
}
```

---

## ٥. الإنشاء برمجيًا (TLS + المصادقة)

```rust
use ecat_tls::TlsClientConfig;

// بناء عميل TLS يدويًا
let tls = TlsClientConfig {
    ca_cert: Some("/etc/ecat/ca.pem".into()),
    client_cert: Some("/etc/ecat/client.pem".into()),
    client_key: Some("/etc/ecat/client-key.pem".into()),
    skip_verify: None,
};
let client = tls.build_reqwest_client()?;

// أو استخدام with_auth + إعداد TLS
let es = ElasticsearchClient::with_auth(
    "https://es.internal:9200", "elastic", "secret"
);
```

---

## ٦. توصيات أمنية

1. **يجب التحقق من الشهادات في بيئة الإنتاج** — عطّل `skip_verify`
2. **تخزين آمن للمفتاح الخاص لـ CA** — لا يُضمَّن في التحكم بالإصدارات
3. **إدارة مدة صلاحية الشهادات** — جدّد وبدّل قبل انتهاء الصلاحية
4. **mTLS يعزز الأمان** — يُوصى بتكوين شهادة العميل في الإنتاج

---

## مستندات ذات صلة

- [برنامج تعليمي لإعداد قاعدة البيانات](database-config-tutorial.md)
- [تقرير التدقيق r5](audit-report-2026-08-01-r5.md)
- [ملف مثال الإعدادات](../../../config/databases.example.yaml)
