# Open-Travel 全球旅游平台 — 项目规划

> 技术栈：**Rust e-cat 微服务框架**（v3.0.2 · 51 crates）· Flutter 多端 + HarmonyOS · MySQL + Redis + OpenSearch
> 覆盖 12+ 语种的全平台 i18n 适配

---

## 一、项目目录结构

```
open-travel/
├── apps/
│   ├── client/           # 客户端目录
│   │   ├── flutter/      # 多端客户端（iOS / Android / Web / Desktop），12+ 语种 ARB 资源
│   │   ├── harmonyos/    # 鸿蒙端（ArkUI）
│   │   └── (未来) wechat-mini/  # 微信小程序
│   └── admin/            # 管理端（Flutter Web）
├── e-cat/                # Rust 微服务框架「一只猫」，v3.0.2，51 crates
│   ├── ecat/                 # 应用生命周期：AppBuilder → App
│   ├── ecat-transport-http/  # HTTP（axum）
│   ├── ecat-transport-grpc/  # gRPC（tonic）
│   ├── ecat-auth/            # JWT / API Key 认证
│   ├── ecat-security/        # SecurityLayer 攻击检测
│   ├── ecat-tls/             # TLS
│   ├── ecat-data-sqlx/       # RDBMS：MySQL / PostgreSQL / SQLite / TiDB
│   ├── ecat-data-redis/      # Redis 缓存
│   ├── ecat-data-opensearch/ # OpenSearch 搜索
│   ├── ecat-mq-kafka/        # Kafka 消息 / 审计日志
│   ├── ecat-registry-consul/ # Consul 注册中心（另有 etcd）
│   └── ecat-cli/             # CLI：new / proto / run --watch / build / upgrade
├── docs/                 # 文档（本文件）
├── config/               # 配置：docker-compose.yml、opensearch.yml 等
└── README.md
```

---

## 二、整体架构

```
┌──────────────────────────────┐
│  apps/client/flutter / apps/client/harmonyos│
│  (iOS/Android/Web/Desktop/鸿蒙)│
└──────────────┬───────────────┘
               │ HTTPS / gRPC / WebSocket
┌──────────────▼───────────────┐
│         API Gateway          │  ← Nginx / Envoy（限流、TLS 终止、路由）
└──────────────┬───────────────┘
┌──────────────▼───────────────┐
│     e-cat Core Services (Rust)│
│  ┌───────────┬─────────────┐ │
│  │ Booking   │ User        │ │
│  ├───────────┼─────────────┤ │
│  │ Review    │ Search      │ │
│  └───────────┴─────────────┘ │
│  Middleware 链（tower::ServiceBuilder）│
│  Recovery → Tracing → Logging → Auth │
│  → RateLimit → Security → CircuitBreaker │
└──────────────┬───────────────┘
               │
┌──────────────▼───────────────┐
│          Data Layer          │
│  MySQL（读写分离）+ Redis + OpenSearch│
│  + Kafka / ClickHouse（审计、事件）  │
└──────────────────────────────┘
```

服务间注册发现：**Consul**（`ecat-registry-consul`）/ etcd（`ecat-registry-etcd`），gRPC 服务间调用走 `GrpcClient`（集成服务发现与负载均衡 + CircuitBreaker 熔断）。

---

## 三、后端：e-cat（Rust 微服务框架）

### 3.1 框架事实

- e-cat（中文名「一只猫」）对标 go-kratos/kratos v3，版本 v3.0.2，共 51 crates，位于仓库 `e-cat/` 子目录。
- **HTTP**：axum（`ecat-transport-http`）；**gRPC**：tonic（`ecat-transport-grpc`）；Protobuf：prost + tonic-build。
- **中间件**（HTTP/gRPC 共用同一套 tower::Layer）：RecoveryLayer（捕获 panic）、TracingLayer（trace_id 注入）、LoggingLayer、TimeoutLayer、RateLimitLayer、SecurityLayer（攻击检测）、CircuitBreaker（熔断）、Auth（JWT/API Key）、MetricsLayer（Prometheus）。
- **数据层**：`ecat-data-sqlx`（MySQL/PostgreSQL/SQLite/TiDB）、`ecat-data-redis`、`ecat-data-opensearch`。
- **消息与事件**：`ecat-mq-kafka`（MessageQueue trait + EventBus Pub/Sub）。
- **CLI**：`ecat-cli`（new / proto / run --watch / build / upgrade），API-first 开发流程。
- 端口约定：HTTP `:8000`，gRPC `:9000`；健康检查 `/health`、`/ready`。

### 3.2 服务骨架（Rust / axum 示例）

