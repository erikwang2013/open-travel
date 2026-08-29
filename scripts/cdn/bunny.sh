# Bunny.net CDN provider：Storage Zone 源站 + Pull Zone CDN（零 CLI 依赖，纯 REST API）。
# 由 scripts/cdn_setup.sh source 后调用：cdn_require_creds / cdn_setup / cdn_upload <src> <prefix> <ttl>
# 环境变量：
#   BUNNY_API_KEY            管理 API Key（bunny.net 控制台账号 API Key，AccessKey header）
#   BUNNY_STORAGE_PASSWORD   Storage Zone 存储密码（存储端点 AccessKey header；未设则 cdn_setup 生成并打印）
#   CDN_STORAGE_ZONE         （可选）Storage Zone 名，覆盖 ${BUCKET}-ot 默认名
# 调度器环境变量：BUCKET DOMAIN(可选) DRY_RUN(1=只打印) STATE_FILE

# 检查 API Key 与存储密码；缺哪个打印哪个。返回 1 时调度器自动切 DRY-RUN。
cdn_require_creds() {
  local ok=0
  command -v curl >/dev/null 2>&1 || { echo "缺少: curl 未安装" >&2; ok=1; }
  [[ -n "${BUNNY_API_KEY:-}" ]] || { echo "缺少: 环境变量 BUNNY_API_KEY（bunny.net 控制台账号 API Key）" >&2; ok=1; }
  [[ -n "${BUNNY_STORAGE_PASSWORD:-}" ]] || { echo "缺少: 环境变量 BUNNY_STORAGE_PASSWORD（Storage Zone 存储密码）" >&2; ok=1; }
  return $ok
}

# 状态持久化：dispatcher 已定义则直接用（DRY_RUN 不写状态）；未定义才自行定义。
if ! declare -F save_state >/dev/null 2>&1; then
  save_state() { [[ ${DRY_RUN:-0} -eq 1 ]] && return 0; printf '%s\n' "$1" >> "$STATE_FILE"; }
fi

