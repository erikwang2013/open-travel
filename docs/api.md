<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Open Travel API 参考

本页汇总 Open Travel 后端（user-service / booking-service）的 HTTP 接口。服务基于 e-cat 框架构建，业务路由由各服务注册，经 Nginx 网关统一暴露。

## 访问入口

| 入口 | 地址 | 说明 |
|------|------|------|
| Nginx 网关 | `http://localhost:8082` | 宿主 8082 → 容器 80，按前缀分流 |
| user-service 直连 | `http://localhost:8001` | 用户服务 |
| booking-service 直连 | `http://localhost:8002` | 目的地服务 |

网关分流规则（`config/nginx.conf`）：`/api/user/*` → user-service，`/api/booking/*` → booking-service；`/health`、`/ready` → user-service。

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

API 版本通过请求头 `X-Api-Version` 传递，**强制要求**：

- 当前仅一个版本：`v1`
- 所有业务请求必须携带 `X-Api-Version: v1`
- 缺失该 header 或值不是 `v1` 时返回 `400`（如 `{"code":400,"message":"unsupported api version","data":null}`）

```bash
curl -H "X-Api-Version: v1" http://localhost:8082/api/user/profile
```

### 鉴权（JWT）

user-service 的 `/api/user/profile` 需在请求头携带 JWT：

```
Authorization: Bearer <JWT>
```

- 密钥来自 `JWT_SECRET` 环境变量（≥32 字节）；未配置时退回开发占位密钥并告警（生产必须配置）
- 无 token / token 无效时返回 `401`
- booking-service 的接口为公开接口，无鉴权

### 限流

- 每个服务独立限流：**100 请求 / 60 秒**，Redis 分布式固定窗口
- 未认证请求也计入限流（防暴力请求耗尽资源）
- 超限返回 `429`；限流 key 按客户端地址统计

### 错误码

| HTTP 状态码 | 场景 |
|------|------|
| 200 | 成功 |
| 400 | 请求参数错误 / `X-Api-Version` 缺失或不受支持（`code` 非 0，`message` 描述原因） |
| 401 | 未认证 / JWT 无效 / 登录凭据错误（统一返回，防枚举） |
| 403 | 请求被安全中间件拦截（SQL 注入 / XSS / SSRF 等攻击模式） |
| 404 | 用户不存在 |
| 409 | 邮箱已注册 |
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

#### `POST /api/user/register` — 用户注册

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
curl -H "X-Api-Version: v1" -H "Content-Type: application/json" -X POST \
  http://localhost:8082/api/user/register -d '{"email":"a@b.com","password":"secret1"}'
```

```json
{ "code": 0, "message": "ok", "data": { "user_id": 1, "email": "a@b.com", "lang": "en" } }
```

#### `POST /api/user/login` — 登录

公开接口。请求体 `{ "email": "...", "password": "..." }`。

- **统一 `401`**（`invalid credentials`）：不区分「邮箱不存在」与「密码错误」，防账号枚举
- 成功返回 JWT（24 小时有效期）与用户信息
- `503`：数据库不可用

```bash
curl -H "X-Api-Version: v1" -H "Content-Type: application/json" -X POST \
  http://localhost:8082/api/user/login -d '{"email":"a@b.com","password":"secret1"}'
```

```json
{ "code": 0, "message": "ok", "data": { "token": "<JWT>", "user_id": 1, "email": "a@b.com" } }
```

#### `GET /api/user/profile` — 查询用户资料

鉴权：**需要 JWT**（`Authorization: Bearer <JWT>`，由登录接口签发）。

- `401`：无 token / token 无效
- `400`：token 中 subject 非法
- `404`：用户不存在
- `503`：数据库不可用

```bash
curl -H "X-Api-Version: v1" -H "Authorization: Bearer <JWT>" http://localhost:8082/api/user/profile
```

```json
{ "code": 0, "message": "ok", "data": { "user_id": 1, "email": "a@b.com", "lang": "en" } }
```

### 目的地服务

#### `GET /api/booking/dates?region_id=N` — 热门目的地日期

公开接口，无鉴权。查询 `region_id` 对应的热门目的地列表，走 Redis 缓存（`hot_destinations:{region_id}`，TTL 300s）→ MySQL 回源（`travel_destinations` 表）→ 占位数据兜底，保证无数据源环境可响应。

```bash
curl -H "X-Api-Version: v1" "http://localhost:8082/api/booking/dates?region_id=1"
```

```json
{
  "code": 0,
  "message": "ok",
  "data": [ { "region_id": 1, "name_en": "placeholder-destination" } ]
}
```

`region_id` 缺省时按 `0` 处理。

## 中间件链

业务路由挂载完整中间件链（执行顺序：外层 → 内层）：

- **user-service**：`Tracing → CircuitBreaker → Security → RateLimit(Redis) → Auth(JWT)`
- **booking-service**：`Tracing → CircuitBreaker → Security → RateLimit`

## 相关文档

- [项目 README](../README.md)（快速开始、端口映射、环境变量）
- [联调报告](integration-report.md)（链路验证结果）
- [压测报告](loadtest-report.md)
