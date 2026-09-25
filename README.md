# Open Travel — 全球旅游平台

<p align="center"><img src="docs/mascot.svg" alt="Dora 小途 — Open Travel 吉祥物" width="180"></p>
<p align="center"><sub>吉祥物「小途」猫造型基于 <a href="https://github.com/jdecked/twemoji">Twemoji</a>（CC-BY 4.0）修改 · 呼应 e-cat（一只猫）框架</sub></p>

[English](docs/i18n/en/README.md) | [日本語](docs/i18n/ja/README.md) | [한국어](docs/i18n/ko/README.md) | [Русский](docs/i18n/ru/README.md) | [Deutsch](docs/i18n/de/README.md) | [Français](docs/i18n/fr/README.md) | [Español](docs/i18n/es/README.md) | [Português](docs/i18n/pt/README.md) | [हिन्दी](docs/i18n/hi/README.md) | [العربية](docs/i18n/ar/README.md) | [বাংলা](docs/i18n/bn/README.md) | [Bahasa Indonesia](docs/i18n/id/README.md)

> 一个面向全球用户的旅游预订平台：Rust 微服务后端 + Flutter / HarmonyOS 多端客户端，支持 **12+ 种语言**、国际支付与多语言搜索。

## 项目简介

Open Travel 是一个全球旅游平台 monorepo，采用 **e-cat（一只猫）** —— 对标 [go-kratos/kratos](https://github.com/go-kratos/kratos) v3 的 **Rust 微服务框架**（v3.0.3 · 52 crates）—— 构建高性能后端，配合 Flutter 多端与鸿蒙原生客户端，为全球用户提供统一的旅行预订体验。

| 维度 | 说明 |
| :--- | :--- |
| **后端框架** | e-cat（Rust）：HTTP/axum + gRPC/tonic，52 crates 微服务生态 |
| **多端客户端** | `apps/client/flutter`（iOS / Android / Web / Desktop）、`apps/client/harmonyos`（鸿蒙）、`apps/admin`（Flutter Web 管理端） |
| **数据库** | MySQL（库名 `travel`，表前缀 `travel_`）+ Redis 缓存 + OpenSearch 多语言搜索 |
| **安全** | ecat-security / ecat-auth（JWT）/ ecat-tls：认证、审计、限流、防注入 |
| **国际化** | 12+ 语种 ARB 语言包，RTL 支持，OpenSearch 多语言分词 |
| **支付** | 微信支付、支付宝 |

## 项目吉祥物「小途」

一只戴探险帽、拖着贴纸行李箱追纸飞机的琥珀色旅行猫，由 **e-cat（一只猫）** 框架「生」出来。猫脸造型基于 [Twemoji](https://github.com/jdecked/twemoji) 1f431（CC-BY 4.0）修改，探险帽 / 行李箱 / 纸飞机为原创。矢量源与完整设定见 [`docs/mascot.svg`](docs/mascot.svg)。

| 落点 | 形式 |
| :--- | :--- |
| `docs/mascot.svg` | **矢量唯一源**（全身像 512×512） |
| `apps/*/web/favicon.svg` | 浏览器标签图标（脸部特写版，16px 下仍可辨认；`favicon.png` 16px 兜底旧浏览器） |
| `apps/*/web/icons/Icon-*.png` | PWA / 主屏图标 192·512（含 maskable 安全区版） |
| `apps/*/assets/mascot.png` | Flutter 应用内展示（管理端登录页、客户端个人页） |
| `apps/client/harmonyos/.../media/mascot.svg` | 鸿蒙应用内（`Image` 原生渲染 SVG；移动端简化变体） |
| 各 README / crate 文档 | 页首品牌位 |

> 改造型只改 `docs/mascot.svg`，其余均为派生件：favicon 是它的脸部特写裁剪（去掉胡须/额纹/行李等小尺寸糊掉的笔画），鸿蒙变体去掉 `<defs>`/渐变改用纯色底色以适配移动端 SVG 渲染器。

## 一键安装

环境要求：Docker + Docker Compose（v2）。（Rust 工具链仅源码构建需要）

```bash
git clone <仓库地址> && cd open-travel
./scripts/install.sh
```

脚本自动：检查环境 → 构建并启动全部服务（MySQL/Redis/OpenSearch/Kafka/9 个微服务/Nginx 网关）→ 初始化搜索索引 → 健康巡检，完成后打印访问地址与默认管理账号。

> 使用说明：访问 http://localhost:8082/health 探活；管理端登录 `admin@travel.local` / `Admin@123`；客户端（`apps/client/flutter`）用 `flutter run -d chrome` 启动。

## 快速开始

> 以下使用说明聚焦后端（Rust 微服务）；客户端（apps/）构建方式见各子目录。

### 环境要求

- Rust 1.85+（stable 工具链，edition 2024）
- Docker + Docker Compose

### 构建与启动

```bash
cd e-cat
cargo check -p ecat --bin user-service --bin booking-service --bin admin-service \
  --bin search-service --bin line-service --bin order-service --bin flight-service \
  --bin hotel-service --bin payment-service   # 编译检查业务服务

docker compose -f config/docker-compose.yml up -d   # 启动数据源 + 服务 + 网关
```

> 不要使用 `--env-file .env` 启动（会报错）。

### 端口

| 服务 | 端口 | 说明 |
|------|------|------|
| user-service | 8001 | 用户注册 / 登录 / 资料 |
| booking-service | 8002 | 热门目的地日期 + 景区列表/详情 + 评价 |
| admin-service | 8003 | 管理端：登录 + 目的地/景区 CRUD |
| search-service | 8004 | 多语种搜索 |
| line-service | 8005 | 旅游线路 |
| order-service | 8006 | 订单 |
| flight-service | 8007 | 机票 |
| hotel-service | 8008 | 酒店 |
| payment-service | 8009 | 支付 |
| Nginx 网关 | 8082→80 | 按 `/api/v1/user/`、`/api/v1/booking/`、`/api/v1/admin/`、`/api/v1/search`、`/api/v1/lines`、`/api/v1/orders`、`/api/v1/flights`、`/api/v1/hotels`、`/api/v1/payments` 前缀分流 |
| MySQL | 3308→3306 | 数据源（宿主端口冲突，临时映射） |
| Redis | 6381→6379 | 缓存 / 限流 |
| OpenSearch | 9201→9200 | 多语言搜索 |

### 验证

> 业务接口版本在 URL 前缀：`/api/v1/...`（无需版本请求头）。

```bash
curl http://localhost:8082/health
curl "http://localhost:8082/api/v1/booking/dates?region_id=1"
# {"code":0,"message":"ok","data":[{"region_id":1,"name_en":"placeholder-destination"}]}

# 管理端登录（admin@travel.local / Admin@123 为开发环境默认账号，仅本地使用）
curl -X POST http://localhost:8082/api/v1/admin/login -H "Content-Type: application/json" \
  -d '{"email":"admin@travel.local","password":"Admin@123"}'
# {"code":0,"message":"ok","data":{"token":"<jwt>"}}

# 景区列表（按目的地查询，多语种字段按 lang 返回）
curl "http://localhost:8082/api/v1/booking/attractions?destination_id=1"
# {"code":0,"message":"ok","data":[{...}]}
```

### 脚本

| 脚本 | 用途 |
|------|------|
| `scripts/install.sh` | 一键安装：环境检查 → 启动全部服务 → 初始化搜索索引 → 健康巡检 → 打印访问地址与账号（幂等，可重复执行） |
| `scripts/up.sh` / `down.sh` | 启动 / 停止 compose 全部服务 |
| `scripts/deploy.sh` | 重新构建并重启服务 |
| `scripts/health_check.sh` | 服务健康巡检 |
| `scripts/opensearch_init.sh` | 幂等创建 OpenSearch 索引（cjk 分析器） |
| `scripts/loadtest.sh` | 压测 |
| `scripts/reconcile_payments.sh` | 支付订单对账 |
| `scripts/env_audit.sh` | 环境变量与配置审计 |
| `scripts/cdn_setup.sh` / `cdn_upload.sh` | CDN 配置与上传（`--provider` 八云插件：cloudfront/aliyun/gcp/azure/cloudflare/tencent/huawei/bunny，`--dry-run` 默认） |

### 后端文档

- 后端 README（含中间件链、环境变量、版本发布流程）：[e-cat/README.md](e-cat/README.md)
- API 参考（端点、鉴权、限流）：[docs/api.md](docs/api.md)

## 核心特性

- 🏨 目的地 / 酒店 / 机票多语言搜索与预订
- 🌍 12+ 语种独立适配（中、英、日、韩、阿、西、法、德……）
- 💳 国际支付（微信支付 / 支付宝）
- 🔐 安全纵深：TLS 1.3、JWT 认证、审计日志、输入过滤、限流、支付回调 HMAC 验签与内部服务鉴权
- 📱 多端一致体验：Flutter（iOS/Android/Web/Desktop）+ 鸿蒙

## 架构设计图

![架构设计图](docs/svg/architecture.svg)

## 功能设计图

![功能设计图](docs/svg/features.svg)

## 请求生命周期图

![请求生命周期图](docs/svg/request-cycle.svg)

## 安全架构图

![安全架构图](docs/svg/security-architecture.svg)

> 分层防护：Tracing → CircuitBreaker → Security → RateLimit(Redis，100 req/60s) → JWT（仅受保护路由）；API 版本在 URL 前缀 `/api/v1/`，未知版本由网关 404；支付回调 HMAC 验签 + 内部 X-Internal-Token 鉴权。

## 项目结构图

![项目结构图](docs/svg/project.svg)

> 上列各图均有 12 语种译本，位于 `docs/svg/<lang>/`，由对应语言 README 引用。

## 项目结构

```
open-travel/
├── apps/                  # 多端客户端与管理端
│   ├── client/
│   │   ├── flutter/       # Flutter：iOS / Android / Web / Desktop（12+ 语种 i18n，web/favicon.svg 为「小途」图标）
│   │   └── harmonyos/     # 鸿蒙原生客户端
│   └── admin/             # Flutter Web 管理端
├── e-cat/                 # e-cat 框架 + 业务服务（同一 Cargo workspace）
│   ├── ecat*/             # 52 个 ecat-* 框架 crate
│   ├── ecat/              # 主框架 crate：门面 + 业务模块（src/business/）+ 服务入口（src/bin/ 9 个服务）
│   ├── config/            # 框架配置示例
│   ├── examples/          # 框架示例项目
│   └── CHANGELOG.md       # 框架 + 项目版本变更日志（版本号即项目版本，见 tags）
├── docs/                  # 技术文档
│   ├── api.md             # API 参考（端点、鉴权、限流）
│   ├── mascot.svg         # 吉祥物「小途」（矢量唯一源，favicon 与各端图标由此派生）
│   ├── svg/               # 架构 / 功能 / 生命周期 / 安全 / 结构 图（含 12 语种译本）
│   ├── i18n/              # 12 语种 README
│   └── coin/              # 打赏二维码
├── config/                # 环境与部署配置（nginx.conf、docker-compose.yml、schema.sql）
├── scripts/               # 安装 / 部署 / 巡检 / 压测 / CDN / 种子数据
├── .github/workflows/     # CI
└── README.md
```

## 数据库

- 数据库名：`travel`
- 表前缀：`travel_`（例如 `travel_users`、`travel_orders`、`travel_reviews`）
- 配套存储：Redis（会话 / 热门缓存）、OpenSearch（多语言搜索索引）

> 详细技术规划见 [docs/travel-project-planning.md](docs/travel-project-planning.md)。

---

## 支持我们

如果这个项目对你有帮助，欢迎请作者喝一杯咖啡 ☕

<p align="center">
  <strong>微信支付（WeChat Pay）</strong> &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp; <strong>支付宝（Alipay）</strong><br/>
  <img src="docs/weixinpay.png" alt="微信支付二维码" width="130" height="130" />
  &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
  <img src="docs/alipay.png" alt="支付宝二维码" width="130" height="130" />
</p>

### 全球转账打赏（Global Bank Transfer）

**收款人信息**

- 收款人姓名：WANG KEXUN
- 收款账户号码：881015918251

**收款银行**

- ZA Bank SWIFT Code：AABLHKHHXXX
- 银行名称：ZA Bank Limited
- 银行编号：387
- 银行地址：Core F, Cyberport 3, 100 Cyberport Road, Hong Kong

**跨境汇款代理银行（如需）**

请留意，此为跨境汇款代理银行（中转银行）信息，非收款银行信息。请向汇款银行查询是否需要提供跨境汇款代理银行信息。

汇入港元、人民币及美元的代理银行为 **Citibank** ——

- 银行名称：Citibank N.A. Hong Kong
- SWIFT Code：CITIHKHXXXX
- 银行编号：006
- 分行名称：Hong Kong Branch
- 分行编号：391
- 银行地址：Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong

汇入其他币种时的代理银行为 **BNY Mellon** ——

- 银行名称：THE BANK OF NEW YORK MELLON
- SWIFT Code：IRVTUS3NXXX
- 银行地址：THE BANK OF NEW YORK MELLON, 240 GREENWICH STREET, NEW YORK, United States

### 虚拟币打赏 (Crypto Donation)

如果这个项目对你有帮助，欢迎扫描二维码打赏支持，谢谢！

| 主网 (Network) | 二维码 (QR Code) | 钱包地址 (Wallet Address) |
|---|---|---|
| BNB Smart Chain (BEP20) | [<img src="docs/coin/1.jpg" width="150" alt="BNB Smart Chain (BEP20)">](docs/coin/1.jpg) | `0x355d429f97511897ccb4e271ec888205f9ab6629` |
| Tron (TRC20) | [<img src="docs/coin/2.jpg" width="150" alt="Tron (TRC20)">](docs/coin/2.jpg) | `TEdDHWLajt1XvqtPDWmQctdrJaC3pzZZzz` |
| Ethereum (ERC20) | [<img src="docs/coin/3.jpg" width="150" alt="Ethereum (ERC20)">](docs/coin/3.jpg) | `0x355d429f97511897ccb4e271ec888205f9ab6629` |
| Aptos | [<img src="docs/coin/4.jpg" width="150" alt="Aptos">](docs/coin/4.jpg) | `0x836e3780edfc3f7b2372b39e2a1a3a5d7adfaccd96c726f21cfde1b50dd68030` |
| Plasma | [<img src="docs/coin/5.jpg" width="150" alt="Plasma">](docs/coin/5.jpg) | `0x355d429f97511897ccb4e271ec888205f9ab6629` |
| Polygon POS | [<img src="docs/coin/6.jpg" width="150" alt="Polygon POS">](docs/coin/6.jpg) | `0x355d429f97511897ccb4e271ec888205f9ab6629` |
| Solana | [<img src="docs/coin/7.jpg" width="150" alt="Solana">](docs/coin/7.jpg) | `2hfhboHdmdrYsY25XfQSsEWxq5ip4EQsR7f4AzSRMUyr` |
| The Open Network (TON) | [<img src="docs/coin/8.jpg" width="150" alt="The Open Network (TON)">](docs/coin/8.jpg) | `UQB9kFQohzmXUir9QSSZq01iwl9aQZIDdBpNmDklljRtCoGK` |
| Arbitrum One | [<img src="docs/coin/9.jpg" width="150" alt="Arbitrum One">](docs/coin/9.jpg) | `0x355d429f97511897ccb4e271ec888205f9ab6629` |
| AVAX C-Chain | [<img src="docs/coin/10.jpg" width="150" alt="AVAX C-Chain">](docs/coin/10.jpg) | `0x355d429f97511897ccb4e271ec888205f9ab6629` |
