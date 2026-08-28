# TLS प्रमाणपत्र कॉन्फ़िगरेशन और प्रमाणीकरण ट्यूटोरियल

**संस्करण:** 2.4.2 · **दिनांक:** 2026-08-01

e-cat के 14 डेटा बैकएंड सभी TLS क्लाइंट प्रमाणपत्र प्रमाणीकरण (mTLS) का समर्थन करते हैं। यह ट्यूटोरियल प्रमाणपत्र जनरेशन, कॉन्फ़िगरेशन, और सभी डेटाबेस बैकएंड से कनेक्ट करने की पूरी प्रक्रिया को कवर करता है।

---

## एक、प्रमाणपत्र जनरेशन

### विधि 1: ecat-tls स्वतः जनरेशन (अनुशंसित)

```rust
use ecat_tls::{generate_ca, generate_server_cert, generate_client_cert};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("certs")?;

    // 1. CA प्रमाणपत्र जनरेट करें
    let ca = generate_ca("MyOrganization")?;
    fs::write("certs/ca.pem", &ca.cert_pem)?;
    fs::write("certs/ca-key.pem", &ca.key_pem)?;

    // 2. सर्वर प्रमाणपत्र जनरेट करें (डेटाबेस सर्वर पर डिप्लॉय करें)
    let server = generate_server_cert("db.internal")?;
    fs::write("certs/server.pem", &server.cert_pem)?;
    fs::write("certs/server-key.pem", &server.key_pem)?;

    // 3. क्लाइंट प्रमाणपत्र जनरेट करें (एप्लिकेशन पक्ष, mTLS)
    let client = generate_client_cert("myapp")?;
    fs::write("certs/client.pem", &client.cert_pem)?;
    fs::write("certs/client-key.pem", &client.key_pem)?;

    Ok(())
}
```

### विधि 2: OpenSSL मैन्युअल जनरेशन

```bash
mkdir -p certs && cd certs

# CA जनरेट करें
openssl req -x509 -newkey rsa:4096 \
  -keyout ca-key.pem -out ca.pem -days 3650 -nodes \
  -subj "/O=MyOrg/CN=MyOrg CA"

# सर्वर प्रमाणपत्र जनरेट करें
openssl req -new -newkey rsa:4096 \
  -keyout server-key.pem -out server.csr -nodes \
  -subj "/CN=db.internal"
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca-key.pem \
  -out server.pem -days 365

# क्लाइंट प्रमाणपत्र जनरेट करें (mTLS)
openssl req -new -newkey rsa:4096 \
  -keyout client-key.pem -out client.csr -nodes \
  -subj "/CN=myapp"
openssl x509 -req -in client.csr -CA ca.pem -CAkey ca-key.pem \
  -out client.pem -days 365

rm -f *.csr
```

---

## दो、TLS कॉन्फ़िगरेशन

### सामान्य TLS फ़ील्ड

सभी बैकएंड Config निम्न वैकल्पिक फ़ील्ड का समर्थन करते हैं (`#[serde(default)]`)：

| फ़ील्ड | प्रकार | स्पष्टीकरण |
|------|------|------|
| `tls.ca_cert` | `Option<String>` | CA प्रमाणपत्र PEM पथ (सर्वर प्रमाणपत्र सत्यापन) |
| `tls.client_cert` | `Option<String>` | क्लाइंट प्रमाणपत्र PEM पथ (mTLS) |
| `tls.client_key` | `Option<String>` | क्लाइंट निजी कुंजी PEM पथ (mTLS) |
| `tls.skip_verify` | `Option<bool>` | प्रमाणपत्र सत्यापन छोड़ें (केवल परीक्षण वातावरण) |

> ⚠️ परस्पर अपवर्जी: `skip_verify=true` और `ca_cert` एक साथ कॉन्फ़िगर करने पर बिल्ड के समय सीधे त्रुटि होती है (`ecat-tls` विरोधाभासी कॉन्फ़िगरेशन अस्वीकार करता है — सत्यापन छोड़कर भी ट्रस्ट एंकर कॉन्फ़िगर करना, गलत कॉन्फ़िगरेशन से प्रमाणपत्र सत्यापन चुपचाप बंद होने से रोकने के लिए)।

### YAML कॉन्फ़िगरेशन उदाहरण

