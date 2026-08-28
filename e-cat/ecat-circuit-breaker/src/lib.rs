// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tower::{Layer, Service};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Closed,
    Open,
    HalfOpen,
}

struct SlidingWindow {
    successes: u64,
    failures: u64,
    window_start: Instant,
    window: Duration,
}

impl SlidingWindow {
    fn new(window: Duration) -> Self {
        Self {
            successes: 0,
            failures: 0,
            window_start: Instant::now(),
            window,
        }
    }

    fn record(&mut self, success: bool) {
        self.rotate();
        if success {
            self.successes += 1;
        } else {
            self.failures += 1;
        }
    }

    fn total(&mut self) -> u64 {
        self.rotate();
        self.successes + self.failures
    }

    fn failure_ratio(&mut self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        self.failures as f64 / total as f64
    }

    fn rotate(&mut self) {
        if self.window_start.elapsed() >= self.window {
            self.successes = 0;
            self.failures = 0;
            self.window_start = Instant::now();
        }
    }

    /// 清空窗口计数并重置窗口起点。
    fn clear(&mut self) {
        self.successes = 0;
        self.failures = 0;
        self.window_start = Instant::now();
    }
}

struct BreakerInner {
    state: State,
    window: SlidingWindow,
    opened_at: Option<Instant>,
    half_open_count: u32,
}

/// 分类回调：把"传输成功"的响应判定为业务失败（如 HTTP 5xx）。
/// 签名经 Any 泛化，配置时类型安全，调用时 downcast 不匹配则视为成功。
type Classify = Arc<dyn Fn(&dyn std::any::Any) -> bool + Send + Sync>;

#[derive(Clone)]
pub struct CircuitBreakerLayer {
    failure_ratio: f64,
    window: Duration,
    half_open_probes: u32,
    open_duration: Duration,
    classify: Option<Classify>,
}

impl CircuitBreakerLayer {
    pub fn new() -> Self {
        Self {
            failure_ratio: 0.5,
            window: Duration::from_secs(30),
            half_open_probes: 3,
            open_duration: Duration::from_secs(10),
            classify: None,
        }
    }

    /// 配置分类回调：返回 true 的响应视为失败计入窗口（如 HTTP 5xx）。
    /// 默认（不配置）时传输成功的响应一律计成功，与旧行为兼容。
    /// `T` 必须与下游服务响应的具体类型一致；不一致时安全降级为成功。
    pub fn classify<F, T>(mut self, f: F) -> Self
    where
        T: 'static,
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        self.classify = Some(Arc::new(move |resp: &dyn std::any::Any| {
            resp.downcast_ref::<T>().map(&f).unwrap_or(false)
        }));
        self
    }

    pub fn failure_ratio(mut self, ratio: f64) -> Self {
        self.failure_ratio = ratio.clamp(0.0, 1.0);
        self
    }

    pub fn window(mut self, window: Duration) -> Self {
        self.window = window;
        self
    }

    pub fn half_open_probes(mut self, probes: u32) -> Self {
        self.half_open_probes = probes.max(1);
        self
    }

    pub fn open_duration(mut self, duration: Duration) -> Self {
        self.open_duration = duration;
        self
    }
}

impl Default for CircuitBreakerLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for CircuitBreakerLayer {
    type Service = CircuitBreakerService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CircuitBreakerService {
            inner,
            breaker: Arc::new(Mutex::new(BreakerInner {
                state: State::Closed,
                window: SlidingWindow::new(self.window),
                opened_at: None,
                half_open_count: 0,
            })),
            config: Arc::new(self.clone()),
        }
    }
}

#[derive(Clone)]
pub struct CircuitBreakerService<S> {
    inner: S,
    breaker: Arc<Mutex<BreakerInner>>,
    config: Arc<CircuitBreakerLayer>,
}

