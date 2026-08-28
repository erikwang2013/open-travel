# Tutorial zur Datenbankkonfiguration

**Version:** 2.4.2 · **Datum:** 2026-08-01

Alle 14 Daten-Backends von e-cat unterstützen das Laden der Verbindungsinformationen aus Konfigurationsdateien, ohne sie im Code hart zu codieren. `username` / `password` sind beides optionale Felder; werden sie weggelassen, entfällt die Authentifizierung.

---

## Schnellstart

### 1. Konfigurationsdatei erstellen

Die Beispielvorlage kopieren und an die eigene Umgebung anpassen:

```bash
cp config/databases.example.yaml databases.yaml
```

`databases.yaml` bearbeiten und die echten Verbindungsinformationen eintragen:

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

### 2. Abhängigkeiten einbinden

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
ecat-data-sqlx = { path = "../ecat-data-sqlx" }
ecat-data-redis = { path = "../ecat-data-redis" }
ecat-data-clickhouse = { path = "../ecat-data-clickhouse" }
```

### 3. Laden und verwenden

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
    // YAML-Konfiguration laden
    let yaml = std::fs::read_to_string("databases.yaml")?;
    let cfg: AppConfig = serde_yaml::from_str(&yaml)?;

    // Datenbank-Clients erstellen — keine hart codierten Verbindungsinformationen
    let db = SqlxClient::from_config(cfg.sql).await?;
    let cache = RedisCache::from_config(cfg.redis).await?;
    let ch = ClickhouseClient::from_config(cfg.clickhouse);

    // Verwenden
    let rows = db.query("SELECT id, name FROM users LIMIT 10").await?;
    cache.set("health", b"ok", std::time::Duration::from_secs(30)).await?;

    Ok(())
}
```

---

## Vollständige Konfigurationsreferenz

### Top-Level-Konfigurationsstruktur definieren

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

### Vollständiges YAML-Beispiel

Siehe `config/databases.example.yaml`.

---

## Feld-Schnellreferenz der Backend-Configs

### RDBMS — SqlxConfig

```yaml
sql:
  url: "postgres://host:5432/dbname"
  # username: "app_user"    # optional
  # password: "secret"      # optional
```

| Feld | Typ | Beschreibung |
|------|------|------|
| `url` | `String` | sqlx-Verbindungsstring, unterstützt SQLite/PG/MySQL/TiDB |
| `username` | `Option<String>` | optional: Authentifizierung über URL-Einbettung (zusammen mit password) |
| `password` | `Option<String>` | optional: Authentifizierung über URL-Einbettung (zusammen mit username) |

### Redis — RedisConfig

```yaml
redis:
  url: "redis://host:6379"
  # password: "auth_token"  # optional
```

| Feld | Typ | Beschreibung |
|------|------|------|
| `url` | `String` | Redis-Verbindungs-URL |
| `password` | `Option<String>` | optional: Redis-AUTH-Passwort |

### Memcached — MemcachedConfig

```yaml
memcached:
  # username: "memcache"    # optional: reserviertes Feld (aktuell Speicherimplementierung)
  # password: "secret"      # optional: reserviertes Feld
  {}
```

| Feld | Typ | Beschreibung |
|------|------|------|
| `username` | `Option<String>` | optional: reserviertes Feld |
| `password` | `Option<String>` | optional: reserviertes Feld |

Aktuell Speicherimplementierung, Authentifizierungsfelder sind vorbehalten.

### ClickHouse — ClickhouseConfig

```yaml
clickhouse:
  base_url: "http://host:8123"
  database: "default"
  # username: "default"   # optional
  # password: "secret"    # optional
```

| Feld | Typ | Standardwert | Beschreibung |
|------|------|--------|------|
| `base_url` | `String` | — | HTTP-Schnittstellenadresse |
| `database` | `String` | `"default"` | Datenbankname |
| `username` | `Option<String>` | `None` | optional: HTTP-Basic-Auth-Benutzername |
| `password` | `Option<String>` | `None` | optional: HTTP-Basic-Auth-Passwort |

