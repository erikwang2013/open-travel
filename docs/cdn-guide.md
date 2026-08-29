# open-travel CDN 部署指南（Phase 4-3）

CDN 仅加速**静态资源**：docs 图片（`docs/svg/*`、`docs/*.png`）、Flutter web 构建产物（`apps/client/flutter/build/web`）、App 图标。API 走 nginx 网关（动态接口）不经过 CDN；CDN 配置中的 `/api/*` 不缓存规则是兜底保护。

默认方案 **CloudFront + S3**（免费额度内 ≈ $0/月）。脚本为插件式，**AWS / 阿里云 / Google Cloud / Azure / Cloudflare / 腾讯云 / 华为云 / Bunny.net** 八云可切换：`--provider cloudfront|aliyun|gcp|azure|cloudflare|tencent|huawei|bunny`（旧名 `oss` 兼容映射到 `aliyun`）。无云凭据时自动进入 dry-run 预览全部命令，配置凭据后重跑即真实执行。

## 一、前置条件

| 云 | CLI | 凭据 |
| :--- | :--- | :--- |
| AWS | aws CLI | `aws configure` 或 `AWS_ACCESS_KEY_ID`/`AWS_SECRET_ACCESS_KEY`/`AWS_DEFAULT_REGION`，权限含 s3、s3api、cloudfront |
| 阿里云 | ossutil | `ALIBABA_CLOUD_ACCESS_KEY_ID` + `ALIBABA_CLOUD_ACCESS_KEY_SECRET`；CDN 控制台配置（无完整 CLI 流程） |
| Google Cloud | gcloud + gsutil | `GOOGLE_APPLICATION_CREDENTIALS`（服务账号 JSON）或 `gcloud auth login` |
| Azure | az CLI | `az login`（可选 `AZURE_SUBSCRIPTION_ID`） |
| Cloudflare | wrangler（R2） | `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ACCOUNT_ID`（可选 `CLOUDFLARE_ZONE_ID`） |
| 腾讯云 | coscmd + tccli | `TENCENTCLOUD_SECRET_ID` + `TENCENTCLOUD_SECRET_KEY`（可选 `TENCENTCLOUD_APP_ID`） |
| 华为云 | obsutil | `OBS_ACCESS_KEY_ID` + `OBS_SECRET_ACCESS_KEY` |
| Bunny.net | 无（纯 REST API，curl） | `BUNNY_API_KEY`；`BUNNY_STORAGE_PASSWORD` 未设则 setup 自动生成并打印 |
| 自定义域名（可选） | — | AWS 需 ACM 证书（**us-east-1** 签发）；其余云按控制台指引绑定 |

## 二、部署步骤

```bash
cd /home/wwwroot/open-travel

# 1. 创建源站 bucket（脚本内自动执行）
# 2. 一键配置 CDN（默认 cloudfront，幂等可重复执行；无凭据自动 dry-run 预览）
scripts/cdn_setup.sh --bucket open-travel-cdn --region ap-southeast-1
#   可选自定义域名（AWS：先到 ACM 申请证书，证书必须在 us-east-1）：
#   scripts/cdn_setup.sh --bucket open-travel-cdn --domain cdn.erik.xyz --cert arn:aws:acm:us-east-1:...:certificate/...
#   其他云：--provider aliyun|gcp|azure，region 含义各云不同（如 azure 为资源组区域）
#   scripts/cdn_setup.sh --provider aliyun --bucket open-travel-cdn --endpoint oss-cn-hangzhou.aliyuncs.com

# 3. 上传静态资源（默认 dry-run 预览，确认后真正执行）
scripts/cdn_upload.sh --bucket open-travel-cdn --no-dry-run
```

脚本输出：Distribution ID、CDN 域名（形如 `d1234567890.cloudfront.net`）。状态存于 `scripts/.cdn-state-<provider>`（已 gitignore），重复执行自动复用已建资源，不会重复创建。

CloudFront Distribution 部署生效需 **5-10 分钟**。

## 三、域名接入（CNAME）

| 记录类型 | 主机记录 | 记录值 |
| :--- | :--- | :--- |
| CNAME | cdn | `d1234567890.cloudfront.net`（阿里云则为 CDN 分配的 CNAME） |

- 自定义域名必须配 HTTPS 证书（ACM 或阿里云免费证书），脚本要求 `--domain` 与 `--cert` 成对出现。
- 无自定义域名时，直接用 `https://d1234567890.cloudfront.net/...` 访问即可。

## 四、回源与缓存策略

| 路径 | 缓存 TTL | 说明 |
| :--- | :--- | :--- |
| `*.png *.jpg *.jpeg *.webp *.gif *.svg` | 30 天 | 目的地图片、文档图，几乎不变 |
| 默认（其余静态文件） | 1 天（min 1d / max 365d） | Flutter web 产物、其余静态资源 |
| `/api/*` | 0 秒（不缓存） | 兜底：动态接口绝不进缓存 |

- **压缩**：CloudFront 缓存行为已启用 `Compress: true`（自动对 text/html、js、css、svg 等 gzip/br）；OSS 方案在阿里云 CDN 控制台开启 gzip。
- **回源鉴权（CloudFront）**：脚本自动生成 OAI（Origin Access Identity），bucket policy 仅允许该 OAI `s3:GetObject`，bucket 本身不可公开访问。
- **上传缓存头**：`cdn_upload.sh` 同时按目录写入对象级 `Cache-Control`（图片 30d、其余 1d），与 Distribution 配置双保险。
- **回源路径结构**：`docs/svg/*` → `$BUCKET/svg/`，`docs/*.png` → `$BUCKET/docs/`，Flutter web → `$BUCKET/web/`。线上 URL 如 `https://cdn.erik.xyz/svg/architecture.svg`、`https://cdn.erik.xyz/web/index.html`。

## 五、更新资源

```bash
# 重新构建 Flutter web 后增量同步（默认 dry-run 预览）
scripts/cdn_upload.sh --bucket open-travel-cdn
scripts/cdn_upload.sh --bucket open-travel-cdn --no-dry-run
```

`s3 sync` 按文件哈希增量上传；Flutter web 文件名带内容哈希，旧文件可定期清理（TTL 1d 后自然过期）。

## 六、清理 / 回滚

| 操作 | 命令 |
| :--- | :--- |
| 停用 CDN（保留资源） | 控制台禁用 Distribution / 删除 CDN 域名 |
| 彻底删除 | 按 provider 控制台删除 CDN/分发 → 删除源站 bucket；并删除 `scripts/.cdn-state-<provider>` |
| 回滚 | 直接改用源站地址（S3 或 nginx），业务代码无 CDN 依赖，无回滚成本 |

## 七、成本

CloudFront 免费额度：**每月 1TB 出流量 + 1000 万次 HTTP/HTTPS 请求**，本阶段静态资源量级远低于此，**≈ $0/月**（见 `docs/travel-project-planning.md` 成本估算）。S3 存储与请求费用极小（几美元/月以内）。阿里云 OSS 方案同样有免费额度起步。

## 八、故障排查

- **403 Forbidden**：OAI/bucket policy 未生效 —— 重跑 `scripts/cdn_setup.sh`（幂等，会重写 policy）。
- **CNAME 不生效**：等待 Distribution 部署完成（5-10 分钟）再解析。
- **缓存不更新**：Distribution TTL 最长 30 天，紧急更新可在控制台 Invalidate（或等待 TTL 过期）。
