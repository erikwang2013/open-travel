# डेटाबेस कॉन्फ़िगरेशन ट्यूटोरियल

**संस्करण:** 2.4.2 · **दिनांक:** 2026-08-01

e-cat के 14 डेटा बैकएंड सभी कॉन्फ़िगरेशन फ़ाइल से कनेक्शन जानकारी लोड करने का समर्थन करते हैं, कोड में हार्डकोडिंग की आवश्यकता नहीं। `username` / `password` दोनों वैकल्पिक फ़ील्ड हैं, छोड़ने पर प्रमाणीकरण छोड़ दिया जाता है।

---

## त्वरित आरंभ

### 1. कॉन्फ़िगरेशन फ़ाइल बनाएं

उदाहरण टेम्पलेट कॉपी करें और वास्तविक वातावरण के अनुसार संशोधित करें:

```bash
cp config/databases.example.yaml databases.yaml
```

`databases.yaml` संपादित करें, वास्तविक कनेक्शन जानकारी भरें:

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

### 2. निर्भरताएँ जोड़ें

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
ecat-data-sqlx = { path = "../ecat-data-sqlx" }
ecat-data-redis = { path = "../ecat-data-redis" }
ecat-data-clickhouse = { path = "../ecat-data-clickhouse" }
```

### 3. लोड करें और उपयोग करें

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
    // YAML कॉन्फ़िगरेशन लोड करें
    let yaml = std::fs::read_to_string("databases.yaml")?;
    let cfg: AppConfig = serde_yaml::from_str(&yaml)?;

    // डेटाबेस क्लाइंट बनाएं — कोई हार्डकोडेड कनेक्शन जानकारी नहीं
    let db = SqlxClient::from_config(cfg.sql).await?;
    let cache = RedisCache::from_config(cfg.redis).await?;
    let ch = ClickhouseClient::from_config(cfg.clickhouse);

    // उपयोग
    let rows = db.query("SELECT id, name FROM users LIMIT 10").await?;
    cache.set("health", b"ok", std::time::Duration::from_secs(30)).await?;

    Ok(())
}
```

---

## पूर्ण कॉन्फ़िगरेशन संदर्भ

### टॉप-लेवल कॉन्फ़िगरेशन स्ट्रक्चर परिभाषित करें

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

### YAML पूर्ण उदाहरण

`config/databases.example.yaml` देखें।

---

## प्रत्येक बैकएंड Config फ़ील्ड त्वरित संदर्भ

### RDBMS — SqlxConfig

```yaml
sql:
  url: "postgres://host:5432/dbname"
  # username: "app_user"    # वैकल्पिक
  # password: "secret"      # वैकल्पिक
```

| फ़ील्ड | प्रकार | स्पष्टीकरण |
|------|------|------|
| `url` | `String` | sqlx कनेक्शन स्ट्रिंग, SQLite/PG/MySQL/TiDB का समर्थन |
| `username` | `Option<String>` | वैकल्पिक: URL में एम्बेडेड प्रमाणीकरण (password के साथ) |
| `password` | `Option<String>` | वैकल्पिक: URL में एम्बेडेड प्रमाणीकरण (username के साथ) |

### Redis — RedisConfig

```yaml
redis:
  url: "redis://host:6379"
  # password: "auth_token"  # वैकल्पिक
```

| फ़ील्ड | प्रकार | स्पष्टीकरण |
|------|------|------|
| `url` | `String` | Redis कनेक्शन URL |
| `password` | `Option<String>` | वैकल्पिक: Redis AUTH पासवर्ड |

### Memcached — MemcachedConfig

```yaml
memcached:
  # username: "memcache"    # वैकल्पिक: आरक्षित फ़ील्ड (वर्तमान में मेमोरी कार्यान्वयन)
  # password: "secret"      # वैकल्पिक: आरक्षित फ़ील्ड
  {}
```

| फ़ील्ड | प्रकार | स्पष्टीकरण |
|------|------|------|
| `username` | `Option<String>` | वैकल्पिक: आरक्षित फ़ील्ड |
| `password` | `Option<String>` | वैकल्पिक: आरक्षित फ़ील्ड |

वर्तमान में मेमोरी कार्यान्वयन है, प्रमाणीकरण फ़ील्ड आरक्षित हैं।

### ClickHouse — ClickhouseConfig

```yaml
clickhouse:
  base_url: "http://host:8123"
  database: "default"
  # username: "default"   # वैकल्पिक
  # password: "secret"    # वैकल्पिक
```