### QuestDB — QuestdbConfig

```yaml
questdb:
  base_url: "http://host:9000"
  # username: "admin"     # optional
  # password: "quest"     # optional
```

| Feld | Typ | Beschreibung |
|------|------|------|
| `base_url` | `String` | HTTP-API-Adresse |
| `username` | `Option<String>` | optional: HTTP-Basic-Auth-Benutzername |
| `password` | `Option<String>` | optional: HTTP-Basic-Auth-Passwort |

### Elasticsearch — ElasticsearchConfig

```yaml
elasticsearch:
  base_url: "http://host:9200"
  # username: "elastic"   # optional
  # password: "secret"    # optional
```

| Feld | Typ | Beschreibung |
|------|------|------|
| `base_url` | `String` | REST-API-Adresse |
| `username` | `Option<String>` | optional: HTTP-Basic-Auth-Benutzername |
| `password` | `Option<String>` | optional: HTTP-Basic-Auth-Passwort |

### OpenSearch — OpenSearchConfig

```yaml
opensearch:
  base_url: "http://host:9200"
  # username: "admin"     # optional
  # password: "secret"    # optional
```

| Feld | Typ | Beschreibung |
|------|------|------|
| `base_url` | `String` | REST-API-Adresse |
| `username` | `Option<String>` | optional: HTTP-Basic-Auth-Benutzername |
| `password` | `Option<String>` | optional: HTTP-Basic-Auth-Passwort |

### InfluxDB — InfluxConfig

```yaml
influxdb:
  base_url: "http://host:8086"
  org: "myorg"
  bucket: "mybucket"
  token: "my-token"
```

| Feld | Typ | Beschreibung |
|------|------|------|
| `base_url` | `String` | InfluxDB-2.x-API-Adresse |
| `org` | `String` | Organisationsname |
| `bucket` | `String` | Bucket-Name |
| `token` | `String` | Authentifizierungs-Token |

### Neo4j — Neo4jConfig

```yaml
neo4j:
  base_url: "http://host:7474"
  username: "neo4j"
  password: "secret"
```

| Feld | Typ | Beschreibung |
|------|------|------|
| `base_url` | `String` | REST-API-Adresse |
| `username` | `String` | Benutzername |
| `password` | `String` | Passwort |

### NebulaGraph — NebulaGraphConfig

```yaml
nebulagraph:
  base_url: "http://host:19669"
  space: "my_space"
  # username: "root"      # optional
  # password: "nebula"    # optional
```

| Feld | Typ | Beschreibung |
|------|------|------|
| `base_url` | `String` | API-Adresse |
| `space` | `String` | Graph-Space-Name |
| `username` | `Option<String>` | optional: HTTP-Basic-Auth-Benutzername |
| `password` | `Option<String>` | optional: HTTP-Basic-Auth-Passwort |

### ArangoDB — ArangoConfig

```yaml
arangodb:
  base_url: "http://host:8529"
  db: "mydb"
  username: "root"
  password: "secret"
```

| Feld | Typ | Beschreibung |
|------|------|------|
| `base_url` | `String` | API-Adresse |
| `db` | `String` | Datenbankname |
| `username` | `String` | Benutzername |
| `password` | `String` | Passwort |

### IoTDB — IotdbConfig

```yaml
iotdb:
  base_url: "http://host:18080"
  username: "root"
  password: "root"
```

| Feld | Typ | Beschreibung |
|------|------|------|
| `base_url` | `String` | REST-API-Adresse |
| `username` | `String` | Benutzername |
| `password` | `String` | Passwort |

---

## Programmgestützte Erstellung

### Ohne Authentifizierung

```rust
let es = ElasticsearchClient::new("http://localhost:9200");
let ch = ClickhouseClient::new("http://localhost:8123", "default");
```

### Mit Authentifizierung

