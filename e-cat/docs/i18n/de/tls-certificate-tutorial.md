# Tutorial zur TLS-Zertifikatskonfiguration und -authentifizierung

**Version:** 2.4.2 · **Datum:** 2026-08-01

Alle 14 Daten-Backends von e-cat unterstützen die TLS-Client-Zertifikatsauthentifizierung (mTLS). Dieses Tutorial deckt den vollständigen Ablauf von Zertifikatsgenerierung, Konfiguration und Verbindung zu allen Datenbank-Backends ab.

---

## I. Zertifikatsgenerierung

### Methode 1: Automatische Generierung mit ecat-tls (empfohlen)

```rust
use ecat_tls::{generate_ca, generate_server_cert, generate_client_cert};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("certs")?;

    // 1. CA-Zertifikat generieren
    let ca = generate_ca("MyOrganization")?;
    fs::write("certs/ca.pem", &ca.cert_pem)?;
    fs::write("certs/ca-key.pem", &ca.key_pem)?;

    // 2. Serverzertifikat generieren (auf dem Datenbankserver bereitstellen)
    let server = generate_server_cert("db.internal")?;
    fs::write("certs/server.pem", &server.cert_pem)?;
    fs::write("certs/server-key.pem", &server.key_pem)?;

    // 3. Clientzertifikat generieren (Anwendungsseite, mTLS)
    let client = generate_client_cert("myapp")?;
    fs::write("certs/client.pem", &client.cert_pem)?;
    fs::write("certs/client-key.pem", &client.key_pem)?;

    Ok(())
}
```

### Methode 2: Manuelle Generierung mit OpenSSL

```bash
mkdir -p certs && cd certs

# CA generieren
openssl req -x509 -newkey rsa:4096 \
  -keyout ca-key.pem -out ca.pem -days 3650 -nodes \
  -subj "/O=MyOrg/CN=MyOrg CA"

# Serverzertifikat generieren
openssl req -new -newkey rsa:4096 \
  -keyout server-key.pem -out server.csr -nodes \
  -subj "/CN=db.internal"
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca-key.pem \
  -out server.pem -days 365

# Clientzertifikat generieren (mTLS)
openssl req -new -newkey rsa:4096 \
  -keyout client-key.pem -out client.csr -nodes \
  -subj "/CN=myapp"
openssl x509 -req -in client.csr -CA ca.pem -CAkey ca-key.pem \
  -out client.pem -days 365

rm -f *.csr
```

---

## II. TLS-Konfiguration

### Allgemeine TLS-Felder

Alle Backend-Configs unterstützen die folgenden optionalen Felder (`#[serde(default)]`):

| Feld | Typ | Beschreibung |
|------|------|------|
| `tls.ca_cert` | `Option<String>` | PEM-Pfad des CA-Zertifikats (Serverzertifikat validieren) |
| `tls.client_cert` | `Option<String>` | PEM-Pfad des Clientzertifikats (mTLS) |
| `tls.client_key` | `Option<String>` | PEM-Pfad des Client-Private-Keys (mTLS) |
| `tls.skip_verify` | `Option<bool>` | Zertifikatsprüfung überspringen (nur Testumgebung) |

> ⚠️ Gegenseitig ausschließend: `skip_verify=true` zusammen mit `ca_cert` führt beim Aufbau direkt zu einem Fehler (`ecat-tls` lehnt widersprüchliche Konfiguration ab — Prüfung überspringen, aber Vertrauensanker konfigurieren —, um ein stilles Deaktivieren der Zertifikatsprüfung durch Fehlkonfiguration zu verhindern).

### YAML-Konfigurationsbeispiel

```yaml
# Nur Serverzertifikat validieren
elasticsearch:
  base_url: "https://es.internal:9200"
  tls:
    ca_cert: "/etc/ecat/ca.pem"

# mTLS (gegenseitige Authentifizierung)
clickhouse:
  base_url: "https://ch.internal:8443"
  database: "analytics"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"

# Testumgebung (Prüfung überspringen)
questdb:
  base_url: "https://localhost:9000"
  tls:
    skip_verify: true
```

---

## III. TLS-Konfiguration je Backend

### HTTP-Backends (9)

Elasticsearch, OpenSearch, ClickHouse, QuestDB, InfluxDB, Neo4j, NebulaGraph, ArangoDB, IoTDB — einheitlich über `TlsClientConfig::build_reqwest_client()` einen TLS-Client aufbauen.

```yaml
# Alle HTTP-Backends verwenden dasselbe Format
backend:
  base_url: "https://host:port"
  tls:
    ca_cert: "/path/to/ca.pem"
    client_cert: "/path/to/client.pem"   # für mTLS erforderlich
    client_key: "/path/to/client-key.pem" # für mTLS erforderlich
```

### Redis — automatischer URL-Scheme-Wechsel

```yaml
redis:
  url: "redis://cache.internal:6379"    # TLS aktivieren → automatischer Wechsel auf rediss://
  tls:
    ca_cert: "/etc/ecat/ca.pem"
```

### RDBMS (Sqlx) — Konfiguration über URL-Parameter

```yaml
sql:
  url: "postgres://db.internal:5432/mydb?sslmode=require"
  tls: {}  # reserviertes Feld
```

| Datenbank | TLS-URL-Parameter |
|--------|------------|
| PostgreSQL | `?sslmode=require` oder `?sslmode=verify-full` |
| MySQL | `?ssl-mode=VERIFY_CA&ssl-ca=/path/to/ca.pem` |
| TiDB | `?ssl-mode=VERIFY_IDENTITY&ssl-ca=/path/to/ca.pem` |
| SQLite | kein TLS erforderlich |

---

## IV. Laden im Rust-Code

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

    // from_config ruft intern tls.build_reqwest_client() auf — TLS wird automatisch angewendet
    let es = ElasticsearchClient::from_config(cfg.elasticsearch);
    let ch = ClickhouseClient::from_config(cfg.clickhouse);

    let results = es.search("logs", &serde_json::json!({"match_all": {}})).await?;
    Ok(())
}
```

---

## V. Programmgestützte Erstellung (TLS + Authentifizierung)

```rust
use ecat_tls::TlsClientConfig;

// TLS-Client manuell aufbauen
let tls = TlsClientConfig {
    ca_cert: Some("/etc/ecat/ca.pem".into()),
    client_cert: Some("/etc/ecat/client.pem".into()),
    client_key: Some("/etc/ecat/client-key.pem".into()),
    skip_verify: None,
};
let client = tls.build_reqwest_client()?;

// Oder with_auth + TLS-Konfiguration verwenden
let es = ElasticsearchClient::with_auth(
    "https://es.internal:9200", "elastic", "secret"
);
```

---

## VI. Sicherheitsempfehlungen

1. **In der Produktion müssen Zertifikate validiert werden** — `skip_verify` deaktivieren
2. **CA-Private-Key sicher aufbewahren** — nicht versionieren
3. **Zertifikatsgültigkeit verwalten** — vor Ablauf erneuern und rotieren
4. **mTLS erhöht die Sicherheit** — in der Produktion zusätzlich Clientzertifikate konfigurieren

---

## Verwandte Dokumente

- [Tutorial zur Datenbankkonfiguration](database-config-tutorial.md)
- [Auditbericht r5](audit-report-2026-08-01-r5.md)
- [Beispiel-Konfigurationsdatei](../../../config/databases.example.yaml)
