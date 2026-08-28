<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat コードレビューレポート（第 3 ラウンド）

**日付**: 2026-07-29  
**ブランチ**: main  
**プロジェクト**: e-cat (Rust workspace, 18 個の crate)  
**レビュー範囲**: 全 37 個のソースファイル、合計 2151 行の Rust コード

---

## 一、レビュー概要

第 2 ラウンドで発見された 3 件の Bug はすべて修正済みです。本ラウンドはクリーンなベースライン（0 error / 0 warning / 60 test passed）上での深い再レビューであり、境界条件、エラーハンドリング、本番堅牢性を重点的に確認しました。

### 検証ベースライン

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

### R2 Bug 修正の確認

| Bug | ファイル | ステータス |
|-----|------|------|
| TracingLayer span ガードのライフサイクル | `ecat-middleware/src/tracing.rs` | ✅ 修正済み |
| LifecycleHook on_stop 未実行 | `ecat/src/hook.rs`, `ecat/src/lib.rs` | ✅ 修正済み |
| Row 値型の抽出優先順位 | `ecat-data-sqlx/src/lib.rs` | ✅ 修正済み |

---

## 二、新たに発見された問題

### 問題 1：[中程度] `metrics_text()` で unwrap() を使用、本番環境で panic の可能性

- **ファイル**: `ecat-metrics/src/lib.rs:14-15`
- **深刻度**: **中程度**
- **影響**: `/metrics` エンドポイントにアクセスするとプロセスが panic する

**根本原因分析**:

```rust
pub fn metrics_text() -> String {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder.encode(&registry().gather(), &mut buffer).unwrap();  // 可能 panic
    String::from_utf8(buffer).unwrap()                           // 可能 panic
}
```

`TextEncoder::encode()` は内部 I/O エラーやシステムメモリ不足時に失敗します。`String::from_utf8()` も、Prometheus ライブラリが非 UTF-8 出力を生成した場合に失敗します。これら 2 つの `unwrap()` は非テストコードパス上にあり、HTTP handler から直接呼び出されるため、panic はプロセスクラッシュを引き起こします。

**推奨修正**: `Result<String, ...>` を返すか、`.unwrap_or_default()` でフォールバック処理します。

---

### 問題 2：[低] Recovery ミドルウェアが spawn した新 task で span コンテキストが失われる

- **ファイル**: `ecat-middleware/src/recovery.rs:40`
- **深刻度**: **低**
- **影響**: Recovery 層が Tracing 層より前にある場合、リクエストの trace_id がビジネスロジックに伝達されない

**根本原因分析**:

```rust
fn call(&mut self, req: Req) -> Self::Future {
    let fut = self.inner.call(req);
    Box::pin(async move {
        match tokio::task::spawn(fut).await {  // 新 task，不继承 span
            // ...
        }
    })
}
```

`tokio::task::spawn()` は新しい Tokio タスクを作成します。tracing span は task-local であり、自動的に伝達されません。

**提案**: ドキュメントでミドルウェアの順序要件を明確にする（Recovery を最外層に置く）、または spawn 前に `.instrument(span)` で手動伝達します。

---

### 問題 3：[低] Registration Drop がエラーを黙って破棄

- **ファイル**: `ecat-registry/src/lib.rs:50-52`
- **深刻度**: **低**
- **影響**: サービス登録解除の失敗を感知できない

```rust
impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(reg) = self.registry.take() {
            let id = self.id.clone();
            tokio::spawn(async move {
                let _ = reg.deregister(&id).await;  // 错误被静默丢弃
            });
        }
    }
}
```

Drop 内でブロックはできませんが、`tracing::warn!` で登録解除失敗を記録できます。

---

### 問題 4：[低] `ecat-data-sqlx` の f64 特殊値の処理

- **ファイル**: `ecat-data-sqlx/src/lib.rs:57-61`
- **深刻度**: **低**
- **影響**: データベースの NaN/Infinity 浮動小数点値が Null に変換される

```rust
row.try_get::<f64, _>(col.as_str())
    .ok()
    .and_then(serde_json::Number::from_f64)  // NaN/Inf → None
    .map(serde_json::Value::Number)
    .ok_or(())
```

`serde_json::Number::from_f64()` は `f64::NAN`、`f64::INFINITY`、`f64::NEG_INFINITY` に対して `None` を返すため、これらの値は Null に格下げされます。