impl<S, Req> Service<Req> for CircuitBreakerService<S>
where
    S: Service<Req> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Response: 'static,
    S::Error: std::fmt::Display + std::error::Error + Send + Sync + 'static,
    Req: Send + 'static,
{
    type Response = S::Response;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(|e| Box::new(e) as _)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let mut breaker = self.breaker.lock().unwrap_or_else(|e| e.into_inner());
        let mut inner = self.inner.clone();
        let breaker_ref = Arc::clone(&self.breaker);
        let config = Arc::clone(&self.config);

        match breaker.state {
            State::Open => {
                if let Some(opened_at) = breaker.opened_at {
                    if opened_at.elapsed() >= config.open_duration {
                        tracing::info!("circuit breaker: open → half-open");
                        breaker.state = State::HalfOpen;
                        breaker.half_open_count = 0;
                    } else {
                        return Box::pin(async move {
                            Err(Box::new(std::io::Error::other("circuit breaker is open"))
                                as Box<dyn std::error::Error + Send + Sync>)
                        });
                    }
                }
            }
            State::HalfOpen => {
                if breaker.half_open_count >= config.half_open_probes {
                    return Box::pin(async move {
                        Err(
                            Box::new(std::io::Error::other("circuit breaker: too many probes"))
                                as Box<dyn std::error::Error + Send + Sync>,
                        )
                    });
                }
                breaker.half_open_count += 1;
            }
            State::Closed => {}
        }

        Box::pin(async move {
            let result = inner.call(req).await;
            let mut breaker = breaker_ref.lock().unwrap_or_else(|e| e.into_inner());

            match &result {
                Ok(resp) => {
                    // 配置 classify 回调后，业务失败响应（如 HTTP 5xx）
                    // 也计入失败窗口；未配置时传输成功一律计成功（兼容）。
                    let is_failure = config
                        .classify
                        .as_ref()
                        .is_some_and(|c| c(resp as &dyn std::any::Any));
                    breaker.window.record(!is_failure);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "circuit breaker: request failed");
                    breaker.window.record(false);
                }
            }

            match breaker.state {
                State::Closed => {
                    if breaker.window.total() >= 5
                        && breaker.window.failure_ratio() >= config.failure_ratio
                    {
                        tracing::warn!(
                            ratio = breaker.window.failure_ratio(),
                            "circuit breaker: closed → open"
                        );
                        breaker.state = State::Open;
                        breaker.opened_at = Some(Instant::now());
                    }
                }
                State::HalfOpen => {
                    if result.is_ok() {
                        tracing::info!("circuit breaker: half-open → closed");
                        breaker.state = State::Closed;
                        breaker.opened_at = None;
                        // 清空窗口，否则旧的高失败率会立即再次触发 open。
                        breaker.window.clear();
                    } else {
                        tracing::warn!("circuit breaker: half-open → open (probe failed)");
                        breaker.state = State::Open;
                        breaker.opened_at = Some(Instant::now());
                    }
                }
                State::Open => {}
            }

            result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_defaults() {
        let layer = CircuitBreakerLayer::new();
        assert!((layer.failure_ratio - 0.5).abs() < f64::EPSILON);
        assert_eq!(layer.window, Duration::from_secs(30));
        assert_eq!(layer.half_open_probes, 3);
    }

    #[test]
    fn layer_builder_methods() {
        let layer = CircuitBreakerLayer::new()
            .failure_ratio(0.8)
            .window(Duration::from_secs(10))
            .half_open_probes(5)
            .open_duration(Duration::from_secs(60));
        assert!((layer.failure_ratio - 0.8).abs() < f64::EPSILON);
        assert_eq!(layer.window, Duration::from_secs(10));
        assert_eq!(layer.half_open_probes, 5);
        assert_eq!(layer.open_duration, Duration::from_secs(60));
    }

    #[test]
    fn default_layer_construction() {
        let _layer = CircuitBreakerLayer::default();
    }

    #[test]
    fn sliding_window_counts() {
        let mut w = SlidingWindow::new(Duration::from_secs(60));
        assert_eq!(w.total(), 0);
        w.record(true);
        w.record(true);
        w.record(false);
        assert_eq!(w.total(), 3);
        assert!((w.failure_ratio() - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn sliding_window_clear_resets_counters() {
        let mut w = SlidingWindow::new(Duration::from_secs(60));
        w.record(false);
        w.record(false);
        assert_eq!(w.failure_ratio(), 1.0);
        w.clear();
        assert_eq!(w.total(), 0);
        assert_eq!(w.failure_ratio(), 0.0);
    }

    /// N10：配置 classify 回调后，业务失败响应（如 HTTP 5xx）计入失败
    /// 窗口——连续 5xx 响应触发熔断 open。
    #[tokio::test]
    async fn classify_callback_counts_5xx_as_failure() {
        use tower::ServiceExt;

        #[derive(Debug)]
        struct Resp {
            code: u16,
        }

        let layer = CircuitBreakerLayer::new()
            .failure_ratio(0.5)
            .window(Duration::from_millis(10))
            .classify(|r: &Resp| r.code >= 500);
        let svc = layer.layer(tower::service_fn(|_: String| async move {
            Ok::<Resp, std::io::Error>(Resp { code: 500 })
        }));

        for _ in 0..5 {
            let _ = svc.clone().oneshot("x".to_string()).await;
        }
        let b = svc.breaker.lock().unwrap();
        assert_eq!(b.state, State::Open, "5xx responses must trip the breaker");
    }

    /// N10：未配置 classify 时传输成功的响应一律计成功（兼容现状）——
    /// 连续 Ok 响应不触发 open。
    #[tokio::test]
    async fn default_classify_treats_ok_as_success() {
        use tower::ServiceExt;

        #[derive(Debug)]
        struct Resp;

        let layer = CircuitBreakerLayer::new()
            .failure_ratio(0.5)
            .window(Duration::from_millis(10));
        let svc = layer.layer(tower::service_fn(|_: String| async move {
            Ok::<Resp, std::io::Error>(Resp)
        }));

        for _ in 0..5 {
            let _ = svc.clone().oneshot("x".to_string()).await;
        }
        let b = svc.breaker.lock().unwrap();
        assert_eq!(
            b.state,
            State::Closed,
            "Ok responses must count as success by default"
        );
    }

    /// open 冷却期内请求被直接拒绝，不触达下游。
    #[tokio::test]
    async fn open_state_rejects_requests_until_cooldown() {
        use tower::ServiceExt;

        let layer = CircuitBreakerLayer::new()
            .failure_ratio(0.5)
            .window(Duration::from_millis(100))
            .open_duration(Duration::from_secs(60));
        let svc = layer.layer(tower::service_fn(|_: String| async move {
            Err::<String, std::io::Error>(std::io::Error::other("fail"))
        }));

        for _ in 0..5 {
            let _ = svc.clone().oneshot("x".to_string()).await;
        }
        {
            let b = svc.breaker.lock().unwrap();
            assert_eq!(b.state, State::Open);
        }

        let err = svc.clone().oneshot("x".to_string()).await.unwrap_err();
        assert!(
            err.to_string().contains("circuit breaker is open"),
            "got: {err}"
        );
    }

    /// half-open 探针失败 → 立即回到 open，需再次等待冷却期。
    #[tokio::test]
    async fn half_open_probe_failure_reopens_circuit() {
        use tower::ServiceExt;

        let layer = CircuitBreakerLayer::new()
            .failure_ratio(0.5)
            .window(Duration::from_millis(100))
            .open_duration(Duration::from_millis(10))
            .half_open_probes(3);
        let svc = layer.layer(tower::service_fn(|_: String| async move {
            Err::<String, std::io::Error>(std::io::Error::other("fail"))
        }));

        for _ in 0..5 {
            let _ = svc.clone().oneshot("x".to_string()).await;
        }
        {
            let b = svc.breaker.lock().unwrap();
            assert_eq!(b.state, State::Open);
        }

        // 冷却期后首个请求作为 half-open 探针放行，失败后重新 open
        tokio::time::sleep(Duration::from_millis(30)).await;
        let _ = svc.clone().oneshot("x".to_string()).await;
        let b = svc.breaker.lock().unwrap();
        assert_eq!(b.state, State::Open, "failed probe must reopen the circuit");
        assert!(b.opened_at.is_some());
    }

    /// half-open 探针名额耗尽后请求被拒绝，不触达下游。
    ///
    /// 注意：顺序探测时每次失败都会重新 open（count 清零），该分支只在
    /// 多个探针并发在飞时可达，故直接操纵内部状态做白盒验证。
    #[tokio::test]
    async fn half_open_probes_exhausted_rejects_requests() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tower::ServiceExt;

        let hits = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&hits);
        let layer = CircuitBreakerLayer::new().half_open_probes(2);
        let svc = layer.layer(tower::service_fn(move |_: String| {
            let counter = Arc::clone(&counter);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok::<String, std::io::Error>("ok".to_string())
            }
        }));
        {
            let mut b = svc.breaker.lock().unwrap();
            b.state = State::HalfOpen;
            b.half_open_count = 2;
        }

        let err = svc.clone().oneshot("x".to_string()).await.unwrap_err();
        assert!(err.to_string().contains("too many probes"), "got: {err}");
        assert_eq!(
            hits.load(Ordering::SeqCst),
            0,
            "rejected probe must not hit downstream"
        );
    }

    /// classify 类型不匹配时安全降级为成功（不 panic、不计失败）。
    #[tokio::test]
    async fn classify_type_mismatch_falls_back_to_success() {
        use tower::ServiceExt;

        #[derive(Debug)]
        struct Resp {
            code: u16,
        }

        // 回调期望 Resp，但下游返回 String → downcast 失败 → 视为成功
        let layer = CircuitBreakerLayer::new()
            .failure_ratio(0.5)
            .window(Duration::from_millis(10))
            .classify(|r: &Resp| r.code >= 500);
        let svc = layer.layer(tower::service_fn(|_: String| async move {
            Ok::<String, std::io::Error>("ok".to_string())
        }));

        for _ in 0..5 {
            let _ = svc.clone().oneshot("x".to_string()).await;
        }
        let b = svc.breaker.lock().unwrap();
        assert_eq!(b.state, State::Closed, "mismatched classify must not trip");
    }

    #[tokio::test]
    async fn half_open_success_resets_window_so_breaker_stays_closed() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use tower::ServiceExt;

        // 50% 失败率即 open；窗口足够短以便触发 open → half-open → closed 流程。
        let layer = CircuitBreakerLayer::new()
            .failure_ratio(0.5)
            .window(Duration::from_millis(10))
            .open_duration(Duration::from_millis(10))
            .half_open_probes(3);

        let healthy = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&healthy);
        let svc_fn = tower::service_fn(move |_: String| {
            let flag = Arc::clone(&flag);
            async move {
                if flag.load(Ordering::SeqCst) {
                    Ok::<String, std::io::Error>("ok".to_string())
                } else {
                    Err(std::io::Error::other("fail"))
                }
            }
        });
        let svc = layer.layer(svc_fn);

        // 5 次失败 → closed → open
        for _ in 0..5 {
            let _ = svc.clone().oneshot("x".to_string()).await;
        }
        {
            let b = svc.breaker.lock().unwrap();
            assert_eq!(b.state, State::Open);
        }

        // 等待 open 超时进入 half-open，然后成功探活。
        tokio::time::sleep(Duration::from_millis(30)).await;
        healthy.store(true, Ordering::SeqCst);
        let _ = svc.clone().oneshot("x".to_string()).await;
        {
            let mut b = svc.breaker.lock().unwrap();
            assert_eq!(b.state, State::Closed);
            assert_eq!(
                b.window.total(),
                0,
                "window must be cleared after half-open success"
            );
        }
    }
}
