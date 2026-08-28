# E-CAT Auditbericht — r5

**Datum**: 2026-08-01  
**Branch**: main  
**Version**: 2.1.7  
**Crate-Anzahl**: 47 (workspace members)
**Status**: ✅ alle behebbaren Probleme gelöst + Daten-Backends mit vollständiger Konfigurationsdatei-Unterstützung

---

## 0. Fix-Protokoll (2026-08-01)

| # | Problem | Datei | Fix |
|---|------|------|------|
| 1 | unused import `axum::routing::get` | `ecat-versioning/src/lib.rs:3` | Top-Level-Import entfernt, in `#[cfg(test)]` verschoben |
| 2 | unused variable `version` | `ecat-versioning/src/lib.rs:61` | in `_version` umbenannt |
| 3 | dead code `extract_version` | `ecat-versioning/src/lib.rs:68` | als `pub fn` deklariert |
| 4 | `useless_format!` | `ecat-versioning/src/lib.rs:62` | direkt `"/api"` |
| 5 | `unnecessary_to_owned` | `ecat-data-questdb/src/lib.rs:39` | `"true".to_string()` → `"true"` |
| 6 | Fehlermeldung verschluckt | `ecat-data-questdb/src/lib.rs:30` | `unwrap_or_default()` → `unwrap_or_else(...)` |
| 7 | `derivable_impls` | `ecat-client/src/lib.rs:249` | `GrpcClientBuilder` nutzt `#[derive(Default)]` |
| 8 | `manual_is_multiple_of` | `ecat-config/src/encrypted.rs:60` | `s.len() % 2 != 0` → `!s.len().is_multiple_of(2)` |
| 9 | `collapsible_if` | `ecat-registry-etcd/src/lib.rs:92` | verschachteltes `if let` zusammengeführt |
| 10 | `collapsible_if` | `ecat-data-clickhouse/src/lib.rs:56` | verschachteltes `if let` zusammengeführt |
| 11 | `type_complexity` | `ecat-data-memcached/src/lib.rs:9` | Typ-Alias `CacheEntry` ergänzt |

**Endergebnis**: `cargo build` null Warnings, `cargo clippy --all-targets` null Warnings, `cargo test` vollständig bestanden (0 Fehler).

### 12 ─ Daten-Backends mit vollständiger Konfigurationsdatei-Unterstützung (Cargo + lib.rs)

Für 12 Daten-Backend-Crates wurden `Config`-Strukturen (`#[derive(Deserialize)]`) und `from_config()`-Konstruktoren ergänzt, die das Laden von Verbindungsinformationen aus JSON/YAML-Konfigurationsdateien ohne Hardcoding unterstützen.

| Crate | Config-Struktur | Felder |
|-------|--------------|------|
| `ecat-data-redis` | `RedisConfig` | `url` |
| `ecat-data-sqlx` | `SqlxConfig` | `url` |
| `ecat-data-clickhouse` | `ClickhouseConfig` | `base_url`, `database` (Standard "default") |
| `ecat-data-questdb` | `QuestdbConfig` | `base_url` |
| `ecat-data-elasticsearch` | `ElasticsearchConfig` | `base_url` |
| `ecat-data-opensearch` | `OpenSearchConfig` | `base_url` |
| `ecat-data-influxdb` | `InfluxConfig` | `base_url`, `org`, `bucket`, `token` |
| `ecat-data-memcached` | `MemcachedConfig` | (leer — In-Memory-Implementierung) |
| `ecat-data-neo4j` | `Neo4jConfig` | `base_url`, `username`, `password` |
| `ecat-data-nebulagraph` | `NebulaGraphConfig` | `base_url`, `space` |
| `ecat-data-arangodb` | `ArangoConfig` | `base_url`, `db`, `username`, `password` |
| `ecat-data-iotdb` | `IotdbConfig` | `base_url`, `username`, `password` |

**Verwendungsbeispiel**:
```rust
// Aus YAML-Konfigurationsdatei laden
let cfg: ClickhouseConfig = serde_json::from_str(r#"{"base_url":"http://localhost:8123"}"#)?;
let client = ClickhouseClient::from_config(cfg);
```

### 13 ─ Optionale Authentifizierungsunterstützung für HTTP-Backends (5 Crates)

Für 5 reine HTTP-Backends wurden optionale `username`-/`password`-Felder und `with_auth()`-Konstruktoren ergänzt. Alle als `Option<String>` (`#[serde(default)]`), ohne Konfiguration keine Authentifizierung.

| Crate | neue Config-Felder | neuer Konstruktor |
|-------|-----------------|-------------|
| `ecat-data-elasticsearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-opensearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-clickhouse` | `username?`, `password?` | `with_auth()` |
| `ecat-data-questdb` | `username?`, `password?` | `with_auth()` |
| `ecat-data-nebulagraph` | `username?`, `password?` | `with_auth()` |

