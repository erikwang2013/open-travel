#!/usr/bin/env bash
# open-travel 一键安装：环境检查 → 构建并启动全部服务 → 初始化 OpenSearch 索引
# → 健康巡检 → 输出访问地址与默认账号。
# 用法：./scripts/install.sh   （幂等，可重复执行；首次启动会自动建库建表）
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> 环境检查"
command -v docker >/dev/null 2>&1 || {
  echo "错误：未检测到 Docker。请先安装 Docker：" >&2
  echo "  - Linux:   https://docs.docker.com/engine/install/" >&2
  echo "  - macOS:   Docker Desktop https://docs.docker.com/desktop/install/mac/" >&2
  echo "  - Windows: Docker Desktop https://docs.docker.com/desktop/install/windows/" >&2
  exit 1
}
docker compose version >/dev/null 2>&1 || {
  echo "错误：需要 Docker Compose v2（docker compose 子命令不可用）" >&2
  exit 1
}

# 可选：按 .env 覆盖默认端口/密码（MYSQL_ROOT_PASSWORD / REDIS_URL 等）
if [ -f .env ]; then
  echo "检测到 .env，将覆盖 compose 默认值"
fi

echo "==> 构建并启动全部服务（首次需拉取镜像，耗时数分钟）"
./scripts/up.sh

echo "==> 初始化 OpenSearch 索引（幂等）"
if [ -x scripts/opensearch_init.sh ]; then
  ./scripts/opensearch_init.sh || echo "警告：OpenSearch 索引初始化失败，搜索可用性受限（不影响其余功能）" >&2
fi

echo "==> 服务健康巡检"
./scripts/health_check.sh || {
  echo "健康检查失败。排查：docker compose -p open-travel -f config/docker-compose.yml ps" >&2
  exit 1
}

cat <<'EOF'

✅ Open Travel 安装完成！

  网关地址   http://localhost:8082        （探活：/health）
  管理端     http://localhost:8082/api/admin 通过 API 访问；
             前端界面见 apps/admin（flutter run -d chrome，端口 8082 已代理 /api/admin）
  默认账号   admin@travel.local / Admin@123（仅本地开发环境）

  常用命令：
    重启服务   scripts/deploy.sh
    停止服务   scripts/down.sh
    健康巡检   scripts/health_check.sh
    接口文档   docs/api.md
EOF
