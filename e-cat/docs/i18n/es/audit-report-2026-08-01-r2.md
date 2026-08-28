# Informe de auditoría del framework e-cat R2 — 2026-08-01

**Versión**: 1.0.5
**Alcance**: los 18 sub-crates
**Conclusión**: `cargo check` / `cargo clippy --all-features` / `cargo test` todos correctos, 70 tests ✅

---

## 1. Repaso de las correcciones anteriores (16/16 corregidas)

Todos los problemas encontrados en la auditoría anterior (R1) están corregidos: SecurityLayer bloquea ataques, soporte prost de ProtoCodec, cierre elegante del servidor, recogida de JoinHandle, implementación de Transaction, detección segura en el Drop de Registration, mejora del mapeo de tipos de columna, generación de archivos del CLI new, unificación de versión/edition, manejo de errores de FileSource, métodos de metadatos de Context, optimización de Arc en discover, optimización de Arc de columns en query, nuevo RateLimitLayer.

---

## 2. Problemas nuevos de esta ronda

### 2.1 [Grave] El código de plantilla generado por el CLI `new` no compila

- **Archivo**: `ecat-cli/src/main.rs:79-97`
- **Problema**: el `Cargo.toml` generado usa referencias de dependencias con `workspace = true` y rutas relativas `path = "../ecat"`, pero el proyecto independiente creado por `ecat new myapp` no está dentro del workspace de e-cat; todas esas referencias fallan al resolver
- **Impacto**: el proyecto creado por `ecat new` no compila en absoluto
- **Corrección**: la plantilla debe usar dependencias reales con número de versión, no referencias del workspace

```toml
# 当前（错误）：
tokio.workspace = true           # 项目不在 workspace 中，报错
ecat = { path = "../ecat" }      # 相对路径无效

# 应改为：
tokio = { version = "1", features = ["full"] }
ecat = "1.0.5"
```

### 2.2 [Grave] `transaction()` de ecat-data-sqlx descarta el manejador de transacción real de la base de datos

- **Archivo**: `ecat-data-sqlx/src/lib.rs:100-106`
- **Problema**: `pool.begin()` devuelve el manejador real de transacción de la base de datos `Transaction<'_, DB>`, pero el código lo vincula como `_tx` y lo descarta de inmediato. Cuando `_tx` se dropea, la transacción de la base de datos hace rollback automático. La `ecat_data::Transaction` devuelta es un cascarón; sus métodos `commit()/rollback()` no tienen ningún efecto
- **Impacto**: todo el código que usa `transaction()` se ejecuta sin protección de transacción; no se puede garantizar la consistencia de datos
- **Corrección**: rediseñar la estructura `ecat_data::Transaction` para que contenga el manejador real de transacción de la base de datos

### 2.3 [Medio] SecurityLayer no escanea el cuerpo de la petición

- **Archivo**: `ecat-security/src/lib.rs:117-127`
- **Problema**: `call()` solo escanea la URI y las cabeceras HTTP; no comprueba en absoluto el cuerpo de la petición. Un atacante puede poner el payload de inyección SQL/XSS en el body POST y evadir la detección con facilidad
- **Impacto**: reduce drásticamente la cobertura efectiva de la detección de ataques
- **Corrección**: añadir capacidad de escaneo del body, o proporcionar un método público `scan_body()` para que el llamador lo use tras leer el body

### 2.4 [Medio] RateLimitLayer usa Mutex síncrono + sin limpieza de expiración

- **Archivo**: `ecat-middleware/src/ratelimit.rs:10-38`
- **Problema 1**: `std::sync::Mutex` usado en contexto async — si hay contención de bloqueo, bloquea todo el hilo worker de tokio
- **Problema 2**: `buckets: HashMap<String, (u32, Instant)>` nunca limpia las claves expiradas; la memoria de un servidor de larga duración crece sin límite (cada IP/clave nueva ocupa memoria permanentemente)
- **Impacto**: degradación de rendimiento bajo alta concurrencia; fuga de memoria en ejecuciones largas
- **Corrección**: cambiar a `tokio::sync::Mutex` y limpiar periódicamente las entradas expiradas en `allow()`

### 2.5 [Medio] SQL crudo de ecat-data-sqlx sin API de parametrización

- **Archivo**: `ecat-data-sqlx/src/lib.rs:24-29, 32-36`
- **Problema**: `execute(&self, sql: &str)` y `query(&self, sql: &str)` solo aceptan cadenas SQL crudas; el trait no tiene métodos de binding de parámetros. Si el llamador concatena entrada de usuario en el SQL, se produce inyección SQL
- **Impacto**: aunque el trait no expone directamente la vulnerabilidad, la falta de API parametrizada induce al llamador a escribir código inseguro
- **Recomendación**: añadir métodos `execute_with` y `query_with` al trait `RdbmsClient` para usar binding de parámetros

### 2.6 [Bajo] Arc::clone de query() sigue dentro de la clausura

