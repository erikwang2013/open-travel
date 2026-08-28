// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use axum::{Router, response::IntoResponse, routing::get};
use ecat_transport::Server as _;
use ecat_transport_http::HttpServer;
use tokio::net::{TcpListener, TcpStream};

async fn health() -> impl IntoResponse {
    "ok"
}

#[tokio::test]
async fn stop_before_start_is_noop() {
    let srv = HttpServer::new("127.0.0.1:0");
    srv.stop().await.unwrap();
}

#[tokio::test]
async fn start_fails_on_occupied_port() {
    let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = probe.local_addr().unwrap().port();
    let srv = HttpServer::new(format!("127.0.0.1:{port}"));
    assert!(
        srv.start().await.is_err(),
        "occupied port must fail to bind"
    );
}

#[tokio::test]
async fn serves_http_and_stops_cleanly() {
    let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    let srv = std::sync::Arc::new(
        HttpServer::new(format!("127.0.0.1:{port}"))
            .router(Router::new().route("/health", get(health))),
    );
    let task = tokio::spawn({
        let srv = std::sync::Arc::clone(&srv);
        async move { srv.start().await }
    });

    let body = loop {
        match TcpStream::connect(("127.0.0.1", port)).await {
            Ok(mut stream) => {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                stream
                    .write_all(
                        b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    )
                    .await
                    .unwrap();
                let mut buf = Vec::new();
                stream.read_to_end(&mut buf).await.unwrap();
                break String::from_utf8_lossy(&buf).into_owned();
            }
            // 服务端绑定存在竞争窗口：连接被拒时重试（std sleep 避免依赖 tokio time feature）
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(100)),
        }
    };
    assert!(body.contains("200 OK"), "unexpected response: {body}");

    srv.stop().await.unwrap();
    task.await.unwrap().unwrap();
}
