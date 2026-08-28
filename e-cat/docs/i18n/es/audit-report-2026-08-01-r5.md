# Informe de auditoría de E-CAT — r5

**Fecha**: 2026-08-01  
**Rama**: main  
**Versión**: 2.1.7  
**Número de crates**: 47 (miembros del workspace)
**Estado**: ✅ todos los problemas corregibles resueltos + los backends de datos soportan por completo archivos de configuración

---

## 0. Registro de correcciones (2026-08-01)

| # | Problema | Archivo | Corrección |
|---|------|------|------|
| 1 | import sin usar `axum::routing::get` | `ecat-versioning/src/lib.rs:3` | eliminar el import de nivel superior, moverlo a `#[cfg(test)]` |
| 2 | variable sin usar `version` | `ecat-versioning/src/lib.rs:61` | cambiar a `_version` |
| 3 | código muerto `extract_version` | `ecat-versioning/src/lib.rs:68` | cambiar a `pub fn` |
| 4 | `useless_format!` | `ecat-versioning/src/lib.rs:62` | usar directamente `"/api"` |
| 5 | `unnecessary_to_owned` | `ecat-data-questdb/src/lib.rs:39` | `"true".to_string()` → `"true"` |
| 6 | mensaje de error tragado | `ecat-data-questdb/src/lib.rs:30` | `unwrap_or_default()` → `unwrap_or_else(...)` |
| 7 | `derivable_impls` | `ecat-client/src/lib.rs:249` | `GrpcClientBuilder` pasa a `#[derive(Default)]` |
| 8 | `manual_is_multiple_of` | `ecat-config/src/encrypted.rs:60` | `s.len() % 2 != 0` → `!s.len().is_multiple_of(2)` |
| 9 | `collapsible_if` | `ecat-registry-etcd/src/lib.rs:92` | fusionar `if let` anidados |
| 10 | `collapsible_if` | `ecat-data-clickhouse/src/lib.rs:56` | fusionar `if let` anidados |
| 11 | `type_complexity` | `ecat-data-memcached/src/lib.rs:9` | añadir el alias `type CacheEntry` |

**Resultado final**: `cargo build` cero warnings, `cargo clippy --all-targets` cero warnings, `cargo test` todo correcto (0 fallos).

### 12 ─ Los backends de datos soportan por completo archivos de configuración (Cargo + lib.rs)

Se añadió la estructura `Config` (`#[derive(Deserialize)]`) y el constructor `from_config()` a los 12 crates de backends de datos, para cargar la información de conexión desde archivos JSON/YAML sin necesidad de hardcodear.

| Crate | Estructura Config | Campos |
|-------|--------------|------|
| `ecat-data-redis` | `RedisConfig` | `url` |
| `ecat-data-sqlx` | `SqlxConfig` | `url` |
| `ecat-data-clickhouse` | `ClickhouseConfig` | `base_url`, `database` (por defecto "default") |
| `ecat-data-questdb` | `QuestdbConfig` | `base_url` |
| `ecat-data-elasticsearch` | `ElasticsearchConfig` | `base_url` |
| `ecat-data-opensearch` | `OpenSearchConfig` | `base_url` |
| `ecat-data-influxdb` | `InfluxConfig` | `base_url`, `org`, `bucket`, `token` |
| `ecat-data-memcached` | `MemcachedConfig` | (vacío — implementación en memoria) |
| `ecat-data-neo4j` | `Neo4jConfig` | `base_url`, `username`, `password` |
| `ecat-data-nebulagraph` | `NebulaGraphConfig` | `base_url`, `space` |
| `ecat-data-arangodb` | `ArangoConfig` | `base_url`, `db`, `username`, `password` |
| `ecat-data-iotdb` | `IotdbConfig` | `base_url`, `username`, `password` |

**Ejemplo de uso**:
```rust
// 从 YAML 配置文件加载
let cfg: ClickhouseConfig = serde_json::from_str(r#"{"base_url":"http://localhost:8123"}"#)?;
let client = ClickhouseClient::from_config(cfg);
```

### 13 ─ Autenticación opcional para los backends HTTP (5 crates)

Se añadieron los campos opcionales `username` / `password` y el constructor `with_auth()` a 5 backends puramente HTTP. Todos son `Option<String>` (`#[serde(default)]`); si no se configuran, no hay autenticación.

| Crate | Campos Config nuevos | Constructor nuevo |
|-------|-----------------|-------------|
| `ecat-data-elasticsearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-opensearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-clickhouse` | `username?`, `password?` | `with_auth()` |
| `ecat-data-questdb` | `username?`, `password?` | `with_auth()` |
| `ecat-data-nebulagraph` | `username?`, `password?` | `with_auth()` |

