// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use axum::Router;
use ecat_transport::{Server as TransportServer, TlsConfig, normalize_addr};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::watch;

mod tls_listener;

use tls_listener::build_server_config;

pub struct HttpServer {
    addr: String,
    router: Option<Router>,
    shutdown_tx: Mutex<Option<watch::Sender<()>>>,
    tls_config: Option<TlsConfig>,
}

impl HttpServer {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            // 空 host（如 ":8000"）会解析到 IPv6 通配 [::]，在无 IPv6 环境绑定失败；
            // 规范化为 IPv4 通配 "0.0.0.0"
            addr: normalize_addr(addr.into()),
            router: None,
            shutdown_tx: Mutex::new(None),
            tls_config: None,
        }
    }

    pub fn router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }

    pub fn tls(mut self, config: TlsConfig) -> Self {
        self.tls_config = Some(config);
        self
    }
}

impl HttpServer {
    /// 用户 router 与内置 /metrics 端点合并。
    /// /metrics 为框架保留路径：用户 router 若也定义该路径，merge 会 panic，
    /// 此时捕获 panic、记录 warn 并降级为用户 router（不挂 /metrics）。
    fn merged_router(&self) -> Router {
        let user = self.router.clone().unwrap_or_default();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            user.clone().merge(ecat_metrics::metrics_router())
        })) {
            Ok(merged) => merged,
            Err(_) => {
                tracing::warn!(
                    "user router defines /metrics; serving user route, framework metrics disabled"
                );
                user
            }
        }
    }
}

#[async_trait::async_trait]
impl TransportServer for HttpServer {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let router = self.merged_router();
        let listener = TcpListener::bind(&self.addr).await?;
        let (tx, mut rx) = watch::channel(());
        *self.shutdown_tx.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
        let shutdown_signal = async move {
            let _ = rx.changed().await;
        };
        if let Some(tls) = &self.tls_config {
            let server_config = build_server_config(tls)?;
            let tls_listener = tls_listener::TlsListener::new(
                listener,
                tokio_rustls::TlsAcceptor::from(Arc::new(server_config)),
            );
            axum::serve(tls_listener, router)
                .with_graceful_shutdown(shutdown_signal)
                .await?;
        } else {
            axum::serve(listener, router)
                .with_graceful_shutdown(shutdown_signal)
                .await?;
        }
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(tx) = self
            .shutdown_tx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            let _ = tx.send(());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{response::IntoResponse, routing::get};
    use tls_listener::ensure_crypto_provider;
    use tokio::net::TcpStream;

    async fn health() -> impl IntoResponse {
        "ok"
    }

    #[test]
    fn new_sets_addr() {
        let srv = HttpServer::new("0.0.0.0:9000");
        assert_eq!(srv.addr, "0.0.0.0:9000");
    }

    #[test]
    fn new_normalizes_bare_port_to_ipv4_wildcard() {
        let srv = HttpServer::new(":9000");
        assert_eq!(srv.addr, "0.0.0.0:9000");
    }

    #[test]
    fn router_sets_router() {
        let router = Router::new().route("/health", get(health));
        let srv = HttpServer::new("0.0.0.0:9000").router(router);
        assert!(srv.router.is_some());
    }

    #[test]
    fn new_without_router_has_none() {
        let srv = HttpServer::new("0.0.0.0:9000");
        assert!(srv.router.is_none());
    }

    #[tokio::test]
    async fn auto_mounts_metrics_endpoint() {
        use tower::ServiceExt;
        let srv = HttpServer::new("127.0.0.1:0");
        let resp = srv
            .merged_router()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }

    fn write_pem_files(
        dir: &std::path::Path,
        suffix: &str,
        pair: &ecat_tls::CertPair,
    ) -> (std::path::PathBuf, std::path::PathBuf) {
        let cert_path = dir.join(format!("{suffix}-cert.pem"));
        let key_path = dir.join(format!("{suffix}-key.pem"));
        std::fs::write(&cert_path, &pair.cert_pem).unwrap();
        std::fs::write(&key_path, &pair.key_pem).unwrap();
        (cert_path, key_path)
    }

