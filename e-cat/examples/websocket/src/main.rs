// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use ecat::App;
use ecat_transport_ws::{WsServer, echo_handler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 已知限制：handler 独占升级后的 WebSocket（框架无法代为收发帧）；
    // 优雅停机时框架只放弃 handler 任务，关闭帧需 handler 自行发送。
    let ws_srv = WsServer::new("0.0.0.0:8000")
        .path("/ws")
        .handler(echo_handler());

    let mut app = App::builder()
        .name("websocket")
        .version("v0.1.0")
        .server(ws_srv)
        .on_start(|| async {
            tracing::info!("websocket example started on 0.0.0.0:8000/ws");
            Ok(())
        })
        .build()?;

    app.run().await?;
    Ok(())
}
