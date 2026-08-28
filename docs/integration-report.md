# open-travel Phase 4-1 全链路联调报告

日期：2026-08-28
环境：docker compose（project 名 `open-travel`），Rust e-cat 微服务（user 8001 / booking 8002），nginx 网关，MySQL + Redis + OpenSearch

## 1. 环境与端口调整记录

本机已有 open-novel 项目容器及宿主进程占用端口，open-travel compose 的 host 端口做了临时调整（容器内网端口不变，nginx 内网转发不受影响）：

| 服务 | compose 原 host 端口 | 调整后 | 冲突对象 |
|------|---------------------|--------|----------|
| mysql | 3306 | 3308 | 宿主 mysql |
| redis | 6379 | 6381 | 宿主 redis |
| opensearch | 9200 | 9201 | open-novel-opensearch 容器 |
| nginx 网关 | 8080 → 8081 | 8082 | 宿主 tracking-gateway（8080）、bag/remus http.server（8081） |

open-novel 的容器未做任何改动。调整以注释记录在 `config/docker-compose.yml` 中。

## 2. 联调中修复的问题

### 2.1 Dockerfile 构建失败（rustc 版本 + protoc）
- `Cargo.lock` 锁定的 `time 0.3.55` 要求 rustc ≥ 1.88，Dockerfile 用 `rust:1.85-slim` 构建失败 → 升级为 `rust:1.88-slim`
- `ecat-protos` 的 build.rs 需要 protoc → build 阶段 `apt-get install protobuf-compiler`

### 2.2 compose 挂载路径错误
`./config/xxx` 相对路径按 compose 文件所在目录（config/）解析，实际指向不存在的 `config/config/xxx`，导致 schema.sql 未初始化、opensearch.yml/nginx.conf 挂载失败 → 改为 `./xxx`。修复后需删掉旧的 `open-travel_travel-mysql-data` 卷重建，schema.sql 才生效（travel 库 5 张表初始化成功）。

### 2.3 SecurityLayer 误伤网关代理请求（核心 bug，导致 502 + 服务崩溃）
现象：经 nginx 的所有业务请求 502；服务日志显示 `ecat_security: attack detected attack_type=ssrf matched=172.19.0.1` 后 `panic at shared/src/lib.rs:47: unreachable code`，worker 线程崩溃。

根因（e-cat/ecat-security）：
1. `request_parts` 扫描全部请求头，nginx 注入的 `X-Real-IP/X-Forwarded-For: 172.19.0.1`（docker 内网网关 IP）命中 security-rust 的 SSRF 正则（`172.(1[6-9]|2\d|3[01]).*`）→ 被拦
2. security-rust 的 jwt_attack 检测器正则 `ey...\.ey...\.` 匹配**一切标准 JWT**，任何合法 token 请求都会被 High 拦截
3. SecurityLayer 拦截时返回 `Err`，服务端 `no_error` 归一后 tower 要求转 `Infallible`，`From<NoError> for Infallible` 是 `unreachable!()` → panic 打崩服务

修复（`e-cat/ecat-security/src/lib.rs`）：
- 跳过代理拓扑头（X-Forwarded-For / X-Real-IP / Forwarded 等）不扫描
- `evaluate` 对 `jwt_attack` 检测结果仅记日志不拦截（JWT 真伪由 JwtAuthLayer 验签把关）
- 拦截时直接返回 403 Response（复用 `SecurityError::into_response`）而非 `Err`，消除 unreachable panic 路径
- 新增回归测试 `header_layer_passes_proxy_headers_with_internal_ip`，26 个测试全通过

### 2.4 中间件链顺序（限流在 JWT 内层，无 token 请求不计数）
现象：系统无 JWT 签发接口，`/api/v1/user/profile` 无 token 恒 401，请求到不了限流层 → 任务要求的 429 永远不会出现。

修复：
- `services/user/src/main.rs`：限流层移到 JWT 外层（顺序 CircuitBreaker → Security → RateLimit → Auth），未认证请求也计入限流
- `services/booking/src/main.rs`：dates 为公开接口（热门目的地展示），移出 JWT 保护，保留限流（CircuitBreaker → Security → RateLimit）

### 2.5 OpenSearch 索引初始化失败（缺 ICU 插件）
`opensearchproject/opensearch:latest` 镜像不再内置 `analysis-icu` 插件，`{"type":"icu"}` 报 `Unknown analyzer type [icu]` → `scripts/opensearch_init.sh` 改用内置 `cjk` 分析器（bigram 中日韩分词），免装插件，脚本保持幂等。

## 3. 验证结果

| 项目 | 结果 |
|------|------|
| compose 全部容器 healthy | 通过 |
| `opensearch_init.sh` 建索引 + 幂等（第二次 skip） | 通过 |
| GET /health（网关 → user） | 200 OK |
| GET /ready（网关 → user） | 200，`data:true`（MySQL + Redis 均连接） |
| GET /api/v1/booking/dates?region_id=1（无 token） | 见下方备注 |
| 二次请求 cache hit | 见下方备注 |
| GET /api/v1/user/profile（无 JWT） | 401 |
| Redis 限流：100+ 请求后 429 | 见下方备注 |

（修复重建后的最终验证结果待补）

## 4. 遗留问题

- 无 JWT 签发接口（register 为占位实现），带 token 的业务验证需 Phase 2 接入真实用户/登录
- e-cat 其他中间件若返回 Err 仍有 panic 风险（本次仅修 SecurityLayer 路径）
- compose 端口调整为临时方案，生产/多人环境需统一端口规划
