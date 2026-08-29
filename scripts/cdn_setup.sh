#!/usr/bin/env bash
# open-travel CDN 一键配置：AWS CloudFront / 阿里云 / Google Cloud CDN / Azure CDN。
# 插件式：每个云一个 provider 脚本（scripts/cdn/<provider>.sh），实现
#   cdn_require_creds  cdn_setup  cdn_upload <src> <prefix> <ttl>
# 幂等：状态存于 scripts/.cdn-state-<provider>，重复执行复用已有资源。
# 无凭据时 --dry-run 可预览全部命令；真实执行需配置对应云 CLI 凭据。
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROVIDER=cloudfront
BUCKET=
REGION=us-east-1
DOMAIN=
CERT_ARN=
ENDPOINT=
DRY_RUN=0
FORCE=0

usage() {
  cat <<EOF
用法: $0 --bucket <bucket> [选项]

  --provider cloudfront|aliyun|gcp|azure   CDN 提供方（默认 cloudfront；aliyun 兼容旧名 oss）
  --bucket <name>              源站 bucket（必填）
  --region <name>              源站 region（默认 us-east-1，各云含义不同）
  --domain <name>              CDN 域名 CNAME（可选；用自定义域名时需先申请证书）
  --cert <arn>                 证书 ARN（配 --domain 时必填）
  --endpoint <url>             源站 endpoint（仅 aliyun，如 oss-cn-hangzhou.aliyuncs.com）
  --dry-run                    只预览命令不执行（无凭据时也能预览，默认在无凭据时自动启用）
  --force                      忽略已有状态，重建（危险）
  -h, --help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --provider) PROVIDER="$2"; shift 2 ;;
    --bucket)   BUCKET="$2"; shift 2 ;;
    --region)   REGION="$2"; shift 2 ;;
    --domain)   DOMAIN="$2"; shift 2 ;;
    --cert)     CERT_ARN="$2"; shift 2 ;;
    --endpoint) ENDPOINT="$2"; shift 2 ;;
    --dry-run)  DRY_RUN=1; shift ;;
    --force)    FORCE=1; shift ;;
    -h|--help)  usage; exit 0 ;;
    *) echo "未知参数: $1" >&2; usage; exit 1 ;;
  esac
done

[[ -n "$BUCKET" ]] || { echo "错误: --bucket 必填" >&2; usage; exit 1; }
[[ "$PROVIDER" == oss ]] && PROVIDER=aliyun
case "$PROVIDER" in
  cloudfront|aliyun|gcp|azure) ;;
  *) echo "错误: --provider 只能为 cloudfront|aliyun|gcp|azure" >&2; exit 1 ;;
esac
[[ -n "$DOMAIN" && -z "$CERT_ARN" ]] && { echo "错误: 使用 --domain 时必须同时提供 --cert" >&2; exit 1; }
[[ -z "$DOMAIN" && -n "$CERT_ARN" ]] && { echo "错误: 提供 --cert 时必须同时提供 --domain" >&2; exit 1; }

PROVIDER_SCRIPT="$SCRIPT_DIR/cdn/$PROVIDER.sh"
[[ -f "$PROVIDER_SCRIPT" ]] || { echo "错误: 缺少 provider 脚本 $PROVIDER_SCRIPT" >&2; exit 1; }
# shellcheck disable=SC1090
source "$PROVIDER_SCRIPT"

STATE_FILE="$SCRIPT_DIR/.cdn-state-$PROVIDER"

# 共享 save_state（provider 未自行定义时使用；DRY_RUN 不写状态）
if ! declare -F save_state >/dev/null 2>&1; then
  save_state() { [[ $DRY_RUN -eq 1 ]] && return 0; printf '%s\n' "$1" >> "$STATE_FILE"; }
fi

load_state() {
  [[ -f "$STATE_FILE" ]] || return 0
  set -a; source "$STATE_FILE"; set +a
}
[[ $FORCE -eq 1 ]] && rm -f "$STATE_FILE"
load_state

if ! cdn_require_creds; then
  echo "==> 凭据未配置，自动进入 DRY-RUN 预览（不会执行任何真实命令）。"
  DRY_RUN=1
fi

cdn_setup
echo "完成。状态文件: $STATE_FILE"
