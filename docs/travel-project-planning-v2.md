# Open Travel 全球旅游平台 — 项目规划 v2

> 技术栈：**Rust e-cat 微服务框架**（v3.0.3 · 51 crates）· Flutter 多端 + HarmonyOS · MySQL + Redis + OpenSearch
> 覆盖 12+ 语种的全平台 i18n · 本文档基于 2026-08-29「已实现功能事实清单」编写，标注**已实现**的均为事实，其余为**规划中**

---

## 一、项目定位与目标

Open Travel 是面向全球用户的旅游平台 monorepo，覆盖**景点、景区、旅游线路、机票、酒店**五大业务域，提供一站式查询、预订、支付与行程管理能力。

| 维度 | 目标 |
| :--- | :--- |
| 业务范围 | 目的地 / 景点景区 / 旅游线路 / 机票 / 酒店，从内容浏览到预订支付全链路 |
| 用户端 | Flutter（iOS / Android / Web / Desktop）+ HarmonyOS（ArkUI），未来扩展微信小程序 |
| 管理端 | Flutter Web，运营人员管理内容、订单、用户与数据看板 |
| 语言 | 12+ 语种（en/zh/ja/ko/ru/de/fr/es/pt/hi/ar/bn/id），RTL 语种支持 |
| 架构 | e-cat 微服务（Rust），MySQL 读写分离 + Redis + OpenSearch，Nginx 网关 |
| 当前阶段 | 早期骨架：基础设施、认证、基础页面已通；业务域（线路/机票/酒店/支付）未实现 |

---

## 二、功能全景

### 2.1 客户端功能矩阵

| 模块 | 功能 | 状态 |
| :--- | :--- | :--- |
| 首页 | 热门目的地列表 + 日期展示（硬编码数据） | **已实现** |
| 导航壳 | 首页 / 预订 / 我的 Tab 切换 | **已实现** |
| 目的地 | 目的地列表、详情（鸿蒙端） | **已实现**（骨架） |
| 用户 | 注册 / 登录 / 个人资料（Flutter 页对接后端接口） | **已实现**（后端完整，页面部分） |
| 我的 | 个人中心页 | **已实现**（骨架） |
| 搜索 | 多条件搜索（关键词/目的地/日期/价格） | 规划中（Phase 3） |
| 景点景区 | 列表、详情、图文、评价 | 规划中（Phase 2-3） |
| 旅游线路 | 线路列表、行程详情、报名预订 | 规划中（Phase 3-4） |
| 机票 | 航班查询、比价、预订 | 规划中（Phase 4-5） |
| 酒店 | 酒店搜索、房型、预订 | 规划中（Phase 4-5） |
| 订单中心 | 订单列表、详情、取消、支付引导 | 规划中（Phase 3-4） |
| 支付 | 在线支付（微信/支付宝等）、回调处理 | 规划中（Phase 4） |
| 多语言 | 13 个 ARB 文件、语言切换 | **已实现**（资源），页面全量适配规划中 |

### 2.2 管理端功能矩阵

管理端仅完成 Flutter Web 工程骨架（单页 AppBar）；完整功能矩阵见「七、管理端规划」（登录、目的地/景区/线路/机票/酒店/订单/用户管理、数据看板，均规划中）。

---

## 三、已实现基础（事实清单）

### 3.1 服务与接口

- **user-service**（`e-cat/services/user`）：注册 / 登录 / 资料查询，JWT 认证
  - `POST /api/user/register` — 公开；邮箱格式 + 密码 ≥6 位校验，bcrypt cost 12，重复邮箱返回 409
  - `POST /api/user/login` — 公开；统一 401 防账号枚举，签发 JWT（24h）
  - `GET /api/user/profile` — 需 JWT，返回 id / email / lang
- **booking-service**（`e-cat/services/booking`）：
  - `GET /api/booking/dates?region_id=N` — 公开；Redis 缓存 `hot_destinations:{id}`（TTL 300s）→ MySQL 回源（`travel_destinations`）→ 占位数据兜底
- 两服务均提供 `/health`、`/ready`；所有业务请求强制 `X-Api-Version: v1`（缺失返回 400）
- 中间件链：`Tracing → CircuitBreaker → Security → RateLimit(Redis 固定窗口 100 次/60s，超限 429)`；仅 profile 挂 JWT

### 3.2 数据层

库 `travel`，表前缀 `travel_`（`config/schema.sql`，当前**仅 DDL + 目的地种子数据**）：

| 表 | 关键字段 | 数据 |
| :--- | :--- | :--- |
| `travel_users` | email、password_hash、lang | 空（运行时写入） |
| `travel_destinations` | name_en/zh/ja、description JSON、region_id | 5 条种子城市 |
| `travel_bookings` | check_in/out、guests、status 0-3、amount_cents | 空 |
| `travel_orders` | user_id、booking_id、status、amount_cents | 空 |
| `travel_reviews` | rating 1-5、lang | 空 |

