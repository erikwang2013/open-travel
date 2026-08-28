# TLS সার্টিফিকেট কনফিগ ও অথেনটিকেশন টিউটোরিয়াল

**ভার্সন:** 2.4.2 · **তারিখ:** 2026-08-01

e-cat-এর 14টি ডেটা ব্যাকএন্ডই TLS ক্লায়েন্ট সার্টিফিকেট অথেনটিকেশন (mTLS) সমর্থন করে। এই টিউটোরিয়ালে সার্টিফিকেট জেনারেশন, কনফিগ এবং সব ডেটাবেস ব্যাকএন্ডে সংযোগের সম্পূর্ণ প্রক্রিয়া কভার করা হয়েছে।

---

## এক、সার্টিফিকেট জেনারেশন

### উপায় 1：ecat-tls অটো-জেনারেশন (প্রস্তাবিত)

```rust
use ecat_tls::{generate_ca, generate_server_cert, generate_client_cert};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("certs")?;

    // 1. CA সার্টিফিকেট তৈরি
    let ca = generate_ca("MyOrganization")?;
    fs::write("certs/ca.pem", &ca.cert_pem)?;
    fs::write("certs/ca-key.pem", &ca.key_pem)?;

    // 2. সার্ভার সার্টিফিকেট তৈরি (ডেটাবেস সার্ভারে ডিপ্লয়)
    let server = generate_server_cert("db.internal")?;
    fs::write("certs/server.pem", &server.cert_pem)?;
    fs::write("certs/server-key.pem", &server.key_pem)?;

    // 3. ক্লায়েন্ট সার্টিফিকেট তৈরি (অ্যাপ্লিকেশন সাইডে ব্যবহার, mTLS)
    let client = generate_client_cert("myapp")?;
    fs::write("certs/client.pem", &client.cert_pem)?;
    fs::write("certs/client-key.pem", &client.key_pem)?;

    Ok(())
}
```

### উপায় 2：OpenSSL ম্যানুয়াল জেনারেশন

```bash
mkdir -p certs && cd certs

# CA তৈরি
openssl req -x509 -newkey rsa:4096 \
  -keyout ca-key.pem -out ca.pem -days 3650 -nodes \
  -subj "/O=MyOrg/CN=MyOrg CA"

# সার্ভার সার্টিফিকেট তৈরি
openssl req -new -newkey rsa:4096 \
  -keyout server-key.pem -out server.csr -nodes \
  -subj "/CN=db.internal"
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca-key.pem \
  -out server.pem -days 365

# ক্লায়েন্ট সার্টিফিকেট তৈরি (mTLS)
openssl req -new -newkey rsa:4096 \
  -keyout client-key.pem -out client.csr -nodes \
  -subj "/CN=myapp"
openssl x509 -req -in client.csr -CA ca.pem -CAkey ca-key.pem \
  -out client.pem -days 365

rm -f *.csr
```

---

## দুই、TLS কনফিগ

### সাধারণ TLS ফিল্ড

সব ব্যাকএন্ড Config নিচের ঐচ্ছিক ফিল্ডগুলো সমর্থন করে (`#[serde(default)]`)：

| ফিল্ড | টাইপ | ব্যাখ্যা |
|------|------|------|
| `tls.ca_cert` | `Option<String>` | CA সার্টিফিকেট PEM পাথ (সার্ভার সার্টিফিকেট যাচাই) |
| `tls.client_cert` | `Option<String>` | ক্লায়েন্ট সার্টিফিকেট PEM পাথ (mTLS) |
| `tls.client_key` | `Option<String>` | ক্লায়েন্ট প্রাইভেট কী PEM পাথ (mTLS) |
| `tls.skip_verify` | `Option<bool>` | সার্টিফিকেট যাচাই স্কিপ (শুধুমাত্র টেস্ট পরিবেশ) |

> ⚠️ পারস্পরিক বর্জন: `skip_verify=true` ও `ca_cert` একসাথে কনফিগ করলে বিল্ড টাইমে সরাসরি এরর হয় (`ecat-tls` পরস্পরবিরোধী কনফিগ প্রত্যাখ্যান করে——স্কিপ-ভেরিফিকেশন অথচ ট্রাস্ট অ্যাঙ্কর কনফিগ, ভুল কনফিগে সার্টিফিকেট যাচাই নীরবে বন্ধ হওয়া প্রতিরোধ করে)।

### YAML কনফিগ উদাহরণ

