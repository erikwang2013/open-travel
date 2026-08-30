# Open Travel 客户端（apps/client/flutter）

<p align="center"><img src="../../../docs/mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>

> Open Travel 全球旅游平台的 Flutter 多端客户端（iOS / Android / Web / Desktop），支持 **12+ 语种** i18n。项目总览见[根 README](../../../README.md)。

## 功能列表

- 多语言搜索：目的地 / 酒店 / 机票 / 线路（12+ 语种，OpenSearch 后端）
- 酒店与机票预订
- 旅游线路浏览与预订
- 订单管理：查询与状态跟踪
- 在线支付（微信支付 / 支付宝）
- 评价系统
- i18n：12+ 语种 ARB 语言包，RTL 支持

## 安装与运行

客户端依赖后端服务，请先完成一键安装：

```bash
cd ../../..        # 回到仓库根目录
./scripts/install.sh
```

启动客户端：

```bash
cd apps/client/flutter
flutter pub get
flutter run -d chrome
```

接口经 Nginx 网关（http://localhost:8082）按前缀分流到各微服务。

## 使用说明

- 注册账号后登录即可使用
- 开发环境默认管理账号：`admin@travel.local` / `Admin@123`（仅本地使用）
- 支持切换语言（12+ 语种，含 RTL 语种）

## 目录结构

```
apps/client/flutter/
├── lib/
│   ├── main.dart          # 入口
│   ├── config.dart        # 网关地址等配置
│   ├── pages/             # 页面（搜索 / 预订 / 订单 / 我的）
│   ├── services/          # API 客户端
│   ├── models/            # 数据模型
│   └── widgets/           # 通用组件
├── assets/                # 静态资源与 ARB 语言包
├── android/ ios/ web/ linux/ macos/ windows/   # 各平台工程
└── pubspec.yaml
```

## 相关文档

- 根 README：[../../../README.md](../../../README.md)
- 后端 README：[../../../e-cat/README.md](../../../e-cat/README.md)
- API 参考：[../../../docs/api.md](../../../docs/api.md)
