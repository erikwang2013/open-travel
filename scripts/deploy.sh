#!/usr/bin/env bash
# open-travel 一键发布：构建镜像 → up -d → 健康检查；失败时给出回滚指引
# 用法：scripts/deploy.sh [compose 附加参数...]
# 生产发布前建议先执行 scripts/env_audit.sh 确认环境变量审计通过。
# 回滚：镜像随代码构建（无独立镜像 tag），回滚 = 重新部署上一发布 tag 的代码：
#   git checkout <上一发布tag> && scripts/deploy.sh
set -euo pipefail
cd "$(dirname "$0")/.."

COMPOSE=(docker compose -p open-travel -f config/docker-compose.yml)

echo "==> 构建镜像"
"${COMPOSE[@]}" build "$@"

echo "==> 启动容器"
"${COMPOSE[@]}" up -d "$@"

echo "==> 等待所有服务健康（最长 120s）..."
for i in $(seq 1 60); do
  status=$("${COMPOSE[@]}" ps --format '{{.Name}}|{{.State}}|{{.Health}}')
  total=$(wc -l <<<"$status")
  ok=$(awk -F'|' '$2 == "running" && ($3 == "healthy" || $3 == "")' <<<"$status" | wc -l)
  [ "$ok" -eq "$total" ] && break
  if [ "$i" -eq 60 ]; then
    echo "超时：以下服务未就绪" >&2
    "${COMPOSE[@]}" ps >&2
    exit 1
  fi
  sleep 2
done

echo "==> 健康巡检"
if ! ./scripts/health_check.sh; then
  echo "部署后健康检查失败！回滚：git checkout <上一发布tag> && scripts/deploy.sh" >&2
  exit 1
fi

echo "==> 部署完成"
