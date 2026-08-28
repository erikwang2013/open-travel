# e-cat Tiefenprüfungsbericht — 2026-08-01 R6

## Gesamtbewertung

| Dimension | Status | Anmerkung |
|------|------|------|
| Kompilierung | bestanden | 50 Crates, null Fehler |
| Tests | bestanden | alle bestanden, null Fehler |
| Clippy | bestanden | null Warnings (`-D warnings`) |
| unsafe | null | keine unsafe-Blöcke im Codebestand |
| Dateigröße | gut | nur `ecat-auth` (540 Zeilen) überschreitet den empfohlenen Wert von 500 Zeilen |

## Befunde (15 Punkte)

### Sicherheitsrelevant

#### 1. [Schwerwiegend] XOR-„Verschlüsselung" ist keine echte Verschlüsselung
**Datei:** `ecat-config/src/encrypted.rs:45-56`
**Problem:** `decrypt()` nutzt XOR mit sich wiederholendem Schlüssel — das ist Obfuskation, keine Verschlüsselung, und leicht zu brechen. Der Schlüssel wird an jeder Byteposition wiederverwendet, wodurch der Chiffretext sehr anfällig für Frequenzanalyse ist.
**Empfehlung:** Durch AES-256-GCM ersetzen (`aes-gcm`-Crate) oder klar als „Obfuskation" statt „Verschlüsselung" kennzeichnen.

#### 2. [Schwerwiegend] Standardimplementierung von `execute_with`/`query_with` verwirft Parameter still
**Datei:** `ecat-data/src/rdbms.rs:86-103`
**Problem:** Die Standardimplementierung im Trait nimmt Parameter entgegen, ignoriert sie aber (`let _ = params;`) und ruft direkt das ursprüngliche `execute(sql)` auf. Alle Backends außer `ecat-data-sqlx` (ClickHouse, QuestDB) erben dieses Verhalten. Tauscht ein Nutzer das Backend durch eine parametrisierte Methode, werden die Parameter still verworfen — SQL-Injection-Lücke.
**Empfehlung:** Die Standardimplementierung sollte einen „nicht unterstützt"-Fehler zurückgeben, oder jedes Backend muss Parameterbindung korrekt implementieren.

#### 3. [Hoch] Passwörter unverschlüsselt in URLs eingebettet
**Datei:** `ecat-data-sqlx/src/lib.rs:40`, `ecat-data-redis/src/lib.rs:43`
**Problem:** `connect_with_auth()` bettet Zugangsdaten über `replacen("://", "://user:pass@")` direkt in die URL ein. Diese URLs können in Logs, Fehlermeldungen oder Debug-Ausgaben landen.
**Empfehlung:** Native Authentifizierungsmechanismen der jeweiligen Backends nutzen; oder zumindest Benutzername/Passwort vor der Verkettung URL-kodieren.

#### 4. [Mittel] TLS-Konfigurationsfehler führen zu panic
**Datei:** 8 data-*-Crates (ClickHouse, QuestDB, Elasticsearch, OpenSearch, ArangoDB, Neo4j, NebulaGraph, InfluxDB, IoTDB)
**Muster:** `.expect("TLS client build failed")` — alle `from_config()`-Konstruktoren panicken bei fehlerhafter TLS-Konfiguration.
**Empfehlung:** `from_config()` soll `Result` zurückgeben, oder der TLS-Client-Aufbau soll lazy/fehlertolerant erfolgen.

### Funktionale Korrektheit

#### 5. [Hoch] Header-Routing in `ecat-versioning` wirkungslos
**Datei:** `ecat-versioning/src/lib.rs:56-64`
**Problem:** `build_header_router()` verschachtelt alle Versionen unter demselben `/api`-Pfad, filtert aber nicht nach dem Versions-Header. axum registriert alle Versionen auf demselben Pfad — Routenkonflikte und unvorhersehbares Verhalten. Die Funktion `extract_version()` existiert, wird aber nie im Routing verwendet.
**Empfehlung:** axum-Middleware/Layer verwenden, die den Accept-Header prüft und zur richtigen Versionsroute weiterleitet, statt alle Versionen auf einen Pfad zu flachen.

