#!/bin/bash
# 从库首次初始化（仅空数据卷时由 docker-entrypoint 在临时服务器上执行）：
#   1. 等待主库就绪（compose depends_on 已保证，此处双保险）
#   2. mysqldump 主库 travel 库（--source-data=2 输出 binlog 坐标，一致性快照）
#   3. 导入本地临时服务器
#   4. CHANGE MASTER 指向主库坐标并 START SLAVE（配置持久化在数据卷，
#      之后重启自动继续复制，本脚本不再执行）
set -e

MASTER_HOST=mysql
MASTER_PORT=3306
MASTER_USER=root
MASTER_PASSWORD="${MYSQL_ROOT_PASSWORD:-travel_dev}"

for i in $(seq 1 60); do
  if mysqladmin ping -h "$MASTER_HOST" -u"$MASTER_USER" -p"$MASTER_PASSWORD" --silent 2>/dev/null; then
    break
  fi
  if [ "$i" = 60 ]; then
    echo "replica-init: primary not reachable, aborting"
    exit 1
  fi
  sleep 2
done

# 临时服务器期间 root 已有密码（镜像先建用户再跑 init 脚本），走 socket
LOCAL=(mysql -uroot -p"$MYSQL_ROOT_PASSWORD")

# --databases 使 dump 含 CREATE DATABASE/USE，可导入空服务器（不带则报 No database selected）
mysqldump -h "$MASTER_HOST" -u"$MASTER_USER" -p"$MASTER_PASSWORD" \
  --databases --single-transaction --source-data=2 --default-character-set=utf8mb4 \
  travel > /tmp/primary-dump.sql \
  || { echo "replica-init: dump failed"; exit 1; }

"${LOCAL[@]}" --default-character-set=utf8mb4 < /tmp/primary-dump.sql \
  || { echo "replica-init: import failed"; exit 1; }

# dump 头部的 CHANGE MASTER TO 行形如：
# -- CHANGE MASTER TO MASTER_LOG_FILE='...', MASTER_LOG_POS=N;
POS=$(grep -m1 'CHANGE MASTER TO' /tmp/primary-dump.sql | sed 's/^-- CHANGE MASTER TO //;s/;$//')

"${LOCAL[@]}" -e "CHANGE MASTER TO MASTER_HOST='$MASTER_HOST', MASTER_PORT=$MASTER_PORT, MASTER_USER='$MASTER_USER', MASTER_PASSWORD='$MASTER_PASSWORD', $POS; START SLAVE;" \
  || { echo "replica-init: change master failed"; exit 1; }

# 验证复制线程已启动，失败则退出让 compose 可见（服务端回退主库）
sleep 2
if ! "${LOCAL[@]}" -e "SHOW SLAVE STATUS\G" 2>/dev/null | grep -q "Slave_IO_Running: Yes"; then
  echo "replica-init: replication not running"
  exit 1
fi
echo "replica-init: dump restored, replication started"