Todas las peticiones HTTP adjuntan automáticamente Basic Auth mediante el método auxiliar `apply_auth()` (solo cuando ambos no son None).

### 14 ─ Campos de autenticación opcionales para Redis / RDBMS / Memcached (3 crates)

| Crate | Campos Config nuevos | Constructor nuevo | Forma de autenticación |
|-------|-----------------|-------------|----------|
| `ecat-data-redis` | `password?` | `connect_with_password()` | contraseña embebida en la URL |
| `ecat-data-sqlx` | `username?`, `password?` | `connect_with_auth()` | autenticación embebida en la URL |
| `ecat-data-memcached` | `username?`, `password?` | `with_auth()` | campo reservado (implementación en memoria) |

Sqlx cubre los cuatro RDBMS SQLite / PostgreSQL / MySQL / TiDB. Los campos Auth se embeben en la URL de conexión mediante `replacen("://", "://user:pass@")`, y solo surten efecto cuando la URL no contiene `@`.

### 15 ─ Soporte de autenticación con certificados TLS + crate ecat-tls (los 12 backends)

Nuevo crate `ecat-tls` que proporciona:
- `TlsClientConfig` — configuración TLS opcional (ca_cert, client_cert, client_key, skip_verify)
- `generate_ca()` — generación de certificado CA autofirmado
- `generate_server_cert()` — generación de certificado de servidor
- `generate_client_cert()` — generación de certificado de cliente (mTLS)

Los Config de los 12 backends de datos añaden el campo `#[serde(default)] tls: Option<TlsClientConfig>`.

| Tipo de backend | Forma TLS |
|----------|----------|
| 9 backends HTTP | `tls.build_reqwest_client()` construye el cliente reqwest TLS |
| Redis | cambio de scheme de URL `redis://` → `rediss://` |
| Sqlx | campo reservado (TLS mediante parámetro de URL `?sslmode=require`) |
| Memcached | campo reservado (reservado para implementación de red) |

---

## 1. Resumen

| Elemento | Estado | Detalle |
|------|------|------|
| `cargo build` | ✅ Correcto | 3 warnings del compilador, 19.85s |
| `cargo test` | ✅ Correcto | ~137 tests unitarios todos correctos, 0 fallos, 1 ignorado |
| `cargo clippy` | ⚠️ Con warnings | 5 lint warnings en 3 crates |
| `cargo fmt` | ✅ Correcto | sin problemas de formato |
| `cargo audit` | ❌ No instalado | no se puede escanear CVE conocidos |

---

## 2. Warnings del compilador (a corregir)

### 2.1 ecat-versioning (3 warnings)

**Archivo**: `ecat-versioning/src/lib.rs`

| # | Warning | Línea | Gravedad |
|---|---------|------|----------|
| 1 | `unused import: axum::routing::get` | 3 | baja |
| 2 | `unused variable: version` | 61 | baja |
| 3 | `function extract_version is never used` | 68 | baja |

**Recomendación**: eliminar el import sin usar, cambiar `version` a `_version` y `extract_version` a `pub` o marcarlo `#[allow(dead_code)]`.

### 2.2 ecat-data-questdb (1 clippy warning)

**Archivo**: `ecat-data-questdb/src/lib.rs:39`

```rust
// 当前:
.query(&[("query", sql), ("count", &"true".to_string())])

// 应改为:
.query(&[("query", sql), ("count", &"true")])
```

### 2.3 ecat-client (1 clippy warning)

**Archivo**: `ecat-client/src/lib.rs:249`

`GrpcClientBuilder` implementa `Default` manualmente; se puede sustituir directamente por `#[derive(Default)]`.

---

## 3. Resumen de lint warnings de Clippy

| Crate | Warning | Tipo |
|-------|---------|------|
| ecat-versioning | `useless_format!` — usar `"/api".to_string()` | rendimiento |
| ecat-versioning | import sin usar / código muerto | limpieza |
| ecat-data-questdb | `unnecessary_to_owned` | rendimiento |
| ecat-client | `derivable_impls` — usar derive Default | simplificación |

---

## 4. Análisis de cobertura de tests

### 4.1 Estadísticas

| Métrica | Valor |
|------|------|
| Total de tests unitarios | ~137 |
| Fallos | 0 |
| Ignorados | 1 |
| Crates con tests | ~24 / 48 |
| **Crates con 0 tests** | **~24 / 48 (50%)** |

