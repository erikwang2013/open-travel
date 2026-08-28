# Informe de auditoría especializada (seguridad y rendimiento) — 2026-08-14

Alcance de la auditoría: workspace de 55 crates (v2.3.5). Método: verificación manual de Cargo.lock (cargo-audit no instalado), auditoría de código de las rutas de autenticación/TLS, comprobación de concurrencia y ciclo de vida de recursos. No se ha enviado código.

## Verificación de CVE en dependencias

- Las versiones de las dependencias principales son relativamente recientes y sin CVEs conocidos sin corregir: rustls 0.23.43, ring 0.17.14, aws-lc-rs 1.17.3, jsonwebtoken 9.3.1, tokio 1.53.1, h2 0.4.15, quinn 0.11.11, sqlx 0.8.6, zerocopy 0.8.55, time 0.3.54, openssl 0.10.81.
- hyper 0.14.32 (solo proviene de rust-s3 0.35.1, vía hyper-tls 0.5) está por encima de la línea de corrección 0.14.28.
- Nota: el CI no tiene instalado cargo-audit; se sugiere añadirlo al flujo de trabajo para la verificación automatizada.

## Hallazgos (ordenados por gravedad)

### S1 [Media] Handshake TLS HTTP serializado → DoS por handshakes lentos
- Ubicación: `ecat-transport-http/src/lib.rs:134-150` (TlsListener::accept)
- Síntoma: el handshake TLS se completa sincrónicamente dentro de `accept()`; axum::serve llama a accept en serie — una conexión que no completa el handshake bloquea todo el bucle de accept.
- Impacto: un atacante que establezca conexiones TCP lentas/zombi en masa puede detener por completo la aceptación de nuevas conexiones del servicio (en el lado gRPC, tonic hace spawn del handshake por conexión, no se ve afectado).
- Sugerencia: tras accept, `tokio::spawn` para el handshake con `tokio::time::timeout(10s)`; cerrar la conexión si falla.

### S2 [Media] Crecimiento sin límite de la caché de introspección OAuth2 → DoS de memoria
- Ubicación: `ecat-auth/src/oauth2.rs:45,84-92`
- Síntoma: `HashMap<String,(String,Instant)>` con el token como clave; el TTL solo controla la frescura, sin límite de capacidad ni expulsión.
- Impacto: peticiones masivas con tokens únicos pueden hacer crecer la memoria sin límite (cada miss además dispara la introspección upstream).
- Sugerencia: añadir límite de capacidad (p. ej. 10k) + limpieza periódica, o cambiar a moka/LRU con expulsión por capacidad y TTL.

### S3 [Baja-media] ecat-data-s3 usa la versión antigua rust-s3 0.35.1 (hyper 0.14 + native-tls/openssl)
- Ubicación: `ecat-data-s3/Cargo.toml` → rust-s3 0.35.1
- Síntoma: el cliente S3 usa de forma independiente la pila hyper-tls/openssl; `ecat-tls::TlsClientConfig` (CA personalizada, certificado de cliente, skip_verify) no tiene efecto en S3; las superficies de configuración TLS no son consistentes.
- Impacto: en entornos empresariales no se puede configurar CA privada/mTLS de S3; la dependencia apenas se mantiene desde 2023.
- Sugerencia: evaluar la actualización de rust-s3 o cambiar a un cliente unificado reqwest/rustls.

### S4 [Baja] La validación JWT por defecto no incluye iss/aud
- Ubicación: `ecat-auth/src/jwt.rs:125` — `Validation::new(HS256)` solo firma+exp.
- Impacto: con clave compartida HS256, el token de un servicio puede ser aceptado por otro (sin aislamiento de emisor).
- Sugerencia: que la documentación exija explícitamente configurar issuer/audience en producción; o añadir una entrada de validación de iss por defecto.

### S5 [Baja] TlsClientConfig.skip_verify por sí solo hace que is_enabled() sea verdadero
- Ubicación: `ecat-tls/src/lib.rs:23-29`
- Síntoma: al configurar solo `skip_verify: true`, el TLS se considera "habilitado" sin verificar certificados, desactivando la verificación en silencio.
- Sugerencia: validación mutuamente excluyente de skip_verify y ca_cert, o exigir una doble confirmación explícita.

