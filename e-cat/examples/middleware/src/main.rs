// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use axum::{Json, Router, routing::get};
use ecat::App;
use ecat_metrics::{MetricsLayer, metrics_router};
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // M1 MetricsLayer 链：每个 HTTP 请求都会记录
    // ecat_http_requests_total / ecat_http_request_duration_seconds。
    let middleware = ServiceBuilder::new()
        .layer(MetricsLayer::new())
        .layer(TracingLayer::new("middleware"))
        .layer(LoggingLayer);

    let router = Router::new()
        .route("/", get(hello))
        .merge(metrics_router())
        .layer(middleware);

    let http_srv = HttpServer::new("0.0.0.0:8000").router(router);

    let mut app = App::builder()
        .name("middleware")
        .version("v0.1.0")
        .server(http_srv)
        .on_start(|| async {
            tracing::info!("middleware example started on 0.0.0.0:8000");
            tracing::info!("scrape Prometheus metrics at GET /metrics");
            Ok(())
        })
        .build()?;

    app.run().await?;
    Ok(())
}