| फ़ील्ड | प्रकार | डिफ़ॉल्ट मान | स्पष्टीकरण |
|------|------|--------|------|
| `base_url` | `String` | — | HTTP इंटरफ़ेस पता |
| `database` | `String` | `"default"` | डेटाबेस नाम |
| `username` | `Option<String>` | `None` | वैकल्पिक: HTTP Basic Auth उपयोगकर्ता नाम |
| `password` | `Option<String>` | `None` | वैकल्पिक: HTTP Basic Auth पासवर्ड |

### QuestDB — QuestdbConfig

```yaml
questdb:
  base_url: "http://host:9000"
  # username: "admin"     # वैकल्पिक
  # password: "quest"     # वैकल्पिक
```

| फ़ील्ड | प्रकार | स्पष्टीकरण |
|------|------|------|
| `base_url` | `String` | HTTP API पता |
| `username` | `Option<String>` | वैकल्पिक: HTTP Basic Auth उपयोगकर्ता नाम |
| `password` | `Option<String>` | वैकल्पिक: HTTP Basic Auth पासवर्ड |

### Elasticsearch — ElasticsearchConfig

```yaml
elasticsearch:
  base_url: "http://host:9200"
  # username: "elastic"   # वैकल्पिक
  # password: "secret"    # वैकल्पिक
```

| फ़ील्ड | प्रकार | स्पष्टीकरण |
|------|------|------|
| `base_url` | `String` | REST API पता |
| `username` | `Option<String>` | वैकल्पिक: HTTP Basic Auth उपयोगकर्ता नाम |
| `password` | `Option<String>` | वैकल्पिक: HTTP Basic Auth पासवर्ड |

### OpenSearch — OpenSearchConfig

```yaml
opensearch:
  base_url: "http://host:9200"
  # username: "admin"     # वैकल्पिक
  # password: "secret"    # वैकल्पिक
```

| फ़ील्ड | प्रकार | स्पष्टीकरण |
|------|------|------|
| `base_url` | `String` | REST API पता |
| `username` | `Option<String>` | वैकल्पिक: HTTP Basic Auth उपयोगकर्ता नाम |
| `password` | `Option<String>` | वैकल्पिक: HTTP Basic Auth पासवर्ड |

### InfluxDB — InfluxConfig

```yaml
influxdb:
  base_url: "http://host:8086"
  org: "myorg"
  bucket: "mybucket"
  token: "my-token"
```

| फ़ील्ड | प्रकार | स्पष्टीकरण |
|------|------|------|
| `base_url` | `String` | InfluxDB 2.x API पता |
| `org` | `String` | संगठन नाम |
| `bucket` | `String` | बकेट नाम |
| `token` | `String` | प्रमाणीकरण टोकन |

### Neo4j — Neo4jConfig

```yaml
neo4j:
  base_url: "http://host:7474"
  username: "neo4j"
  password: "secret"
```

| फ़ील्ड | प्रकार | स्पष्टीकरण |
|------|------|------|
| `base_url` | `String` | REST API पता |
| `username` | `String` | उपयोगकर्ता नाम |
| `password` | `String` | पासवर्ड |

### NebulaGraph — NebulaGraphConfig

```yaml
nebulagraph:
  base_url: "http://host:19669"
  space: "my_space"
  # username: "root"      # वैकल्पिक
  # password: "nebula"    # वैकल्पिक
```

| फ़ील्ड | प्रकार | स्पष्टीकरण |
|------|------|------|
| `base_url` | `String` | API पता |
| `space` | `String` | ग्राफ स्पेस नाम |
| `username` | `Option<String>` | वैकल्पिक: HTTP Basic Auth उपयोगकर्ता नाम |
| `password` | `Option<String>` | वैकल्पिक: HTTP Basic Auth पासवर्ड |

### ArangoDB — ArangoConfig

```yaml
arangodb:
  base_url: "http://host:8529"
  db: "mydb"
  username: "root"
  password: "secret"
```

| फ़ील्ड | प्रकार | स्पष्टीकरण |
|------|------|------|
| `base_url` | `String` | API पता |
| `db` | `String` | डेटाबेस नाम |
| `username` | `String` | उपयोगकर्ता नाम |
| `password` | `String` | पासवर्ड |

### IoTDB — IotdbConfig

```yaml
iotdb:
  base_url: "http://host:18080"
  username: "root"
  password: "root"
```

| फ़ील्ड | प्रकार | स्पष्टीकरण |
|------|------|------|
| `base_url` | `String` | REST API पता |
| `username` | `String` | उपयोगकर्ता नाम |
| `password` | `String` | पासवर्ड |

---

## प्रोग्रामेटिक निर्माण

### प्रमाणीकरण के बिना

