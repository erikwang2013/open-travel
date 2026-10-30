# Changelog

Open Travel 项目版本变更记录。项目版本独立于 e-cat 框架版本；框架变更见 [e-cat/CHANGELOG.md](e-cat/CHANGELOG.md)。
更早的项目版本（v1.5.1–v1.5.5）见 GitHub Releases。

## [1.5.6] — 2026-10-30

### 修复

- **库存**：取消订单/超时扫描改为先抢占状态（`status=0→4`），仅抢到的请求回补库存，避免并发取消、支付与超时之间的重复回补与超卖。
- **支付**：支付回调在流水已成功但订单确认失败时会重试订单确认；支付流水按订单唯一（`uk_order`），并发发起支付不再产生两条流水；失败支付复用原流水重新发起。
- **订单 ID**：订单响应新增 `id_str`，支付发起兼容字符串 `order_id`，修复 Flutter Web 上雪花 ID 超过 2^53 的精度丢失。
- **客户端**：修复订单列表、支付渠道、订单快照、景点名称的响应解析；航班搜索日期参数改为 `date`；支付页不再依赖后端未返回的 `enabled` 字段。
- **检索**：航班/酒店缓存键纳入全部分页参数；酒店新增 `star` 过滤与 `page_size` 上限；搜索分页在目的地/景点两个索引合并后统一切页；修复 MySQL 回退 `AND AND` 拼接导致景点关键词检索为空。
- **限流**：限流键按客户端 IP 隔离；payment-service 启用 Redis 限流。
- **安全/部署**：`ecat-order` 补充 `INTERNAL_TOKEN`；收银台链接支持 `PUBLIC_BASE_URL` 配置。
- **CI/测试**：CI 改为编译并运行全部集成测试；idgen 用例在无 Redis 时可跳过；新增订单并发取消、支付确认重试、缓存分页、星级过滤等回归用例。
- **清理**：移除无引用的 `travel_bookings` / `travel_reviews` 表定义；补 `travel_orders(status, expire_at)` 索引；搜索日志保留 30 天并每小时清理。

### 文档

- 更新 `README.md`、英文 README、`docs/api.md`、`e-cat/README.md` 与发布流程中的版本/接口/限流说明。
- 更新请求生命周期图与安全架构图（中/英）中的限流文案：每客户端 100 req/60s。

### 升级注意

- 已有数据库需手工执行：
  - `ALTER TABLE travel_payments ADD UNIQUE KEY uk_order (order_id);`（先清理重复 `order_id`）
  - `ALTER TABLE travel_orders ADD INDEX idx_status_expire (status, expire_at);`
- 限流改为按 `X-Real-IP` / `X-Forwarded-For` 统计；服务端口应只经网关暴露，直连时该头可被伪造。
