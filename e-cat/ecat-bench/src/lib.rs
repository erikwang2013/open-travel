// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct BenchResult {
    pub name: String,
    pub total_requests: u64,
    pub total_duration: Duration,
    pub avg_latency_us: f64,
    pub p50_latency_us: f64,
    pub p95_latency_us: f64,
    pub p99_latency_us: f64,
    pub throughput_rps: f64,
}

impl BenchResult {
    pub fn print(&self) {
        println!("=== {} ===", self.name);
        println!("  requests:   {}", self.total_requests);
        println!("  duration:   {:.2?}", self.total_duration);
        println!("  throughput: {:.0} req/s", self.throughput_rps);
        println!("  avg:        {:.0} µs", self.avg_latency_us);
        println!("  p50:        {:.0} µs", self.p50_latency_us);
        println!("  p95:        {:.0} µs", self.p95_latency_us);
        println!("  p99:        {:.0} µs", self.p99_latency_us);
    }
}

pub async fn run_bench<F, Fut>(name: &str, concurrency: usize, total: u64, f: F) -> BenchResult
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
    run_bench_with_warmup(name, concurrency, total, 0, f).await
}

/// 与 [`run_bench`] 相同，但在测量前执行 `warmup` 次预热请求（延迟不进入
/// 统计），让冷启动（连接建立、缓存预热等）不污染 p99/avg。
///
/// 稳态窗口通过屏障同步：所有 worker 完成预热后才开始计时，因此
/// `total_duration` 与 throughput 只覆盖稳态测量阶段。
pub async fn run_bench_with_warmup<F, Fut>(
    name: &str,
    concurrency: usize,
    total: u64,
    warmup: u64,
    f: F,
) -> BenchResult
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
    if concurrency == 0 || total == 0 {
        return BenchResult {
            name: name.to_string(),
            total_requests: 0,
            total_duration: Duration::ZERO,
            avg_latency_us: 0.0,
            p50_latency_us: 0.0,
            p95_latency_us: 0.0,
            p99_latency_us: 0.0,
            throughput_rps: 0.0,
        };
    }
    let mut latencies = Vec::with_capacity(total as usize);
    let chunk_size = total / concurrency as u64;
    let remainder = total % concurrency as u64;
    let warmup_chunk = warmup / concurrency as u64;
    let warmup_remainder = warmup % concurrency as u64;
    let mut handles = Vec::with_capacity(concurrency);
    let shared_f = std::sync::Arc::new(f);
    // concurrency+1 方：所有 worker 完成预热后（含主线程）同时释放，
    // 主线程此刻起表，测量窗口恰好覆盖稳态阶段。
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(concurrency + 1));

    for i in 0..concurrency {
        let f = std::sync::Arc::clone(&shared_f);
        let barrier = std::sync::Arc::clone(&barrier);
        // Spread the remainder so no requests are dropped.
        let n = chunk_size + u64::from((i as u64) < remainder);
        let wn = warmup_chunk + u64::from((i as u64) < warmup_remainder);
        handles.push(tokio::spawn(async move {
            // 预热阶段：延迟丢弃，不计入统计
            for _ in 0..wn {
                f().await;
            }
            barrier.wait().await;
            let mut lats = Vec::with_capacity(n as usize);
            for _ in 0..n {
                let t0 = Instant::now();
                f().await;
                lats.push(t0.elapsed().as_micros() as f64);
            }
            lats
        }));
    }

    // worker 若在预热阶段 panic，屏障永远不会释放——超时失败而不是挂死
    let _ = tokio::time::timeout(Duration::from_secs(30), barrier.wait())
        .await
        .unwrap_or_else(|_| panic!("bench workers failed to reach steady state (warmup panic?)"));
    let start = Instant::now();

    for handle in handles {
        if let Ok(lats) = handle.await {
            latencies.extend(lats);
        }
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let total_duration = start.elapsed();
    let count = latencies.len();
    let avg = if count > 0 {
        latencies.iter().sum::<f64>() / count as f64
    } else {
        0.0
    };
    let p50 = if count > 0 { latencies[count / 2] } else { 0.0 };
    let p95 = if count > 0 {
        latencies[(count as f64 * 0.95) as usize]
    } else {
        0.0
    };
    let p99 = if count > 0 {
        latencies[(count as f64 * 0.99) as usize]
    } else {
        0.0
    };

    BenchResult {
        name: name.to_string(),
        total_requests: count as u64,
        total_duration,
        avg_latency_us: avg,
        p50_latency_us: p50,
        p95_latency_us: p95,
        p99_latency_us: p99,
        throughput_rps: count as f64 / total_duration.as_secs_f64(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn bench_simple() {
        let result = run_bench("noop", 2, 20, || async {}).await;
        assert_eq!(result.total_requests, 20);
        assert!(result.throughput_rps > 0.0);
    }

    #[tokio::test]
    async fn bench_even_split() {
        let result = run_bench("even", 5, 20, || async {}).await;
        assert_eq!(result.total_requests, 20);
    }

    #[tokio::test]
    async fn bench_distributes_remainder() {
        let result = run_bench("rem", 3, 10, || async {}).await;
        assert_eq!(result.total_requests, 10);
        assert!(result.p50_latency_us >= 0.0);
        assert!(result.p95_latency_us >= 0.0);
        assert!(result.p99_latency_us >= 0.0);
    }

    #[tokio::test]
    async fn bench_concurrency_greater_than_total() {
        let result = run_bench("over", 10, 3, || async {}).await;
        assert_eq!(result.total_requests, 3);
        assert!(result.p50_latency_us >= 0.0);
        assert!(result.p95_latency_us >= 0.0);
    }

    #[tokio::test]
    async fn bench_zero_total() {
        let result = run_bench("zero", 4, 0, || async {}).await;
        assert_eq!(result.total_requests, 0);
        assert_eq!(result.p50_latency_us, 0.0);
        assert_eq!(result.p95_latency_us, 0.0);
        assert_eq!(result.p99_latency_us, 0.0);
    }

    #[tokio::test]
    async fn bench_zero_concurrency() {
        let result = run_bench("noconc", 0, 10, || async {}).await;
        assert_eq!(result.total_requests, 0);
    }

    /// P2：预热请求必须真实执行但不计入统计。
    #[tokio::test]
    async fn warmup_excluded_from_stats() {
        let calls = Arc::new(AtomicUsize::new(0));
        let f = {
            let calls = Arc::clone(&calls);
            move || {
                let calls = Arc::clone(&calls);
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                }
            }
        };
        let result = run_bench_with_warmup("warm", 2, 10, 6, f).await;
        assert_eq!(
            result.total_requests, 10,
            "measured phase must exclude warmup"
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            16,
            "warmup requests must actually run"
        );
    }

    /// P2：冷启动（首个请求 50ms）只落在预热阶段，不能污染稳态 p99/avg。
    #[tokio::test]
    async fn warmup_removes_cold_start_from_p99() {
        let first = Arc::new(AtomicUsize::new(0));
        let f = {
            let first = Arc::clone(&first);
            move || {
                let first = Arc::clone(&first);
                async move {
                    if first.fetch_add(1, Ordering::SeqCst) == 0 {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                }
            }
        };
        let result = run_bench_with_warmup("warm", 1, 20, 5, f).await;
        assert_eq!(result.total_requests, 20);
        assert!(
            result.p99_latency_us < 45_000.0,
            "cold start leaked into p99: {} µs",
            result.p99_latency_us
        );
        assert!(
            result.avg_latency_us < 45_000.0,
            "cold start leaked into avg: {} µs",
            result.avg_latency_us
        );
    }

    /// P2：warmup=0 时行为与 run_bench 等价。
    #[tokio::test]
    async fn warmup_zero_is_noop() {
        let result = run_bench_with_warmup("zero-warm", 3, 12, 0, || async {}).await;
        assert_eq!(result.total_requests, 12);
    }

    #[test]
    fn bench_result_print() {
        let r = BenchResult {
            name: "test".into(),
            total_requests: 100,
            total_duration: Duration::from_secs(1),
            avg_latency_us: 500.0,
            p50_latency_us: 450.0,
            p95_latency_us: 700.0,
            p99_latency_us: 900.0,
            throughput_rps: 100.0,
        };
        r.print();
    }

    /// P3：p95 介于 p50 与 p99 之间（确定性延迟下三分位一致）。
    #[tokio::test]
    async fn p95_between_p50_and_p99() {
        let result = run_bench("pct", 1, 20, || async {
            tokio::time::sleep(Duration::from_millis(1)).await;
        })
        .await;
        assert_eq!(result.total_requests, 20);
        assert!(
            result.p50_latency_us <= result.p95_latency_us,
            "p95 must be >= p50: p50={} p95={}",
            result.p50_latency_us,
            result.p95_latency_us
        );
        assert!(
            result.p95_latency_us <= result.p99_latency_us,
            "p95 must be <= p99: p95={} p99={}",
            result.p95_latency_us,
            result.p99_latency_us
        );
    }
}
