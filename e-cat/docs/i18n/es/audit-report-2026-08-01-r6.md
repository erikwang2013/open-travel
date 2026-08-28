# Informe de auditoría profunda de e-cat — 2026-08-01 R6

## Evaluación general

| Dimensión | Estado | Descripción |
|------|------|------|
| Compilación | Correcta | 50 crates, cero errores |
| Tests | Correctos | todos correctos, cero fallos |
| Clippy | Correcto | cero warnings (`-D warnings`) |
| unsafe | cero | la base de código no tiene bloques unsafe |
| Tamaño de archivos | Bien | solo `ecat-auth` (540 líneas) supera el valor recomendado de 500 líneas |

## Hallazgos (15)

### Relacionados con seguridad

#### 1. [Grave] El «cifrado» XOR no es cifrado real
**Archivo:** `ecat-config/src/encrypted.rs:45-56`
**Problema:** `decrypt()` usa XOR + clave repetida; esto es ofuscación, no cifrado, y se puede romper con facilidad. La clave se reutiliza en cada posición de byte, haciendo que el texto cifrado sea muy vulnerable al análisis de frecuencias.
**Recomendación:** sustituir por AES-256-GCM (crate `aes-gcm`), o etiquetarlo explícitamente como «ofuscación» en lugar de «cifrado».

#### 2. [Grave] La implementación por defecto de `execute_with`/`query_with` descarta parámetros en silencio
**Archivo:** `ecat-data/src/rdbms.rs:86-103`
**Problema:** la implementación por defecto del trait recibe los parámetros pero los ignora (`let _ = params;`), llamando directamente al `execute(sql)` original. Todos los backends excepto `ecat-data-sqlx` (ClickHouse, QuestDB) heredan este comportamiento. Si el usuario sustituye el backend con métodos parametrizados, los parámetros se descartan en silencio, provocando vulnerabilidades de inyección SQL.
**Recomendación:** la implementación por defecto debería devolver un error de «no soportado», o cada backend debería implementar el binding de parámetros correctamente.

#### 3. [Alto] Contraseñas embebidas en claro en la URL
**Archivo:** `ecat-data-sqlx/src/lib.rs:40`, `ecat-data-redis/src/lib.rs:43`
**Problema:** `connect_with_auth()` usa `replacen("://", "://user:pass@")` para incrustar las credenciales directamente en la URL. Estas URLs pueden quedar registradas en logs, mensajes de error o salidas de depuración.
**Recomendación:** usar los mecanismos de autenticación nativos de cada backend; o al menos codificar en URL el usuario/contraseña antes de la concatenación.

#### 4. [Medio] La configuración TLS fallida provoca panic
**Archivo:** 8 crates data-* (ClickHouse, QuestDB, Elasticsearch, OpenSearch, ArangoDB, Neo4j, NebulaGraph, InfluxDB, IoTDB)
**Patrón:** `.expect("TLS client build failed")` — todos los constructores `from_config()` entran en panic si la configuración TLS es incorrecta.
**Recomendación:** cambiar `from_config()` para que devuelva `Result`, o hacer que la construcción del cliente TLS sea perezosa/tolerante a fallos.

### Corrección funcional

#### 5. [Alto] El enrutado Header de `ecat-versioning` no funciona
**Archivo:** `ecat-versioning/src/lib.rs:56-64`
**Problema:** `build_header_router()` anida todas las versiones bajo la misma ruta `/api`, pero no filtra por header de versión. axum registra todas las versiones en la misma ruta, lo que produce conflictos de ruta y comportamiento impredecible. La función `extract_version()` existe pero nunca se usa en el enrutado.
**Recomendación:** usar middleware/layer de axum que compruebe el header Accept y enrute a la ruta de la versión correcta, en lugar de aplanar todas las versiones en la misma ruta.

