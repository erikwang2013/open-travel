<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Open Travel API 参考

本页汇总 Open Travel 后端（user-service / booking-service / admin-service）的 HTTP 接口。服务基于 e-cat 框架构建，业务路由由各服务注册，经 Nginx 网关统一暴露。

## 访问入口

| 入口 | 地址 | 说明 |
|------|------|------|
| Nginx 网关 | `http://localhost:8082` | 宿主 8082 → 容器 80，按前缀分流 |
| user-service 直连 | `http://localhost:8001` | 用户服务 |
| booking-service 直连 | `http://localhost:8002` | 目的地服务 |
| admin-service 直连 | `http://localhost:8003` | 管理服务 |

网关分流规则（`config/nginx.conf`）：`/api/v1/user/*` → user-service，`/api/v1/booking/*` → booking-service，`/api/v1/admin/*` → admin-service；`/health`、`/ready` → user-service。

## 通用约定

### 响应格式

所有业务接口返回统一 JSON 结构：

```json
{ "code": 0, "message": "ok", "data": { } }
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `code` | number | 业务码，`0` 表示成功，非 0 为错误码 |
| `message` | string | 提示信息 |
| `data` | object \| array \| null | 业务数据，失败时为 `null` |

### API 版本

API 版本在 URL 前缀：`/api/v1/...`（版本即路由）。当前仅一个版本 `v1`；无版本前缀或未知版本（如 `/api/v2/`）的路径由 nginx 返回 `404`，无 header 校验。

```bash
curl http://localhost:8082/api/v1/user/profile
```

### 鉴权（JWT）

user-service 的 `/api/v1/user/profile` 需在请求头携带 JWT：

```
Authorization: Bearer <JWT>
```

- 密钥来自 `JWT_SECRET` 环境变量（≥32 字节）；未配置时退回开发占位密钥并告警（生产必须配置）
- 无 token / token 无效时返回 `401`
- booking-service 的接口为公开接口，无鉴权
- admin-service：`POST /api/v1/admin/login` 公开；其余管理接口需携带 `Authorization: Bearer <admin JWT>`（登录签发，claims 含 `role=admin`，24 小时有效）。token 缺失 / 无效返回 `401`，非 admin role 返回 `403`

### 限流

- 每个服务独立限流：**100 请求 / 60 秒**，Redis 分布式固定窗口
- 未认证请求也计入限流（防暴力请求耗尽资源）
- 超限返回 `429`；限流 key 按客户端地址统计

### 错误码

| HTTP 状态码 | 场景 |
|------|------|
| 200 | 成功 |
| 400 | 请求参数错误（`code` 非 0，`message` 描述原因） |
| 401 | 未认证 / JWT 无效 / 登录凭据错误（统一返回，防枚举） |
| 403 | 请求被安全中间件拦截（SQL 注入 / XSS / SSRF 等攻击模式）；admin 接口非 admin role（`admin role required`） |
| 404 | 用户 / 目的地 / 景区不存在（各服务对应返回） |
| 409 | 邮箱已注册；删除仍有关联景区的目的地 |
| 429 | 超过限流阈值 |
| 500 | 服务内部错误 |
| 503 | 数据库不可用 |

错误响应示例（限流）：

```json
{ "code": 429001, "message": "rate limit exceeded", "data": null }
```

## 端点

### 健康检查

**`GET /health`**（两服务均提供）— 存活检查

```bash
curl http://localhost:8082/health
```

响应：`OK`（纯文本）。

**`GET /ready`**（两服务均提供）— 就绪检查，报告数据源（MySQL / Redis）降级状态，不阻塞服务启动

```bash
curl http://localhost:8082/ready
```

```json
{ "code": 0, "message": "ready", "data": true }
```

### 用户服务

#### `POST /api/v1/user/register` — 用户注册

公开接口。请求体 JSON：

```json
{ "email": "a@b.com", "password": "secret1", "lang": "en" }
```

| 字段 | 必填 | 说明 |
|------|------|------|
| `email` | 是 | 邮箱格式（需含 `@`），非法返回 400 |
| `password` | 是 | 至少 6 位，bcrypt（cost 12）哈希存储 |
| `lang` | 否 | 语言偏好，默认 `en` |

- `400`：邮箱格式无效 / 密码少于 6 位
- `409`：邮箱已注册（`email already registered`）
- `503`：数据库不可用

```bash
curl -H "Content-Type: application/json" -X POST \
  http://localhost:8082/api/v1/user/register -d '{"email":"a@b.com","password":"secret1"}'