### 4.2 Crates con tests insuficientes (0 o solo de construcción)

Los siguientes crates tienen cobertura débil:

- ecat-data-arangodb, ecat-data-clickhouse, ecat-data-elasticsearch
- ecat-data-influxdb, ecat-data-iotdb, ecat-data-nebulagraph
- ecat-data-neo4j, ecat-data-opensearch, ecat-data-questdb
- ecat-data-redis, ecat-data-sqlx, ecat-data-memcached
- ecat-mq, ecat-mq-kafka, ecat-graphql, ecat-openapi
- ecat-transport, ecat-transport-grpc, ecat-transport-http
- ecat-transport-ws, ecat-tracing, ecat-logging
- ecat-middleware, ecat-registry-consul, ecat-registry-etcd

### 4.3 Doc-tests

Los **48 crates tienen 0 doc-tests**. No hay ejemplos `/// ````rust` en el código.

---

## 5. Problemas de dependencias

### 5.1 ⚠️ yaml_serde vs serde_yaml (riesgo medio)

**Archivo**: `ecat-config/Cargo.toml:9`

```toml
yaml_serde = "0.10"
```

La librería YAML estándar del ecosistema Rust es `serde_yaml` (última versión `0.9.34+`), mientras que `yaml_serde` es un crate **diferente y menos mantenido**.

**Recomendación**: confirmar si `yaml_serde` es la dependencia prevista. Si la intención era `serde_yaml`, sustitúyela.

### 5.2 Falta cargo-audit

`cargo audit` no está instalado. Se recomienda `cargo install cargo-audit` e integrarlo en el CI.

### 5.3 Falta el campo description

`[workspace.package]` no tiene `description`, y ningún sub-crate define description.

---

## 6. Problemas de calidad de código

### 6.1 unwrap/expect en código de producción

| Archivo | Línea | Llamada | Riesgo |
|------|------|------|------|
| `ecat-client/src/lib.rs` | 28 | `.expect("StaticResolver poisoned")` | bajo — razonable |
| `ecat/src/signal.rs` | 8 | `.expect("failed to install SIGTERM handler")` | medio — panic al arrancar |
| `ecat-protos/build.rs` | 5 | `.unwrap()` | bajo — build script |

### 6.2 extract_version de ecat-versioning

La función `extract_version` (línea 68) implementa la extracción del número de versión desde el header Accept, pero `build_header_router()` no la llama.

### 6.3 Manejo de errores de ecat-data-questdb

```rust
// 第 30 行: 网络响应体读取使用 unwrap_or_default
Err(RdbmsError::Database(resp.text().await.unwrap_or_default()))
```

Cuando `resp.text()` falla, el mensaje de error se traga en silencio. Se recomienda cambiar a `unwrap_or_else(|e| format!("questdb parse: {e}"))`.

---

## 7. Evaluación de arquitectura

### Puntos fuertes

- los 48 crates tienen separación de responsabilidades clara
- versión unificada del workspace `version.workspace = true`
- dependencias reducidas, sin frameworks pesados
- sin TODO/FIXME/HACK

### A mejorar

| Problema | Prioridad |
|------|--------|
| 50% de crates sin tests | alta |
| confusión yaml_serde vs serde_yaml | media |
| falta cargo-audit | media |
| código muerto en ecat-versioning | baja |
| sin doc-tests | baja |

---

## 8. Resumen de seguridad

| Comprobación | Resultado |
|--------|------|
| Claves hardcodeadas | no encontradas |
| Fuga de archivos .env | no encontrada |
| unwrap peligroso (código de producción) | 2 (signal.rs, client.rs) |
| Escaneo CVE | no ejecutado (requiere instalar cargo-audit) |

---

## 9. Plan de acción

### P0 — corregir de inmediato
1. limpiar los 3 warnings del compilador de ecat-versioning
2. corregir el clippy de ecat-data-questdb
3. corregir derivable_impls de ecat-client

### P1 — corto plazo
4. instalar `cargo-audit` para escanear vulnerabilidades de dependencias
5. confirmar la elección entre `yaml_serde` y `serde_yaml`
6. añadir doc-tests a los crates núcleo

### P2 — medio plazo
7. añadir tests a los crates transport/data/security
8. añadir el campo `description` a todos los crates
9. integrar o eliminar `extract_version`

### P3 — largo plazo
10. establecer CI: build → test → clippy → audit → coverage

---

*Informe generado el 2026-08-01. Cadena de herramientas: cargo 1.92.0, rustc 1.92.0, clippy 1.92.0*
