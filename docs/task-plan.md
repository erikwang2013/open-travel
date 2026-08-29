# Open Travel 项目任务计划

> 基于 [travel-project-planning-v2.md](travel-project-planning-v2.md)（功能全景、阶段划分、数据库规划全部沿用）与 [api.md](api.md)（现有接口）编制。
> 已实现功能标「已完成」，规划中任务标「待开始」/「进行中」。

## 一、任务分解说明

- **编号规则**：`P{Phase}-{序号}`（如 P3-05）；状态字段：待开始 / 进行中 / 已完成。
- **Phase 划分沿用 v2 规划**：Phase 1 基础巩固（已实现项为基线，补全测试与部署）；Phase 2 内容域（景点景区 + 管理端登录/内容管理 + 客户端接真实数据，含 OpenSearch 集群与内容索引准备）；Phase 3 搜索与交易（search-service 检索、线路商品域、订单服务、客户端搜索/线路/订单中心）；Phase 4 机票/酒店与支付（flight/hotel/payment 服务、管理端订单/用户管理、预订支付闭环、联调压测 CDN）；Phase 5 优化与上线（评价、看板、i18n 完善、多端对齐）。
- 依赖列引用任务编号（如 `P2-01`）；负责端：后端 / 客户端 / 管理端 / 全端。
- 管理端登录归属以 v2 §9 阶段规划为准（Phase 2，P2-06/07）；v2 §7 标注 Phase 3 为旧标注。
- 基线（v2 已实现，不在任务表重复）：user-service 注册/登录/资料、booking-service dates 接口、5 张 MySQL 表、Redis 缓存链路、统一中间件链、Nginx 网关、Flutter 4 页面 + 13 ARB、鸿蒙 4 页面、admin Flutter Web 骨架。

## 二、Phase 1 基础巩固

| 任务编号 | 任务 | 所属模块 | 依赖 | 验收标准 | 负责端 | 状态 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| P1-01 | 基础服务与缓存链路（user/booking 接口、JWT、限流、Redis→MySQL→兜底） | 基础设施 | — | api.md 全部端点 curl 可通 | 后端 | 已完成 |
| P1-02 | MySQL 5 表 + 目的地种子数据 + Nginx 网关分流 | 基础设施 | — | schema.sql 可全量初始化，网关 8082 路由正常 | 后端 | 已完成 |
| P1-03 | Flutter 4 页面（首页/导航壳/预订/我的）+ 13 语种 ARB + 鸿蒙 4 页面骨架 | 客户端 | — | 双端可构建，语言切换生效 | 客户端 | 已完成 |
| P1-04 | user/booking 接口集成测试补全 | 基础设施 | P1-01 | 注册/登录/资料/dates 各含正常+异常用例，`cargo test` 通过 | 后端 | 已完成 |
| P1-05 | MySQL 读写分离：主库写/从库读按 pool 路由 | 基础设施 | P1-02 | 配置双数据源后读请求走从库，/ready 报告双源状态 | 后端 | 已完成 |
| P1-06 | 审计日志入 Kafka（登录/下单/支付等事件） | 基础设施 | P1-05 | 关键操作发布审计事件，消费端可查询 | 后端 | 已完成 |
| P1-07 | 部署脚本与 CI：docker-compose 一键启动 + cargo check/flutter analyze/test 流水线 | 基础设施 | P1-04 | 新机器按 README 可一键起服务；CI 绿 | 后端 | 已完成 |

## 三、Phase 2 搜索与内容（景点景区业务域 + 管理端基础）

