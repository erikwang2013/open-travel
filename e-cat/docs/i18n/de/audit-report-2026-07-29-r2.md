<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat Code-Review-Bericht (zweite Runde)

**Datum**: 2026-07-29  
**Branch**: main  
**Projekt**: e-cat (Rust workspace, 17 crates)

---

## I. Prüfungsübersicht

Aufbauend auf den Clippy-Fixes und Testergänzungen der ersten Runde wurde in dieser Runde eine tiefe Code-Logik-Prüfung durchgeführt, mit Schwerpunkt auf Laufzeitkorrektheit, Nebenläufigkeitssicherheit und API-Semantik-Konsistenz. Insgesamt 32 Quelldateien geprüft.

### Verifikationsbasis

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

---

## II. Gefundene Bugs und Fixes

### Bug 1: [Kritisch] Lebenszyklusfehler des span-Guards in TracingLayer

- **Datei**: `ecat-middleware/src/tracing.rs:37`
- **Schweregrad**: **hoch**
- **Auswirkung**: alle Requests durch TracingLayer werden von keinem tracing-span erfasst

**Ursachenanalyse**:

```rust
// vor dem Fix
fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let _guard = span.enter();  // guard wird bei Rückkehr von call() gedroppt
    let fut = self.inner.call(req);
    Box::pin(fut)               // future wird erst beim späteren poll ausgeführt
}
```

Der von `span.enter()` zurückgegebene Guard hält den span nur im aktuellen synchronen Kontext aktiv. `call()` gibt einen noch nicht gepollten future zurück; die tatsächliche asynchrone Ausführung erfolgt in der späteren Poll-Phase — zu diesem Zeitpunkt ist der Guard längst gedroppt, der span wird nicht wirksam. Kein Request durch TracingLayer erscheint in der tracing-Ausgabe.

**Fix**:

```rust
// nach dem Fix
use tracing::Instrument;

fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let fut = self.inner.call(req);
    Box::pin(fut.instrument(span))  // span hängt am future-Lebenszyklus
}
```

Mit `tracing::Instrument::instrument()` wird der span am future befestigt, sodass er über den gesamten Poll-Lebenszyklus des futures aktiv bleibt.

---

### Bug 2: [Kritisch] Implementierungsfehler des LifecycleHook-Closures — on_stop wird nie ausgeführt

- **Datei**: `ecat/src/hook.rs:14-23`, `ecat/src/lib.rs:11-16`
- **Schweregrad**: **hoch**
- **Auswirkung**: per `.on_stop()` registrierte Closure-Hooks tun beim Shutdown nichts

**Ursachenanalyse**:

Im ursprünglichen Design schoben sowohl `on_start()` als auch `on_stop()` die Hooks in denselben `lifecycle_hooks`-Vec. Bei `run()` riefen alle Hooks nacheinander `on_start()` auf, beim Shutdown riefen alle Hooks nacheinander `on_stop()` auf.

Das Problem liegt in der blanket-Implementierung des `LifecycleHook`-Traits für Closures `Fn() -> Fut`: **sie deckt nur `on_start()` ab, `on_stop()` verwendet die Trait-Standardimplementierung (No-op)**.

Das bedeutet: Wenn der Nutzer die Closure-Syntax `.on_stop(|| async { ... })` verwendet, wird der Closure zwar zur Hook-Liste hinzugefügt, beim Shutdown wird aber nur das leere Standard-`on_stop()` ausgeführt — die Nutzerlogik läuft nie.

**Fix (zweiteilig)**:

1. **start_hooks und stop_hooks trennen** (`ecat/src/lib.rs`):

```rust
// App-Struktur — zwei unabhängige Vecs
pub struct App {
    start_hooks: Vec<Box<dyn LifecycleHook>>,
    stop_hooks: Vec<Box<dyn LifecycleHook>>,
    // ...
}

// on_start() → start_hooks, on_stop() → stop_hooks
pub fn on_start(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.start_hooks.push(Box::new(hook));
    self
}
pub fn on_stop(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.stop_hooks.push(Box::new(hook));
    self
}
```

2. **Blanket-Impl für Closures vervollständigen** (`ecat/src/hook.rs`):

```rust
impl<F, Fut> LifecycleHook for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Result<...>> + Send,
{
    async fn on_start(&self) -> ... { (self)().await }
    async fn on_stop(&self) -> ...  { (self)().await }  // neu
}
```

Jetzt implementiert der Closure sowohl `on_start` als auch `on_stop`; zusammen mit den getrennten Vecs wird jeder Hook nur in der richtigen Lebenszyklusphase aufgerufen.

---

### Bug 3: [Mittel] Fehlerhafte Extraktionspriorität der Row-Werttypen in SqlxClient