## Rendimiento y recursos

### P1 [Baja] La ruta de acierto de la caché OAuth2 deserializa JSON en cada petición
- Ubicación: `ecat-auth/src/oauth2.rs:87` — la caché guarda la cadena serializada; tras el acierto aún se hace `serde_json::from_str`.
- Sugerencia: guardar directamente la estructura `AuthClaims` en la caché, ahorrando el parse por petición.

### P2 [Baja] ecat-bench sin calentamiento ni criterio de estado estable
- Ubicación: `ecat-bench/src/lib.rs:run_bench` — cronometra directamente, sin warmup; el arranque en frío/primera asignación del pool de conexiones se mezcla en el p99.
- Sugerencia: añadir rondas de calentamiento y criterio de convergencia a estado estable para resultados más fiables.

### P3 [Baja] El consumidor Kafka serializa 100ms de poll + 100ms de sleep
- Ubicación: `ecat-mq-kafka/src/lib.rs:84-92` — el límite superior de latencia extremo a extremo del mensaje es de unos 200ms.
- Sugerencia: tras poll no hace falta dormir; en escenarios de bajo throughput se puede acortar el intervalo de poll.

## Confirmación de buenas prácticas

- Sin unwrap/expect/panic en rutas de producción (transport/auth/middleware solo en tests).
- El fallback de API key por parámetro query lleva log de advertencia de fuga; HashMap usa SipHash anti-colisión.
- La capa SQL pasa el SQL del llamador (naturaleza de framework); el user:pass de la cadena de conexión está percent-encoded correctamente.
- Cuando el canal de consumo de Kafka está lleno, bloquea con backpressure en lugar de descartar; tras el drop de rx, la tarea de poll sale correctamente.
- El fetch de config-remote lleva timeout (5s/30s); las consultas bloqueantes reportan error si falta el índice para evitar busy-wait.

---

## Auditoría de corrección del dominio central (complemento, complementaria a la especializada de seguridad/rendimiento anterior)

Método de auditoría: escaneo del código de producción de todo el workspace (localización de unwrap/expect/panic, errores tragados en silencio, detención asíncrona, estado de concurrencia) + reverificación completa con `cargo test --workspace` (primera ronda toda en verde; la corrección de S1 en curso provocó warnings de compilación intermedios en transport-http; tras el cierre hay que volver a ejecutar). No se ha enviado código.

### N1 [Media] Fuga de handle tras la salida de la tarea de consumo de ecat-events → pérdida silenciosa de eventos
- Ubicación: `ecat-events/src/lib.rs:97-101` (el bucle de consumo en las líneas 89-95 hace `None => break`)
- Síntoma: cuando el stream de mq devuelve None (p. ej. el broadcast channel de kafka se cierra) o la tarea entra en panic, el bucle de consumo sale, pero el JoinHandle permanece en el map `consumers`; después, un `subscribe()` del mismo tipo de evento no reinicia la tarea de consumo porque el `contains_key` de la línea 68 siempre es verdadero → los eventos de ese tipo se pierden para siempre en silencio.
- Impacto: tras la interrupción del flujo de eventos remoto no hay auto-curación; la recuperación exige reiniciar el proceso.
- Sugerencia: en la ruta de salida de la tarea, eliminar el handle del map (spawn de un watcher o limpieza perezosa con `handle.is_finished()`).

### N2 [Media] Semántica incorrecta de group_id en subscribe de ecat-mq-kafka
- Ubicación: `ecat-mq-kafka/src/lib.rs:71-84`
- a. Cuando `group_id` es None por defecto, `consumer.subscribe()` de rdkafka exige group.id (librdkafka reporta INVALID_ARG); con la configuración por defecto la suscripción probablemente falla directamente (necesita verificación en máquina real).
- b. Con group_id configurado (ecat-events hace subscribe una vez por tipo de evento, mismo group), Kafka reparte el topic entre los múltiples consumidores del mismo grupo por particiones → un tipo de evento puede caer en la tarea de consumo de otro tipo y descartarse en silencio (auto.offset.reset=latest y sin commit).
- Impacto: el bus de eventos pierde eventos bajo el backend de kafka.
- Sugerencia: generar un group.id aleatorio único cuando no haya group_id; o usar assign() en el lado del consumidor para asignar particiones explícitamente; la documentación debe aclarar que las suscripciones múltiples requieren groups independientes.

