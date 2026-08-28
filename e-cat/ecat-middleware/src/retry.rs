// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use tower::{Layer, Service};

/// 指数退避：第 n 次重试（1 起）延迟 = min(base * 2^(n-1), max_delay)。
pub fn exponential_backoff(base: Duration, max_delay: Duration, retry_number: u32) -> Duration {
    let exp = retry_number.saturating_sub(1);
    let nanos = base.as_nanos().saturating_mul(1u128 << exp.min(127));
    Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX)).min(max_delay)
}

/// 重试判定规则：决定某次失败（或响应）是否值得重试。
///
/// 实现必须 `Clone`——tower::retry 每次调用会克隆一份 policy，attempt 计数
/// 在克隆副本内推进，并发请求互不干扰。
pub trait RetryRule<Req, Res, E>: Clone + Send + 'static {
    fn should_retry(&self, req: &Req, result: &Result<&Res, &E>) -> bool;
}

/// 默认规则：仅对服务错误（Err）重试；成功响应一律不重试。
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultRule;

impl<Req, Res, E> RetryRule<Req, Res, E> for DefaultRule {
    fn should_retry(&self, _req: &Req, result: &Result<&Res, &E>) -> bool {
        result.is_err()
    }
}

#[derive(Clone)]
struct RetryPolicy<R> {
    max_attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
    rule: R,
    attempts: u32,
}

impl<R, Req, Res, E> tower::retry::Policy<Req, Res, E> for RetryPolicy<R>
where
    R: RetryRule<Req, Res, E>,
    Req: Clone,
{
    type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

    fn retry(&mut self, req: &mut Req, result: &mut Result<Res, E>) -> Option<Self::Future> {
        self.attempts += 1;
        if self.attempts >= self.max_attempts {
            return None;
        }
        if !self.rule.should_retry(req, &result.as_ref()) {
            return None;
        }
        let delay = exponential_backoff(self.base_delay, self.max_delay, self.attempts);
        Some(Box::pin(tokio::time::sleep(delay)))
    }

    fn clone_request(&mut self, req: &Req) -> Option<Req> {
        Some(req.clone())
    }
}

/// Retry Layer：失败的请求按重试规则重试，指数退避。
///
/// ⚠️ 幂等性：重试会重新执行请求——仅对幂等请求（GET/HEAD/PUT/DELETE 等
/// 无副作用操作）安全。默认规则仅重试服务错误；对 HTTP 栈请通过
/// [`RetryLayer::with_rule`] 提供按状态码/响应内容的规则。
#[derive(Clone)]
pub struct RetryLayer<R = DefaultRule> {
    max_attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
    rule: R,
}

impl Default for RetryLayer {
    fn default() -> Self {
        Self::new(3, Duration::from_secs(1), Duration::from_secs(30))
    }
}

impl RetryLayer {
    /// `max_attempts`：总尝试次数（含首次）；`base_delay`：首次重试延迟；
    /// `max_delay`：退避封顶。
    pub fn new(max_attempts: u32, base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            max_attempts,
            base_delay,
            max_delay,
            rule: DefaultRule,
        }
    }

    pub fn with_rule<R>(self, rule: R) -> RetryLayer<R> {
        RetryLayer {
            max_attempts: self.max_attempts,
            base_delay: self.base_delay,
            max_delay: self.max_delay,
            rule,
        }
    }
}

impl<S, R: Clone> Layer<S> for RetryLayer<R> {
    type Service = RetryService<R, S>;