Alle HTTP-Anfragen hängen über die Hilfsmethode `apply_auth()` automatisch Basic Auth an (nur wenn beide Werte nicht None sind).

### 14 ─ Optionale Authentifizierungsfelder für Redis / RDBMS / Memcached (3 Crates)

| Crate | neue Config-Felder | neuer Konstruktor | Auth-Methode |
|-------|-----------------|-------------|----------|
| `ecat-data-redis` | `password?` | `connect_with_password()` | Passwort in URL eingebettet |
| `ecat-data-sqlx` | `username?`, `password?` | `connect_with_auth()` | Auth in URL eingebettet |
| `ecat-data-memcached` | `username?`, `password?` | `with_auth()` | Felder reserviert (In-Memory-Implementierung) |

Sqlx deckt vier RDBMS ab: SQLite / PostgreSQL / MySQL / TiDB. Auth-Felder werden über `replacen("://", "://user:pass@")` in die Verbindungs-URL eingebettet, nur wenn die URL kein `@` enthält.

### 15 ─ TLS-Zertifikat-Authentifizierung + ecat-tls Crate (alle 12 Backends)

Neues Crate `ecat-tls`, das Folgendes bietet:
- `TlsClientConfig` — optionale TLS-Konfiguration (ca_cert, client_cert, client_key, skip_verify)
- `generate_ca()` — Erzeugung selbstsignierter CA-Zertifikate
- `generate_server_cert()` — Erzeugung von Serverzertifikaten
- `generate_client_cert()` — Erzeugung von Clientzertifikaten (mTLS)

Alle 12 Daten-Backend-Configs erhielten das Feld `#[serde(default)] tls: Option<TlsClientConfig>`.

| Backend-Typ | TLS-Methode |
|----------|----------|
| 9 HTTP-Backends | `tls.build_reqwest_client()` erzeugt TLS-reqwest-Client |
| Redis | URL-Scheme-Wechsel `redis://` → `rediss://` |
| Sqlx | Feld reserviert (TLS über URL-Parameter `?sslmode=require`) |
| Memcached | Feld reserviert (für Netzimplementierung vorgesehen) |

---

## 1. Überblick

| Punkt | Status | Details |
|------|------|------|
| `cargo build` | ✅ bestanden | 3 Compiler-Warnings, 19,85s |
| `cargo test` | ✅ bestanden | ~137 Unit-Tests alle bestanden, 0 Fehler, 1 ignored |
| `cargo clippy` | ⚠️ mit Warnings | 3 Crates mit insgesamt 5 Lint-Warnings |
| `cargo fmt` | ✅ bestanden | keine Formatierungsprobleme |
| `cargo audit` | ❌ nicht installiert | bekannte CVEs nicht prüfbar |

---

## 2. Compiler-Warnings (zu beheben)

### 2.1 ecat-versioning (3 Warnings)

**Datei**: `ecat-versioning/src/lib.rs`

| # | Warning | Zeile | Schweregrad |
|---|---------|------|----------|
| 1 | `unused import: axum::routing::get` | 3 | niedrig |
| 2 | `unused variable: version` | 61 | niedrig |
| 3 | `function extract_version is never used` | 68 | niedrig |

**Empfehlung**: Unbenutzten Import entfernen, `version` in `_version` umbenennen, `extract_version` als `pub` deklarieren oder mit `#[allow(dead_code)]` markieren.

### 2.2 ecat-data-questdb (1 Clippy-Warning)

**Datei**: `ecat-data-questdb/src/lib.rs:39`

```rust
// aktuell:
.query(&[("query", sql), ("count", &"true".to_string())])

// sollte:
.query(&[("query", sql), ("count", &"true")])
```

### 2.3 ecat-client (1 Clippy-Warning)

**Datei**: `ecat-client/src/lib.rs:249`

`GrpcClientBuilder` implementiert `Default` manuell, direkt durch `#[derive(Default)]` ersetzbar.

---

## 3. Clippy-Lint-Warnings Übersicht

| Crate | Warning | Typ |
|-------|---------|------|
| ecat-versioning | `useless_format!` — nutzt `"/api".to_string()` | Performance |
| ecat-versioning | unused import / dead code | Aufräumen |
| ecat-data-questdb | `unnecessary_to_owned` | Performance |
| ecat-client | `derivable_impls` — derive Default nutzen | Vereinfachung |

---

## 4. Testabdeckungsanalyse

### 4.1 Statistik

| Kennzahl | Wert |
|------|------|
| Unit-Tests gesamt | ~137 |
| Fehler | 0 |
| Ignored | 1 |
| Crates mit Tests | ~24 / 48 |
| **Crates mit 0 Tests** | **~24 / 48 (50 %)** |

### 4.2 Crates mit fehlenden Tests (0 oder nur Konstruktortests)

Die folgenden Crates haben eine schwache Testabdeckung:

