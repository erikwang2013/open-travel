# Spezialprüfungsbericht (Sicherheit und Performance) — 2026-08-14

Prüfumfang: 55-Crate-Workspace (v2.3.5). Methode: manuelle Cargo.lock-Prüfung (cargo-audit nicht installiert), Quellcode-Audit der Auth/TLS-Pfade, Prüfung von Nebenläufigkeit und Ressourcenlebenszyklen. Keine Commits.

## Abhängigkeits-CVE-Prüfung

- Die Kernabhängigkeiten sind alle aktuell und ohne bekannte unbehobene CVEs: rustls 0.23.43, ring 0.17.14, aws-lc-rs 1.17.3, jsonwebtoken 9.3.1, tokio 1.53.1, h2 0.4.15, quinn 0.11.11, sqlx 0.8.6, zerocopy 0.8.55, time 0.3.54, openssl 0.10.81.
- hyper 0.14.32 (nur von rust-s3 0.35.1 über hyper-tls 0.5) liegt über der Fix-Linie 0.14.28.
- Hinweis: CI installiert kein cargo-audit, Empfehlung: automatisierte Prüfung in den Workflow aufnehmen.

## Befunde (nach Schweregrad sortiert)

### S1 [Mittel] HTTP-TLS-Handshake serialisiert → langsame Handshake-DoS
- Stelle: `ecat-transport-http/src/lib.rs:134-150` (TlsListener::accept)
- Symptom: Der TLS-Handshake läuft synchron innerhalb `accept()`, axum::serve ruft accept seriell auf — eine Verbindung, die den Handshake nicht abschließt, blockiert die gesamte accept-Schleife.
- Auswirkung: Angreifer können mit Massen von langsamen/Zombie-TCP-Verbindungen den Dienst komplett daran hindern, neue Verbindungen anzunehmen (auf gRPC-Seite spawnt tonic pro Verbindung einen Handshake, dort nicht betroffen).
- Empfehlung: Nach accept den Handshake mit `tokio::spawn` ausführen und mit `tokio::time::timeout(10s)` versehen, bei Fehlschlag Verbindung schließen.

### S2 [Mittel] OAuth2-Introspection-Cache wächst unbegrenzt → Memory-DoS
- Stelle: `ecat-auth/src/oauth2.rs:45,84-92`
- Symptom: `HashMap<String,(String,Instant)>` mit Token als Schlüssel; die TTL steuert nur die Frische, es gibt kein Kapazitätslimit und keine Eviction.
- Auswirkung: Massen von einzigartigen Token-Requests können den Speicher unbegrenzt wachsen lassen (jeder Miss löst zusätzlich eine Upstream-Introspection aus).
- Empfehlung: Kapazitätslimit (z. B. 10k) + regelmäßige Bereinigung, oder moka/LRU mit Kapazität und TTL-Eviction.

### S3 [Niedrig-mittel] ecat-data-s3 nutzt veraltetes rust-s3 0.35.1 (hyper 0.14 + native-tls/openssl)
- Stelle: `ecat-data-s3/Cargo.toml` → rust-s3 0.35.1
- Symptom: Der S3-Client verwendet einen eigenen hyper-tls/openssl-Stack; `ecat-tls::TlsClientConfig` (Custom-CA, Client-Zertifikate, skip_verify) wirkt nicht auf S3 — inkonsistente TLS-Konfigurationsfläche.
- Auswirkung: Private-CA/mTLS für S3 in Unternehmensumgebungen nicht konfigurierbar; Abhängigkeit seit 2023 nur langsam gepflegt.
- Empfehlung: Upgrade von rust-s3 evaluieren oder auf einen einheitlichen reqwest/rustls-Client umstellen.

### S4 [Niedrig] JWT-Standardvalidierung ohne iss/aud
- Stelle: `ecat-auth/src/jwt.rs:125` — `Validation::new(HS256)` nur Signatur + exp.
- Auswirkung: Mit gemeinsamem HS256-Schlüssel kann ein Token eines Dienstes von einem anderen Dienst akzeptiert werden (keine Aussteller-Isolation).
- Empfehlung: Doku verlangt explizit issuer/audience für die Produktion; oder standardmäßig eine iss-Validierungsoption anbieten.

### S5 [Niedrig] `TlsClientConfig.skip_verify` allein setzt is_enabled() auf wahr
- Stelle: `ecat-tls/src/lib.rs:23-29`
- Symptom: Nur mit `skip_verify: true` gilt TLS als „aktiviert", ohne dass Zertifikate geprüft werden — Validierung wird still abgeschaltet.
- Empfehlung: skip_verify und ca_cert gegenseitig ausschließend validieren oder explizite doppelte Bestätigung verlangen.