```

```json
{ "code": 0, "message": "ok", "data": { "user_id": 1, "email": "a@b.com", "lang": "en" } }
```

#### `POST /api/v1/user/login` — 登录

公开接口。请求体 `{ "email": "...", "password": "..." }`。

- **统一 `401`**（`invalid credentials`）：不区分「邮箱不存在」与「密码错误」，防账号枚举
- 成功返回 JWT（24 小时有效期）与用户信息
- `503`：数据库不可用

```bash
curl -H "Content-Type: application/json" -X POST \
  http://localhost:8082/api/v1/user/login -d '{"email":"a@b.com","password":"secret1"}'
```

```json
{ "code": 0, "message": "ok", "data": { "token": "<JWT>", "user_id": 1, "email": "a@b.com" } }
```

#### `GET /api/v1/user/profile` — 查询用户资料

鉴权：**需要 JWT**（`Authorization: Bearer <JWT>`，由登录接口签发）。

- `401`：无 token / token 无效
- `400`：token 中 subject 非法
- `404`：用户不存在
- `503`：数据库不可用

```bash
curl -H "Authorization: Bearer <JWT>" http://localhost:8082/api/v1/user/profile
```

```json
{ "code": 0, "message": "ok", "data": { "user_id": 1, "email": "a@b.com", "lang": "en" } }
```

#### `PUT /api/v1/user/profile` — 更新用户资料

鉴权：**需要 JWT**（与 GET 共用同一路径）。请求体 JSON：

```json
{ "nickname": "alice", "lang": "zh" }
```

| 字段 | 必填 | 说明 |
|------|------|------|
| `nickname` | 否 | 昵称，最多 100 字符；省略保持原值 |
| `lang` | 否 | 界面语言，支持 `en/zh/ja/ko/ru/de/fr/es/pt/hi/ar/bn/id` 13 种；省略保持原值 |

- `400`：body 非法 / nickname 超 100 字符 / lang 不支持
- `401`：无 token / token 无效
- `404`：用户不存在
- `503`：数据库不可用

```bash
curl -H "Content-Type: application/json" -H "Authorization: Bearer <JWT>" -X PUT \
  http://localhost:8082/api/v1/user/profile -d '{"nickname":"alice","lang":"zh"}'
