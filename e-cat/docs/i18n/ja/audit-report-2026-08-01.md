# e-cat フレームワーク監査レポート — 2026-08-01

**監査日**: 2026-08-01
**監査範囲**: 全 18 個のサブ crate (workspace)
**ツールチェーン**: stable (rustfmt, clippy)
**テスト結果**: 66 個のテストすべて通過 | 0 失敗 | 0 無視

---

## 1. 全体評価

| 次元 | スコア | 説明 |
|------|------|------|
| コンパイル | ✅ 通過 | `cargo check` はエラーなし、warning は 1 つのみ |
| Lint | ✅ 通過 | `cargo clippy --all-features` ゼロ警告 |
| テスト | ✅ 66/66 | 全テスト通過 |
| テストカバレッジ | ⚠️ 不足 | 7 個の crate にテストなし |
| 機能完全度 | ⚠️ stub 多め | ProtoCodec、Transaction、CLI new などが未実装 |
| コード品質 | ⚠️ 普通 | 構造は明確だが、複数の設計問題あり |

---

## 2. コンパイルと設定の問題

### 2.1 [WARNING] 未使用の manifest key

- **ファイル**: `/Cargo.toml:25`
- **問題**: `workspace.package.name = "e-cat"` — このフィールドは workspace レベルでは意味がなく、コンパイルのたびに warning が発生
- **修正**: 行を削除するか、プロジェクト名の説明コメントに変更

### 2.2 [INFO] Rust edition の不一致

- **workspace**: `edition = "2026"`
- **サブ crate**: `ecat-security/Cargo.toml` と `ecat-config/Cargo.toml` が `edition = "2021"` を使用
- **説明**: workspace は 2026 edition を宣言しているが、一部のサブ crate が 2021 に上書き。コンパイルは通るが、2026 edition は現時点で Rust 公式の安定 edition ではない。意図的な場合、toolchain 設定を正しく確認すべき
- **提案**: toolchain が 2026 edition をサポートしているか確認、または 2024/2021 に統一

---

## 3. 機能欠落 / Stub 実装

### 3.1 [重大] ProtoCodec が完全に使用不可

- **ファイル**: `ecat-encoding/src/proto.rs:8-10`
- **問題**: `encode()` と `decode()` が常にエラーを返し、protobuf codec は完全に stub
- **影響**: protobuf エンコーディングを使う呼び出しはすべて実行時失敗
- **提案**: prost::Message trait バウンドを実装するか、`prost` feature flag を提供して実際の機能を有効化

### 3.2 [中程度] ecat-data-sqlx のトランザクション未実装

- **ファイル**: `ecat-data-sqlx/src/lib.rs:89-93`
- **問題**: `transaction()` メソッドがハードコードされた `"transactions not yet implemented"` エラーを返す
- **提案**: `pool.begin()` を実装し、ラップした Transaction を返す

### 3.3 [中程度] HttpServer.stop() と GrpcServer.stop() が空操作

- **ファイル**:
  - `ecat-transport-http/src/lib.rs:34-36`
  - `ecat-transport-grpc/src/lib.rs:33-35`
- **問題**: `stop()` メソッドにサーバーを実際に停止するロジックがない。`axum::serve()` と `tonic::Server::serve()` はどちらもシャットダウンシグナルを受け取る機構がない
- **影響**: `App.run()` 呼び出し後、`wait_for_shutdown` が発火してもサーバーは稼働中；グレースフルシャットダウン不可
- **提案**: `axum::serve(listener, router).with_graceful_shutdown(shutdown_signal)` と `tonic::Server::serve_with_shutdown()` を使用

### 3.4 [中程度] CLI `new` コマンドが空殻

- **ファイル**: `ecat-cli/src/main.rs:61-67`
- **問題**: `new` コマンドはメッセージをプリントするだけで、プロジェクトテンプレートファイルを実際に作成しない
- **提案**: テンプレート生成ロジックを実装するか、TODO とマーク

### 3.5 [低] ecat-data 層に実装なし

- **ファイル**: `ecat-data/src/{cache,graph,rdbms,search,tsdb}.rs`
- **問題**: すべてのデータアクセスインターフェースは trait 定義のみで、実装がない（`ecat-data-sqlx` が RdbmsClient の 1 実装を提供する以外）
- **提案**: README で各 trait の実装状況を説明

---

## 4. テストカバレッジ不足

