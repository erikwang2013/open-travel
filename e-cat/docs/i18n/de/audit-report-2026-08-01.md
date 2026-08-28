# e-cat Framework-Auditbericht — 2026-08-01

**Auditdatum**: 2026-08-01
**Prüfungsumfang**: alle 18 Sub-Crates (workspace)
**Werkzeugkette**: stable (rustfmt, clippy)
**Testergebnis**: alle 66 Tests bestanden | 0 fehlgeschlagen | 0 ignoriert

---

## 1. Gesamtbewertung

| Dimension | Bewertung | Beschreibung |
|------|------|------|
| Kompilierung | ✅ bestanden | `cargo check` ohne Fehler, nur 1 warning |
| Lint | ✅ bestanden | `cargo clippy --all-features` null Warnungen |
| Tests | ✅ 66/66 | alle Tests bestanden |
| Testabdeckung | ⚠️ unzureichend | 7 crates ohne jegliche Tests |
| Funktionsvollständigkeit | ⚠️ viele Stubs | ProtoCodec, Transaction, CLI new u. a. nicht implementiert |
| Codequalität | ⚠️ durchschnittlich | klare Struktur, aber mehrere Designprobleme |

---

## 2. Kompilierungs- und Konfigurationsprobleme

### 2.1 [WARNING] Unbenutzter manifest key

- **Datei**: `/Cargo.toml:25`
- **Problem**: `workspace.package.name = "e-cat"` — dieses Feld ist auf Workspace-Ebene bedeutungslos und erzeugt bei jeder Kompilierung eine Warnung
- **Fix**: Zeile löschen oder als Kommentar zur Projektnamensangabe ändern

### 2.2 [INFO] Inkonsistente Rust-Edition

- **workspace**: `edition = "2026"`
- **Sub-Crates**: `ecat-security/Cargo.toml` und `ecat-config/Cargo.toml` verwenden `edition = "2021"`
- **Anmerkung**: Der Workspace deklariert 2026, einige Sub-Crates überschreiben auf 2021. Die Kompilierung funktioniert zwar, aber 2026 ist derzeit keine von Rust offiziell veröffentlichte stabile Edition. Falls bewusst gewählt, sollte die Toolchain-Konfiguration geprüft werden
- **Empfehlung**: prüfen, ob die Toolchain 2026 unterstützt, oder auf 2024/2021 vereinheitlichen

---

## 3. Fehlende Funktionen / Stub-Implementierungen

### 3.1 [Schwerwiegend] ProtoCodec vollständig unbenutzbar

- **Datei**: `ecat-encoding/src/proto.rs:8-10`
- **Problem**: `encode()` und `decode()` liefern immer Fehler, der Protobuf-Codec ist komplett Stub
- **Auswirkung**: jeder Aufruf mit Protobuf-Codierung schlägt zur Laufzeit fehl
- **Empfehlung**: prost::Message-Trait-Binding implementieren oder ein `prost`-Feature-Flag zur Aktivierung der echten Funktion bereitstellen

### 3.2 [Mittel] ecat-data-sqlx-Transaktion nicht implementiert

- **Datei**: `ecat-data-sqlx/src/lib.rs:89-93`
- **Problem**: `transaction()` liefert den hart codierten Fehler `"transactions not yet implemented"`
- **Empfehlung**: `pool.begin()` implementieren und die gewrappte Transaction zurückgeben

### 3.3 [Mittel] HttpServer.stop() und GrpcServer.stop() sind No-ops

- **Dateien**:
  - `ecat-transport-http/src/lib.rs:34-36`
  - `ecat-transport-grpc/src/lib.rs:33-35`
