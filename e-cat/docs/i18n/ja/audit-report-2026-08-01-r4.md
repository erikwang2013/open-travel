# e-cat コードレビューレポート — 2026-08-01（第 4 ラウンド・全修正済み）

**プロジェクトバージョン:** 2.1.0  
**最終状態:** 0 warnings, ~116 tests, clippy clean, fmt clean

**第 5 ラウンドのクリーンアップ:** 未使用の 12 依存を削除（ecat-health/reqwest, ecat-circuit-breaker/tokio, ecat-bench/tracing, ecat-mq/serde+serde_json, ecat-events/async-trait, ecat-config-remote/tracing, ecat-testing/transport-http+axum, ecat-client/serde+serde_json）
**レビュー範囲:** 全 18 個の crate

## 最終状態

| ツール | ステータス |
|------|------|
| `cargo build` | 通過 (0 warnings) |
| `cargo test` | 77 passed, 0 failed, 1 ignored |
| `cargo clippy` | 通過 (0 warnings) |
| `cargo fmt` | 通過 |

---

## 修正リスト（全部）

### 中リスク

1. **[修正済み]** `Mutex::lock().unwrap()` → `ecat-transport-http/lib.rs`, `ecat-transport-grpc/lib.rs`
2. **[修正済み]** CLI `fs::write().unwrap()` → `ecat-cli/src/main.rs`

### 低リスク

3. **[修正済み]** ProtoCodec doc-test → `ecat-encoding/src/proto.rs`
4. **[修正済み]** 単体テストゼロの crate → transport-http/grpc に各 3 テストを追加
5. **[修正済み]** `Transaction::commit()` が空操作 → 新規 `TransactionInner` trait
6. **[修正済み]** `SecurityScanner::new()` のコメント修正
7. **[修正済み]** 未使用の `opentelemetry` 依存 → `ecat-logging` および workspace ルート Cargo.toml
8. **[修正済み]** Doc-test のフォーマット

### 最適化

9. **[修正済み]** `scan_parts` の事前確保 → `Vec::with_capacity`
10. **[修正済み]** `serde_yaml` 0.9 の非推奨 → `yaml_serde` 0.10 に移行
11. **[修正済み]** `Transaction::commit()` が空操作でなくなった → `SqlxTransactionWrapper` による実コミット/ロールバック

### 修正不要（設計判断）

- **`ecat` crate の追加依存** — 意図的な「meta crate」パターンで、下流に便利な推移依存を提供
- **ProtoCodec の Codec trait がエラーを返す** — serde と prost::Message の根本的な型の違いによるもの。`encode_message()`/`decode_message()` の分離 API と明確なドキュメントで対応
- **`ecat-data` に具体実装なし** — trait インターフェース設計で、実装は `ecat-data-sqlx` に配置

---

## 変更ファイル一覧

| ファイル | 変更 |
|------|------|
| `ecat-transport-http/src/lib.rs` | Mutex 毒化対策 + 新規 3 テスト |
| `ecat-transport-grpc/src/lib.rs` | Mutex 毒化対策 + 新規 3 テスト |
| `ecat-cli/src/main.rs` | エラー処理の統一 |
| `ecat-security/src/lib.rs` | コメント修正 + 事前確保の最適化 |
| `ecat-logging/Cargo.toml` | 未使用の opentelemetry を削除 |
| `ecat-encoding/src/proto.rs` | doc-test を改善 |
| `ecat-data/src/lib.rs` | TransactionInner をエクスポート |
| `ecat-data/src/rdbms.rs` | TransactionInner trait を追加 |
| `ecat-data-sqlx/src/lib.rs` | SqlxTransactionWrapper で TransactionInner を実装 |
| `ecat-config/Cargo.toml` | serde_yaml → yaml_serde |
| `ecat-config/src/file.rs` | serde_yaml → yaml_serde |
| `Cargo.toml` | orphaned な opentelemetry workspace 依存を削除 |
| `README.md` | バージョン番号更新、可観測性の説明修正、エコシステム計画リンク追加 |
| `docs/ecosystem-plan.md` | エコシステム計画ドキュメントを追加（三期 15 個の crate） |
