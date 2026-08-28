# e-cat フレームワーク監査レポート R2 — 2026-08-01

**バージョン**: 1.0.5
**範囲**: 全 18 個のサブ crate
**結論**: `cargo check` / `cargo clippy --all-features` / `cargo test` すべて通過、70 tests ✅

---

## 1. 前回修正の振り返り（16/16 修正済み）

前回監査（R1）で発見された問題はすべて修正済み：SecurityLayer の攻撃ブロック、ProtoCodec の prost 対応、Server のグレースフルシャットダウン、JoinHandle 収集、Transaction 実装、Registration Drop の安全検出、列型マッピング強化、CLI new のファイル生成、バージョン/edition 統一、FileSource のエラーハンドリング、Context のメタデータメソッド、discover の Arc 最適化、query columns の Arc 最適化、RateLimitLayer 新規追加。

---

## 2. 本ラウンドの新発見問題

### 2.1 [重大] CLI `new` が生成するテンプレートコードがコンパイル不可

- **ファイル**: `ecat-cli/src/main.rs:79-97`
- **問題**: 生成される `Cargo.toml` は `workspace = true` の依存参照と `path = "../ecat"` の相対パスを使用するが、`ecat new myapp` が作成する独立プロジェクトは e-cat workspace 内にないため、これらの参照はすべて解決に失敗する
- **影響**: `ecat new` で作成したプロジェクトはそもそもコンパイルできない
- **修正**: テンプレートは workspace 参照ではなく、バージョン付きの実際の依存関係を使用すべき

```toml
# 当前（错误）：
tokio.workspace = true           # 项目不在 workspace 中，报错
ecat = { path = "../ecat" }      # 相对路径无效

# 应改为：
tokio = { version = "1", features = ["full"] }
ecat = "1.0.5"
```

### 2.2 [重大] ecat-data-sqlx `transaction()` が実際の DB トランザクションハンドルを破棄

- **ファイル**: `ecat-data-sqlx/src/lib.rs:100-106`
- **問題**: `pool.begin()` は実際のデータベーストランザクションハンドル `Transaction<'_, DB>` を返すが、コードは `_tx` にバインドした直後に破棄する。`_tx` が drop されると、データベーストランザクションは自動ロールバックされる。返される `ecat_data::Transaction` は空殻であり、その `commit()/rollback()` メソッドは全く効果がない
- **影響**: `transaction()` を使うコードはすべてトランザクション保護なしで実行され、データ整合性が保証されない
- **修正**: `ecat_data::Transaction` 構造体を再設計し、実際のデータベーストランザクションハンドルを保持させる

### 2.3 [中程度] SecurityLayer がリクエストボディをスキャンしない

- **ファイル**: `ecat-security/src/lib.rs:117-127`
- **問題**: `call()` は URI と HTTP ヘッダーのみスキャンし、リクエストボディは完全に検査しない。攻撃者は SQL インジェクション/XSS payload を POST body に入れて簡単に検知を回避できる
- **影響**: 攻撃検知の有効カバレッジが大幅に低下
- **修正**: body スキャン機能を追加するか、呼び出し側が body を読んだ後に使える `scan_body()` 公開メソッドを提供

### 2.4 [中程度] RateLimitLayer が同期 Mutex + 期限切れクリーンアップなし

- **ファイル**: `ecat-middleware/src/ratelimit.rs:10-38`
- **問題 1**: `std::sync::Mutex` を async コンテキストで使用 — ロック競合が発生すると tokio worker スレッド全体をブロック
- **問題 2**: `buckets: HashMap<String, (u32, Instant)>` が期限切れキーを一切クリーンアップせず、長期稼働サーバーのメモリが無限に増加（新しい IP/key ごとにメモリを永久占有）
- **影響**: 高並行下で性能低下、長時間稼働後にメモリリーク
- **修正**: `tokio::sync::Mutex` に変更し、`allow()` 内で期限切れエントリを定期的にクリーンアップ

### 2.5 [中程度] ecat-data-sqlx に裸 SQL のパラメータ化 API がない

- **ファイル**: `ecat-data-sqlx/src/lib.rs:24-29, 32-36`
- **問題**: `execute(&self, sql: &str)` と `query(&self, sql: &str)` は生の SQL 文字列のみを受け付け、trait レベルにパラメータバインドメソッドがない。呼び出し側がユーザー入力を SQL に連結すると SQL インジェクションになる
- **影響**: trait 自体は直接のセキュリティ脆弱性を露出しないが、パラメータ化 API がないと呼び出し側が安全でないコードを書く誘因になる
- **提案**: `RdbmsClient` trait に `execute_with` と `query_with` メソッドを追加し、パラメータバインドを使用

### 2.6 [低] query() の Arc::clone が依然としてクロージャ内

- **ファイル**: `ecat-data-sqlx/src/lib.rs:50-53`
- **問題**: `let cols = std::sync::Arc::clone(&columns)` が `rows.iter().map()` クロージャ内で実行される。Arc::clone は軽量（アトミック参照カウント増加のみ）だが、行ごとのアトミック操作を避けるためクロージャ外に出すことができる
- **提案**: `iter()` の前に 1 回クローンし、クロージャ内ではそのクローンをキャプチャ