# 从 Bunny JSON 数组里按 Name 找对象，输出 "Id|field"（field 如 StoragePassword/Hostname）。
# 解析按对象级 JSON 切分（tr '{'），依赖 Bunny 响应中除 Hostnames 数组外无嵌套对象。
_bunny_find() {
  local json="$1" name="$2" field="$3"
  local chunk id n val
  while IFS= read -r chunk; do
    id="$(printf '%s' "$chunk" | sed -n 's/.*"Id": *\([0-9][0-9]*\).*/\1/p')"
    n="$(printf '%s' "$chunk" | sed -n 's/.*"Name": *"\([^"]*\)".*/\1/p')"
    if [[ -n "$id" && "$n" == "$name" ]]; then
      val="$(printf '%s' "$chunk" | grep -oE "\"$field\": *\"[^\"]*\"" | head -1 | sed -E 's/^[^"]*"([^"]*)"$/\1/')"
      echo "$id|$val"
      return 0
    fi
  done < <(printf '%s' "$json" | tr '{' '\n')
  return 1
}

# 幂等配置：Storage Zone（有则复用，无则建，409 全局重名自动加随机后缀）→ Pull Zone（有则复用，无则建）。
cdn_setup() {
  [[ -n "${BUCKET:-}" ]] || { echo "错误: cdn_setup 需要 BUCKET" >&2; return 1; }
  DRY_RUN="${DRY_RUN:-0}"
  [[ -n "${STATE_FILE:-}" && -f "$STATE_FILE" ]] && source "$STATE_FILE"

  local zone_name="${BUCKET}-ot" pull_name="${BUCKET}-ot-cdn"
  local zone_id="" found="" list="" code="" tmp="" pass=""

  # 1. Storage Zone（幂等）
  if [[ -n "${CDN_STORAGE_ZONE:-}" ]]; then
    echo "==> 复用已有 Storage Zone: $CDN_STORAGE_ZONE（来自状态文件）"
  elif [[ $DRY_RUN -eq 1 ]]; then
    echo "==> [DRY-RUN] GET https://api.bunny.net/storagezone -H \"AccessKey: \$BUNNY_API_KEY\"（按 Name 匹配 $zone_name）"
    echo "==> [DRY-RUN] POST https://api.bunny.net/storagezone -H \"AccessKey: \$BUNNY_API_KEY\" --data {\"Name\":\"$zone_name\",\"StoragePassword\":\"\$BUNNY_STORAGE_PASSWORD\"}"
    echo "    # 若返回 409（全局重名），自动改用 ${BUCKET}-ot-<随机3位> 重试"
    CDN_STORAGE_ZONE="__DRY_RUN_STORAGE_ZONE__"
  else
    list="$(curl -sS "https://api.bunny.net/storagezone" -H "AccessKey: ${BUNNY_API_KEY:-}")"
    found="$(_bunny_find "$list" "$zone_name" StoragePassword)" || true
    if [[ -n "$found" ]]; then
      zone_id="${found%%|*}"
      [[ -z "${BUNNY_STORAGE_PASSWORD:-}" ]] && BUNNY_STORAGE_PASSWORD="${found#*|}"
      CDN_STORAGE_ZONE="$zone_name"
      echo "==> Storage Zone 已存在，复用: $zone_name (id: $zone_id)"
    else
      # ponytail: 最多重试 3 次带随机后缀的名字；仍被占则报错退出
      for _ in 1 2 3; do
        pass="${BUNNY_STORAGE_PASSWORD:-$(openssl rand -hex 8)}"
        tmp="$(mktemp)"
        code="$(curl -sS -o "$tmp" -w '%{http_code}' -X POST "https://api.bunny.net/storagezone" \
          -H "AccessKey: ${BUNNY_API_KEY:-}" -H "Content-Type: application/json" \
          --data "{\"Name\":\"$zone_name\",\"StoragePassword\":\"$pass\"}")"
        if [[ "$code" == "409" ]]; then
          echo "    Storage Zone 名 $zone_name 已被全局占用，追加随机后缀重试"
          zone_name="${BUCKET}-ot-$((RANDOM % 900 + 100))"
          rm -f "$tmp"
          continue
        fi
        if [[ "$code" != "200" && "$code" != "201" ]]; then
          echo "错误: 创建 Storage Zone 失败 (HTTP $code): $(cat "$tmp")" >&2
          rm -f "$tmp"
          return 1
        fi
        zone_id="$(grep -oE '"Id": *[0-9]+' "$tmp" | head -1 | grep -oE '[0-9]+' | head -1)"
        rm -f "$tmp"
        CDN_STORAGE_ZONE="$zone_name"
        echo "==> 已创建 Storage Zone: $zone_name (id: $zone_id)"
        if [[ -z "${BUNNY_STORAGE_PASSWORD:-}" ]]; then
          echo "    本次生成存储密码: $pass —— 请保存并导出 BUNNY_STORAGE_PASSWORD=$pass（上传需要）"
        fi
        break
      done
      if [[ -z "${CDN_STORAGE_ZONE:-}" ]]; then
        echo "错误: 3 次创建 Storage Zone 均被占用（409），请换 BUCKET 名" >&2
        return 1
      fi
    fi
  fi
  save_state "CDN_STORAGE_ZONE=$CDN_STORAGE_ZONE"

  # 2. Pull Zone（CDN，幂等）
  if [[ -n "${CDN_PULL_ZONE_ID:-}" ]]; then
    echo "==> 复用已有 Pull Zone: $CDN_PULL_ZONE_ID（来自状态文件）"
  elif [[ $DRY_RUN -eq 1 ]]; then
    echo "==> [DRY-RUN] GET https://api.bunny.net/pullzone -H \"AccessKey: \$BUNNY_API_KEY\"（按 Name 匹配 $pull_name）"
    echo "==> [DRY-RUN] POST https://api.bunny.net/pullzone -H \"AccessKey: \$BUNNY_API_KEY\" --data {\"Name\":\"$pull_name\",\"OriginUrl\":\"https://${CDN_STORAGE_ZONE}.storage.bunnycdn.com\",\"StorageZoneId\":<id>}"
    CDN_PULL_ZONE_ID="__DRY_RUN_PULL_ZONE_ID__"
    CDN_PULL_ZONE_HOST="__DRY_RUN_PULL_ZONE_HOST__.b-cdn.net"
  else
    # 兜底：状态里只有 zone 名时查一次 id（创建 Pull Zone 需关联 StorageZoneId 才能免鉴权拉源）
    if [[ -z "$zone_id" ]]; then
      list="$(curl -sS "https://api.bunny.net/storagezone" -H "AccessKey: ${BUNNY_API_KEY:-}")"
      found="$(_bunny_find "$list" "$CDN_STORAGE_ZONE" StoragePassword)" || true
      zone_id="${found%%|*}"
    fi
    list="$(curl -sS "https://api.bunny.net/pullzone" -H "AccessKey: ${BUNNY_API_KEY:-}")"
    found="$(_bunny_find "$list" "$pull_name" Hostname)" || true
    if [[ -n "$found" ]]; then
      CDN_PULL_ZONE_ID="${found%%|*}"
      CDN_PULL_ZONE_HOST="${found#*|}"
      echo "==> Pull Zone 已存在，复用: $pull_name (id: $CDN_PULL_ZONE_ID)"
    else
      tmp="$(mktemp)"
      code="$(curl -sS -o "$tmp" -w '%{http_code}' -X POST "https://api.bunny.net/pullzone" \
        -H "AccessKey: ${BUNNY_API_KEY:-}" -H "Content-Type: application/json" \
        --data "{\"Name\":\"$pull_name\",\"OriginUrl\":\"https://${CDN_STORAGE_ZONE}.storage.bunnycdn.com\"${zone_id:+, \"StorageZoneId\": $zone_id}}")"
      if [[ "$code" != "200" && "$code" != "201" ]]; then
        echo "错误: 创建 Pull Zone 失败 (HTTP $code): $(cat "$tmp")" >&2
        rm -f "$tmp"
        return 1
      fi
      CDN_PULL_ZONE_ID="$(grep -oE '"Id": *[0-9]+' "$tmp" | head -1 | grep -oE '[0-9]+' | head -1)"
      CDN_PULL_ZONE_HOST="$(grep -oE '"Hostname": *"[^"]*"' "$tmp" | head -1 | sed -E 's/.*"([^"]*)"$/\1/')"
      rm -f "$tmp"
      echo "==> 已创建 Pull Zone: $pull_name (id: $CDN_PULL_ZONE_ID)"
      echo "    CDN 域名: https://$CDN_PULL_ZONE_HOST"
      echo "    注意: OriginUrl 未携带存储凭据；若源站 401，请在控制台 Pull Zone -> 源站 补充存储密码。"
    fi
  fi
  save_state "CDN_PULL_ZONE_ID=$CDN_PULL_ZONE_ID"
  save_state "CDN_PULL_ZONE_HOST=$CDN_PULL_ZONE_HOST"

  # 3. 缓存规则与域名指引（Bunny 缓存 TTL 在控制台配置，非对象 meta）
  cat <<EOF
