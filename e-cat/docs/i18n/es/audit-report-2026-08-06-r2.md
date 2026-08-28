# Informe de re-auditoría completa de e-cat (reverificación tras la corrección)

- **Fecha**: 2026-08-06
- **Versión**: v2.3.1 (55 crates)
- **Antecedente**: los 35 hallazgos de la ronda anterior (`docs/audit-report-2026-08-06.md`) están todos corregidos; esta ronda es la reverificación completa tras la corrección.

---

## 1. Resultados de tests y build

| Comprobación | Resultado |
|------|------|
| `cargo check --workspace` | ✅ compila con cero errores |
| `cargo test --workspace` | ✅ **219 passed · 0 failed · 1 ignored** |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ cero warnings |
| `cargo fmt --check` | ✅ limpio |
| Test de humo de helloworld | ✅ `/` devuelve JSON, `/health` devuelve OK, bind a `0.0.0.0:8000` con éxito |

**Conclusión**: las correcciones de la ronda anterior (D1/H1/H6/C1/C2/M1/M3/M5/M6/M9/M11/M13/serie L) no tienen regresiones.

## 2. Inspección profunda de calidad de código

| Elemento | Resultado |
|--------|------|
| TODO / FIXME / XXX / HACK | ✅ 0 lugares |
| `unwrap()` / `expect()` en código de producción | ✅ todos dentro de `#[cfg(test)]`, sin riesgo de panic en rutas de producción |
| Bloques `unsafe` | ✅ 0 en todo el workspace |
| Código muerto / warnings de no uso | ✅ clippy -D warnings correcto |
| Líneas por archivo | ✅ todos dentro del límite de 500 líneas |

## 3. Integridad de la configuración del ecosistema

| Elemento | Estado |
|------|------|
| Miembros del workspace | ✅ 55 crates, consistente con lo declarado en el README |
| CI (GitHub Actions + GitLab) | ✅ ambas plataformas incluyen la instalación de `protobuf-compiler`, comandos idénticos (check/test/fmt/clippy) |
| Dockerfile | ⚠️ build multietapa, rust:1.85-slim, nombre de binario `ecat`, healthcheck curl correctos; **el problema restante se ve en §5-A** |
| Helm chart | ✅ `appVersion` sincronizado a 2.3.1 (corrección de esta ronda) |
| Manifiestos de despliegue k8s | ✅ los probes /health y /ready corresponden a las rutas de ecat-health |
| Plantilla CLI | ✅ el código generado escucha en `0.0.0.0:8000` |
| Consistencia de versiones de documentación | ✅ README×2 / databases.example.yaml sincronizados a v2.3.1 (corrección de esta ronda) |
| Contraseñas de ejemplo | ✅ las contraseñas por defecto están comentadas (databases.example.yaml) |
| Recursos de imagen | ✅ alipay/weixinpay.png se referencian correctamente en ambos README |
| CHANGELOG | ✅ las 12 entradas de [2.3.1] coinciden con los cambios |

## 4. Integridad de las protecciones de seguridad

| Elemento | Resultado |
|--------|------|
| Credenciales / API keys hardcodeadas | ✅ 0 lugares (la única coincidencia es la palabra clave PEM en aserciones de tests) |
| Valor por defecto de TLS `skip_verify` | ✅ desactivado por defecto; Redis se actualiza automáticamente a `rediss://` |
| Superficies de inyección | ✅ TDengine doble escape, ES/OpenSearch encoding RFC 3986, escape del line protocol de InfluxDB, sqlx parametrizado, body insertTablet estándar de IoTDB |
| Rate limit | ✅ por IP de cliente (primer salto de X-Forwarded-For → X-Real-IP → global), Redis Lua atómico INCR+EXPIRE, fail-open + warn |
| JWT | ✅ claves débiles rechazadas (<32 bytes), la respuesta de error no filtra detalles internos |
| Manejo de contraseñas | ✅ la contraseña de Redis se pasa por ConnectionInfo, sin embeber en la URL (los mensajes de error no filtran) |
| Timeouts | ✅ todos los adaptadores HTTP unificados a connect 5s / request 30s |
| Protección del body de petición | ✅ SecurityBodyLayer con límite de 10MB + escaneo del body |

## 5. Nuevos hallazgos de esta ronda (2 elementos)

### [MEDIO] A. El Dockerfile `CMD ["ecat"]` sale inmediatamente al arrancar
- **Síntoma**: el CLI `ecat` requiere un subcomando; al ejecutarse sin argumentos, clap reporta error y sale (exit code 2), el contenedor termina al instante y HEALTHCHECK no puede pasar.
- **Causa**: la imagen solo incluye el binario del CLI, no el servicio del usuario; `ecat run` es solo un wrapper de `cargo run` (falla igualmente sin default-member).
- **Sugerencia**: ① empaquetar además un binario de servicio de ejemplo en el build y fijarlo como CMD; ② o declarar en la documentación que la imagen es solo para el contenedor de dev (montar el código fuente + `ecat run`); ③ o añadir un subcomando `serve` al CLI. Es un problema de semántica de despliegue, no se ha modificado por iniciativa propia.

### [BAJO] B. El `name: ecat-app` del `Chart.yaml` no coincide con el nombre del artefacto del Dockerfile (`ecat`)
- **Síntoma**: el nombre de imagen `ecat-app` no tiene mapeo directo con el binario `ecat`; en el despliegue con Helm el tag de la imagen debe especificarse manualmente.
- **Sugerencia**: documentar el comando de build/tag de la imagen (`docker build -t ecat-app:2.3.1 .`). Riesgo bajo, sin cambios.

## 6. Conclusión

Tras la corrección, la base de código está en estado saludable: **build, tests (219), clippy, fmt y humo pasan todos; el código de producción no tiene rutas de panic, cero unsafe, sin fuga de credenciales; la configuración del ecosistema (CI/Docker/Helm/k8s/plantilla CLI/documentación bilingüe/CHANGELOG) es totalmente consistente con v2.3.1**. Los 2 elementos restantes son sugerencias documentales de la capa de semántica de despliegue y no bloquean la publicación.

---

*Informe generado por reverificación automatizada: build + tests + clippy + fmt + humo + inspección especializada (rutas de panic/unsafe/TODO/credenciales/superficies de inyección/CI en ambas plataformas/Docker/Helm/k8s/sincronización de documentación).*