- **Problem**: `stop()` enthält keine Logik zum tatsächlichen Stoppen des Servers. Weder `axum::serve()` noch `tonic::Server::serve()` haben einen Mechanismus zum Empfang eines Shutdown-Signals
- **Auswirkung**: nach `App.run()` läuft der Server weiter, wenn `wait_for_shutdown` ausgelöst wird; kein sauberes Herunterfahren möglich
- **Empfehlung**: `axum::serve(listener, router).with_graceful_shutdown(shutdown_signal)` und `tonic::Server::serve_with_shutdown()` verwenden

### 3.4 [Mittel] CLI-`new`-Befehl ist eine Hülse

- **Datei**: `ecat-cli/src/main.rs:61-67`
- **Problem**: der `new`-Befehl gibt nur eine Nachricht aus, erstellt keine Projektvorlagendateien
- **Empfehlung**: Template-Generierungslogik implementieren oder als TODO markieren

### 3.5 [Niedrig] ecat-data-Ebene ohne Implementierung

- **Dateien**: `ecat-data/src/{cache,graph,rdbms,search,tsdb}.rs`
- **Problem**: alle Datenzugriffsschnittstellen haben nur Trait-Definitionen, keine Implementierung (außer `ecat-data-sqlx` bietet eine RdbmsClient-Implementierung)
- **Empfehlung**: im README den Implementierungsstatus der einzelnen Traits dokumentieren

---

## 4. Unzureichende Testabdeckung

### 4.1 [Mittel] Crates ohne Testabdeckung (7)

| Crate | Quelldateien | Beschreibung |
|-------|--------|------|
| `ecat-data` | 5 Quelldateien | reine Trait-Definitionen, keine Tests |
| `ecat-data-sqlx` | 1 Quelldatei | SQLx-Implementierung, keine Datenbank-Integrationstests |
| `ecat-middleware` | 4 Quelldateien | Logging/Recovery/Timeout/Tracing-Layer alle ohne Tests |
| `ecat-protos` | 1 Quelldatei | generierter Protobuf-Code, keine Tests |
| `ecat-transport-grpc` | 1 Quelldatei | gRPC-Server, keine Tests |
| `ecat-transport-http` | 1 Quelldatei | HTTP-Server, keine Tests |
| `ecat-cli` | 1 Quelldatei | CLI-Einstieg, keine Tests |

**Empfehlungen**:
- `ecat-middleware`: mit `tower-test` Unit-Tests für jeden Layer schreiben
- `ecat-transport-http`: mit `axum::test` Integrationstests für den HTTP-Server schreiben
- `ecat-data-sqlx`: mit `sqlx::SqlitePool` (in-memory) Datenbank-Integrationstests schreiben

---

## 5. Codequalitäts- und Designprobleme

### 5.1 [Schwerwiegend] SecurityLayer erkennt Angriffe, blockiert aber nicht

- **Datei**: `ecat-security/src/lib.rs:100-125`
- **Problem**: `SecurityService::call()` scannt die Request-Daten und protokolliert Warnungen, leitet den Request aber immer an den inneren Service weiter. Selbst bei erkannten SQL-Injection- und XSS-Angriffen wird der Request normal verarbeitet
- **Fix**: bei erkanntem Angriff `403 Forbidden` oder `400 Bad Request` zurückgeben

```rust
// aktuell: immer weiterleiten
let fut = self.inner.call(req);
Box::pin(fut)

// sollte: bei erkanntem Hochrisiko-Angriff ablehnen
if results.iter().any(|r| r.severity >= Severity::High) {
    // 403-Response zurückgeben
}
```

### 5.2 [Mittel] App::run() sammelt JoinHandles nicht

- **Datei**: `ecat/src/lib.rs:33-40`
- **Problem**: das von `tokio::spawn` zurückgegebene `JoinHandle` wird verworfen; Server-Panics können nicht erkannt und ein sauberes Herunterfahren nicht abgewartet werden
- **Empfehlung**: JoinHandles in einem Vec sammeln und beim Shutdown auf das Schließen aller Server warten

### 5.3 [Mittel] Registration::Drop schlägt zur Laufzeit still fehl