### 2.7 [低] ProtoCodec の trait impl と新 API の不一致

- **ファイル**: `ecat-encoding/src/proto.rs`
- **問題**: `Codec` trait の `encode/decode` は依然としてエラーのみを返す；新しく追加された `encode_message/decode_message` が正しいパスだが、メソッド名が trait と一致しない。利用者は `codec.encode()` を先に試して、失敗理由に困惑する可能性がある
- **提案**: ドキュメント/コメントで説明：proto 型は Codec trait メソッドではなく `encode_message/decode_message` を使用すべき

---

## 3. 現在の状態概要

| 次元 | ステータス |
|------|------|
| `cargo check` | ✅ warning ゼロ |
| `cargo clippy --all-features` | ✅ ゼロ警告 |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 通過 |
| バージョン統一 | ✅ 1.0.5 |
| Edition 統一 | ✅ 2024 |

### テスト分布

| Crate | Tests | 説明 |
|-------|-------|------|
| ecat | 4 | ✅ |
| ecat-config | 9 | ✅ |
| ecat-encoding | 15 | ✅ |
| ecat-errors | 4 | ✅ |
| ecat-logging | 1 | ✅ |
| ecat-metadata | 9 | ✅ |
| ecat-metrics | 2 | ✅ |
| ecat-middleware | 4 | ✅ (RateLimitLayer 含む) |
| ecat-registry | 5 | ✅ |
| ecat-security | 6 | ✅ |
| ecat-transport | 11 | ✅ |
| ecat-data | 0 | — (純 trait 定義) |
| ecat-data-sqlx | 0 | ⚠️ DB 統合テストなし |
| ecat-protos | 0 | — (生成コード) |
| ecat-transport-grpc | 0 | ⚠️ |
| ecat-transport-http | 0 | ⚠️ |
| ecat-cli | 0 | ⚠️ |

---

## 4. 問題優先度

| # | 深刻度 | 問題 | ファイル | ユーザー影響 |
|---|--------|------|------|----------|
| 1 | 🔴 | CLI `new` テンプレートがコンパイル不可のコードを生成 | `ecat-cli/src/main.rs:79` | 新ユーザーの最初のコマンドで失敗 |
| 2 | 🔴 | transaction() が実際の DB トランザクションハンドルを破棄 | `ecat-data-sqlx/src/lib.rs:100` | データ整合性が無保証 |
| 3 | 🟠 | SecurityLayer が body をスキャンしない | `ecat-security/src/lib.rs:117` | 攻撃者が検知を回避可能 |
| 4 | 🟠 | RateLimitLayer の std Mutex + メモリリーク | `ecat-middleware/src/ratelimit.rs:10,25` | 並行性能 + OOM |
| 5 | 🟠 | 裸 SQL にパラメータ化 API なし | `ecat-data-sqlx/src/lib.rs:24` | SQL インジェクションリスク |
| 6 | 🟡 | query() の Arc clone 位置 | `ecat-data-sqlx/src/lib.rs:53` | 微小な性能最適化 |
| 7 | 🟡 | ProtoCodec API の不一致 | `ecat-encoding/src/proto.rs` | 利用者の困惑 |

---

## 6. 修正記録 (2026-08-01 R2)

| # | 問題 | 修正方法 | ステータス |
|---|------|----------|------|
| 1 | CLI new テンプレートがコンパイル不可 | バージョン付き依存に変更 (`ecat = "1.0"`, `tokio = "1"` など) | ✅ |
| 2 | transaction() が DB トランザクションを破棄 | `Transaction::with_inner()` が実ハンドルを保持、sqlx は `Box<dyn Any>` 経由で受け渡し | ✅ |
| 3 | SecurityLayer が body をスキャンしない | `scan_body(&[u8])` 公開メソッドを新規追加 | ✅ |
| 4 | RateLimitLayer の Mutex + リーク | `tokio::sync::Mutex` + 100 key ごとに期限切れエントリをクリーンアップ | ✅ |
| 5 | 裸 SQL にパラメータ化 API なし | `RdbmsClient` に `execute_with`/`query_with` パラメータ化メソッドを追加 | ✅ |
| 6 | query() の Arc clone 位置 | `Arc::clone` を `iter()` 外に移動、全行で参照共有 | ✅ |
| 7 | ProtoCodec API の不一致 | モジュールレベルドキュメント + struct ドキュメントで使用方法を説明 | ✅ |

### 最終状態

| チェック項目 | 結果 |
|--------|------|
| `cargo check` | ✅ error ゼロ / warning ゼロ |
| `cargo clippy --all-features` | ✅ warning ゼロ |
| `cargo fmt --all` | ✅ |
| `cargo test --workspace` | ✅ 70/70 通過 |
| バージョン | 1.0.5 (すべて workspace 継承に統一) |
| Edition | 2024 |
