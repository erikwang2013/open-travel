// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};

/// 安装默认 rustls CryptoProvider（ring）。
/// 同时编译 aws-lc-rs 与 ring features 时，rustls 无法自动选择 provider，
/// 构造 ClientConfig/ServerConfig 会 panic；此处用 OnceLock 保证只安装一次。
pub(crate) fn ensure_crypto_provider() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let _ = rustls::crypto::CryptoProvider::install_default(
            rustls::crypto::ring::default_provider(),
        );
    });
}

/// 从 TlsConfig 构建 rustls 服务端配置：加载 cert/key，ca_cert_path +
/// require_client_auth 时要求并校验客户端证书（mTLS）。
pub(crate) fn build_server_config(
    tls: &ecat_transport::TlsConfig,
) -> Result<rustls::ServerConfig, Box<dyn std::error::Error + Send + Sync>> {
    ensure_crypto_provider();
    let certs = rustls_pemfile::certs(&mut std::io::BufReader::new(std::fs::File::open(
        &tls.cert_path,
    )?))
    .collect::<Result<Vec<_>, _>>()?;
    if certs.is_empty() {
        return Err(format!("no certificates found in {}", tls.cert_path.display()).into());
    }
    let key = rustls_pemfile::private_key(&mut std::io::BufReader::new(std::fs::File::open(
        &tls.key_path,
    )?))?
    .ok_or_else(|| format!("no private key found in {}", tls.key_path.display()))?;

    let builder = rustls::ServerConfig::builder();
    let config = if tls.require_client_auth {
        let ca_path = tls
            .ca_cert_path
            .as_ref()
            .ok_or("require_client_auth requires ca_cert_path")?;
        let ca_certs =
            rustls_pemfile::certs(&mut std::io::BufReader::new(std::fs::File::open(ca_path)?))
                .collect::<Result<Vec<_>, _>>()?;
        if ca_certs.is_empty() {
            return Err(format!("no CA certificates found in {}", ca_path.display()).into());
        }
        let mut roots = rustls::RootCertStore::empty();
        roots.add_parsable_certificates(ca_certs);
        let verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(roots)).build()?;
        builder.with_client_cert_verifier(verifier)
    } else {
        builder.with_no_client_auth()
    };
    let mut server_config = config.with_single_cert(certs, key)?;
    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(server_config)
}

/// TLS 握手超时：慢速/僵尸连接超过该时间即断开，避免长期占用握手任务。
const HANDSHAKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// 并发握手上限：防止握手风暴时无限 spawn 握手任务耗尽资源。
/// 达到上限后新连接直接断开（不排队等待），保证 accept 循环自身不阻塞。
const MAX_CONCURRENT_HANDSHAKES: usize = 1024;

/// 断开日志限频窗口：慢速攻击会持续触发并发上限断开，每连接一条
/// warn 会被刷成日志放大面；窗口内只记一条（含窗口内累计断开数）。
const DROP_LOG_WINDOW: std::time::Duration = std::time::Duration::from_secs(10);

/// 握手断开日志限频器：窗口内首次断开立即记日志，其余静默累计，
/// 窗口结束时补记累计数。仅 accept 循环单任务访问。
struct DropLimiter {
    last_log: std::time::Instant,
    dropped: u64,
    /// 本窗口是否还未记过"首次断开"日志；用独立标志区分首次与后续，
    /// 避免计数归零后无法辨别而每条都记（S1 日志放大修复）。
    first: bool,
}

impl DropLimiter {
    fn new() -> Self {
        Self {
            last_log: std::time::Instant::now(),
            dropped: 0,
            first: true,
        }
    }

    /// 记录一次断开；返回 Some(窗口内累计断开数) 表示应记日志。
    /// 窗口内首次断开立即记一条（计数归零，不重复计入后续累计），
    /// 其余静默累计，窗口到期时补记累计数并重置窗口。
    fn record(&mut self) -> Option<u64> {
        self.dropped += 1;
        if self.last_log.elapsed() >= DROP_LOG_WINDOW {
            let n = self.dropped;
            self.dropped = 0;
            self.last_log = std::time::Instant::now();
            self.first = true;
            Some(n)
        } else if self.first {
            self.first = false;
            self.dropped = 0;
            Some(1)
        } else {
            None
        }
    }
}

