#!/usr/bin/env bash
# Cloudflare R2 provider：R2 bucket 作源站 + 自定义域名 Cache Rules 做 CDN 缓存。
# 由 scripts/cdn_setup.sh source 后调用：cdn_require_creds / cdn_setup / cdn_upload <src> <prefix> <ttl>
# 凭据仅从环境变量读取：CLOUDFLARE_API_TOKEN + CLOUDFLARE_ACCOUNT_ID（必填）；CLOUDFLARE_ZONE_ID（绑定自定义域名时需要）
# 调度器环境变量：BUCKET REGION(不适用，R2 无区域概念) DOMAIN DRY_RUN(1=只打印) STATE_FILE

# 检查 wrangler CLI 与凭据；缺哪个打印哪个。返回 1 时调度器自动切 DRY-RUN。
cdn_require_creds() {
  local ok=0
  command -v wrangler >/dev/null 2>&1 || { echo "缺少: wrangler 未安装（npm install -g @cloudflare/wrangler）" >&2; ok=1; }
  [[ -n "${CLOUDFLARE_API_TOKEN:-}" ]] || { echo "缺少: 环境变量 CLOUDFLARE_API_TOKEN" >&2; ok=1; }
  [[ -n "${CLOUDFLARE_ACCOUNT_ID:-}" ]] || { echo "缺少: 环境变量 CLOUDFLARE_ACCOUNT_ID" >&2; ok=1; }
  # ZONE_ID 仅在绑定自定义域名时需要，缺失不阻断
  [[ -n "${CLOUDFLARE_ZONE_ID:-}" ]] || echo "提示: CLOUDFLARE_ZONE_ID 未设置（绑定自定义域名时需在控制台指定 zone）" >&2
  return $ok
}

# 状态持久化：向 $STATE_FILE 追加一行 KEY=VALUE（纯文本，可被 source 读回）。
# DRY-RUN 不写状态（占位值会污染真实状态文件）。dispatcher 已定义则直接沿用。
if ! declare -F save_state >/dev/null 2>&1; then
  save_state() {
    [[ $DRY_RUN -eq 1 ]] && return
    printf '%s\n' "$1" >> "$STATE_FILE"
  }
fi

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

# 幂等配置：bucket 不存在则创建。R2 本身不缓存，CDN 缓存靠自定义域名 + Cache Rules，
# 因此创建完 bucket 后打印控制台启用指引。
cdn_setup() {
  [[ -n "${BUCKET:-}" ]] || { echo "错误: cdn_setup 需要 BUCKET" >&2; return 1; }
  [[ -n "${STATE_FILE:-}" ]] || { echo "错误: cdn_setup 需要 STATE_FILE" >&2; return 1; }
  DRY_RUN="${DRY_RUN:-0}"
  DOMAIN="${DOMAIN:-}"

  # 1. R2 bucket：bucket list 查存在，不存在才创建
  if [[ $DRY_RUN -eq 1 ]]; then
    echo "==> [DRY-RUN] wrangler r2 bucket list（检查 $BUCKET 是否存在）"
    echo "==> [DRY-RUN] wrangler r2 bucket create $BUCKET"
  elif wrangler r2 bucket list 2>/dev/null | grep -qw "$BUCKET"; then
    echo "==> bucket 已存在，跳过创建: $BUCKET"
  else
    echo "==> 创建 R2 bucket: $BUCKET"
    _run wrangler r2 bucket create "$BUCKET"
  fi

  save_state "CDN_R2_BUCKET=$BUCKET"

  # 2. CDN 启用指引（R2 无 CLI 缓存配置，全部走控制台）
  cat <<EOF
==> Cloudflare CDN 启用指引（R2 不缓存，需在控制台配置）:
    1. 控制台 -> R2 -> $BUCKET -> 设置 -> 自定义域${DOMAIN:+：$DOMAIN}
       （域名需在同一个 Cloudflare 账户，且 CLOUDFLARE_ZONE_ID 指向该 zone）
       临时验证可先用 R2 分配的 r2.dev 子域（仅限测试，生产用自定义域名）
    2. 自定义域生效后，配置 Cache Rules（控制台 -> 规则 -> Cache Rules）:
       - *.png *.jpg *.jpeg *.webp *.gif *.svg：缓存 30 天
       - 默认缓存：1 天
       - /api/*：0 秒（不缓存，若需直通源站）
    3. 验证: curl -I https://${DOMAIN:-<你的域名>}/xxx（看 cf-cache-status: HIT）
EOF
}

# 上传文件或目录到 <bucket>/<prefix>/。目录用 find 逐个 put，保留相对路径。
# R2 对象的浏览器缓存由自定义域名 Cache Rules 控制（非对象 meta），ttl 仅打印指引；
# --metadata 里仍带上 Cache-Control 供 S3 兼容客户端参考。
cdn_upload() {
  local src="$1" prefix="$2" ttl="$3"
  prefix="${prefix%/}"
  local meta="{\"Cache-Control\":\"public, max-age=$ttl\"}"
  echo "==> 提示: R2 缓存由 Cache Rules 控制（TTL=${ttl}s），对象级 Cache-Control 仅作元数据"

  if [[ -d "$src" ]]; then
    local f rel
    while IFS= read -r -d '' f; do
      rel="${f#"$src"/}"
      _run wrangler r2 object put "$BUCKET/$prefix/$rel" --file "$f" --metadata "$meta"
    done < <(find "$src" -type f -print0)
  else
    _run wrangler r2 object put "$BUCKET/$prefix/$(basename "$src")" --file "$src" --metadata "$meta"
  fi
}