#### 6. [Mittel] Redis-TTL-Abschneidung: Subsekunden-Ablauf wird zu „läuft nie ab"
**Datei:** `ecat-data-redis/src/lib.rs:76-77`
**Problem:** `Duration::as_secs()` schneidet in Richtung Null ab. Eine auf 500 ms gesetzte TTL wird bei `secs == 0` still zu „läuft nie ab" und nimmt den `SET`- statt des `SETEX`-Zweigs.
**Empfehlung:** Für Subsekunden-TTLs mindestens 1 Sekunde setzen, oder `SET ... PX` (Millisekunden) statt `SETEX` verwenden.

#### 7. [Mittel] `StaticResolver::add_service` panickt bei Lock-Konkurrenz
**Datei:** `ecat-client/src/lib.rs:27-29`
**Problem:** Verwendet `try_write()` mit expect — panickt, wenn irgendein anderer Schreiblock-Halter existiert. Das Builder-Muster macht das schwer auslösbar, aber in nebenläufigem Code ist es eine Zeitbombe.
**Empfehlung:** `blocking_write()` verwenden (im synchronen Kontext) oder auf `&mut self` umstellen, um den Lock-Bedarf zu vermeiden.

### Codequalität

#### 8. [Mittel] `std::sync::Mutex` im asynchronen Kontext
**Datei:** `ecat-data-memcached/src/lib.rs:7,24`
**Problem:** In async-Trait-Implementierungen wird `std::sync::Mutex` verwendet. Obwohl die Lock-Haltezeit extrem kurz ist (nur HashMap-Operationen), kann es unter hoher Konkurrenz theoretisch die async-Runtime blockieren.
**Empfehlung:** Für diesen spezifischen In-Memory-Cache-Anwendungsfall ist `std::sync::Mutex` wegen des extrem kurzen kritischen Abschnitts ohne `.await`-Punkte tatsächlich akzeptabel. Falls künftig I/O innerhalb des Locks nötig wird, auf `tokio::sync::Mutex` wechseln.

#### 9. [Niedrig] Handgeschriebene base64-Implementierung
**Datei:** `ecat-registry-etcd/src/lib.rs:148-193`
**Problem:** ~45 Zeilen handgeschriebener base64-Codec mit möglichen Randfall-Bugs. Im Rust-Ökosystem gibt es gründlich geprüfte Alternativen wie das `base64`-Crate.
**Empfehlung:** Durch das `base64`-Crate ersetzen — reduziert Wartungslast und potenzielle Bugs.

#### 10. [Niedrig] `RandomBalancer` ist nicht zufällig
**Datei:** `ecat-client/src/lib.rs:91-105`
**Problem:** Nutzt einen Hash von `Instant::now()` als Zufallsquelle. Gleichzeitige Aufrufe innerhalb derselben Instanz erhalten dieselbe „zufällige" Auswahl. `checked_add(0)` ist eine überflüssige Operation.
**Empfehlung:** Das `rand`-Crate verwenden oder zumindest `std::collections::hash_map::RandomState`.

#### 11. [Niedrig] Unnötiges `Arc<Vec<String>>` in `ecat-data-sqlx`
**Datei:** `ecat-data-sqlx/src/lib.rs:79-87, 197-203`
**Problem:** Spaltennamen sind in `Arc<Vec<String>>` verpackt, aber jeder `Row`-Konstruktor klont die gesamte Spaltenliste (`(*cols).clone()`). Der `Arc` wird nur einmal während der Iteration genutzt — `Rc` oder direktes `clone()` genügt.
**Empfehlung:** In `query()` und `query_with()` das `Arc<Vec<String>>` durch ein gewöhnliches `Vec<String>` ersetzen. Die Kosten des Klonens pro Zeile sind identisch mit Arc-Deref + Klonen.

### Design/Architektur

#### 12. [Info] QuestDB nutzt GET + Query-Parameter
**Datei:** `ecat-data-questdb/src/lib.rs:76, 91`
**Problem:** SQL wird über GET-Query-Parameter gesendet und unterliegt URL-Längenbegrenzungen (üblicherweise ~2000-8000 Zeichen). Große Abfragen werden abgeschnitten.
**Empfehlung:** Auf POST + Body umstellen, oder GET für einfache Abfragen behalten und POST für komplexe nutzen.

#### 13. [Info] `#[allow(dead_code)]` verstreut
**Datei:** `ecat-registry-consul/src/lib.rs:225`, `ecat-data-memcached/src/lib.rs:25-28`, `ecat-auth/src/lib.rs:52`
**Problem:** username/password-Felder werden im Speicher gehalten, aber als dead_code markiert (im In-Memory-memcached unnötig; die RSA-Variante in auth ist noch nicht implementiert).
**Empfehlung:** Entweder die fehlenden Funktionspfade implementieren, die Felder entfernen oder dokumentieren, warum sie behalten werden.