| 任务编号 | 任务 | 所属模块 | 依赖 | 验收标准 | 负责端 | 状态 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| P2-01 | `travel_destinations` 扩展（+cover_url/status/sort_order，lat/lng 已存在）+ 新增 `travel_attractions` 表（destination_id/name_*/price_cents/open_hours/rating_avg/cover_url） | 数据层 | P1-02 | schema.sql 增量 DDL 可执行，索引齐全 | 后端 | 已完成 |
| P2-02 | booking-service `GET /api/booking/attractions?destination_id=N` 列表接口（多语种字段按 lang 返回） | 内容服务 | P2-01 | 按目的地返回景区列表，Redis 缓存 TTL 5min，无数据兜底 | 后端 | 已完成 |
| P2-03 | booking-service `GET /api/booking/attractions/:id` 详情接口（图文/票价/开放时间；评价位预留，P5-01 落地后接入） | 内容服务 | P2-02 | 详情字段完整，404 处理，未翻译语种回退英文 | 后端 | 已完成 |
| P2-04 | 景点多语种种子数据（每目的地 ≥3 条，覆盖 12+ 语种名称） | 数据层 | P2-01 | 种子脚本幂等可重复执行 | 后端 | 已完成 |
| P2-05 | OpenSearch 集群搭建 + destinations/attractions 索引结构设计（语言分字段 + 屈折语/kuromoji/ICU 分词器） | 搜索基础设施 | P2-01 | 集群可启动，索引 mapping 含全语种字段 | 后端 | 已完成 |
| P2-06 | admin-service 创建：`POST /api/admin/login` + `travel_admins` 表 + JWT 鉴权 + 统一中间件链 | 管理后端 | P1-01 | 管理员登录签发 JWT，admin 接口鉴权生效 | 后端 | 已完成 |
| P2-07 | 管理端 Flutter Web：登录页 + 框架（侧边栏/内容区/路由/权限菜单） | 管理端 | P2-06 | 登录后可进入框架，未登录跳登录页 | 管理端 | 已完成 |
| P2-08 | 管理端目的地管理：列表/多语种表单编辑/上下架/排序/封面图上传 | 管理端 | P2-07, P2-01 | 目的地 CRUD 全流程可用，列表分页 | 管理端 | 已完成 |
| P2-09 | 管理端景区管理：CRUD + 多语种表单 + 票价/开放时间 | 管理端 | P2-08, P2-02 | 景区增删改查生效，列表即查即得 | 管理端 | 已完成 |
| P2-10 | 客户端首页接真实数据（替换硬编码目的地，调 dates + attractions 接口；P3-02 落地后首页检索切换 search-service） | 客户端 | P2-02 | 首页展示真实目的地与景点推荐，失败有兜底 | 客户端 | 已完成 |
| P2-11 | 客户端目的地/景点详情页（图文、票价、开放时间、语言切换） | 客户端 | P2-03, P2-10 | 详情页字段完整展示，13 语种文案齐全 | 客户端 | 已完成 |
| P2-12 | 鸿蒙端景点列表/详情页同构补齐 | 客户端 | P2-11 | 鸿蒙端可浏览景点列表与详情 | 客户端 | 已完成 |
| P2-13 | 客户端「我的」页完善（资料编辑、语言切换、地址簿、客服入口） | 客户端 | P1-03, P2-14 | 可编辑 lang 并即时切换界面语言，地址簿/客服入口可用 | 客户端 | 已完成 |
| P2-14 | user-service 资料更新接口：`PUT /api/user/profile`（编辑 lang/昵称），JWT 鉴权 | 用户服务 | P1-01 | 登录后可更新 lang，接口鉴权生效 | 后端 | 已完成 |

## 四、Phase 3 搜索与交易（search-service + 线路 + 订单）

