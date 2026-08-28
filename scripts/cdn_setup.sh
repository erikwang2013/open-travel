#!/usr/bin/env bash
# open-travel CDN 一键配置：CloudFront(S3) 或 阿里云 OSS。
# 幂等：状态存于 scripts/.cdn-state，重复执行复用已有 OAI/Distribution。
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STATE_FILE="$SCRIPT_DIR/.cdn-state"

PROVIDER=cloudfront
BUCKET=
REGION=us-east-1
DOMAIN=
CERT_ARN=
ENDPOINT=
FORCE=0

usage() {
  cat <<EOF
用法: $0 --bucket <bucket> [选项]

  --provider cloudfront|oss    CDN 提供方（默认 cloudfront）
  --bucket <name>              源站 bucket（必填）
  --region <name>              S3 region（默认 us-east-1，仅 cloudfront）
  --domain <name>              CDN 域名 CNAME（可选；用自定义域名时需先申请 ACM 证书）
  --cert <arn>                 ACM 证书 ARN（配 --domain 时必填，证书必须在 us-east-1）
  --endpoint <url>             OSS endpoint（仅 oss，如 oss-cn-hangzhou.aliyuncs.com）
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
    --force)    FORCE=1; shift ;;
    -h|--help)  usage; exit 0 ;;
    *) echo "未知参数: $1" >&2; usage; exit 1 ;;
  esac
done

[[ -n "$BUCKET" ]] || { echo "错误: --bucket 必填" >&2; usage; exit 1; }
case "$PROVIDER" in
  cloudfront|oss) ;;
  *) echo "错误: --provider 只能为 cloudfront|oss" >&2; exit 1 ;;
esac
[[ -n "$DOMAIN" && -z "$CERT_ARN" ]] && { echo "错误: 使用 --domain 时必须同时提供 --cert（ACM 证书，us-east-1）" >&2; exit 1; }
[[ -z "$DOMAIN" && -n "$CERT_ARN" ]] && { echo "错误: 提供 --cert 时必须同时提供 --domain" >&2; exit 1; }

load_state() { [[ -f "$STATE_FILE" ]] && { set -a; source "$STATE_FILE"; set +a; }; }
save_state() { printf 'CDN_OAI_ID=%s\nCDN_DIST_ID=%s\n' "${CDN_OAI_ID:-}" "${CDN_DIST_ID:-}" > "$STATE_FILE"; }
[[ $FORCE -eq 1 ]] && rm -f "$STATE_FILE"
load_state

err() { echo "跳过（已存在或不可重复操作）: $*"; }