```yaml
# केवल सर्वर प्रमाणपत्र सत्यापन
elasticsearch:
  base_url: "https://es.internal:9200"
  tls:
    ca_cert: "/etc/ecat/ca.pem"

# mTLS (द्विपक्षीय प्रमाणीकरण)
clickhouse:
  base_url: "https://ch.internal:8443"
  database: "analytics"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"

# परीक्षण वातावरण (सत्यापन छोड़ें)
questdb:
  base_url: "https://localhost:9000"
  tls:
    skip_verify: true
```

---

## तीन、प्रत्येक बैकएंड TLS कॉन्फ़िगरेशन

### HTTP बैकएंड (9)

Elasticsearch, OpenSearch, ClickHouse, QuestDB, InfluxDB, Neo4j, NebulaGraph, ArangoDB, IoTDB — सभी एकीकृत रूप से `TlsClientConfig::build_reqwest_client()` से TLS क्लाइंट बनाते हैं।

```yaml
# सभी HTTP बैकएंड एक ही प्रारूप का उपयोग करते हैं
backend:
  base_url: "https://host:port"
  tls:
    ca_cert: "/path/to/ca.pem"
    client_cert: "/path/to/client.pem"   # mTLS के लिए आवश्यक
    client_key: "/path/to/client-key.pem" # mTLS के लिए आवश्यक
```

### Redis — स्वचालित URL scheme स्विच

```yaml
redis:
  url: "redis://cache.internal:6379"    # TLS सक्षम → स्वतः rediss:// पर स्विच
  tls:
    ca_cert: "/etc/ecat/ca.pem"
```

### RDBMS (Sqlx) — URL पैरामीटर कॉन्फ़िगरेशन

```yaml
sql:
  url: "postgres://db.internal:5432/mydb?sslmode=require"
  tls: {}  # आरक्षित फ़ील्ड
```

| डेटाबेस | TLS URL पैरामीटर |
|--------|------------|
| PostgreSQL | `?sslmode=require` या `?sslmode=verify-full` |
| MySQL | `?ssl-mode=VERIFY_CA&ssl-ca=/path/to/ca.pem` |
| TiDB | `?ssl-mode=VERIFY_IDENTITY&ssl-ca=/path/to/ca.pem` |
| SQLite | TLS की आवश्यकता नहीं |

---

## चार、Rust कोड लोडिंग

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

    // from_config आंतरिक रूप से tls.build_reqwest_client() कॉल करता है — TLS स्वतः लागू होता है
    let es = ElasticsearchClient::from_config(cfg.elasticsearch);
    let ch = ClickhouseClient::from_config(cfg.clickhouse);

    let results = es.search("logs", &serde_json::json!({"match_all": {}})).await?;
    Ok(())
}
```

---

## पाँच、प्रोग्रामेटिक निर्माण (TLS + प्रमाणीकरण)

```rust
use ecat_tls::TlsClientConfig;

// मैन्युअल रूप से TLS क्लाइंट बनाएं
let tls = TlsClientConfig {
    ca_cert: Some("/etc/ecat/ca.pem".into()),
    client_cert: Some("/etc/ecat/client.pem".into()),
    client_key: Some("/etc/ecat/client-key.pem".into()),
    skip_verify: None,
};
let client = tls.build_reqwest_client()?;

// या with_auth + TLS कॉन्फ़िगरेशन का उपयोग करें
let es = ElasticsearchClient::with_auth(
    "https://es.internal:9200", "elastic", "secret"
);
```

---

## छह、सुरक्षा सुझाव

1. **प्रोडक्शन में प्रमाणपत्र सत्यापन अनिवार्य है** — `skip_verify` अक्षम करें
2. **CA निजी कुंजी सुरक्षित रूप से संग्रहीत करें** — संस्करण नियंत्रण में शामिल न करें
3. **प्रमाणपत्र वैधता अवधि प्रबंधन** — समाप्ति से पहले नवीनीकरण और रोटेशन करें
4. **mTLS सुरक्षा बढ़ाता है** — प्रोडक्शन में क्लाइंट प्रमाणपत्र कॉन्फ़िगर करने की अनुशंसा

---

## संबंधित दस्तावेज़

- [डेटाबेस कॉन्फ़िगरेशन ट्यूटोरियल](database-config-tutorial.md)
- [ऑडिट रिपोर्ट r5](audit-report-2026-08-01-r5.md)
- [उदाहरण कॉन्फ़िगरेशन फ़ाइल](../../../config/databases.example.yaml)
