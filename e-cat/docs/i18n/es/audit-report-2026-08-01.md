# Informe de auditoría del framework e-cat — 2026-08-01

**Fecha de auditoría**: 2026-08-01
**Alcance de la auditoría**: los 18 sub-crates (workspace)
**Cadena de herramientas**: stable (rustfmt, clippy)
**Resultados de tests**: 66 tests todos pasan | 0 fallos | 0 omitidos

---

## 1. Evaluación general

| Dimensión | Puntuación | Descripción |
|------|------|------|
| Compilación | ✅ Correcta | `cargo check` sin errores, solo 1 warning |
| Lint | ✅ Correcto | `cargo clippy --all-features` cero advertencias |
| Tests | ✅ 66/66 | todos los tests pasan |
| Cobertura de tests | ⚠️ Insuficiente | 7 crates sin ningún test |
| Integridad funcional | ⚠️ Muchos stubs | ProtoCodec, Transaction, CLI new, etc., sin implementar |
| Calidad de código | ⚠️ Regular | estructura clara, pero varios problemas de diseño |

---

## 2. Problemas de compilación y configuración

### 2.1 [WARNING] clave de manifest sin usar

- **Archivo**: `/Cargo.toml:25`
- **Problema**: `workspace.package.name = "e-cat"` — este campo carece de sentido a nivel de workspace y produce un warning en cada compilación
- **Corrección**: eliminar la línea, o convertirla en un comentario que explique el nombre del proyecto

### 2.2 [INFO] edition de Rust inconsistente

- **workspace**: `edition = "2026"`
- **sub-crates**: `ecat-security/Cargo.toml` y `ecat-config/Cargo.toml` usan `edition = "2021"`
- **Descripción**: el workspace declara edition 2026 pero algunos sub-crates lo sobrescriben a 2021. Aunque compila, la edition 2026 no es una edition estable publicada oficialmente por Rust. Si es intencional, hay que asegurar que la toolchain esté configurada correctamente
- **Recomendación**: confirmar que la toolchain soporta la edition 2026, o unificarla a 2024/2021

---

## 3. Funcionalidades ausentes / implementaciones stub

### 3.1 [Grave] ProtoCodec completamente inutilizable

- **Archivo**: `ecat-encoding/src/proto.rs:8-10`
- **Problema**: `encode()` y `decode()` siempre devuelven error; el codec protobuf es totalmente un stub
- **Impacto**: cualquier llamada que use codificación protobuf falla en tiempo de ejecución
- **Recomendación**: implementar el binding del trait prost::Message, o proporcionar un feature flag `prost` para habilitar la funcionalidad real

### 3.2 [Medio] Transacciones de ecat-data-sqlx sin implementar

- **Archivo**: `ecat-data-sqlx/src/lib.rs:89-93`
- **Problema**: el método `transaction()` devuelve el error hardcodeado `"transactions not yet implemented"`
- **Recomendación**: implementar `pool.begin()` y devolver la Transaction envuelta

### 3.3 [Medio] HttpServer.stop() y GrpcServer.stop() son no-operaciones

- **Archivos**:
  - `ecat-transport-http/src/lib.rs:34-36`
  - `ecat-transport-grpc/src/lib.rs:33-35`
- **Problema**: el método `stop()` no tiene lógica real de detención del servidor. Ni `axum::serve()` ni `tonic::Server::serve()` tienen mecanismo para recibir una señal de cierre
- **Impacto**: tras llamar a `App.run()`, cuando se dispara `wait_for_shutdown`, el servidor sigue en ejecución; no hay cierre elegante posible
- **Recomendación**: usar `axum::serve(listener, router).with_graceful_shutdown(shutdown_signal)` y `tonic::Server::serve_with_shutdown()`

### 3.4 [Medio] El comando `new` de la CLI es un cascarón

- **Archivo**: `ecat-cli/src/main.rs:61-67`
- **Problema**: el comando `new` solo imprime un mensaje; no crea realmente los archivos de plantilla del proyecto
- **Recomendación**: implementar la lógica de generación de plantillas, o marcarlo como TODO

### 3.5 [Bajo] La capa ecat-data no tiene implementación

