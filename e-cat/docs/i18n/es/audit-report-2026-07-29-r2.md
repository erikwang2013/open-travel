<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Informe de revisión de código de e-cat (segunda ronda)

**Fecha**: 2026-07-29  
**Rama**: main  
**Proyecto**: e-cat (workspace Rust, 17 crates)

---

## 一、Resumen de la revisión

Sobre la base de las correcciones de clippy y el complemento de tests de la primera ronda, esta ronda realizó una revisión profunda de la lógica del código, centrada en la corrección en tiempo de ejecución, la seguridad de concurrencia y la coherencia semántica de la API. Se revisaron 32 archivos fuente.

### Línea base de verificación

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

---

## 二、Bugs encontrados y correcciones

### Bug 1: [Crítico] Error en el ciclo de vida del guard del span de TracingLayer

- **Archivo**: `ecat-middleware/src/tracing.rs:37`
- **Gravedad**: **alta**
- **Impacto**: ninguna petición que pase por TracingLayer queda cubierta por un span de tracing

**Análisis de la causa raíz**:

```rust
// 修复前
fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let _guard = span.enter();  // guard 在 call() 返回时 drop
    let fut = self.inner.call(req);
    Box::pin(fut)               // future 在后续 poll 时才执行
}
```

El guard que devuelve `span.enter()` solo mantiene el span activo en el contexto síncrono actual. `call()` devuelve un future aún no sometido a poll; la ejecución asíncrona real ocurre en la fase de poll posterior, cuando el guard ya se ha dropeado y el span no surte efecto. Ninguna petición que pase por TracingLayer aparece en la salida de tracing.

**Corrección**:

```rust
// 修复后
use tracing::Instrument;

fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let fut = self.inner.call(req);
    Box::pin(fut.instrument(span))  // span 附着在 future 生命周期上
}
```

Usar `tracing::Instrument::instrument()` adjunta el span al future, garantizando que el span permanezca activo durante todo el ciclo de vida de poll del future.

---

### Bug 2: [Crítico] Defecto en la implementación de la clausura de LifecycleHook — on_stop nunca se ejecuta

- **Archivo**: `ecat/src/hook.rs:14-23`, `ecat/src/lib.rs:11-16`
- **Gravedad**: **alta**
- **Impacto**: el hook de clausura registrado con `.on_stop()` no hace nada en el shutdown

**Análisis de la causa raíz**:

En el diseño original, los métodos `on_start()` y `on_stop()` empujan ambos el hook al mismo Vec `lifecycle_hooks`. En `run()`, todos los hooks llaman secuencialmente a `on_start()`; en el shutdown, todos llaman secuencialmente a `on_stop()`.

El problema está en el blanket impl del trait `LifecycleHook` para clausuras `Fn() -> Fut`: **solo cubre `on_start()`; `on_stop()` usa la implementación por defecto del trait (no-op)**.

Esto significa que cuando el usuario usa la sintaxis de clausura `.on_stop(|| async { ... })`, la clausura se añade a la lista de hooks, pero en el shutdown solo se ejecuta el `on_stop()` vacío por defecto: la lógica del usuario nunca se ejecuta.

**Corrección (en dos partes)**:

1. **Separar start_hooks y stop_hooks** (`ecat/src/lib.rs`):

