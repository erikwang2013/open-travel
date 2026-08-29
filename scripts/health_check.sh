#!/usr/bin/env bash
# open-travel 服务健康巡检：轮询各服务 /health，失败/恢复时输出告警
# 用法：scripts/health_check.sh [--notify-cmd '命令']（或环境变量 NOTIFY_CMD）
#   NOTIFY_CMD 示例：NOTIFY_CMD='curl -fsS -X POST -d @- https://hooks.example/webhook'
#   告警消息以单个参数传给该命令；默认仅写日志。
# 退出码：0 = 全部健康；1 = 存在不健康服务（供 cron 判断）。
# crontab 示例（每 2 分钟）：
#   */2 * * * * /home/wwwroot/open-travel/scripts/health_check.sh >> /var/log/open-travel-health.log 2>&1
set -uo pipefail
cd "$(dirname "$0")/.."

# 服务名|健康检查地址。网关 /health 仅代理 user，故直连各服务 host 端口；
# 生产仅暴露网关时可用 HEALTH_SERVICES 覆盖为 https://网关/api/{name}/health
SERVICES="${HEALTH_SERVICES:-"
user|http://localhost:8001/health
booking|http://localhost:8002/health
admin|http://localhost:8003/health
search|http://localhost:8004/health
line|http://localhost:8005/health
order|http://localhost:8006/health
flight|http://localhost:8007/health
hotel|http://localhost:8008/health
payment|http://localhost:8009/health
"}"

if [ "${1:-}" = "--notify-cmd" ]; then NOTIFY_CMD="$2"; shift 2; fi
NOTIFY_CMD="${NOTIFY_CMD:-}"
STATE_DIR="${TMPDIR:-/tmp}/open-travel-health"
mkdir -p "$STATE_DIR"

fail=0
while IFS='|' read -r name url; do
  [ -z "$name" ] && continue
  code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 5 "$url")
  state_file="$STATE_DIR/$name"
  prev=""; [ -f "$state_file" ] && prev=$(cat "$state_file")
  if [ "$code" = "200" ]; then
    if [ "$prev" != "up" ]; then
      msg="[RECOVER] $(date '+%F %T') $name 恢复健康（$url）"
      echo "$msg"
      [ -n "$NOTIFY_CMD" ] && $NOTIFY_CMD "$msg"
    fi
    echo up > "$state_file"
  else
    fail=1
    msg="[ALERT] $(date '+%F %T') $name 不健康（$url）HTTP=$code"
    echo "$msg" >&2
    # 首次失败才触发通知，避免持续刷屏；日志仍每次记录
    if [ "$prev" != "down" ]; then
      [ -n "$NOTIFY_CMD" ] && $NOTIFY_CMD "$msg"
    fi
    echo down > "$state_file"
  fi
done <<< "$SERVICES"

if [ "$fail" = 0 ]; then
  echo "[OK] $(date '+%F %T') 全部服务健康"
fi
exit "$fail"
