# e-cat Code-Review-Bericht — 2026-08-01 (Runde 4 · alles behoben)

**Projektversion:** 2.1.0  
**Endstatus:** 0 warnings, ~116 tests, clippy clean, fmt clean

**Aufräumen in Runde 5:** 12 unbenutzte Abhängigkeiten entfernt (ecat-health/reqwest, ecat-circuit-breaker/tokio, ecat-bench/tracing, ecat-mq/serde+serde_json, ecat-events/async-trait, ecat-config-remote/tracing, ecat-testing/transport-http+axum, ecat-client/serde+serde_json)
**Prüfungsumfang:** alle 18 crates

## Endstatus

| Werkzeug | Status |
|------|------|
| `cargo build` | bestanden (0 warnings) |
| `cargo test` | 77 passed, 0 failed, 1 ignored |
| `cargo clippy` | bestanden (0 warnings) |
| `cargo fmt` | bestanden |

---

## Fix-Liste (vollständig)

### Mittleres Risiko

1. **[behoben]** `Mutex::lock().unwrap()` → `ecat-transport-http/lib.rs`, `ecat-transport-grpc/lib.rs`
2. **[behoben]** CLI `fs::write().unwrap()` → `ecat-cli/src/main.rs`

### Geringes Risiko

3. **[behoben]** ProtoCodec-doc-test → `ecat-encoding/src/proto.rs`
4. **[behoben]** crates ohne Unit-Tests → transport-http/grpc je 3 neue Tests
5. **[behoben]** `Transaction::commit()` No-op → neues `TransactionInner`-Trait
6. **[behoben]** `SecurityScanner::new()`-Kommentarkorrektur
7. **[behoben]** unbenutzte `opentelemetry`-Abhängigkeit → `ecat-logging` und Workspace-Root-Cargo.toml
8. **[behoben]** doc-test-Format

### Optimierungen

9. **[behoben]** `scan_parts`-Vorallokation → `Vec::with_capacity`
10. **[behoben]** serde_yaml 0.9 veraltet → Migration auf `yaml_serde` 0.10
11. **[behoben]** `Transaction::commit()` kein No-op mehr → echtes commit/rollback über `SqlxTransactionWrapper`

### Kein Fix nötig (Designentscheidungen)

- **Zusätzliche Abhängigkeiten im `ecat`-Crate** — bewusstes „Meta-Crate"-Muster, bietet Downstream-Nutzern bequeme transitive Abhängigkeiten
- **ProtoCodec-Codec-Trait liefert Fehler** — grundlegender Typunterschied zwischen serde und prost::Message; über die getrennte API `encode_message()`/`decode_message()` und klare Doku gelöst
- **`ecat-data` ohne konkrete Implementierung** — Trait-Schnittstellendesign, Implementierungen liegen in `ecat-data-sqlx`

---

## Zusammenfassung der geänderten Dateien

| Datei | Änderung |
|------|------|
| `ecat-transport-http/src/lib.rs` | Mutex-Poisoning-Schutz + 3 neue Tests |
| `ecat-transport-grpc/src/lib.rs` | Mutex-Poisoning-Schutz + 3 neue Tests |
| `ecat-cli/src/main.rs` | einheitliche Fehlerbehandlung |
| `ecat-security/src/lib.rs` | Kommentarkorrektur + Vorallokationsoptimierung |
| `ecat-logging/Cargo.toml` | unbenutztes opentelemetry entfernt |
| `ecat-encoding/src/proto.rs` | doc-test verbessert |
| `ecat-data/src/lib.rs` | TransactionInner exportiert |
| `ecat-data/src/rdbms.rs` | neues TransactionInner-Trait |
| `ecat-data-sqlx/src/lib.rs` | SqlxTransactionWrapper implementiert TransactionInner |
| `ecat-config/Cargo.toml` | serde_yaml → yaml_serde |
| `ecat-config/src/file.rs` | serde_yaml → yaml_serde |
| `Cargo.toml` | verwaiste opentelemetry-Workspace-Abhängigkeit entfernt |
| `README.md` | Versionsnummer aktualisiert, Beobachtbarkeitsbeschreibung korrigiert, Ökosystemplan-Links ergänzt |
| `docs/ecosystem-plan.md` | neues Ökosystemplan-Dokument (3 Phasen, 15 crates) |