---

## 三、crate 別レビューノート

### ecat (コア) — 4 ファイル
| ファイル | ステータス | 備考 |
|------|------|------|
| `lib.rs` | ✅ | start_hooks/stop_hooks の分離は正しい |
| `hook.rs` | ✅ | クロージャ blanket impl が on_start/on_stop をカバー |
| `signal.rs` | ⚠️ | SIGTERM handler の `.expect()` は合理的だが厳格 |

### ecat-transport — 4 ファイル
| ファイル | ステータス | 備考 |
|------|------|------|
| `lib.rs` | ✅ | Server trait の設計は簡潔 |
| `context.rs` | ✅ | 既に `tokio::sync::RwLock` を使用 |
| `request.rs` | ✅ | |
| `response.rs` | ✅ | |

### ecat-transport-http / ecat-transport-grpc — 2 ファイル
| ファイル | ステータス | 備考 |
|------|------|------|
| `ecat-transport-http/src/lib.rs` | ⚠️ | `start()` がブロックして戻らず、`stop()` は空操作（既知の制限） |
| `ecat-transport-grpc/src/lib.rs` | ⚠️ | 同上 |

### ecat-middleware — 5 ファイル
| ファイル | ステータス | 備考 |
|------|------|------|
| `tracing.rs` | ✅ | `fut.instrument(span)` の修正は正しい |
| `recovery.rs` | ⚠️ | `tokio::task::spawn` が span コンテキストを喪失（問題 2） |
| `logging.rs` | ✅ | `elapsed.as_millis() as u64` の理論上の切り捨ては実質影響なし |
| `timeout.rs` | ✅ | |

### ecat-registry — 2 ファイル
| ファイル | ステータス | 備考 |
|------|------|------|
| `lib.rs` | ⚠️ | Registration Drop がエラーを黙って破棄（問題 3） |
| `memory.rs` | ⚠️ | 同期 `std::sync::RwLock` が async コンテキスト内（既知の制限） |

### ecat-config — 3 ファイル
| ファイル | ステータス | 備考 |
|------|------|------|
| `lib.rs` | ✅ | Config trait の設計は合理的 |
| `env.rs` | ✅ | 型解析順序は正しい（bool→i64→f64→String） |
| `file.rs` | ⚠️ | YAML マルチドキュメント非対応、watch 機構なし（既知の制限） |

### ecat-data — 6 ファイル
| ファイル | ステータス | 備考 |
|------|------|------|
| `rdbms.rs` | ✅ | Transaction Drop のコメントで自動ロールバックを説明、ただし実体なし |
| `cache.rs` | ✅ | trait 定義は完全 |
| `graph.rs` | ✅ | |
| `search.rs` | ✅ | |
| `tsdb.rs` | ✅ | DataPoint builder パターンの設計は良好 |

### ecat-data-sqlx — 1 ファイル
| ファイル | ステータス | 備考 |
|------|------|------|
| `lib.rs` | ⚠️ | 値抽出順序は修正済み；transaction 未実装；f64 特殊値（問題 4） |

### ecat-errors — 2 ファイル
| ファイル | ステータス | 備考 |
|------|------|------|
| `lib.rs` | ✅ | gRPC→ErrorCode マッピングは完全、Display フォーマットは明確 |
| `codes.rs` | ✅ | HTTP ステータスコードマッピングが gRPC セマンティクスと一致 |

### ecat-encoding — 3 ファイル
| ファイル | ステータス | 備考 |
|------|------|------|
| `lib.rs` | ✅ | CodecBox enum、codec_for/codec_from_content_type の設計は良好 |
| `json.rs` | ✅ | |
| `proto.rs` | ⚠️ | ProtoCodec はプレースホルダ実装（既知の制限） |

### その他の crate
| Crate | ステータス | 備考 |
|-------|------|------|
| `ecat-logging` | ✅ | `try_init` で重複初期化を防止 |
| `ecat-metadata` | ✅ | HTTP/gRPC 双方向変換は充実 |
| `ecat-metrics` | ⚠️ | `metrics_text()` に unwrap() あり（問題 1） |
| `ecat-protos` | ✅ | prost/tonic コード生成 |
| `ecat-cli` | ⚠️ | 大半のコマンドはメッセージをプリントするのみで、実際にファイルを作成しない（既知の制限） |
| `examples/helloworld` | ✅ | サンプルコードが新 API を正しく使用 |