    fn layer(&self, inner: S) -> Self::Service {
        RetryService {
            inner,
            max_attempts: self.max_attempts,
            base_delay: self.base_delay,
            max_delay: self.max_delay,
            rule: self.rule.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RetryService<R, S> {
    inner: S,
    max_attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
    rule: R,
}

impl<R, S, Req, Res, E> Service<Req> for RetryService<R, S>
where
    R: RetryRule<Req, Res, E>,
    // tower::retry 每次调用克隆 service，故要求 S: Clone。
    S: Service<Req, Response = Res, Error = E> + Clone + Send + 'static,
    S::Future: Send + 'static,
    Req: Clone + Send + 'static,
    E: Send + 'static,
{
    type Response = Res;
    type Error = E;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let policy = RetryPolicy {
            max_attempts: self.max_attempts,
            base_delay: self.base_delay,
            max_delay: self.max_delay,
            rule: self.rule.clone(),
            attempts: 0,
        };
        Box::pin(tower::retry::Retry::new(policy, self.inner.clone()).call(req))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tower::ServiceExt;

    #[tokio::test]
    async fn retries_until_max_attempts() {
        let calls = Arc::new(AtomicU32::new(0));
        let svc = RetryLayer::new(3, Duration::from_millis(1), Duration::from_millis(2)).layer(
            tower::service_fn({
                let calls = Arc::clone(&calls);
                move |_req: ()| {
                    let calls = Arc::clone(&calls);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err::<(), _>(std::io::Error::other("boom"))
                    }
                }
            }),
        );
        let result = svc.oneshot(()).await;
        assert!(result.is_err());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            3,
            "must stop after max_attempts"
        );
    }

    #[tokio::test]
    async fn succeeds_on_retry() {
        let calls = Arc::new(AtomicU32::new(0));
        let svc = RetryLayer::new(3, Duration::from_millis(1), Duration::from_millis(2)).layer(
            tower::service_fn({
                let calls = Arc::clone(&calls);
                move |_req: ()| {
                    let calls = Arc::clone(&calls);
                    async move {
                        let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
                        if n <= 2 {
                            Err(std::io::Error::other("boom"))
                        } else {
                            Ok(())
                        }
                    }
                }
            }),
        );
        let result = svc.oneshot(()).await;
        assert!(result.is_ok(), "third attempt must succeed");
        assert_eq!(calls.load(Ordering::SeqCst), 3, "fail twice, succeed third");
    }

    #[derive(Clone)]
    struct NeverRetry;
    impl<Req, Res, E> RetryRule<Req, Res, E> for NeverRetry {
        fn should_retry(&self, _req: &Req, _result: &Result<&Res, &E>) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn non_retryable_rule_does_not_retry() {
        let calls = Arc::new(AtomicU32::new(0));
        let svc = RetryLayer::new(3, Duration::from_millis(1), Duration::from_millis(2))
            .with_rule(NeverRetry)
            .layer(tower::service_fn({
                let calls = Arc::clone(&calls);
                move |_req: ()| {
                    let calls = Arc::clone(&calls);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err::<(), _>(std::io::Error::other("boom"))
                    }
                }
            }));
        let result = svc.oneshot(()).await;
        assert!(result.is_err());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "rule declined, single attempt"
        );
    }

    #[test]
    fn exponential_backoff_caps_at_max_delay() {
        let base = Duration::from_millis(1);
        let max = Duration::from_millis(10);
        assert_eq!(exponential_backoff(base, max, 1), Duration::from_millis(1));
        assert_eq!(exponential_backoff(base, max, 2), Duration::from_millis(2));
        assert_eq!(exponential_backoff(base, max, 3), Duration::from_millis(4));
        assert_eq!(exponential_backoff(base, max, 4), Duration::from_millis(8));
        assert_eq!(
            exponential_backoff(base, max, 5),
            Duration::from_millis(10),
            "capped at max_delay"
        );
        assert_eq!(
            exponential_backoff(base, max, 100),
            Duration::from_millis(10),
            "large retry number capped"
        );
    }

    /// 退避只做下界断言（sleep 不会提前触发），避免 flaky。
    #[tokio::test]
    async fn retry_waits_for_backoff_delay() {
        let calls = Arc::new(AtomicU32::new(0));
        let svc = RetryLayer::new(2, Duration::from_millis(25), Duration::from_millis(50)).layer(
            tower::service_fn({
                let calls = Arc::clone(&calls);
                move |_req: ()| {
                    let calls = Arc::clone(&calls);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err::<(), _>(std::io::Error::other("boom"))
                    }
                }
            }),
        );
        let start = std::time::Instant::now();
        let result = svc.oneshot(()).await;
        let elapsed = start.elapsed();
        assert!(result.is_err());
        assert!(
            elapsed >= Duration::from_millis(25),
            "must wait for first backoff, got {elapsed:?}"
        );
    }
}