```rust
// App 结构体 — 两个独立的 Vec
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

2. **Completar el blanket impl de clausuras** (`ecat/src/hook.rs`):

```rust
impl<F, Fut> LifecycleHook for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Result<...>> + Send,
{
    async fn on_start(&self) -> ... { (self)().await }
    async fn on_stop(&self) -> ...  { (self)().await }  // 新增
}
```

Ahora la clausura implementa tanto `on_start` como `on_stop`; con los Vecs separados, cada hook solo se invoca en la fase correcta del ciclo de vida.

---

### Bug 3: [Medio] Prioridad incorrecta en la extracción de tipos de valores Row de SqlxClient

- **Archivo**: `ecat-data-sqlx/src/lib.rs:53-68`
- **Gravedad**: media
- **Impacto**: los valores enteros y de coma flotante de la base de datos se extraen como cadenas JSON en lugar de números

**Análisis de la causa raíz**:

`try_get::<String>()` se intenta en primer lugar. La mayoría de los drivers de base de datos pueden ejecutar `try_get::<String>()` con éxito sobre columnas numéricas (conversión implícita), de modo que el valor entero `42` se extrae como `"42"` en lugar de `42`.

**Corrección**: ajustar el orden de intentos de `try_get` a `i64 → f64 → String → Null`, preservando primero los tipos numéricos.

---

## 三、Otros hallazgos de la revisión (sin modificar / limitaciones conocidas)

| Categoría | Archivo | Descripción | Recomendación |
|------|------|------|------|
| Funcionalidad incompleta | `ecat-transport-http/src/lib.rs:30` | `axum::serve().await` bloquea y nunca retorna; `stop()` es una no-operación | implementar graceful shutdown |
| Funcionalidad incompleta | `ecat-transport-grpc/src/lib.rs:29` | igual que arriba | implementar graceful shutdown |
| Funcionalidad incompleta | `ecat-data-sqlx/src/lib.rs:79` | `transaction()` devuelve error de no implementado | implementar soporte de transacciones |
| Estilo de código | `ecat-middleware/src/logging.rs:42` | `elapsed.as_millis() as u64` truncamiento teórico u128→u64 | sin impacto real |
| Tests ausentes | `ecat-middleware/` | 4 Tower Services sin tests unitarios | requieren tests de integración |
| Tests ausentes | `ecat-data/` | solo definiciones de trait | aceptable por ahora |
| Bloqueo de RwLock | `ecat-registry/src/memory.rs` | el RwLock síncrono puede bloquear en contexto asíncrono | considerar cambiar a tokio::sync::RwLock |

---

## 四、Resultados de las pruebas

```
cargo test → 60 passed, 0 failed

Distribución por crate:
  ecat                  4   (Builder/valores por defecto/hooks de ciclo de vida)
  ecat-config           9   (parseo env ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (mapeo HTTP/conversión gRPC/metadata/Display)
  ecat-logging          1   (smoke de init)
  ecat-metadata         9   (acceso/From HeaderMap/From MetadataMap/iterador)
  ecat-metrics          2   (singleton/text sin panic)
  ecat-registry         5   (registro/descubrimiento/baja/lista/filtro)
  ecat-transport       11   (Context/Request/Response/trait Server)
  Los otros 8 crates    0   (solo traits/generación de código/requieren tests de integración/solo impresión)
```

---

## 五、Lista de archivos modificados

| Archivo | Tipo de cambio | Descripción del cambio |
|------|----------|----------|
| `ecat/src/lib.rs` | corrección de bug | App separa start_hooks/stop_hooks; AppBuilder actualizado en consecuencia; tests adaptados |
| `ecat/src/hook.rs` | corrección de bug | blanket impl de clausuras completa la implementación de on_stop() |
| `ecat-middleware/src/tracing.rs` | corrección de bug | guard del span → `fut.instrument(span)` |
| `ecat-data-sqlx/src/lib.rs` | corrección de bug | orden de extracción de valores Row i64→f64→String→Null |

---

## 六、Resumen

Esta ronda de revisión encontró 2 bugs de alta gravedad en tiempo de ejecución y 1 problema de corrección de datos de gravedad media:

1. **Span de TracingLayer inoperante** — afecta la observabilidad de todas las peticiones
2. **on_stop de LifecycleHook no se ejecuta** — afecta la corrección de toda la lógica de shutdown
3. **Pérdida del tipo numérico en Row** — afecta la corrección de tipos de los resultados de consultas a la base de datos

Los tres problemas están corregidos; tras la corrección, los 60 tests pasan y la compilación tiene cero errores y cero advertencias.

### Recomendaciones posteriores

- Implementar graceful shutdown para los servidores HTTP/gRPC
- Añadir tests de integración para `ecat-middleware` (mock de Service + verificación de comportamiento de span/timeout/recovery)
- Añadir tests de integración para `ecat-data-sqlx` (usando base de datos SQLite en memoria)
- Sustituir el RwLock síncrono de `ecat-registry/memory.rs` por `tokio::sync::RwLock`