#### 14. [Info] Einige HTTP-Clients ohne Content-Type-Header
**Datei:** `ecat-data-influxdb/src/lib.rs:96-103`, `ecat-data-clickhouse/src/lib.rs:87-89`
**Problem:** Einige POST-Anfragen setzen keinen `Content-Type`-Header und verlassen sich auf die automatische Erkennung des Servers.
**Empfehlung:** Immer einen expliziten Content-Type setzen, um Kompatibilität zu gewährleisten.

#### 15. [Info] `ecat-auth` über 500 Zeilen
**Datei:** `ecat-auth/src/lib.rs` (540 Zeilen)
**Problem:** CLAUDE.md verlangt Dateien unter 500 Zeilen. Das auth-Crate ist die einzige Datei, die diese Grenze überschreitet.
**Empfehlung:** JWT-Validierungslogik nach `ecat-auth/src/jwt.rs` auslagern oder nach Funktionsbereichen aufteilen.

## Optimierungsmöglichkeiten (keine Bugs)

| # | Stelle | Empfehlung |
|---|------|------|
| O1 | alle data-*-Crates | Das in allen `from_config()` wiederholte TLS-Client-Aufbaumuster lässt sich in ein gemeinsames Makro oder eine Funktion extrahieren |
| O2 | `ecat-data-sqlx` | Die Zeilentyp-Konvertierungslogik in `query()` und `query_with()` (117 doppelte Zeilen) lässt sich in eine Hilfsfunktion auslagern |
| O3 | `ecat-client` | `HttpClient::get()` und `post()` teilen dieselbe „resolve → pick → build URL"-Pipeline — extrahierbar |
| O4 | `ecat-data` | Die benutzerdefinierten Fehlertypen aller 5 Traits (Rdbms/Cache/Graph/Search/Tsdb) lassen sich in einer einzigen `DataError`-Enum vereinheitlichen |
| O5 | `ecat-data-redis` | `self.conn.clone()` in jeder Methode ist unnötig — `MultiplexedConnection` ist für gemeinsame Nutzung als `Clone` konzipiert |

## Kennzahlenübersicht

| Kennzahl | Wert |
|------|------|
| Crates gesamt | 50 |
| Rust-Quelldateien gesamt (Zeilen) | 7.968 |
| `expect()` außerhalb von Tests | 12 |
| `unwrap()` außerhalb von Tests | 0 |
| `unsafe`-Blöcke | 0 |
| `panic!` außerhalb von Tests | 0 |
| `#[allow(dead_code)]` | 4 |
| TODO/FIXME/HACK | 0 |
| std-Mutex im async-Code | 1 (memcached) |

## Fazit

Der Codebestand ist in gutem Zustand — Kompilierung, Tests und Clippy laufen alle durch, kein unsafe-Code, keine panic-Makros. Die beiden kritischsten Probleme sind **XOR-„Verschlüsselung"** (vorgetäuschte Sicherheit) und **parametrisierte Abfragen, deren Standardimplementierung Parameter still verwirft** (Sicherheitslücke). Auch die Header-Routing-Funktion ist vollständig unbrauchbar. Die übrigen Probleme sind relativ klein und liegen im Bereich der Wartbarkeitsoptimierung.

**Empfohlene Prioritätsreihenfolge:**
1. `execute_with`/`query_with` Standardimplementierung → Fehler zurückgeben statt Parameter still verwerfen
2. XOR-Verschlüsselung → echte AEAD-Verschlüsselung oder Umbenennung in „Obfuskation"
3. Header-Versionsrouting → tatsächliches Header-Routing implementieren
4. `from_config()` → `Result` zurückgeben statt expect-panic
5. Redis-TTL-Abschneidung → Subsekunden-TTLs mindestens 1 Sekunde

## Fix-Status (R6 → R6.1)

