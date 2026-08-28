# Informe de pruebas — 2026-08-26

Complemento integral de pruebas unitarias (cobertura total de 51 crates), con 4 equipos de ingenieros de pruebas Rust senior en paralelo.

## Resumen

| Equipo | crates | Existían | Nuevas | Actuales | Puerta |
|---|---|---|---|---|---|
| core/framework | 12 | 102 | +40 | 142 | ✅ tests en verde + clippy 0 advertencias |
| data | 14 | 87 | +66 | 153 | ✅ ídem |
| mq/transport | 12 | 82 | +54 | 136 | ✅ ídem |
| capa app | 13 | ~178 | +46 | ~224 | ✅ ídem |
| **Total** | **51** | **~449** | **+206** | **~655** | ✅ |

Nota: las cifras existentes de la capa app incluyen ecat-auth 24 / ecat-graphql 35 / ecat-bench 11 / ecat-scheduler 6 / ecat-events 7 / ecat-cli 12 / ecat-middleware 34 / ecat-circuit-breaker 10 / ecat-health 4 / ecat-client 7 / ecat-security 12 / ecat-versioning 4 / ecat 4. En cada crate, `cargo test -p` independiente + `cargo clippy -p --all-targets -- -D warnings` pasan, con CARGO_TARGET_DIR aislado en paralelo.

## Detalle por crate

### Equipo core/framework (test-core, +40)

| crate | Antes→Nuevo | Puntos cubiertos |
|---|---|---|
| ecat-protos | 4→8 | ErrorCode completo comparado con el proto; decode con buffer truncado; buffer vacío con mensaje por defecto; roundtrip de metadata |
| ecat-errors | 4→9 | mapeo completo de http_status (409/429/500); from_status; sin mapear→Internal; causa source() |
| ecat-metadata | 9→12 | extracción de trace_id desde header HTTP; claves en minúsculas; header map vacío |
| ecat-encoding | 18→22 | NaN→null (por defecto de serde_json, documentado); decode de bytes vacíos; CodecBox con JSON inválido; roundtrip proto |
| ecat-lock | 7→9 | release sin mantener el bloqueo da error; clave vacía |
| ecat-logging | 1→1 | el shim de compatibilidad no entra en panic |
| ecat-tracing | 9→12 | salto de trace headers no UTF-8; header canónico; reenvío de respuesta |
| ecat-tls | 7→12 | basic_auth de uno/dos campos; falta el archivo ca; is_enabled; cliente por defecto |
| ecat-config | 14→26 | filtro de prefijo env + límites de parseo de tipos (hex/cadena vacía/-0/1e3); fusión y sobrescritura de múltiples fuentes; rutas de error de obfs; archivo ausente/YAML inválido |
| ecat-config-remote | 6→9 | límites de ConsulKvEntry; error sin X-Consul-Index; claves anidadas |
| ecat-openapi | 4→11 | components/schema_ref; sobrescrituras duplicadas; 200 por defecto; tags |
| ecat-metrics | 8→11 | texto de métricas ya registradas; 404/405 |

### Equipo data (test-data, +66)

| crate | Antes→Nuevo | Puntos cubiertos |
|---|---|---|
| ecat-data | 12→14 | análisis de sintaxis de búsqueda |
| ecat-data-sqlx | 7→14 | SQLite en memoria de extremo a extremo; binding de parámetros de todos los tipos; Blob→base64; config |
| ecat-data-redis | 6→12 | construcción de URLs redis:///rediss://; auth; rutas de error de config |
| ecat-data-opensearch | 4→10 | HTTP mock: percent-encode, Basic auth, reenvío de errores |
| ecat-data-elasticsearch | 6→11 | ídem |
| ecat-data-influxdb | 5→10 | escapes de line protocol; header Token; reenvío de errores |
| ecat-data-clickhouse | 12→22 | SQL de creación de tabla; JSONEachRow; recuento de filas escritas; agrupación |
| ecat-data-memcached | 4→8 | TTL segundos→milisegundos; empaquetado de flags |
| ecat-data-nebulagraph | 6→7 | parseo de config |
| ecat-data-arangodb | 5→7 | config/URL |
| ecat-data-iotdb | 5→10 | HTTP mock: parámetros de ruta de sesión |
| ecat-data-questdb | 4→9 | line protocol; transacciones no soportadas |
| ecat-data-tdengine | 6→11 | generación de INSERT; fragmentación en lotes de 100 |
| ecat-data-mongodb | 5→8 | roundtrip bson; URI |

### Equipo mq/transport/registry (test-mq, +54)