- **Datei**: `ecat-data-sqlx/src/lib.rs:53-68`
- **Schweregrad**: mittel
- **Auswirkung**: ganzzahlige und Gleitkommawerte aus der Datenbank werden als JSON-Strings statt als Zahlen extrahiert

**Ursachenanalyse**:

`try_get::<String>()` stand an erster Stelle. Die meisten Datenbanktreiber können `try_get::<String>()` für numerische Spalten erfolgreich ausführen (implizite Konvertierung), wodurch der Ganzzahlwert `42` als `"42"` statt als `42` extrahiert wird.

**Fix**: Reihenfolge der `try_get`-Versuche auf `i64 → f64 → String → Null` umgestellt, numerische Typen werden bevorzugt erhalten.

---

## III. Weitere Prüffindungen (unverändert / bekannte Einschränkungen)

| Kategorie | Datei | Beschreibung | Empfehlung |
|------|------|------|------|
| Funktion unvollständig | `ecat-transport-http/src/lib.rs:30` | `axum::serve().await` blockiert und kehrt nie zurück, `stop()` ist No-op | graceful shutdown implementieren |
| Funktion unvollständig | `ecat-transport-grpc/src/lib.rs:29` | wie oben | graceful shutdown implementieren |
| Funktion unvollständig | `ecat-data-sqlx/src/lib.rs:79` | `transaction()` liefert Fehler „nicht implementiert" | Transaktionsunterstützung implementieren |
| Codestil | `ecat-middleware/src/logging.rs:42` | `elapsed.as_millis() as u64` theoretische u128→u64-Abschneidung | praktisch ohne Auswirkung |
| Tests fehlen | `ecat-middleware/` | 4 Tower-Services ohne Unit-Tests | Integrationstests nötig |
| Tests fehlen | `ecat-data/` | reine Trait-Definitionen | aktuell akzeptabel |
| RwLock-Blockade | `ecat-registry/src/memory.rs` | synchrones RwLock kann im async-Kontext blockieren | Wechsel auf tokio::sync::RwLock erwägen |

---

## IV. Testergebnisse

```
cargo test → 60 passed, 0 failed

Verteilung nach Crate:
  ecat                  4   (Builder/Standardwerte/Lebenszyklus-Hooks)
  ecat-config           9   (env parse ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (HTTP-Zuordnung/gRPC-Umwandlung/metadata/Display)
  ecat-logging          1   (init-Smoke)
  ecat-metadata         9   (Zugriff/From HeaderMap/From MetadataMap/Iterator)
  ecat-metrics          2   (Singleton/text ohne panic)
  ecat-registry         5   (Registrierung/Discovery/Abmeldung/Liste/Filter)
  ecat-transport       11   (Context/Request/Response/Server-Trait)
  andere 8 crates       0   (reine Traits/Codegenerierung/Integrationstests nötig/reine Ausgabe)
```

---

## V. Liste der geänderten Dateien

| Datei | Änderungstyp | Änderungsbeschreibung |
|------|----------|----------|
| `ecat/src/lib.rs` | Bug-Fix | App trennt start_hooks/stop_hooks; AppBuilder entsprechend aktualisiert; Tests angepasst |
| `ecat/src/hook.rs` | Bug-Fix | blanket-Impl für Closures um on_stop()-Implementierung ergänzt |
| `ecat-middleware/src/tracing.rs` | Bug-Fix | span-Guard → `fut.instrument(span)` |
| `ecat-data-sqlx/src/lib.rs` | Bug-Fix | Extraktionsreihenfolge der Row-Werte i64→f64→String→Null |

---

## VI. Zusammenfassung

Diese Runde fand 2 Laufzeit-Bugs mit hohem Schweregrad und 1 Datenkorrektheitsproblem mit mittlerem Schweregrad:

1. **TracingLayer-span wirkungslos** — beeinträchtigt die Beobachtbarkeit aller Requests
2. **LifecycleHook on_stop wird nicht ausgeführt** — beeinträchtigt die Korrektheit aller Shutdown-Logik
3. **Row-Numeriktyp geht verloren** — beeinträchtigt die Typkorrektheit der Datenbankabfrageergebnisse

Alle drei Probleme sind behoben; nach dem Fix bestehen alle 60 Tests, Kompilierung ohne Fehler und Warnungen.

### Folgeempfehlungen

- graceful shutdown für HTTP/gRPC-Server implementieren
- Integrationstests für `ecat-middleware` hinzufügen (mock Service + span/Timeout/Recovery-Verhalten verifizieren)
- Integrationstests für `ecat-data-sqlx` hinzufügen (mit SQLite-In-Memory-Datenbank)
- das synchrone RwLock in `ecat-registry/memory.rs` durch `tokio::sync::RwLock` ersetzen
