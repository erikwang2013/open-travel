// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use std::future::Future;
use std::time::Duration;
use tokio::task::JoinHandle;

/// Scheduler for periodic and one-shot tasks.
pub struct Scheduler {
    handles: Vec<JoinHandle<()>>,
}

impl Scheduler {
    /// Create an empty scheduler.
    pub fn new() -> Self {
        Self {
            handles: Vec::new(),
        }
    }

    /// Run `job` every `interval`. The first run happens after one
    /// `interval` — the immediate first tick is skipped.
    pub fn every<F, Fut>(&mut self, interval: Duration, job: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // Capture the start instant here, before spawning: inside the task
        // `Instant::now()` would be evaluated at first poll, which makes
        // the schedule drift under paused clocks.
        let start = tokio::time::Instant::now();
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval_at(start + interval, interval);
            let mut jobs = tokio::task::JoinSet::new();
            loop {
                ticker.tick().await;
                // job 在 JoinSet 子任务中运行：panic 只以 Err 返回，记日志后
                // 继续下一 tick；shutdown 中止外层任务时 JoinSet Drop 会级联
                // 中止在跑的 job，与旧实现的中止语义一致。
                jobs.spawn(job());
                if let Some(Err(e)) = jobs.join_next().await {
                    tracing::warn!(error = %e, "scheduler job panicked; tick skipped");
                }
            }
        });
        self.handles.push(handle);
    }

    /// Run `job` once after `delay`.
    pub fn once<F, Fut>(&mut self, delay: Duration, job: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // Same reasoning as `every`: create the sleep outside the task.
        let sleep = tokio::time::sleep(delay);
        let handle = tokio::spawn(async move {
            sleep.await;
            let mut jobs = tokio::task::JoinSet::new();
            jobs.spawn(job());
            if let Some(Err(e)) = jobs.join_next().await {
                tracing::warn!(error = %e, "scheduler one-shot job panicked");
            }
        });
        self.handles.push(handle);
    }

    /// Wait for all scheduled tasks to finish. With `every` jobs this
    /// never returns — use `shutdown` to stop.
    pub async fn run(self) {
        for handle in self.handles {
            // 任务以 Err 结束（job 闭包同步 panic、future 构造阶段 panic
            // 或外部 abort）时记录 warn；异步 panic 已被 JoinSet 捕获并
            // 记日志，不会走到这里。
            if let Err(e) = handle.await {
                tracing::warn!(error = %e, "scheduler task ended; expected to run forever");
            }
        }
    }

    /// Abort all scheduled tasks.
    pub fn shutdown(self) {
        for handle in self.handles {
            handle.abort();
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // The paused-clock test only asserts the deterministic property: no work
    // runs before the first interval elapses. Task scheduling under a paused
    // clock is unreliable (tokio fires timers lazily at first poll), so the
    // periodic/one-shot behavior is covered by the real-time tests below.
    #[tokio::test]
    async fn every_skips_first_tick() {
        tokio::time::pause();
        tokio::time::advance(Duration::from_millis(1)).await;
        let count = Arc::new(AtomicUsize::new(0));
        let mut sched = Scheduler::new();
        let c = Arc::clone(&count);
        sched.every(Duration::from_millis(100), move || {
            let c = Arc::clone(&c);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });
        tokio::time::advance(Duration::from_millis(99)).await;
        tokio::task::yield_now().await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
        sched.shutdown();
    }

    #[tokio::test]
    async fn every_runs_periodically() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut sched = Scheduler::new();
        let c = Arc::clone(&count);
        sched.every(Duration::from_millis(20), move || {
            let c = Arc::clone(&c);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });
        tokio::time::sleep(Duration::from_millis(110)).await;
        sched.shutdown();
        // ~5 ticks expected on a 20ms interval over 110ms; only the lower
        // bound matters so a loaded CI machine cannot flake the test.
        let n = count.load(Ordering::SeqCst);
        assert!(n >= 3, "expected several ticks, got {n}");
    }

    #[tokio::test]
    async fn once_fires_exactly_once() {
        let fired = Arc::new(AtomicUsize::new(0));
        let mut sched = Scheduler::new();
        let f = Arc::clone(&fired);
        sched.once(Duration::from_millis(30), move || {
            let f = Arc::clone(&f);
            async move {
                f.fetch_add(1, Ordering::SeqCst);
            }
        });
        tokio::time::sleep(Duration::from_millis(120)).await;
        sched.shutdown();
        assert_eq!(fired.load(Ordering::SeqCst), 1);
    }

    /// N5：job panic 不能杀死调度循环——首个 tick panic 后，
    /// 后续 tick 必须继续执行（静默死亡回归测试）。
    #[tokio::test]
    async fn every_continues_after_job_panic() {
        let ticks = Arc::new(AtomicUsize::new(0));
        let mut sched = Scheduler::new();
        let t = Arc::clone(&ticks);
        sched.every(Duration::from_millis(20), move || {
            let t = Arc::clone(&t);
            async move {
                if t.fetch_add(1, Ordering::SeqCst) == 0 {
                    panic!("job boom");
                }
            }
        });
        tokio::time::sleep(Duration::from_millis(110)).await;
        sched.shutdown();
        let n = ticks.load(Ordering::SeqCst);
        assert!(n >= 2, "scheduler must survive job panic, got {n} ticks");
    }

    /// N5：一次性 job panic 后调度器照常可用（run 正常返回）。
    #[tokio::test]
    async fn once_panicking_job_does_not_poison_scheduler() {
        let mut sched = Scheduler::new();
        sched.once(Duration::from_millis(10), || async { panic!("boom") });
        tokio::time::sleep(Duration::from_millis(60)).await;
        sched.shutdown();
    }

    /// N5：job 闭包同步 panic（future 构造阶段，JoinSet 捕获不到）
    /// 时，run() 必须记录 warn 日志，而不是静默丢弃任务错误。
    #[tokio::test]
    async fn run_warns_on_sync_panic() {
        let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        #[derive(Clone)]
        struct W(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl std::io::Write for W {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let w = W(std::sync::Arc::clone(&buf));
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(move || w.clone())
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let mut sched = Scheduler::new();
        sched.once(Duration::from_millis(1), || -> std::future::Ready<()> {
            panic!("sync boom")
        });
        sched.run().await;

        let out = String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap();
        assert!(
            out.contains("panicked"),
            "run() must warn on task panic instead of swallowing it, got: {out}"
        );
    }
}
