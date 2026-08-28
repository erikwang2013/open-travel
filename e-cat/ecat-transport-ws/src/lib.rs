// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::routing::get;
use ecat_transport::{Server as TransportServer, normalize_addr};
use std::sync::Arc;
use tokio::net::TcpListener;

pub type WsHandler = Arc<
    dyn Fn(WebSocket) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// 优雅停机：已升级的 WebSocket 连接由 JoinSet 跟踪；stop() 发关闭
/// 信号后等待连接任务结束，超时强制 abort（N7 升级为正式功能）。
const CONNECTION_DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

pub struct WsServer {
    addr: String,
    path: String,
    handler: Option<WsHandler>,
    max_message_size: Option<usize>,
    shutdown_tx: std::sync::Mutex<Option<tokio::sync::watch::Sender<()>>>,
    serve_task: std::sync::Mutex<Option<tokio::task::JoinHandle<std::io::Result<()>>>>,
    close_tx: std::sync::Mutex<Option<tokio::sync::watch::Sender<()>>>,
    conns: std::sync::Mutex<Option<Arc<tokio::sync::Mutex<tokio::task::JoinSet<()>>>>>,
}

impl WsServer {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            // 空 host（如 ":3000"）会解析到 IPv6 通配 [::]，在无 IPv6 环境
            // 绑定失败；规范化为 IPv4 通配 "0.0.0.0"
            addr: normalize_addr(addr.into()),
            path: "/ws".into(),
            handler: None,
            max_message_size: None,
            shutdown_tx: std::sync::Mutex::new(None),
            serve_task: std::sync::Mutex::new(None),
            close_tx: std::sync::Mutex::new(None),
            conns: std::sync::Mutex::new(None),
        }
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn handler(mut self, handler: WsHandler) -> Self {
        self.handler = Some(handler);
        self
    }

    /// 单条消息大小上限（字节），缺省用底层默认（tungstenite 上限
    /// 64 MiB）。超限消息被拒绝并断开连接。
    pub fn max_message_size(mut self, max: usize) -> Self {
        self.max_message_size = Some(max);
        self
    }
}

#[async_trait]
impl TransportServer for WsServer {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let handler = self.handler.clone().ok_or("ws handler not set")?;
        let path = self.path.clone();
        let max_message_size = self.max_message_size;

