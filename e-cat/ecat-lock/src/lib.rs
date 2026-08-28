// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use std::time::Duration;

/// Distributed lock abstraction.
///
/// `acquire` returns an ownership token that must be passed to `release`,
/// so a lock can only be released by the process that holds it.
#[async_trait]
pub trait DistributedLock: Send + Sync {
    /// Try to acquire the lock for `key` with the given `ttl`.
    /// Returns `Some(token)` on success, `None` if the lock is held by someone else.
    async fn acquire(&self, key: &str, ttl: Duration) -> Result<Option<String>, LockError>;

    /// Release the lock for `key`, but only if `token` still matches the holder.
    async fn release(&self, key: &str, token: &str) -> Result<(), LockError>;
}

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("lock error: {0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;
    use tokio::sync::Mutex;

    /// 内存版参考实现：锁定 trait 契约——token 匹配才能 release、
    /// TTL 过期后可重新获取、同一 key 同时只允许一个持有者。
    struct MemoryLock {
        held: Mutex<HashMap<String, (String, Instant)>>,
    }

    impl MemoryLock {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                held: Mutex::new(HashMap::new()),
            })
        }
    }

    #[async_trait]
    impl DistributedLock for MemoryLock {
        async fn acquire(&self, key: &str, ttl: Duration) -> Result<Option<String>, LockError> {
            let mut held = self.held.lock().await;
            if let Some((_, expires)) = held.get(key)
                && *expires > Instant::now()
            {
                return Ok(None);
            }
            let token = format!("tok-{key}");
            held.insert(key.into(), (token.clone(), Instant::now() + ttl));
            Ok(Some(token))
        }

        async fn release(&self, key: &str, token: &str) -> Result<(), LockError> {
            let mut held = self.held.lock().await;
            match held.get(key) {
                Some((t, _)) if t == token => {
                    held.remove(key);
                    Ok(())
                }
                Some(_) => Err(LockError::Other("token mismatch".into())),
                None => Err(LockError::Other("lock not held".into())),
            }
        }
    }

    #[tokio::test]
    async fn acquire_free_key_returns_token() {
        let lock = MemoryLock::new();
        let token = lock
            .acquire("job-a", Duration::from_secs(30))
            .await
            .unwrap()
            .expect("free key must be acquirable");
        assert_eq!(token, "tok-job-a");
    }

    #[tokio::test]
    async fn acquire_while_held_returns_none() {
        let lock = MemoryLock::new();
        lock.acquire("job-a", Duration::from_secs(30))
            .await
            .unwrap();
        assert_eq!(
            lock.acquire("job-a", Duration::from_secs(30))
                .await
                .unwrap(),
            None,
            "lock held by someone else must not be acquirable"
        );
    }

    #[tokio::test]
    async fn release_matching_token_frees_the_lock() {
        let lock = MemoryLock::new();
        let token = lock
            .acquire("job-a", Duration::from_secs(30))
            .await
            .unwrap()
            .unwrap();
        lock.release("job-a", &token).await.unwrap();
        // 释放后原持有者可再次获取。
        lock.acquire("job-a", Duration::from_secs(30))
            .await
            .unwrap()
            .expect("released lock must be acquirable again");
    }

    #[tokio::test]
    async fn release_wrong_token_fails() {
        let lock = MemoryLock::new();
        lock.acquire("job-a", Duration::from_secs(30))
            .await
            .unwrap();
        let err = lock.release("job-a", "tok-forgery").await.unwrap_err();
        assert_eq!(err.to_string(), "lock error: token mismatch");
    }

    #[tokio::test]
    async fn ttl_expiry_allows_reacquire() {
        let lock = MemoryLock::new();
        lock.acquire("job-a", Duration::from_millis(50))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(150)).await;
        lock.acquire("job-a", Duration::from_secs(30))
            .await
            .unwrap()
            .expect("expired lock must be acquirable again");
    }

    #[tokio::test]
    async fn concurrent_acquire_yields_single_holder() {
        let lock = MemoryLock::new();
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let lock = lock.clone();
            tasks.push(tokio::spawn(async move {
                lock.acquire("shared", Duration::from_secs(30))
                    .await
                    .unwrap()
            }));
        }
        let mut winners = 0;
        for task in tasks {
            if task.await.unwrap().is_some() {
                winners += 1;
            }
        }
        assert_eq!(winners, 1, "exactly one acquirer must win the lock");
    }

    #[test]
    fn lock_error_displays() {
        assert_eq!(
            LockError::Other("boom".into()).to_string(),
            "lock error: boom"
        );
    }

    #[tokio::test]
    async fn release_unheld_lock_fails() {
        let lock = MemoryLock::new();
        let err = lock.release("job-a", "tok-job-a").await.unwrap_err();
        assert_eq!(err.to_string(), "lock error: lock not held");
    }

    #[tokio::test]
    async fn acquire_empty_key_works() {
        let lock = MemoryLock::new();
        let token = lock
            .acquire("", Duration::from_secs(30))
            .await
            .unwrap()
            .expect("empty key must be acquirable");
        lock.release("", &token).await.unwrap();
    }
}
