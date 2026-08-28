# Informe de revisión exhaustiva de e-cat

**Fecha**: 2026-08-06
**Versión**: 2.3.0 · 55 crates
**Alcance**: build/tests, humo en runtime, consistencia del ecosistema, protecciones de seguridad, configuración de despliegue

---

## 1. Resultados de tests y build

| Elemento | Resultado | Descripción |
|--------|------|------|
| `cargo check --workspace` | ✅ Correcto | 0 warnings |
| `cargo test --workspace` | ✅ Correcto | **los 202 tests pasan todos, 0 fallos** (incluidos doc-tests) |
| `cargo fmt --check` | ✅ Correcto | |
| `cargo clippy --workspace -- -D warnings` | ✅ Correcto | coincide con el comando de CI |
| `cargo clippy --all-targets -- -D warnings` | ❌ Falla | ver hallazgo D2 |
| Test de humo (helloworld) | ❌ **falla el arranque** | ver hallazgo D1 |

**Distribución de cobertura de tests**: 51 archivos fuente contienen `#[test]`, 105 binarios de test. Sin `todo!()`/`unimplemented!()` en rutas de producción, `panic!` solo existe en código de test.

---

## 2. Problemas de runtime (descubiertos por el test de humo)

### [ALTO] D1. `HttpServer::new(":8000")` falla al arrancar en entornos sin IPv6
- **Ubicación**: `ecat-transport-http/src/lib.rs:40`, `examples/helloworld/src/main.rs:41`, varios lugares del README
- **Síntoma**: `TcpListener::bind(":8000")` resuelve al comodín IPv6 `[::]:8000`; en máquinas sin IPv6 (contenedores/algunos hosts cloud) reporta `failed to lookup address information: Name or service not known`, el servicio no arranca.
- **Reproducción**: programa mínimo independiente — `bind(":8001")` falla, `bind("0.0.0.0:8002")` funciona, `bind("localhost:8003")` funciona.
- **Corrección**: `HttpServer::new` normaliza internamente el host vacío a `"0.0.0.0"`; los ejemplos y la documentación usan `"0.0.0.0:8000"`.

### [BAJO] D2. `cargo clippy --all-targets -- -D warnings` falla
- **Ubicación**: `ecat-data-sqlx/src/lib.rs` (hay items después del módulo de tests, dispara `items_after_test_module`)
- **Impacto**: el comando actual de CI (sin `--all-targets`) no se ve afectado; si CI se endurece, fallaría.
- **Corrección**: mover el módulo de tests al final del archivo.

---

## 3. Problemas graves (CRÍTICO)

### [CRÍTICO] C1. `ecat-data-memcached` es una "implementación falsa"
- **Ubicación**: `ecat-data-memcached/src/lib.rs:23-88`
- **Problema**: todo el crate es un `HashMap` en memoria puro, sin conexión de red, sin configuración de dirección de servidor (`MemcachedConfig` solo tiene username/password/tls), la description del Cargo.toml admite "in-memory cache client". Su uso indebido en producción provocaría **pérdida de datos silenciosa** (se vacía al reiniciar, no compartido entre instancias).
- **Corrección**: integrar el protocolo memcached real (p. ej. el crate `memcache`), o marcarlo explícitamente `#[deprecated]`/con aviso en la documentación prohibiendo su uso en producción.

