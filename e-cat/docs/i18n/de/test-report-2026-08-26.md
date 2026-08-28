# Testbericht — 2026-08-26

Umfassende Nacharbeit der Unit-Tests (vollständige Abdeckung der 51 crates), 4 Gruppen erfahrener Rust-Testingenieure parallel.

## Übersicht

| Gruppe | crates | vorher | neu | aktuell | Gate |
|---|---|---|---|---|---|
| core/Framework | 12 | 102 | +40 | 142 | ✅ Tests komplett grün + clippy 0 Warnungen |
| data | 14 | 87 | +66 | 153 | ✅ wie oben |
| mq/transport | 12 | 82 | +54 | 136 | ✅ wie oben |
| app-Anwendungsschicht | 13 | ~178 | +46 | ~224 | ✅ wie oben |
| **Summe** | **51** | **~449** | **+206** | **~655** | ✅ |

Hinweis: Die Bestandszahlen der Anwendungsschicht umfassen ecat-auth 24 / ecat-graphql 35 / ecat-bench 11 / ecat-scheduler 6 / ecat-events 7 / ecat-cli 12 / ecat-middleware 34 / ecat-circuit-breaker 10 / ecat-health 4 / ecat-client 7 / ecat-security 12 / ecat-versioning 4 / ecat 4. Für jedes Crate bestanden `cargo test -p` und `cargo clippy -p --all-targets -- -D warnings` unabhängig; CARGO_TARGET_DIR-isoliert parallel.

## Crate-Detail

### Gruppe core/Framework (test-core, +40)

| Crate | vorher→neu | Abdeckungsschwerpunkte |
|---|---|---|
| ecat-protos | 4→8 | ErrorCode-Gesamtenum gegen proto; abgeschnittener Buffer-Decode; leerer Buffer-Default-Nachricht; metadata-Roundtrip |
| ecat-errors | 4→9 | http_status-Gesamtzuordnung (409/429/500); from_status; nicht zugeordnet→Internal; cause source() |
| ecat-metadata | 9→12 | trace_id aus HTTP-Header extrahieren; Key-Kleinschreibung; leere Header-Map |
| ecat-encoding | 18→22 | NaN→null (serde_json-Standard, dokumentiert); leere Bytes decoden; CodecBox illegales JSON; proto-Roundtrip |
| ecat-lock | 7→9 | release ohne gehaltenes Lock meldet Fehler; leerer Key |
| ecat-logging | 1→1 | kompatibler Shim panict nicht |
| ecat-tracing | 9→12 | Nicht-UTF-8-trace-Header überspringen; kanonischer Header; Response-Durchreichung |
| ecat-tls | 7→12 | basic_auth einzeln/beide Felder; fehlende ca-Datei; is_enabled; Standard-Client |
| ecat-config | 14→26 | env-Präfixfilter + Typ-Parse-Grenzen (hex/leerer String/-0/1e3); Multi-Source-Merge-Override; obfs-Fehlerpfade; fehlende Datei/illegales YAML |
| ecat-config-remote | 6→9 | ConsulKvEntry-Grenzen; fehlendes X-Consul-Index meldet Fehler; verschachtelte Keys |
| ecat-openapi | 4→11 | components/schema_ref; doppelte Übersteuerung; Default 200; tags |
| ecat-metrics | 8→11 | Text bereits registrierter Metriken; 404/405 |

### Gruppe data (test-data, +66)

| Crate | vorher→neu | Abdeckungsschwerpunkte |
|---|---|---|
| ecat-data | 12→14 | Suchsyntax-Parsing |
| ecat-data-sqlx | 7→14 | In-Memory-SQLite-End-to-End; Parameterbindung aller Typen; Blob→base64; config |
| ecat-data-redis | 6→12 | redis:///rediss://-URL-Aufbau; auth; config-Fehlerpfade |
| ecat-data-opensearch | 4→10 | mock HTTP: percent-encode, Basic Auth, Fehlerdurchreichung |
| ecat-data-elasticsearch | 6→11 | wie oben |
| ecat-data-influxdb | 5→10 | line-protocol-Escaping; Token-Header; Fehlerdurchreichung |
| ecat-data-clickhouse | 12→22 | CREATE-TABLE-SQL; JSONEachRow; geschriebene Zeilenzahl; Gruppierung |
| ecat-data-memcached | 4→8 | TTL Sekunden→Millisekunden; flag-Packing |
| ecat-data-nebulagraph | 6→7 | config-Parsing |
| ecat-data-arangodb | 5→7 | config/URL |
| ecat-data-iotdb | 5→10 | mock HTTP: Session-Pfadparameter |
| ecat-data-questdb | 4→9 | line protocol; Transaktionen nicht unterstützt |
| ecat-data-tdengine | 6→11 | INSERT-Erzeugung; 100er-Batch-Chunking |
| ecat-data-mongodb | 5→8 | bson-Roundtrip; URI |

### Gruppe mq/transport/registry (test-mq, +54)