```yaml
# শুধুমাত্র সার্ভার সার্টিফিকেট যাচাই
elasticsearch:
  base_url: "https://es.internal:9200"
  tls:
    ca_cert: "/etc/ecat/ca.pem"

# mTLS（দ্বিমুখী অথেনটিকেশন）
clickhouse:
  base_url: "https://ch.internal:8443"
  database: "analytics"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"

# টেস্ট পরিবেশ (যাচাই স্কিপ)
questdb:
  base_url: "https://localhost:9000"
  tls:
    skip_verify: true
```

---

## তিন、প্রতিটি ব্যাকএন্ডের TLS কনফিগ

### HTTP ব্যাকএন্ড (9টি)

Elasticsearch, OpenSearch, ClickHouse, QuestDB, InfluxDB, Neo4j, NebulaGraph, ArangoDB, IoTDB — সবগুলো `TlsClientConfig::build_reqwest_client()` দিয়ে TLS Client তৈরি করে।

```yaml
# সব HTTP ব্যাকএন্ড একই ফরম্যাট ব্যবহার করে
backend:
  base_url: "https://host:port"
  tls:
    ca_cert: "/path/to/ca.pem"
    client_cert: "/path/to/client.pem"   # mTLS প্রয়োজন
    client_key: "/path/to/client-key.pem" # mTLS প্রয়োজন
```

### Redis — অটো URL scheme সুইচ

```yaml
redis:
  url: "redis://cache.internal:6379"    # TLS সক্ষম → অটো rediss:// সুইচ
  tls:
    ca_cert: "/etc/ecat/ca.pem"
```

### RDBMS (Sqlx) — URL প্যারামিটার কনফিগ

```yaml
sql:
  url: "postgres://db.internal:5432/mydb?sslmode=require"
  tls: {}  # রিজার্ভড ফিল্ড
```

| ডেটাবেস | TLS URL প্যারামিটার |
|--------|------------|
| PostgreSQL | `?sslmode=require` বা `?sslmode=verify-full` |
| MySQL | `?ssl-mode=VERIFY_CA&ssl-ca=/path/to/ca.pem` |
| TiDB | `?ssl-mode=VERIFY_IDENTITY&ssl-ca=/path/to/ca.pem` |
| SQLite | TLS প্রয়োজন নেই |

---

## চার、Rust কোডে লোড

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

    // from_config ভেতরে tls.build_reqwest_client() কল করে — TLS অটো অ্যাপ্লাই হয়
    let es = ElasticsearchClient::from_config(cfg.elasticsearch);
    let ch = ClickhouseClient::from_config(cfg.clickhouse);

    let results = es.search("logs", &serde_json::json!({"match_all": {}})).await?;
    Ok(())
}
```

---

## পাঁচ、প্রোগ্রাম্যাটিক তৈরি (TLS + অথেনটিকেশন)

```rust
use ecat_tls::TlsClientConfig;

// ম্যানুয়ালি TLS ক্লায়েন্ট তৈরি
let tls = TlsClientConfig {
    ca_cert: Some("/etc/ecat/ca.pem".into()),
    client_cert: Some("/etc/ecat/client.pem".into()),
    client_key: Some("/etc/ecat/client-key.pem".into()),
    skip_verify: None,
};
let client = tls.build_reqwest_client()?;

// অথবা with_auth + TLS কনফিগ ব্যবহার
let es = ElasticsearchClient::with_auth(
    "https://es.internal:9200", "elastic", "secret"
);
```

---

## ছয়、সিকিউরিটি পরামর্শ

1. **প্রোডাকশনে সার্টিফিকেট যাচাই বাধ্যতামূলক** — `skip_verify` নিষ্ক্রিয় রাখুন
2. **CA প্রাইভেট কী নিরাপদে সংরক্ষণ** — ভার্সন কন্ট্রোলে অন্তর্ভুক্ত করবেন না
3. **সার্টিফিকেট মেয়াদ ম্যানেজমেন্ট** — মেয়াদ শেষ হওয়ার আগে রিনিউ ও রোটেট করুন
4. **mTLS সিকিউরিটি বাড়ায়** — প্রোডাকশনে ক্লায়েন্ট সার্টিফিকেট কনফিগ করার পরামর্শ

---

## সম্পর্কিত ডকুমেন্ট

- [ডেটাবেস কনফিগ টিউটোরিয়াল](database-config-tutorial.md)
- [অডিট রিপোর্ট r5](audit-report-2026-08-01-r5.md)
- [কনফিগ উদাহরণ ফাইল](../../../config/databases.example.yaml)
