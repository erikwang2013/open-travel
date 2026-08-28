// ecat-bench 正式压测示例：对比中间件层开销（基线可复现工具）。
//
// 用法：
//   cargo run -p ecat-bench --example http_bench --release
//
// 环境变量（均可选）：
//   BENCH_TOTAL        每个端点的请求数（默认 20000）
//   BENCH_CONCURRENCY  并发数（默认 64）
//   BENCH_WARMUP       预热请求数（默认 2000，不计入统计）
//   BENCH_BASE_URL     可选：外部服务 URL，设置后追加一行对比（如 release helloworld）
//
// 同进程起 3 个 axum 服务对比层开销：
//   bare     裸 axum（无中间件）
//   metrics  + MetricsLayer（M1，指标记录）
//   full     + MetricsLayer + TracingLayer + LoggingLayer（近似 helloworld 生产栈）
//
// 输出每个端点的 requests/QPS/p50/p95/p99，以及与 bare 的 QPS/p95/p99 开销对比。
use axum::{Json, Router, routing::get};
use ecat_bench::{BenchResult, run_bench_with_warmup};
use ecat_metrics::MetricsLayer;
use ecat_middleware::{LoggingLayer, TracingLayer};
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

async fn start_server(port: u16) {
    let router = Router::new()
        .route("/", get(hello))
        .route("/health", get(health));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
}

async fn start_metrics_server(port: u16) {
    let router = Router::new()
        .route("/", get(hello))
        .route("/health", get(health))
        .layer(MetricsLayer::new());
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
}

async fn start_full_server(port: u16) {
    let middleware = ServiceBuilder::new()
        .layer(MetricsLayer::new())
        .layer(TracingLayer::new("http_bench"))
        .layer(LoggingLayer);
    let router = Router::new()
        .route("/", get(hello))
        .route("/health", get(health))
        .layer(middleware);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
}

async fn bench_endpoint(
    client: reqwest::Client,
    name: &str,
    url: &str,
    concurrency: usize,
    total: u64,
    warmup: u64,
) -> BenchResult {
    let client = std::sync::Arc::new(client);
    let url = url.to_string();
    let f = move || {
        let client = std::sync::Arc::clone(&client);
        let url = url.clone();
        async move {
            let _ = client.get(&url).send().await.map(|r| r.bytes());
        }
    };
    run_bench_with_warmup(name, concurrency, total, warmup, f).await
}

fn compare(name: &str, r: &BenchResult, bare: &BenchResult) {
    let qps = (r.throughput_rps / bare.throughput_rps - 1.0) * 100.0;
    let p95 = (r.p95_latency_us / bare.p95_latency_us - 1.0) * 100.0;
    let p99 = (r.p99_latency_us / bare.p99_latency_us - 1.0) * 100.0;
    println!(
        "  {name}: QPS {qps:+.1}% (bare {:.0} -> {:.0}), p95 {p95:+.1}% (bare {:.0}us -> {:.0}us), p99 {p99:+.1}% (bare {:.0}us -> {:.0}us)",
        bare.throughput_rps,
        r.throughput_rps,
        bare.p95_latency_us,
        r.p95_latency_us,
        bare.p99_latency_us,
        r.p99_latency_us
    );
}

#[tokio::main]
async fn main() {
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(256)
        .build()
        .unwrap();

    let total: u64 = std::env::var("BENCH_TOTAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20_000);
    let concurrency: usize = std::env::var("BENCH_CONCURRENCY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(64);
    let warmup: u64 = std::env::var("BENCH_WARMUP")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2_000);

    start_server(18081).await;
    start_metrics_server(18083).await;
    start_full_server(18082).await;

    let mut results = vec![
        bench_endpoint(
            client.clone(),
            "bare (no layer)",
            "http://127.0.0.1:18081/",
            concurrency,
            total,
            warmup,
        )
        .await,
        bench_endpoint(
            client.clone(),
            "+MetricsLayer",
            "http://127.0.0.1:18083/",
            concurrency,
            total,
            warmup,
        )
        .await,
        bench_endpoint(
            client.clone(),
            "+MetricsLayer+TracingLayer+LoggingLayer",
            "http://127.0.0.1:18082/",
            concurrency,
            total,
            warmup,
        )
        .await,
    ];

    // 可选外部服务对比行（如 release helloworld 起在 8000）：
    // BENCH_BASE_URL=http://127.0.0.1:8000/ cargo run -p ecat-bench --example http_bench --release
    if let Ok(base) = std::env::var("BENCH_BASE_URL") {
        results.push(
            bench_endpoint(
                client,
                "external (BENCH_BASE_URL)",
                &base,
                concurrency,
                total,
                warmup,
            )
            .await,
        );
    }

    for r in &results {
        r.print();
    }

    let bare = &results[0];
    println!("\n=== middleware overhead vs bare ===");
    for r in results.iter().skip(1) {
        compare(&r.name, r, bare);
    }
}