## Performance und Ressourcen

### P1 [Niedrig] OAuth2-Cache-Hit-Pfad deserialisiert pro Request JSON
- Stelle: `ecat-auth/src/oauth2.rs:87` — Cache speichert serialisierte Strings, Treffer parsen trotzdem via `serde_json::from_str`.
- Empfehlung: Cache direkt die `AuthClaims`-Struktur speichern, spart das Parse pro Request.

### P2 [Niedrig] ecat-bench ohne Warmup und Steady-State-Erkennung
- Stelle: `ecat-bench/src/lib.rs:run_bench` — direkt Zeitmessung, kein Warmup; Kaltstart-/Pool-Erstallokationen verfälschen p99.
- Empfehlung: Warmup-Runden und Steady-State-Konvergenzerkennung ergänzen, Ergebnisse werden verlässlicher.

### P3 [Niedrig] Kafka-Consumer pollt 100 ms + schläft 100 ms seriell
- Stelle: `ecat-mq-kafka/src/lib.rs:84-92` — End-to-End-Latenz der Nachrichten bei ~200 ms gedeckelt.
- Empfehlung: Nach poll kein zusätzlicher sleep nötig; bei Niedrig-Durchsatz-Szenarien poll-Intervall verkürzen.

## Bestätigte gute Praktiken

- Keine unwrap/expect-panics auf Produktionspfaden (transport/auth/middleware nur in Tests).
- API-Key-Query-Parameter-Fallback mit Leak-Warnung im Log; HashMap nutzt SipHash gegen Kollisionsangriffe.
- SQL-Layer reicht Aufrufer-SQL durch (Framework-Natur), user:pass in Verbindungsstrings korrekt Prozent-kodiert.
- Kafka-Consumer blockiert bei vollem Kanal (Backpressure) statt zu verwerfen; nach rx-drop beendet sich der poll-Task sauber.
- config-remote-Pull mit Timeout (5s/30s); blockierende Abfragen melden fehlenden Index als Fehler statt Busy-Wait.

---

## Korrektheitsaudit der Kerndomänen (Ergänzung, komplementär zum obigen Sicherheits-/Performance-Spezial)

Auditmethode: Scan des gesamten Workspace-Produktionscodes (unwrap/expect/panic-Lokalisierung, still verschluckte Fehler, asynchrones Stoppen, nebenläufige Zustände) + vollständige Re-Verifikation mit `cargo test --workspace` (erste Runde komplett grün; während des laufenden S1-Fixes gab es mitten in der Kompilierung Warnungen in transport-http, nach Abschluss muss neu gelaufen werden). Keine Commits.

### N1 [Mittel] Nach Beendigung des ecat-events-Consumtasks bleibt ein Handle-Leak zurück → stiller Event-Verlust
- Stelle: `ecat-events/src/lib.rs:97-101` (Consum-Schleife 89-95, `None => break`)
- Symptom: Gibt der mq-Stream None zurück (z. B. geschlossener Kafka-Broadcast-Kanal) oder panickt der Task, beendet sich die Consum-Schleife, aber das JoinHandle bleibt in der `consumers`-Map zurück; danach startet ein erneutes `subscribe()` desselben Eventtyps wegen `contains_key` (Zeile 68, immer wahr) den Consum-Task nicht neu → dieser Eventtyp ist dauerhaft still verloren.
- Auswirkung: Nach Unterbrechung des entfernten Event-Streams keine Selbstheilung; Wiederherstellung nur durch Prozessneustart.
- Empfehlung: Beim Task-Ende das Handle aus der Map entfernen (Watcher spawnen oder `handle.is_finished()`-Lazy-Cleanup).

### N2 [Mittel] Falsche group_id-Semantik bei ecat-mq-kafka subscribe
- Stelle: `ecat-mq-kafka/src/lib.rs:71-84`
- a. Bei standardmäßigem `group_id` None verlangt rdkafka `consumer.subscribe()` eine group.id (librdkafka meldet INVALID_ARG) — mit Standardkonfiguration schlägt das Subscriben wahrscheinlich direkt fehl (auf echter Hardware zu verifizieren).
- b. Bei konfiguriertem group_id (ecat-events subscribet pro Eventtyp einmal, gleiche Gruppe) teilt Kafka das Topic partitionenweise auf die Consumer derselben Gruppe auf → ein Eventtyp kann auf einen fremden Consum-Task fallen und still verworfen werden (auto.offset.reset=latest, kein Commit).
- Auswirkung: Der Event-Bus verliert unter dem Kafka-Backend Events.
- Empfehlung: Ohne group_id eine zufällige eindeutige group.id erzeugen; oder der Consumer teilt Partitionen explizit per assign() zu; Doku: Mehrfach-Subscribe erfordert getrennte Gruppen.

