<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Informe de revisión de código de e-cat (tercera ronda)

**Fecha**: 2026-07-29  
**Rama**: main  
**Proyecto**: e-cat (workspace Rust, 18 crates)  
**Alcance de la revisión**: los 37 archivos fuente, 2151 líneas de código Rust

---

## 一、Resumen de la revisión

Los 3 bugs encontrados en la segunda ronda ya están corregidos; esta ronda hace una re-revisión profunda sobre la línea base limpia (0 error / 0 warning / 60 tests pasando), centrada en condiciones de borde, manejo de errores y robustez de producción.

### Línea base de verificación

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

### Confirmación de corrección de bugs de R2

| Bug | Archivo | Estado |
|-----|------|------|
| Ciclo de vida del guard del span de TracingLayer | `ecat-middleware/src/tracing.rs` | ✅ corregido |
| on_stop de LifecycleHook no se ejecuta | `ecat/src/hook.rs`, `ecat/src/lib.rs` | ✅ corregido |
| Prioridad de extracción de tipos de valores Row | `ecat-data-sqlx/src/lib.rs` | ✅ corregido |

---

## 二、Problemas recién encontrados

### Problema 1: [Medio] `unwrap()` en `metrics_text()`, puede entrar en panic en producción

- **Archivo**: `ecat-metrics/src/lib.rs:14-15`
- **Gravedad**: **media**
- **Impacto**: el proceso entra en panic al acceder al endpoint `/metrics`

**Análisis de la causa raíz**:

```rust
pub fn metrics_text() -> String {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder.encode(&registry().gather(), &mut buffer).unwrap();  // 可能 panic
    String::from_utf8(buffer).unwrap()                           // 可能 panic
}
```

`TextEncoder::encode()` puede fallar ante errores I/O internos o falta de memoria del sistema. `String::from_utf8()` también puede fallar teóricamente si la librería Prometheus produce salida no UTF-8. Estos dos `unwrap()` están en rutas de código no-test, expuestas directamente a la llamada del handler HTTP; un panic provoca el colapso del proceso.

**Corrección sugerida**: devolver `Result<String, ...>` o degradar con `.unwrap_or_default()`.

---

### Problema 2: [Bajo] Recovery middleware hace spawn de un nuevo task y pierde el contexto del span

- **Archivo**: `ecat-middleware/src/recovery.rs:40`
- **Gravedad**: **baja**
- **Impacto**: cuando la capa Recovery está antes de la capa Tracing, el trace_id de la petición no llega a la lógica de negocio

**Análisis de la causa raíz**:

```rust
fn call(&mut self, req: Req) -> Self::Future {
    let fut = self.inner.call(req);
    Box::pin(async move {
        match tokio::task::spawn(fut).await {  // 新 task，不继承 span
            // ...
        }
    })
}
```

`tokio::task::spawn()` crea una nueva tarea Tokio; los spans de tracing son task-local y no se propagan automáticamente.

**Sugerencia**: documentar explícitamente el requisito de orden de los middleware (Recovery debe ir en la capa más externa), o pasar manualmente el span con `.instrument(span)` antes del spawn.

---

### Problema 3: [Bajo] El Drop de Registration descarta errores en silencio

- **Archivo**: `ecat-registry/src/lib.rs:50-52`
- **Gravedad**: **baja**
- **Impacto**: el fallo de baja del servicio pasa desapercibido

```rust
impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(reg) = self.registry.take() {
            let id = self.id.clone();
            tokio::spawn(async move {
                let _ = reg.deregister(&id).await;  // 错误被静默丢弃
            });
        }
    }
}
```

Aunque no se puede bloquear en Drop, se puede registrar el fallo de baja con `tracing::warn!`.

---

### Problema 4: [Bajo] Manejo de valores especiales f64 en `ecat-data-sqlx`

- **Archivo**: `ecat-data-sqlx/src/lib.rs:57-61`
- **Gravedad**: **baja**
- **Impacto**: los valores de coma flotante NaN/Infinity de la base de datos se convierten en Null

```rust
row.try_get::<f64, _>(col.as_str())
    .ok()
    .and_then(serde_json::Number::from_f64)  // NaN/Inf → None
    .map(serde_json::Value::Number)
    .ok_or(())
```