/// axum::serve::Listener：TCP 连接由后台 accept 循环接收，握手在各连接
/// 自己的 spawn 任务中异步完成（带 HANDSHAKE_TIMEOUT），accept() 只从通道
/// 取已握手连接。修复前握手在 accept() 内同步完成，axum::serve 串行调用
/// accept()，批量慢速/僵尸连接会阻塞整个 accept 循环（S1 DoS）。
pub(crate) struct TlsListener {
    rx: mpsc::Receiver<(std::io::Result<TlsStream>, SocketAddr)>,
    local_addr: SocketAddr,
    shutdown_tx: watch::Sender<()>,
}

type TlsStream = tokio_rustls::server::TlsStream<TcpStream>;

impl TlsListener {
    pub(crate) fn new(listener: TcpListener, acceptor: tokio_rustls::TlsAcceptor) -> Self {
        let local_addr = listener.local_addr().expect("listener has local addr");
        let (tx, rx) = mpsc::channel::<(std::io::Result<TlsStream>, SocketAddr)>(64);
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        tokio::spawn(accept_loop(
            listener,
            acceptor,
            tx,
            shutdown_rx,
            Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_HANDSHAKES)),
            Arc::new(std::sync::Mutex::new(DropLimiter::new())),
        ));
        Self {
            rx,
            local_addr,
            shutdown_tx,
        }
    }
}

/// 后台 accept 循环：只负责接收 TCP 连接并把握手派给各自 spawn 的任务，
/// 自身不参与握手；TlsListener 释放（服务停止）时 watch 信号触发退出。
async fn accept_loop(
    listener: TcpListener,
    acceptor: tokio_rustls::TlsAcceptor,
    tx: mpsc::Sender<(std::io::Result<TlsStream>, SocketAddr)>,
    mut shutdown_rx: watch::Receiver<()>,
    semaphore: Arc<tokio::sync::Semaphore>,
    limiter: Arc<std::sync::Mutex<DropLimiter>>,
) {
    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => break,
            res = listener.accept() => {
                let (stream, addr) = match res {
                    Ok(pair) => pair,
                    Err(e) => {
                        tracing::warn!(error = %e, "tcp accept failed");
                        continue;
                    }
                };
                // 无可用许可时直接断开连接：不排队、不堆积握手任务，
                // 握手风暴下并发握手数有上限（S1 补充）。断开日志按
                // 时间窗口限频，防刷日志放大 DoS 面。
                let Some(permit) = semaphore.clone().try_acquire_owned().ok() else {
                    let dropped = limiter
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .record();
                    if let Some(dropped) = dropped {
                        tracing::warn!(
                            addr = %addr,
                            dropped,
                            "tls handshake concurrency limit reached; dropping connection"
                        );
                    }
                    continue;
                };
                let acceptor = acceptor.clone();
                let tx = tx.clone();
                tokio::spawn(async move {
                    // permit 持有到握手任务结束（含超时/失败），届时释放许可。
                    let _permit = permit;
                    let result = match tokio::time::timeout(
                        HANDSHAKE_TIMEOUT,
                        acceptor.accept(stream),
                    )
                    .await
                    {
                        Ok(Ok(tls)) => Ok(tls),
                        Ok(Err(e)) => Err(e),
                        Err(_) => {
                            tracing::warn!(
                                addr = %addr,
                                timeout_secs = HANDSHAKE_TIMEOUT.as_secs(),
                                "tls handshake timed out"
                            );
                            Err(std::io::Error::new(
                                std::io::ErrorKind::TimedOut,
                                "tls handshake timed out",
                            ))
                        }
                    };
                    // 接收端已关闭（服务停止）时投递失败，连接随任务结束释放。
                    let _ = tx.send((result, addr)).await;
                });
            }
        }
    }
}

impl Drop for TlsListener {
    fn drop(&mut self) {
        // 通知后台 accept 循环退出，避免监听器随任务泄漏、端口被持续占用。
        let _ = self.shutdown_tx.send(());
    }
}

