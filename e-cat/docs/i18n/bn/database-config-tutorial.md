# ডেটাবেস কনফিগ টিউটোরিয়াল

**ভার্সন:** 2.4.2 · **তারিখ:** 2026-08-01

e-cat-এর 14টি ডেটা ব্যাকএন্ডই কনফিগ ফাইল থেকে সংযোগ তথ্য লোড করতে সমর্থন করে, কোডে হার্ডকোড করার প্রয়োজন নেই। `username` / `password` দুটোই ঐচ্ছিক ফিল্ড, বাদ দিলে অথেনটিকেশন স্কিপ হয়।

---

## দ্রুত শুরু

### 1. কনফিগ ফাইল তৈরি করুন

উদাহরণ টেমপ্লেট কপি করে বাস্তব পরিবেশ অনুযায়ী পরিবর্তন করুন:

```bash
cp config/databases.example.yaml databases.yaml
```

`databases.yaml` এডিট করে আসল সংযোগ তথ্য দিন:

```yaml
# databases.yaml
sql:
  url: "postgres://myapp:secret@db.internal:5432/myapp"

redis:
  url: "redis://cache.internal:6379"

clickhouse:
  base_url: "http://ch.internal:8123"
  database: "analytics"
```

### 2. ডিপেন্ডেন্সি যোগ করুন

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
ecat-data-sqlx = { path = "../ecat-data-sqlx" }
ecat-data-redis = { path = "../ecat-data-redis" }
ecat-data-clickhouse = { path = "../ecat-data-clickhouse" }
```

### 3. লোড ও ব্যবহার

```rust
use ecat_data_redis::{RedisCache, RedisConfig};
use ecat_data_sqlx::{SqlxClient, SqlxConfig};
use ecat_data_clickhouse::{ClickhouseClient, ClickhouseConfig};
use serde::Deserialize;

#[derive(Deserialize)]
struct AppConfig {
    sql: SqlxConfig,
    redis: RedisConfig,
    clickhouse: ClickhouseConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // YAML কনফিগ লোড
    let yaml = std::fs::read_to_string("databases.yaml")?;
    let cfg: AppConfig = serde_yaml::from_str(&yaml)?;

    // ডেটাবেস ক্লায়েন্ট তৈরি — কোনো হার্ডকোডেড সংযোগ তথ্য নেই
    let db = SqlxClient::from_config(cfg.sql).await?;
    let cache = RedisCache::from_config(cfg.redis).await?;
    let ch = ClickhouseClient::from_config(cfg.clickhouse);

    // ব্যবহার
    let rows = db.query("SELECT id, name FROM users LIMIT 10").await?;
    cache.set("health", b"ok", std::time::Duration::from_secs(30)).await?;

    Ok(())
}
```

---

## সম্পূর্ণ কনফিগ রেফারেন্স

### টপ-লেভেল কনফিগ স্ট্রাক্ট সংজ্ঞায়িত করা

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct DatabasesConfig {
    pub sql: ecat_data_sqlx::SqlxConfig,
    pub redis: ecat_data_redis::RedisConfig,
    pub memcached: ecat_data_memcached::MemcachedConfig,
    pub clickhouse: ecat_data_clickhouse::ClickhouseConfig,
    pub questdb: ecat_data_questdb::QuestdbConfig,
    pub elasticsearch: ecat_data_elasticsearch::ElasticsearchConfig,
    pub opensearch: ecat_data_opensearch::OpenSearchConfig,
    pub neo4j: ecat_data_neo4j::Neo4jConfig,
    pub nebulagraph: ecat_data_nebulagraph::NebulaGraphConfig,
    pub arangodb: ecat_data_arangodb::ArangoConfig,
    pub influxdb: ecat_data_influxdb::InfluxConfig,
    pub iotdb: ecat_data_iotdb::IotdbConfig,
}
```

### YAML সম্পূর্ণ উদাহরণ

`config/databases.example.yaml` দেখুন।

---

## প্রতিটি ব্যাকএন্ডের Config ফিল্ড দ্রুত রেফারেন্স