#### 6. [Medio] Truncamiento de TTL en Redis: las expiraciones sub-segundo se vuelven permanentes
**Archivo:** `ecat-data-redis/src/lib.rs:76-77`
**Problema:** `Duration::as_secs()` trunca hacia cero. Un TTL de 500ms con `secs == 0` se convierte silenciosamente en expiración permanente, tomando la rama `SET` en lugar de `SETEX`.
**Recomendación:** para TTL sub-segundo, fijar al menos 1 segundo, o usar `SET ... PX` (milisegundos) en lugar de `SETEX`.

#### 7. [Medio] `StaticResolver::add_service` entra en panic ante contención de bloqueo
**Archivo:** `ecat-client/src/lib.rs:27-29`
**Problema:** usa `try_write()` con expect; si existe cualquier otro poseedor del lock de escritura, entra en panic. El patrón builder hace difícil de disparar este problema, pero es una bomba de relojería en código concurrente.
**Recomendación:** usar `blocking_write()` (si se está en contexto síncrono) o cambiar a aceptar `&mut self` para evitar la necesidad del lock.

### Calidad de código

#### 8. [Medio] Uso de `std::sync::Mutex` en contexto asíncrono
**Archivo:** `ecat-data-memcached/src/lib.rs:7,24`
**Problema:** `std::sync::Mutex` usado en implementaciones de traits async. Aunque el lock se mantiene muy poco tiempo (solo operaciones de HashMap), en teoría podría bloquear el runtime asíncrono bajo alta contención.
**Recomendación:** para este caso de uso específico de caché en memoria, dado que la sección crítica es extremadamente corta y no tiene puntos `.await`, usar `std::sync::Mutex` es en realidad aceptable. Pero si en el futuro se necesita hacer I/O dentro del lock, habría que cambiar a `tokio::sync::Mutex`.

#### 9. [Bajo] Implementación manual de base64
**Archivo:** `ecat-registry-etcd/src/lib.rs:148-193`
**Problema:** ~45 líneas de códec base64 escrito a mano, con posibles bugs en casos límite. El ecosistema Rust tiene alternativas bien revisadas como el crate `base64`.
**Recomendación:** sustituir por el crate `base64` para reducir la carga de mantenimiento y los bugs potenciales.

#### 10. [Bajo] `RandomBalancer` no es aleatorio
**Archivo:** `ecat-client/src/lib.rs:91-105`
**Problema:** usa el hash de `Instant::now()` como fuente de aleatoriedad. Las llamadas simultáneas dentro de la misma instancia obtienen la misma elección «aleatoria». `checked_add(0)` es una operación superflua.
**Recomendación:** usar el crate `rand`, o al menos `std::collections::hash_map::RandomState`.

#### 11. [Bajo] `Arc<Vec<String>>` innecesario en `ecat-data-sqlx`
**Archivo:** `ecat-data-sqlx/src/lib.rs:79-87, 197-203`
**Problema:** los nombres de columna se envuelven en `Arc<Vec<String>>`, pero cada constructor de `Row` clona toda la lista de nombres de columna (`(*cols).clone()`). El `Arc` solo se usa una vez durante la iteración; bastaría `Rc` o un `clone()` directo.
**Recomendación:** en `query()` y `query_with()`, sustituir `Arc<Vec<String>>` por un `Vec<String>` normal. El coste del clone por fila es el mismo que desreferenciar el Arc + clonar.

### Diseño/arquitectura

#### 12. [Informativo] QuestDB usa GET + parámetros de consulta
**Archivo:** `ecat-data-questdb/src/lib.rs:76, 91`
**Problema:** el SQL se envía mediante parámetros de consulta GET, sujeto al límite de longitud de URL (normalmente ~2000-8000 caracteres). Las consultas grandes se truncan.
**Recomendación:** cambiar a POST + body, o mantener GET para consultas simples y usar POST para las complejas.

