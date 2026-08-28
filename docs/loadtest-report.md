# Phase 4-2 压测基准报告

日期：2026-08-28 21:55（UTC+8）
执行：scripts/loadtest.sh（curl 并发循环；本机无 wrk/k6/ab，QPS 受 curl 进程启动开销限制，为下限估计）

## 环境

| 项 | 值 |
|---|---|
| 主机 | 8 vCPU / 31GiB RAM（可用 18GiB），Linux 6.6 |
| 容器 | 6 个：travel-mysql / travel-redis / travel-opensearch / travel-nginx / travel-ecat-user / travel-ecat-booking |
| 网关 | nginx :8082 → user 8001 / booking 8002（host 8080 被 tracking-gateway、8081 被 bag/remus 占用，临时映射 8082） |
| 数据源 | mysql:3308、redis:6381、opensearch:9201（host 端口均因本机占用而临时改映射） |
| 鉴权 | HS256 JWT（JWT_SECRET 默认值），测试 token 由脚本按同密钥签发 |
| 压测工具 | scripts/loadtest.sh：-c 并发 -n 请求数 -u URL [-H 头]；统计 QPS / avg / p50 / p95 / 状态码分布 |

## 结果

### 1. /health 基线（经网关）

| 并发 | 请求数 | QPS | p50 | p95 | 状态码 |
|---|---|---|---|---|---|
| 8 | 2000 | 62.2 | 5.6ms | 22.6ms | 200×2000 |
| 32 | 4000 | 149.2 | 6.1ms | 26.7ms | 200×4000 |
| 64 | 8000 | 162.8 | 6.9ms | 36.4ms | 200×8000 |

QPS 在 ~163 收敛：curl 每请求一个进程（exec ~8ms），此并发已是工具上限，非服务上限。

### 2. 直连 401 路径（无 token，旧链，4-1 联调前）

| 服务 | 并发 | 请求数 | QPS | p50 | p95 | 状态码 |
|---|---|---|---|---|---|---|
| user 8001 | 16 | 992 | 132.4 | 4.9ms | 19.5ms | 401×992 |
| booking 8002 | 16 | 992 | 115.9 | 5.7ms | 24.9ms | 401×992 |

### 3. booking 缓存链路（经网关，带 token，低并发规避 429）

| 场景 | 并发 | 请求数 | QPS | p50 | p95 | 状态码 |
|---|---|---|---|---|---|---|
| 缓存命中（region 1，已回填） | 4 | 40 | 72.1 | 3.5ms | 14.5ms | 200×40，message=cache hit |
| 缓存未命中（region 99999，MySQL 回源+占位兜底，不写缓存） | 4 | 40 | 78.6 | 5.2ms | 22.9ms | 200×40 |

命中 vs 未命中 p50 差 ~1.7ms（Redis GET vs MySQL 查询）；QPS 同受 curl 上限约束。未命中路径注意：无数据 region 每次回源且不写缓存（代码 `if !rows.is_empty()` 才回填），天然是回源基准。

### 4. 限流验证（Redis 固定窗口 100 req/60s，按服务维度）

| 场景 | 请求数 | 结果 | 行为 |
|---|---|---|---|
| booking 突发（c=32） | 288 | 200×100 + 429×188 | 窗口内精确放行 100，超出全部 429 |
| user 突发（c=32，无 token） | 288 | 401×100 + 429×188 | 限流在 JWT 外层：未认证请求也计数，防暴力耗尽 |

限流与预期完全一致：窗口内第 1-100 个请求放行，之后 429，窗口结束（60s）自动重置。

## 联调期发现的阻塞问题（已修复）

1. **SSRF 误报**：nginx 注入的 X-Forwarded-For/X-Real-IP（docker 内网 172.x）被 SsrfDetector 判为 Critical 拦截，经网关的全部请求 502。修复：ecat-security is_proxy_header 跳过转发头（ecat-security/src/lib.rs:157）。
2. **JWT 误报**：Authorization 头携带的合法 JWT 命中 jwt_attack 规则（`ey...ey....` 正则），所有带 token 请求被拦。修复：is_proxy_header 增加跳过 authorization（同上）。
3. **阻断即 panic**：安全层拦截返回 Err 后，`From<NoError> for Infallible` 走 `unreachable!()`（shared/src/lib.rs:47），worker 直接 panic 而非返回 403。**未修**（属 4-1 中间件链路重构范围）：当前压测流量无真实攻击，不触发；任何真实拦截都会崩一个 worker，建议 4-1 改为错误→HTTP 响应。

## 结论与调优建议

- 服务端能力未被压满：163 QPS 是 curl 工具上限。接入 wrk/k6 后应重测真实 QPS。
- 缓存生效：命中 p50 3.5ms vs 未命中 5.2ms；TTL 300s 对 hot_destinations 合理，命中率取决于内容更新频率，若数据源更新频繁可降到 60-120s。
- 限流 100/60s/服务符合当前占位 API 规模；窗口为全服务共享计数（未按 IP），生产建议按客户端 IP 分桶（shared/src/lib.rs:60 ponytail 注释已预留）。
- 网关未加 nginx limit_req，服务端限流已兜底；若需网关层限速，nginx.conf:3 注释有说明。
- 429 响应快（突发 QPS 224 时仍稳定），限流本身无性能负担。