| 任务编号 | 任务 | 所属模块 | 依赖 | 验收标准 | 负责端 | 状态 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| P3-01 | search-service 创建 + `travel_searches` 表 + 内容写入 OpenSearch 索引（同步任务） | 搜索服务 | P2-05 | 目的地/景点数据进索引，增量同步可用 | 后端 | 已完成 |
| P3-02 | search-service `GET /api/search?q=&destination_id=&lang=&price_min=&price_max=&page=` 多条件检索接口 + 热词日志 | 搜索服务 | P3-01 | 关键词/目的地/价格过滤生效，结果按语言分词匹配 | 后端 | 已完成 |
| P3-03 | 客户端搜索页（关键词 + 目的地/日期/价格多条件筛选） | 客户端 | P3-02 | 搜索交互闭环，空结果有提示 | 客户端 | 已完成 |
| P3-04 | `travel_lines` 表（title_*/destination_id/days/departure_date/price_cents/max_pax/itinerary JSON/status）+ 种子数据 | 数据层 | P2-01 | 增量 DDL 可执行，种子 ≥5 条线路 | 后端 | 已完成 |
| P3-05 | line-service：`GET /api/lines?destination_id=` 列表 + `GET /api/lines/:id` 详情（行程/价格/成团/余位） | 线路服务 | P3-04 | 列表/详情接口 curl 验证通过，404/参数校验齐全 | 后端 | 已完成 |
| P3-06 | line-service 出发日历与余位：`GET /api/lines/:id/dates`（日期+价格+余位，Redis 缓存） | 线路服务 | P3-05 | 日历数据准确，余位与订单联动（预占扣减） | 后端 | 已完成 |
| P3-07 | 订单状态机统一落地：`travel_orders` 扩展（order_type 1线路/2机票/3酒店、product_id、product_snapshot、expire_at）+ `travel_bookings` status 重映射迁移 | 数据层 | P1-02 | 迁移脚本可逆，旧数据 status 正确映射新编号 | 后端 | 已完成 |
| P3-08 | order-service：`POST /api/orders` 下单（商品+日历价+Redis 预占库存+快照）+ 订单状态机（待支付→已支付→已确认→已完成/已取消） | 订单服务 | P3-07, P3-06 | 下单成功扣余位，重复下单防超卖，超时未付释放 | 后端 | 已完成 |
| P3-09 | order-service：`GET /api/orders` 列表 + `GET /api/orders/:id` 详情 + `POST /api/orders/:id/cancel` 取消（释放库存） | 订单服务 | P3-08 | 状态流转正确，取消后余位回补 | 后端 | 已完成 |
| P3-10 | 管理端线路管理：CRUD + 行程编排（多日 JSON）+ 价格/成团人数 + 出发日历维护 | 管理端 | P2-08, P3-04 | 线路全流程管理可用，出发日历可视化编辑 | 管理端 | 已完成 |
| P3-11 | 客户端线路列表/详情页（行程安排、出发日历、价格与余位） | 客户端 | P3-05, P3-06 | 详情字段完整，余位实时展示 | 客户端 | 已完成 |
| P3-12 | 客户端线路预订流程（选出发日期 → 确认订单 → 提交） | 客户端 | P3-08, P3-11 | 下单成功跳订单详情，库存不足有提示 | 客户端 | 已完成 |
| P3-13 | 客户端订单中心（列表/详情/取消/支付引导，状态实时刷新） | 客户端 | P3-09 | 订单列表详情正确，取消后状态更新 | 客户端 | 已完成 |
| P3-14 | 鸿蒙端搜索/线路/订单页面同构补齐 | 客户端 | P3-13 | 鸿蒙端三页面可用 | 客户端 | 已完成 |

## 五、Phase 4 机票/酒店与支付

| 任务编号 | 任务 | 所属模块 | 依赖 | 验收标准 | 负责端 | 状态 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| P4-01 | `travel_flights` 表（airline/flight_no/from_code/to_code/depart_at/arrive_at/cabin/price_cents/seats_left）+ 种子数据 | 数据层 | P2-01 | 增量 DDL 可执行，种子覆盖多条航线 | 后端 | 已完成 |
| P4-02 | flight-service：`GET /api/flights/search?from=&to=&date=&cabin=` 航班查询/比价 + `GET /api/flights/:id` 详情 | 机票服务 | P4-01 | 查询支持日期/舱位过滤，结果按价格排序 | 后端 | 已完成 |
| P4-03 | `travel_hotels`（name_*/city_code/star/lat/lng/cover_url/status）+ `travel_hotel_rooms`（room_type_*/price_cents/breakfast/inventory）表 + 种子 | 数据层 | P2-01 | 增量 DDL 可执行，种子 ≥5 家酒店含房型 | 后端 | 已完成 |
| P4-04 | hotel-service：`GET /api/hotels/search?city=&check_in=&check_out=` 搜索 + `GET /api/hotels/:id` 详情（房型/房价日历） | 酒店服务 | P4-03 | 搜索/详情接口 curl 验证通过，房价日历准确 | 后端 | 已完成 |
| P4-05 | `travel_payments` 表（order_id/channel/amount_cents/status/txn_no/paid_at） | 数据层 | P3-08 | 增量 DDL 可执行 | 后端 | 已完成 |
| P4-06 | payment-service：`POST /api/payments` 发起支付（渠道下单）+ 回调接口（验签 + 按流水号幂等）+ 每日对账脚本 | 支付服务 | P4-05 | 回调重复投递不重复入账，验签失败拒绝 | 后端 | 已完成 |
| P4-07 | 订单支付闭环：支付成功 → 订单确认 → 库存确认 → Kafka 事件（订单/通知） | 订单服务 | P4-06, P3-08 | 支付后订单状态自动流转，事件消费方收到通知 | 后端 | 已完成 |
| P4-08 | 管理端订单管理（列表/筛选/详情/改退操作/支付记录查看） | 管理端 | P3-09, P4-06 | 订单全流程可视，改退操作生效 | 管理端 | 已完成 |
| P4-09 | 管理端用户管理（列表、禁用/启用） | 管理端 | P2-06, P4-14 | 禁用后该用户接口 403 | 管理端 | 已完成 |
| P4-10 | 管理端机票/酒店管理：航班 CRUD + 舱位价格/余票调整；酒店/房型 CRUD + 房价日历 + 库存 | 管理端 | P4-02, P4-04 | 双域管理全流程可用 | 管理端 | 已完成 |
| P4-11 | 客户端机票/酒店搜索预订页（Flutter 先行：航班比价、酒店房型选择） | 客户端 | P4-02, P4-04 | 搜索→选品→下单链路可走通 | 客户端 | 已完成 |
| P4-12 | 客户端预订支付流程（接入 payment-service，支付引导/回调刷新）+ 取消退款 | 客户端 | P4-07, P3-12 | 下单→支付→订单确认全链路走通 | 客户端 | 已完成 |
| P4-13 | 全链路联调（下单→支付→回调→状态刷新）+ 压测 + CDN 接入 | 全端 | P4-12 | 联调报告全绿，压测 QPS 达标，CDN 生效 | 全端 | 进行中 |
| P4-14 | `travel_users` 扩展 status 字段（0 正常/1 禁用）+ user-service 管理接口（用户列表、`PATCH /api/admin/users/:id/status` 禁用/启用） | 用户服务 | P1-01 | 禁用用户 JWT 请求返回 403，列表分页 | 后端 | 已完成 |
| P4-15 | 支付渠道体系：国际卡（Stripe）+ 本地支付（按用户 lang/国家路由，如微信/支付宝、PayPay、KakaoPay 等）+ USDT 等加密渠道（NOWPayments/Binance Pay 等）；`travel_payment_channels` 表（channel_code/name/type/enabled/priority/languages 或 countries/merchant_config）+ 渠道注册表抽象，payment-service 按用户语言优先路由本国渠道 | 支付服务 | P4-06 | 各渠道开关可控，按 lang 返回可用渠道列表，渠道层可插拔 | 后端 | 已完成 |
| P4-16 | 管理端支付管理：渠道开关（按语言/国家控制前端展示）+ 流水账单（travel_payments 列表/筛选：渠道、状态、时间、金额） | 管理端 | P4-15, P4-08 | 渠道开关即时生效，账单筛选分页可用 | 管理端 | 已完成 |
| P4-17 | 客户端支付页：按用户语言展示可用渠道（本国优先）并跳转渠道收银台/扫码，支付结果轮询与回调刷新 | 客户端 | P4-15, P4-12 | 多语言下支付渠道列表正确，支付链路走通 | 客户端 | 已完成 |

