<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat コードレビューと TDD テストレポート

**日付**: 2026-07-29  
**ブランチ**: main  
**プロジェクト**: e-cat (Rust workspace, 17 個の crate)

---

## 一、レビュー範囲

workspace の全 17 個の crate にあるすべての Rust ソース（38 個の `.rs` ファイル）をレビューしました。

| Crate | 説明 | ファイル数 |
|-------|------|--------|
| `ecat-protos` | Protobuf 定義とコード生成 | 2 |
| `ecat-errors` | 統一エラー型 | 2 |
| `ecat-metadata` | リクエストメタデータ抽象 | 1 |
| `ecat-encoding` | JSON/Protobuf エンコード・デコード | 3 |
| `ecat-logging` | ログ/Tracing 初期化 | 1 |
| `ecat-config` | 設定ロード（ファイル/環境変数） | 3 |
| `ecat-data` | データ層 trait 抽象 | 5 |
| `ecat-data-sqlx` | SQLx RDBMS 実装 | 1 |
| `ecat-registry` | サービス登録・ディスカバリ | 2 |
| `ecat-metrics` | Prometheus メトリクス | 1 |
| `ecat-middleware` | Tower ミドルウェア層 | 4 |
| `ecat-transport` | トランスポート層抽象 | 4 |
| `ecat-transport-http` | HTTP/Axum トランスポート実装 | 1 |
| `ecat-transport-grpc` | gRPC/Tonic トランスポート実装 | 1 |
| `ecat` | アプリケーションフレームワークコア | 3 |
| `ecat-cli` | CLI ツール | 1 |
| `examples/helloworld` | サンプルプロジェクト | 1 |

---

## 二、発見された問題と修正

### 問題 1：[Clippy] `map_identity` — 意味のない identity map

- **ファイル**: `ecat-config/src/file.rs:30`
- **深刻度**: 低
- **問題**: `map(|(k, v)| (k, v))` は何の変換も行わず、無効なコード
- **修正**: 不要な `.map()` 呼び出しを削除

### 問題 2：[Clippy] `new_without_default` — Config に Default 実装がない

- **ファイル**: `ecat-config/src/lib.rs:27`
- **深刻度**: 低
- **問題**: `Config` に `new()` メソッドがあるが `Default` trait を実装していない
- **修正**: 手動実装を `#[derive(Default)]` に置き換え

### 問題 3：[Clippy] `io_other_error` — 旧式の Error 構築

- **ファイル**: `ecat-middleware/src/recovery.rs:42`
- **深刻度**: 低
- **問題**: `std::io::Error::new(std::io::ErrorKind::Other, ...)` にはより簡潔な代替がある
- **修正**: `std::io::Error::other("task panicked")` に変更

### 問題 4：[Clippy] `redundant_async_block` — 冗長な async ブロック

- **ファイル**: `ecat-middleware/src/tracing.rs:38`
- **深刻度**: 低
- **問題**: `Box::pin(async move { fut.await })` の async ブロックは不要
- **修正**: `Box::pin(fut)` に簡略化

### 問題 5：[Clippy] `redundant_closure` — 冗長なクロージャ

- **ファイル**: `ecat-data-sqlx/src/lib.rs:63`
- **深刻度**: 低
- **問題**: `.and_then(|f| serde_json::Number::from_f64(f))` のクロージャは省略可能
- **修正**: 直接 `.and_then(serde_json::Number::from_f64)` を使用

### 問題 6：[Clippy] `unwrap_or_default` — unwrap_or_default で簡略化可能

- **ファイル**: `ecat-transport-http/src/lib.rs:27`
- **深刻度**: 低
- **問題**: `unwrap_or_else(Router::new)` は `unwrap_or_default()` と等価
- **修正**: `unwrap_or_default()` に変更

---

## 三、テストカバレッジ状況

### 修正前

| Crate | テスト数 |
|-------|--------|
| `ecat-errors` | 4 |
| `ecat-transport` | 11 |
| その他 15 個の crate | **0** |
| **合計** | **15** |

### 修正後

| Crate | テスト数 | 追加 | テスト内容 |
|-------|--------|------|----------|
| `ecat-encoding` | 15 | +15 | JsonCodec エンコード・デコード往復、不正デコード、content_type；CodecBox ディスパッチ；codec_from_content_type 正常/エラーパス；Encoding バリアント |
| `ecat-errors` | 4 | — | HTTP ステータスコードマッピング、gRPC ステータス変換、metadata 蓄積、Display フォーマット |
| `ecat-metadata` | 9 | +9 | キーバリューアクセス、trace_id、From\<HeaderMap\>（非 UTF-8 値スキップ含む）、From\<MetadataMap\>（ASCII およびバイナリスキップ）、IntoIterator |
| `ecat-logging` | 1 | +1 | init スモークテスト |
| `ecat-config` | 4 | +4 | 新規作成/デフォルト値、型付き読み取り、ConfigSource からのロード |
| `ecat-registry` | 5 | +5 | 登録/ディスカバリ、登録解除/削除、不存在エラー、サービス一覧、名前フィルタ |
| `ecat-metrics` | 2 | +2 | シングルトン registry、metrics_text が panic しない |
| `ecat` | 4 | +4 | Builder デフォルト値、カスタム名/バージョン、server 登録、lifecycle hook |
| `ecat-transport` | 11 | — | Context/Request/Response の作成とデフォルト値、Server trait |
| **合計** | **55** | **+40** | |

### ユニットテスト不要の crate

- `ecat-protos` — protobuf コード生成のみ
- `ecat-data` — 純粋な trait 定義、実装ロジックなし
- `ecat-data-sqlx` — データベース接続が必要、統合テストの範疇
- `ecat-middleware` — Tower Service 実装、統合テストが必要
- `ecat-transport-http` / `ecat-transport-grpc` — ネットワーク待ち受けが必要、統合テストの範疇
- `ecat-cli` — 出力をプリントするのみ、ロジックなし

---

## 四、検証結果

```
cargo test   → 55 passed, 0 failed
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
```

---

## 五、変更ファイル一覧

| ファイル | 変更 |
|------|------|
| `ecat-config/src/file.rs` | identity map を削除 |
| `ecat-config/src/lib.rs` | `#[derive(Default)]` + 4 テスト |
| `ecat-data-sqlx/src/lib.rs` | 冗長クロージャを簡略化 |
| `ecat-middleware/src/recovery.rs` | `std::io::Error::other()` を使用 |
| `ecat-middleware/src/tracing.rs` | 冗長 async ブロックを削除 |
| `ecat-transport-http/src/lib.rs` | `unwrap_or_else` → `unwrap_or_default` |
| `ecat-metrics/src/lib.rs` | 2 テスト |
| `ecat-registry/src/memory.rs` | 5 テスト |
| `ecat/src/lib.rs` | 4 テスト |