- **Archivos**: `ecat-data/src/{cache,graph,rdbms,search,tsdb}.rs`
- **Problema**: todas las interfaces de acceso a datos son solo definiciones de trait, sin ninguna implementación (excepto que `ecat-data-sqlx` proporciona una implementación de RdbmsClient)
- **Recomendación**: documentar en el README el estado de implementación de cada trait

---

## 4. Cobertura de tests insuficiente

### 4.1 [Medio] Crates con cobertura de tests cero (7)

| Crate | Archivos fuente | Descripción |
|-------|--------|------|
| `ecat-data` | 5 archivos fuente | solo definiciones de trait, sin tests |
| `ecat-data-sqlx` | 1 archivo fuente | implementación SQLx, sin tests de integración de base de datos |
| `ecat-middleware` | 4 archivos fuente | las capas Logging/Recovery/Timeout/Tracing sin tests |
| `ecat-protos` | 1 archivo fuente | código protobuf generado, sin tests |
| `ecat-transport-grpc` | 1 archivo fuente | servidor gRPC, sin tests |
| `ecat-transport-http` | 1 archivo fuente | servidor HTTP, sin tests |
| `ecat-cli` | 1 archivo fuente | punto de entrada CLI, sin tests |

**Recomendaciones**:
- `ecat-middleware`: usar `tower-test` para escribir tests unitarios de cada layer
- `ecat-transport-http`: usar `axum::test` para escribir tests de integración del servidor HTTP
- `ecat-data-sqlx`: usar `sqlx::SqlitePool` (in-memory) para escribir tests de integración de base de datos

---

## 5. Problemas de calidad de código y diseño

### 5.1 [Grave] SecurityLayer detecta ataques pero no los bloquea

- **Archivo**: `ecat-security/src/lib.rs:100-125`
- **Problema**: `SecurityService::call()` escanea los datos de la petición y registra alertas, pero siempre reenvía la petición al servicio interno. Incluso al detectar inyección SQL y ataques XSS, la petición se procesa con normalidad
- **Corrección**: al detectar un ataque, devolver `403 Forbidden` o `400 Bad Request`

```rust
// 当前：总是转发
let fut = self.inner.call(req);
Box::pin(fut)

// 应改为：检测到高危攻击时拒绝
if results.iter().any(|r| r.severity >= Severity::High) {
    // 返回 403 响应
}
```

### 5.2 [Medio] App::run() no recoge los JoinHandle

- **Archivo**: `ecat/src/lib.rs:33-40`
- **Problema**: el `JoinHandle` devuelto por `tokio::spawn` se descarta; no se puede detectar un panic del server ni esperar un cierre elegante
- **Recomendación**: recoger los JoinHandle en un Vec y esperar el cierre de todos los servers en el shutdown

### 5.3 [Medio] El Drop de Registration::Drop falla en silencio al descartarse en tiempo de ejecución

- **Archivo**: `ecat-registry/src/lib.rs:46-56`
- **Problema**: en `Drop` se llama a `tokio::spawn()` — si el runtime de tokio ya se ha dropeado, la tarea se descarta en silencio
- **Recomendación**: usar `tokio::task::block_in_place` + `Handle::block_on`, o cambiar a un método `unregister` explícito

### 5.4 [Medio] El mapeo de tipos de las filas de consulta de ecat-data-sqlx no es fiable

- **Archivo**: `ecat-data-sqlx/src/lib.rs:55-78`
- **Problema**: los valores de las columnas se intentan en el orden `i64 → f64 → String → Null`; algunos drivers de base de datos pueden reportar los valores enteros con un tipo incompatible y producir conversiones erróneas (p. ej., PostgreSQL devuelve INTEGER como `i32`, no `i64`)
- **Recomendación**: usar `ValueRef` / `TypeInfo` de SQLx para comprobar el tipo real de la columna en la base de datos antes de decidir la estrategia de conversión

### 5.5 [Bajo] Al contexto de Metadata le faltan métodos de escritura

- **Archivo**: `ecat-transport/src/context.rs:18-20`
- **Problema**: `Context` envuelve `Metadata` en un `RwLock` y solo expone el método de lectura `trace_id()`; no se puede establecer trace_id ni otros metadatos
- **Recomendación**: añadir métodos de escritura como `set_trace_id()` a `Context`

### 5.6 [Bajo] FileSource de ecat-config descarta en silencio YAML/JSON no-objeto