impl axum::serve::Listener for TlsListener {
    type Io = TlsStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (result, addr) = match self.rx.recv().await {
                Some(pair) => pair,
                // accept 循环异常退出（任务被 abort/panic）后 sender 全部
                // 释放、通道关闭：已无新连接来源。记录错误后挂起而非 panic
                // ——serve 任务保持存活，在途连接与优雅停机信号照常处理
                // （axum serve 循环把 accept 与 shutdown 信号 select，挂起
                // 的 accept 不阻塞停机）。
                None => {
                    tracing::error!("tls accept loop exited unexpectedly; listener is dead");
                    std::future::pending::<(std::io::Result<Self::Io>, Self::Addr)>().await
                }
            };
            match result {
                Ok(tls) => return (tls, addr),
                Err(e) => {
                    tracing::warn!(error = %e, "tls handshake failed");
                }
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok(self.local_addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::serve::Listener;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpStream;
    use tokio::sync::{Semaphore, mpsc, watch};

    /// 日志限频：窗口内首次断开立即记，其余静默累计，窗口结束补记。
    #[test]
    fn drop_limiter_aggregates_within_window() {
        let mut limiter = DropLimiter::new();
        assert_eq!(limiter.record(), Some(1), "first drop must log immediately");
        assert_eq!(limiter.record(), None, "later drops in window stay silent");
        assert_eq!(limiter.record(), None);
        limiter.last_log = std::time::Instant::now() - DROP_LOG_WINDOW;
        assert_eq!(
            limiter.record(),
            Some(3),
            "window end must log cumulative dropped count"
        );
    }

    #[test]
    fn build_server_config_missing_cert_file_errors() {
        let tls = ecat_transport::TlsConfig::new("/no-such-cert.pem", "/no-such-key.pem");
        assert!(build_server_config(&tls).is_err());
    }

    #[test]
    fn build_server_config_missing_key_file_errors() {
        let dir = std::env::temp_dir().join(format!("ecat-http-nokey-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let srv = ecat_tls::generate_server_cert("localhost").unwrap();
        let cert = dir.join("cert.pem");
        std::fs::write(&cert, &srv.cert_pem).unwrap();
        let tls = ecat_transport::TlsConfig::new(cert, "/no-such-key.pem");
        assert!(build_server_config(&tls).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_server_config_client_auth_requires_ca_path() {
        let dir = std::env::temp_dir().join(format!("ecat-http-noca-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let srv = ecat_tls::generate_server_cert("localhost").unwrap();
        let cert = dir.join("cert.pem");
        let key = dir.join("key.pem");
        std::fs::write(&cert, &srv.cert_pem).unwrap();
        std::fs::write(&key, &srv.key_pem).unwrap();
        let tls = ecat_transport::TlsConfig {
            cert_path: cert,
            key_path: key,
            ca_cert_path: None,
            require_client_auth: true,
        };
        let err = build_server_config(&tls).unwrap_err().to_string();
        assert!(err.contains("ca_cert_path"), "got: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// S1 补充：断开日志限频——同一窗口内多个溢出连接只记一条
    /// warn（含累计数），防止握手风暴刷日志放大 DoS 面。
    /// 限频状态机由 DropLimiter 单测覆盖；此处断言 accept 循环对
    /// 共享 limiter 的实际调用效果：首条立即上报（first 复位、计数
    /// 归零），其余静默累计（dropped==2）。此前用 tracing 捕获断言
    /// 日志条数，多 crate 并行下事件偶发丢失（flaky），已改为对
    /// limiter 状态的稳定断言，warn 宏调用路径保持原样。
    #[tokio::test]
    async fn drop_logs_are_rate_limited_per_window() {
        let dir = std::env::temp_dir().join(format!("ecat-http-tls-rate-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let srv = ecat_tls::generate_server_cert("localhost").unwrap();
        let (cert_path, key_path) = write_pem_files(&dir, &srv);
        let server_cfg =
            build_server_config(&ecat_transport::TlsConfig::new(cert_path, key_path)).unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_cfg));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, _rx) = mpsc::channel::<(std::io::Result<TlsStream>, SocketAddr)>(64);
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let semaphore = Arc::new(Semaphore::new(1));
        let limiter = Arc::new(std::sync::Mutex::new(DropLimiter::new()));
        tokio::spawn(accept_loop(
            listener,
            acceptor,
            tx,
            shutdown_rx,
            semaphore,
            Arc::clone(&limiter),
        ));

        // 占住唯一许可：连接 1 只建 TCP 不握手。
        let _tcp1 = TcpStream::connect(addr).await.unwrap();
        // 3 个溢出连接全部被断开（EOF），每次触发一次 record()。
        for _ in 0..3 {
            let mut tcp = TcpStream::connect(addr).await.unwrap();
            let mut b = [0u8; 1];
            let r = tokio::time::timeout(std::time::Duration::from_secs(2), tcp.read(&mut b)).await;
            assert!(
                matches!(r, Ok(Ok(0)) | Ok(Err(_))),
                "overflow connection must be dropped, got {r:?}"
            );
        }

        // 3 次 record() 后：首条已立即上报（first 复位、计数归零），
        // 其余 2 条静默累计。若限频失效（每条都记），dropped 将为 0。
        let l = limiter.lock().unwrap_or_else(|e| e.into_inner());
        assert!(!l.first, "first drop must have been reported immediately");
        assert_eq!(
            l.dropped, 2,
            "remaining drops must accumulate silently within the window"
        );

        let _ = shutdown_tx.send(());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn write_pem_files(
        dir: &std::path::Path,
        pair: &ecat_tls::CertPair,
    ) -> (std::path::PathBuf, std::path::PathBuf) {
        let cert_path = dir.join("server-cert.pem");
        let key_path = dir.join("server-key.pem");
        std::fs::write(&cert_path, &pair.cert_pem).unwrap();
        std::fs::write(&key_path, &pair.key_pem).unwrap();
        (cert_path, key_path)
    }

    /// 第三轮：accept 循环异常退出（sender 全部释放、通道关闭）时
    /// accept() 必须挂起而非 panic——serve 任务保持存活，在途连接与
    /// 优雅停机信号照常处理（旧实现 panic 会杀死服务线程）。
    #[tokio::test]
    async fn accept_hangs_not_panics_when_loop_exits() {
        let (tx, rx) = mpsc::channel::<(std::io::Result<TlsStream>, SocketAddr)>(1);
        drop(tx); // 模拟 accept 循环异常退出：通道关闭
        let (shutdown_tx, _) = watch::channel(());
        let mut listener = TlsListener {
            rx,
            local_addr: "127.0.0.1:0".parse().unwrap(),
            shutdown_tx,
        };
        let r =
            tokio::time::timeout(std::time::Duration::from_millis(200), listener.accept()).await;
        assert!(
            r.is_err(),
            "accept() must hang, not panic, when accept loop is dead"
        );
    }

    fn test_client_config(root_pem: &str) -> rustls::ClientConfig {
        ensure_crypto_provider();
        let mut roots = rustls::RootCertStore::empty();
        for cert in rustls_pemfile::certs(&mut std::io::BufReader::new(root_pem.as_bytes()))
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
        {
            roots.add(cert).unwrap();
        }
        rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth()
    }

    /// S1 补充：并发握手上限——无可用许可时新连接直接断开（读立即
    /// EOF/重置而非挂起）；被占住许可的连接不被误断开；握手完成后
    /// 许可释放，新连接恢复握手。
    #[tokio::test]
    async fn handshake_concurrency_limit_drops_overflow_connections() {
        let dir = std::env::temp_dir().join(format!("ecat-http-tls-sem-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let srv = ecat_tls::generate_server_cert("localhost").unwrap();
        let (cert_path, key_path) = write_pem_files(&dir, &srv);
        let server_cfg =
            build_server_config(&ecat_transport::TlsConfig::new(cert_path, key_path)).unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_cfg));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, _rx) = mpsc::channel::<(std::io::Result<TlsStream>, SocketAddr)>(64);
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let semaphore = Arc::new(Semaphore::new(1));
        tokio::spawn(accept_loop(
            listener,
            acceptor,
            tx,
            shutdown_rx,
            semaphore,
            Arc::new(std::sync::Mutex::new(DropLimiter::new())),
        ));

        // 连接 1 只建 TCP、不握手：占住唯一许可，握手任务挂起。
        let mut tcp1 = TcpStream::connect(addr).await.unwrap();
        // 连接 2：无许可，accept 循环必须直接断开（读立即返回 EOF/重置）。
        let mut tcp2 = TcpStream::connect(addr).await.unwrap();
        let mut buf = [0u8; 1];
        let r = tokio::time::timeout(std::time::Duration::from_secs(2), tcp2.read(&mut buf)).await;
        assert!(
            matches!(r, Ok(Ok(0)) | Ok(Err(_))),
            "overflow connection must be dropped immediately, got {r:?}"
        );

        // 连接 1 不被误断开：读挂起（timeout 超时而非 EOF）。
        let r =
            tokio::time::timeout(std::time::Duration::from_millis(300), tcp1.read(&mut buf)).await;
        assert!(r.is_err(), "held connection must not be dropped, got {r:?}");

        // 连接 1 完成真实 TLS 握手 → 握手任务结束 → 许可释放：
        // 新连接获得许可（握手任务等待 ClientHello，读挂起而非断开）。
        let cfg = test_client_config(&srv.cert_pem);
        let connector = tokio_rustls::TlsConnector::from(Arc::new(cfg));
        let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
        let _tls1 = connector.connect(server_name, tcp1).await.unwrap();
        let mut tcp3 = TcpStream::connect(addr).await.unwrap();
        let r =
            tokio::time::timeout(std::time::Duration::from_millis(300), tcp3.read(&mut buf)).await;
        assert!(
            r.is_err(),
            "permit must be released after handshake completes, got {r:?}"
        );

        let _ = shutdown_tx.send(());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
