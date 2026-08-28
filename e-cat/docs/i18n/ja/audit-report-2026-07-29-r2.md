<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat コードレビューレポート（第 2 ラウンド）

**日付**: 2026-07-29  
**ブランチ**: main  
**プロジェクト**: e-cat (Rust workspace, 17 個の crate)

---

## 一、レビュー概要

第 1 ラウンドの clippy 修正とテスト補充を踏まえ、本ラウンドでは深いコードロジックレビューを実施し、ランタイムの正確性、並行安全、API セマンティクスの一貫性を重点的に確認しました。合計 32 個のソースファイルをレビューしました。

### 検証ベースライン

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

---

## 二、発見された Bug と修正

### Bug 1：[重大] TracingLayer span ガードのライフサイクル誤り

- **ファイル**: `ecat-middleware/src/tracing.rs:37`
- **深刻度**: **高**
- **影響**: TracingLayer を通過するすべてのリクエストが tracing span でカバーされない

**根本原因分析**:

```rust
// 修复前
fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let _guard = span.enter();  // guard 在 call() 返回时 drop
    let fut = self.inner.call(req);
    Box::pin(fut)               // future 在后续 poll 时才执行
}
```

`span.enter()` が返す guard は現在の同期コンテキスト内でのみ span をアクティブに保ちます。`call()` が返すのはまだ poll されていない future であり、実際の非同期実行はその後の poll 段階で発生します — この時点で guard は既に drop されており、span は有効になりません。TracingLayer を通過するすべてのリクエストは tracing 出力に現れません。

**修正**:

```rust
// 修复后
use tracing::Instrument;

fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let fut = self.inner.call(req);
    Box::pin(fut.instrument(span))  // span 附着在 future 生命周期上
}
```

`tracing::Instrument::instrument()` を使用して span を future に付着させ、future の全 poll ライフサイクルにわたって span がアクティブであることを保証します。

---

### Bug 2：[重大] LifecycleHook クロージャ実装の欠陥 — on_stop が実行されない

- **ファイル**: `ecat/src/hook.rs:14-23`、`ecat/src/lib.rs:11-16`
- **深刻度**: **高**
- **影響**: `.on_stop()` で登録したクロージャ hook が shutdown 時に何も実行しない

**根本原因分析**:

元の設計では、`on_start()` と `on_stop()` の両メソッドが hook を同じ `lifecycle_hooks` Vec にプッシュしていました。`run()` 時にはすべての hook が順に `on_start()` を呼び出し、shutdown 時にはすべての hook が順に `on_stop()` を呼び出します。

問題は `LifecycleHook` trait のクロージャ `Fn() -> Fut` に対する blanket impl にあります：**`on_start()` のみを実装し、`on_stop()` は trait のデフォルト実装（no-op）のまま**でした。

つまり、ユーザーがクロージャ構文 `.on_stop(|| async { ... })` を使うと、クロージャは hooks リストに追加されますが、shutdown 時にはデフォルトの空の `on_stop()` しか実行されず、ユーザーのロジックは決して実行されません。

**修正（2 部構成）**:

1. **start_hooks と stop_hooks の分離**（`ecat/src/lib.rs`）：

```rust
// App 结构体 — 两个独立的 Vec
pub struct App {
    start_hooks: Vec<Box<dyn LifecycleHook>>,
    stop_hooks: Vec<Box<dyn LifecycleHook>>,
    // ...
}

// on_start() → start_hooks, on_stop() → stop_hooks
pub fn on_start(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.start_hooks.push(Box::new(hook));
    self
}
pub fn on_stop(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.stop_hooks.push(Box::new(hook));
    self
}
```

2. **クロージャ blanket impl の補完**（`ecat/src/hook.rs`）：

```rust
impl<F, Fut> LifecycleHook for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Result<...>> + Send,
{
    async fn on_start(&self) -> ... { (self)().await }
    async fn on_stop(&self) -> ...  { (self)().await }  // 新增
}
```

これでクロージャは `on_start` と `on_stop` の両方を実装し、分離された Vec と組み合わせることで、各 hook が正しいライフサイクル段階でのみ呼び出されます。

---

### Bug 3：[中程度] SqlxClient Row 値型の抽出優先順位の誤り