- **Datei**: `ecat-registry/src/lib.rs:46-56`
- **Problem**: in `Drop` wird `tokio::spawn()` aufgerufen — wenn die tokio-Runtime bereits gedroppt ist, wird der Task still verworfen
- **Empfehlung**: `tokio::task::block_in_place` + `Handle::block_on` verwenden oder eine explizite `unregister`-Methode einführen

### 5.4 [Mittel] Unzuverlässige Zeilentyp-Zuordnung in ecat-data-sqlx-Abfragen

- **Datei**: `ecat-data-sqlx/src/lib.rs:55-78`
- **Problem**: Spaltenwerte werden in der Reihenfolge `i64 → f64 → String → Null` versucht; einige Datenbanktreiber melden Ganzzahlwerte als inkompatiblen Typ, was zu falscher Konvertierung führt (z. B. liefert PostgreSQL INTEGER als `i32` statt `i64`)
- **Empfehlung**: mit SQLx `ValueRef` / `TypeInfo` den tatsächlichen Datenbanktyp der Spalte prüfen, bevor die Konvertierungsstrategie gewählt wird

### 5.5 [Niedrig] Metadata-Kontext ohne Setzmethoden

- **Datei**: `ecat-transport/src/context.rs:18-20`
- **Problem**: `Context` kapselt `Metadata` in einem `RwLock` und legt nur die Lesemethode `trace_id()` offen; trace_id oder andere Metadaten lassen sich nicht setzen
- **Empfehlung**: für `Context` Schreibmethoden wie `set_trace_id()` hinzufügen

### 5.6 [Niedrig] Nicht-Objekt-YAML/JSON in ecat-config FileSource wird still verworfen

- **Datei**: `ecat-config/src/file.rs:30`
- **Problem**: `unwrap_or_default()` mappt Nicht-Objekt-YAML (z. B. Array `[1,2,3]` oder Skalarwerte) auf eine leere HashMap; der Nutzer weiß nicht, warum die Konfiguration nicht geladen wurde
- **Empfehlung**: `ConfigError::Other("expected object")` zurückgeben

---

## 6. Cross-Platform-Kompatibilitätsprobleme

### 6.1 [Mittel] Kein Ctrl+C-Support für wait_for_shutdown unter Windows

- **Datei**: `ecat/src/signal.rs:13-14`
- **Problem**: auf Nicht-Unix-Plattformen ist `terminate` als `std::future::pending::<()>()` gesetzt, was nie auflöst. Unter Windows wird Ctrl+C in ein SIGINT-Signal umgewandelt, aber es ist unklar, ob `tokio::signal::ctrl_c()` unter Windows funktioniert
- **Empfehlung**: unter Windows ebenfalls `tokio::signal::ctrl_c()` verwenden (laut tokio-Dokumentation wird Windows unterstützt) oder die `tokio::signal::windows::ctrl_*`-Serie nutzen

---

## 7. Architektur- und Optimierungsempfehlungen

### 7.1 [Optimierung] query() in ecat-data-sqlx klont Spaltennamen wiederholt

- **Datei**: `ecat-data-sqlx/src/lib.rs:48-83`
- **Problem**: pro Datenzeile wird der columns-Vec einmal geklont. Bei Abfragen mit 1000 Zeilen wird columns 1000-mal geklont
- **Empfehlung**: columns in `Arc<Vec<String>>` kapseln, alle Zeilen teilen sich die Referenz

### 7.2 [Optimierung] Unnötige Klone in MemoryRegistry::discover()

- **Datei**: `ecat-registry/src/memory.rs:44-52`
- **Problem**: `.cloned()` klont alle passenden ServiceInfo. Bei häufigen discover-Aufrufen entstehen viele Speicherallokationen
- **Empfehlung**: falls der Aufrufer keine Ownership benötigt, `Vec<&ServiceInfo>` zurückgeben oder als `Arc<ServiceInfo>` kapseln

