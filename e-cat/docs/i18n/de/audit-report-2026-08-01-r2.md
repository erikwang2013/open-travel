# e-cat Framework-Auditbericht R2 — 2026-08-01

**Version**: 1.0.5
**Umfang**: alle 18 Sub-Crates
**Fazit**: `cargo check` / `cargo clippy --all-features` / `cargo test` alle bestanden, 70 tests ✅

---

## 1. Rückblick auf die letzten Fixes (16/16 behoben)

Alle im letzten Audit (R1) gefundenen Probleme sind behoben: SecurityLayer blockt Angriffe, ProtoCodec-prost-Unterstützung, sauberes Server-Shutdown, JoinHandle-Sammlung, Transaction-Implementierung, Registration-Drop-Sicherheitserkennung, verbesserte Spaltentyp-Zuordnung, CLI-new-Dateigenerierung, Versions-/Editions-Vereinheitlichung, FileSource-Fehlerbehandlung, Context-Metadatenmethoden, discover-Arc-Optimierung, query-columns-Arc-Optimierung, neuer RateLimitLayer.

---

## 2. Neue Probleme dieser Runde

### 2.1 [Schwerwiegend] Vom CLI-`new` generierter Template-Code kompiliert nicht

- **Datei**: `ecat-cli/src/main.rs:79-97`
- **Problem**: das generierte `Cargo.toml` verwendet `workspace = true`-Abhängigkeitsverweise und den relativen Pfad `path = "../ecat"`, aber das von `ecat new myapp` erstellte eigenständige Projekt liegt nicht im e-cat-Workspace — alle diese Verweise scheitern beim Auflösen
- **Auswirkung**: von `ecat new` erstellte Projekte kompilieren überhaupt nicht
- **Fix**: das Template sollte versionierte echte Abhängigkeiten verwenden statt Workspace-Verweisen

```toml
# aktuell (falsch):
tokio.workspace = true           # Projekt nicht im Workspace, Fehler
ecat = { path = "../ecat" }      # relativer Pfad ungültig

# sollte:
tokio = { version = "1", features = ["full"] }
ecat = "1.0.5"
```

### 2.2 [Schwerwiegend] ecat-data-sqlx `transaction()` verwirft das echte Datenbank-Transaktionshandle

- **Datei**: `ecat-data-sqlx/src/lib.rs:100-106`
- **Problem**: `pool.begin()` liefert das echte Datenbank-Transaktionshandle `Transaction<'_, DB>`, aber der Code bindet es als `_tx` und verwirft es sofort. Beim Drop von `_tx` wird die Datenbanktransaktion automatisch zurückgerollt. Die zurückgegebene `ecat_data::Transaction` ist eine Hülse, deren `commit()/rollback()`-Methoden wirkungslos sind
- **Auswirkung**: aller Code, der `transaction()` verwendet, läuft ohne Transaktionsschutz; Datenkonsistenz ist nicht gewährleistet
- **Fix**: die `ecat_data::Transaction`-Struktur muss neu gestaltet werden, sodass sie das echte Datenbank-Transaktionshandle hält

### 2.3 [Mittel] SecurityLayer scannt den Request-Body nicht

- **Datei**: `ecat-security/src/lib.rs:117-127`
- **Problem**: `call()` scannt nur URI und HTTP-Header, prüft den Request-Body überhaupt nicht. Angreifer können SQL-Injection-/XSS-Payloads problemlos im POST-Body verstecken und die Erkennung umgehen
- **Auswirkung**: verringert die effektive Abdeckung der Angriffserkennung erheblich
- **Fix**: Body-Scan-Fähigkeit hinzufügen oder eine öffentliche Methode `scan_body()` bereitstellen, die der Aufrufer nach dem Lesen des Bodys nutzen kann

### 2.4 [Mittel] RateLimitLayer verwendet synchrones Mutex + keine Ablaufbereinigung

- **Datei**: `ecat-middleware/src/ratelimit.rs:10-38`
- **Problem 1**: `std::sync::Mutex` im async-Kontext — bei Lock-Konkurrenz wird der gesamte tokio-Worker-Thread blockiert
- **Problem 2**: `buckets: HashMap<String, (u32, Instant)>` bereinigt abgelaufene Keys nie; lang laufende Server wachsen unbegrenzt im Speicher (jede neue IP/jeder neue Key belegt dauerhaft Speicher)
- **Auswirkung**: Leistungseinbußen bei hoher Nebenläufigkeit, Speicherleck bei langem Betrieb
- **Fix**: `tokio::sync::Mutex` verwenden und in `allow()` abgelaufene Einträge regelmäßig bereinigen

### 2.5 [Mittel] ecat-data-sqlx: rohes SQL ohne parametrisierte API

- **Datei**: `ecat-data-sqlx/src/lib.rs:24-29, 32-36`
- **Problem**: `execute(&self, sql: &str)` und `query(&self, sql: &str)` akzeptieren nur rohe SQL-Strings, auf Trait-Ebene gibt es keine Parameterbindungsmethode. Wenn Aufrufer Benutzereingaben in SQL einbauen, droht SQL-Injection
- **Auswirkung**: das Trait selbst legt keine Sicherheitslücke offen, aber das Fehlen einer parametrisierten API verleitet Aufrufer zu unsicherem Code
- **Empfehlung**: dem `RdbmsClient`-Trait `execute_with`- und `query_with`-Methoden mit Parameterbindung hinzufügen

