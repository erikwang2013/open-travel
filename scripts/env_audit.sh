#!/usr/bin/env bash
# open-travel 生产环境变量审计：检查必需变量已设置且非占位值
# 用法：scripts/env_audit.sh [--env .env路径]   （默认读取仓库根 .env，若存在）
# 退出码：0 = 无 FAIL；1 = 存在 FAIL（WARN 不计数）。幂等，可重复执行。
set -uo pipefail
cd "$(dirname "$0")/.."

ENV_FILE="${ENV_FILE:-.env}"
[ -f "$ENV_FILE" ] || echo "WARN 未找到 $ENV_FILE（生产部署必须提供）"

# 加载 .env 到当前 shell（覆盖同名的已导出变量）；.env 由运维控制，视为可信输入
if [ -f "$ENV_FILE" ]; then
  set -a; # shellcheck disable=SC1090
  . "$ENV_FILE"
  set +a
fi

FAIL=0
WARN=0
out() { printf '%s\n' "$1"; } # 汇总行

# $1=级别 $2=变量 $3=说明
report() {
  if [ "$1" = FAIL ]; then FAIL=1; fi
  if [ "$1" = WARN ]; then WARN=1; fi
  printf '%s %s %s\n' "$1" "$2" "$3"
}

# ---- JWT_SECRET：生产必须，非占位，>=32 字节 ----
if [ -z "${JWT_SECRET:-}" ]; then
  report FAIL JWT_SECRET "未设置 — 生成随机值：openssl rand -base64 48"
elif [ "$JWT_SECRET" = "dev-only-change-me-32-bytes-minimum-secret" ]; then
  report FAIL JWT_SECRET "仍为开发占位密钥 — 生成随机值：openssl rand -base64 48"
elif [ "${#JWT_SECRET}" -lt 32 ]; then
  report FAIL JWT_SECRET "长度 ${#JWT_SECRET} < 32 字节 — 需 >=32 字节随机值"
fi

# ---- INTERNAL_TOKEN：内部服务鉴权，非占位 ----
if [ -z "${INTERNAL_TOKEN:-}" ]; then
  report FAIL INTERNAL_TOKEN "未设置 — 生成随机值：openssl rand -hex 32"
elif [ "$INTERNAL_TOKEN" = "dev-internal-secret" ]; then
  report FAIL INTERNAL_TOKEN "仍为开发占位值 — 生成随机值：openssl rand -hex 32"
elif [ "${#INTERNAL_TOKEN}" -lt 16 ]; then
  report WARN INTERNAL_TOKEN "长度仅 ${#INTERNAL_TOKEN}，建议 >=16 字符"
fi

# ---- DATABASE_URL：生产必须，禁用开发默认密码 ----
if [ -z "${DATABASE_URL:-}" ]; then
  report FAIL DATABASE_URL "未设置 — 格式 mysql://用户:密码@host:3306/travel?charset=utf8mb4"
elif [[ "$DATABASE_URL" == *"travel_dev"* ]]; then
  report FAIL DATABASE_URL "含开发默认密码 travel_dev — 替换为生产数据库密码"
fi

# ---- KAFKA_BROKERS：order/payment 必需 ----
if [ -z "${KAFKA_BROKERS:-}" ]; then
  report FAIL KAFKA_BROKERS "未设置 — order/payment 服务依赖 Kafka，如 kafka:9092 或生产地址"
fi

# ---- REDIS_URL / MYSQL_ROOT_PASSWORD：占位提醒 ----
if [ -z "${REDIS_URL:-}" ]; then
  report WARN REDIS_URL "未设置（compose 默认 redis://redis:6379）"
fi
if [ "${MYSQL_ROOT_PASSWORD:-}" = "travel_dev" ]; then
  report WARN MYSQL_ROOT_PASSWORD "仍为开发默认密码 travel_dev — 生产建议更换"
fi

echo "---- 结果：$([ $FAIL = 1 ] && echo '存在 FAIL（生产不可上线）' || echo '无 FAIL') / $([ $WARN = 1 ] && echo '存在 WARN' || echo '无 WARN')"
exit "$FAIL"
