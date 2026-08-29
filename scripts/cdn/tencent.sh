#!/usr/bin/env bash
# 腾讯云 CDN provider：COS bucket 作源站 + CDN 域名（tccli 配置）。
# 由 scripts/cdn_setup.sh / cdn_upload.sh source 后调用：cdn_require_creds / cdn_setup / cdn_upload <src> <prefix> <ttl>
# 凭据仅从环境变量读取：TENCENTCLOUD_SECRET_ID / TENCENTCLOUD_SECRET_KEY（tccli/coscmd 官方约定），
# TENCENTCLOUD_APP_ID 可选（COS bucket 名规范为 <name>-<APPID>，设置后自动拼接）
# 调度器环境变量：BUCKET REGION(默认 ap-guangzhou) DOMAIN DRY_RUN(1=只打印) STATE_FILE

# 检查 coscmd/tccli 与 AK/SK 环境变量；缺哪个打印哪个。返回 1 时调度器自动切 DRY-RUN。
cdn_require_creds() {
  local ok=0
  command -v coscmd >/dev/null 2>&1 || { echo "缺少: coscmd 未安装（pip install coscmd）" >&2; ok=1; }
  command -v tccli >/dev/null 2>&1 || { echo "缺少: tccli 未安装（pip install tccli）" >&2; ok=1; }
  [[ -n "${TENCENTCLOUD_SECRET_ID:-}" ]] || { echo "缺少: 环境变量 TENCENTCLOUD_SECRET_ID" >&2; ok=1; }
  [[ -n "${TENCENTCLOUD_SECRET_KEY:-}" ]] || { echo "缺少: 环境变量 TENCENTCLOUD_SECRET_KEY" >&2; ok=1; }
  return $ok
}

# COS bucket 名规范为 <name>-<APPID>：设了 TENCENTCLOUD_APP_ID 则拼接，否则用 BUCKET 原样（指引中提示）。
_cos_bucket() {
  if [[ -n "${TENCENTCLOUD_APP_ID:-}" ]]; then
    echo "${BUCKET}-${TENCENTCLOUD_APP_ID}"
  else
    echo "$BUCKET"
  fi
}

# 状态持久化：向 $STATE_FILE 追加一行 KEY=VALUE（纯文本，可被 source 读回）。
# 调度器（cdn_setup.sh）已定义则直接用；未定义才自行定义；DRY-RUN 不写状态（占位值会污染真实状态文件）。
if ! declare -F save_state >/dev/null 2>&1; then
  save_state() {
    [[ $DRY_RUN -eq 1 ]] && return 0
    printf '%s\n' "$1" >> "$STATE_FILE"
  }
fi

# coscmd 每次调用前须先 config（写 ~/.cos.conf，腾讯官方约定：AK/SK、bucket、region 存于该配置文件）。
# 统一入口：DRY_RUN 只打印 config + 命令预览，绝不执行真实命令。
_run_cos() {
  if [[ $DRY_RUN -eq 1 ]]; then
    echo "# dry-run: coscmd config -a ${TENCENTCLOUD_SECRET_ID:-<SECRET_ID>} -s ${TENCENTCLOUD_SECRET_KEY:-<SECRET_KEY>} -b $COS_BUCKET -r $REGION"
    printf '# dry-run: coscmd'
    printf ' %q' "$@"
    printf '\n'
    return 0
  fi
  coscmd config -a "$TENCENTCLOUD_SECRET_ID" -s "$TENCENTCLOUD_SECRET_KEY" -b "$COS_BUCKET" -r "$REGION"
  coscmd "$@"
}

# 通用执行（tccli 无 config 步骤）；DRY_RUN 只打印。
_run_tc() {
  if [[ $DRY_RUN -eq 1 ]]; then
    printf '# dry-run:'
    printf ' %q' "$@"
    printf '\n'
    return 0
  fi
  "$@"
}

