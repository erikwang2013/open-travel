<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat API リファレンス

本ページは Ecat フレームワークのインターフェース（API）面をまとめたものです：ポート規約、組み込みエンドポイント、エラーフォーマット、拡張インターフェース。ビジネスルートは各サービスが自ら登録します。

## ポート規約

| プロトコル | 待ち受けアドレス | 説明 |
|------|----------|------|
| HTTP | `0.0.0.0:8000` | axum ルーティング、デフォルトのサンプルポート |
| gRPC | `0.0.0.0:9000` | tonic Server、デフォルトのサンプルポート |

## 組み込みエンドポイント

以下のエンドポイントはエコシステム crate が提供し、サービスとともにマウントされます：

| エンドポイント | 提供元 | 説明 |
|------|------|------|
| `/health` | ecat-health | 生存チェック（サービス名、バージョン、起動時間を返す） |
| `/ready` | ecat-health | 準備完了チェック（依存関係が準備完了後に 200 を返す） |
| `/metrics` | ecat-metrics | Prometheus メトリクス公開（`ecat_http_requests_total` / `ecat_http_request_duration_seconds`） |
| `/{service}/{method}` | ユーザールート | 例：`/helloworld/ecat` |

> メトリクスエンドポイントはパスに ID 等の高カーディナリティがある場合、`MetricsLayer::new().with_path_fn(...)` で正規化し、メトリクスのカーディナリティ爆発を防いでください。

## リクエスト処理フロー

```
客户端请求
  ├─ HTTP :8000 ──→ axum::Router ─┐
  └─ gRPC :9000 ──→ tonic::Server ─┤
                              ┌─────┴──────┐
                              │ Middleware │  Recovery→Tracing→Logging→Auth→Metrics→Security→CircuitBreaker
                              └─────┬──────┘
                                    ▼
                               Handler（tower::Service）
                                    ▼
                               Response（JSON/Protobuf 编码）
```

## エラーフォーマット

`ecat-errors` は `ErrorCode` + `Error` を提供し、コンパイル時に HTTP ステータスコードへマッピングします：

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

エラーレスポンスは middleware によって JSON（または Protobuf）にエンコードされ、code / reason / message を保持します。

## 拡張インターフェース

| 能力 | Crate | インターフェース |
|------|-------|------|
| GraphQL | ecat-graphql | `/graphql` エンドポイント；フィールドパラメータとネスト selection をサポート、エイリアス・fragment・複数トップレベルフィールドは未対応 |
| OpenAPI | ecat-openapi | ルートから OpenAPI spec を生成 |
| WebSocket | ecat-transport-ws | アップグレードされた WS トランスポート |
| API バージョンルーティング | ecat-versioning | `/v1/...` プレフィックスのバージョンルーティング |
| 認証 | ecat-auth | JWT / API Key ミドルウェア；JWT キーは ≥32 バイト必須、チェーンで `required_issuer`/`required_audience` を強制可能 |
| gRPC クライアント | ecat-transport-grpc | サービスディスカバリとロードバランシングを統合 |

## サービス間通信

- `HttpClient`（ecat-client）：サービスディスカバリとロードバランシングを統合、CircuitBreaker によるサーキットブレーカー保護
- `GrpcClient`（ecat-transport-grpc）：同上、gRPC プロトコル
- ミドルウェアは統一して `tower::ServiceBuilder` で組み合わせます（Recovery / Tracing / Logging / Timeout / RateLimit / Security / CircuitBreaker / Metrics / Retry / Validate / CORS）

## データバックエンドインターフェース

すべてのデータバックエンド（`ecat-data-*`）は統一 trait（`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`）で抽象化されています；REST 系バックエンド（Neo4j / NebulaGraph / ArangoDB / InfluxDB / IoTDB / QuestDB / TDengine / OpenSearch / Elasticsearch / S3）は `base_url` ベースで対応する HTTP インターフェースにアクセスします。接続設定は [データベース設定チュートリアル](database-config-tutorial.md) を参照してください。
