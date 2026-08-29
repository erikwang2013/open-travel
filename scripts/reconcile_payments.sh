#!/usr/bin/env bash
# 支付对账（P4-06）：待支付流水超过 24h 置为失败（status=2），并输出各状态摘要。
# 用法：./scripts/reconcile_payments.sh [DATABASE_URL]
set -euo pipefail

DB_URL="${1:-${DATABASE_URL:-mysql://root:travel_dev@localhost:3308/travel?charset=utf8mb4}}"

mysql --default-character-set=utf8mb4 "$DB_URL" <<'SQL'
UPDATE travel_payments SET status = 2
WHERE status = 0 AND created_at < DATE_SUB(NOW(), INTERVAL 24 HOUR);
SELECT status, COUNT(*) AS cnt FROM travel_payments GROUP BY status ORDER BY status;
SQL
