// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use super::ratelimit::RateLimitStore;
use async_trait::async_trait;
use redis::aio::MultiplexedConnection;

/// Redis-backed fixed-window rate limit store (keys prefixed with `rl:`).
pub struct RedisRateLimitStore {
    conn: MultiplexedConnection,
}

impl RedisRateLimitStore {
    pub async fn connect(url: &str) -> Result<Self, String> {
        let client = redis::Client::open(url).map_err(|e| e.to_string())?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| e.to_string())?;
        Ok(Self { conn })
    }

    pub fn from_connection(conn: MultiplexedConnection) -> Self {
        Self { conn }
    }
}

/// Atomic fixed-window increment: INCR, and on the first request set the
/// window TTL. If EXPIRE fails (e.g. wrong key type), the key is deleted so a
/// later successful request can restart the window instead of leaving a
/// permanent key that would block the client forever.
const INCR_SCRIPT: &str = r#"
local c = redis.call('INCR', KEYS[1])
if c == 1 then
    local ok, err = pcall(redis.call, 'EXPIRE', KEYS[1], ARGV[1])
    if not ok then
        redis.call('DEL', KEYS[1])
    end
end
return c
"#;

#[async_trait]
impl RateLimitStore for RedisRateLimitStore {
    async fn check(&self, key: &str, max: u32, window_secs: u64) -> Result<(), String> {
        let mut conn = self.conn.clone();
        let rkey = format!("rl:{key}");
        let script = redis::Script::new(INCR_SCRIPT);
        // Fail-open: if Redis is unreachable we cannot rate limit, so allow
        // the request and log — an outage must not lock every client out.
        let count: i64 = match script
            .key(&rkey)
            .arg(window_secs)
            .invoke_async(&mut conn)
            .await
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "redis rate-limit store unavailable; allowing request (fail-open)"
                );
                return Ok(());
            }
        };
        if count as u32 > max {
            Err("rate limit exceeded".into())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_fails_bad_url() {
        let result = RedisRateLimitStore::connect("redis://nonexistent:9999").await;
        assert!(result.is_err());
    }
}