#### 13. [Informativo] `#[allow(dead_code)]` dispersos
**Archivo:** `ecat-registry-consul/src/lib.rs:225`, `ecat-data-memcached/src/lib.rs:25-28`, `ecat-auth/src/lib.rs:52`
**Problema:** los campos username/password se almacenan en memoria pero están marcados como dead_code (no necesarios en memcached en memoria; la variante RSA de auth aún no está implementada).
**Recomendación:** implementar las rutas de funcionalidad que faltan, o eliminar estos campos, o añadir documentación que explique por qué se conservan.

#### 14. [Informativo] A algunos clientes HTTP les falta el header Content-Type
**Archivo:** `ecat-data-influxdb/src/lib.rs:96-103`, `ecat-data-clickhouse/src/lib.rs:87-89`
**Problema:** algunas peticiones POST no establecen el header `Content-Type`, dependiendo de la detección automática del servidor.
**Recomendación:** establecer siempre un Content-Type explícito para garantizar la compatibilidad.

#### 15. [Informativo] `ecat-auth` supera las 500 líneas
**Archivo:** `ecat-auth/src/lib.rs` (540 líneas)
**Problema:** CLAUDE.md exige mantener los archivos por debajo de 500 líneas. El crate auth es el único que supera este límite.
**Recomendación:** dividir la lógica de verificación JWT a `ecat-auth/src/jwt.rs`, o dividir por funcionalidad.

## Oportunidades de optimización (no bugs)

| # | Ubicación | Sugerencia |
|---|------|------|
| O1 | todos los crates data-* | el patrón repetido de construcción de cliente TLS en todos los `from_config()` se puede extraer a una macro o función compartida |
| O2 | `ecat-data-sqlx` | la lógica de conversión de tipos de filas de `query()` y `query_with()` (117 líneas duplicadas) se puede extraer a una función auxiliar |
| O3 | `ecat-client` | `HttpClient::get()` y `post()` comparten el mismo pipeline «resolve → pick → build URL» — se puede extraer |
| O4 | `ecat-data` | los tipos de error personalizados de los 5 traits (Rdbms/Cache/Graph/Search/Tsdb) se pueden unificar en un único enum `DataError` |
| O5 | `ecat-data-redis` | el `self.conn.clone()` de cada método es innecesario — `MultiplexedConnection` ya está diseñada con `Clone` para soportar el uso compartido |

## Resumen de métricas

| Métrica | Valor |
|------|------|
| Total de crates | 50 |
| Líneas totales de archivos fuente Rust | 7,968 |
| `expect()` en código no-test | 12 |
| `unwrap()` en código no-test | 0 |
| Bloques `unsafe` | 0 |
| `panic!` en código no-test | 0 |
| `#[allow(dead_code)]` | 4 |
| TODO/FIXME/HACK | 0 |
| Mutex std en código async | 1 (memcached) |

## Conclusión

La base de código está en buen estado: compilación, tests y clippy pasan todos, sin código unsafe, sin macros de panic. Los dos problemas más críticos son el **«cifrado» XOR** (seguridad falsa) y **la implementación por defecto de consultas parametrizadas que descarta parámetros en silencio** (vulnerabilidad de seguridad). El enrutado por Header tampoco funciona en absoluto. Los demás problemas son relativamente menores y pertenecen al plano de la mantenibilidad.

**Orden de corrección recomendado:**
1. implementación por defecto de `execute_with`/`query_with` → devolver error en lugar de descartar parámetros en silencio
2. cifrado XOR → AEAD real, o renombrarlo a «ofuscación»
3. enrutado por versión Header → implementar el enrutado por header real
4. `from_config()` → devolver Result en lugar de expect-panic
5. truncamiento de TTL de Redis → los TTL sub-segundo deben usar al menos 1 segundo

## Estado de las correcciones (R6 → R6.1)

