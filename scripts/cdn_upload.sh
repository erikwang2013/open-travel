#!/usr/bin/env bash
# 将待加速的静态资源同步到 CDN 源站 bucket（插件式：source scripts/cdn/<provider>.sh）。
# 默认 --dry-run 只打印不执行；--no-dry-run 才真正上传。
# 三个目标：docs/svg/* 与 docs/*.png → TTL 30d；apps/flutter/build/web → TTL 1d。
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROVIDER=cloudfront
BUCKET=
REGION=us-east-1
ENDPOINT=
DRY_RUN=1

usage() {
  cat <<EOF
用法: $0 --bucket <bucket> [选项]

  --provider cloudfront|aliyun|bunny|gcp|azure|huawei|cloudflare|tencent   源站类型（默认 cloudfront；aliyun 兼容旧名 oss）
  --bucket <name>              源站 bucket（必填）
  --region <name>              region（默认 us-east-1，各云含义不同）
  --endpoint <url>             endpoint（仅 aliyun，如 oss-cn-hangzhou.aliyuncs.com）
  --dry-run                    只预览将要上传的文件（默认）
  --no-dry-run                 真正执行上传
  -h, --help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --provider)   PROVIDER="$2"; shift 2 ;;
    --bucket)     BUCKET="$2"; shift 2 ;;
    --region)     REGION="$2"; shift 2 ;;
    --endpoint)   ENDPOINT="$2"; shift 2 ;;
    --dry-run)    DRY_RUN=1; shift ;;
    --no-dry-run) DRY_RUN=0; shift ;;
    -h|--help)    usage; exit 0 ;;
    *) echo "未知参数: $1" >&2; usage; exit 1 ;;
  esac
done

[[ -n "$BUCKET" ]] || { echo "错误: --bucket 必填" >&2; usage; exit 1; }
[[ "$PROVIDER" == oss ]] && PROVIDER=aliyun
PROVIDER_SCRIPT="$SCRIPT_DIR/cdn/$PROVIDER.sh"
[[ -f "$PROVIDER_SCRIPT" ]] || { echo "错误: 缺少 provider 脚本 $PROVIDER_SCRIPT" >&2; exit 1; }
# shellcheck disable=SC1090
source "$PROVIDER_SCRIPT"

[[ $DRY_RUN -eq 1 ]] && echo "==> DRY-RUN 模式（只预览，--no-dry-run 执行）"
if ! cdn_require_creds; then
  echo "==> 凭据未配置，本次仅预览（配置后会自动上传）。"
fi

# 目的地图片：docs/svg/*、docs/*.png → 30 天长缓存
if [[ -d "$PROJECT_DIR/docs/svg" ]]; then
  echo "--> docs/svg -> $BUCKET/svg/（TTL 30d）"
  cdn_upload "$PROJECT_DIR/docs/svg" "svg/" 2592000
fi
for png in "$PROJECT_DIR"/docs/*.png; do
  [[ -f "$png" ]] || continue
  echo "--> $(basename "$png") -> $BUCKET/docs/（TTL 30d）"
  cdn_upload "$png" "docs/" 2592000
done

# Flutter web 构建产物：TTL 1 天（版本化文件名，1d 足够且更新快）
if [[ -d "$PROJECT_DIR/apps/flutter/build/web" ]]; then
  echo "--> apps/flutter/build/web -> $BUCKET/web/（TTL 1d）"
  cdn_upload "$PROJECT_DIR/apps/flutter/build/web" "web/" 86400
else
  echo "跳过: apps/flutter/build/web 不存在（先 flutter build web）"
fi

[[ $DRY_RUN -eq 1 ]] && echo "==> 预览完成。确认无误后加 --no-dry-run 执行。"
