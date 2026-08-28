// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use axum::{Json, Router, routing::get};
use ecat::App;
use ecat_middleware::{LoggingLayer, TracingLayer};
use ecat_transport_http::HttpServer;
use serde::Serialize;
use tower::ServiceBuilder;

#[derive(Serialize)]
struct HelloResponse {
    message: String,
}

async fn hello() -> Json<HelloResponse> {
    Json(HelloResponse {
        message: "Hello, e-cat!".into(),
    })
}

async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let middleware = ServiceBuilder::new()
        .layer(TracingLayer::new("helloworld"))
        .layer(LoggingLayer);

    let router = Router::new()
        .route("/", get(hello))
        .route("/health", get(health))
        .layer(middleware);

    let http_srv = HttpServer::new("0.0.0.0:8000").router(router);

    let mut app = App::builder()
        .name("helloworld")
        .version("v0.1.0")
        .server(http_srv)
        .on_start(|| async {
            tracing::info!("helloworld service started on 0.0.0.0:8000");
            Ok(())
        })
        .on_stop(|| async {
            tracing::info!("helloworld service stopped");
            Ok(())
        })
        .build()?;

    app.run().await?;
    Ok(())
}