```rust
let es = ElasticsearchClient::with_auth("http://es:9200", "elastic", "secret");
let ch = ClickhouseClient::with_auth("http://ch:8123", "default", "admin", "pass");
let qdb = QuestdbClient::with_auth("http://qdb:9000", "admin", "quest");
let ng = NebulaGraphClient::with_auth("http://ng:19669", "space1", "root", "nebula");
```

---

---

## TLS-Zertifikatskonfiguration

Alle Daten-Backends unterstützen optionale TLS-Client-Authentifizierung (Feld `tls`).

### Konfigurationsbeispiel

```yaml
clickhouse:
  base_url: "https://ch.internal:8443"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"
    # skip_verify: true  # nur Testumgebung
```

### Automatische Zertifikatsgenerierung (ecat-tls)

```rust
use ecat_tls::{generate_ca, generate_server_cert, generate_client_cert};

// 1. CA generieren
let ca = generate_ca("MyOrg")?;
std::fs::write("ca.pem", &ca.cert_pem)?;
std::fs::write("ca-key.pem", &ca.key_pem)?;

// 2. Serverzertifikat generieren
let srv = generate_server_cert("db.example.com")?;
std::fs::write("server.pem", &srv.cert_pem)?;
std::fs::write("server-key.pem", &srv.key_pem)?;

// 3. Clientzertifikat generieren (mTLS)
let client = generate_client_cert("myapp")?;
std::fs::write("client.pem", &client.cert_pem)?;
std::fs::write("client-key.pem", &client.key_pem)?;
```

### Manuelle Generierung (OpenSSL)

```bash
# CA
openssl req -x509 -newkey rsa:4096 -keyout ca-key.pem -out ca.pem -days 3650 -nodes

# Serverzertifikat
openssl req -new -newkey rsa:4096 -keyout server-key.pem -out server.csr -nodes -subj "/CN=db.example.com"
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca-key.pem -out server.pem -days 365

# Clientzertifikat (mTLS)
openssl req -new -newkey rsa:4096 -keyout client-key.pem -out client.csr -nodes -subj "/CN=myapp"
openssl x509 -req -in client.csr -CA ca.pem -CAkey ca-key.pem -out client.pem -days 365
```

### TLS-Feld-Beschreibung

| Feld | Typ | Beschreibung |
|------|------|------|
| `ca_cert` | `Option<String>` | PEM-Pfad des CA-Zertifikats (Servervalidierung) |
| `client_cert` | `Option<String>` | PEM-Pfad des Clientzertifikats (mTLS) |
| `client_key` | `Option<String>` | PEM-Pfad des Client-Private-Keys (mTLS) |
| `skip_verify` | `Option<bool>` | Zertifikatsprüfung überspringen (nur Tests) |

---

## Fortgeschrittene Verwendung

### Umgebungsvariablen-Überschreibung

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

### Kombination mit dem ecat-config-Framework

```rust
use ecat_config::{Config, FileSource};

let mut app_config = Config::new();
app_config.load(&FileSource::new("databases.yaml")).await?;

let redis_cfg: RedisConfig = serde_json::from_value(
    app_config.get::<serde_json::Value>("redis").unwrap()
)?;
let cache = RedisCache::from_config(redis_cfg).await?;
```

### Bedarfsgerechte Konfiguration

Nicht verwendete Datenbanken im YAML weglassen, in der Rust-Struktur mit `Option` markieren:

```rust
#[derive(Deserialize)]
struct AppConfig {
    sql: SqlxConfig,
    redis: Option<RedisConfig>,
    clickhouse: Option<ClickhouseConfig>,
}
```

---

## Verwandte Dokumente

- [Auditbericht r5](audit-report-2026-08-01-r5.md)
- [Tutorial zur TLS-Zertifikatsauthentifizierung](tls-certificate-tutorial.md)
- [Beispiel-Konfigurationsdatei](../../../config/databases.example.yaml)