- ecat-data-arangodb, ecat-data-clickhouse, ecat-data-elasticsearch
- ecat-data-influxdb, ecat-data-iotdb, ecat-data-nebulagraph
- ecat-data-neo4j, ecat-data-opensearch, ecat-data-questdb
- ecat-data-redis, ecat-data-sqlx, ecat-data-memcached
- ecat-mq, ecat-mq-kafka, ecat-graphql, ecat-openapi
- ecat-transport, ecat-transport-grpc, ecat-transport-http
- ecat-transport-ws, ecat-tracing, ecat-logging
- ecat-middleware, ecat-registry-consul, ecat-registry-etcd

### 4.3 Doc-Tests

Alle **48 Crates haben 0 Doc-Tests**. Keine `/// ````rust`-Dokumentbeispiele im Code.

---

## 5. Abhängigkeitsprobleme

### 5.1 ⚠️ yaml_serde vs serde_yaml (mittleres Risiko)

**Datei**: `ecat-config/Cargo.toml:9`

```toml
yaml_serde = "0.10"
```

Die Standard-YAML-Bibliothek im Rust-Ökosystem ist `serde_yaml` (neueste Version `0.9.34+`), während `yaml_serde` ein **anderes und weniger gepflegtes Crate** ist.

**Empfehlung**: Prüfen, ob `yaml_serde` die beabsichtigte Abhängigkeit ist. Falls `serde_yaml` gemeint war, austauschen.

### 5.2 cargo-audit fehlt

`cargo audit` ist nicht installiert. Empfehlung: `cargo install cargo-audit` und in CI aufnehmen.

### 5.3 description-Feld fehlt

`[workspace.package]` enthält kein `description`, auch keine Sub-Crate definiert ein description.

---

## 6. Codequalitätsprobleme

### 6.1 unwrap/expect im Produktionscode

| Datei | Zeile | Aufruf | Risiko |
|------|------|------|------|
| `ecat-client/src/lib.rs` | 28 | `.expect("StaticResolver poisoned")` | niedrig — vertretbar |
| `ecat/src/signal.rs` | 8 | `.expect("failed to install SIGTERM handler")` | mittel — panic beim Start |
| `ecat-protos/build.rs` | 5 | `.unwrap()` | niedrig — Build-Skript |

### 6.2 extract_version in ecat-versioning

Die Funktion `extract_version` (Zeile 68) extrahiert die Versionsnummer aus dem Accept-Header, wird aber von `build_header_router()` nicht aufgerufen.

### 6.3 Fehlerbehandlung in ecat-data-questdb

```rust
// Zeile 30: Netzwerk-Response-Body wird mit unwrap_or_default gelesen
Err(RdbmsError::Database(resp.text().await.unwrap_or_default()))
```

Bei Fehlern von `resp.text()` wird die Fehlermeldung still verschluckt. Empfehlung: `unwrap_or_else(|e| format!("questdb parse: {e}"))`.

---

## 7. Architekturbewertung

### Stärken

- klare Verantwortungstrennung über 48 Crates
- workspace-einheitliche Version `version.workspace = true`
- schlanke Abhängigkeiten, keine großen Frameworks
- keine TODO/FIXME/HACK

### Verbesserungsbedarf

| Problem | Priorität |
|------|--------|
| 50 % der Crates ohne Tests | hoch |
| yaml_serde vs serde_yaml Verwechslung | mittel |
| cargo-audit fehlt | mittel |
| Toter Code in ecat-versioning | niedrig |
| keine Doc-Tests | niedrig |

---

## 8. Sicherheitsübersicht

| Prüfpunkt | Ergebnis |
|--------|------|
| Hartcodierte Schlüssel | nicht gefunden |
| .env-Datei-Leak | nicht gefunden |
| Gefährliche unwrap (Produktionscode) | 2 Stellen (signal.rs, client.rs) |
| CVE-Scan | nicht ausgeführt (cargo-audit muss installiert werden) |

---

## 9. Aktionsplan

### P0 — sofort beheben
1. die 3 Compiler-Warnings von ecat-versioning bereinigen
2. ecat-data-questdb Clippy beheben
3. ecat-client derivable_impls beheben

### P1 — kurzfristig
4. `cargo-audit` installieren und Abhängigkeitslücken scannen
5. Auswahl `yaml_serde` vs `serde_yaml` bestätigen
6. Doc-Tests für Kern-Crates ergänzen

### P2 — mittelfristig
7. Tests für transport/data/security-Crates ergänzen
8. `description`-Feld für alle Crates ergänzen
9. `extract_version` integrieren oder entfernen

### P3 — langfristig
10. CI aufbauen: build → test → clippy → audit → coverage

---

*Bericht erstellt am 2026-08-01. Toolchain: cargo 1.92.0, rustc 1.92.0, clippy 1.92.0*
