# Open Travel 管理端（apps/admin）

<p align="center"><img src="../../docs/mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>

> Open Travel 全球旅游平台的 Flutter Web 管理端，依赖后端 API 运行。项目总览见[根 README](../../README.md)。

## 功能列表

- 数据看板：订单量 / GMV / 转化率 / Top 目的地与线路
- 报表中心：销售日报 / 支付渠道汇总 / 日期范围筛选
- 目的地、景区、线路、航班、酒店管理（多语种字段按 lang 维护）
- 接口版本在 URL 前缀：`/api/v1/...`
- 订单管理：查询、状态跟踪、退款
- 用户管理：用户列表与资料
- 支付渠道配置
- CDN 云商管理（八云插件：cloudfront/aliyun/gcp/azure/cloudflare/tencent/huawei/bunny）

## 安装与运行

管理端依赖后端服务，请先完成一键安装：

```bash
cd ../..            # 回到仓库根目录
./scripts/install.sh
```

启动管理端：

```bash
cd apps/admin
flutter pub get
flutter run -d chrome
```

开发环境经 Nginx 网关代理 `/api/v1/admin`（宿主端口 8082）。

## 使用说明

- 默认管理账号：`admin@travel.local` / `Admin@123`（仅本地开发环境）
- 登录接口：`POST /api/v1/admin/login`
- 报表接口：`GET /api/v1/admin/reports/sales`、`GET /api/v1/admin/reports/payments`
- CDN 云凭据不入库，命令需在部署机配置云 CLI 凭据后执行

## 目录结构

```
apps/admin/
├── lib/
│   ├── main.dart          # 入口
│   ├── api.dart           # API 客户端（网关 8082）
│   └── pages/             # 看板 / 报表 / 管理页面
├── assets/                # 静态资源
└── pubspec.yaml
```

## 相关文档

- 根 README：[../../README.md](../../README.md)
- 后端 README：[../../e-cat/README.md](../../e-cat/README.md)
- API 参考：[../../docs/api.md](../../docs/api.md)