### N3 [Baja] GrpcServer/WsServer no normalizan el host vacío (corrección de D1 incompleta)
- Ubicación: `ecat-transport-grpc/src/lib.rs:52`, `ecat-transport-ws/src/lib.rs:58`
- Síntoma: `addr.parse::<SocketAddr>()` de `GrpcServer::new(":8000")` devuelve AddrParseError (verificado con pruebas reales); `TcpListener::bind(":8000")` de WsServer resuelve al comodín IPv6 y falla el arranque en entornos sin IPv6. HttpServer ya normaliza a 0.0.0.0; las tres APIs de server se comportan de forma inconsistente.
- Sugerencia: unificar la normalización del host vacío dentro de new.

### N4 [Baja] TracingLayer no inyecta trace_id, no coincide con lo declarado en CHANGELOG 2.3.3
- Ubicación: `ecat-tracing/src/lib.rs:72-84` (el span solo contiene el campo service; el comentario del código admite que el Req genérico no puede leer los headers); `inject_trace_id()` genera un UUID nuevo cada vez, sin reutilizar el trace_id extraído upstream.
- Impacto: el trazado distribuido configurado según la documentación no puede correlacionar entre servicios.
- Sugerencia: binding diferido del campo del span o especialización para `http::Request<B>`; inject soporta portar el id upstream.

### N5 [Baja] El panic de un job de ecat-scheduler lo detiene en silencio
- Ubicación: `ecat-scheduler/src/lib.rs:53-57,83` (`let _ = handle.await` en `run()`)
- Síntoma: tras un panic del job programado, la tarea muere sin reinicio ni log; `run()` descarta el error del JoinHandle.
- Sugerencia: capturar el panic con log + política de reinicio opcional.

### N6 [Baja] unwrap residual en código de producción (rutas de envenenamiento/panic)
- `ecat-events/src/lib.rs:68,98` `Mutex::lock().unwrap()` de std (panic si envenenado); `ecat-versioning/src/lib.rs:86` unwrap del Response builder (no puede fallar pero es ruta de panic); `ecat-mq/src/lib.rs:110` expect ya está protegido por guarda is_none (seguro).
- Sugerencia: en los dos lugares de events, cambiar a `unwrap_or_else(|e| e.into_inner())`.

### N7 [Info] WsServer::stop() no espera las conexiones WebSocket ya actualizadas
- Ubicación: `ecat-transport-ws/src/lib.rs:63-87`
- Las conexiones axum on_upgrade corren en tareas independientes, el graceful shutdown no las cubre; los handlers de conexiones largas permanecen tras stop(), el proceso no sale limpiamente (semántica de App::stop incompleta).

### N8 [Info] Crates con cero tests: ecat-data / ecat-lock / ecat-protos
- Son todos crates de traits/definiciones; se ha verificado que los métodos por defecto fallan ruidosamente (devuelven error en lugar de silencioso), pero los contratos de traits (semántica de rollback de Transaction en drop, validación de token del lock) no tienen ningún test unitario.
- Sugerencia: añadir tests unitarios mínimos para la semántica de RdbmsError/Transaction y DistributedLock.

### N9 [Info] Los parámetros y campos anidados de graphql siguen descartándose
- En `ecat-graphql/src/lib.rs` execute solo pasa `variables` al resolver; los parámetros de campo de `{ hello(name: "x") }` y las selecciones anidadas no se pasan; el README no indica la limitación (el L8 del informe antiguo exigía documentarlo; tras la reescritura 2.3.3 sigue sin hacerse).

### N10 [Info] circuit-breaker solo cuenta errores de la capa de transporte
- `ecat-circuit-breaker/src/lib.rs:203-209` solo registra el Err interno como fallo; los HTTP 5xx cuentan como éxito → el disyuntor no sirve contra la indisponibilidad del servicio (tormentas de 5xx); la documentación no lo indica.

**Estado de verificación**: la primera ronda de `cargo test --workspace` fue toda verde (incluidos doc-tests; no se vio ningún fallo al final de la salida); durante la edición de la corrección de S1, transport-http mostró un error de compilación y 2 warnings (import sin usar `ensure_crypto_provider`, `shutdown_tx` sin leer) — estado intermedio; tras cerrar S1 hay que re-ejecutar los tests completos y `clippy --all-targets -D warnings`.