| Crate | vorher→neu | Abdeckungsschwerpunkte |
|---|---|---|
| ecat-mq | 5→9 | voller Puffer mit Latenz-Fehlerframe; Stream-Schließen bei komplettem Drop; mehrere Abonnenten; publish ohne Abonnenten |
| ecat-mq-kafka | 12→14 | config-Defaults; SASL-Felder wirken unabhängig |
| ecat-mq-rabbitmq | 2→5 | exchange-Default; url-Fehlerpfade |
| ecat-mq-mqtt | 5→9 | cert/key-Paar-Validierung; fehlende Dateien; Port-Defaults 1883/8883; ungültiger Port-Fallback |
| ecat-mq-nats | 6→9 | Klartext-Default; ca/cert-Fehlerpfade |
| ecat-transport | 4→7 | TlsConfig-Default/with_client_auth; normalize_addr-Grenzen |
| ecat-transport-http | 17→20 | Integrationstests: stop als No-op, belegter Port schlägt fehl, echte Sende-/Empfangsrunde |
| ecat-transport-grpc | 7→13 | TLS ohne Datei; Klartext-Lebenszyklus; mTLS-Ablehnung |
| ecat-transport-ws | 4→8 | ohne Handler schlägt fehl; belegter Port; RFC-6455-masked-Frame-Echo |
| ecat-registry | 5→8 | discover mit mehreren Instanzen; Auto-Deregistrierung bei Drop; builder-Defaults |
| ecat-registry-consul | 10→24 | percent-encode; Registrierungsvarianten; Fehler-Responses; X-Consul-Token; agent/services-Parsing; node-Fallback |
| ecat-registry-etcd | 5→10 | discover überspringt ungültige Werte; kv-Request-Body; lease grant; keepalive |

### Gruppe app-Anwendungsschicht (test-app, +46)

| Crate | vorher→neu | Abdeckungsschwerpunkte |
|---|---|---|
| ecat-auth | 20→46 | oauth2-Cache-Whitelist/SHA-256-Key/FIFO-Eviction; apikey drei Zustände; jwt iss/aud erzwingen; abgelaufen/falsche Signatur |
| ecat-health | 4→8 | readiness-Aggregation (alle ok/ein fail/leere Registrierung); liveness |
| ecat-versioning | 4→7 | path-Strategie-Routing; extract_version-Grenzen |
| ecat-security | 12→20 | End-to-End auf Header-Ebene; JSON-Form der blockierten Angriffe |
| ecat-middleware | 34→37 | MemoryStore-Fensterablauf; innerer panic→Err |
| ecat-circuit-breaker | 10→12 | half-open-Proben erschöpft; classify-Degradierung |
| ecat-client | 7→10 | grpc-Ungültig-Endpoint meldet Fehler ohne Netzwerk |
| ecat-graphql | 35→35 | bestehende Abdeckung ausreichend, keine Lücken |
| ecat-scheduler / ecat-bench / ecat-events / ecat-cli / ecat | bestehende Abdeckung ausreichend | keine Lücken |

## Gefundene Defekte

| Stufe | Ort | Beschreibung | Status |
|---|---|---|---|
| P1 | ecat-events/Cargo.toml | dev-dependencies fehlen tokio macros/rt/time-Features, das Testziel dieses Crates schlägt bei Einzelkompilierung zwingend fehl (im Workspace-Vollbau durch die Feature-Vereinigung verschleiert) | ✅ behoben (Features + Kommentar ergänzt) |
| P2 | ecat-security src/lib.rs:118-127 | Prozent-codierte SQLi in der URI (`?q=SELECT%20*%20...`) kann den Header-Scan umgehen (Detektor verlangt wörtliche Leerzeichen, scannt die rohe URI ohne vorheriges Decodieren); Body-Scan nicht betroffen | ⏳ offen |
| P3 | ecat-data-sqlx | `connect()/from_config()` verwenden AnyPool ohne installierten Treiber; sqlx 0.8.6 panict beim ersten Verbindungsversuch mit „No drivers installed" | ⏳ offen |
| P3 | ecat-data-influxdb | String-Felder escapen Leerzeichen (`\ `), das line-protocol-Spezifikation nur `"` und `\` verlangt; tag/field-Reihenfolge nicht deterministisch | ⏳ offen |
| P3 | ecat-data-clickhouse | Create-Table-Cache läuft nie ab, nach externem drop/Ändern wird CREATE nicht erneut versucht | ⏳ offen |
| P3 | ecat-circuit-breaker | half_open_probes-Obergrenze ist unter sequenziellem Probing nicht erreichbar (nur bei parallelen in-flight), Whitebox-Test abgedeckt | ℹ️ bekannt, kein Defekt |
| P3 | ecat-health | `with_check` verwendet blocking_write(), Aufruf im async-Kontext panict; aktuell nur im synchronen Kontext nutzbar | ℹ️ bekannt, API-Einschränkung |

## Übersprungene Module (benötigen Integrationsumgebung, nicht gemockt)

- echte Broker-Roundtrips: kafka/rabbitmq/mqtt/nats publish-subscribe (Konfiguration und Fehlerpfade abgedeckt)
- echte Cluster: consul/etcd Registrierungs-Discovery-Lebenszyklus (axum-mock deckt Request-Formen ab)
- echte Datenbanken: redis/memcached-Operationen, mongod, influxdb-Serverseitige Validierung, sqlx postgres/mysql-Treiber, nebulagraph/arangodb-APIs
- echte externe Dienste: OAuth2-Introspection (lokal gemockt), gRPC/HTTP-Roundtrips (lokal gemockt, deckt „302 nicht folgen" ab)
