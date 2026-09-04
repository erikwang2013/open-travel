//! 雪花 ID 加固回归：worker id 领号唯一性（风险 1）与 Redis 不可用拒绝启动（风险 2）。
//! 依赖 Redis（compose 映射 6381），连不上自动跳过。
use std::sync::Arc;

use ecat::business::shared::{connect_worker_claim_redis, snowflake_id};
use ecat_data::Cache as _;
use ecat_data_redis::RedisCache;

const CLAIM_KEY: &str = "ex:idgen:worker-idx";

/// 领号计数器是共享全局状态（`ex:idgen:worker-idx` + idgen_rs 全局单例），
/// 用例并发跑会互相删计数器导致假失败，故全部经此串行。
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn test_redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6381".into())
}

async fn test_cache() -> Option<Arc<RedisCache>> {
    match RedisCache::connect(&test_redis_url()).await {
        Ok(c) => Some(Arc::new(c)),
        Err(e) => {
            eprintln!("redis unavailable, skipping idgen test: {e}");
            None
        }
    }
}

/// 风险 1：同进程并发取号必须领到同一个 worker id 且 id 不重复。
/// 断言方向：同进程同 worker id 是**正确**行为（跨进程唯一性由 Redis INCR 保证，
/// 不在进程边界内），此处证明稳定性与不重复。
#[tokio::test]
async fn concurrent_ids_share_worker_id_and_are_unique() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let Some(cache) = test_cache().await else { return };
    let _ = cache.delete(CLAIM_KEY).await;

    let handles: Vec<_> = (0..64)
        .map(|_| tokio::task::spawn(async { snowflake_id().await }))
        .collect();
    let mut ids = Vec::with_capacity(handles.len());
    for h in handles {
        ids.push(h.await.unwrap());
    }

    let workers: std::collections::HashSet<u16> = ids
        .iter()
        .map(|&id| idgen_rs::id_helper::extract_id_info(id).worker_id)
        .collect();
    let uniq: std::collections::HashSet<u64> = ids.iter().copied().collect();
    assert_eq!(workers.len(), 1, "同进程并发取号应领到同一个 worker id: {workers:?}");
    assert_eq!(
        uniq.len(),
        ids.len(),
        "并发取号不得重复（含同毫秒序列）: {} 个里只有 {} 个唯一",
        ids.len(),
        uniq.len()
    );
}

/// 风险 1（跨进程，真正的故障场景）：两个进程必须领到**不同** worker id。
/// 加固前 `ECAT_WORKER_ID` 是静态值，两个进程配同一值会静默生成同位宽的 ID 序列
/// 导致 PK 碰撞；`redis-cli` 不在 PATH，故用同套件的 `idgen_probe` 子进程探针。
#[tokio::test]
async fn distinct_processes_claim_distinct_worker_ids() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let Some(cache) = test_cache().await else { return };
    let _ = cache.delete(CLAIM_KEY).await;

    let exe = std::env::current_exe().expect("测试二进制路径");
    let url = test_redis_url();
    let mut claimed: Vec<u16> = Vec::new();
    for expected in 1..=4u16 {
        // 探针断言本进程领到的正是 `expected`（计数器清零后第 N 个进程必须是 N）；
        // 不满足则退出非 0。用退出码而非 stdout——test harness 会吞掉子进程测试的 stdout。
        let out = std::process::Command::new(&exe)
            .arg("idgen_probe")
            .arg("--exact")
            .arg("--ignored")
            .env("REDIS_URL", &url)
            .env("IDGEN_EXPECTED_WORKER", expected.to_string())
            .output()
            .expect("探针子进程启动失败");
        if !out.status.success() {
            panic!("第 {expected} 个进程未领到预期 worker id（stderr {}）",
                String::from_utf8_lossy(&out.stderr));
        }
        claimed.push(expected);
    }
    let seen = claimed;
    let uniq: std::collections::HashSet<u16> = seen.iter().copied().collect();
    assert_eq!(
        uniq.len(),
        seen.len(),
        "每个进程必须领到不同 worker id，实际 {seen:?}——重复即静默 PK 碰撞"
    );
    let c = cache.increment(CLAIM_KEY, 0).await.expect("计数器读取失败");
    let _ = cache.delete(CLAIM_KEY).await;
    // increment(_, 0) 是 INCRBY 0（无操作），此处只用于探测 key 是否已建立。
    assert!(c > 0 || seen.is_empty(), "计数器 key 应存在（实际 {c}）");
}

/// 子进程探针：领号后断言正是期望值（计数器清零后第 N 个进程必须是 N），不满足即非 0 退出。
/// `#[ignore]` 保证只经上方 `Command` 显式调用。
#[test]
#[ignore]
fn idgen_probe() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("测试运行时构建失败");
    let id = rt.block_on(snowflake_id());
    let w = idgen_rs::id_helper::extract_id_info(id).worker_id;
    let want: u16 = std::env::var("IDGEN_EXPECTED_WORKER")
        .ok()
        .and_then(|v| v.parse().ok())
        .expect("IDGEN_EXPECTED_WORKER 未设置");
    assert_eq!(
        w, want,
        "worker id 碰撞：第 {want} 个进程领到 {w}，与已领号进程重复"
    );
}

/// 风险 2：领号通道不通必须拒绝启动，不得降级到 worker_id=0 继续写库。
/// 直测 `claim_worker_id()`：`snowflake_id()` 走 idgen_rs 全局单例，某测试一旦
/// init 过后续测试无法重置，`#[should_panic]` 会与其他套件互斥。
/// 用不可达 URL 验证同一语义；本文件无并发写 env，安全。
#[tokio::test]
async fn claim_worker_id_fails_closed_when_redis_unreachable() {
    let _ = test_cache().await.expect("其他用例需 Redis，此处应已连通");
    // 指向未监听端口：确定性失败，不依赖网络抖动。env 是本测试二进制私有的
    // （每个 cargo test target 独立进程），改完即还原，不影响同进程其他用例。
    let real = test_redis_url();
    unsafe { std::env::set_var("REDIS_URL", "redis://127.0.0.1:1"); }
    let err = connect_worker_claim_redis()
        .await
        .err()
        .expect("Redis 不可用时必须返回 Err——降级到 worker_id=0 会静默 PK 碰撞");
    unsafe { std::env::set_var("REDIS_URL", real); }
    assert!(
        err.to_lowercase().contains("connect") || err.to_lowercase().contains("redis"),
        "错误应指向 Redis 连接失败: {err}"
    );
}
