#!/usr/bin/env bash
# open-travel 一键停止（保留数据卷）
set -euo pipefail
cd "$(dirname "$0")/.."

docker compose -p open-travel -f config/docker-compose.yml down
