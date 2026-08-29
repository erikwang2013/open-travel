# Azure CDN provider：Blob Storage 源站 + 经典 CDN（profile open-travel-cdn + endpoint）。
# 凭据：az login 登录（AZURE_SUBSCRIPTION_ID 可选，指定订阅）；无硬编码密钥。
# 存储数据面命令带 --auth-mode login，需给登录账号授予 "Storage Blob Data Owner"
# （或至少 Blob 数据贡献者/读者）RBAC 角色，否则上传/建容器会 403。
# 注意：Azure 存储账号名仅限 3-24 位小写字母/数字，--bucket 需符合该规则。

RG="${BUCKET}-rg"
SUB=()
[[ -n "${AZURE_SUBSCRIPTION_ID:-}" ]] && SUB=(--subscription "$AZURE_SUBSCRIPTION_ID")
[[ -z "${REGION:-}" ]] && REGION=eastus

save_state() {  # 追加 KEY=VALUE 到状态文件（读取：source "$STATE_FILE"）
  echo "$1" >>"$STATE_FILE"
}

run() {  # 打印命令；DRY_RUN=1 时只打印不执行
  echo "    \$ $*"
  [[ $DRY_RUN -eq 1 ]] && return 0
  "$@"
}

cdn_require_creds() {
  command -v az >/dev/null 2>&1 || { echo "缺少 az CLI：请安装 Azure CLI（如 apt install azure-cli）后重试"; return 1; }
  if ! az account show "${SUB[@]}" >/dev/null 2>&1; then
    if [[ -n "${AZURE_SUBSCRIPTION_ID:-}" ]]; then
      echo "az 未登录或订阅不可用：请执行 az login（并确认 AZURE_SUBSCRIPTION_ID 有效）"
    else
      echo "az 未登录：请先执行 az login 登录 Azure"
    fi
    return 1
  fi
  return 0
}

cdn_setup() {
  if [[ -f "$STATE_FILE" && -n "${CDN_ENDPOINT:-}" ]]; then
    echo "==> 状态文件存在，Azure CDN 已配置：$CDN_ENDPOINT（已存在，复用）"
    return 0
  fi

  # (a) 资源组（无则建）+ 存储账号（无则建，Standard_LRS / StorageV2）
  if ! az group show -n "$RG" "${SUB[@]}" >/dev/null 2>&1; then
    run az group create -n "$RG" -l "$REGION" "${SUB[@]}"
  fi
  if ! az storage account show -n "$BUCKET" "${SUB[@]}" >/dev/null 2>&1; then
    run az storage account create -n "$BUCKET" -g "$RG" -l "$REGION" --sku Standard_LRS --kind StorageV2 "${SUB[@]}"
  fi

  # (b) 容器 static（公开读，作为 CDN 源站）
  if [[ $(az storage container exists -n static --account-name "$BUCKET" --auth-mode login "${SUB[@]}" -o tsv 2>/dev/null) != True ]]; then
    run az storage container create -n static --account-name "$BUCKET" --public-access blob --auth-mode login "${SUB[@]}"
  fi

  # (c) CDN profile（经典 CDN，Standard_Microsoft）+ endpoint
  if ! az cdn profile show -n open-travel-cdn -g "$RG" "${SUB[@]}" >/dev/null 2>&1; then
    run az cdn profile create -n open-travel-cdn -g "$RG" --sku Standard_Microsoft "${SUB[@]}"
  fi
  if ! az cdn endpoint show -n "$BUCKET-endpoint" --profile-name open-travel-cdn -g "$RG" "${SUB[@]}" >/dev/null 2>&1; then
    run az cdn endpoint create -n "$BUCKET-endpoint" --profile-name open-travel-cdn -g "$RG" \
      --origin "$BUCKET.blob.core.windows.net" --origin-host-header "$BUCKET.blob.core.windows.net" "${SUB[@]}"
  fi

  [[ $DRY_RUN -eq 1 ]] && return 0
  save_state "CDN_PROFILE=open-travel-cdn"
  save_state "CDN_ENDPOINT=$BUCKET-endpoint"
  echo "==> 已就绪：$BUCKET-endpoint.azureedge.net（源站 $BUCKET.blob.core.windows.net）"

  if [[ -n "${DOMAIN:-}" ]]; then
    echo "==> 自定义域名 $DOMAIN：Azure 经典 CDN 的绑定域名 + HTTPS 需在门户完成："
    echo "    门户 -> open-travel-cdn 配置文件 -> $BUCKET-endpoint -> 自定义域，先添加 CNAME 指向 *.azureedge.net，"
    echo "    再启用 HTTPS（Azure 托管证书，无需自带；故 CERT_ARN 参数在 Azure 下不使用）。"
  fi
}

cdn_upload() {  # $1=src 文件或目录 $2=prefix $3=TTL 秒
  local src="$1" prefix="$2" ttl="$3"
  if [[ -d "$src" ]]; then
    run az storage blob upload-batch -d static -s "$src" --destination-path "$prefix" \
      --account-name "$BUCKET" --content-cache-control "public, max-age=$ttl" --auth-mode login "${SUB[@]}"
  else
    run az storage blob upload -f "$src" -c static -n "$prefix/$(basename "$src")" \
      --account-name "$BUCKET" --content-cache-control "public, max-age=$ttl" --auth-mode login "${SUB[@]}"
  fi
}