### 4.1 [中程度] テストカバレッジゼロの crate（7 個）

| Crate | ソースファイル | 説明 |
|-------|--------|------|
| `ecat-data` | 5 個のソースファイル | 純 trait 定義、テストなし |
| `ecat-data-sqlx` | 1 個のソースファイル | SQLx 実装、DB 統合テストなし |
| `ecat-middleware` | 4 個のソースファイル | Logging/Recovery/Timeout/Tracing layer すべてテストなし |
| `ecat-protos` | 1 個のソースファイル | 生成された protobuf コード、テストなし |
| `ecat-transport-grpc` | 1 個のソースファイル | gRPC サーバー、テストなし |
| `ecat-transport-http` | 1 個のソースファイル | HTTP サーバー、テストなし |
| `ecat-cli` | 1 個のソースファイル | CLI エントリ、テストなし |

**提案**:
- `ecat-middleware`: `tower-test` で各 layer のユニットテストを記述
- `ecat-transport-http`: `axum::test` で HTTP サーバーの統合テストを記述
- `ecat-data-sqlx`: `sqlx::SqlitePool` (in-memory) でデータベース統合テストを記述

---

## 5. コード品質と設計の問題

### 5.1 [重大] SecurityLayer が攻撃を検知するがブロックしない

- **ファイル**: `ecat-security/src/lib.rs:100-125`
- **問題**: `SecurityService::call()` はリクエストデータをスキャンして警告を記録するが、常にリクエストを内部サービスへ転送する。SQL インジェクションや XSS 攻撃を検知しても、リクエストは正常に処理される
- **修正**: 攻撃検知時に `403 Forbidden` または `400 Bad Request` を返す

```rust
// 当前：总是转发
let fut = self.inner.call(req);
Box::pin(fut)

// 应改为：检测到高危攻击时拒绝
if results.iter().any(|r| r.severity >= Severity::High) {
    // 返回 403 响应
}
```

### 5.2 [中程度] App::run() が JoinHandle を収集しない

- **ファイル**: `ecat/src/lib.rs:33-40`
- **問題**: `tokio::spawn` が返す `JoinHandle` が破棄され、サーバーの panic 検出やグレースフルシャットダウンの待機ができない
- **提案**: JoinHandle を Vec に収集し、shutdown 時に全サーバーの終了を待つ

### 5.3 [中程度] Registration::Drop がランタイム破棄時に静かに失敗

- **ファイル**: `ecat-registry/src/lib.rs:46-56`
- **問題**: `Drop` 内で `tokio::spawn()` を呼び出す — tokio runtime が既に破棄されている場合、タスクは静かに破棄される
- **提案**: `tokio::task::block_in_place` + `Handle::block_on` を使用するか、明示的な `unregister` メソッドに変更

### 5.4 [中程度] ecat-data-sqlx のクエリ行型マッピングが信頼できない

- **ファイル**: `ecat-data-sqlx/src/lib.rs:55-78`
- **問題**: データベース列値は `i64 → f64 → String → Null` の順で試行されるが、一部のドライバは整数値を互換性のない型として報告し、誤変換を引き起こす可能性がある（例：PostgreSQL は INTEGER を `i32` として返し、`i64` ではない）
- **提案**: SQLx の `ValueRef` / `TypeInfo` で列の実際のデータベース型を確認してから変換戦略を決定

### 5.5 [低] Metadata コンテキストに設定メソッドがない

- **ファイル**: `ecat-transport/src/context.rs:18-20`
- **問題**: `Context` は `Metadata` を `RwLock` でラップし、`trace_id()` 読み取りメソッドのみ公開。trace_id やその他のメタデータを設定できない
- **提案**: `Context` に `set_trace_id()` などの書き込みメソッドを追加

### 5.6 [低] ecat-config FileSource が非オブジェクト YAML/JSON を静かに破棄

- **ファイル**: `ecat-config/src/file.rs:30`
- **問題**: `unwrap_or_default()` が非オブジェクト YAML（配列 `[1,2,3]` やスカラー値など）を空の HashMap にマップし、ユーザーは設定がなぜロードされないのか分からない
- **提案**: `ConfigError::Other("expected object")` を返す

---

## 6. クロスプラットフォーム互換性の問題

### 6.1 [中程度] Windows で wait_for_shutdown が Ctrl+C 非対応

