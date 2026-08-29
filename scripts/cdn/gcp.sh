# Google Cloud 提供方：GCS bucket 源站 + Cloud CDN（global 负载均衡）。
# 被 cdn_setup.sh / cdn_upload.sh source，只定义函数，无顶层副作用。
# 凭据：gcloud 应用默认凭据（GOOGLE_APPLICATION_CREDENTIALS）或 gcloud auth login。

# 幂等执行：DRY_RUN=1 只打印；否则执行。所有 gcloud/gsutil 命令都走这里。
run() {
  echo "    $*"
  [[ ${DRY_RUN:-0} -eq 1 ]] && return 0
  "$@"
}

# 状态持久化：KEY=VALUE 追加到 $STATE_FILE，dispatcher 用 source 读回。
# dispatcher 未定义时才自己补上（向后兼容）。DRY_RUN 不写状态。
if ! declare -F save_state >/dev/null; then
  save_state() { [[ ${DRY_RUN:-0} -eq 1 ]] && return 0; echo "$1" >> "$STATE_FILE"; }
fi

# 已存在则打印提示并跳过（幂等的一部分）。
existing() { echo "    已存在，复用: $1"; }

# 返回 0：gcloud + gsutil 已安装且凭据可用；否则打印缺失项并返回 1。
cdn_require_creds() {
  local missing=0
  command -v gcloud >/dev/null 2>&1 || { echo "缺少 gcloud CLI（https://cloud.google.com/sdk/docs/install）"; missing=1; }
  command -v gsutil >/dev/null 2>&1 || { echo "缺少 gsutil（随 gcloud SDK 一起安装）"; missing=1; }
  [[ $missing -eq 1 ]] && return 1

  # ADC 或 gcloud 活跃账号，任一即可
  if [[ -n "${GOOGLE_APPLICATION_CREDENTIALS:-}" && -f "$GOOGLE_APPLICATION_CREDENTIALS" ]]; then
    return 0
  fi
  if gcloud auth list --filter=status:ACTIVE --format='value(account)' 2>/dev/null | grep -q .; then
    return 0
  fi
  echo "缺少凭据：请设置 GOOGLE_APPLICATION_CREDENTIALS（指向服务账号 JSON）或执行 gcloud auth login / gcloud auth application-default login"
  return 1
}

# 幂等配置：bucket → 公开读 → Cloud CDN（backend-bucket → url-map → proxy → forwarding-rule）。
cdn_setup() {
  local project=""
  if command -v gcloud >/dev/null 2>&1; then
    project="$(gcloud config get-value project 2>/dev/null | tr -d '\n')"
  fi
  # DRY-RUN 无 gcloud 时用占位 project 预览；真实执行仍需已配置 project
  if [[ ${DRY_RUN:-0} -eq 1 ]] && { [[ -z "$project" || "$project" == "(unset)" ]]; }; then
    project="<PROJECT_ID>"
  fi
  [[ -n "$project" && "$project" != "(unset)" ]] || {
    echo "错误: 未配置 GCP project（gcloud config get-value project 为空），请先: gcloud config set project <PROJECT_ID>" >&2
    return 1
  }

  # (a) 创建 GCS bucket（幂等：已存在则复用）
  if gsutil ls "gs://$BUCKET" >/dev/null 2>&1; then
    existing "gs://$BUCKET"
  else
    echo "==> 创建 bucket gs://$BUCKET（$REGION, standard）"
    run gsutil mb -p "$project" -c standard -l "$REGION" "gs://$BUCKET"
  fi

  # (b) 公开读：Cloud CDN 源站必须是公开可读的（经 CDN 回源），统一访问控制 + 公共读
  echo "==> 设置公开读（CDN 回源需要）: allUsers:objectViewer"
  run gsutil iam ch allUsers:objectViewer "gs://$BUCKET"

  # (c) Cloud CDN：已有状态则整体跳过
  if [[ -n "${CDN_BACKEND_BUCKET:-}" ]]; then
    echo "==> 已配置过（状态文件），复用 CDN_BACKEND_BUCKET=$CDN_BACKEND_BUCKET"
    return 0
  fi

  # (c-1) backend bucket：开启 Cloud CDN 缓存
  if gcloud compute backend-buckets describe "$BUCKET-cdn" >/dev/null 2>&1; then
    existing "backend-bucket $BUCKET-cdn"
  else
    echo "==> 创建 backend-bucket $BUCKET-cdn（--enable-cdn）"
    run gcloud compute backend-buckets create "$BUCKET-cdn" --gcs-bucket-name="$BUCKET" --enable-cdn
  fi

  # (c-2) url-map：请求路由到 backend bucket
  if gcloud compute url-maps describe "$BUCKET-url-map" >/dev/null 2>&1; then
    existing "url-map $BUCKET-url-map"
  else
    echo "==> 创建 url-map $BUCKET-url-map"
    run gcloud compute url-maps create "$BUCKET-url-map" --default-backend-bucket="$BUCKET-cdn"
  fi

  # (c-3) target-http-proxy：HTTP(80)。HTTPS 需要证书，GCP 证书走 SSL 证书资源而非 ARN，
  #        --domain/--cert 参数在本提供方不适用（可用 gcloud compute ssl-certificates 自行扩展）。
  if gcloud compute target-http-proxies describe "$BUCKET-proxy" >/dev/null 2>&1; then
    existing "target-http-proxy $BUCKET-proxy"
  else
    echo "==> 创建 target-http-proxy $BUCKET-proxy"
    run gcloud compute target-http-proxies create "$BUCKET-proxy" --url-map="$BUCKET-url-map"
  fi

  # (c-4) 全局转发规则：端口 80 → proxy
  if gcloud compute forwarding-rules describe "$BUCKET-fwd" --global >/dev/null 2>&1; then
    existing "forwarding-rule $BUCKET-fwd"
  else
    echo "==> 创建全局 forwarding-rule $BUCKET-fwd（:80）"
    run gcloud compute forwarding-rules create "$BUCKET-fwd" --global \
      --target-http-proxy="$BUCKET-proxy" --ports=80
  fi

  echo "==> 记录状态"
  save_state "CDN_BACKEND_BUCKET=$BUCKET-cdn"
}

# 上传文件或目录到源站 bucket，带缓存头。
#   $1=src（文件或目录） $2=prefix（GCS 对象前缀） $3=ttl（秒）
cdn_upload() {
  local src="$1" prefix="$2" ttl="$3"
  prefix="${prefix%/}"
  [[ -e "$src" ]] || { echo "错误: 源不存在: $src" >&2; return 1; }
  local cache_header="Cache-Control:public, max-age=$ttl"
  if [[ -d "$src" ]]; then
    echo "==> 上传目录 $src -> gs://$BUCKET/$prefix/（保留相对路径, ttl=${ttl}s）"
    run gsutil -m cp -r -h "$cache_header" "$src" "gs://$BUCKET/$prefix/"
  else
    echo "==> 上传文件 $src -> gs://$BUCKET/$prefix/（ttl=${ttl}s）"
    run gsutil cp -h "$cache_header" "$src" "gs://$BUCKET/$prefix/"
  fi
}