---

## Tercera ronda: validación dinámica + revisión de CVE + superficie de panic (especializada, 2026-08-14)

### Revisión de CVE (nuevos hallazgos, por gravedad)

1. **[Media] rustls-webpki 0.102.8 permanece en el árbol de dependencias** (RUSTSEC-2026-0049/0098/0099/0104: bypass de distributionPoint de CRL, restricciones de nombre URI/wildcard; versión corregida 0.103.10). La cadena principal es 0.103.13 (vía rustls 0.23.43, segura); 0.102.8 entra vía async-nats 0.38.0 / rumqttc 0.25.1, cubre las cadenas del cliente TLS de NATS/MQTT. Upstream no ha migrado a rustls 0.23, no hay versión corregida — riesgo controlado, se sugiere seguir con comentario de seguimiento.
2. **[Media-baja] rdkafka 0.36.2 embebe librdkafka con cJSON 1.7.14** (CVE-2023-53154 y la serie cJSON; CVE-2025-57052 marca CVSS 9.8, pero el archivo afectado cJSON_utils.c no lo usa librdkafka, aplicabilidad dudosa). La corrección upstream está en librdkafka 2.10+ (PR #5346 de 2026-03). ecat-mq-kafka enlaza estáticamente; hay que comprobar la versión empaquetada de librdkafka-sys y seguir la actualización.
3. **[Baja] rustls-pemfile 2.2.0 sin mantenimiento** (RUSTSEC-2025-0134) — ecat-transport-http lo usa en el arranque para parsear archivos locales, no es entrada del atacante.
4. **[Baja] rsa 0.9.10** (RUSTSEC-2023-0071, canal lateral de timing Marvin) — entra vía el TLS de sqlx-mysql, relevante solo en escenarios de MySQL + intercambio de claves RSA.
5. async-nats 0.38.0 ya está por encima de la línea de corrección de RUSTSEC-2023-0027 (bypass de validación de CN), sin problemas.

### Validación dinámica (examples/helloworld, build de debug, puerto temporal 18080, ya limpiado)

- /health 200, / (serialización JSON) 200 (27B), 404 normal; el middleware Logging registra las peticiones correctamente.
- **/metrics montado pero devuelve 200 + body vacío (0 bytes)**: sin métricas registradas no hay ninguna salida; el lado de monitorización no puede distinguir "sano/sin métricas". Se sugiere una línea de comentario en el registry vacío o 503.
- Peticiones malformadas (headers con 0x01/0x02) → 400 Bad Request; el servicio sigue vivo, el /health posterior sigue 200, sin panic.
- Rutas TLS/mTLS y middleware de disyuntor/rate limit: cubiertas por los tests de ecat-transport-http/grpc y ecat-middleware (tras la corrección de la carrera de mTLS, todas en verde; pasan los casos que rechazan certificados anónimos/incorrectos).

### Línea base de bench

- ecat-bench no tiene targets [[bench]]/bin, no hay entrada para cargo bench; run_bench_with_warmup ya incluye calentamiento (la corrección de P2 aterrizada), los tests del harness están todos en verde.
- La medición real fue un smoke de build de debug: / sobre 1.3ms, /health sobre 1.8ms (incluye el overhead del proceso curl, sin valor de línea base). Se sugiere build de release + presión con wrk/hey para obtener una línea base real.

### Revisión de la superficie de panic (todo el workspace, excluidos los módulos de tests)

- En total 31 lugares de unwrap/expect/panic, todos de bajo riesgo: `Response::builder().body().unwrap()` (ramas infalibles de jwt/apikey/oauth2), respaldo de envenenamiento de locks (etcd/testing), `serde_json::to_string().unwrap()` de clickhouse (panic teórico ante entradas NaN/inf extremas).
- **1 lugar a tener en cuenta**: `ecat-transport-http/src/tls_listener.rs:234` — cuando el bucle de accept en background sale de forma anómala, `panic!` dentro de `accept()`, el hilo del servicio muere (condición de disparo estricta: solo errores fatales del listener); se sugiere degradar a devolución de error con log.