### RDBMS — SqlxConfig

```yaml
sql:
  url: "postgres://host:5432/dbname"
  # username: "app_user"    # ঐচ্ছিক
  # password: "secret"      # ঐচ্ছিক
```

| ফিল্ড | টাইপ | ব্যাখ্যা |
|------|------|------|
| `url` | `String` | sqlx সংযোগ স্ট্রিং, SQLite/PG/MySQL/TiDB সমর্থন করে |
| `username` | `Option<String>` | ঐচ্ছিক: URL-এ এমবেডেড অথেনটিকেশন (password-এর সাথে) |
| `password` | `Option<String>` | ঐচ্ছিক: URL-এ এমবেডেড অথেনটিকেশন (username-এর সাথে) |

### Redis — RedisConfig

```yaml
redis:
  url: "redis://host:6379"
  # password: "auth_token"  # ঐচ্ছিক
```

| ফিল্ড | টাইপ | ব্যাখ্যা |
|------|------|------|
| `url` | `String` | Redis সংযোগ URL |
| `password` | `Option<String>` | ঐচ্ছিক: Redis AUTH পাসওয়ার্ড |

### Memcached — MemcachedConfig

```yaml
memcached:
  # username: "memcache"    # ঐচ্ছিক: রিজার্ভড ফিল্ড (বর্তমানে মেমরি-ভিত্তিক)
  # password: "secret"      # ঐচ্ছিক: রিজার্ভড ফিল্ড
  {}
```

| ফিল্ড | টাইপ | ব্যাখ্যা |
|------|------|------|
| `username` | `Option<String>` | ঐচ্ছিক: রিজার্ভড ফিল্ড |
| `password` | `Option<String>` | ঐচ্ছিক: রিজার্ভড ফিল্ড |

বর্তমানে মেমরি-ভিত্তিক ইমপ্লিমেন্টেশন, অথেনটিকেশন ফিল্ড রিজার্ভড।

### ClickHouse — ClickhouseConfig

```yaml
clickhouse:
  base_url: "http://host:8123"
  database: "default"
  # username: "default"   # ঐচ্ছিক
  # password: "secret"    # ঐচ্ছিক
```

| ফিল্ড | টাইপ | ডিফল্ট | ব্যাখ্যা |
|------|------|--------|------|
| `base_url` | `String` | — | HTTP ইন্টারফেস ঠিকানা |
| `database` | `String` | `"default"` | ডেটাবেস নাম |
| `username` | `Option<String>` | `None` | ঐচ্ছিক: HTTP Basic Auth ইউজারনেম |
| `password` | `Option<String>` | `None` | ঐচ্ছিক: HTTP Basic Auth পাসওয়ার্ড |

### QuestDB — QuestdbConfig

```yaml
questdb:
  base_url: "http://host:9000"
  # username: "admin"     # ঐচ্ছিক
  # password: "quest"     # ঐচ্ছিক
```

| ফিল্ড | টাইপ | ব্যাখ্যা |
|------|------|------|
| `base_url` | `String` | HTTP API ঠিকানা |
| `username` | `Option<String>` | ঐচ্ছিক: HTTP Basic Auth ইউজারনেম |
| `password` | `Option<String>` | ঐচ্ছিক: HTTP Basic Auth পাসওয়ার্ড |

### Elasticsearch — ElasticsearchConfig

```yaml
elasticsearch:
  base_url: "http://host:9200"
  # username: "elastic"   # ঐচ্ছিক
  # password: "secret"    # ঐচ্ছিক
```

| ফিল্ড | টাইপ | ব্যাখ্যা |
|------|------|------|
| `base_url` | `String` | REST API ঠিকানা |
| `username` | `Option<String>` | ঐচ্ছিক: HTTP Basic Auth ইউজারনেম |
| `password` | `Option<String>` | ঐচ্ছিক: HTTP Basic Auth পাসওয়ার্ড |

### OpenSearch — OpenSearchConfig

```yaml
opensearch:
  base_url: "http://host:9200"
  # username: "admin"     # ঐচ্ছিক
  # password: "secret"    # ঐচ্ছিক
```