```

```json
{ "code": 0, "message": "ok", "data": { "id": 1, "email": "a@b.com", "nickname": "alice", "lang": "zh" } }
```

### 目的地服务

#### `GET /api/v1/booking/dates?region_id=N` — 热门目的地日期

公开接口，无鉴权。查询 `region_id` 对应的热门目的地列表（仅 `status=1`，按 `sort_order ASC, id ASC` 排序），走 Redis 缓存（`hot_destinations:{region_id}`，TTL 300s）→ MySQL 回源（`travel_destinations` 表）→ 占位数据兜底，保证无数据源环境可响应。

```bash
curl "http://localhost:8082/api/v1/booking/dates?region_id=1"
```

```json
{
  "code": 0,
  "message": "ok",
  "data": [ { "id": 1, "region_id": 1, "name_en": "Tokyo", "name_zh": "东京" } ]
}
```

`region_id` 缺省时按 `0` 处理。

#### `GET /api/v1/booking/attractions?destination_id=N&lang=xx` — 景区列表

公开接口，无鉴权。`destination_id` 必填，返回该目的地的上架景区（`status=1`），按 `id` 升序。走 Redis 缓存（`travel:attractions:{destination_id}:{lang}`，TTL 300s）→ MySQL 回源（从库优先，失败回退主库）；无数据返回空数组。

- `400`：缺少 `destination_id`
- `lang` 缺省按 `en` 处理；`name` 优先取 `name_{lang}` 列，为空回退 `name_en`

```bash
curl "http://localhost:8082/api/v1/booking/attractions?destination_id=1&lang=zh"
```

```json
{
  "code": 0,
  "message": "ok",
  "data": [ { "id": 1, "destination_id": 1, "name": "东京塔", "price_cents": 5000, "open_hours": "09:00-22:00", "rating_avg": 4.5, "cover_url": "https://..." } ]
}
```

#### `GET /api/v1/booking/attractions/{id}?lang=xx` — 景区详情

公开接口，无鉴权。仅返回上架景区（`status=1`）。`description` 为 JSON 对象（键为语言代码），按 `lang` 取键，缺失或为空回退 `en`。

- `404`：景区不存在或已下架
- `reviews` 返回该景区最近 20 条真实评价（按 id 倒序），`rating_avg` 为读时 `AVG(rating)` 聚合，无评价时保留表内字段

```bash
curl "http://localhost:8082/api/v1/booking/attractions/1?lang=zh"
```

```json
{
  "code": 0,
  "message": "ok",
  "data": { "id": 1, "destination_id": 1, "name": "东京塔", "description": "东京地标", "price_cents": 5000, "open_hours": "09:00-22:00", "rating_avg": 4.5, "cover_url": "https://...", "reviews": [ { "id": 1, "attraction_id": 1, "user_id": 2, "rating": 5, "comment": "非常棒的体验", "nickname": "alice", "created_at": "2026-08-29T10:00:00Z" } ] }
}
```

#### `POST /api/v1/reviews` — 提交评价

鉴权：**需要 JWT**（`Authorization: Bearer <JWT>`，由登录接口签发）。请求体 JSON：

```json
{ "attraction_id": 1, "rating": 5, "comment": "非常棒的体验" }
```

| 字段 | 必填 | 说明 |
|------|------|------|
| `attraction_id` | 是 | 目标景区 ID，景区不存在或已下架返回 404 |
| `rating` | 是 | 评分，1-5 整数，越界返回 400 |
| `comment` | 否 | 评价内容，最多 500 字符，超长返回 400 |

- `400`：`attraction_id` 缺失 / `rating` 越界 / `comment` 超长
- `401`：无 token / token 无效
- `404`：景区不存在或已下架
- 成功返回 `201`，`data` 为创建后的评价对象（含 `id` / `created_at`）

```bash
curl -H "Content-Type: application/json" -H "Authorization: Bearer <JWT>" -X POST \
  http://localhost:8082/api/v1/reviews -d '{"attraction_id":1,"rating":5,"comment":"非常棒的体验"}'
```

#### `GET /api/v1/reviews?attraction_id=N&page=&page_size=` — 评价列表

公开接口，无鉴权。`attraction_id` 必填（缺失返回 400），按 `id` 倒序分页（`page` 默认 1，`page_size` 默认 10、上限 50）。

响应 `data` 为裸数组：`[ { "id", "attraction_id", "user_id", "rating", "comment", "nickname", "created_at" } ]`。

```bash
curl "http://localhost:8082/api/v1/reviews?attraction_id=1&page=1&page_size=10"
```

### 管理服务

管理端接口全部需要 **admin JWT**（`POST /api/v1/admin/login` 签发，claims 含 `role=admin`）：

- token 缺失 / 无效 → `401`；非 admin role → `403`（`admin role required`）
- 限流独立统计：100 请求 / 60 秒

#### `POST /api/v1/admin/login` — 管理员登录

公开接口。请求体 `{ "email": "...", "password": "..." }`，仅 `status=1`（启用）的管理员可登录。

- **统一 `401`**（`invalid credentials`）：不区分「邮箱不存在」与「密码错误」，防账号枚举
- 成功返回 admin JWT（24 小时有效期）
- `503`：数据库不可用

```bash
curl -H "Content-Type: application/json" -X POST \
  http://localhost:8082/api/v1/admin/login -d '{"email":"admin@travel.local","password":"Admin@123"}'