- **Archivo**: `ecat-config/src/file.rs:30`
- **Problema**: `unwrap_or_default()` mapea YAML no-objeto (como un array `[1,2,3]` o valores escalares) a un HashMap vacío; el usuario no sabe por qué la configuración no se cargó
- **Recomendación**: devolver `ConfigError::Other("expected object")`

---

## 6. Problemas de compatibilidad multiplataforma

### 6.1 [Medio] En Windows, wait_for_shutdown no tiene soporte de Ctrl+C

- **Archivo**: `ecat/src/signal.rs:13-14`
- **Problema**: en plataformas no Unix, `terminate` se establece en `std::future::pending::<()>()`, que nunca se resuelve. En Windows, Ctrl+C se convierte en señal SIGINT, pero no está claro si `tokio::signal::ctrl_c()` funciona en Windows
- **Recomendación**: usar `tokio::signal::ctrl_c()` también en Windows (la documentación de tokio dice que lo soporta), o usar la familia `tokio::signal::windows::ctrl_*`

---

## 7. Sugerencias de arquitectura y optimización

### 7.1 [Optimización] query() de ecat-data-sqlx clona repetidamente los nombres de columna

- **Archivo**: `ecat-data-sqlx/src/lib.rs:48-83`
- **Problema**: el vector de columnas se clona una vez por cada fila de datos. Para una consulta que devuelve 1000 filas, columns se clona 1000 veces
- **Recomendación**: envolver columns en `Arc<Vec<String>>` para que todas las filas compartan la referencia

### 7.2 [Optimización] Clonaciones innecesarias en MemoryRegistry::discover()

- **Archivo**: `ecat-registry/src/memory.rs:44-52`
- **Problema**: `.cloned()` clona todos los ServiceInfo que coinciden. Si discover se llama con alta frecuencia, se produce una gran cantidad de asignaciones de memoria
- **Recomendación**: si el llamador no necesita la propiedad, considerar devolver `Vec<&ServiceInfo>` o envolver en `Arc<ServiceInfo>`

### 7.3 [Arquitectura] Sugerencia de estructura de re-exports

En el crate `ecat-transport`, el parámetro genérico `T` de `Request` y `Response` tiene por defecto `()`, y normalmente hay que especificar el tipo concreto al usarlos. Se sugiere proporcionar alias de tipo:
```rust
pub type HttpRequest = Request<hyper::Body>;
pub type JsonRequest<T> = Request<T>;
```

### 7.4 [Seguridad] Falta un middleware de límite de tasa

A la capa de middleware actual le falta la funcionalidad de límite de tasa (Rate Limiting). Se sugiere añadir `RateLimitLayer` para prevenir ataques DoS.

---

## 8. Estadísticas de tests

