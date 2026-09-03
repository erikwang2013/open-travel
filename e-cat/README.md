<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Open Travel — 全球旅游平台

<p align="center"><img src="../docs/mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>


[English](README.en.md) | [日本語](../docs/i18n/ja/README.md) | [한국어](../docs/i18n/ko/README.md) | [Русский](../docs/i18n/ru/README.md) | [Deutsch](../docs/i18n/de/README.md) | [Français](../docs/i18n/fr/README.md) | [Español](../docs/i18n/es/README.md) | [Português](../docs/i18n/pt/README.md) | [हिन्दी](../docs/i18n/hi/README.md) | [العربية](../docs/i18n/ar/README.md) | [বাংলা](../docs/i18n/bn/README.md) | [Bahasa Indonesia](../docs/i18n/id/README.md) | 简体中文

> 一个面向全球用户的旅游预订平台：Rust 微服务后端（**e-cat** 框架） + Flutter / HarmonyOS 多端客户端，支持 **12+ 种语言**、国际支付与多语言搜索。

## 项目简介

Open Travel 是一个全球旅游平台 monorepo。后端基于 **e-cat（一只猫）** Rust 微服务框架（v3.0.3 · 51 crates）构建 —— 对标 [go-kratos/kratos](https://github.com/go-kratos/kratos) v3，提供 API-first 开发体验、可插拔组件架构与统一的 HTTP/gRPC 中间件抽象。

| 维度 | 说明 |
| :--- | :--- |
| **后端** | e-cat（Rust）：HTTP/axum + gRPC/tonic，51 crates 微服务生态 |
| **业务服务** | user-service（:8001）～ payment-service（:8009）九个服务；业务逻辑在 `e-cat/ecat/src/business/`，服务入口在 `e-cat/ecat/src/bin/` |
| **网关** | Nginx（`config/nginx.conf`），按 URL 前缀分流 |
| **多端客户端** | `apps/client/flutter`（iOS / Android / Web / Desktop）、`apps/client/harmonyos`（鸿蒙） |
| **数据源** | MySQL + Redis 缓存 + OpenSearch 多语言搜索 |
| **安全** | ecat-security / ecat-auth（JWT）/ ecat-tls：认证、审计、限流、防注入、支付回调 HMAC 验签与内部服务鉴权 |
| **国际化** | 12+ 语种，RTL 支持，OpenSearch 多语言分词 |

## 项目结构

```
open-travel/
├── apps/                  # client/ 多端客户端 + admin/ 管理端
├── config/                # docker-compose.yml、nginx.conf、schema.sql、opensearch.yml
├── docs/                  # 规划文档、联调/压测报告、SVG 架构图、i18n 翻译
├── scripts/               # opensearch_init / loadtest / cdn_setup / cdn_upload / release
└── e-cat/                 # e-cat 框架 + 业务服务（同一 Cargo workspace）
    ├── ecat*/             # 51 个 ecat-* 框架 crate
    ├── ecat/              # 主框架 crate：门面 + 业务模块（src/business/）+ 服务入口（src/bin/）
    ├── config/            # 框架配置示例
    └── examples/          # 框架示例项目
```

## 业务服务

| 服务 | 端口 | 说明 |
|------|------|------|
| user-service | 8001 | `POST /api/v1/user/register`、`POST /api/v1/user/login`（公开）、`GET /api/v1/user/profile`（需 JWT） |
| booking-service | 8002 | `GET /api/v1/booking/dates?region_id=N`（公开接口）、`/api/v1/reviews/` 评价 |
| admin-service | 8003 | 管理端：登录 + 目的地/景区 CRUD（`/api/v1/admin/`） |
| search-service | 8004 | 多语种搜索（`/api/v1/search`） |
| line-service | 8005 | 旅游线路（`/api/v1/lines`） |
| order-service | 8006 | 订单（`/api/v1/orders`） |
| flight-service | 8007 | 机票（`/api/v1/flights`） |
| hotel-service | 8008 | 酒店（`/api/v1/hotels`） |
| payment-service | 8009 | 支付（`/api/v1/payments`） |
| Nginx 网关 | 8082→80 | 按 `/api/v1/user/`、`/api/v1/booking/`、`/api/v1/admin/`、`/api/v1/search`、`/api/v1/lines`、`/api/v1/orders`、`/api/v1/flights`、`/api/v1/hotels`、`/api/v1/payments` 前缀分流 |

所有服务均提供 `GET /health`（存活）与 `GET /ready`（就绪，报告数据源降级状态）。

> 接口详情（请求/响应示例、鉴权与限流说明）见 [API 参考](../docs/api.md)。

## 一键安装

环境要求：Docker + Docker Compose（v2）。（Rust 工具链仅源码构建需要）

```bash
git clone <仓库地址> && cd open-travel
./scripts/install.sh
```

脚本自动：检查环境 → 构建并启动全部服务（MySQL/Redis/OpenSearch/Kafka/9 个微服务/Nginx 网关）→ 初始化搜索索引 → 健康巡检，完成后打印访问地址与默认管理账号。

> 使用说明：访问 http://localhost:8082/health 探活；管理端登录 `admin@travel.local` / `Admin@123`；客户端（`apps/client/flutter`）用 `flutter run -d chrome` 启动。

## 快速开始

### 前提条件

- Rust 1.85+（stable 工具链，edition 2024）+ [protoc](https://github.com/protocolbuffers/protobuf)
- Docker + Docker Compose

### 构建

```bash
cd e-cat
cargo check -p ecat --bin user-service --bin booking-service --bin admin-service \
  --bin search-service --bin line-service --bin order-service --bin flight-service \
  --bin hotel-service --bin payment-service   # 编译检查业务服务
```

本地开发模式运行（各自监听 `0.0.0.0:8001` ～ `0.0.0.0:8009`）：

```bash
cd e-cat
cargo run -p ecat --bin user-service &
cargo run -p ecat --bin booking-service
```

构建 Docker 镜像（`e-cat/Dockerfile`，按 `-p ecat --bin` 构建九个服务，镜像内产物位于 `/usr/local/bin/`）：

```bash
docker build -f e-cat/Dockerfile -t travel-services .
```

### 启动（Docker Compose）

```bash
docker compose -f config/docker-compose.yml up -d
```

> ⚠️ 不要使用 `--env-file .env` 启动（会报错）。

### 验证

业务接口版本在 URL 前缀：`/api/v1/...`（无需版本请求头）。

curl 直连服务：

```bash
curl http://localhost:8002/health                 # OK
curl "http://localhost:8002/api/v1/booking/dates?region_id=1"
# {"code":0,"message":"ok","data":[{"region_id":1,"name_en":"placeholder-destination"}]}
curl -H "Content-Type: application/json" -X POST \
  http://localhost:8001/api/v1/user/register -d '{"email":"a@b.com","password":"secret1"}'
# {"code":0,"message":"ok","data":{"user_id":1,"email":"a@b.com","lang":"en"}}
```

经网关（Nginx，host `8082` → 容器 `80`）：

```bash
curl "http://localhost:8082/api/v1/booking/dates?region_id=1"
curl http://localhost:8082/health
```

带鉴权的接口（`/api/v1/user/profile`）需在请求头携带 JWT：

```bash
curl -H "Authorization: Bearer <JWT>" http://localhost:8082/api/v1/user/profile
```

### 端口映射

| 服务 | 宿主端口 → 容器端口 |
|------|---------------------|
| Nginx 网关 | 8082 → 80 |
| user-service | 8001 → 8001 |
| booking-service | 8002 → 8002 |
| admin-service | 8003 → 8003 |
| search-service | 8004 → 8004 |
| line-service | 8005 → 8005 |
| order-service | 8006 → 8006 |
| flight-service | 8007 → 8007 |
| hotel-service | 8008 → 8008 |
| payment-service | 8009 → 8009 |
| MySQL | 3308 → 3306 |
| Redis | 6381 → 6379 |
| OpenSearch | 9201 → 9200 |

> 数据源端口映射为本地临时方案（宿主 3306/6379/9200 已被占用），见 `../docs/integration-report.md`。

### 运行测试

```bash
cd e-cat
cargo test -p ecat --bins                        # 业务服务
cargo test --workspace                          # 全 workspace
```

### 脚本

| 脚本 | 用途 |
|------|------|
| `scripts/opensearch_init.sh` | 幂等创建 OpenSearch 索引（cjk 分析器） |
| `scripts/loadtest.sh` | 接口压测 |
| `scripts/cdn_setup.sh` / `cdn_upload.sh` | CDN 配置与资源上传（`--dry-run` 为默认） |
| `scripts/release.sh` | 发布流程辅助 |

管理端 CDN 云商管理接口（提供商列表 / 启停 / 配置更新 / dry-run 命令预览）由 admin-service 提供，详见 `../docs/api.md` 管理服务章节；云凭据不入库，命令需在部署机配置云 CLI 凭据后执行。

### 版本发布流程

项目版本（当前 v1.0.0，按 semver 演进）**独立于** e-cat 框架版本（当前 3.0.3）。

1. `CHANGELOG.md` 顶部新增版本节，格式 `## [x.y.z] — YYYY-MM-DD`，记录变更
2. 打 annotated tag：`git tag -a vX.Y.Z -m "vX.Y.Z"`，`git push origin vX.Y.Z`
3. 创建 release：`gh release create vX.Y.Z --title vX.Y.Z --notes-file <节>`，body 取自 CHANGELOG 对应节；最新版本自动置为 Latest

增量原则：只补缺失的 tag/release，已存在的跳过。

## 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DATABASE_URL` | `mysql://travel:pass@localhost:3306/travel` | MySQL 连接串 |
| `REDIS_URL` | `redis://localhost:6379` | Redis 连接串 |
| `JWT_SECRET` | 开发占位密钥 | JWT 签名密钥，需 ≥32 字节；未配置或长度不足时退回占位密钥并告警，**生产必须配置** |

## 架构（e-cat 框架）

```
┌──────────────────────────────────────────────────────────────┐
│                         ecat-cli                             │
│        (new │ proto │ run --watch │ build │ upgrade)         │
├──────────────────────────────────────────────────────────────┤
│                     ecat (应用生命周期)                         │
│      AppBuilder → App { name, servers, hooks, ... }         │
├────────────────────┬────────────────────┬────────────────────┤
│     transport      │    middleware      │     registry       │
│     ─────────      │    ──────────      │     ────────       │
│     HTTP (axum)    │    RecoveryLayer   │     memory         │
│     gRPC (tonic)   │    TracingLayer    │     consul         │
│     encoding       │    LoggingLayer    │                    │
│                    │    TimeoutLayer    │                    │
│                    │    RateLimitLayer  │                    │
│                    │    SecurityLayer   │                    │
│                    │    CircuitBreaker  │                    │
│                    │    Auth (JWT/API)  │                    │
├────────────────────┼────────────────────┼────────────────────┤
│     config         │     errors         │     metadata       │
│     file / env     │     ErrorCode      │     key-value      │
│     remote source  │     Error          │     HTTP/gRPC      │
├────────────────────┴────────────────────┴────────────────────┤
│                         data 层                               │
│     rdbms:   SQLite / PostgreSQL / MySQL / TiDB              │
│     cache:   Redis ✓ / Memcached（内存实现）                  │
│     search:  OpenSearch / Elasticsearch                      │
│     olap / graph / tsdb / document / storage: 另 11 个后端    │
├──────────────────────────────────────────────────────────────┤
│                       ecat-protos                             │
│     (共享 .proto 定义: errors, metadata, ...)                 │
└──────────────────────────────────────────────────────────────┘
```

### 请求处理流程

```
客户端请求
  │
  ├─ HTTP 0.0.0.0:8000 ──→ axum::Router ──┐
  │                                        │
  └─ gRPC 0.0.0.0:9000 ──→ tonic::Server ─┤
                                      │
                              ┌───────┴───────┐
                              │   Middleware   │
                              │ 1. Recovery    │  捕获 panic
                              │ 2. Tracing     │  注入 trace_id
                              │ 3. Logging     │  请求日志
                              │ 4. Security    │  攻击检测
                              │ 5. CircuitBrk  │  熔断保护
                              │ 6. Auth        │  认证鉴权
                              └───────┬───────┘
                                      │
                              ┌───────┴───────┐
                              │    Handler     │  业务逻辑
                              │ (tower::Service)│
                              └───────┬───────┘
                                      │
                              ┌───────┴───────┐
                              │   Response     │  JSON/Protobuf 编码
                              └───────────────┘
```

### 本项目中间件链

业务路由挂载完整中间件链（执行顺序：外层 → 内层）：

- **user-service**：`Tracing → CircuitBreaker → Security → RateLimit(Redis) → [profile 仅] Auth(JWT)`
- **booking-service**：`Tracing → CircuitBreaker → Security → RateLimit`（dates 为公开接口，无 JWT）

业务路由版本在 URL 前缀 `/api/v1/`，无版本前缀路由即 404。

**限流**：每服务 100 req/60s，Redis 分布式固定窗口；未认证请求也计入限流（防暴力请求耗尽资源），超限返回 429。

## e-cat 框架速览

### 技术栈

| 组件 | 选型 | 组件 | 选型 |
|------|------|------|------|
| 运行时 | **tokio** | RDBMS | **sqlx** |
| HTTP | **axum** | Redis | **redis-rs** |
| gRPC | **tonic** | JWT | **jsonwebtoken** |
| Protobuf | **prost + tonic-build** | HTTP Client | **reqwest** |
| 中间件 | **tower::Service / Layer** | CLI | **clap** |
| 日志/追踪 | **tracing + trace_id** | 指标 | **prometheus** |

### Kratos 概念映射

| Kratos (Go) | e-cat (Rust) | 说明 |
|-------------|-------------|------|
| `kratos.New()` | `App::builder()` | Builder 模式 |
| `http.Handler` | `tower::Service` | Rust 生态标准 trait |
| `http.Server` | `axum::Router` | 社区主流 HTTP 框架 |
| `grpc.Server` | `tonic::transport::Server` | 最成熟的 gRPC 实现 |
| `proto generate` | `prost + tonic-build` | 社区标准 protobuf |
| `registry.Discovery` | `Registry` trait | 可插拔注册发现 |
| `config.Source` | `ConfigSource` trait | 多源配置加载 |

### 数据后端

全部 18 类数据后端通过统一 trait（`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`）抽象，均提供 `XxxConfig` + `from_config()` 从 JSON/YAML 加载连接信息：RDBMS（SQLite/PG/MySQL/TiDB）、缓存（Redis/Memcached）、搜索（OpenSearch/Elasticsearch）、OLAP（ClickHouse）、图（Neo4j/NebulaGraph/ArangoDB）、时序（InfluxDB/IoTDB/QuestDB/TDengine）、文档（MongoDB）、对象存储（S3/MinIO）。

### 聚合 crate（ecat）

`ecat` 提供 feature-gated re-export 入口：`use ecat::transport_http::HttpServer;`（feature "http"）、`use ecat::auth::JwtAuthLayer;`（feature "auth"）等。默认 features = `http+grpc`；完整列表：`http` `grpc` `middleware` `auth` `client` `events` `metrics` `tracing` `circuit-breaker` `consul` `remote` `redis`。

### 错误处理

`ecat-errors` 提供 `ErrorCode` + `Error`，编译期映射 HTTP 状态码；错误响应经中间件编码为 JSON，携带 `code` / `reason` / `message`：

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

### 实现阶段与已知限制

- 框架 Phase 1–16 全部完成（详见 [CHANGELOG](CHANGELOG.md)）
- 已知限制：GraphQL 不支持别名/fragment/多顶层字段；OAuth2 内省缓存默认白名单过滤 claims；Kafka 默认 `auto_commit=false`（重启从分区末尾重读）

## 设计文档

- [API 参考](../docs/api.md)
- [项目规划](../docs/travel-project-planning.md)

## 支持

欢迎支持本项目！

| 微信支付 | 支付宝 |
|:---:|:---:|
| <img src="../docs/weixinpay.png" width="130" height="130" alt="微信支付"> | <img src="../docs/alipay.png" width="130" height="130" alt="支付宝"> |

### 全球转账（银行汇款）

| 项目 | 信息 |
|------|------|
| 收款人姓名 | WANG KEXUN |
| 收款账户号码 | 881015918251 |
| 收款银行 | ZA Bank Limited |
| SWIFT Code | AABLHKHHXXX |
| 银行编号 | 387 |
| 银行地址 | Core F, Cyberport 3, 100 Cyberport Road, Hong Kong |

> **跨境汇款代理银行（如需）**：此为代理银行（中转银行）信息，非收款银行信息，请向汇款银行查询是否需要提供。
>
> - 汇入港元、人民币及美元：**Citibank N.A. Hong Kong**（SWIFT：`CITIHKHXXXX`，银行编号：006，分行：Hong Kong Branch，分行编号：391，地址：Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong）
> - 汇入其他币种：**THE BANK OF NEW YORK MELLON**（SWIFT：`IRVTUS3NXXX`，地址：240 GREENWICH STREET, NEW YORK, United States）

## 许可证

Apache-2.0
