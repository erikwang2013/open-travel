#!/usr/bin/env bash
# 压测脚本：curl 并发循环（无 wrk/k6 环境时的基准工具）
# 用法: scripts/loadtest.sh -c <并发> -n <总请求数> -u <URL> [-H <额外头>] [-o <原始数据文件>]
set -u

C=1 N=10 URL="" OUT=""
HEADERS=()

usage() { echo "用法: $0 -c <并发> -n <总请求数> -u <URL> [-H <头>] [-o <原始数据文件>]"; exit 1; }
while getopts "c:n:u:H:o:" opt; do
  case $opt in
    c) C=$OPTARG ;; n) N=$OPTARG ;; u) URL=$OPTARG ;;
    H) HEADERS+=(-H "$OPTARG") ;; o) OUT=$OPTARG ;;
    *) usage ;;
  esac
done
[ -z "$URL" ] && usage

PER_WORKER=$((N / C)); [ "$PER_WORKER" -lt 1 ] && PER_WORKER=1
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

echo "==> 压测开始: URL=$URL 并发=$C 请求数=$((PER_WORKER * C)) ($(date '+%H:%M:%S'))"
start=$(date +%s.%N)
for ((w = 0; w < C; w++)); do
  (
    for ((i = 0; i < PER_WORKER; i++)); do
      curl -s -o /dev/null -w "%{http_code} %{time_total}\n" "${HEADERS[@]}" "$URL" >>"$TMP/r.txt"
    done
  ) &
done
wait
end=$(date +%s.%N)
elapsed=$(awk "BEGIN{printf \"%.3f\", $end - $start}")

cat "$TMP/r.txt" | sort -k2 -n >"$TMP/s.txt"
total=$(wc -l <"$TMP/s.txt")
qps=$(awk "BEGIN{printf \"%.1f\", $total / $elapsed}")
avg=$(awk '{s+=$2} END{printf "%.3f", s/NR}' "$TMP/s.txt")
p50=$(awk -v n="$total" 'NR==int(n*0.5)+1{print $2}' "$TMP/s.txt")
p95=$(awk -v n="$total" 'NR==int(n*0.95)+1{print $2}' "$TMP/s.txt")
codes=$(awk '{c[$1]++} END{for (k in c) printf "%s:%d ", k, c[k]}' "$TMP/r.txt")

echo "==> 完成: 耗时=${elapsed}s 请求数=$total QPS=$qps"
echo "    延迟(s): avg=$avg p50=$p50 p95=$p95"
echo "    HTTP 状态码: $codes"
[ -n "$OUT" ] && cp "$TMP/r.txt" "$OUT" && echo "    原始数据已写入: $OUT"
exit 0