| # | Problema | Estado | Cambio |
|---|------|------|------|
| 1 | «cifrado» XOR | corregido | `EncryptedSource` → `ObfuscatedSource`, `decrypt` → `deobfuscate`, prefijo `enc:` → `obfs:`, documentación añadida explicando que es ofuscación, no cifrado |
| 2 | `execute_with`/`query_with` descartan parámetros en silencio | corregido | la implementación por defecto devuelve el error `"parameterized ... not supported by this backend"` |
| 3 | contraseñas embebidas en claro en la URL | corregido | `percent_encode()` codifica las credenciales en `connect_with_auth` |
| 4 | panic del `expect()` TLS | corregido | `from_config()` de 9 crates pasa a devolver `Result`; nueva variante `Config` en `RdbmsError` |
| 5 | enrutado Header inoperante | corregido | middleware `from_fn_with_state` implementa la verificación de versiones; test nuevo `header_versioned_router_builds` |
| 6 | truncamiento de TTL de Redis | corregido | `set_ex` → `pset_ex`, usando precisión de milisegundos para evitar que los TTL sub-segundo se trunquen a expiración permanente |
| 7 | panic por contención de lock en `StaticResolver` | corregido | `try_write()` → `blocking_write()` |
| 8 | `RandomBalancer` no aleatorio | corregido | `RandomState::new().build_hasher()` sustituye al hash de `Instant::now()` |
| 9 | `std::sync::Mutex` en contexto async | corregido | sustituido por `tokio::sync::Mutex` |
| 10 | base64 manual | corregido | sustituido por el crate `base64` 0.22 |
| 11 | sobrecoste de `Arc<Vec<String>>` | corregido | sustituido por `Vec<String>` normal, eliminada la envoltura Arc innecesaria |
| 12 | QuestDB envía SQL por GET | corregido | cambio a POST + body, con header Content-Type |
| 13 | `#[allow(dead_code)]` | corregido | campos de memcached con prefijo `_`; campos de consul con prefijo `_` y allow eliminado; `Rsa` → `RsaReserved` en auth |
| 14 | falta de Content-Type | corregido | headers Content-Type explícitos en las peticiones de InfluxDB, ClickHouse, IoTDB |
| 15 | `ecat-auth` supera las 500 líneas | corregido | dividido en `claims.rs`(31) + `jwt.rs`(139) + `apikey.rs`(96) + `oauth2.rs`(173) + `helpers.rs`(28) + `lib.rs`(98) |

### Crates afectados

| Crate | Tipo de cambio |
|-------|----------|
| `ecat-data` | implementaciones por defecto de traits, variante `RdbmsError::Config` |
| `ecat-config` | `EncryptedSource` → `ObfuscatedSource` |
| `ecat-versioning` | implementación del middleware de enrutado por Header |
| `ecat-data-redis` | TTL con precisión de milisegundos, codificación URL de credenciales |
| `ecat-data-sqlx` | codificación URL de credenciales, eliminado el sobrecoste de Arc |
| `ecat-data-clickhouse` | `from_config` → `Result`, header Content-Type |
| `ecat-data-questdb` | `from_config` → `Result`, GET → POST |
| `ecat-data-elasticsearch` | `from_config` → `Result` |
| `ecat-data-opensearch` | `from_config` → `Result` |
| `ecat-data-arangodb` | `from_config` → `Result` |
| `ecat-data-neo4j` | `from_config` → `Result` |
| `ecat-data-nebulagraph` | `from_config` → `Result` |
| `ecat-data-influxdb` | `from_config` → `Result`, header Content-Type |
| `ecat-data-iotdb` | `from_config` → `Result`, header Content-Type |
| `ecat-data-memcached` | `std::sync::Mutex` → `tokio::sync::Mutex`, limpieza de dead_code |
| `ecat-client` | correcciones de `StaticResolver`, `RandomBalancer` |
| `ecat-registry-etcd` | base64 sustituido por crate |
| `ecat-registry-consul` | limpieza de dead_code |
| `ecat-auth` | dividido en 6 módulos, limpieza de dead_code |

### Verificación final (R6.2)

| Dimensión | Estado |
|------|------|
| Build | Correcto, cero errores cero warnings |
| Test | todos correctos, cero fallos |
| Clippy (`-D warnings`) | Correcto, cero warnings |
| Tamaño de archivos | todos ≤ 300 líneas |