## 六、Phase 5 优化与上线

| 任务编号 | 任务 | 所属模块 | 依赖 | 验收标准 | 负责端 | 状态 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| P5-01 | 评价体系：review-service（或并入 booking）`POST /api/reviews` + 列表接口，接入 `travel_reviews` 表，详情页展示评分 | 评价服务 | P2-03 | 评价提交/列表可用，评分聚合准确 | 后端 | 已完成 |
| P5-02 | 管理端数据看板（订单量/GMV/转化率、Top 目的地与线路排行、图表） | 管理端 | P4-08 | 看板数据与订单库一致，图表渲染正常 | 管理端 | 已完成 |
| P5-03 | 热词推荐（`travel_searches` 聚合，搜索页热门关键词） | 搜索服务 | P3-02 | 热词按周期聚合展示 | 后端 | 已完成 |
| P5-04 | i18n 完善：13 ARB 全页面适配、RTL 语种方向、后端内容未翻译回退英文 | 全端 | P2-11, P3-13, P4-12 | 全页面 13 语种无缺失 key，AR 界面 RTL 正确 | 全端 | 已完成 |
| P5-05 | 鸿蒙端全页面同构补齐（我的/订单/支付/搜索） | 客户端 | P3-14, P4-12 | 鸿蒙端功能与 Flutter 对齐 | 客户端 | 已完成 |
| P5-06 | 性能优化（索引/缓存/慢查询治理）+ 压测报告 | 后端 | P4-13 | 压测报告达标（P99 < 500ms），慢查询清零 | 后端 | 已完成 |
| P5-07 | 上线部署：HTTPS/TLS、生产环境变量审计（JWT_SECRET 等）、监控告警、灰度 | 基础设施 | P5-06 | 生产环境安全基线检查通过，监控告警可用 | 后端 | 已完成 |

---

**任务统计**：Phase 1 共 7 项（全部已完成），Phase 2 共 14 项（全部已完成），Phase 3 共 14 项（全部已完成），Phase 4 共 17 项，Phase 5 共 7 项（全部已完成），合计 **59 项**（已完成 58 项；P4-13 仅余 CDN 接入，需云凭据）。
