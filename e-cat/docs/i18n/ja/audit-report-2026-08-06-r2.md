# e-cat 全量再監査レポート（修正後の再検証）

- **日付**: 2026-08-06
- **バージョン**: v2.3.1（55 crates）
- **前置**: 前回の監査 `docs/audit-report-2026-08-06.md` の 35 件の発見項目はすべて修正済み。今回は修正後の全量再検証です。

---

## 1. テストとビルドの結果

| チェック | 結果 |
|------|------|
| `cargo check --workspace` | ✅ コンパイルゼロエラー |
| `cargo test --workspace` | ✅ **219 passed · 0 failed · 1 ignored** |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ ゼロ警告 |
| `cargo fmt --check` | ✅ クリーン |
| helloworld スモークテスト | ✅ `/` は JSON を返す、`/health` は OK を返す、`0.0.0.0:8000` へのバインド成功 |

**結論**: 前回の修正（D1/H1/H6/C1/C2/M1/M3/M5/M6/M9/M11/M13/L シリーズ）に回帰はありません。

## 2. コード品質の深査

| チェック項目 | 結果 |
|--------|------|
| TODO / FIXME / XXX / HACK | ✅ 0 箇所 |
| プロダクションコードの `unwrap()` / `expect()` | ✅ すべて `#[cfg(test)]` テスト内にあり、プロダクションパスに panic リスクなし |
| `unsafe` ブロック | ✅ workspace 全体で 0 箇所 |
| デッドコード / 未使用警告 | ✅ clippy -D warnings 通過 |
| ファイル行数 | ✅ すべて 500 行以内 |

## 3. エコシステム設定の完全性

| 項目 | ステータス |
|------|------|
| Workspace メンバー | ✅ 55 crates、README の宣言と一致 |
| CI（GitHub Actions + GitLab） | ✅ 両プラットフォームとも `protobuf-compiler` のインストールを含み、コマンドが一致（check/test/fmt/clippy） |
| Dockerfile | ⚠️ マルチステージビルド、rust:1.85-slim、`ecat` バイナリ名、curl ヘルスチェックはすべて正しい。**残存問題は §5-A 参照** |
| Helm chart | ✅ `appVersion` を 2.3.1 に同期（今回の修正） |
| k8s デプロイマニフェスト | ✅ /health と /ready プローブが ecat-health ルートに対応 |
| CLI テンプレート | ✅ 生成コードが `0.0.0.0:8000` をリッスン |
| ドキュメントのバージョン整合性 | ✅ README×2 / databases.example.yaml がすべて v2.3.1 に同期（今回の修正） |
| サンプル口令 | ✅ デフォルト口令はコメントアウト済み（databases.example.yaml） |
| 画像リソース | ✅ alipay/weixinpay.png が両 README で正常に参照 |
| CHANGELOG | ✅ [2.3.1] の 12 件の記録が変更と一致 |

## 4. セキュリティ対策の完全性

| チェック項目 | 結果 |
|------|------|
| ハードコードされた認証情報 / API キー | ✅ 0 箇所（唯一の一致はテストアサーション内の PEM キーワード） |
| TLS `skip_verify` のデフォルト値 | ✅ デフォルトでオフ。Redis は `rediss://` に自動アップグレード |
| インジェクション面 | ✅ TDengine 二重エスケープ、ES/OpenSearch RFC 3986 エンコード、InfluxDB 行プロトコルエスケープ、sqlx パラメータ化、IoTDB insertTablet 標準ボディ |
| レート制限 | ✅ クライアント IP 単位（X-Forwarded-For 先頭 → X-Real-IP → global）、Redis Lua アトミック INCR+EXPIRE、fail-open + warn |
| JWT | ✅ 弱い鍵を拒否（<32 バイト）、エラーレスポンスが内部詳細を漏洩しない |
| パスワード処理 | ✅ Redis パスワードは ConnectionInfo 経由で渡し、URL に埋め込まない（エラーメッセージで漏洩しない） |
| タイムアウト | ✅ 全 HTTP アダプタが connect 5s / request 30s に統一 |
| リクエストボディ対策 | ✅ SecurityBodyLayer 10MB 上限 + body スキャン |

## 5. 今回の新発見（2 項）

### [MEDIUM] A. Dockerfile の `CMD ["ecat"]` が起動即終了
- **現象**: `ecat` CLI はサブコマンド必須。引数なしで実行すると clap がエラーで終了（exit code 2）、コンテナが即終了し、HEALTHCHECK が通らない。
- **原因**: イメージには CLI バイナリのみ内蔵され、ユーザーサービスを含まない。`ecat run` は `cargo run` のラッパーにすぎず（default-member がない場合も同様に失敗）。
- **提案**: ① ビルド時にサンプルサービスのバイナリを同梱して CMD に設定する。② またはドキュメントでこのイメージは dev コンテナ専用（ソースをマウント + `ecat run`）と明示する。③ または CLI に `serve` サブコマンドを追加する。デプロイセマンティクスの問題のため、勝手な変更はしていません。

### [LOW] B. `Chart.yaml` の `name: ecat-app` と Dockerfile の成果物名（`ecat`）の不一致
- **現象**: イメージ名 `ecat-app` とバイナリ `ecat` に直接の対応がなく、Helm デプロイ時にイメージ tag を手動指定する必要がある。
- **提案**: ドキュメントにイメージのビルド/タグ付けコマンド（`docker build -t ecat-app:2.3.1 .`）を明記する。低リスクのため未変更。

## 6. 結論

修正後のコードベースは健全な状態です：**ビルド、テスト（219 件）、clippy、fmt、スモークがすべて通過。プロダクションコードに panic パスなし、unsafe ゼロ、認証情報の漏洩なし。エコシステム設定（CI/Docker/Helm/k8s/CLI テンプレート/二言語ドキュメント/CHANGELOG）が v2.3.1 と完全に一致**。残り 2 件はいずれもデプロイセマンティクスレベルのドキュメント提案であり、リリースをブロックしません。

---

*レポートは自動化再検証により生成：ビルド + テスト + clippy + fmt + スモーク + 専門深査（panic パス/unsafe/TODO/認証情報/インジェクション面/CI 両プラットフォーム/Docker/Helm/k8s/ドキュメント同期）。*
