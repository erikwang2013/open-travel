#!/usr/bin/env bash
# open-travel 一键启动：构建镜像、启动容器、等待健康检查
set -euo pipefail
cd "$(dirname "$0")/.."

COMPOSE=(docker compose -p open-travel -f config/docker-compose.yml)

"${COMPOSE[@]}" up -d --build

echo "等待所有服务健康（最长 120s）..."
for i in $(seq 1 60); do
  status=$("${COMPOSE[@]}" ps --format '{{.Name}}|{{.State}}|{{.Health}}')
  total=$(wc -l <<<"$status")
  ok=$(awk -F'|' '$2 == "running" && ($3 == "healthy" || $3 == "")' <<<"$status" | wc -l)
  if [ "$ok" -eq "$total" ]; then
    echo "全部服务就绪"
    break
  fi
  if [ "$i" -eq 60 ]; then
    echo "超时：以下服务未就绪" >&2
    "${COMPOSE[@]}" ps
    exit 1
  fi
  sleep 2
done

"${COMPOSE[@]}" ps