```rust
let es = ElasticsearchClient::new("http://localhost:9200");
let ch = ClickhouseClient::new("http://localhost:8123", "default");
```

### प्रमाणीकरण के साथ

```rust
let es = ElasticsearchClient::with_auth("http://es:9200", "elastic", "secret");
let ch = ClickhouseClient::with_auth("http://ch:8123", "default", "admin", "pass");
let qdb = QuestdbClient::with_auth("http://qdb:9000", "admin", "quest");
let ng = NebulaGraphClient::with_auth("http://ng:19669", "space1", "root", "nebula");
```

---

---

## TLS प्रमाणपत्र कॉन्फ़िगरेशन

सभी डेटा बैकएंड वैकल्पिक TLS क्लाइंट प्रमाणीकरण (`tls` फ़ील्ड) का समर्थन करते हैं।

### कॉन्फ़िगरेशन उदाहरण

```yaml
clickhouse:
  base_url: "https://ch.internal:8443"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"
    # skip_verify: true  # केवल परीक्षण वातावरण
```

### प्रमाणपत्र स्वतः जनरेशन (ecat-tls)

```rust
use ecat_tls::{generate_ca, generate_server_cert, generate_client_cert};

// 1. CA जनरेट करें
let ca = generate_ca("MyOrg")?;
std::fs::write("ca.pem", &ca.cert_pem)?;
std::fs::write("ca-key.pem", &ca.key_pem)?;

// 2. सर्वर प्रमाणपत्र जनरेट करें
let srv = generate_server_cert("db.example.com")?;
std::fs::write("server.pem", &srv.cert_pem)?;
std::fs::write("server-key.pem", &srv.key_pem)?;

// 3. क्लाइंट प्रमाणपत्र जनरेट करें (mTLS)
let client = generate_client_cert("myapp")?;
std::fs::write("client.pem", &client.cert_pem)?;
std::fs::write("client-key.pem", &client.key_pem)?;
```

### मैन्युअल जनरेशन (OpenSSL)

```bash
# CA
openssl req -x509 -newkey rsa:4096 -keyout ca-key.pem -out ca.pem -days 3650 -nodes

# सर्वर प्रमाणपत्र
openssl req -new -newkey rsa:4096 -keyout server-key.pem -out server.csr -nodes -subj "/CN=db.example.com"
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca-key.pem -out server.pem -days 365

# क्लाइंट प्रमाणपत्र (mTLS)
openssl req -new -newkey rsa:4096 -keyout client-key.pem -out client.csr -nodes -subj "/CN=myapp"
openssl x509 -req -in client.csr -CA ca.pem -CAkey ca-key.pem -out client.pem -days 365
```

### TLS फ़ील्ड स्पष्टीकरण

| फ़ील्ड | प्रकार | स्पष्टीकरण |
|------|------|------|
| `ca_cert` | `Option<String>` | CA प्रमाणपत्र PEM पथ (सर्वर सत्यापन) |
| `client_cert` | `Option<String>` | क्लाइंट प्रमाणपत्र PEM पथ (mTLS) |
| `client_key` | `Option<String>` | क्लाइंट निजी कुंजी PEM पथ (mTLS) |
| `skip_verify` | `Option<bool>` | प्रमाणपत्र सत्यापन छोड़ें (केवल परीक्षण) |

---

## उन्नत उपयोग

### पर्यावरण चर ओवरराइड

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

### ecat-config फ्रेमवर्क के साथ संयोजन

```rust
use ecat_config::{Config, FileSource};

let mut app_config = Config::new();
app_config.load(&FileSource::new("databases.yaml")).await?;

let redis_cfg: RedisConfig = serde_json::from_value(
    app_config.get::<serde_json::Value>("redis").unwrap()
)?;
let cache = RedisCache::from_config(redis_cfg).await?;
```

### आवश्यकता अनुसार कॉन्फ़िगरेशन

अनुपयोगी डेटाबेस YAML में छोड़ दें, Rust स्ट्रक्चर में `Option` से चिह्नित करें:

```rust
#[derive(Deserialize)]
struct AppConfig {
    sql: SqlxConfig,
    redis: Option<RedisConfig>,
    clickhouse: Option<ClickhouseConfig>,
}
```

---

## संबंधित दस्तावेज़

- [ऑडिट रिपोर्ट r5](audit-report-2026-08-01-r5.md)
- [TLS प्रमाणपत्र प्रमाणीकरण ट्यूटोरियल](tls-certificate-tutorial.md)
- [उदाहरण कॉन्फ़िगरेशन फ़ाइल](../../../config/databases.example.yaml)