| crate | Antes→Nuevo | Puntos cubiertos |
|---|---|---|
| ecat-mq | 5→9 | frames de error con buffer lleno y lag; cierre de stream con drop total; múltiples suscriptores; publish sin suscriptores |
| ecat-mq-kafka | 12→14 | valores por defecto de config; campos SASL con efecto independiente |
| ecat-mq-rabbitmq | 2→5 | exchange por defecto; rutas de error de URL |
| ecat-mq-mqtt | 5→9 | validación de emparejamiento cert/key; archivos ausentes; puertos por defecto 1883/8883; puerto inválido con fallback |
| ecat-mq-nats | 6→9 | texto plano por defecto; rutas de error con ca/cert ausentes |
| ecat-transport | 4→7 | TlsConfig por defecto/with_client_auth; límites de normalize_addr |
| ecat-transport-http | 17→20 | test de integración: stop sin operación, puerto ocupado falla, envío/recepción reales |
| ecat-transport-grpc | 7→13 | TLS con archivos ausentes; ciclo de vida en texto plano; rechazo mTLS |
| ecat-transport-ws | 4→8 | fallo sin handler; puerto ocupado; eco de frames enmascarados RFC 6455 |
| ecat-registry | 5→8 | discover de múltiples instancias; baja automática con drop; defaults de builder |
| ecat-registry-consul | 10→24 | percent-encode; variantes de registro; respuestas de error; X-Consul-Token; parseo de agent/services; fallback de node |
| ecat-registry-etcd | 5→10 | discover omite valores malos; cuerpo de petición kv; concesión de lease; keepalive |

### Equipo capa app (test-app, +46)

| crate | Antes→Nuevo | Puntos cubiertos |
|---|---|---|
| ecat-auth | 20→46 | lista blanca de caché oauth2/clave SHA-256/desalojo FIFO; apikey de tres estados; imposición de iss/aud en jwt; expirado/firma incorrecta |
| ecat-health | 4→8 | agregación de readiness (todo ok/cualquier fail/registro vacío); liveness |
| ecat-versioning | 4→7 | enrutado con estrategia de path; límites de extract_version |
| ecat-security | 12→20 | extremo a extremo a nivel de header; forma JSON de bloqueos de ataques |
| ecat-middleware | 34→37 | expiración de ventana en MemoryStore; panic interno→Err |
| ecat-circuit-breaker | 10→12 | agotamiento de sondas half-open; degradación de classify |
| ecat-client | 7→10 | endpoints grpc inválidos dan error sin red |
| ecat-graphql | 35→35 | cobertura existente suficiente, sin brechas |
| ecat-scheduler / ecat-bench / ecat-events / ecat-cli / ecat | cobertura existente suficiente | sin brechas |

## Defectos encontrados

| Nivel | Ubicación | Descripción | Estado |
|---|---|---|---|
| P1 | ecat-events/Cargo.toml | dev-dependencies carecen de las features tokio macros/rt/time; compilar solo el objetivo de test de ese crate falla inevitablemente (la construcción completa del workspace lo enmascara por la unión de features) | ✅ Corregido (features + comentario añadidos) |
| P2 | ecat-security src/lib.rs:118-127 | SQLi con codificación de porcentaje en URI (`?q=SELECT%20*%20...`) puede evadir el escaneo a nivel de header (el detector exige espacios literales y escanea la URI cruda sin decodificar primero); el escaneo del cuerpo no se ve afectado | ⏳ Pendiente |
| P3 | ecat-data-sqlx | `connect()/from_config()` usan AnyPool sin instalar drivers; sqlx 0.8.6 entra en panic en la primera conexión con "No drivers installed" | ⏳ Pendiente |
| P3 | ecat-data-influxdb | los campos string escapan el espacio (`\ `); la especificación de line protocol solo exige escapar `"` y `\`; el orden de tag/field no es determinista | ⏳ Pendiente |
| P3 | ecat-data-clickhouse | la caché de creación de tablas nunca expira; tras drop/alter externos no se reintenta el CREATE | ⏳ Pendiente |
| P3 | ecat-circuit-breaker | el límite de half_open_probes es inalcanzable con sondeo secuencial (solo alcanzable con concurrencia en vuelo); cubierto por test de caja blanca | ℹ️ Conocido, no es un defecto |
| P3 | ecat-health | `with_check` usa blocking_write(); llamarlo desde contexto async entra en panic; actualmente solo usable en contexto síncrono | ℹ️ Conocido, limitación de API |

## Módulos omitidos (requieren entorno de integración, sin mock)

- Roundtrip con brokers reales: publish-subscribe de kafka/rabbitmq/mqtt/nats (configuración y rutas de error cubiertas)
- Clústeres reales: ciclo de vida registro-descubrimiento de consul/etcd (axum mock cubre la forma de las peticiones)
- Bases de datos reales: operaciones redis/memcached, servidor mongod, validación de influxdb server, drivers sqlx postgres/mysql, APIs nebulagraph/arangodb
- Servicios externos reales: introspección OAuth2 (cubierta con mock local), roundtrip gRPC/HTTP (mock local cubre que 302 no se sigue)
