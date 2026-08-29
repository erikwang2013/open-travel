#!/usr/bin/env bash
# 华为云 CDN provider：OBS bucket 作源站 + 控制台配 CDN。
# 由 scripts/cdn_setup.sh source 后调用：cdn_require_creds / cdn_setup / cdn_upload <src> <prefix> <ttl>
# 凭据仅从环境变量读取：OBS_ACCESS_KEY_ID / OBS_SECRET_ACCESS_KEY（obsutil 每次调用带 -i/-k，不改全局 config）

# 检查 obsutil 与 AK/SK 环境变量；缺哪个打印哪个。返回 1 时调度器自动切 DRY-RUN。
cdn_require_creds() {
  local ok=0
  command -v obsutil >/dev/null 2>&1 || { echo "缺少: obsutil 未安装（华为云官方 OBS 工具，下载二进制：support.huaweicloud.com/obsutil）" >&2; ok=1; }
  [[ -n "${OBS_ACCESS_KEY_ID:-}" ]] || { echo "缺少: 环境变量 OBS_ACCESS_KEY_ID" >&2; ok=1; }
  [[ -n "${OBS_SECRET_ACCESS_KEY:-}" ]] || { echo "缺少: 环境变量 OBS_SECRET_ACCESS_KEY" >&2; ok=1; }
  return $ok
}

# 状态持久化：dispatcher 已定义则直接用；未定义才自行定义（DRY_RUN 不写状态）。
if ! declare -F save_state >/dev/null 2>&1; then
  save_state() {
    [[ $DRY_RUN -eq 1 ]] && return 0
    printf '%s\n' "$1" >> "$STATE_FILE"
  }
fi

# obsutil 需要完整 endpoint；由 REGION 推导，如 obs.cn-north-4.myhuaweicloud.com
_huawei_endpoint() {
  echo "obs.${REGION}.myhuaweicloud.com"
}

# 幂等创建 OBS bucket；存在则跳过。创建后打印华为云 CDN 控制台指引（无实用 CLI）。
cdn_setup() {
  [[ -n "${BUCKET:-}" ]] || { echo "错误: cdn_setup 需要 BUCKET" >&2; return 1; }
  local ep key secret
  ep="$(_huawei_endpoint)"
  key="${OBS_ACCESS_KEY_ID:-}"
  secret="${OBS_SECRET_ACCESS_KEY:-}"

  # dispatcher 默认 REGION=us-east-1 不适用于华为云，提示用户显式传 --region
  if [[ "${REGION:-us-east-1}" == us-east-1 ]]; then
    echo "==> 提示: REGION 为默认值 us-east-1，不适用于华为云。请用 --region 指定区域（如 cn-north-4）。"
  fi

  # 检查已存在：obsutil ls obs://BUCKET 退出码 0 = 存在，跳过
  if [[ $DRY_RUN -eq 1 ]]; then
    echo "==> [DRY-RUN] obsutil ls obs://$BUCKET -e $ep -i ${key:-__ACCESS_KEY__} -k ${secret:-__SECRET_KEY__}"
    echo "==> [DRY-RUN] obsutil mb obs://$BUCKET -e $ep -i ${key:-__ACCESS_KEY__} -k ${secret:-__SECRET_KEY__}"
  elif obsutil ls "obs://$BUCKET" -e "$ep" -i "$key" -k "$secret" >/dev/null 2>&1; then
    echo "==> bucket 已存在，跳过创建: $BUCKET"
  else
    echo "==> 创建 OBS bucket: $BUCKET"
    obsutil mb "obs://$BUCKET" -e "$ep" -i "$key" -k "$secret"
  fi

  save_state "CDN_OBS_BUCKET=$BUCKET"

  cat <<EOF
==> 华为云 CDN（无实用 CLI，请在控制台配置）:
    1. 控制台 -> CDN -> 添加域名${DOMAIN:+：$DOMAIN}，源站类型选 OBS，源站填 $BUCKET.obs.${REGION:-us-east-1}.myhuaweicloud.com
    2. 缓存配置：
       - 默认缓存：静态文件 TTL 1 天
       - *.png *.jpg *.jpeg *.webp *.gif *.svg：30 天
       - /api/*：0 秒（不缓存）
    3. 开启 gzip 压缩（CDN 控制台 -> 性能优化）
    4. 域名接入：${DOMAIN:+给 $DOMAIN 加 }CNAME 记录指向 CDN 分配给你的 CNAME 域名
EOF
}

# 上传文件或目录到 obs://$BUCKET/$prefix/，带 Cache-Control: max-age=$ttl（秒）。
# 目录用 -r 递归（保留相对路径），-f 强制覆盖，可重复执行。
cdn_upload() {
  local src="$1" prefix="$2" ttl="$3"
  prefix="${prefix%/}"
  local ep key secret meta
  ep="$(_huawei_endpoint)"
  key="${OBS_ACCESS_KEY_ID:-}"
  secret="${OBS_SECRET_ACCESS_KEY:-}"
  meta="CacheControl:max-age=$ttl"

  if [[ $DRY_RUN -eq 1 ]]; then
    if [[ -d "$src" ]]; then
      echo "==> [DRY-RUN] obsutil cp -f -r $src obs://$BUCKET/$prefix/ -e $ep -i ${key:-__ACCESS_KEY__} -k ${secret:-__SECRET_KEY__} -meta $meta"
    else
      echo "==> [DRY-RUN] obsutil cp -f $src obs://$BUCKET/$prefix/ -e $ep -i ${key:-__ACCESS_KEY__} -k ${secret:-__SECRET_KEY__} -meta $meta"
    fi
    return 0
  fi

  if [[ -d "$src" ]]; then
    obsutil cp -f -r "$src" "obs://$BUCKET/$prefix/" -e "$ep" -i "$key" -k "$secret" -meta "$meta"
  else
    obsutil cp -f "$src" "obs://$BUCKET/$prefix/" -e "$ep" -i "$key" -k "$secret" -meta "$meta"
  fi
}
