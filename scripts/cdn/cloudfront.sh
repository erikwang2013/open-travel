# AWS CloudFront provider：S3 bucket 作源站 + CloudFront 分发。
# 由 scripts/cdn_setup.sh source 后调用：cdn_require_creds / cdn_setup / cdn_upload <src> <prefix> <ttl>
# 凭据仅从 AWS CLI 默认链读取（~/.aws/credentials + ~/.aws/config，或 AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY/AWS_DEFAULT_REGION 环境变量）
# 调度器环境变量：BUCKET REGION(默认 us-east-1) DOMAIN CERT_ARN DRY_RUN(1=只打印) STATE_FILE

# 检查 aws CLI 与凭据/region 配置；缺哪个打印哪个。返回 1 时调度器自动切 DRY-RUN。
cdn_require_creds() {
  local ok=0
  command -v aws >/dev/null 2>&1 || { echo "缺少: aws CLI 未安装（pip install awscli 或官方安装包）" >&2; ok=1; }
  # 凭据：~/.aws/credentials 文件，或 AK/SK 环境变量
  if [[ ! -f "$HOME/.aws/credentials" ]] && { [[ -z "${AWS_ACCESS_KEY_ID:-}" || -z "${AWS_SECRET_ACCESS_KEY:-}" ]]; }; then
    echo "缺少: ~/.aws/credentials 或环境变量 AWS_ACCESS_KEY_ID + AWS_SECRET_ACCESS_KEY" >&2
    ok=1
  fi
  # region：环境变量或 ~/.aws/config
  if [[ -z "${AWS_DEFAULT_REGION:-}" && -z "${AWS_REGION:-}" ]] && ! grep -q '^\s*region\s*=' "$HOME/.aws/config" 2>/dev/null; then
    echo "缺少: 环境变量 AWS_DEFAULT_REGION 或 ~/.aws/config 中的 region" >&2
    ok=1
  fi
  return $ok
}

# 状态持久化：向 $STATE_FILE 追加一行 KEY=VALUE（纯文本，可被 source 读回）。
# DRY-RUN 不写状态（占位 ID 会污染真实状态文件）。
save_state() {
  [[ $DRY_RUN -eq 1 ]] && return
  printf '%s\n' "$1" >> "$STATE_FILE"
}

# 执行命令；DRY_RUN=1 时只打印（# dry-run: 前缀），绝不执行。
_run() {
  if [[ $DRY_RUN -eq 1 ]]; then
    printf '# dry-run:'
    printf ' %q' "$@"
    printf '\n'
    return 0
  fi
  "$@"
}