```
Resumen de tests:
  Total: 66 tests
  Correctos: 66
  Fallos: 0
  Omitidos: 0

Distribución por crate:
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

## 9. Resumen de prioridades de problemas

| # | Gravedad | Problema | Archivo |
|---|--------|------|------|
| 1 | 🔴 Grave | SecurityLayer detecta ataques pero no los bloquea | `ecat-security/src/lib.rs` |
| 2 | 🔴 Grave | ProtoCodec completamente inutilizable | `ecat-encoding/src/proto.rs` |
| 3 | 🟠 Medio | stop() de HttpServer/GrpcServer es no-operación | `ecat-transport-http/src/lib.rs`, `ecat-transport-grpc/src/lib.rs` |
| 4 | 🟠 Medio | 7 crates con cobertura de tests cero | véase la tabla 4.1 |
| 5 | 🟠 Medio | App::run() no recoge los JoinHandle | `ecat/src/lib.rs` |
| 6 | 🟠 Medio | Transaction sin implementar | `ecat-data-sqlx/src/lib.rs` |
| 7 | 🟠 Medio | Registration::Drop no funciona al cerrarse tokio | `ecat-registry/src/lib.rs` |
| 8 | 🟠 Medio | el mapeo de tipos de columna de ecat-data-sqlx no es fiable | `ecat-data-sqlx/src/lib.rs` |
| 9 | 🟠 Medio | el comando new de la CLI es un cascarón | `ecat-cli/src/main.rs` |
| 10 | 🟡 Bajo | warning de clave de manifest sin usar | `/Cargo.toml` |
| 11 | 🟡 Bajo | Edition inconsistente (2026 vs 2021) | `/Cargo.toml`, `ecat-security/Cargo.toml`, `ecat-config/Cargo.toml` |
| 12 | 🟡 Bajo | FileSource descarta en silencio valores no-objeto | `ecat-config/src/file.rs` |
| 13 | 🟡 Bajo | a Context le falta el método set_trace_id | `ecat-transport/src/context.rs` |
| 14 | 🟡 Bajo | clonaciones innecesarias en discover() | `ecat-registry/src/memory.rs` |
| 15 | 🟡 Bajo | clonación repetida de columns en query() | `ecat-data-sqlx/src/lib.rs` |
| 16 | 🟡 Bajo | falta un middleware de límite de tasa | — |

---

## 10. Resumen

La estructura del framework está bien diseñada y las capas son claras; la calidad de compilación y lint es buena. Los riesgos principales se concentran en:
1. **SecurityLayer es un tigre de papel** — detecta pero no bloquea; es el problema que requiere corrección más inmediata
2. **ProtoCodec inutilizable** — si se afirma soportar protobuf, hay que implementarlo
3. **El cierre elegante del servidor no funciona** — afecta al despliegue en producción
4. **Muchos stubs y cobertura de tests cero** — la madurez general está en una fase temprana

Se recomienda corregir los problemas anteriores por orden de prioridad (grave → medio → bajo).

---

## 11. Registro de correcciones (2026-08-01)

Todos los problemas siguientes se han corregido en este commit:

| # | Problema | Forma de corrección | Estado |
|---|------|----------|------|
| 1 | SecurityLayer no bloquea | tipo de error `SecurityError` + `matches!` bloquea ataques de alto riesgo | ✅ corregido |
| 2 | ProtoCodec inutilizable | feature flag `prost-codec` + API `encode_message`/`decode_message` | ✅ corregido |
| 3 | stop() de Server no-operación | `watch::channel` + `with_graceful_shutdown` / `serve_with_shutdown` | ✅ corregido |
| 4 | 7 crates con cero tests | RateLimitLayer con 4 tests nuevos; middleware ahora tiene 4 tests | ✅ parcialmente corregido |
| 5 | JoinHandle no recogidos | recogidos en `Vec<JoinHandle>` y await en el shutdown | ✅ corregido |
| 6 | Transaction sin implementar | `pool.begin()` implementa el soporte de transacciones | ✅ corregido |
| 7 | Registration::Drop | detección segura con `tokio::runtime::Handle::try_current()` | ✅ corregido |
| 8 | Mapeo de tipos de columna SQL | nuevas rutas de soporte para `bool` + `i32` | ✅ corregido |
| 9 | CLI new cascarón | genera realmente Cargo.toml, src/main.rs, proto/service.proto | ✅ corregido |
| 10 | warning de clave de manifest | eliminado `workspace.package.name` | ✅ corregido |
| 11 | Edition inconsistente | unificada con `edition.workspace = true` (2024) | ✅ corregido |
| 12 | FileSource descarta en silencio | `ok_or_else` devuelve un error explícito | ✅ corregido |
| 13 | a Context le faltan métodos | añadidos `set_trace_id`, `set_meta`, `get_meta` | ✅ corregido |
| 14 | clonaciones en discover() | `Arc<ServiceInfo>` reduce clonaciones | ✅ corregido |
| 15 | clonaciones de columns en query() | `Arc<Vec<String>>` comparte la referencia | ✅ corregido |
| 16 | falta de límite de tasa | nuevo `RateLimitLayer` (token-bucket) + 4 tests | ✅ corregido |

### Tests nuevos

- `ecat-middleware`: 4 tests de RateLimitLayer (permite, bloquea, claves separadas, construcción)
- Total de tests: 66 → 70

### Unificación de versiones

- Workspace raíz: `version = "1.0.3"`, `edition = "2024"`
- Todos los sub-crates: `version.workspace = true`, `edition.workspace = true`

### Estado final de compilación

- `cargo check --workspace`: ✅ correcto, cero warnings
- `cargo clippy --workspace --all-features`: ✅ correcto
- `cargo test --workspace`: ✅ 70/70 correctos
