#!/usr/bin/env bash
# open-travel OpenSearch 索引初始化（幂等，可重复执行：先删后建，保证 mapping 与最新 DDL 一致）
# 注意：本集群持久设置了 cluster.blocks.create_index=true，运行本脚本前需先解除：
#   curl -X PUT localhost:9201/_cluster/settings -H 'Content-Type: application/json' \
#     -d '{"persistent":{"cluster":{"blocks":{"create_index":null}}}}'
# 建完可恢复原设置（向已有索引写文档不受此限制）
# 用法：bash scripts/opensearch_init.sh [base_url]
# 默认 http://localhost:9200（本机 compose 暴露 9201，compose 内网用 http://opensearch:9200）
set -euo pipefail

BASE="${1:-http://localhost:9200}"

create_index() {
  local name="$1" body="$2"
  curl -sf -X DELETE "$BASE/$name" >/dev/null 2>&1 || true
  curl -sf -X PUT "$BASE/$name" -H 'Content-Type: application/json' -d "$body" >/dev/null
  echo "index $name created"
}

# cjk 分析器（内置 bigram 中日韩分词，无需插件；镜像无 analysis-icu/kuromoji 插件，
# 中文/日文用 cjk，其余语言用 standard）
SETTINGS='"settings": {
    "number_of_shards": 1,
    "number_of_replicas": 0,
    "analysis": { "analyzer": { "i18n_text": { "type": "cjk" } } }
  }'

# 目的地索引（覆盖 travel_destinations 全字段）
create_index "travel_destinations" "{
  $SETTINGS,
  \"mappings\": {
    \"properties\": {
      \"id\":          { \"type\": \"long\" },
      \"name_en\":     { \"type\": \"text\", \"analyzer\": \"standard\" },
      \"name_zh\":     { \"type\": \"text\", \"analyzer\": \"i18n_text\" },
      \"name_ja\":     { \"type\": \"text\", \"analyzer\": \"i18n_text\" },
      \"description\": { \"type\": \"text\", \"analyzer\": \"i18n_text\" },
      \"category\":    { \"type\": \"keyword\" },
      \"region_id\":   { \"type\": \"long\" },
      \"latitude\":    { \"type\": \"double\" },
      \"longitude\":   { \"type\": \"double\" },
      \"cover_url\":   { \"type\": \"keyword\" },
      \"status\":      { \"type\": \"keyword\" },
      \"sort_order\":  { \"type\": \"keyword\" }
    }
  }
}"

# 景点索引（13 语种 name 字段与客户端 ARB 语种一致）
create_index "travel_attractions" "{
  $SETTINGS,
  \"mappings\": {
    \"properties\": {
      \"id\":             { \"type\": \"long\" },
      \"destination_id\": { \"type\": \"long\" },
      \"name_en\":        { \"type\": \"text\", \"analyzer\": \"standard\" },
      \"name_zh\":        { \"type\": \"text\", \"analyzer\": \"i18n_text\" },
      \"name_ja\":        { \"type\": \"text\", \"analyzer\": \"i18n_text\" },
      \"name_ko\":        { \"type\": \"text\", \"analyzer\": \"standard\" },
      \"name_ar\":        { \"type\": \"text\", \"analyzer\": \"standard\" },
      \"name_es\":        { \"type\": \"text\", \"analyzer\": \"standard\" },
      \"name_fr\":        { \"type\": \"text\", \"analyzer\": \"standard\" },
      \"name_de\":        { \"type\": \"text\", \"analyzer\": \"standard\" },
      \"name_pt\":        { \"type\": \"text\", \"analyzer\": \"standard\" },
      \"name_hi\":        { \"type\": \"text\", \"analyzer\": \"standard\" },
      \"name_bn\":        { \"type\": \"text\", \"analyzer\": \"standard\" },
      \"name_id\":        { \"type\": \"text\", \"analyzer\": \"standard\" },
      \"name_ru\":        { \"type\": \"text\", \"analyzer\": \"standard\" },
      \"description\":    { \"type\": \"text\", \"analyzer\": \"i18n_text\" },
      \"price_cents\":    { \"type\": \"keyword\" },
      \"status\":         { \"type\": \"keyword\" },
      \"open_hours\":     { \"type\": \"keyword\" },
      \"rating_avg\":     { \"type\": \"double\" },
      \"cover_url\":      { \"type\": \"keyword\" }
    }
  }
}"