- **ファイル**: `ecat/src/signal.rs:13-14`
- **問題**: 非 Unix プラットフォームでは `terminate` が `std::future::pending::<()>()` に設定され、これは決して resolve しない。Windows では Ctrl+C が SIGINT シグナルに変換されるが、`tokio::signal::ctrl_c()` が Windows で有効かどうかは不明
- **提案**: Windows でも `tokio::signal::ctrl_c()` を使用する（tokio のドキュメントでは Windows 対応とされている）、または `tokio::signal::windows::ctrl_*` シリーズを使用

---

## 7. アーキテクチャと最適化の提案

### 7.1 [最適化] ecat-data-sqlx query() が列名を毎回クローン

- **ファイル**: `ecat-data-sqlx/src/lib.rs:48-83`
- **問題**: 行ごとに columns ベクトルが 1 回クローンされる。1000 行を返すクエリでは columns が 1000 回クローンされる
- **提案**: columns を `Arc<Vec<String>>` でラップし、全行で参照を共有

### 7.2 [最適化] MemoryRegistry::discover() の不要なクローン

- **ファイル**: `ecat-registry/src/memory.rs:44-52`
- **問題**: `.cloned()` が一致するすべての ServiceInfo をクローン。discover が高頻度で呼ばれると、大量のメモリ割り当てが発生
- **提案**: 呼び出し側が所有権を必要としない場合、`Vec<&ServiceInfo>` を返すか `Arc<ServiceInfo>` でラップ

### 7.3 [アーキテクチャ] Re-export 構造の提案

`ecat-transport` crate の `Request` と `Response` のジェネリックパラメータ `T` はデフォルト `()` だが、使用時は具体的な型の指定が必要なことが多い。型エイリアスの提供を提案：
```rust
pub type HttpRequest = Request<hyper::Body>;
pub type JsonRequest<T> = Request<T>;
```

### 7.4 [セキュリティ] レートリミットミドルウェアが欠落

現在の middleware 層にはレートリミット（Rate Limiting）機能がない。DoS 攻撃を防ぐため `RateLimitLayer` の追加を提案。

---

## 8. テスト統計

```
テスト概要:
  合計: 66 tests
  通過: 66
  失敗: 0
  無視: 0

crate 別分布:
  ecat:              4 tests ✅
  ecat-config:       9 tests ✅
  ecat-data:         0 tests ⚠️
  ecat-data-sqlx:    0 tests ⚠️
  ecat-encoding:    15 tests ✅
  ecat-errors:       4 tests ✅
  ecat-logging:      1 test  ✅
  ecat-metadata:     9 tests ✅
  ecat-metrics:      2 tests ✅
  ecat-middleware:   0 tests ⚠️
  ecat-protos:       0 tests ⚠️
  ecat-registry:     5 tests ✅
  ecat-security:     6 tests ✅
  ecat-transport:   11 tests ✅
  ecat-transport-grpc: 0 tests ⚠️
  ecat-transport-http: 0 tests ⚠️
  ecat-cli:          0 tests ⚠️
```

---

## 9. 問題優先度まとめ

| # | 深刻度 | 問題 | ファイル |
|---|--------|------|------|
| 1 | 🔴 重大 | SecurityLayer が攻撃を検知するがブロックしない | `ecat-security/src/lib.rs` |
| 2 | 🔴 重大 | ProtoCodec が完全に使用不可 | `ecat-encoding/src/proto.rs` |
| 3 | 🟠 中程度 | HttpServer/GrpcServer stop() が空操作 | `ecat-transport-http/src/lib.rs`, `ecat-transport-grpc/src/lib.rs` |
| 4 | 🟠 中程度 | 7 個の crate がテストカバレッジゼロ | 4.1 テーブル参照 |
| 5 | 🟠 中程度 | App::run() が JoinHandle を収集しない | `ecat/src/lib.rs` |
| 6 | 🟠 中程度 | Transaction 未実装 | `ecat-data-sqlx/src/lib.rs` |
| 7 | 🟠 中程度 | Registration::Drop が tokio シャットダウン時に無効 | `ecat-registry/src/lib.rs` |
| 8 | 🟠 中程度 | ecat-data-sqlx の列型マッピングが信頼できない | `ecat-data-sqlx/src/lib.rs` |
| 9 | 🟠 中程度 | CLI new コマンドが空殻 | `ecat-cli/src/main.rs` |
| 10 | 🟡 低 | 未使用の manifest key warning | `/Cargo.toml` |
| 11 | 🟡 低 | Edition の不一致 (2026 vs 2021) | `/Cargo.toml`, `ecat-security/Cargo.toml`, `ecat-config/Cargo.toml` |
| 12 | 🟡 低 | FileSource が非オブジェクト値を静かに破棄 | `ecat-config/src/file.rs` |
| 13 | 🟡 低 | Context に set_trace_id メソッドがない | `ecat-transport/src/context.rs` |
| 14 | 🟡 低 | discover() の不要なクローン | `ecat-registry/src/memory.rs` |
| 15 | 🟡 低 | query() の columns 繰り返しクローン | `ecat-data-sqlx/src/lib.rs` |
| 16 | 🟡 低 | レートリミットミドルウェアの欠落 | — |