# 幂等配置：bucket（不存在则建）→ OAI（无则建）→ bucket policy（仅 OAI 可读）→ distribution（有状态则复用）。
cdn_setup() {
  [[ -n "${BUCKET:-}" ]] || { echo "错误: cdn_setup 需要 BUCKET" >&2; return 1; }
  [[ -n "${STATE_FILE:-}" ]] || { echo "错误: cdn_setup 需要 STATE_FILE" >&2; return 1; }
  REGION="${REGION:-us-east-1}"
  DRY_RUN="${DRY_RUN:-0}"
  [[ -f "$STATE_FILE" ]] && source "$STATE_FILE"

  # 1. S3 bucket：head-bucket 探活，不存在才创建
  if [[ $DRY_RUN -eq 1 ]]; then
    echo "==> [DRY-RUN] aws s3api head-bucket --bucket $BUCKET --region $REGION"
    echo "==> [DRY-RUN] aws s3 mb s3://$BUCKET --region $REGION"
  elif aws s3api head-bucket --bucket "$BUCKET" --region "$REGION" >/dev/null 2>&1; then
    echo "==> bucket 已存在，跳过创建: $BUCKET"
  else
    echo "==> 创建 S3 bucket: $BUCKET (region: $REGION)"
    _run aws s3 mb "s3://$BUCKET" --region "$REGION"
  fi

  # 2. OAI：状态里有且可用则复用，否则创建
  if [[ -n "${CDN_OAI_ID:-}" ]]; then
    echo "==> 复用已有 OAI: $CDN_OAI_ID"
  else
    echo "==> 创建 CloudFront Origin Access Identity"
    if [[ $DRY_RUN -eq 1 ]]; then
      echo "# dry-run: aws cloudfront create-cloud-front-origin-access-identity --region us-east-1 --cloud-front-origin-access-identity-config CallerReference=open-travel-oai-<时间戳>,Comment=open-travel-static"
      CDN_OAI_ID="__DRY_RUN_OAI__"
    else
      CDN_OAI_ID="$(aws cloudfront create-cloud-front-origin-access-identity --region us-east-1 \
        --cloud-front-origin-access-identity-config "CallerReference=open-travel-oai-$(date +%s),Comment=open-travel-static" \
        --query 'CloudFrontOriginAccessIdentity.Id' --output text)"
    fi
    echo "    OAI ID: $CDN_OAI_ID"
    save_state "CDN_OAI_ID=$CDN_OAI_ID"
  fi

  # 3. bucket policy：仅允许该 OAI 读取
  echo "==> 设置 bucket policy（仅允许 OAI 读取）"
  local policy
  policy="$(mktemp)"
  cat > "$policy" <<EOF
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
  _run aws s3api put-bucket-policy --bucket "$BUCKET" --policy "file://$policy" --region "$REGION"
  rm -f "$policy"

  # 4. Distribution：状态已有则复用
  if [[ -n "${CDN_DIST_ID:-}" ]]; then
    echo "已存在，复用: Distribution $CDN_DIST_ID（如需重建请清空状态文件）"
    return 0
  fi

  echo "==> 创建 CloudFront Distribution"
  if [[ -n "${DOMAIN:-}" ]]; then
    ALIASES_JSON="\"Aliases\": { \"Quantity\": 1, \"Items\": [\"$DOMAIN\"] }"
    CERT_JSON="\"ViewerCertificate\": { \"ACMCertificateArn\": \"$CERT_ARN\", \"SSLSupportMethod\": \"sni-only\", \"MinimumProtocolVersion\": \"TLSv1.2_2021\" }"
  else
    ALIASES_JSON="\"Aliases\": { \"Quantity\": 0, \"Items\": [] }"
    CERT_JSON="\"ViewerCertificate\": { \"CloudFrontDefaultCertificate\": true }"
  fi

  local cfg
  cfg="$(mktemp)"
  cat > "$cfg" <<EOF
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
    "MinTTL": 86400, "DefaultTTL": 86400, "MaxTTL": 31536000,
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

  if [[ $DRY_RUN -eq 1 ]]; then
    echo "# dry-run: aws cloudfront create-distribution --region us-east-1 --distribution-config file://$cfg --query Distribution.Id"
    CDN_DIST_ID=__DRY_RUN_DIST__
  else
    CDN_DIST_ID="$(aws cloudfront create-distribution --region us-east-1 --distribution-config "file://$cfg" \
      --query 'Distribution.Id' --output text)"
    echo "    Distribution ID: $CDN_DIST_ID"
    echo "    CDN 域名: $(aws cloudfront get-distribution --id "$CDN_DIST_ID" --region us-east-1 --query 'Distribution.DomainName' --output text)"
    echo "    CNAME 接入: ${DOMAIN:-（未配置自定义域名）}"
    echo "    注意: Distribution 部署需 5-10 分钟生效。"
  fi
  rm -f "$cfg"
  save_state "CDN_DIST_ID=$CDN_DIST_ID"
}

# 上传文件（aws s3 cp）或目录（aws s3 sync，保留相对路径）到 s3://$BUCKET/$prefix/，Cache-Control: public, max-age=$ttl（秒）。
cdn_upload() {
  local src="$1" prefix="$2" ttl="$3" dest
  dest="s3://$BUCKET/${prefix%/}"
  REGION="${REGION:-us-east-1}"
  DRY_RUN="${DRY_RUN:-0}"
  if [[ -d "$src" ]]; then
    _run aws s3 sync "$src" "$dest/" --region "$REGION" --cache-control "public, max-age=$ttl"
  else
    _run aws s3 cp "$src" "$dest/" --region "$REGION" --cache-control "public, max-age=$ttl"
  fi
}