```rust
// e-cat/services/booking/src/main.rs
use ecat::App;
use ecat_auth::jwt::JwtAuthLayer;
use ecat_middleware::{RateLimitLayer, SecurityLayer, TracingLayer, CircuitBreaker};
use ecat_data_sqlx::MySqlPool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = MySqlPool::connect("mysql://travel:pass@mysql:3306/travel").await?;

    App::builder("travel-booking")
        .http("0.0.0.0:8000", |router| {
            router
                .route("/api/v1/booking/dates", axum::routing::get(get_available_dates))
                .layer(JwtAuthLayer::<UserClaims>::default())
                .layer(RateLimitLayer::new(100, std::time::Duration::from_secs(60)))
                .layer(SecurityLayer::default())
                .layer(CircuitBreaker::default())
                .layer(TracingLayer::default())
        })
        .registry(ConsulRegistry::connect("consul:8500").await?)
        .run()
        .await
}
```

### 3.3 多语言内容查询链路（Redis → MySQL 从库 → 审计）

```rust
// 1. Redis 缓存热门目的地（TTL 5 分钟）
let cache_key = format!("hot_destinations:{}", region_id);
if let Some(hit) = redis.get::<String>(&cache_key).await? {
    return Ok(hit);
}

// 2. 回源 MySQL 从库（读写分离，预编译语句防注入）
let rows: Vec<Destination> = sqlx::query_as(
    "SELECT * FROM travel_destinations WHERE region_id = ?"
).bind(region_id).fetch_all(&read_pool).await?;

// 3. 审计日志异步写入 Kafka（ecat-mq-kafka），不阻塞主流程
let _ = mq.publish("audit.log", AuditEvent {
    user_id, action: "GET /booking/dates", ip,
}).await;

// 4. 回填缓存
let _ = redis.set_ex(&cache_key, &serde_json::to_string(&rows)?, 300).await;
```

---

## 四、安全层（e-cat 内置 crates，非独立服务）

- **ecat-security**：`SecurityLayer` 自动检测 SQL 注入、XSS、SSRF 等攻击模式，阻断高危请求，挂在服务中间件链上即可，无需独立二进制。
- **ecat-auth**：JWT / API Key 认证中间件，Claims 注入请求上下文。
- **ecat-tls**：TLS 支持（网关层终止，或服务间 mTLS）。
- **审计日志**：敏感操作（预订确认、支付完成）经 `ecat-mq-kafka` 异步写入 Kafka，下游可落 ClickHouse 供审计查询。

```rust
// 服务内集成示例：不需要额外安全服务，直接挂中间件
router
    .layer(SecurityLayer::default())
    .layer(JwtAuthLayer::<UserClaims>::default());
```

安全规则库随 e-cat 框架版本迭代更新，升级走 cargo 依赖管理。

---

## 五、多语言 i18n（12+ 语种）

### 5.1 后端搜索多语言（OpenSearch）

`destination` 索引的 `name` / `description` 按语种分字段（en/es/fr/de/ja/ko/ar/…），分词器按语言族配置：

- 屈折语（英/德/法/西/葡）：**ICU Normalizer + Snowball stemmer**
- 日语：**kuromoji**
- 阿拉伯语：ICU normalization（配合 RTL 排版）

```json
{
  "settings": { "analysis": { "analyzer": {
    "multilang_analyzer": {
      "type": "custom",
      "tokenizer": "icu_tokenizer",
      "filter": ["icu_normalizer", "snowball"]
    },
    "ja_analyzer": { "tokenizer": "kuromoji_tokenizer" }
  } } },
  "query": { "multi_match": {
    "query": "mountain resort",
    "fields": ["name.en^2", "name.ja", "name.de"],
    "type": "best_fields"
  } },
  "highlight": { "pre_tags": ["<em>"], "post_tags": ["</em>"] }
}
```

### 5.2 Flutter 端（apps/client/flutter）

- `flutter_localizations` + `intl`，ARB 文件按语种组织：`assets/i18n/{lang}.arb`。
- 语种清单（12+）：en、zh、ja、ko、ru、de、fr、es、pt、hi、ar、bn、id。
- RTL 语种（阿拉伯语）自动切换 `Directionality` / ReadingOrder。
- 桌面端（Windows/macOS）侧边栏 + 列表布局；移动端卡片流 + 分页加载。

### 5.3 鸿蒙端（apps/client/harmonyos）

- ArkUI + 系统 i18n 能力，resource 按语种分包。
- 与 Flutter 共享后端 API（REST + WebSocket），未来可扩展微信小程序端。