- **Archivo**: `ecat-data-sqlx/src/lib.rs:50-53`
- **Problema**: `let cols = std::sync::Arc::clone(&columns)` se ejecuta dentro de la clausura de `rows.iter().map()`. Aunque Arc::clone es muy ligero (solo incremento atómico del contador de referencias), se puede sacar fuera de la clausura para evitar una operación atómica por fila
- **Recomendación**: hacer un clone antes de `iter()` y capturar ese clone en la clausura

### 2.7 [Bajo] El impl de trait de ProtoCodec es inconsistente con la nueva API

- **Archivo**: `ecat-encoding/src/proto.rs`
- **Problema**: `encode/decode` del trait `Codec` siguen devolviendo solo errores; los nuevos `encode_message/decode_message` son la ruta correcta pero el nombre del método no coincide con el trait. El usuario puede intentar `codec.encode()` primero y preguntarse por qué falla
- **Recomendación**: explicar en la documentación/comentarios que los tipos proto deben usar `encode_message/decode_message` en lugar de los métodos del trait Codec

---

## 3. Estado actual general

| Dimensión | Estado |
|------|------|
| `cargo check` | ✅ cero warnings |
| `cargo clippy --all-features` | ✅ cero advertencias |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 correctos |
| Versión unificada | ✅ 1.0.5 |
| Edition unificada | ✅ 2024 |

### Distribución de tests

| Crate | Tests | Descripción |
|-------|-------|------|
| ecat | 4 | ✅ |
| ecat-config | 9 | ✅ |
| ecat-encoding | 15 | ✅ |
| ecat-errors | 4 | ✅ |
| ecat-logging | 1 | ✅ |
| ecat-metadata | 9 | ✅ |
| ecat-metrics | 2 | ✅ |
| ecat-middleware | 4 | ✅ (incluye RateLimitLayer) |
| ecat-registry | 5 | ✅ |
| ecat-security | 6 | ✅ |
| ecat-transport | 11 | ✅ |
| ecat-data | 0 | — (solo definiciones de trait) |
| ecat-data-sqlx | 0 | ⚠️ sin tests de integración DB |
| ecat-protos | 0 | — (código generado) |
| ecat-transport-grpc | 0 | ⚠️ |
| ecat-transport-http | 0 | ⚠️ |
| ecat-cli | 0 | ⚠️ |

---

## 4. Prioridades de problemas

| # | Gravedad | Problema | Archivo | Impacto en el usuario |
|---|--------|------|------|----------|
| 1 | 🔴 | la plantilla generada por el CLI `new` no compila | `ecat-cli/src/main.rs:79` | el primer comando de un nuevo usuario ya falla |
| 2 | 🔴 | transaction() descarta el manejador real de transacción DB | `ecat-data-sqlx/src/lib.rs:100` | consistencia de datos sin garantía |
| 3 | 🟠 | SecurityLayer no escanea el body | `ecat-security/src/lib.rs:117` | los atacantes pueden evadir la detección |
| 4 | 🟠 | RateLimitLayer con Mutex std + fuga de memoria | `ecat-middleware/src/ratelimit.rs:10,25` | rendimiento concurrente + OOM |
| 5 | 🟠 | SQL crudo sin API de parametrización | `ecat-data-sqlx/src/lib.rs:24` | riesgo de inyección SQL |
| 6 | 🟡 | posición del Arc clone en query() | `ecat-data-sqlx/src/lib.rs:53` | micro-optimización de rendimiento |
| 7 | 🟡 | API de ProtoCodec inconsistente | `ecat-encoding/src/proto.rs` | confusión del usuario |

---

## 6. Registro de correcciones (2026-08-01 R2)

| # | Problema | Forma de corrección | Estado |
|---|------|----------|------|
| 1 | la plantilla del CLI new no compila | dependencias con versión (`ecat = "1.0"`, `tokio = "1"`, etc.) | ✅ |
| 2 | transaction() descarta la transacción DB | `Transaction::with_inner()` contiene el manejador real; sqlx lo pasa mediante `Box<dyn Any>` | ✅ |
| 3 | SecurityLayer no escanea el body | nuevo método público `scan_body(&[u8])` | ✅ |
| 4 | Mutex de RateLimitLayer + fuga | `tokio::sync::Mutex` + limpieza de entradas expiradas cada 100 claves | ✅ |
| 5 | SQL crudo sin API parametrizada | nuevos métodos `execute_with`/`query_with` en `RdbmsClient` | ✅ |
| 6 | posición del Arc clone en query() | `Arc::clone` movido fuera de `iter()`; todas las filas comparten la referencia | ✅ |
| 7 | API de ProtoCodec inconsistente | documentación a nivel de módulo + documentación de la struct explicando el uso | ✅ |

### Estado final

| Comprobación | Resultado |
|--------|------|
| `cargo check` | ✅ cero errors / cero warnings |
| `cargo clippy --all-features` | ✅ cero warnings |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 correctos |
| Versión | 1.0.5 (todo unificado con herencia del workspace) |
| Edition | 2024 |