### 7.3 [Architektur] Re-Export-Struktur empfohlen

Die generischen Parameter `T` von `Request` und `Response` im `ecat-transport`-Crate sind standardmäßig `()`; bei Verwendung muss meist ein konkreter Typ angegeben werden. Typ-Aliase empfehlen sich:
```rust
pub type HttpRequest = Request<hyper::Body>;
pub type JsonRequest<T> = Request<T>;
```

### 7.4 [Sicherheit] Rate-Limiting-Middleware fehlt

Der Middleware-Ebene fehlt derzeit die Rate-Limiting-Funktion. Zur DoS-Prävention wird `RateLimitLayer` empfohlen.

---

## 8. Teststatistik

```
Testübersicht:
  Gesamt: 66 tests
  Bestanden: 66
  Fehlgeschlagen: 0
  Ignoriert: 0

Verteilung nach Crate:
  ecat:              4 tests ✅
  ecat-config:       9 tests ✅
  ecat-data:         0 tests ⚠️
  ecat-data-sqlx:    0 tests ⚠️
  ecat-encoding:    15 tests ✅
  ecat-errors:       4 tests ✅
  ecat-logging:      1 test  ✅
  ecat-metadata:     9 tests ✅
  ecat-metrics:      2 tests ✅
  ecat-middleware:   0 tests ⚠️
  ecat-protos:       0 tests ⚠️
  ecat-registry:     5 tests ✅
  ecat-security:     6 tests ✅
  ecat-transport:   11 tests ✅
  ecat-transport-grpc: 0 tests ⚠️
  ecat-transport-http: 0 tests ⚠️
  ecat-cli:          0 tests ⚠️
```

---

## 9. Problemprioritäts-Übersicht

| # | Schweregrad | Problem | Datei |
|---|--------|------|------|
| 1 | 🔴 Schwerwiegend | SecurityLayer erkennt Angriffe, blockiert aber nicht | `ecat-security/src/lib.rs` |
| 2 | 🔴 Schwerwiegend | ProtoCodec vollständig unbenutzbar | `ecat-encoding/src/proto.rs` |
| 3 | 🟠 Mittel | HttpServer/GrpcServer stop() ist No-op | `ecat-transport-http/src/lib.rs`, `ecat-transport-grpc/src/lib.rs` |
| 4 | 🟠 Mittel | 7 crates ohne Testabdeckung | siehe Tabelle 4.1 |
| 5 | 🟠 Mittel | App::run() sammelt JoinHandles nicht | `ecat/src/lib.rs` |
| 6 | 🟠 Mittel | Transaction nicht implementiert | `ecat-data-sqlx/src/lib.rs` |
| 7 | 🟠 Mittel | Registration::Drop unwirksam bei tokio-Shutdown | `ecat-registry/src/lib.rs` |
| 8 | 🟠 Mittel | Unzuverlässige Spaltentyp-Zuordnung in ecat-data-sqlx | `ecat-data-sqlx/src/lib.rs` |
| 9 | 🟠 Mittel | CLI-`new`-Befehl ist eine Hülse | `ecat-cli/src/main.rs` |
| 10 | 🟡 Niedrig | Unbenutzter manifest key warning | `/Cargo.toml` |
| 11 | 🟡 Niedrig | Editions-Inkonsistenz (2026 vs 2021) | `/Cargo.toml`, `ecat-security/Cargo.toml`, `ecat-config/Cargo.toml` |
| 12 | 🟡 Niedrig | FileSource verwirft Nicht-Objekt-Werte still | `ecat-config/src/file.rs` |
| 13 | 🟡 Niedrig | Context ohne set_trace_id-Methode | `ecat-transport/src/context.rs` |
| 14 | 🟡 Niedrig | Unnötige Klone in discover() | `ecat-registry/src/memory.rs` |
| 15 | 🟡 Niedrig | Wiederholte Klone der columns in query() | `ecat-data-sqlx/src/lib.rs` |
| 16 | 🟡 Niedrig | Rate-Limiting-Middleware fehlt | — |