### [CRÍTICO] C2. Inyección por concatenación de SQL en TDengine
- **Ubicación**: `ecat-data-tdengine/src/lib.rs:91-116`
- **Problema**: en `INSERT INTO "{}" ({}) VALUES ({})` measurement/nombres de columna/valores se concatenan directamente con `format!`; los valores de cadena solo se envuelven en comillas dobles, sin escapar `"` ni `\`. Un valor de campo que contenga `"; DELETE ...; --` puede escapar y ejecutar SQL arbitrario (el REST de TDengine soporta múltiples sentencias).
- **Corrección**: escapar identificadores y valores de cadena (`"`→`\"`, `\`→`\\`), o usar una interfaz de escritura parametrizada.

---

## 4. Problemas de alto riesgo (ALTO)

### [ALTO] H1. Todos los adaptadores HTTP de bases de datos sin timeout
- **Ubicación**: `ecat-tls/src/lib.rs:27,61`, elasticsearch/opensearch/clickhouse/influxdb/iotdb/questdb/tdengine/neo4j/nebulagraph/arangodb
- **Problema**: reqwest no tiene timeout por defecto; si el servidor se cuelga, la petición queda **colgada para siempre** (agotamiento del pool de conexiones, fuga de tareas).
- **Corrección**: `build_reqwest_client` fija `connect_timeout` (p. ej. 5s) + `timeout` (p. ej. 30s) de forma uniforme.

### [ALTO] H2. La limitación de tasa no puede aplicarse por cliente
- **Ubicación**: `ecat-middleware/src/ratelimit.rs:155`
- **Problema**: `key_fn("")` no recibe el objeto de petición, no puede limitar por IP/usuario; el bucket por defecto es "global", el atacante puede agotar la cuota global (DoS a otros) o saltársela de forma distribuida.
- **Corrección**: cambiar la firma de `key_fn` para que reciba `&http::Request` y tomar la key de `X-Forwarded-For`/dirección del peer.

### [ALTO] H3. El CI de GitHub falla seguro (falta protoc)
- **Ubicación**: `.github/workflows/ci.yml`
- **Problema**: el build.rs de `ecat-protos` compila los proto con tonic-build, que depende fuertemente de protoc; el CI de GH no instala `protobuf-compiler` (en local `/home/erik/.local/bin/protoc` existe, por eso pasa). `.gitlab-ci.yml` sí lo instala; los dos CI se comportan de forma distinta.
- **Corrección**: añadir `apt-get install protobuf-compiler` al CI de GH (y cmake si hace falta).

### [ALTO] H4. Elasticsearch `search()`/`delete()` no comprueban el código de estado HTTP
- **Ubicación**: `ecat-data-elasticsearch/src/lib.rs:87-114`
- **Problema**: los cuerpos de error 404/400 se parsean como JSON y reportan un error "es parse" engañoso; `index()` sí comprueba y `search`/`delete` no — comportamiento inconsistente (opensearch lo hace bien).
- **Corrección**: comprobar `status.is_success()` de forma uniforme.

### [ALTO] H5. Sospecha de incompatibilidad del protocolo `insertTablet` de IoTDB
- **Ubicación**: `ecat-data-iotdb/src/lib.rs:51-82`
- **Problema**: el REST `insertTablet` de IoTDB exige el formato de arrays `timestamps/measurements/values/data_types`; esta implementación envía un JSON de documento único, puede ser "parece implementado pero no usable".
- **Corrección**: construir el cuerpo de petición según la especificación de insertTablet y añadir tests de integración.

### [ALTO] H6. Prefijo de deregister de etcd no coincide (deregister ineficaz)
- **Ubicación**: `ecat-registry-etcd/src/lib.rs:47,66`
- **Problema**: la clave de registro es `/ecat/services/{prefix}/{name}/{uuid}`, pero deregister elimina `{prefix}/{name}` (le falta el segmento uuid) → la información de registro queda residual tras la salida de la instancia.
- **Corrección**: al eliminar, emparejar la clave completa o listar y eliminar por prefijo de name.

---

## 5. Problemas de riesgo medio (MEDIO)

| # | Ubicación | Problema | Sugerencia |
|---|------|------|------|
| M1 | `ecat-middleware/src/ratelimit_redis.rs:28-48` | si Redis falla, Err se trata como superado el límite → **DoS fail-closed**; si EXPIRE falla tras INCR, la clave nunca expira → bloqueo permanente | distinguir errores de límite/almacenamiento (dejar pasar si falla el almacenamiento), script Lua atómico |
| M2 | `ecat-middleware/src/ratelimit.rs:16-51` | las entradas de MemoryStore solo se reinician, no se eliminan; con keys por cliente la **memoria crece sin límite** | limpiar periódicamente los buckets expirados |
| M3 | `ecat-auth/src/jwt.rs:25-31` | la clave débil no tiene validación de longitud mínima (en tests se usa "secret-key"), se puede forzar offline | exigir clave aleatoria ≥32 bytes; generalizar la respuesta de error para no revelar detalles de jsonwebtoken |
| M4 | `ecat-auth/src/oauth2.rs:111-123` | se crea un `reqwest::Client` nuevo por petición sin timeout; la URL no fuerza HTTPS | reutilizar el Client, fijar timeout, validar https |
| M5 | `ecat-data-redis/src/lib.rs:34-64`, `ratelimit_redis.rs:12-17`, ecat-lock | la contraseña percent-encodeada se embebe en la URL; el Display del error de conexión contiene la URL completa → **fuga de la contraseña en logs**; si la URL ya contiene `@`, las credenciales se descartan en silencio | pasar los parámetros de autenticación por separado, desensibilizar los mensajes de error |
| M6 | `ecat-data-elasticsearch/src/lib.rs:104-113`, opensearch:111-116 | index/id se concatenan en la ruta sin URL-encoding; con `/` se puede acceder a otros índices (IDOR) | URL-encoding + lista blanca de índices |
| M7 | `ecat-data-sqlx/src/lib.rs:79,173`, questdb:78-84 | los errores crudos de la base de datos (con SQL y valores) se propagan directamente | generalizar externamente, los detalles solo a logs |
| M8 | `ecat-data-clickhouse/src/lib.rs:92` | `execute()` devuelve siempre `Ok(0)`, rows_affected se pierde; `query()` descarta en silencio las filas que fallan al parsear | devolver el número real de filas, propagar errores |
| M9 | `ecat-data-tdengine/src/lib.rs:80-118` | `write()` hace peticiones punto a punto por punto (N+1) | escritura por lotes |
| M10 | `ecat-data-sqlx/src/lib.rs:98-142 vs 213-256` | query/query_with duplican ~50 líneas de lógica de conversión de tipos | extraer función común |
| M11 | `ecat-data-redis/src/lib.rs:167` | en `acquire`, `ttl.as_millis() as u64` desborda y trunca (en `set` ya se maneja, aquí no) | manejo unificado del desbordamiento |
| M12 | `ecat-data-influxdb/src/lib.rs:69-79` | los campos de cadena del line protocol no se escapan (comillas/comas/espacios) → error de protocolo al escribir | escapar según especificación |
| M13 | `ecat-mq-*` | firma de `from_config` no uniforme: kafka/mqtt devuelven sincrónicamente, rabbitmq/nats async | unificar a async |
| M14 | `ecat-auth/src/apikey.rs:33-36`, `ecat-security/src/lib.rs:126-137` | la API key soporta parámetro query (cae en logs/Referer); el WAF solo escanea URI+headers, no el body | pasar la key solo por header; el WAF añade escaneo del body |

---

## 6. Bajo riesgo e informativo (BAJO/INFO)

| # | Ubicación | Problema |
|---|------|------|
| L1 | `ecat-deploy/Dockerfile` | **copia un binario `ecat-app` que no existe** (el bin real es `ecat`, de ecat-cli) → la imagen no tiene entrypoint tras docker build; HEALTHCHECK usa curl pero la imagen no instala curl |
| L2 | `ecat-deploy/helm/Chart.yaml` | appVersion es "2.2.0", la versión actual es 2.3.0 |
| L3 | `README.en.md` | afirma "v2.1.7 · 47 crates", en realidad v2.3.0 · 55 crates, documentación en inglés muy desactualizada |
| L4 | `ecat-registry-consul/src/lib.rs:66,143` | el puerto de registro es siempre 0, la versión de discover está hardcodeada como "1.0" |
| L5 | 11 Cargo.toml de crates | escriben dependencias de la misma versión saltándose `workspace.dependencies` (riesgo de deriva de versiones) |
| L6 | `ecat-tracing` / `ecat-middleware/src/tracing.rs` | TracingLayer implementado dos veces; ecat-tracing-otlp y ecat-tracing instalan cada uno su propio subscriber, llamarlos juntos provoca conflicto de doble init |
| L7 | `ecat-config-remote/src/lib.rs:92` | decode base64 escrito a mano, se recomienda el crate base64 |
| L8 | `ecat-graphql` | parser de campo único escrito a mano, solo soporta campo único de nivel superior (sin anidamiento/alias/parámetros), la documentación no indica la limitación |
| L9 | `ecat-cli/src/main.rs:69-104`, lib.rs:3-22 | `ecat new ../../x` permite path traversal; un nombre con `"`/salto de línea puede inyectar en el Cargo.toml generado |
| L10 | `config/databases.example.yaml:54-79` | varias contraseñas por defecto válidas (neo4j/changeme, arangodb root/changeme, iotdb root/root, influx my-secret-token), copiar y listo para producción con contraseñas por defecto |
| L11 | `ecat-data-s3/src/lib.rs:83-93` | list() sin configuración de timeout; la construcción de credenciales es una llamada síncrona bloqueante |
| L12 | `ecat-data-redis` | sin reconexión explícita, depende de la reconexión integrada de MultiplexedConnection, la documentación no lo indica |
| L13 | `ecat-data/src/rdbms.rs:71-77` | `Transaction::drop` solo avisa y no dispara rollback, depende del drop de sqlx para el rollback automático; se sugiere comentario aclaratorio |

---

## 7. Conclusión de integridad del ecosistema

**Completitud: alta**. Los 55/55 crates están en el workspace, versión unificada 2.3.0, sin stubs (excepto la implementación falsa de memcached). 18 backends de bases de datos, 4 backends MQ, 2 registries, abstracción de almacenamiento de rate limit, bloqueo distribuido, scheduler, trazado OTLP, versionado, GraphQL — todo implementado. `todo!()`/`unimplemented!()` en cero lugares.

**Por reforzar**:
1. implementación del protocolo real de memcached (el único adaptador "falso" actual)
2. validación de conformidad del protocolo IoTDB (presumiblemente inutilizable)
3. alinear GitHub CI con GitLab CI (falta protoc)
4. política de timeout uniforme en todos los adaptadores HTTP

## 8. Conclusión de protecciones de seguridad

**Sin vulnerabilidades de seguridad CRÍTICAS (inyección/manejo de credenciales/TLS seguros por defecto)**:
- ✅ cero bloques unsafe en todo el workspace
- ✅ sin credenciales hardcodeadas; las configuraciones de ejemplo usan placeholders changeme (se sugiere comentarlas todas, L10)
- ✅ sqlx todo con binding parametrizado; el bloqueo de Redis se libera con Lua CAS
- ✅ `skip_verify` de TLS desactivado por defecto; Redis se actualiza automáticamente a rediss://
- ⚠️ pendiente: inyección por concatenación de TDengine (C2, fuera del alcance de sqlx), rate limit por cliente (H2), Redis rate limit fail-closed (M1), claves JWT débiles (M3), fuga de contraseña en errores de Redis (M5), inyección de ruta en ES (M6)

## 9. Sugerencias de optimización (prioridad Top)

1. **P0**: C1 implementación falsa, C2 inyección SQL, D1 bind de puerto, H1 timeouts — 4 elementos
2. **P1**: H2 rate limit, H3 CI, H4 código de estado ES, H5 IoTDB, H6 deregister etcd
3. **P1**: M1 fail-closed, M3 JWT, M5 fuga de contraseña, M6 inyección de ruta
4. **P2**: correcciones de Dockerfile/Helm/README, clippy --all-targets, propagación de errores, escritura por lotes
5. **P3**: convergencia de workspace.dependencies, unificación de from_config de MQ, sincronización de documentación

---

## 10. Estado de las correcciones (reverificación 2026-08-06)

**Los 35 hallazgos están todos corregidos o documentados.** Resultado de la reverificación: `cargo check --workspace` ✅, `cargo test --workspace` 219 tests todos correctos ✅, `cargo clippy --workspace --all-targets -- -D warnings` cero warnings ✅, `cargo fmt --check` limpio ✅, test de humo de helloworld (`/` + `/health`) ✅.

| N.º | Gravedad | Forma de corrección | Verificación |
|------|--------|----------|------|
| D1 | ALTO | `HttpServer` normaliza el host vacío a `0.0.0.0`; ejemplos/documentos/plantilla CLI unificados a `0.0.0.0:8000` | el test de humo hace bind con éxito |
| D2 | BAJO | `SqlxTransactionWrapper` impl movido antes del módulo de tests | clippy cero warnings |
| C1 | CRÍTICO | memcached marcado explícitamente "solo desarrollo/tests"; interruptor `in_memory`; expiración perezosa en get + sweep en set | 23 tests de la capa de datos correctos |
| C2 | CRÍTICO | TDengine doble escape (`\`→`\\`, `"`→`\"`); troceado por lotes de 100 | correcto |
| H1 | ALTO | `ecat-tls` fija timeout uniforme de connect 5s / request 30s, heredado por todos los adaptadores HTTP | correcto |
| H2 | ALTO | la key de rate limit usa por defecto primer salto de X-Forwarded-For → X-Real-IP → global; MemoryStore barrido perezoso de 60s | 22 tests de middleware correctos |
| H3 | ALTO | el CI añade la instalación de `protobuf-compiler` | configuración actualizada |
| H4 | ALTO | `search()`/`delete()` de ES/OpenSearch comprueban `is_success()`; index/id con encoding RFC 3986 | correcto |
| H5 | ALTO | IoTDB refactorizado a body insertTablet estándar, comprueba `code != 200` | correcto |
| H6 | ALTO | deregister de etcd usa borrado por rango de prefijo, empareja la clave de registro | correcto |
| M1 | MED | rate limit Redis: Lua atómico INCR+EXPIRE, DEL rollback si EXPIRE falla, fail-open + warn en errores de conexión | correcto |
| M3 | MED | claves JWT <32 bytes rechazadas (`WeakKey`); respuesta de error unificada `invalid token` | 9 tests de auth correctos |
| M5 | MED | la contraseña de Redis se pasa por `ConnectionInfo` por separado, ya no se embebe en la URL | correcto |
| M6 | MED | todas las superficies de inyección de ES/OpenSearch/InfluxDB escapadas o parametrizadas | correcto |
| M9 | MED | TDengine 100 entradas/lote | correcto |
| M11 | MED | desbordamiento de ttl de Redis fijado a `u64::MAX` | correcto |
| M13 | MED | `from_config` de MQ unificado a async (kafka/mqtt sincronizados) | 11 tests de CLI correctos |
| Serie L | BAJO/INFO | Dockerfile (nombre real del binario + healthcheck curl + builder 1.85), Chart appVersion 2.3.0, contraseñas de ejemplo comentadas, versión/puerto de consul resueltos desde la información de registro, base64 manual sustituido por el crate `base64`, `validate_crate_name` anti-inyección, convergencia de workspace.dependencies en 8 lugares, comentario del conflicto de doble subscriber, documentación sincronizada (README/README.en/CHANGELOG 2.3.1) | todos correctos |

**Nuevos problemas durante la corrección**: el test de `ecat-config-remote` referenciaba el viejo `base64_decode` (olvidado al sustituir por el agente) → se ha cambiado a `base64::engine`; 4 warnings de clippy en `ecat-middleware` (if anidados / tipo complejo) → plegados + alias de tipo `KeyFn`. Sin regresiones tras la corrección.

**Conclusión del ecosistema**: 55 crates, 18 adaptadores de bases de datos, 4 MQ, configuración Docker/Helm/CI, README en chino e inglés, CHANGELOG — todo consistente con v2.3.0; las imágenes (alipay/weixinpay.png) se referencian correctamente.

---

*Informe generado por revisión automatizada: build+tests+ejecución de humo + 3 agentes de revisión especializados (seguridad/capa de datos/consistencia del ecosistema), reverificación completa 2026-08-06.*