    async fn free_port() -> u16 {
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        port
    }

    /// 在 TLS 连接上发起 /health 请求并读取响应（超时按空处理）。
    async fn request_over_tls(mut tls: tokio_rustls::client::TlsStream<TcpStream>) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let _ = tls
            .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await;
        let mut buf = Vec::new();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), tls.read_to_end(&mut buf))
            .await;
        String::from_utf8_lossy(&buf).into_owned()
    }

    fn client_config(
        root_pem: &str,
        client_pair: Option<&ecat_tls::CertPair>,
    ) -> Result<rustls::ClientConfig, String> {
        ensure_crypto_provider();
        let mut roots = rustls::RootCertStore::empty();
        for cert in rustls_pemfile::certs(&mut std::io::BufReader::new(root_pem.as_bytes()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
        {
            roots.add(cert).map_err(|e| e.to_string())?;
        }
        let builder = rustls::ClientConfig::builder().with_root_certificates(roots);
        match client_pair {
            Some(pair) => {
                let mut pem = std::io::BufReader::new(pair.cert_pem.as_bytes());
                let certs = rustls_pemfile::certs(&mut pem)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| format!("parse client cert: {e}"))?;
                let key = rustls_pemfile::private_key(&mut std::io::BufReader::new(
                    pair.key_pem.as_bytes(),
                ))
                .map_err(|e| format!("parse client key: {e}"))?
                .ok_or_else(|| "no client private key".to_string())?;
                builder
                    .with_client_auth_cert(certs, key)
                    .map_err(|e| e.to_string())
            }
            None => Ok(builder.with_no_client_auth()),
        }
    }

    async fn tls_client(
        root_pem: &str,
        client_pair: Option<&ecat_tls::CertPair>,
        port: u16,
    ) -> Result<tokio_rustls::client::TlsStream<TcpStream>, String> {
        let cfg = client_config(root_pem, client_pair)?;
        let connector = tokio_rustls::TlsConnector::from(Arc::new(cfg));
        let server_name =
            rustls::pki_types::ServerName::try_from("localhost").map_err(|e| e.to_string())?;
        // 服务端绑定存在竞争窗口：连接被拒时重试，直到超时。
        let mut last_err = String::new();
        for _ in 0..50 {
            let stream = match TcpStream::connect(("127.0.0.1", port)).await {
                Ok(stream) => stream,
                Err(e) => {
                    last_err = format!("connect: {e}");
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            };
            return match connector.connect(server_name.clone(), stream).await {
                Ok(tls) => Ok(tls),
                Err(e) => Err(format!("tls handshake: {e}")),
            };
        }
        Err(last_err)
    }

    /// 经 UnixStream 对驱动双方握手：server 在阻塞线程上运行，本线程驱动 client。
    /// 返回 (server 侧结果, client 侧结果)。注意 TLS 1.3 中服务端在其首条 flight
    /// 就发送 Finished，客户端在服务端拒绝（如 mTLS 校验失败）前已完成自身握手，
    /// 因此"拒绝"以 server 侧结果为准。
    fn in_memory_handshake(
        server_cfg: rustls::ServerConfig,
        client_cfg: rustls::ClientConfig,
    ) -> (Result<(), String>, Result<(), String>) {
        use std::os::unix::net::UnixStream;
        let (server_side, mut client_side) = UnixStream::pair().expect("unix stream pair");
        let server_task = std::thread::spawn(move || {
            let mut server =
                rustls::ServerConnection::new(Arc::new(server_cfg)).map_err(|e| e.to_string())?;
            loop {
                match server.complete_io(&mut &server_side) {
                    Ok(_) if server.is_handshaking() => {}
                    Ok(_) => return Ok::<(), String>(()),
                    Err(e) => return Err(format!("server: {e}")),
                }
            }
        });
        let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
        let mut client = match rustls::ClientConnection::new(Arc::new(client_cfg), server_name) {
            Ok(c) => c,
            Err(e) => {
                let _ = server_task.join();
                return (Err(format!("{e}")), Err(format!("{e}")));
            }
        };
        let client_result = loop {
            match client.complete_io(&mut client_side) {
                Ok(_) if client.is_handshaking() => {}
                Ok(_) => break Ok(()),
                Err(e) => break Err(format!("client: {e}")),
            }
        };
        let server_result = server_task
            .join()
            .unwrap_or_else(|_| Err("server thread panicked".into()));
        (server_result, client_result)
    }

    fn client_cfg_without_cert(root_pem: &str) -> rustls::ClientConfig {
        client_config(root_pem, None).unwrap()
    }

    #[test]
    fn mtls_config_rejects_anonymous_client_in_memory() {
        let dir = std::env::temp_dir().join(format!("ecat-http-mtls-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let srv = ecat_tls::generate_server_cert("localhost").unwrap();
        let client = ecat_tls::generate_client_cert("test-client").unwrap();
        let (cert_path, key_path) = write_pem_files(&dir, "server", &srv);
        let ca_path = dir.join("client-ca.pem");
        std::fs::write(&ca_path, &client.cert_pem).unwrap();

        let server_cfg =
            build_server_config(&TlsConfig::new(cert_path, key_path).with_client_auth(ca_path))
                .unwrap();
        // 匿名客户端：服务端必须拒绝（TLS 1.3 下以服务端结果为准）。
        let (server_r, _client_r) =
            in_memory_handshake(server_cfg.clone(), client_cfg_without_cert(&srv.cert_pem));
        assert!(
            server_r.is_err(),
            "anonymous client must be rejected, got {server_r:?}"
        );

        // 不受信任的客户端证书：服务端必须拒绝。
        let wrong = ecat_tls::generate_client_cert("wrong-client").unwrap();
        let (server_r, _client_r) = in_memory_handshake(
            server_cfg.clone(),
            client_config(&srv.cert_pem, Some(&wrong)).unwrap(),
        );
        assert!(
            server_r.is_err(),
            "untrusted client cert must be rejected, got {server_r:?}"
        );

        // 受信任的客户端证书：双方握手成功。
        let (server_r, client_r) = in_memory_handshake(
            server_cfg,
            client_config(&srv.cert_pem, Some(&client)).unwrap(),
        );
        assert!(
            server_r.is_ok(),
            "trusted client must be accepted, got {server_r:?}"
        );
        assert!(
            client_r.is_ok(),
            "trusted client handshake failed: {client_r:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn tls_server_completes_handshake_and_serves_http() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let dir = std::env::temp_dir().join(format!("ecat-http-tls-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let srv = ecat_tls::generate_server_cert("localhost").unwrap();
        let (cert_path, key_path) = write_pem_files(&dir, "server", &srv);

        let port = free_port().await;
        let server = Arc::new(
            HttpServer::new(format!("127.0.0.1:{port}"))
                .router(Router::new().route("/health", get(health)))
                .tls(TlsConfig::new(cert_path, key_path)),
        );
        let task = tokio::spawn({
            let server = Arc::clone(&server);
            async move { server.start().await }
        });

        // 等待服务就绪后完成 TLS 握手并发一个 HTTP 请求。
        let mut tls = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tls_client(&srv.cert_pem, None, port),
        )
        .await
        .expect("connect timed out")
        .expect("tls handshake failed");
        tls.write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut buf = Vec::new();
        tls.read_to_end(&mut buf).await.unwrap();
        assert!(
            String::from_utf8_lossy(&buf).contains("200 OK"),
            "unexpected response: {}",
            String::from_utf8_lossy(&buf)
        );

        drop(tls);
        server.stop().await.unwrap();
        let _ = task.await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// S1 DoS 回归：慢速/僵尸 TLS 连接（只建 TCP、不发 ClientHello）不得阻塞
    /// accept 循环——有效客户端必须在僵尸连接存活期间快速完成握手并得到 200。
    /// 修复前握手在 accept() 内同步完成，axum::serve 串行调用 accept()，
    /// 一个僵尸连接就会卡住整个 accept 循环。
    #[tokio::test]
    async fn zombie_handshake_does_not_block_accept_loop() {
        let dir = std::env::temp_dir().join(format!("ecat-http-zombie-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let srv = ecat_tls::generate_server_cert("localhost").unwrap();
        let (cert_path, key_path) = write_pem_files(&dir, "server", &srv);

        let port = free_port().await;
        let server = Arc::new(
            HttpServer::new(format!("127.0.0.1:{port}"))
                .router(Router::new().route("/health", get(health)))
                .tls(TlsConfig::new(cert_path, key_path)),
        );
        let task = tokio::spawn({
            let server = Arc::clone(&server);
            async move { server.start().await }
        });

        // 僵尸连接：等服务绑定后建立 TCP 连接，不发任何数据并保持打开。
        let zombie = loop {
            match TcpStream::connect(("127.0.0.1", port)).await {
                Ok(stream) => break stream,
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
            }
        };
        // 让 accept 循环先接到僵尸连接（修复前会卡在它的握手上）。
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // 有效客户端必须不被僵尸阻塞：握手 + 请求在 3s 内完成并返回 200。
        let body = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            let tls = tls_client(&srv.cert_pem, None, port)
                .await
                .expect("valid client tls handshake failed");
            request_over_tls(tls).await
        })
        .await
        .expect("valid client blocked by zombie handshake");
        assert!(body.contains("200 OK"), "unexpected response: {body:?}");

        drop(zombie);
        server.stop().await.unwrap();
        let _ = task.await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn mtls_server_accepts_client_cert_and_rejects_missing() {
        let dir = std::env::temp_dir().join(format!("ecat-http-mtls-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let srv = ecat_tls::generate_server_cert("localhost").unwrap();
        let client = ecat_tls::generate_client_cert("test-client").unwrap();
        let (cert_path, key_path) = write_pem_files(&dir, "server", &srv);
        // 信任锚：把客户端自签证书本身作为 CA（rustls 支持自签证书作根）。
        let ca_path = dir.join("client-ca.pem");
        std::fs::write(&ca_path, &client.cert_pem).unwrap();

        let port = free_port().await;
        let server = Arc::new(
            HttpServer::new(format!("127.0.0.1:{port}"))
                .router(Router::new().route("/health", get(health)))
                .tls(TlsConfig::new(cert_path, key_path).with_client_auth(ca_path)),
        );
        let task = tokio::spawn({
            let server = Arc::clone(&server);
            async move { server.start().await }
        });

        // 带客户端证书：握手成功并发起 HTTP 请求得到 200。
        let ok = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tls_client(&srv.cert_pem, Some(&client), port),
        )
        .await
        .expect("connect timed out");
        assert!(ok.is_ok(), "client-cert handshake should succeed: {ok:?}");
        drop(ok);

        // 不带客户端证书：TLS 1.3 下客户端握手先完成，拒绝在后续 I/O 显现，
        // 因此断言请求无法得到 200。
        let missing = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tls_client(&srv.cert_pem, None, port),
        )
        .await
        .expect("connect timed out")
        .expect("tls handshake failed");
        let body = request_over_tls(missing).await;
        assert!(
            !body.contains("200 OK"),
            "handshake without client cert must fail: {body:?}"
        );

        // 用错误客户端证书：同样无法得到 200。
        let wrong = ecat_tls::generate_client_cert("wrong-client").unwrap();
        let bad = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tls_client(&srv.cert_pem, Some(&wrong), port),
        )
        .await
        .expect("connect timed out")
        .expect("tls handshake failed");
        let body = request_over_tls(bad).await;
        assert!(
            !body.contains("200 OK"),
            "handshake with untrusted client cert must fail: {body:?}"
        );

        server.stop().await.unwrap();
        let _ = task.await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn user_metrics_route_wins_without_panic() {
        use tower::ServiceExt;
        async fn user_metrics() -> impl IntoResponse {
            "user-metrics"
        }
        let router = Router::new().route("/metrics", get(user_metrics));
        let srv = HttpServer::new("127.0.0.1:0").router(router);
        let resp = srv
            .merged_router()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        assert_eq!(String::from_utf8(body.to_vec()).unwrap(), "user-metrics");
    }
}