---

## 六、数据库设计

- **库名**：`travel`；**表前缀**：`travel_`（如 `travel_users`、`travel_orders`、`travel_destinations`、`travel_reviews`、`travel_bookings`）。
- **读写分离**：主库写（订单创建、支付回调、注册），从库读（目的地列表、评论），应用层按 pool 路由（见 3.3）。
- **Redis**：会话存储（TTL 30 分钟）、热门内容缓存（TTL 5 分钟）、OpenSearch 降级缓存。
- 全表 `utf8mb4`，时间字段 DATETIME + `CURRENT_TIMESTAMP`。

```sql
CREATE TABLE travel_users (
  id            BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  email         VARCHAR(255) NOT NULL UNIQUE,
  password_hash VARCHAR(255) NOT NULL,
  lang          VARCHAR(8)   NOT NULL DEFAULT 'en',
  created_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE travel_orders (
  id            BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  user_id       BIGINT UNSIGNED NOT NULL,
  destination_id BIGINT UNSIGNED NOT NULL,
  status        TINYINT NOT NULL DEFAULT 0,   -- 0 待支付 / 1 已支付 / 2 已取消
  amount_cents  BIGINT NOT NULL,
  created_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  INDEX idx_user (user_id),
  INDEX idx_status_created (status, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

---

## 七、部署与 CI/CD

### 7.1 docker-compose（`config/docker-compose.yml`）

```yaml
services:
  mysql:
    image: mysql:8.0
    environment:
      - MYSQL_ROOT_PASSWORD=secret123
      - MYSQL_DATABASE=travel        # 库名 travel
  redis:
    image: redis:alpine
  opensearch:
    image: opensearchproject/opensearch:latest
    volumes:
      - ./config/opensearch.yml:/usr/share/opensearch/config/opensearch.yml
  ecat-api:
    build: ./e-cat
    depends_on: [mysql, redis, opensearch]
    ports: ["8080:8000"]
  flutter-web:
    build: ./apps/client/flutter
```

### 7.2 GitHub Actions

1. **Flutter**：构建 Web 端 → 打包静态 HTML/CSS/JS。
2. **Rust**：`cargo test` 单元测试 + `cargo clippy` 代码扫描 + `cargo audit` 依赖安全审计。
3. **部署**：镜像构建 → Kubernetes（阿里云 ACK / AWS EKS），HPA 自动扩缩容。

---

## 八、阶段规划

| 阶段 | 周期 | 任务重点 |
| :--- | :--- | :--- |
| **Phase 1** | 2-3 周 | e-cat 基础服务搭建 + MySQL（读写分离）/Redis/OpenSearch 集成，验证缓存与搜索链路 |
| **Phase 2** | 4-5 周 | Flutter 多端布局适配 + 12+ 语种 ARB 资源编写 + 鸿蒙端骨架 |
| **Phase 3** | 2 周 | 安全加固：SecurityLayer 规则调优 + JWT 认证 + 审计日志入 Kafka/ClickHouse |
| **Phase 4** | 1-2 周 | 全链路联调 + CDN 加速（八云插件：CloudFront/阿里云/腾讯云/GCP/Azure/Cloudflare/华为云/Bunny，管理端可配置管理）+ 压测 |

## 九、成本估算（首年，AWS 参考）

| 项 | 配置 | 月成本 |
| :--- | :--- | :--- |
| 服务器 | EC2 t3.medium (2 vCPU / 4GB) × 2 | ≈ $70 |
| 数据库 | RDS MySQL Medium | ≈ $80 |
| 缓存 | ElastiCache Redis | ≈ $60 |
| 搜索 | OpenSearch t3.small | ≈ $50 |
| CDN | 八云插件（CloudFront 免费额度内起步） | ≈ $0 |

**合计约 $3,200 – $4,500/月**，初期可用各云厂商免费额度起步。

---

## 十、关键决策与注意事项

1. **服务间通信**：统一 gRPC（tonic），对外 HTTP 走 axum + 网关；Consul 注册发现 + CircuitBreaker 熔断。
2. **多语言资源**：大体积 ARB 文件用 Git LFS 管理，避免仓库膨胀。
3. **Flutter 性能**：图片用 `flutter_cache_manager` 本地缓存；时区用 `flutter_native_timezone` 获取而非依赖服务器时间。
4. **安全规则更新**：ecat-security 规则库随框架版本升级，纳入依赖升级流程（Dependabot）。
5. **API-first**：所有接口先写 .proto（ecat-cli proto 生成），错误码走 protobuf 错误码体系，编译期映射 HTTP 状态码。