### 2.6 [Niedrig] Arc::clone in query() liegt weiterhin im Closure

- **Datei**: `ecat-data-sqlx/src/lib.rs:50-53`
- **Problem**: `let cols = std::sync::Arc::clone(&columns)` läuft im Closure von `rows.iter().map()`. Obwohl Arc::clone leichtgewichtig ist (nur atomare Referenzzählung), kann es vor das Closure gezogen werden, um eine Atomoperation pro Zeile zu sparen
- **Empfehlung**: einmal vor `iter()` klonen, im Closure den Klon erfassen

### 2.7 [Niedrig] Trait-Impl von ProtoCodec inkonsistent mit neuer API

- **Datei**: `ecat-encoding/src/proto.rs`
- **Problem**: `encode/decode` des `Codec`-Traits liefern weiterhin nur Fehler; die neuen `encode_message/decode_message` sind der korrekte Weg, aber die Methodennamen passen nicht zum Trait. Nutzer könnten zuerst `codec.encode()` versuchen und sich wundern, warum es fehlschlägt
- **Empfehlung**: in Dokumentation/Kommentaren erklären: für proto-Typen `encode_message/decode_message` statt der Codec-Trait-Methoden verwenden

---

## 3. Aktueller Status-Überblick

| Dimension | Status |
|------|------|
| `cargo check` | ✅ null Warnungen |
| `cargo clippy --all-features` | ✅ null Warnungen |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 bestanden |
| Versionsvereinheitlichung | ✅ 1.0.5 |
| Editions-Vereinheitlichung | ✅ 2024 |

### Testverteilung

| Crate | Tests | Beschreibung |
|-------|-------|------|
| ecat | 4 | ✅ |
| ecat-config | 9 | ✅ |
| ecat-encoding | 15 | ✅ |
| ecat-errors | 4 | ✅ |
| ecat-logging | 1 | ✅ |
| ecat-metadata | 9 | ✅ |
| ecat-metrics | 2 | ✅ |
| ecat-middleware | 4 | ✅ (inkl. RateLimitLayer) |
| ecat-registry | 5 | ✅ |
| ecat-security | 6 | ✅ |
| ecat-transport | 11 | ✅ |
| ecat-data | 0 | — (reine Trait-Definitionen) |
| ecat-data-sqlx | 0 | ⚠️ keine DB-Integrationstests |
| ecat-protos | 0 | — (generierter Code) |
| ecat-transport-grpc | 0 | ⚠️ |
| ecat-transport-http | 0 | ⚠️ |
| ecat-cli | 0 | ⚠️ |

---

## 4. Problemprioritäten

| # | Schweregrad | Problem | Datei | Nutzerauswirkung |
|---|--------|------|------|----------|
| 1 | 🔴 | CLI-`new`-Template erzeugt nicht kompilierbaren Code | `ecat-cli/src/main.rs:79` | erster Befehl neuer Nutzer schlägt fehl |
| 2 | 🔴 | transaction() verwirft echtes DB-Transaktionshandle | `ecat-data-sqlx/src/lib.rs:100` | Datenkonsistenz ungeschützt |
| 3 | 🟠 | SecurityLayer scannt Body nicht | `ecat-security/src/lib.rs:117` | Angreifer können Erkennung umgehen |
| 4 | 🟠 | RateLimitLayer std Mutex + Speicherleck | `ecat-middleware/src/ratelimit.rs:10,25` | Nebenläufigkeitsleistung + OOM |
| 5 | 🟠 | rohes SQL ohne parametrisierte API | `ecat-data-sqlx/src/lib.rs:24` | SQL-Injection-Risiko |
| 6 | 🟡 | Arc-clone-Position in query() | `ecat-data-sqlx/src/lib.rs:53` | winzige Leistungsoptimierung |
| 7 | 🟡 | ProtoCodec-API inkonsistent | `ecat-encoding/src/proto.rs` | Nutzerverwirrung |

---

## 6. Fix-Protokoll (2026-08-01 R2)

| # | Problem | Fix-Methode | Status |
|---|------|----------|------|
| 1 | CLI-new-Template nicht kompilierbar | versionierte Abhängigkeiten (`ecat = "1.0"`, `tokio = "1"` usw.) | ✅ |
| 2 | transaction() verwirft DB-Transaktion | `Transaction::with_inner()` hält echtes Handle, sqlx übergibt über `Box<dyn Any>` | ✅ |
| 3 | SecurityLayer scannt Body nicht | neue öffentliche Methode `scan_body(&[u8])` | ✅ |
| 4 | RateLimitLayer Mutex + Leck | `tokio::sync::Mutex` + Bereinigung abgelaufener Einträge alle 100 Keys | ✅ |
| 5 | rohes SQL ohne parametrisierte API | `RdbmsClient` um parametrisierte Methoden `execute_with`/`query_with` erweitert | ✅ |
| 6 | Arc-clone-Position in query() | `Arc::clone` vor `iter()` gezogen, alle Zeilen teilen die Referenz | ✅ |
| 7 | ProtoCodec-API inkonsistent | Modul-Doku + Struct-Doku erklärt die Verwendung | ✅ |

### Endstatus

| Prüfpunkt | Ergebnis |
|--------|------|
| `cargo check` | ✅ null Fehler / null Warnungen |
| `cargo clippy --all-features` | ✅ null Warnungen |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 bestanden |
| Version | 1.0.5 (alle einheitlich über Workspace-Vererbung) |
| Edition | 2024 |