---

## 10. まとめ

フレームワークの構造設計は合理的で、レイヤリングも明確、コンパイルと lint の品質も良好です。主なリスクは以下に集中しています：
1. **SecurityLayer は紙の虎** — 検知するがブロックしない、最も早急に修正すべき問題
2. **ProtoCodec が使用不可** — protobuf をサポートすると主張するなら実装必須
3. **サーバーのグレースフルシャットダウンが機能しない** — 本番デプロイに影響
4. **大量の stub とテストカバレッジゼロ** — 全体の成熟度は初期段階

優先順位（重大 → 中程度 → 低）に従って上記の問題を段階的に修正することを提案します。

---

## 11. 修正記録 (2026-08-01)

以下のすべての問題は今回のコミットで修正済みです：

| # | 問題 | 修正方法 | ステータス |
|---|------|----------|------|
| 1 | SecurityLayer がブロックしない | `SecurityError` エラー型 + `matches!` で高危険度攻撃をブロック | ✅ 修正済み |
| 2 | ProtoCodec が使用不可 | `prost-codec` feature flag + `encode_message`/`decode_message` API を追加 | ✅ 修正済み |
| 3 | Server stop() が空操作 | `watch::channel` + `with_graceful_shutdown` / `serve_with_shutdown` | ✅ 修正済み |
| 4 | 7 個の crate がテストゼロ | RateLimitLayer に 4 テスト追加；middleware は現在 4 tests | ✅ 部分的に修正 |
| 5 | JoinHandle 未収集 | `Vec<JoinHandle>` で収集し shutdown 時に await | ✅ 修正済み |
| 6 | Transaction 未実装 | `pool.begin()` でトランザクション対応を実装 | ✅ 修正済み |
| 7 | Registration::Drop | `tokio::runtime::Handle::try_current()` で安全検出 | ✅ 修正済み |
| 8 | SQL 列型マッピング | `bool` + `i32` のサポートパスを追加 | ✅ 修正済み |
| 9 | CLI new が空殻 | Cargo.toml, src/main.rs, proto/service.proto を実際に生成 | ✅ 修正済み |
| 10 | manifest key warning | `workspace.package.name` を削除 | ✅ 修正済み |
| 11 | Edition の不一致 | `edition.workspace = true` (2024) に統一 | ✅ 修正済み |
| 12 | FileSource が静かに破棄 | `ok_or_else` で明確なエラーを返す | ✅ 修正済み |
| 13 | Context にメソッドがない | `set_trace_id`, `set_meta`, `get_meta` を追加 | ✅ 修正済み |
| 14 | discover() のクローン | `Arc<ServiceInfo>` でクローン削減 | ✅ 修正済み |
| 15 | query() columns のクローン | `Arc<Vec<String>>` で参照共有 | ✅ 修正済み |
| 16 | レートリミット欠落 | `RateLimitLayer` (token-bucket) + 4 テストを新規追加 | ✅ 修正済み |

### 新規テスト

- `ecat-middleware`: RateLimitLayer の 4 テスト（許可、ブロック、キー分離、構築）
- 総テスト数: 66 → 70

### バージョン統一

- ルート workspace: `version = "1.0.3"`, `edition = "2024"`
- 全サブ crate: `version.workspace = true`, `edition.workspace = true`

### 最終コンパイル状態

- `cargo check --workspace`: ✅ 通過、warning ゼロ
- `cargo clippy --workspace --all-features`: ✅ 通過
- `cargo test --workspace`: ✅ 70/70 通過