### 3.3 客户端

- **Flutter**（`apps/client/flutter/lib/pages/`）：`home_page`（硬编码目的地 + 日期展示）、`home_shell`（导航壳）、`booking_list_page`、`profile_page`；`services/` 有 `api_client`、`localization_service`
- **HarmonyOS**（ArkUI）：`Index`（Tab 导航）、`DestinationList`、`DestinationDetail`、`Profile`

### 3.4 多语言与部署

- 13 个 ARB 文件（en/zh/ja/ko/ru/de/fr/es/pt/hi/ar/bn/id）；`docs/i18n` 下 12 语种文档翻译
- Nginx 网关：宿主 8082 → 容器 80，`/api/user/*` → user-service、`/api/booking/*` → booking-service

### 3.5 阶段进度小结

| 原规划阶段 | 实际状态 |
| :--- | :--- |
| Phase 1 基础服务 + MySQL/Redis 集成、缓存链路 | **已完成** |
| Phase 2 多端布局 + 12+ 语种 ARB + 鸿蒙骨架 | **部分完成**（i18n 资源与鸿蒙骨架已落地，业务页面未做） |
| Phase 3 安全加固（限流 / JWT / SecurityLayer） | **已完成**（审计日志入 Kafka 未实现，规划中） |
| Phase 4 全链路联调 + CDN + 压测 | 未达 |

---

## 四、系统架构

### 4.1 微服务拆分规划

```
客户端 (Flutter 多端 / HarmonyOS)
        │ HTTPS
        ▼
  Nginx API Gateway（限流 / TLS / 路由分流）
        │
        ▼
┌─────────────────────────────────────────────────┐
│  e-cat 微服务（Rust，Consul 注册发现 + gRPC 内调） │
│  ┌───────┬─────────┬────────┬────────┬────────┐ │
│  │ user  │ booking │ search │ order  │ payment│ │
│  ├───────┼─────────┼────────┼────────┼────────┤ │
│  │ line  │ flight  │ hotel  │ review │ admin  │ │
│  └───────┴─────────┴────────┴────────┴────────┘ │
│  中间件链：Tracing → CircuitBreaker → Security    │
│  → RateLimit → Auth（全服务统一挂载）             │
└──────────────────────┬──────────────────────────┘
                       ▼
  数据层：MySQL 读写分离 + Redis + OpenSearch + Kafka
```

| 服务 | 职责 | 状态 |
| :--- | :--- | :--- |
| user-service | 注册/登录/资料 | **已实现**（基础） |
| booking-service | 目的地/景点内容、预订日期 | **已实现**（dates 接口） |
| search-service | 多语种全文检索、聚合过滤 | 规划中（Phase 3） |
| order-service | 订单创建/状态机/改退 | 规划中（Phase 3-4） |
| payment-service | 支付渠道对接、回调、对账 | 规划中（Phase 4） |
| line-service | 旅游线路商品域 | 规划中（Phase 3-4） |
| flight-service | 机票查询/舱位/预订 | 规划中（Phase 4-5） |
| hotel-service | 酒店/房型/房价 | 规划中（Phase 4-5） |
| review-service | 评价/评分（或并入 booking） | 规划中（Phase 5） |
| admin-service | 管理端鉴权/运营接口 | 规划中（Phase 3） |

服务间通信统一 gRPC（tonic）+ Consul 注册发现 + CircuitBreaker；对外 HTTP 走 axum + 网关。

### 4.2 数据层

- **MySQL 读写分离**：主库写（订单、支付回调、注册），从库读（目的地列表、评论），应用层按 pool 路由
- **Redis**：会话（TTL 30min）、热门内容缓存（TTL 5min）、限流计数、OpenSearch 降级缓存
- **OpenSearch**：目的地/景点/线路/酒店/航班索引，多语种分字段分词
- **Kafka**：审计日志、支付事件、订单状态事件（异步解耦）
- **接口约定**：统一 JSON（`code/message/data`），强制 `X-Api-Version`，错误码体系映射 HTTP 状态码

---

## 五、核心业务域设计

### 5.1 旅游商品域

所有商品域统一抽象「**商品 → 日历价格 → 库存 → 订单**」：

| 商品域 | 核心实体 | 关键属性 | 状态 |
| :--- | :--- | :--- | :--- |
| 目的地 | destination | 多语种名称/描述、region_id、经纬度、封面图 | **已实现**（基础表） |
| 景点景区 | attraction | 所属目的地、票价、开放时间、热度 | 规划中（Phase 2-3） |
| 旅游线路 | line | 行程天数、出发日期、价格、成团人数、行程安排 | 规划中（Phase 3-4） |
| 机票 | flight | 航段、舱位、价格、余票、行李额 | 规划中（Phase 4-5） |
| 酒店 | hotel / room | 房型、入住/离店、房价、早餐 | 规划中（Phase 4-5） |