| ফিল্ড | টাইপ | ব্যাখ্যা |
|------|------|------|
| `base_url` | `String` | REST API ঠিকানা |
| `username` | `Option<String>` | ঐচ্ছিক: HTTP Basic Auth ইউজারনেম |
| `password` | `Option<String>` | ঐচ্ছিক: HTTP Basic Auth পাসওয়ার্ড |

### InfluxDB — InfluxConfig

```yaml
influxdb:
  base_url: "http://host:8086"
  org: "myorg"
  bucket: "mybucket"
  token: "my-token"
```

| ফিল্ড | টাইপ | ব্যাখ্যা |
|------|------|------|
| `base_url` | `String` | InfluxDB 2.x API ঠিকানা |
| `org` | `String` | সংস্থার নাম |
| `bucket` | `String` | বাকেটের নাম |
| `token` | `String` | অথেনটিকেশন টোকেন |

### Neo4j — Neo4jConfig

```yaml
neo4j:
  base_url: "http://host:7474"
  username: "neo4j"
  password: "secret"
```

| ফিল্ড | টাইপ | ব্যাখ্যা |
|------|------|------|
| `base_url` | `String` | REST API ঠিকানা |
| `username` | `String` | ইউজারনেম |
| `password` | `String` | পাসওয়ার্ড |

### NebulaGraph — NebulaGraphConfig

```yaml
nebulagraph:
  base_url: "http://host:19669"
  space: "my_space"
  # username: "root"      # ঐচ্ছিক
  # password: "nebula"    # ঐচ্ছিক
```

| ফিল্ড | টাইপ | ব্যাখ্যা |
|------|------|------|
| `base_url` | `String` | API ঠিকানা |
| `space` | `String` | গ্রাফ স্পেস নাম |
| `username` | `Option<String>` | ঐচ্ছিক: HTTP Basic Auth ইউজারনেম |
| `password` | `Option<String>` | ঐচ্ছিক: HTTP Basic Auth পাসওয়ার্ড |

### ArangoDB — ArangoConfig

```yaml
arangodb:
  base_url: "http://host:8529"
  db: "mydb"
  username: "root"
  password: "secret"
```

| ফিল্ড | টাইপ | ব্যাখ্যা |
|------|------|------|
| `base_url` | `String` | API ঠিকানা |
| `db` | `String` | ডেটাবেস নাম |
| `username` | `String` | ইউজারনেম |
| `password` | `String` | পাসওয়ার্ড |

### IoTDB — IotdbConfig

```yaml
iotdb:
  base_url: "http://host:18080"
  username: "root"
  password: "root"
```

| ফিল্ড | টাইপ | ব্যাখ্যা |
|------|------|------|
| `base_url` | `String` | REST API ঠিকানা |
| `username` | `String` | ইউজারনেম |
| `password` | `String` | পাসওয়ার্ড |

---

## প্রোগ্রাম্যাটিকভাবে তৈরি

### অথেনটিকেশন ছাড়া

```rust
let es = ElasticsearchClient::new("http://localhost:9200");
let ch = ClickhouseClient::new("http://localhost:8123", "default");
```

### অথেনটিকেশন সহ

```rust
let es = ElasticsearchClient::with_auth("http://es:9200", "elastic", "secret");
let ch = ClickhouseClient::with_auth("http://ch:8123", "default", "admin", "pass");
let qdb = QuestdbClient::with_auth("http://qdb:9000", "admin", "quest");
let ng = NebulaGraphClient::with_auth("http://ng:19669", "space1", "root", "nebula");
```

---

---

## TLS সার্টিফিকেট কনফিগ

সব ডেটা ব্যাকএন্ড ঐচ্ছিক TLS ক্লায়েন্ট অথেনটিকেশন (`tls` ফিল্ড) সমর্থন করে।

### কনফিগ উদাহরণ

```yaml
clickhouse:
  base_url: "https://ch.internal:8443"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"
    # skip_verify: true  # শুধুমাত্র টেস্ট পরিবেশ
```