### N3 [Niedrig] GrpcServer/WsServer: leere Hosts nicht normalisiert (D1-Fix unvollständig)
- Stelle: `ecat-transport-grpc/src/lib.rs:52`, `ecat-transport-ws/src/lib.rs:58`
- Symptom: `GrpcServer::new(":8000")` → `addr.parse::<SocketAddr>()` liefert AddrParseError (bereits praktisch verifiziert); `WsServer` `TcpListener::bind(":8000")` löst zu IPv6-Wildcard auf und scheitert ohne IPv6-Umgebung. HttpServer normalisiert bereits auf 0.0.0.0 — die drei Server-APIs verhalten sich inkonsistent.
- Empfehlung: Einheitlich leere Hosts in new normalisieren.

### N4 [Niedrig] TracingLayer injiziert keine trace_id, widerspricht CHANGELOG-2.3.3-Angabe
- Stelle: `ecat-tracing/src/lib.rs:72-84` (Span enthält nur das service-Feld; Code-Kommentar räumt selbst ein, dass beim generischen Req keine Header entnehmbar sind); `inject_trace_id()` erzeugt pro Aufruf eine neue UUID und übernimmt keine vom Upstream extrahierte trace_id.
- Auswirkung: Das laut Doku konfigurierte verteilte Tracing lässt sich nicht über Dienste hinweg verknüpfen.
- Empfehlung: Span-Feld lazy binden oder auf `http::Request<B>` spezialisieren; inject soll eine Upstream-ID mitführen können.

### N5 [Niedrig] Job-panic in ecat-scheduler stoppt still
- Stelle: `ecat-scheduler/src/lib.rs:53-57,83` (`run()` mit `let _ = handle.await`)
- Symptom: Nach einem panic des geplanten Tasks stirbt der Task ohne Neustart und ohne Log; `run()` verwirft den JoinHandle-Fehler.
- Empfehlung: panic abfangen, loggen + optional Neustart-Strategie.

### N6 [Niedrig] Verbliebene unwraps im Produktionscode (Poisoning-/panic-Pfade)
- `ecat-events/src/lib.rs:68,98` std-`Mutex::lock().unwrap()` (panic bei Poisoning); `ecat-versioning/src/lib.rs:86` Response-Builder-unwrap (kann nicht fehlschlagen, aber panic-Pfad); `ecat-mq/src/lib.rs:110` expect ist durch is_none-Guard geschützt (sicher).
- Empfehlung: Die zwei Stellen in events auf `unwrap_or_else(|e| e.into_inner())` umstellen.

### N7 [Info] `WsServer::stop()` wartet nicht auf hochgestufte WebSocket-Verbindungen
- Stelle: `ecat-transport-ws/src/lib.rs:63-87`
- axum-on_upgrade-Verbindungen laufen in separaten Tasks, graceful shutdown deckt sie nicht ab; Long-Lived-Connection-Handler verbleiben nach stop(), Prozess beendet sich nicht sauber (App::stop-Semantik unvollständig).

### N8 [Info] Crates ohne Tests: ecat-data / ecat-lock / ecat-protos
- Alles Trait-/Definitions-Crates; verifiziert: Standardmethoden sind fail-loud (geben Fehler zurück statt still zu schweigen), aber die Trait-Verträge (Transaction-Drop-Rollback-Semantik, Lock-Token-Validierung) haben keinerlei Unit-Tests.
- Empfehlung: Minimale Unit-Tests für die Semantik von RdbmsError/Transaction und DistributedLock ergänzen.

### N9 [Info] GraphQL-Parameter und verschachtelte Felder werden weiterhin verworfen
- `ecat-graphql/src/lib.rs` execute übergibt an den Resolver nur `variables`; Feldparameter wie `{ hello(name: "x") }` und verschachtelte selections werden nicht durchgereicht; README weist die Einschränkung nicht aus (Alter Befund L8 forderte Dokumentation, nach der 2.3.3-Neufassung immer noch offen).

### N10 [Info] circuit-breaker zählt nur Transportlayer-Fehler
- `ecat-circuit-breaker/src/lib.rs:203-209` wertet nur inner-Err als Fehlschlag, HTTP-5xx gilt als Erfolg → der Circuit Breaker greift bei Dienstunverfügbarkeit (5xx-Sturm) nicht; nicht dokumentiert.

**Verifikationsstatus**: Erste Runde `cargo test --workspace` komplett grün (inkl. Doc-Tests, keine Fehler in der Ausgabe); während der S1-Fix-Agent-Editierungen traten in transport-http Kompilierfehler und 2 Warnungen auf (unused import `ensure_crypto_provider`, `shutdown_tx` ungenutzt) — Zwischenzustand, nach S1-Abschluss müssen Tests und `clippy --all-targets -D warnings` vollständig neu laufen.