| # | Problem | Status | Änderung |
|---|------|------|------|
| 1 | XOR-„Verschlüsselung" | behoben | `EncryptedSource` → `ObfuscatedSource`, `decrypt` → `deobfuscate`, Präfix `enc:` → `obfs:`, Dokumentation ergänzt, dass es Obfuskation und keine Verschlüsselung ist |
| 2 | `execute_with`/`query_with` verwirft Parameter still | behoben | Standardimplementierung gibt Fehler `"parameterized ... not supported by this backend"` zurück |
| 3 | Passwörter unverschlüsselt in URL | behoben | `percent_encode()` kodiert Zugangsdaten in `connect_with_auth` |
| 4 | TLS-`expect()`-panic | behoben | `from_config()` in 9 Crates gibt `Result` zurück, `RdbmsError` erhält Variante `Config` |
| 5 | Header-Routing wirkungslos | behoben | Version-Validierung per `from_fn_with_state`-Middleware, neuer Test `header_versioned_router_builds` |
| 6 | Redis-TTL-Abschneidung | behoben | `set_ex` → `pset_ex`, Millisekunden-Präzision verhindert Abschneidung von Subsekunden-TTLs auf „läuft nie ab" |
| 7 | `StaticResolver` Lock-Konkurrenz-panic | behoben | `try_write()` → `blocking_write()` |
| 8 | `RandomBalancer` nicht zufällig | behoben | `RandomState::new().build_hasher()` ersetzt den `Instant::now()`-Hash |
| 9 | `std::sync::Mutex` im async-Kontext | behoben | durch `tokio::sync::Mutex` ersetzt |
| 10 | Handgeschriebenes base64 | behoben | durch `base64`-Crate 0.22 ersetzt |
| 11 | `Arc<Vec<String>>`-Overhead | behoben | durch gewöhnliches `Vec<String>` ersetzt, unnötige Arc-Umhüllung entfernt |
| 12 | QuestDB sendet SQL per GET | behoben | auf POST + Body umgestellt, Content-Type-Header ergänzt |
| 13 | `#[allow(dead_code)]` | behoben | memcached-Felder mit `_`-Präfix; consul-Felder mit `_`-Präfix und allow entfernt; in auth `Rsa` → `RsaReserved` |
| 14 | Content-Type fehlt | behoben | expliziter Content-Type für InfluxDB-, ClickHouse-, IoTDB-Anfragen |
| 15 | `ecat-auth` über 500 Zeilen | behoben | aufgeteilt in `claims.rs`(31) + `jwt.rs`(139) + `apikey.rs`(96) + `oauth2.rs`(173) + `helpers.rs`(28) + `lib.rs`(98) |

### Betroffene Crates

| Crate | Änderungstyp |
|-------|----------|
| `ecat-data` | Trait-Standardimplementierung, `RdbmsError::Config`-Variante |
| `ecat-config` | `EncryptedSource` → `ObfuscatedSource` |
| `ecat-versioning` | Header-Routing-Middleware implementiert |
| `ecat-data-redis` | TTL-Millisekunden-Präzision, URL-Kodierung der Zugangsdaten |
| `ecat-data-sqlx` | URL-Kodierung der Zugangsdaten, Arc-Overhead entfernt |
| `ecat-data-clickhouse` | `from_config` → `Result`, Content-Type-Header |
| `ecat-data-questdb` | `from_config` → `Result`, GET → POST |
| `ecat-data-elasticsearch` | `from_config` → `Result` |
| `ecat-data-opensearch` | `from_config` → `Result` |
| `ecat-data-arangodb` | `from_config` → `Result` |
| `ecat-data-neo4j` | `from_config` → `Result` |
| `ecat-data-nebulagraph` | `from_config` → `Result` |
| `ecat-data-influxdb` | `from_config` → `Result`, Content-Type-Header |
| `ecat-data-iotdb` | `from_config` → `Result`, Content-Type-Header |
| `ecat-data-memcached` | `std::sync::Mutex` → `tokio::sync::Mutex`, dead_code-Bereinigung |
| `ecat-client` | `StaticResolver`-, `RandomBalancer`-Fixes |
| `ecat-registry-etcd` | base64 durch Crate ersetzt |
| `ecat-registry-consul` | dead_code-Bereinigung |
| `ecat-auth` | in 6 Module aufgeteilt, dead_code-Bereinigung |

### Abschließende Verifikation (R6.2)

| Dimension | Status |
|------|------|
| Build | bestanden, null Fehler null Warnings |
| Test | alle bestanden, null Fehler |
| Clippy (`-D warnings`) | bestanden, null Warnings |
| Dateigröße | alle ≤ 300 Zeilen |
