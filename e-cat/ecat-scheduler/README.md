# ecat-scheduler

<p align="center"><img src="../../docs/mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>


Periodic and one-shot task scheduling for the e-cat ecosystem.

```rust
let mut sched = Scheduler::new();
sched.every(Duration::from_secs(60), || async { /* cleanup */ });
sched.once(Duration::from_secs(5), || async { /* warmup */ });
sched.run().await; // or sched.shutdown();
```

- `every(interval, job)` — repeats forever, skips the immediate first tick
- `once(delay, job)` — fires a single job after the delay
- `run()` — blocks until all tasks finish; `shutdown()` — aborts all tasks

Pure tokio, no extra dependencies.

Part of the [e-cat](https://github.com/erik/e-cat) ecosystem.