# 幂等配置：COS bucket（不存在则建）→ CDN 域名（有 DOMAIN 则探测/添加，否则打印控制台指引）。
cdn_setup() {
  [[ -n "${BUCKET:-}" ]] || { echo "错误: cdn_setup 需要 BUCKET" >&2; return 1; }
  REGION="${REGION:-ap-guangzhou}"
  DRY_RUN="${DRY_RUN:-0}"
  [[ -n "${STATE_FILE:-}" && -f "$STATE_FILE" ]] && source "$STATE_FILE"
  local COS_BUCKET COS_ORIGIN
  COS_BUCKET="$(_cos_bucket)"
  COS_ORIGIN="$COS_BUCKET.cos.$REGION.myqcloud.com"

  # 1. COS bucket：coscmd list 探活，不存在才创建（探测/创建前均先 config）
  echo "==> 检查 COS bucket: $COS_BUCKET (region: $REGION)"
  if [[ $DRY_RUN -eq 1 ]]; then
    _run_cos list
    _run_cos createbucket
  elif _run_cos list >/dev/null 2>&1; then
    echo "==> bucket 已存在，跳过创建: $COS_BUCKET"
  else
    echo "==> 创建 COS bucket: $COS_BUCKET"
    _run_cos createbucket
  fi
  save_state "CDN_COS_BUCKET=$COS_BUCKET"
  save_state "CDN_REGION=$REGION"

  # 2. CDN：提供 DOMAIN 则 tccli 探测/添加；否则控制台指引
  if [[ -n "${DOMAIN:-}" ]]; then
    echo "==> 配置 CDN 域名: $DOMAIN -> 源站 $COS_ORIGIN"
    if [[ $DRY_RUN -eq 1 ]]; then
      _run_tc tccli cdn DescribeDomains --Domain "$DOMAIN"
      _run_tc tccli cdn AddDomain --Domain "$DOMAIN" --Origin "{\"Origins\":[\"$COS_ORIGIN\"],\"OriginType\":\"cos\"}"
    elif tccli cdn DescribeDomains --Domain "$DOMAIN" 2>/dev/null | grep -q "\"Domain\": \"$DOMAIN\""; then
      echo "==> CDN 域名已存在，跳过: $DOMAIN"
    else
      _run_tc tccli cdn AddDomain --Domain "$DOMAIN" --Origin "{\"Origins\":[\"$COS_ORIGIN\"],\"OriginType\":\"cos\"}"
    fi
  else
    if [[ -z "${TENCENTCLOUD_APP_ID:-}" ]]; then
      echo "==> 提示: COS bucket 名规范为 <name>-<APPID>，建议设置 TENCENTCLOUD_APP_ID 后使用 $BUCKET-<APPID>"
    fi
    cat <<EOF
==> 腾讯云 CDN（未提供 --domain，请在控制台配置）:
    1. 控制台 -> CDN -> 添加域名，源站类型选 COS，源站填 $COS_ORIGIN
    2. 缓存配置：
       - 默认缓存：静态文件 TTL 1 天
       - *.png *.jpg *.jpeg *.webp *.gif *.svg：30 天
       - /api/*：0 秒（不缓存）
    3. 开启 gzip 压缩（CDN 控制台 -> 性能优化）
    4. 域名接入：给自定义域名加 CNAME 记录，指向 CDN 分配的 CNAME 域名
EOF
  fi
}

# 上传文件或目录到 COS 的 $prefix/ 下（目录用 -r 递归，保留相对路径），带 Cache-Control: max-age=$ttl（秒）。
cdn_upload() {
  local src="$1" prefix="$2" ttl="$3"
  prefix="${prefix%/}"
  REGION="${REGION:-ap-guangzhou}"
  DRY_RUN="${DRY_RUN:-0}"
  local COS_BUCKET header
  COS_BUCKET="$(_cos_bucket)"
  header="{\"Cache-Control\": \"max-age=$ttl\"}"
  if [[ -d "$src" ]]; then
    _run_cos upload -r "$src" "$prefix/" -H "$header"
  else
    _run_cos upload "$src" "$prefix/" -H "$header"
  fi
}