---

## 四、テストカバレッジ分析

```
cargo test → 60 passed, 0 failed

crate 別分布:
  ecat                  4   (Builder/デフォルト値/ライフサイクル hook)
  ecat-config           9   (env parse ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (HTTP マッピング/gRPC 変換/metadata/Display)
  ecat-logging          1   (init スモーク)
  ecat-metadata         9   (アクセス/From HeaderMap/From MetadataMap/イテレータ)
  ecat-metrics          2   (シングルトン/text が panic しない)
  ecat-registry         5   (登録/ディスカバリ/登録解除/一覧/フィルタ)
  ecat-transport       11   (Context/Request/Response/Server trait)
  その他 8 crate          0   (純 trait/コード生成/統合テスト必要)
```

### テストギャップ

| 優先度 | Crate | 欠落内容 |
|--------|-------|----------|
| 高 | `ecat-middleware` | 4 個の Tower Service にユニットテストなし |
| 高 | `ecat-data-sqlx` | 統合テストなし（SQLite メモリ DB が可能） |
| 中 | `ecat-transport-http` | HTTP server 起動フローのテストなし |
| 中 | `ecat-transport-grpc` | gRPC server 起動フローのテストなし |
| 低 | `ecat-data` | 純 trait 定義、許容可能 |

---

## 五、コード品質指標

| 指標 | 値 | 評価 |
|------|-----|------|
| 総行数 | 2151 | — |
| コンパイル警告 | 0 | ✅ |
| Clippy 警告 | 0 | ✅ |
| テスト通過 | 60/60 | ✅ |
| テストカバレッジ（推定） | ~35% | ⚠️ |
| 非テスト unwrap() | 2 箇所（metrics） | ⚠️ |
| 安全でないコード | 0 | ✅ |
| panic リスク箇所 | 3 箇所（metrics×2 + signal expect） | ⚠️ |

---

## 六、修正提案まとめ

### 推奨修正（本ラウンド — すべて修正済み ✅）

| # | ファイル | 問題 | 優先度 | ステータス |
|---|------|------|--------|------|
| 1 | `ecat-metrics/src/lib.rs:14-15` | `metrics_text()` unwrap → フォールバック処理 | 中 | ✅ 修正済み |
| 2 | `ecat-registry/src/lib.rs:51` | Drop 内で `tracing::warn!` を追加し deregister 失敗を記録 | 低 | ✅ 修正済み |
| 3 | `ecat-data-sqlx/src/lib.rs:57-61` | f64 NaN/Inf 値に特殊処理を追加 | 低 | ✅ 修正済み |
| 4 | `ecat-middleware/src/recovery.rs:40` | `tokio::task::spawn` が span 喪失 → `fut.instrument(span)` | 低 | ✅ 修正済み |
| 5 | `ecat-registry/src/memory.rs` | 同期 RwLock → `tokio::sync::RwLock` | 低 | ✅ 修正済み |

### 既知の制限（ブロッキングなし）

| # | ファイル | 説明 |
|---|------|------|
| K1 | `ecat-transport-http` / `ecat-transport-grpc` | start() がブロック / stop() が空操作（graceful shutdown が必要） |
| K2 | `ecat-data-sqlx` | `transaction()` が未実装エラーを返す |
| K3 | `ecat-middleware` | 4 個の Service にユニットテストなし |
| K4 | `ecat-config/file.rs` | watch 機構なし |
| K5 | `ecat-encoding/proto.rs` | ProtoCodec はプレースホルダ実装 |
| K6 | `ecat-cli` | 大半のコマンドは mock 出力 |

---

## 七、まとめ

第 3 ラウンドは R2 の全修正を踏まえて実施しました。本ラウンドで発見された 5 件の問題はすべて修正済みです。

R2 との比較：
- R2 は高深刻度 2 件 + 中深刻度 1 件のランタイム Bug を発見 → すべて修正済み ✅
- R3 は中深刻度 1 件 + 低深刻度 4 件の堅牢性問題を発見 → すべて修正済み ✅
- テスト数は 60 を維持

### 今後の優先提案

1. `ecat-data-sqlx` に SQLite 統合テストを追加
2. `ecat-middleware` にユニットテストを追加（span/タイムアウト/リカバリ挙動の検証）
3. HTTP/gRPC server の graceful shutdown を実装