```

```json
{ "code": 0, "message": "ok", "data": { "token": "<JWT>" } }
```

#### `GET /api/v1/admin/destinations` — 目的地列表

鉴权：需要 admin JWT。

| 查询参数 | 必填 | 说明 |
|------|------|------|
| `page` | 否 | 页码，默认 `1` |
| `page_size` | 否 | 每页条数，默认 `10`（上限 100） |
| `status` | 否 | 按上下架过滤（`0` 下架 / `1` 上架），非法值 400 |
| `keyword` | 否 | 按 `name_en` / `name_zh` 模糊搜索 |

按 `sort_order ASC, id ASC` 排序。返回字段：`id`、`name_en`、`name_zh`、`name_ja`、`description`（JSON 对象）、`latitude`、`longitude`、`category`、`region_id`、`cover_url`、`status`、`sort_order`。

```bash
curl -H "Authorization: Bearer <JWT>" "http://localhost:8082/api/v1/admin/destinations?page=1&page_size=10"
```

```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "list": [ { "id": 1, "name_en": "Tokyo", "name_zh": "东京", "name_ja": "東京", "description": { "en": "Capital of Japan", "zh": "日本首都" }, "latitude": 35.6762, "longitude": 139.6503, "category": "city", "region_id": 1, "cover_url": "https://...", "status": 1, "sort_order": 0 } ],
    "total": 1,
    "page": 1,
    "page_size": 10
  }
}
```

#### `POST /api/v1/admin/destinations` — 创建目的地

鉴权：需要 admin JWT。请求体为 JSON 对象，`name_en` 与 `name_zh` 必填（否则 400）。

- 可写字段：`name_en`、`name_zh`、`name_ja`、`description`（JSON 对象，落库为 JSON 字符串）、`cover_url`、`status`、`sort_order`、`latitude`、`longitude`、`region_id`、`category`
- 缺省补默认值：`name_ja=""`、`latitude` / `longitude=0`、`region_id=0`
- 成功返回创建后的完整目的地（含 `id`）

```bash
curl -H "Content-Type: application/json" -H "Authorization: Bearer <JWT>" -X POST \
  http://localhost:8082/api/v1/admin/destinations -d '{"name_en":"Kyoto","name_zh":"京都","region_id":1,"category":"city"}'
```

#### `PUT /api/v1/admin/destinations/{id}` — 更新目的地

鉴权：需要 admin JWT。请求体为 JSON 对象，至少一个可写字段（否则 400）。可写字段与创建相同，成功返回更新后的目的地。

- `404`：目的地不存在

#### `PUT /api/v1/admin/destinations/{id}/status` — 上下架

鉴权：需要 admin JWT。请求体 `{ "status": 0 | 1 }`（非法值 400），成功返回更新后的目的地。

```bash
curl -H "Content-Type: application/json" -H "Authorization: Bearer <JWT>" -X PUT \
  http://localhost:8082/api/v1/admin/destinations/1/status -d '{"status":0}'