---

## Dritte Runde: dynamische Verifikation + CVE-Nachprüfung + panic-Fläche (Spezial, 2026-08-14)

### CVE-Nachprüfung (neue Befunde, nach Schweregrad)

1. **[Mittel] rustls-webpki 0.102.8 verbleibt im Abhängigkeitsbaum** (RUSTSEC-2026-0049/0098/0099/0104: CRL-distributionPoint-Umgehung, URI-/Wildcard-name-constraints, Fix-Version 0.103.10). Hauptkette ist 0.103.13 (über rustls 0.23.43, sicher); 0.102.8 kommt über async-nats 0.38.0 / rumqttc 0.25.1 herein und deckt die NATS-/MQTT-TLS-Clientketten ab. Upstream hat nicht auf rustls 0.23 migriert, keine Fix-Version — kontrolliertes Risiko, Kommentar-Tracking empfohlen.
2. **[Mittel-niedrig] rdkafka 0.36.2 bringt eingebettetes librdkafka mit cJSON 1.7.14 mit** (CVE-2023-53154 und die cJSON-Serie; CVE-2025-57052 mit CVSS 9.8, aber die betroffene Datei cJSON_utils.c wird von librdkafka nicht genutzt, Anwendbarkeit fraglich). Upstream-Fix in librdkafka 2.10+ (2026-03, PR #5346). ecat-mq-kafka verlinkt statisch; Packversion von librdkafka-sys prüfen und Upgrade verfolgen.
3. **[Niedrig] rustls-pemfile 2.2.0 ungepflegt** (RUSTSEC-2025-0134) — ecat-transport-http parst beim Start lokale Dateien, keine Angreifereingaben.
4. **[Niedrig] rsa 0.9.10** (RUSTSEC-2023-0071 Marvin-Timing-Seitenkanal) — über sqlx-mysql-TLS eingebracht, nur bei MySQL + RSA-Schlüsselaustausch relevant.
5. async-nats 0.38.0 liegt über der Fix-Linie von RUSTSEC-2023-0027 (CN-Validierungsumgehung), kein Problem.

### Dynamische Verifikation (examples/helloworld, debug-Build, temporärer Port 18080, bereinigt)

- /health 200, / (JSON-Serialisierung) 200 (27B), 404 normal; Logging-Middleware protokolliert Requests korrekt.
- **/metrics gemountet, liefert aber 200 + leeren Body (0 Bytes)**: Ohne registrierte Metriken gibt es keinerlei Ausgabe — Monitoring kann „gesund/keine Metriken" nicht unterscheiden. Empfehlung: leere Registry gibt eine Kommentarzeile oder 503 aus.
- Fehlerhafte Requests (Header mit 0x01/0x02) → 400 Bad Request, Dienst bleibt am Leben, nachfolgendes /health weiterhin 200, kein panic.
- TLS/mTLS-Pfade und Circuit-Breaker-/Rate-Limit-Middleware: durch ecat-transport-http/grpc- und ecat-middleware-Tests abgedeckt (nach mTLS-Race-Fix komplett grün, Fälle „anonyme/fehlerhafte Client-Zertifikate abgelehnt" bestehen).

### Bench-Baseline

- ecat-bench hat kein [[bench]]/bin-Target, kein cargo-bench-Einstiegspunkt; run_bench_with_warmup bringt bereits Warmup mit (P2-Fix umgesetzt), Harness-Tests alle grün.
- Praxismessung als debug-Build-Smoke: / ~1,3 ms, /health ~1,8 ms (inkl. curl-Prozess-Overhead, ohne Baseline-Aussage). Empfehlung: release-Build + wrk/hey-Benchmark für echte Baseline.

### Re-Prüfung der panic-Fläche (gesamter Workspace, Testmodule ausgenommen)

- Insgesamt 31 unwrap/expect/panic-Stellen, alle risikoarm: `Response::builder().body().unwrap()` (nicht fehlschlagbare Zweige in jwt/apikey/oauth2), Lock-Poisoning-Fallback (etcd/testing), clickhouse `serde_json::to_string().unwrap()` (theoretischer panic bei extremen NaN/inf-Eingaben).
- **1 Stelle beachten**: `ecat-transport-http/src/tls_listener.rs:234` — bei abnormalem Beenden der Hintergrund-accept-Schleife panickt `accept()` selbst, der Dienst-Thread stirbt (Auslösebedingungen eng: nur fatale Listener-Fehler); Empfehlung: auf Fehlerrückgabe degradieren und loggen.