### সার্টিফিকেট অটো-জেনারেশন (ecat-tls)

```rust
use ecat_tls::{generate_ca, generate_server_cert, generate_client_cert};

// 1. CA তৈরি
let ca = generate_ca("MyOrg")?;
std::fs::write("ca.pem", &ca.cert_pem)?;
std::fs::write("ca-key.pem", &ca.key_pem)?;

// 2. সার্ভার সার্টিফিকেট তৈরি
let srv = generate_server_cert("db.example.com")?;
std::fs::write("server.pem", &srv.cert_pem)?;
std::fs::write("server-key.pem", &srv.key_pem)?;

// 3. ক্লায়েন্ট সার্টিফিকেট তৈরি (mTLS)
let client = generate_client_cert("myapp")?;
std::fs::write("client.pem", &client.cert_pem)?;
std::fs::write("client-key.pem", &client.key_pem)?;
```

### ম্যানুয়াল জেনারেশন (OpenSSL)

```bash
# CA
openssl req -x509 -newkey rsa:4096 -keyout ca-key.pem -out ca.pem -days 3650 -nodes

# সার্ভার সার্টিফিকেট
openssl req -new -newkey rsa:4096 -keyout server-key.pem -out server.csr -nodes -subj "/CN=db.example.com"
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca-key.pem -out server.pem -days 365

# ক্লায়েন্ট সার্টিফিকেট (mTLS)
openssl req -new -newkey rsa:4096 -keyout client-key.pem -out client.csr -nodes -subj "/CN=myapp"
openssl x509 -req -in client.csr -CA ca.pem -CAkey ca-key.pem -out client.pem -days 365
```

### TLS ফিল্ড ব্যাখ্যা

| ফিল্ড | টাইপ | ব্যাখ্যা |
|------|------|------|
| `ca_cert` | `Option<String>` | CA সার্টিফিকেট PEM পাথ (সার্ভার যাচাই) |
| `client_cert` | `Option<String>` | ক্লায়েন্ট সার্টিফিকেট PEM পাথ (mTLS) |
| `client_key` | `Option<String>` | ক্লায়েন্ট প্রাইভেট কী PEM পাথ (mTLS) |
| `skip_verify` | `Option<bool>` | সার্টিফিকেট যাচাই স্কিপ (শুধুমাত্র টেস্ট) |

---

## অ্যাডভান্সড ব্যবহার

### এনভায়রনমেন্ট ভেরিয়েবল ওভাররাইড

```rust
use std::env;

fn load_config() -> Result<SqlxConfig, Box<dyn std::error::Error>> {
    let mut cfg: SqlxConfig = serde_yaml::from_str(
        &std::fs::read_to_string("databases.yaml")?
    )?;
    if let Ok(url) = env::var("DATABASE_URL") {
        cfg.url = url;
    }
    Ok(cfg)
}
```

### ecat-config ফ্রেমওয়ার্কের সাথে

```rust
use ecat_config::{Config, FileSource};

let mut app_config = Config::new();
app_config.load(&FileSource::new("databases.yaml")).await?;

let redis_cfg: RedisConfig = serde_json::from_value(
    app_config.get::<serde_json::Value>("redis").unwrap()
)?;
let cache = RedisCache::from_config(redis_cfg).await?;
```

### প্রয়োজনে কনফিগ

অব্যবহৃত ডেটাবেস YAML-এ বাদ দিন, Rust স্ট্রাক্টে `Option` দিয়ে চিহ্নিত করুন:

```rust
#[derive(Deserialize)]
struct AppConfig {
    sql: SqlxConfig,
    redis: Option<RedisConfig>,
    clickhouse: Option<ClickhouseConfig>,
}
```

---

## সম্পর্কিত ডকুমেন্ট

- [অডিট রিপোর্ট r5](audit-report-2026-08-01-r5.md)
- [TLS সার্টিফিকেট অথেনটিকেশন টিউটোরিয়াল](tls-certificate-tutorial.md)
- [কনফিগ উদাহরণ ফাইল](../../../config/databases.example.yaml)
