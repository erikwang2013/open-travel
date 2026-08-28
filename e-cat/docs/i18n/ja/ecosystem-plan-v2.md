# e-cat エコシステム計画 v2 — 完了分と今後の課題

**バージョン:** 2.1.7  
**日付:** 2026-08-01  
**ステータス:** 全計画完了、47 crates

---

## 一、完了済み（すべて納品）

| 期次 | Crate | 能力 | テスト |
|------|-------|------|------|
| 一期 | `ecat-health` | ヘルスチェック（/health、/ready） | 4 |
| 一期 | `ecat-client` | HTTP/gRPC クライアント + サービスディスカバリ + ロードバランシング | 7 |
| 一期 | `ecat-circuit-breaker` | 三態サーキットブレーカー（Tower Layer） | 4 |
| 一期 | `ecat-auth` | JWT + API Key + OAuth2 認証ミドルウェア | 8 |
| 一期 | `ecat-registry-consul` | Consul サービス登録 | 2 |
| 二期 | `ecat-data-redis` | Redis キャッシュ（Cache trait） | 1 |
| 二期 | `ecat-mq` | メッセージキュー抽象 + InMemoryMq | 2 |
| 二期 | `ecat-events` | ローカル + リモートイベントバス | 2 |
| 二期 | `ecat-config-remote` | Consul KV リモート設定 | 2 |
| 三期 | `ecat-testing` | MockServer + ChaosConfig | 5 |
| 三期 | `ecat-openapi` | OpenAPI 3.0 spec 生成 | 2 |
| 三期 | `ecat-bench` | 並行性能ベンチマーク | 2 |
| 三期 | `ecat-deploy` | Dockerfile + K8s + Helm | — |
| 四期 | `ecat-tracing` | 分散トレーシング（span + trace_id） | 2 |
| 四期 | `ecat-client` 拡張 | GrpcClient + TlsConfig | — |
| 四期 | `ecat-auth` 拡張 | OAuth2Layer | — |
| 五期 | `ecat-registry-etcd` | etcd サービス登録 | 4 |
| 五期 | `ecat-mq-kafka` | Kafka メッセージキュー | 1 |
| 五期 | `ecat-data-opensearch` | OpenSearch 検索 | 1 |
| 五期 | `ecat-data-influxdb` | InfluxDB 時系列 | 2 |
| 五期 | `ecat-data-elasticsearch` | Elasticsearch 検索 | 2 |
| 五期 | `ecat-data-clickhouse` | ClickHouse OLAP | 1 |
| 五期 | `ecat-data-memcached` | Memcached キャッシュ | 3 |
| 五期 | `ecat-data-neo4j` | Neo4j グラフデータベース | 1 |
| 五期 | `ecat-data-nebulagraph` | NebulaGraph グラフデータベース | 1 |
| 五期 | `ecat-data-arangodb` | ArangoDB グラフデータベース | 1 |
| 五期 | `ecat-data-iotdb` | IoTDB 時系列 | 1 |
| 五期 | `ecat-data-questdb` | QuestDB 時系列 | 1 |
| 六期 | `ecat-transport-ws` | WebSocket サポート | 2 |
| 六期 | `ecat-versioning` | API バージョンルーティング | 2 |
| 六期 | `ecat-graphql` | GraphQL endpoint | 9 |
| 六期 | CI/CD テンプレート | GitHub Actions | — |

---

## 二、残りのギャップ（3 項目）

| # | ギャップ | 作業量 |
|---|------|--------|
| 1 | **transport への mTLS 接続** | 小 |
| 2 | **Redis レートリミットバックエンド** | 小 |
| 3 | **GitLab CI テンプレート** | 小 |

---

## 三、バージョンロードマップ

```
v1.0.x  コア骨格（18 crates）                    ✅ 完了
v2.0.x  エコシステム一期〜三期（+13 crates = 31 total）   ✅ 完了
v2.1.x  通信とセキュリティ + データバックエンド + 運用体験             ✅ 完了（現在 47 crates）
```

## 四、エコシステムに含めないもの

| 要件 | 方針 | 理由 |
|------|------|------|
| API ゲートウェイ | Kong / Envoy | 言語非依存 |
| サービスメッシュ | Linkerd | Rust に成熟したソリューションなし |
| コンテナオーケストレーション | Kubernetes | 業界標準 |
| ログ収集 | Vector | Rust ネイティブ |