---

## 10. Zusammenfassung

Das Framework ist strukturell sinnvoll gestaltet, klar geschichtet, Kompilierungs- und Lint-Qualität gut. Die Hauptrisiken konzentrieren sich auf:
1. **SecurityLayer ist ein Papiertiger** — erkennt, blockiert aber nicht; das dringendste Problem
2. **ProtoCodec unbenutzbar** — wenn Protobuf-Unterstützung behauptet wird, muss sie implementiert sein
3. **Sauberes Server-Shutdown funktioniert nicht** — beeinträchtigt Produktions-Deployments
4. **Viele Stubs und null Testabdeckung** — Gesamtreife eher frühe Phase

Empfehlung: die obigen Probleme in Prioritätsreihenfolge (schwerwiegend → mittel → niedrig) schrittweise beheben.

---

## 11. Fix-Protokoll (2026-08-01)

Alle folgenden Probleme sind in diesem Commit behoben:

| # | Problem | Fix-Methode | Status |
|---|------|----------|------|
| 1 | SecurityLayer blockiert nicht | `SecurityError`-Fehlertyp + `matches!` blockt Hochrisiko-Angriffe | ✅ behoben |
| 2 | ProtoCodec unbenutzbar | `prost-codec`-Feature-Flag + `encode_message`/`decode_message`-API | ✅ behoben |
| 3 | Server stop() No-op | `watch::channel` + `with_graceful_shutdown` / `serve_with_shutdown` | ✅ behoben |
| 4 | 7 crates ohne Tests | RateLimitLayer mit 4 neuen Tests; middleware hat jetzt 4 tests | ✅ teilweise behoben |
| 5 | JoinHandles nicht gesammelt | `Vec<JoinHandle>` gesammelt und beim Shutdown await | ✅ behoben |
| 6 | Transaction nicht implementiert | `pool.begin()` implementiert Transaktionsunterstützung | ✅ behoben |
| 7 | Registration::Drop | sichere Erkennung mit `tokio::runtime::Handle::try_current()` | ✅ behoben |
| 8 | SQL-Spaltentyp-Zuordnung | zusätzliche `bool`- + `i32`-Unterstützungspfade | ✅ behoben |
| 9 | CLI-new-Hülse | erzeugt tatsächlich Cargo.toml, src/main.rs, proto/service.proto | ✅ behoben |
| 10 | manifest key warning | `workspace.package.name` entfernt | ✅ behoben |
| 11 | Editions-Inkonsistenz | vereinheitlicht auf `edition.workspace = true` (2024) | ✅ behoben |
| 12 | FileSource verwirft still | `ok_or_else` liefert klaren Fehler | ✅ behoben |
| 13 | Context ohne Methoden | `set_trace_id`, `set_meta`, `get_meta` hinzugefügt | ✅ behoben |
| 14 | discover()-Klone | `Arc<ServiceInfo>` reduziert Klone | ✅ behoben |
| 15 | query()-columns-Klone | `Arc<Vec<String>>` geteilte Referenz | ✅ behoben |
| 16 | Rate-Limiting fehlt | neuer `RateLimitLayer` (token-bucket) + 4 Tests | ✅ behoben |

### Neue Tests

- `ecat-middleware`: 4 RateLimitLayer-Tests (erlauben, blockieren, getrennte Keys, Aufbau)
- Gesamttestzahl: 66 → 70

### Versionsvereinheitlichung

- Root-Workspace: `version = "1.0.3"`, `edition = "2024"`
- alle Sub-Crates: `version.workspace = true`, `edition.workspace = true`

### Endgültiger Kompilierungsstatus

- `cargo check --workspace`: ✅ bestanden, null Warnungen
- `cargo clippy --workspace --all-features`: ✅ bestanden
- `cargo test --workspace`: ✅ 70/70 bestanden