setup_cloudfront() {
  command -v aws >/dev/null || { echo "错误: 未安装 aws CLI" >&2; exit 1; }

  echo "==> 创建 S3 bucket: $BUCKET (region: $REGION)"
  aws s3 mb "s3://$BUCKET" --region "$REGION" 2>/dev/null || err "bucket $BUCKET 已存在"

  if [[ -n "${CDN_OAI_ID:-}" ]] && aws cloudfront get-cloud-front-origin-access-identity --id "$CDN_OAI_ID" --region us-east-1 >/dev/null 2>&1; then
    echo "==> 复用已有 OAI: $CDN_OAI_ID"
  else
    echo "==> 创建 CloudFront Origin Access Identity"
    CDN_OAI_ID=$(aws cloudfront create-cloud-front-origin-access-identity \
      --region us-east-1 \
      --cloud-front-origin-access-identity-config \
      "CallerReference=open-travel-oai-$(date +%s),Comment=open-travel-static" \
      --query 'CloudFrontOriginAccessIdentity.Id' --output text)
    echo "    OAI ID: $CDN_OAI_ID"
  fi

  echo "==> 设置 bucket policy（仅允许 OAI 读取）"
  POLICY=$(mktemp)
  cat > "$POLICY" <<EOF
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Principal": { "AWS": "arn:aws:iam::cloudfront:user/CloudFront Origin Access Identity $CDN_OAI_ID" },
    "Action": "s3:GetObject",
    "Resource": "arn:aws:s3:::$BUCKET/*"
  }]
}
EOF
  aws s3api put-bucket-policy --bucket "$BUCKET" --policy "file://$POLICY" --region "$REGION"
  rm -f "$POLICY"

  if [[ -n "${CDN_DIST_ID:-}" ]] && aws cloudfront get-distribution --id "$CDN_DIST_ID" --region us-east-1 >/dev/null 2>&1; then
    echo "==> 复用已有 Distribution: $CDN_DIST_ID"
    echo "    CDN 域名: $(aws cloudfront get-distribution --id "$CDN_DIST_ID" --region us-east-1 --query 'Distribution.DomainName' --output text)"
    save_state
    return
  fi

  echo "==> 创建 CloudFront Distribution"
  if [[ -n "$DOMAIN" ]]; then
    ALIASES_JSON="\"Aliases\": { \"Quantity\": 1, \"Items\": [\"$DOMAIN\"] }"
    CERT_JSON="\"ViewerCertificate\": { \"ACMCertificateArn\": \"$CERT_ARN\", \"SSLSupportMethod\": \"sni-only\", \"MinimumProtocolVersion\": \"TLSv1.2_2021\" }"
  else
    ALIASES_JSON="\"Aliases\": { \"Quantity\": 0, \"Items\": [] }"
    CERT_JSON="\"ViewerCertificate\": { \"CloudFrontDefaultCertificate\": true }"
  fi

  CFG=$(mktemp)
  cat > "$CFG" <<EOF
{
  "CallerReference": "open-travel-cdn-$(date +%s)",
  "Comment": "open-travel static CDN",
  "Enabled": true,
  "DefaultRootObject": "index.html",
  "PriceClass": "PriceClass_100",
  $ALIASES_JSON,
  "Origins": {
    "Quantity": 1,
    "Items": [{
      "Id": "S3-$BUCKET",
      "DomainName": "$BUCKET.s3.$REGION.amazonaws.com",
      "OriginPath": "",
      "S3OriginConfig": { "OriginAccessIdentity": "origin-access-identity/cloudfront/$CDN_OAI_ID" }
    }]
  },
  "DefaultCacheBehavior": {
    "TargetOriginId": "S3-$BUCKET",
    "ViewerProtocolPolicy": "redirect-to-https",
    "Compress": true,
    "MinTTL": 86400,
    "DefaultTTL": 86400,
    "MaxTTL": 31536000,
    "ForwardedValues": { "QueryString": false, "Cookies": { "Forward": "none" }, "Headers": { "Quantity": 0, "Items": [] } },
    "TrustedSigners": { "Enabled": false, "Quantity": 0 },
    "AllowedMethods": { "Quantity": 2, "Items": ["GET", "HEAD"] }
  },
  "CacheBehaviors": {
    "Quantity": 7,
    "Items": [
      {
        "PathPattern": "/api/*",
        "TargetOriginId": "S3-$BUCKET",
        "ViewerProtocolPolicy": "https-only",
        "Compress": false,
        "MinTTL": 0, "DefaultTTL": 0, "MaxTTL": 0,
        "ForwardedValues": { "QueryString": true, "Cookies": { "Forward": "all" }, "Headers": { "Quantity": 0, "Items": [] } },
        "TrustedSigners": { "Enabled": false, "Quantity": 0 },
        "AllowedMethods": { "Quantity": 2, "Items": ["GET", "HEAD"] }
      },
      {
        "PathPattern": "*.png",
        "TargetOriginId": "S3-$BUCKET",
        "ViewerProtocolPolicy": "redirect-to-https",
        "Compress": true,
        "MinTTL": 2592000, "DefaultTTL": 2592000, "MaxTTL": 31536000,
        "ForwardedValues": { "QueryString": false, "Cookies": { "Forward": "none" }, "Headers": { "Quantity": 0, "Items": [] } },
        "TrustedSigners": { "Enabled": false, "Quantity": 0 },
        "AllowedMethods": { "Quantity": 2, "Items": ["GET", "HEAD"] }
      },
      {
        "PathPattern": "*.jpg",
        "TargetOriginId": "S3-$BUCKET",
        "ViewerProtocolPolicy": "redirect-to-https",
        "Compress": true,
        "MinTTL": 2592000, "DefaultTTL": 2592000, "MaxTTL": 31536000,
        "ForwardedValues": { "QueryString": false, "Cookies": { "Forward": "none" }, "Headers": { "Quantity": 0, "Items": [] } },
        "TrustedSigners": { "Enabled": false, "Quantity": 0 },
        "AllowedMethods": { "Quantity": 2, "Items": ["GET", "HEAD"] }
      },
      {
        "PathPattern": "*.jpeg",
        "TargetOriginId": "S3-$BUCKET",
        "ViewerProtocolPolicy": "redirect-to-https",
        "Compress": true,
        "MinTTL": 2592000, "DefaultTTL": 2592000, "MaxTTL": 31536000,
        "ForwardedValues": { "QueryString": false, "Cookies": { "Forward": "none" }, "Headers": { "Quantity": 0, "Items": [] } },
        "TrustedSigners": { "Enabled": false, "Quantity": 0 },
        "AllowedMethods": { "Quantity": 2, "Items": ["GET", "HEAD"] }
      },
      {
        "PathPattern": "*.webp",
        "TargetOriginId": "S3-$BUCKET",
        "ViewerProtocolPolicy": "redirect-to-https",
        "Compress": true,
        "MinTTL": 2592000, "DefaultTTL": 2592000, "MaxTTL": 31536000,
        "ForwardedValues": { "QueryString": false, "Cookies": { "Forward": "none" }, "Headers": { "Quantity": 0, "Items": [] } },
        "TrustedSigners": { "Enabled": false, "Quantity": 0 },
        "AllowedMethods": { "Quantity": 2, "Items": ["GET", "HEAD"] }
      },
      {
        "PathPattern": "*.gif",
        "TargetOriginId": "S3-$BUCKET",
        "ViewerProtocolPolicy": "redirect-to-https",
        "Compress": true,
        "MinTTL": 2592000, "DefaultTTL": 2592000, "MaxTTL": 31536000,
        "ForwardedValues": { "QueryString": false, "Cookies": { "Forward": "none" }, "Headers": { "Quantity": 0, "Items": [] } },
        "TrustedSigners": { "Enabled": false, "Quantity": 0 },
        "AllowedMethods": { "Quantity": 2, "Items": ["GET", "HEAD"] }
      },
      {
        "PathPattern": "*.svg",
        "TargetOriginId": "S3-$BUCKET",
        "ViewerProtocolPolicy": "redirect-to-https",
        "Compress": true,
        "MinTTL": 2592000, "DefaultTTL": 2592000, "MaxTTL": 31536000,
        "ForwardedValues": { "QueryString": false, "Cookies": { "Forward": "none" }, "Headers": { "Quantity": 0, "Items": [] } },
        "TrustedSigners": { "Enabled": false, "Quantity": 0 },
        "AllowedMethods": { "Quantity": 2, "Items": ["GET", "HEAD"] }
      }
    ]
  },
  $CERT_JSON,
  "Logging": { "Enabled": false, "Bucket": "", "Prefix": "" },
  "CustomErrorResponses": { "Quantity": 0, "Items": [] }
}
EOF
  OUT=$(aws cloudfront create-distribution --region us-east-1 --distribution-config "file://$CFG" \
    --query 'Distribution.{Id:Id,DomainName:DomainName}' --output text)
  rm -f "$CFG"
  read -r CDN_DIST_ID CDN_DOMAIN <<< "$OUT"
  save_state
  echo "    Distribution ID: $CDN_DIST_ID"
  echo "    CDN 域名: $CDN_DOMAIN"
  echo "    CNAME 接入: ${DOMAIN:-（未配置自定义域名）}"
  echo "    注意: Distribution 部署需 5-10 分钟生效。"
}

setup_oss() {
  command -v aliyun >/dev/null || { echo "错误: 未安装 aliyun CLI" >&2; exit 1; }

  echo "==> 创建 OSS bucket: $BUCKET"
  aliyun oss mb "oss://$BUCKET" ${ENDPOINT:+-e "$ENDPOINT"} 2>/dev/null || err "bucket $BUCKET 已存在"

  cat <<EOF
==> 阿里云 CDN（不在本脚本范围，需在控制台配置）:
    1. 控制台 -> CDN -> 添加域名：源站填 oss://$BUCKET（私有读）
    2. 缓存配置：
       - 默认缓存：静态文件 TTL 1 天（1d+）
       - *.png *.jpg *.jpeg *.webp *.gif *.svg：30 天
       - /api/*：0 秒（不缓存）
    3. 开启 gzip 压缩（CDN 控制台 -> 性能优化）
    4. 域名接入：CNAME 记录指向 CDN 分配的 CNAME 域名
EOF
}

case "$PROVIDER" in
  cloudfront) setup_cloudfront ;;
  oss)        setup_oss ;;
esac

echo "完成。状态文件: $STATE_FILE"