### 5.2 预订 — 订单 — 支付链路

```
选品/选日期（商品 + 日历价格）→ 锁库存（Redis 预占 + MySQL 扣减）
→ 创建订单（order-service，状态机：待支付/已支付/已确认/已取消）
→ 发起支付（payment-service，渠道下单）
→ 支付回调（验签 → 幂等更新订单状态 → 确认库存 → Kafka 事件）
→ 订单中心（查询/取消/改期）
```

- 订单状态机（规划）：`0 待支付 → 1 已支付 → 2 已确认 → 3 已完成`，任意未支付状态可 `4 已取消`。注意：现有 `travel_bookings.status`（0 待确认/1 已确认/2 已完成/3 已取消）与 `travel_orders.status`（0 待支付/1 已支付/2 已取消）语义均与新编号冲突，落地时需 status 重映射迁移
- 金额一律以**分**存储（`amount_cents`），涉及金额的计算统一由后端完成
- 支付回调必须**验签 + 幂等**（按支付流水号去重），防止重复入账
- 取消订单需处理库存释放与退款（Phase 4 规划）

### 5.3 多语言 i18n 设计

- **后端**：内容表按语种分列（`name_en/name_zh/name_ja`…）或 JSON 字段（现有 `travel_destinations.description JSON`），OpenSearch 索引按语言分字段 + 语言族分词器（屈折语 Snowball / 日语 kuromoji / 阿拉伯语 ICU + RTL）
- **用户语言**：注册时记录 `lang`；已登录取用户档案语言，未登录取请求头/系统默认
- **Flutter 端**：`flutter_localizations` + `intl`，13 个 ARB 文件已就绪，RTL 语种自动切换方向
- **HarmonyOS 端**：ArkUI 系统 i18n，resource 按语种分包，与 Flutter 共享后端 API

---

## 六、数据库规划

> 现有 5 表（`travel_users/destinations/bookings/orders/reviews`）为基线；以下为规划新增与扩展（全部 Phase 2-5 逐步落地，前缀 `travel_`）

| 表 | 关键字段 | 阶段 |
| :--- | :--- | :--- |
| `travel_destinations`（扩展） | +cover_url、status（上/下架）、sort_order（lat/lng 已有） | Phase 2 |
| `travel_attractions` | destination_id、name_*、price_cents、open_hours、rating_avg、cover_url | Phase 2 |
| `travel_lines` | title_*、destination_id、days、departure_date、price_cents、max_pax、itinerary JSON、status | Phase 3 |
| `travel_flights` | airline、flight_no、from_code/to_code、depart_at/arrive_at、cabin、price_cents、seats_left | Phase 4 |
| `travel_hotels` | name_*、city_code、star、lat/lng、cover_url、status | Phase 4 |
| `travel_hotel_rooms` | hotel_id、room_type_*、price_cents、breakfast、inventory | Phase 4 |
| `travel_orders`（扩展） | +order_type（1 线路/2 机票/3 酒店）、product_id、product_snapshot JSON、expire_at | Phase 3 |
| `travel_payments` | order_id、channel、amount_cents、status、txn_no、paid_at | Phase 4 |
| `travel_admins` | username、password_hash、role、status | Phase 3 |
| `travel_searches` | keyword、lang、results、created_at（搜索日志/热词） | Phase 3 |

通用约定：全表 `utf8mb4`、时间字段 DATETIME + `CURRENT_TIMESTAMP`、金额用 BIGINT（分）、多语种文本分列或 JSON、关键查询字段建索引。

---

## 七、管理端规划（Flutter Web）

| 模块 | 功能 | 阶段 |
| :--- | :--- | :--- |
| 登录 | 管理员账号登录（对接 admin-service JWT）、退出 | Phase 3 |
| 框架 | 侧边栏 + 内容区布局、路由、权限菜单 | Phase 3 |
| 目的地/景区管理 | 列表、编辑（多语种内容表单）、上下架、排序、封面图上传 | Phase 3 |
| 线路管理 | 线路 CRUD、行程编排（多日 JSON）、价格与成团人数、出发日历 | Phase 3-4 |
| 机票管理 | 航班 CRUD、舱位与价格维护、余票调整 | Phase 4-5 |
| 酒店管理 | 酒店/房型 CRUD、房价日历、库存 | Phase 4-5 |
| 订单管理 | 订单列表/筛选/详情、改退操作、支付记录查看 | Phase 4 |
| 用户管理 | 用户列表、禁用/启用 | Phase 4 |
| 数据看板 | 订单量/GMV/转化率、Top 目的地与线路排行、图表 | Phase 5 |