- **ファイル**: `ecat-data-sqlx/src/lib.rs:53-68`
- **深刻度**: 中
- **影響**: データベースの整数型・浮動小数点型の値が JSON 数値ではなく JSON 文字列として抽出される

**根本原因分析**:

`try_get::<String>()` が最初に試行されていました。多くのデータベースドライバは数値カラムに対して `try_get::<String>()` を成功させることができ（暗黙的変換）、整数値 `42` が `42` ではなく `"42"` として抽出されていました。

**修正**: `try_get` の試行順序を `i64 → f64 → String → Null` に調整し、数値型を優先的に保持します。

---

## 三、その他のレビュー発見（未修正 / 既知の制限）

| カテゴリ | ファイル | 説明 | 提案 |
|------|------|------|------|
| 機能未完了 | `ecat-transport-http/src/lib.rs:30` | `axum::serve().await` がブロックして戻らず、`stop()` は空操作 | graceful shutdown を実装 |
| 機能未完了 | `ecat-transport-grpc/src/lib.rs:29` | 同上 | graceful shutdown を実装 |
| 機能未完了 | `ecat-data-sqlx/src/lib.rs:79` | `transaction()` が未実装エラーを返す | トランザクション対応を実装 |
| コードスタイル | `ecat-middleware/src/logging.rs:42` | `elapsed.as_millis() as u64` u128→u64 の理論上の切り捨て | 実質的な影響なし |
| テスト欠落 | `ecat-middleware/` | 4 個の Tower Service にユニットテストなし | 統合テストが必要 |
| テスト欠落 | `ecat-data/` | 純粋な trait 定義 | 現状で許容可能 |
| RwLock ブロッキング | `ecat-registry/src/memory.rs` | 同期 RwLock が非同期コンテキストでブロックする可能性 | tokio::sync::RwLock への変更を検討 |

---

## 四、テスト結果

```
cargo test → 60 passed, 0 failed

crate 別分布:
  ecat                  4   (Builder/デフォルト値/ライフサイクル hook)
  ecat-config           9   (env parse ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (HTTPマッピング/gRPC変換/metadata/Display)
  ecat-logging          1   (initスモーク)
  ecat-metadata         9   (アクセス/From HeaderMap/From MetadataMap/イテレータ)
  ecat-metrics          2   (シングルトン/textがpanicしない)
  ecat-registry         5   (登録/ディスカバリ/登録解除/一覧/フィルタ)
  ecat-transport       11   (Context/Request/Response/Server trait)
  その他 8 crate          0   (純trait/コード生成/統合テスト必要/プリントのみ)
```

---

## 五、変更ファイル一覧

| ファイル | 変更タイプ | 変更説明 |
|------|----------|----------|
| `ecat/src/lib.rs` | Bug 修正 | App が start_hooks/stop_hooks を分離；AppBuilder を対応更新；テスト適応 |
| `ecat/src/hook.rs` | Bug 修正 | クロージャ blanket impl に on_stop() 実装を補完 |
| `ecat-middleware/src/tracing.rs` | Bug 修正 | span ガード → `fut.instrument(span)` |
| `ecat-data-sqlx/src/lib.rs` | Bug 修正 | Row 値抽出順序 i64→f64→String→Null |

---

## 六、まとめ

本ラウンドでは高深刻度のランタイム Bug 2 件と中程度のデータ正確性問題 1 件を発見しました：

1. **TracingLayer span 無効** — 全リクエストの可観測性に影響
2. **LifecycleHook on_stop 未実行** — 全 shutdown ロジックの正確性に影響
3. **Row 数値型の消失** — データベースクエリ結果の型正確性に影響

3 件すべて修正済みで、修正後は全 60 テストが通過、コンパイルエラー・警告ゼロです。

### 今後の提案

- HTTP/gRPC server に graceful shutdown を実装
- `ecat-middleware` に統合テストを追加（mock Service + span/タイムアウト/リカバリ挙動の検証）
- `ecat-data-sqlx` に統合テストを追加（SQLite メモリデータベース使用）
- `ecat-registry/memory.rs` の同期 RwLock を `tokio::sync::RwLock` に置き換え