`serde_json::Number::from_f64()` devuelve `None` para `f64::NAN`, `f64::INFINITY` y `f64::NEG_INFINITY`, lo que degrada esos valores a Null.

---

## 三、Notas de revisión por crate

### ecat (núcleo) — 4 archivos
| Archivo | Estado | Nota |
|------|------|------|
| `lib.rs` | ✅ | separación de start_hooks/stop_hooks correcta |
| `hook.rs` | ✅ | blanket impl de clausuras cubre on_start/on_stop |
| `signal.rs` | ⚠️ | el `.expect()` del handler SIGTERM es razonable pero estricto |

### ecat-transport — 4 archivos
| Archivo | Estado | Nota |
|------|------|------|
| `lib.rs` | ✅ | diseño conciso del trait Server |
| `context.rs` | ✅ | ya usa `tokio::sync::RwLock` |
| `request.rs` | ✅ | |
| `response.rs` | ✅ | |

### ecat-transport-http / ecat-transport-grpc — 2 archivos
| Archivo | Estado | Nota |
|------|------|------|
| `ecat-transport-http/src/lib.rs` | ⚠️ | `start()` bloquea sin retornar, `stop()` no-operación (limitación conocida) |
| `ecat-transport-grpc/src/lib.rs` | ⚠️ | igual que arriba |

### ecat-middleware — 5 archivos
| Archivo | Estado | Nota |
|------|------|------|
| `tracing.rs` | ✅ | corrección de `fut.instrument(span)` correcta |
| `recovery.rs` | ⚠️ | `tokio::task::spawn` pierde el contexto del span (problema 2) |
| `logging.rs` | ✅ | el truncamiento teórico de `elapsed.as_millis() as u64` no tiene impacto real |
| `timeout.rs` | ✅ | |

### ecat-registry — 2 archivos
| Archivo | Estado | Nota |
|------|------|------|
| `lib.rs` | ⚠️ | el Drop de Registration descarta errores en silencio (problema 3) |
| `memory.rs` | ⚠️ | `std::sync::RwLock` síncrono en contexto async (limitación conocida) |

### ecat-config — 3 archivos
| Archivo | Estado | Nota |
|------|------|------|
| `lib.rs` | ✅ | diseño razonable del trait Config |
| `env.rs` | ✅ | orden correcto de análisis de tipos (bool→i64→f64→String) |
| `file.rs` | ⚠️ | no admite YAML de varios documentos ni mecanismo watch (limitación conocida) |

### ecat-data — 6 archivos
| Archivo | Estado | Nota |
|------|------|------|
| `rdbms.rs` | ✅ | el comentario del Drop de Transaction explica el rollback automático pero sin cuerpo implementado |
| `cache.rs` | ✅ | definición completa del trait |
| `graph.rs` | ✅ | |
| `search.rs` | ✅ | |
| `tsdb.rs` | ✅ | buen diseño del patrón builder de DataPoint |

### ecat-data-sqlx — 1 archivo
| Archivo | Estado | Nota |
|------|------|------|
| `lib.rs` | ⚠️ | orden de extracción de valores corregido; transaction sin implementar; valores especiales f64 (problema 4) |

### ecat-errors — 2 archivos
| Archivo | Estado | Nota |
|------|------|------|
| `lib.rs` | ✅ | mapeo gRPC→ErrorCode completo, formato Display claro |
| `codes.rs` | ✅ | el mapeo de códigos de estado HTTP es coherente con la semántica gRPC |

### ecat-encoding — 3 archivos
| Archivo | Estado | Nota |
|------|------|------|
| `lib.rs` | ✅ | buen diseño del enum CodecBox y de codec_for/codec_from_content_type |
| `json.rs` | ✅ | |
| `proto.rs` | ⚠️ | ProtoCodec es una implementación provisional (limitación conocida) |