---

## 八、客户端规划（Flutter / HarmonyOS）

| 页面 | 功能 | 阶段 |
| :--- | :--- | :--- |
| 首页 | 热门目的地、景点推荐、线路/机票/酒店入口（替换硬编码数据，接 search-service） | Phase 2-3 |
| 搜索 | 关键词 + 目的地/日期/价格多条件筛选，多语种检索 | Phase 3 |
| 目的地/景点详情 | 图文详情、票价、开放时间、评价列表 | Phase 2-3 |
| 线路详情 | 行程安排、出发日历、价格与余位、报名 | Phase 3-4 |
| 机票/酒店搜索 | 航班比价、酒店房型预订（Flutter 先行，鸿蒙跟随） | Phase 4-5 |
| 预订流程 | 选日期 → 确认订单 → 支付（接入 payment-service） | Phase 4 |
| 订单中心 | 订单列表/详情/取消/支付引导，状态实时刷新 | Phase 3-4 |
| 我的 | 资料编辑、语言切换、地址簿、客服入口 | Phase 2-3 |
| 鸿蒙端 | 与 Flutter 同构页面逐步补齐（Index/目的地已实现） | 持续 |

---

## 九、阶段规划

| 阶段 | 周期 | 任务重点 | 状态 |
| :--- | :--- | :--- | :--- |
| **Phase 1** | 已完成 | e-cat 基础服务 + MySQL/Redis 集成，缓存链路、限流/安全中间件（读写分离为规划中） | ✅ **已完成** |
| **Phase 2** | 4-6 周 | 景点景区业务域（attraction 表 + 接口 + 客户端详情页）；管理端登录与目的地/景区管理；客户端首页接真实数据 | 部分完成（i18n/鸿蒙骨架），业务域**规划中** |
| **Phase 3** | 4-6 周 | search-service 多语种检索 + 线路业务域（line 表 + 接口）；订单服务（状态机）；管理端线路管理；客户端搜索/线路预订/订单中心 | 规划中 |
| **Phase 4** | 6-8 周 | 机票/酒店业务域；payment-service 支付（微信/支付宝回调、幂等）；管理端订单/用户管理；客户端预订支付闭环；全链路联调 + 压测 + CDN | 规划中 |
| **Phase 5** | 持续 | 评价体系、数据看板、热词推荐、多端功能对齐（鸿蒙）、运营工具完善 | 规划中 |

---

## 十、关键决策与风险

**关键决策**

1. **商品域统一抽象**：目的地/线路/机票/酒店共享「商品 → 日历价格 → 库存 → 订单」模型与订单状态机，避免各域各造一套订单系统。
2. **订单快照**：下单时把商品信息（名称/价格/行程）快照进订单，商品后续改价改文案不影响历史订单与售后。
3. **金额整数分存储**：全链路 `amount_cents`（BIGINT），杜绝浮点误差；金额计算只由后端完成。
4. **支付幂等**：回调按支付流水号幂等处理，先验签再改状态，配合 Kafka 事件解耦订单/库存/通知。
5. **多语言文本进库分列**：内容表按语种分列（或 JSON），索引按语言分字段 + 语言族分词器；用户 `lang` 与 `Accept-Language` 结合。
6. **统一中间件链**：所有服务挂同一套 Tracing → CircuitBreaker → Security → RateLimit → Auth，安全规则随 e-cat 框架升级（cargo 依赖管理）。
7. **API-first**：新服务接口先写 .proto（ecat-cli proto 生成），错误码编译期映射 HTTP 状态码。
8. **客户端先 Flutter 后鸿蒙**：新功能 Flutter 先行验证，鸿蒙端按页面同构补齐，控制双端维护成本。

**风险与对策**

| 风险 | 对策 |
| :--- | :--- |
| 业务域多（5 域）易摊薄资源 | 按 Phase 2-5 顺序落地，优先景点景区与线路，机票/酒店最后 |
| 库存超卖（线路成团/机票余票/酒店房量） | 下单预占 + 支付确认 + 超时释放（expire_at），Redis 计数 + MySQL 乐观锁 |
| 支付渠道对接与对账复杂 | 抽象 payment-service 统一渠道接口，先接入 1-2 个渠道（微信/支付宝），回调验签 + 幂等 + 每日对账 |
| 多语种内容维护成本高 | 内容多语种进库，管理端分语种编辑；未翻译语种回退英文 |
| 现有 `travel_bookings`/`travel_orders` 结构偏简单 | 以扩展列 + 快照 JSON 方式演进，避免一次性大重构 |
| 安全规则库迭代 | SecurityLayer 规则随 e-cat 升级纳入 Dependabot，审计日志入 Kafka 供查询 |
