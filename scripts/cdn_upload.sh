#!/usr/bin/env bash
# 将待加速的静态资源同步到 CDN 源站 bucket。
# 默认 --dry-run 只打印不执行；--no-dry-run 才真正上传。
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROVIDER=cloudfront
BUCKET=
REGION=us-east-1
ENDPOINT=
DRY=1

usage() {
  cat <<EOF
用法: $0 --bucket <bucket> [选项]

  --provider cloudfront|oss    源站类型（默认 cloudfront）
  --bucket <name>              源站 bucket（必填）
  --region <name>              S3 region（默认 us-east-1，仅 cloudfront）
  --endpoint <url>             OSS endpoint（仅 oss）
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
    --dry-run)    DRY=1; shift ;;
    --no-dry-run) DRY=0; shift ;;
    -h|--help)    usage; exit 0 ;;
    *) echo "未知参数: $1" >&2; usage; exit 1 ;;
  esac
done

[[ -n "$BUCKET" ]] || { echo "错误: --bucket 必填" >&2; usage; exit 1; }

sync_aws() {  # $1=src $2=prefix $3=cache-control
  aws s3 sync "$1" "s3://$BUCKET/$2" --region "$REGION" \
    --cache-control "public, max-age=$3" $( [[ $DRY -eq 1 ]] && echo --dryrun )
}

sync_oss() {  # $1=src $2=prefix
  aliyun oss cp -r "$1" "oss://$BUCKET/$2" ${ENDPOINT:+-e "$ENDPOINT"} $( [[ $DRY -eq 1 ]] && echo --dryrun )
}

[[ $DRY -eq 1 ]] && echo "==> DRY-RUN 模式（只预览，--no-dry-run 执行）"

# 目的地图片：docs/svg/*、docs/*.png → 30 天长缓存
if [[ -d "$PROJECT_DIR/docs/svg" ]]; then
  echo "--> docs/svg -> $BUCKET/svg/（TTL 30d）"
  [[ "$PROVIDER" == oss ]] && sync_oss "$PROJECT_DIR/docs/svg" "svg/" || sync_aws "$PROJECT_DIR/docs/svg" "svg/" 2592000
fi
for png in "$PROJECT_DIR"/docs/*.png; do
  [[ -f "$png" ]] || continue
  echo "--> $(basename "$png") -> $BUCKET/docs/（TTL 30d）"
  [[ "$PROVIDER" == oss ]] && sync_oss "$png" "docs/" || sync_aws "$png" "docs/" 2592000
done

# Flutter web 构建产物：TTL 1 天（版本化文件名，1d 足够且更新快）
if [[ -d "$PROJECT_DIR/apps/flutter/build/web" ]]; then
  echo "--> apps/flutter/build/web -> $BUCKET/web/（TTL 1d）"
  [[ "$PROVIDER" == oss ]] && sync_oss "$PROJECT_DIR/apps/flutter/build/web" "web/" || sync_aws "$PROJECT_DIR/apps/flutter/build/web" "web/" 86400
else
  echo "跳过: apps/flutter/build/web 不存在（先 flutter build web）"
fi

[[ $DRY -eq 1 ]] && echo "==> 预览完成。确认无误后加 --no-dry-run 执行。"
