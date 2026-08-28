# e-cat エコシステム計画

**バージョン:** 2.1.7  
**日付:** 2026-08-01  
**ステータス:** すべて完了 · 47 crates

| 領域 | カバー済み | ステータス |
|------|--------|------|
| トランスポート層 | HTTP (axum), gRPC (tonic), WebSocket | ✅ |
| エンコーディング | JSON, Protobuf | ✅ |
| ミドルウェア | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth (JWT/API Key/OAuth2) | ✅ |
| 設定 | env, file (JSON/YAML), Consul KV リモート, 暗号化 | ✅ |
| レジストリ | memory, Consul, etcd | ✅ |
| セキュリティ | 攻撃検知, JWT, API Key, OAuth2, TlsConfig | ✅ |
| データ | RDBMS (sqlx), Redis, Memcached, OpenSearch, Elasticsearch, ClickHouse, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB | ✅ |
| 可観測性 | tracing, Prometheus, Health, 分散トレーシング | ✅ |
| 通信 | HTTP/gRPC Client, MessageQueue (InMemory/Kafka), EventBus | ✅ |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | ✅ |
| API ツール | OpenAPI, Versioning, GraphQL | ✅ |

## 残りのギャップ（小規模最適化 3 項目）

1. **transport への mTLS 接続** — TlsConfig はあるが HttpServer/GrpcServer に未接続
2. **Redis レートリミットバックエンド** — RateLimitLayer はメモリのみ、複数インスタンスで共有が必要
3. **GitLab CI テンプレート** — 現在は GitHub Actions のみ

## バージョン推移

```
v1.0.x  コア骨格（18 crates）                    ✅
v2.0.x  エコシステム一期〜三期（+13 crates）      ✅
v2.1.x  通信とセキュリティ強化 + データバックエンド補充 + 運用体験   ✅ (現在)
```

## エコシステムに含めないもの

| 要件 | 方針 | 理由 |
|------|------|------|
| API ゲートウェイ | Kong / Envoy | 言語非依存 |
| サービスメッシュ | Linkerd | Rust に成熟したソリューションなし |
| コンテナオーケストレーション | Kubernetes | 業界標準 |
| ログ収集 | Vector | Rust ネイティブ |