### Resto de crates
| Crate | Estado | Nota |
|-------|------|------|
| `ecat-logging` | ✅ | `try_init` evita la doble inicialización |
| `ecat-metadata` | ✅ | conversión bidireccional HTTP/gRPC completa |
| `ecat-metrics` | ⚠️ | `metrics_text()` tiene unwrap() (problema 1) |
| `ecat-protos` | ✅ | generación de código prost/tonic |
| `ecat-cli` | ⚠️ | la mayoría de comandos solo imprimen mensajes, no crean archivos realmente (limitación conocida) |
| `examples/helloworld` | ✅ | el código de ejemplo usa la nueva API correctamente |

---

## 四、Análisis de cobertura de pruebas

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
  Los otros 8 crates    0   (solo traits/generación de código/requieren tests de integración)
```

### Brechas de tests

| Prioridad | Crate | Contenido ausente |
|--------|-------|----------|
| Alta | `ecat-middleware` | 4 Tower Services sin tests unitarios |
| Alta | `ecat-data-sqlx` | sin tests de integración (base SQLite en memoria viable) |
| Media | `ecat-transport-http` | el flujo de arranque del servidor HTTP sin test |
| Media | `ecat-transport-grpc` | el flujo de arranque del servidor gRPC sin test |
| Baja | `ecat-data` | solo definiciones de trait, aceptable |

---

## 五、Métricas de calidad de código

| Métrica | Valor | Calificación |
|------|-----|------|
| Líneas totales | 2151 | — |
| Advertencias de compilación | 0 | ✅ |
| Advertencias de Clippy | 0 | ✅ |
| Tests que pasan | 60/60 | ✅ |
| Cobertura de tests (estimada) | ~35% | ⚠️ |
| unwrap() fuera de tests | 2 (metrics) | ⚠️ |
| Código inseguro | 0 | ✅ |
| Puntos de riesgo de panic | 3 (metrics×2 + expect de signal) | ⚠️ |

---

## 六、Resumen de correcciones sugeridas

### Correcciones sugeridas (esta ronda — todas corregidas ✅)

| # | Archivo | Problema | Prioridad | Estado |
|---|------|------|--------|------|
| 1 | `ecat-metrics/src/lib.rs:14-15` | unwrap de `metrics_text()` → tratamiento degradado | media | ✅ corregido |
| 2 | `ecat-registry/src/lib.rs:51` | añadir `tracing::warn!` en Drop para registrar fallos de deregister | baja | ✅ corregido |
| 3 | `ecat-data-sqlx/src/lib.rs:57-61` | añadir tratamiento especial para valores f64 NaN/Inf | baja | ✅ corregido |
| 4 | `ecat-middleware/src/recovery.rs:40` | `tokio::task::spawn` pierde el span → `fut.instrument(span)` | baja | ✅ corregido |
| 5 | `ecat-registry/src/memory.rs` | RwLock síncrono → `tokio::sync::RwLock` | baja | ✅ corregido |

### Limitaciones conocidas (no bloqueantes)

| # | Archivo | Descripción |
|---|------|------|
| K1 | `ecat-transport-http` / `ecat-transport-grpc` | start() bloquea / stop() no-operación (requiere graceful shutdown) |
| K2 | `ecat-data-sqlx` | `transaction()` devuelve error de no implementado |
| K3 | `ecat-middleware` | 4 Services sin tests unitarios |
| K4 | `ecat-config/file.rs` | sin mecanismo watch |
| K5 | `ecat-encoding/proto.rs` | ProtoCodec implementación provisional |
| K6 | `ecat-cli` | la mayoría de comandos son salidas mock |

---

## 七、Resumen

La tercera ronda de revisión se realizó sobre la base de las correcciones completas de R2. Los 5 problemas encontrados en esta ronda están todos corregidos.

Comparación con R2:
- R2 encontró 2 bugs de gravedad alta + 1 de media en tiempo de ejecución → todos corregidos ✅
- R3 encontró 1 problema de robustez medio + 4 bajos → todos corregidos ✅
- El número de tests se mantiene en 60

### Recomendaciones prioritarias posteriores

1. Añadir tests de integración SQLite para `ecat-data-sqlx`
2. Añadir tests unitarios para `ecat-middleware` (verificar comportamiento de span/timeout/recovery)
3. Implementar graceful shutdown para los servidores HTTP/gRPC
