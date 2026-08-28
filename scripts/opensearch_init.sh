#!/usr/bin/env bash
# open-travel OpenSearch 索引初始化（幂等，可重复执行）
# 用法：bash scripts/opensearch_init.sh [base_url]
# 默认 http://localhost:9200（compose 内网用 http://opensearch:9200）
set -euo pipefail

BASE="${1:-http://localhost:9200}"
INDEX="travel_destinations"

# 幂等：索引已存在则跳过
if curl -sf "$BASE/$INDEX" >/dev/null 2>&1; then
  echo "index $INDEX already exists, skip"
  exit 0
fi

# 多语言目的地索引：cjk 分析器（内置，bigram 中日韩分词；
# opensearch:latest 镜像不含 analysis-icu 插件，用 cjk 免装插件）
curl -sf -X PUT "$BASE/$INDEX" -H 'Content-Type: application/json' -d '{
  "settings": {
    "number_of_shards": 1,
    "number_of_replicas": 0,
    "analysis": {
      "analyzer": {
        "i18n_text": { "type": "cjk" }
      }
    }
  },
  "mappings": {
    "properties": {
      "id":          { "type": "long" },
      "name_en":     { "type": "text", "analyzer": "standard" },
      "name_zh":     { "type": "text", "analyzer": "i18n_text" },
      "name_ja":     { "type": "text", "analyzer": "i18n_text" },
      "description": { "type": "text", "analyzer": "i18n_text" },
      "category":    { "type": "keyword" },
      "region_id":   { "type": "long" },
      "latitude":    { "type": "double" },
      "longitude":   { "type": "double" }
    }
  }
}' >/dev/null

echo "index $INDEX created (cjk analyzer)"