        // 已升级连接统一进 JoinSet：stop() 能等待全部连接结束。
        // 关闭信号经 watch 广播给每个连接任务。
        let conns = Arc::new(tokio::sync::Mutex::new(tokio::task::JoinSet::new()));
        let (close_tx, close_rx) = tokio::sync::watch::channel(());
        *self.close_tx.lock().unwrap_or_else(|e| e.into_inner()) = Some(close_tx);
        *self.conns.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::clone(&conns));

        let app = axum::Router::new().route(
            &path,
            get(move |ws: WebSocketUpgrade| {
                let conns = Arc::clone(&conns);
                let close_rx = close_rx.clone();
                let ws = if let Some(max) = max_message_size {
                    ws.max_message_size(max)
                } else {
                    ws
                };
                async move {
                    ws.on_upgrade(move |socket| async move {
                        conns.lock().await.spawn(ws_loop(socket, handler, close_rx));
                    })
                }
            }),
        );

        let listener = TcpListener::bind(&self.addr).await?;
        let (tx, mut rx) = tokio::sync::watch::channel(());
        *self.shutdown_tx.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
        let shutdown_signal = async move {
            let _ = rx.changed().await;
        };
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal)
                .await
        });
        *self.serve_task.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 1) 广播关闭信号：每个连接任务收到后结束（handler 被 select 放弃，
        //    连接随之关闭）。
        if let Some(tx) = self
            .close_tx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            let _ = tx.send(());
        }
        // 2) 停止 serve 循环。
        if let Some(tx) = self
            .shutdown_tx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            let _ = tx.send(());
        }
        let handle = self
            .serve_task
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(handle) = handle {
            handle
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)??;
        }
        // 3) 等待已升级连接全部结束；超时强制 abort（防挂死）。
        let conns = self.conns.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(conns) = conns {
            let mut set = conns.lock().await;
            let deadline = tokio::time::Instant::now() + CONNECTION_DRAIN_TIMEOUT;
            loop {
                match tokio::time::timeout_at(deadline, set.join_next()).await {
                    Ok(Some(_)) => continue,
                    Ok(None) => break,
                    Err(_) => {
                        tracing::warn!(
                            timeout_secs = CONNECTION_DRAIN_TIMEOUT.as_secs(),
                            "ws graceful shutdown timed out; aborting connections"
                        );
                        set.abort_all();
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}

/// 连接任务：运行用户 handler（独占 socket 的既有 API），同时监听
/// 优雅停机信号。信号到达时放弃 handler 结束任务——关闭帧由 handler
/// 负责发送（框架无法代发，socket 归 handler 所有）；连接随之关闭，
/// 对端收到流结束。
async fn ws_loop(
    socket: WebSocket,
    handler: WsHandler,
    mut close_rx: tokio::sync::watch::Receiver<()>,
) {
    let fut = handler(socket);
    tokio::pin!(fut);
    tokio::select! {
        _ = &mut fut => {}
        _ = close_rx.changed() => {}
    }
}

pub fn echo_handler() -> WsHandler {
    Arc::new(|mut ws: WebSocket| {
        Box::pin(async move {
            while let Some(Ok(msg)) = ws.recv().await {
                if let Message::Text(text) = msg {
                    let _ = ws.send(Message::Text(text)).await;
                }
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn free_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    /// 手写 WebSocket 升级握手（避免引入 dev-dependency），返回已升级的流。
    async fn ws_upgrade(port: u16) -> tokio::net::TcpStream {
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        stream
            .write_all(
                b"GET /ws HTTP/1.1\r\n\
                  Host: localhost\r\n\
                  Upgrade: websocket\r\n\
                  Connection: Upgrade\r\n\
                  Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                  Sec-WebSocket-Version: 13\r\n\r\n",
            )
            .await
            .unwrap();
        let mut buf = [0u8; 4096];
        let n = tokio::time::timeout(std::time::Duration::from_secs(2), stream.read(&mut buf))
            .await
            .expect("upgrade response timed out")
            .unwrap();
        let resp = String::from_utf8_lossy(&buf[..n]);
        assert!(
            resp.starts_with("HTTP/1.1 101"),
            "expected 101 Switching Protocols, got: {resp}"
        );
        stream
    }

    /// N7 回归：stop() 广播关闭信号后等待已升级连接任务结束——
    /// 连接被关闭（对端读 EOF）、stop() 在超时内及时返回。
    #[tokio::test]
    async fn stop_closes_upgraded_connections_promptly() {
        let port = free_port();
        let srv = WsServer::new(format!("127.0.0.1:{port}")).handler(echo_handler());
        srv.start().await.unwrap();

        let mut stream = ws_upgrade(port).await;
        // 升级完成后连接保持存活：读挂起而非 EOF（handler 仍在运行）。
        let mut buf = [0u8; 1];
        let r = tokio::time::timeout(std::time::Duration::from_millis(200), stream.read(&mut buf))
            .await;
        assert!(
            r.is_err(),
            "connection must stay open before stop, got {r:?}"
        );

        // stop() 必须及时返回（远小于 5s 的 drain 超时上限）。
        tokio::time::timeout(std::time::Duration::from_secs(3), srv.stop())
            .await
            .expect("stop() must not hang")
            .unwrap();

        // 关闭信号到达后 handler 被放弃，连接随之关闭：对端读 EOF。
        let r =
            tokio::time::timeout(std::time::Duration::from_secs(2), stream.read(&mut buf)).await;
        assert!(
            matches!(r, Ok(Ok(0)) | Ok(Err(_))),
            "connection must be closed after stop, got {r:?}"
        );
    }

    #[test]
    fn ws_server_constructs() {
        let srv = WsServer::new("0.0.0.0:3000")
            .path("/chat")
            .handler(echo_handler());
        assert_eq!(srv.path, "/chat");
    }

    #[test]
    fn max_message_size_knob_is_optional_and_configurable() {
        assert_eq!(WsServer::new("0.0.0.0:3000").max_message_size, None);
        let srv = WsServer::new("0.0.0.0:3000").max_message_size(1024);
        assert_eq!(srv.max_message_size, Some(1024));
    }

    /// N3：空 host 地址（":3000"）必须规范化为 IPv4 通配，与 HttpServer 一致。
    #[test]
    fn new_normalizes_empty_host_addr() {
        let srv = WsServer::new(":3000");
        assert_eq!(srv.addr, "0.0.0.0:3000");
    }

    #[tokio::test]
    async fn start_without_handler_fails() {
        let port = free_port();
        let srv = WsServer::new(format!("127.0.0.1:{port}"));
        assert!(srv.start().await.is_err());
    }

    #[tokio::test]
    async fn start_fails_on_occupied_port() {
        let hold = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = hold.local_addr().unwrap().port();
        let srv = WsServer::new(format!("127.0.0.1:{port}")).handler(echo_handler());
        assert!(srv.start().await.is_err());
    }

    #[tokio::test]
    async fn stop_before_start_is_noop() {
        let srv = WsServer::new("127.0.0.1:0");
        srv.stop().await.unwrap();
    }

    /// 端到端 echo：手写掩码文本帧（客户端→服务端必须带掩码），
    /// 服务端回帧不带掩码（RFC 6455 服务端不发掩码）。
    #[tokio::test]
    async fn echo_roundtrip_text_message() {
        let port = free_port();
        let srv = WsServer::new(format!("127.0.0.1:{port}")).handler(echo_handler());
        srv.start().await.unwrap();
        let mut stream = ws_upgrade(port).await;

        let payload = b"hey";
        let mask = [0x11u8, 0x22, 0x33, 0x44];
        let mut frame = Vec::new();
        frame.push(0x81); // FIN + text
        frame.push(0x80 | payload.len() as u8); // masked, len=3
        frame.extend_from_slice(&mask);
        for (i, b) in payload.iter().enumerate() {
            frame.push(b ^ mask[i % 4]);
        }
        stream.write_all(&frame).await.unwrap();

        let mut buf = [0u8; 5];
        let n = tokio::time::timeout(std::time::Duration::from_secs(2), stream.read(&mut buf))
            .await
            .expect("echo frame timed out")
            .unwrap();
        assert_eq!(
            &buf[..n],
            &[0x81, 0x03, b'h', b'e', b'y'],
            "echo frame mismatch: {buf:?}"
        );
        drop(stream);
        srv.stop().await.unwrap();
    }
}