```

#### `DELETE /api/v1/admin/destinations/{id}` — 删除目的地

鉴权：需要 admin JWT。

- `409`：目的地仍有关联景区（`destination has related attractions, delete them first`），需先删除景区
- `404`：目的地不存在
- 成功：`data` 为 `null`

#### `GET /api/v1/admin/attractions` — 景区列表

鉴权：需要 admin JWT。分页参数同目的地列表（`page` / `page_size`，另支持 `destination_id` 过滤），按 `id ASC` 排序。

- 返回字段：`id`、`destination_id`、13 个语种名称（`name_en/name_zh/name_ja/name_ko/name_ar/name_es/name_fr/name_de/name_pt/name_hi/name_bn/name_id/name_ru`）、`description`（JSON 对象）、`price_cents`、`status`、`open_hours`、`rating_avg`、`cover_url`

#### `POST /api/v1/admin/attractions` — 创建景区

鉴权：需要 admin JWT。请求体为 JSON 对象。

- `name_en` 必填；`destination_id` 必填且目的地必须存在（否则 400）
- 可写字段：`destination_id`、13 个 `name_*`、`description`（JSON 对象）、`price_cents`、`status`、`open_hours`、`rating_avg`、`cover_url`；其余 `name_*` 与 `open_hours` 缺省补空串
- 成功返回创建后的完整景区（含 `id`）

#### `PUT /api/v1/admin/attractions/{id}` — 更新景区

鉴权：需要 admin JWT。请求体为 JSON 对象，至少一个可写字段（否则 400）；不允许修改 `destination_id`。

- `404`：景区不存在

#### `DELETE /api/v1/admin/attractions/{id}` — 删除景区

鉴权：需要 admin JWT。`404`：景区不存在；成功：`data` 为 `null`。

### 搜索服务（search-service）

公开接口，无需 JWT。OpenSearch 索引优先，索引不可用时回退 MySQL `LIKE` 检索；检索词写入 `travel_searches` 热词日志。

#### `GET /api/v1/search` — 多条件检索

| 查询参数 | 必填 | 说明 |
|------|------|------|
| `q` | 否 | 关键词（空则按目的地过滤或返回全部） |
| `destination_id` | 否 | 目的地过滤 |
| `lang` | 否 | 语言，默认 `en`；命中多语种字段 |
| `price_min` / `price_max` | 否 | 价格区间（分） |
| `page` | 否 | 页码，默认 `1` |

响应 `data`：`{ "total", "page", "page_size", "items": [ { "id", "type"("destination"\|"attraction"), "name", "price_cents", "cover_url", "description" } ] }`。

```bash
curl "http://localhost:8082/api/v1/search?q=tokyo&lang=zh"
```

#### `GET /api/v1/search/hotwords?period=day|week|all&limit=N` — 热词推荐

公开接口，无鉴权。按 `travel_searches` 检索日志对 `keyword` 分组计数，倒序返回 Top N。

| 查询参数 | 必填 | 说明 |
|------|------|------|
| `period` | 否 | 统计周期：`day`（默认，近 1 天）/ `week`（近 7 天）/ `all`（全部），非法值 400 |
| `limit` | 否 | 返回条数，默认 `10`，上限 50，非法值 400 |

结果走 Redis 缓存（`hotwords:{period}:{limit}`，TTL 300s）→ MySQL 聚合回源。

响应 `data`：`[ { "keyword", "count" } ]`。

```bash
curl "http://localhost:8082/api/v1/search/hotwords?period=day&limit=10"
```

### 线路服务（line-service）

公开接口，无需 JWT。列表走 Redis 缓存（TTL 5 分钟），详情/日历直读 MySQL（从库优先）。

#### `GET /api/v1/lines` — 线路列表

| 查询参数 | 必填 | 说明 |
|------|------|------|
| `destination_id` | 否 | 目的地过滤，缺省返回全部上架线路 |
| `lang` | 否 | 语言，默认 `en`；标题按 `title_{lang}` → `title_zh` → `title_en` 回退 |

响应 `data`：`[ { "id", "title", "destination_id", "days", "price_cents", "max_pax", "cover_url" } ]`。

#### `GET /api/v1/lines/{id}` — 线路详情

`data` 含 `itinerary`：`[ { "day", "title", "description" } ]`（按 lang 取行程标题，回退链同上）。`404`：线路不存在或已下架。

#### `GET /api/v1/lines/{id}/dates` — 出发日历与余位

未来班期（`depart_date >= 今天`）按日期升序，余位实时读取（随订单预占扣减），**不缓存**。

响应 `data`：`[ { "id", "date", "price_cents", "seats_left", "sold_out" } ]`（`sold_out = seats_left == 0`）。

### 订单服务（order-service）

全部接口需要 **用户 JWT**（`POST /api/v1/user/login` 签发）。限流独立统计。

#### `POST /api/v1/orders` — 下单

请求体：`{ "order_type": 1, "product_id": 10020001, "line_date_id": 1, "quantity": 1 }`。

- 仅支持 `order_type=1`（线路）；机票/酒店（2/3）返回 `501`
- **防超卖双防线**：Redis 原子预占（`INCRBY travel:stock:{line_date_id}`）+ 数据库原子扣减（`seats_left >= quantity` 才扣）
- 快照含标题/单价/出发日期/数量，订单 `expire_at = now + 15 分钟`，超时未支付由后台任务释放余位
- `409`：余位不足（`insufficient stock`）

```bash
curl -H "Content-Type: application/json" -H "Authorization: Bearer <JWT>" -X POST \
  http://localhost:8082/api/v1/orders -d '{"order_type":1,"product_id":10020001,"line_date_id":1,"quantity":1}'
