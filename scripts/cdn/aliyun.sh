# 阿里云 CDN provider：OSS bucket 作源站 + 控制台配 CDN。
# 由 scripts/cdn_setup.sh source 后调用：cdn_require_creds / cdn_setup / cdn_upload <src> <prefix> <ttl>
# 凭据仅从环境变量读取：ALIBABA_CLOUD_ACCESS_KEY_ID / ALIBABA_CLOUD_ACCESS_KEY_SECRET（ossutil 官方约定）

# 检查 ossutil 与 AK/SK 环境变量；缺哪个打印哪个。返回 1 时调度器自动切 DRY-RUN。
cdn_require_creds() {
  local ok=0
  command -v ossutil >/dev/null 2>&1 || { echo "缺少: ossutil 未安装（pip install oss2 或下载官方二进制）" >&2; ok=1; }
  [[ -n "${ALIBABA_CLOUD_ACCESS_KEY_ID:-}" ]] || { echo "缺少: 环境变量 ALIBABA_CLOUD_ACCESS_KEY_ID" >&2; ok=1; }
  [[ -n "${ALIBABA_CLOUD_ACCESS_KEY_SECRET:-}" ]] || { echo "缺少: 环境变量 ALIBABA_CLOUD_ACCESS_KEY_SECRET" >&2; ok=1; }
  return $ok
}

# ossutil 需要完整 endpoint；未给 --endpoint 时由 REGION 推导（如 oss-cn-hangzhou.aliyuncs.com）
_aliyun_endpoint() {
  echo "${ENDPOINT:-oss-${REGION}.aliyuncs.com}"
}

# 幂等创建 bucket；存在则跳过。创建后打印控制台指引（阿里云 CDN 无完整 CLI 流程）。
cdn_setup() {
  local ep
  ep="$(_aliyun_endpoint)"

  # 检查已存在：ossutil ls oss://BUCKET 退出码 0 = 存在，跳过
  if [[ $DRY_RUN -eq 1 ]]; then
    echo "==> [DRY-RUN] ossutil ls oss://$BUCKET --endpoint $ep"
    echo "==> [DRY-RUN] ossutil mb oss://$BUCKET --endpoint $ep"
  elif ossutil ls "oss://$BUCKET" --endpoint "$ep" >/dev/null 2>&1; then
    echo "==> bucket 已存在，跳过创建: $BUCKET"
  else
    echo "==> 创建 OSS bucket: $BUCKET"
    ossutil mb "oss://$BUCKET" --endpoint "$ep"
  fi

  save_state "CDN_BUCKET=$BUCKET"
  save_state "CDN_ENDPOINT=$ep"

  cat <<EOF
==> 阿里云 CDN（无完整 CLI 流程，请在控制台配置）:
    1. 控制台 -> CDN -> 添加域名${DOMAIN:+：$DOMAIN}，源站填 oss://$BUCKET（建议私有读）
    2. 缓存配置：
       - 默认缓存：静态文件 TTL 1 天（1d+）
       - *.png *.jpg *.jpeg *.webp *.gif *.svg：30 天
       - /api/*：0 秒（不缓存）
    3. 开启 gzip 压缩（CDN 控制台 -> 性能优化）
    4. 域名接入：${DOMAIN:+给 $DOMAIN 加 }CNAME 记录指向 CDN 分配的 CNAME 域名
EOF
}

# 上传文件或目录到 oss://$BUCKET/$prefix/，带 Cache-Control: max-age=$ttl（秒）。
# 目录用 --update 只传变更文件，幂等可重跑。
cdn_upload() {
  local src="$1" prefix="$2" ttl="$3"
  prefix="${prefix%/}"
  local ep args=()
  ep="$(_aliyun_endpoint)"
  args=(cp --meta "Cache-Control:max-age=$ttl" --endpoint "$ep")

  if [[ -d "$src" ]]; then
    args+=(-r "$src" "oss://$BUCKET/$prefix/" --update)
  else
    args+=("$src" "oss://$BUCKET/$prefix/")
  fi

  if [[ $DRY_RUN -eq 1 ]]; then
    echo "==> [DRY-RUN] ossutil ${args[*]}"
  else
    ossutil "${args[@]}"
  fi
}