==> Bunny.net CDN 配置指引:
    1. 缓存规则（控制台 -> Pull Zone -> Cache Control / Cache Rules）:
       - *.png *.jpg *.jpeg *.webp *.gif *.svg -> 30 天（2592000s）
       - 默认规则 -> 1 天（86400s）
    2. HTTPS: Pull Zone 自带免费 HTTPS，直接使用 https://$CDN_PULL_ZONE_HOST 即可
EOF
  if [[ -n "${DOMAIN:-}" ]]; then
    cat <<EOF
    3. 自定义域名 $DOMAIN: 控制台 -> Pull Zone -> Manage Hostnames 添加，
       域名 DNS 加 CNAME 记录指向 $CDN_PULL_ZONE_HOST，并在控制台申请免费 SSL 证书
EOF
  fi
  echo "    4. 存储密码保存在环境变量 BUNNY_STORAGE_PASSWORD，请勿泄露"
}

# 上传文件或目录到 https://storage.bunnycdn.com/<zone>/<prefix>/（PUT，AccessKey=存储密码）。
# 目录递归保留相对路径；ttl 仅用于打印指引（Bunny 缓存 TTL 在控制台 Cache Rules 配置）。
cdn_upload() {
  local src="$1" prefix="$2" ttl="$3"
  local zone="${CDN_STORAGE_ZONE:-${BUCKET}-ot}"
  prefix="${prefix%/}"
  echo "--> 上传目标: $zone/$prefix/（TTL 请在控制台 Cache Rules 设置，本参数 $ttl 秒仅供参考）"

  _bunny_put() { # 单文件上传；DRY-RUN 只打印
    local f="$1" rel="$2"
    if [[ $DRY_RUN -eq 1 ]]; then
      echo "# dry-run: curl -X PUT https://storage.bunnycdn.com/$zone/$prefix/$rel -H \"AccessKey: \$BUNNY_STORAGE_PASSWORD\" --data-binary @$f --max-time 60"
      return 0
    fi
    echo "--> $rel"
    curl -sS -o /dev/null -w '    HTTP %{http_code}\n' -X PUT "https://storage.bunnycdn.com/$zone/$prefix/$rel" \
      -H "AccessKey: ${BUNNY_STORAGE_PASSWORD:-}" --data-binary @"$f" --max-time 60
  }

  local f rel
  if [[ -d "$src" ]]; then
    while IFS= read -r -d '' f; do
      rel="${f#"$src"/}"
      _bunny_put "$f" "$rel"
    done < <(find "$src" -type f -print0)
  else
    _bunny_put "$src" "$(basename "$src")"
  fi
}