```

响应 `data`：`{ "id", "order_type", "product_id", "status", "amount_cents", "snapshot", "expire_at", "created_at" }`。

#### `GET /api/v1/orders` — 订单列表

当前用户订单按创建时间倒序，分页参数 `page` / `page_size`。响应 `data`：`{ "items": [ ...订单对象 ], "total", "page", "page_size" }`。

#### `GET /api/v1/orders/{id}` — 订单详情

仅限本人订单，否则 `404`。

#### `POST /api/v1/orders/{id}/cancel` — 取消订单

仅 `status=0`（待支付）可取消；取消后释放余位（DB 回补 + Redis 预占回滚），订单置 `status=4`。其他状态 `409`。

### 管理端线路（admin-service 扩展）

鉴权同管理服务：需要 admin JWT（`role=admin`）。

#### `GET /api/v1/admin/lines` — 线路列表

分页参数 `page` / `page_size`（同目的地列表），`keyword` 匹配 `title_zh` / `title_en`。

响应 `data`：`{ "items": [ { "id", "title_en", "title_zh", "title_ja", "title_ko", "title_ru", "destination_id", "days", "departure_date", "price_cents", "max_pax", "itinerary", "status", "cover_url" } ], "total", "page", "page_size" }`。

#### `POST /api/v1/admin/lines` — 创建线路

请求体：5 个 `title_*`（`title_en` / `title_zh` 必填，否则 400）、`destination_id`、`days`、`departure_date`、`price_cents`、`max_pax`、`cover_url`、`status`、`itinerary`。

- `itinerary` 为前端数组格式 JSON 字符串：`[{"day":1,"title":{"en":"...","zh":"..."},"description":{"zh":"...","en":"..."}}]`，后端转换为存储格式 `{"days":[{day,title_en,title_zh,...,description}]}`（`description` 取 zh 优先、en 回退）
- 兼容种子数据已有的 `{"days":[...]}` 格式（原样落库）

#### `PUT /api/v1/admin/lines/{id}` — 更新线路

可写字段与创建相同（至少一个，否则 400）。`404`：线路不存在。

#### `PUT /api/v1/admin/lines/{id}/status` — 上下架

请求体 `{ "status": 0 | 1 }`（非法值 400）。

#### `DELETE /api/v1/admin/lines/{id}` — 删除线路

- `409`：线路仍有班期（`line has related dates, delete them first`）
- `404`：线路不存在

#### `GET /api/v1/admin/lines/{id}/dates` — 班期列表

响应 `data`：裸数组 `[ { "id", "line_id", "depart_date", "price_cents", "seats_left", "status" } ]`。

#### `POST /api/v1/admin/lines/{id}/dates` — 新增班期

请求体 `{ "depart_date", "price_cents", "seats_left", "status" }`。

- `409`：同线路同日班期已存在（唯一键 `uk_line_date` 兜底）
- `404`：线路不存在

#### `PUT /api/v1/admin/lines/{id}/dates/{date_id}` — 更新班期

可写字段同新增（至少一个，否则 400）；更新 `depart_date` 撞唯一键返回 `409`。`404`：班期不存在。

#### `DELETE /api/v1/admin/lines/{id}/dates/{date_id}` — 删除班期

`404`：班期不存在；成功：`data` 为 `null`。

### 航班服务（flight-service，P4-01/02）

#### `GET /api/v1/flights/search?from=HND&to=HKG&depart_date=YYYY-MM-DD&cabin=0&page=&page_size=` — 航班检索

公开。`from`/`to` 为 IATA 三字码，必填；`cabin` 0经济/1商务/2头等，缺省全部；`depart_date` 精确匹配当天航班（可省）。返回 `items`（`id/airline/flight_no/from_code/to_code/depart_at/arrive_at/cabin/price_cents/seats_left`）、`total`、`page`、`page_size`。`404`：该航线无航班。

#### `GET /api/v1/flights/{id}` — 航班详情

公开。`404`：航班不存在或已下架。

### 酒店服务（hotel-service，P4-03/04）

#### `GET /api/v1/hotels/search?city=HKG&star=&page=&page_size=` — 酒店检索

公开。`city` 三字码必填；`star` 1-5 可省。返回 `items`（`id/name_en/name_zh/city_code/star/latitude/longitude/cover_url`）、分页信息。

#### `GET /api/v1/hotels/{id}` — 酒店详情（含房型）

公开。返回酒店信息 + `rooms` 列表（`id/room_type_en/room_type_zh/price_cents/breakfast/inventory`）。`404`：酒店不存在或已下架。

### 支付服务（payment-service，P4-05/06/15）

#### `POST /api/v1/payments` — 发起支付（用户 JWT）

请求 `{"order_id": 123, "channel_code": "stripe"}`。校验订单属于当前用户且 `status=0`（已支付/已取消返回 400）。写支付流水（`status=0` 待支付）并返回 `{"txn_no", "amount_cents", "checkout_url"}`——沙箱环境 `checkout_url` 指向 `GET /api/v1/payments/sandbox/{txn_no}`（模拟收银台页）。`404`：订单不存在。

#### `POST /api/v1/payments/callback/{channel_code}` — 支付渠道回调（内部）

无 JWT，以 `X-Internal-Token`（环境变量 `INTERNAL_TOKEN`，默认 `dev-internal-secret`）防护。请求 `{"txn_no": "...", "status": 1}`（status 1 成功/2 失败）。按 `txn_no` 幂等：已处理过直接返回 `{"duplicate": true}`。成功时更新流水 `status=1/paid_at` 并调用 order-service `POST /api/v1/orders/{id}/pay-success` 推进订单 `0→1`（内部 X-Internal-Token）。回调请求体以 `X-Signature`（HMAC-SHA256，密钥固定 `sandbox-secret`，开发环境模拟验签）签名校验，非法返回 401。

#### `GET /api/v1/payments/sandbox/{txn_no}` — 沙箱收银台页

公开。展示待支付流水与「模拟支付成功/失败」按钮（开发环境用）。

#### `GET /api/v1/payments/channels?lang=zh` — 渠道列表（按语言路由）

公开。返回全部启用渠道，本国渠道（`languages` 含当前 lang）排前、全语言渠道（`languages=''`）兜底，同档按 `priority` 降序；`name` 取当前语言。返回 `items`（`channel_code/name/type/priority`）。

### 管理端支付（admin-service，P4-16）

#### `GET /api/v1/admin/payments?channel=&status=&page=&page_size=` — 支付流水列表（管理 JWT）

`channel_code` 精确匹配；`status` 0待支付/1成功/2失败/3已退款，越界 400。JOIN 订单/用户返回 `email`。按 id 倒序分页。

#### `GET /api/v1/admin/payments/channels` — 渠道列表（含禁用项）

全量返回，按 `priority` 升序。

#### `PATCH /api/v1/admin/payments/channels/{code}/enabled` — 渠道开关

请求 `{"enabled": true}`，即时生效（payment-service 路由渠道时实时读表）。`404`：渠道不存在；成功返回更新后渠道对象。

### 管理端订单/用户/航班/酒店（admin-service，P4-08/09/10/14）

#### `GET /api/v1/admin/orders?page=&page_size=` — 订单列表

全量订单倒序分页，含用户 `email` 与商品快照。

#### `POST /api/v1/admin/orders/{id}/refund` — 退款

`status IN (1,2)` 才可退，置 `4` 并回补对应商品库存（线路回补班期余位、航班回补余票、酒店回补房态库存）。已退/其他状态 `400`；`404`：订单不存在。

#### `GET /api/v1/admin/flights` / `POST /api/v1/admin/flights` / `PUT /api/v1/admin/flights/{id}` / `PUT /api/v1/admin/flights/{id}/status` — 航班管理

航班 CRUD 与上下架；新增字段 `airline/flight_no/from_code/to_code/depart_at/arrive_at/cabin/price_cents/seats_left`。

#### `GET /api/v1/admin/hotels` / `POST /api/v1/admin/hotels` / `PUT /api/v1/admin/hotels/{id}` / `PUT /api/v1/admin/hotels/{id}/status` — 酒店管理

酒店 CRUD 与上下架。

#### `GET/POST /api/v1/admin/hotels/{id}/rooms`、`PUT/DELETE /api/v1/admin/hotels/{id}/rooms/{room_id}` — 房型管理

房型 CRUD，字段 `room_type_en/room_type_zh/room_type_ja/price_cents/breakfast/inventory`。

#### `GET /api/v1/admin/users?page=&page_size=` — 用户列表

全量用户分页，含注册时间；`PATCH /api/v1/admin/users/{id}/status` 请求 `{"status": 0 | 1}`（非法值 400），`1` 禁用 / `0` 恢复。`404`：用户不存在。禁用用户 JWT 请求返回 `403`（user-service 所有接口生效）。

### 数据看板（admin-service，P5-02）

鉴权同管理服务。聚合在 MySQL 内完成，金额单位分（cents）。

#### `GET /api/v1/admin/stats/overview` — 总览

`data`：`{ "total_orders", "paid_orders", "gmv_cents"（status>=1 求和）, "conversion_rate"（百分比，两位小数）, "status_counts": { "0".."4" } }`。

#### `GET /api/v1/admin/stats/top` — Top 目的地与线路

`data`：`{ "top_destinations": [ { "id", "name"（中文优先）, "orders" } ], "top_lines": [ ... ] }`，各 Top 5（按线路订单量）。

#### `GET /api/v1/admin/stats/trend` — 近 7 天订单趋势

`data.items`：`[ { "day"（YYYY-MM-DD）, "orders" } ]`，恒定 7 行（含当日，无单日补零）。

### CDN 云商管理（admin-service，P5-08）

鉴权同管理服务。**服务端不存储云凭据**：接口仅管理配置并生成 dry-run 命令预览文本，真实执行需在部署机配置对应云 CLI 凭据。

#### `GET /api/v1/admin/cdn/providers` — 提供商列表

全量返回（含禁用项），按 `provider_code` 升序。`data.items` 字段：`provider_code`、`name`、`enabled`（bool）、`bucket`、`region`、`domain`、`endpoint`、`updated_at`。

#### `PATCH /api/v1/admin/cdn/providers/{code}/status` — 启停

请求体 `{ "enabled": true | false }`。`404`：提供商不存在；成功返回更新后对象。

#### `PUT /api/v1/admin/cdn/providers/{code}` — 更新配置

请求体可选字段 `bucket` / `region` / `domain` / `endpoint`（null 跳过，空串表示清空；`region` 不允许为空）。`400`：无可更新字段 / region 为空；`404`：提供商不存在。

#### `POST /api/v1/admin/cdn/providers/{code}/plan` — dry-run 命令预览

`404`：提供商不存在；`400`：bucket 未配置（`configure bucket first`）。成功返回 `{ "provider_code", "commands": ["cdn_setup.sh ...", "cdn_upload.sh ..."], "hint" }`——命令仅输出预览，本服务不执行。

### 订单扩展（order-service，P4-12）

#### `POST /api/v1/orders` — 下单（用户 JWT，支持 type 1/2/3）

请求 `{"order_type": 1, "product_id": 123, "line_date_id": 45, "quantity": 2}`：

- `order_type=1` 线路：`product_id`=线路 id、`line_date_id`=班期 id（必填且须匹配该线路，否则 400）；库存扣减/Redis 预占以班期余位计。
- `order_type=2` 航班：`product_id`=航班 id；已起飞/下架返回 404，余票不足 409。
- `order_type=3` 酒店：`product_id`=房型 id，可带 `check_in`/`check_out`（存快照）；房态库存不足 409。

`quantity` 1-99，越界 400。防超卖双防线不变：Redis 预占（`travel:stock:{order_type}:{stock_id}`）+ DB 原子扣减（`WHERE ... >= ?`，受影响 0 行回滚并 409）。快照含商品标题/价格/日期，`destination_id` 仅线路填真实目的地，航班/酒店记 0。成功 `data` 含 `expire_at`（15 分钟后自动取消，惰性 + 60s 后台扫描）。

## 中间件链

业务路由挂载完整中间件链（执行顺序：外层 → 内层）：

- **user-service**：`Tracing → CircuitBreaker → Security → RateLimit(Redis) → Auth(JWT)`
- **booking-service**：`Tracing → CircuitBreaker → Security → RateLimit`
- **admin-service**：`Tracing → CircuitBreaker → Security → RateLimit(Redis) → JWT(Auth)`（`/api/v1/admin/login` 公开，其余业务路由挂 JWT）
- **search-service / line-service**：`Tracing → CircuitBreaker → Security → RateLimit`（公开，无 JWT）
- **order-service**：`Tracing → CircuitBreaker → Security → RateLimit → JWT(Auth)`（全部接口需用户 JWT；`POST /api/v1/orders/{id}/pay-success` 内部接口无 JWT，以 X-Internal-Token 防护）
- **flight-service / hotel-service**：`Tracing → CircuitBreaker → Security → RateLimit`（公开，无 JWT）
- **payment-service**：`Tracing → CircuitBreaker → Security → RateLimit`；`/api/v1/payments` 需用户 JWT，`/api/v1/payments/callback/{channel_code}` 无 JWT（X-Internal-Token + HMAC 签名防护）

## 相关文档

- [项目 README](../README.md)（快速开始、端口映射、环境变量）
- [联调报告](integration-report.md)（链路验证结果）
- [压测报告](loadtest-report.md)
