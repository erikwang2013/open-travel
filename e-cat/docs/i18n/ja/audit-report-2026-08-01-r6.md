# e-cat 深度レビューレポート — 2026-08-01 R6

## 全体評価

| 次元 | ステータス | 説明 |
|------|------|------|
| コンパイル | 通過 | 50 crates, ゼロエラー |
| テスト | 通過 | すべて通過, ゼロ失敗 |
| Clippy | 通過 | ゼロ警告 (`-D warnings`) |
| unsafe | ゼロ | コードベースに unsafe ブロックなし |
| ファイル規模 | 良好 | `ecat-auth` (540行) のみ 500行 の推奨値を超過 |

## 発見項目 (15 項)

### セキュリティ関連

#### 1. [深刻] XOR「暗号化」は本当の暗号化ではない
**ファイル:** `ecat-config/src/encrypted.rs:45-56`
**問題:** `decrypt()` は XOR + 反復鍵を使用しており、これは暗号化ではなく難読化であり、簡単に解読できます。鍵は各バイト位置で再利用されるため、暗号文は頻度分析攻撃に非常に脆弱です。
**提案:** AES-256-GCM (`aes-gcm` crate) に置き換えるか、「暗号化」ではなく「難読化」と明確に注記します。

#### 2. [深刻] `execute_with`/`query_with` のデフォルト実装がパラメータを静かに破棄
**ファイル:** `ecat-data/src/rdbms.rs:86-103`
**問題:** trait のデフォルト実装はパラメータを受け取るが無視します (`let _ = params;`)。そのまま元の `execute(sql)` を呼び出します。`ecat-data-sqlx` 以外の全バックエンド（ClickHouse、QuestDB）がこの挙動を引き継ぎます。ユーザーがパラメータ化メソッドでバックエンドを差し替えると、パラメータは静かに破棄され、SQL インジェクションの脆弱性になります。
**提案:** デフォルト実装は「非対応」エラーを返すか、各バックエンドが正しくパラメータバインドを実装します。

#### 3. [高危] パスワードが URL に平文で埋め込まれる
**ファイル:** `ecat-data-sqlx/src/lib.rs:40`, `ecat-data-redis/src/lib.rs:43`
**問題:** `connect_with_auth()` は `replacen("://", "://user:pass@")` で認証情報を直接 URL に埋め込みます。これらの URL はログ、エラーメッセージ、デバッグ出力に記録される可能性があります。
**提案:** 各バックエンドのネイティブ認証機構を使用するか、少なくとも連結前にユーザー名/パスワードを URL エンコードします。

#### 4. [中危] TLS 設定の失敗で panic
**ファイル:** 8 個の data-* crate（ClickHouse、QuestDB、Elasticsearch、OpenSearch、ArangoDB、Neo4j、NebulaGraph、InfluxDB、IoTDB）
**パターン:** `.expect("TLS client build failed")` — すべての `from_config()` コンストラクタは TLS 設定エラー時に panic します。
**提案:** `from_config()` を `Result` を返すように変更するか、TLS クライアント構築を遅延/フォールトレラントにします。

### 機能の正確性

#### 5. [高危] `ecat-versioning` の Header ルーティングが機能しない
**ファイル:** `ecat-versioning/src/lib.rs:56-64`
**問題:** `build_header_router()` は全バージョンを同じ `/api` パス配下にネストしますが、バージョン header でフィルタリングしません。axum は全バージョンのルートを同一パスに登録するため、ルート衝突と予測不能な挙動が発生します。`extract_version()` 関数は存在しますが、ルーティングで一度も使われていません。
**提案:** axum の middleware/layer で Accept header を検査して正しいバージョンルートに振り分け、全バージョンを同一パスに平坦化するのをやめます。

#### 6. [中危] Redis TTL の切り捨て：サブ秒の有効期限が永久有効になる
**ファイル:** `ecat-data-redis/src/lib.rs:76-77`
**問題:** `Duration::as_secs()` はゼロ方向に切り捨てます。500ms の TTL を設定すると `secs == 0` となり、静かに「無期限」として扱われ、`SETEX` ではなく `SET` の分岐に入ります。
**提案:** サブ秒 TTL は最低 1 秒にするか、`SETEX` の代わりに `SET ... PX`（ミリ秒）を使用します。

#### 7. [中危] `StaticResolver::add_service` がロック競合時に panic
**ファイル:** `ecat-client/src/lib.rs:27-29`
**問題:** `try_write()` + expect を使用しており、他の書き込みロック保持者がいると panic します。builder パターンのため発生は難しいですが、並行コードでは時限爆弾です。
**提案:** `blocking_write()`（同期コンテキストの場合）を使うか、`&mut self` を受け取る形に変えてロック自体を不要にします。

### コード品質

#### 8. [中危] 非同期コンテキストでの `std::sync::Mutex` 使用
**ファイル:** `ecat-data-memcached/src/lib.rs:7,24`
**問題:** async trait 実装で `std::sync::Mutex` を使用しています。ロック保持時間は極めて短く（HashMap 操作のみ）ですが、高競合下では理論上非同期ランタイムをブロックする可能性があります。
**提案:** このメモリキャッシュの特定の利用シーンでは、クリティカルセクションが極短く `.await` ポイントもないため、`std::sync::Mutex` の使用は実際には許容できます。ただし、将来ロック内で I/O を実行する場合は `tokio::sync::Mutex` に変更すべきです。

#### 9. [低] 手書きの base64 実装
**ファイル:** `ecat-registry-etcd/src/lib.rs:148-193`
**問題:** ~45 行の手書き base64 エンコーダ/デコーダで、境界ケースの bug がある可能性があります。Rust エコシステムには `base64` crate など、十分にレビューされた代替品があります。
**提案:** `base64` crate に置き換え、メンテナンス負担と潜在的な bug を減らします。

#### 10. [低] `RandomBalancer` がランダムではない
**ファイル:** `ecat-client/src/lib.rs:91-105`
**問題:** 乱数源として `Instant::now()` のハッシュを使用しています。同一インスタンス内で同時に発行された呼び出しは同じ「ランダム」な選択を得ます。`checked_add(0)` は冗長な操作です。
**提案:** `rand` crate を使うか、少なくとも `std::collections::hash_map::RandomState` を使用します。

#### 11. [低] `ecat-data-sqlx` の不要な `Arc<Vec<String>>`
**ファイル:** `ecat-data-sqlx/src/lib.rs:79-87, 197-203`
**問題:** 列名は `Arc<Vec<String>>` にラップされていますが、各 `Row` コンストラクタが列名リスト全体をクローンします (`(*cols).clone()`)。`Arc` はイテレーション中に一度だけ使われ、`Rc` か直接 `clone()` で十分です。
**提案:** `query()` と `query_with()` で `Arc<Vec<String>>` を普通の `Vec<String>` に置き換えます。行ごとの個別クローンコストは、Arc の deref + クローンと同等です。

### 設計/アーキテクチャ

#### 12. [情報] QuestDB が GET + クエリパラメータを使用
**ファイル:** `ecat-data-questdb/src/lib.rs:76, 91`
**問題:** SQL が GET クエリパラメータで送信され、URL 長制限（通常 ~2000-8000 文字）を受けます。大きなクエリは切り詰められます。
**提案:** POST + body 方式に変更するか、単純なクエリは GET のまま、複雑なクエリは POST にします。

#### 13. [情報] `#[allow(dead_code)]` が散在
**ファイル:** `ecat-registry-consul/src/lib.rs:225`, `ecat-data-memcached/src/lib.rs:25-28`, `ecat-auth/src/lib.rs:52`
**問題:** username/password フィールドがメモリに保持されていますが dead_code とマークされています（in-memory memcached では不要、auth の RSA 変種は未実装）。
**提案:** 欠落した機能パスを実装するか、フィールドを削除するか、保持理由を説明するドキュメントを追加します。

#### 14. [情報] 一部の HTTP クライアントに Content-Type header がない
**ファイル:** `ecat-data-influxdb/src/lib.rs:96-103`, `ecat-data-clickhouse/src/lib.rs:87-89`
**問題:** 一部の POST リクエストで `Content-Type` header が設定されておらず、サーバー側の自動検出に依存しています。
**提案:** 互換性を確保するため、常に明示的な Content-Type を設定します。

#### 15. [情報] `ecat-auth` が 500 行を超過
**ファイル:** `ecat-auth/src/lib.rs` (540 行)
**問題:** CLAUDE.md はファイルを 500 行未満に保つことを要求しています。auth crate はこの制限を超える唯一のファイルです。
**提案:** JWT 検証ロジックを `ecat-auth/src/jwt.rs` に分割するか、機能ごとに分割します。

## 最適化機会（Bug ではない）

| # | 場所 | 提案 |
|---|------|------|
| O1 | 全 data-* crate | 全 `from_config()` で繰り返される TLS クライアント構築パターンを共有マクロまたは関数に抽出可能 |
| O2 | `ecat-data-sqlx` | `query()` と `query_with()` の行型変換ロジック（117行の重複）をヘルパー関数に抽出可能 |
| O3 | `ecat-client` | `HttpClient::get()` と `post()` は同じ「resolve → pick → build URL」パイプラインを共有 — 抽出可能 |
| O4 | `ecat-data` | 5 個すべての traits（Rdbms/Cache/Graph/Search/Tsdb）のカスタムエラー型を単一の `DataError` 列挙型に統一可能 |
| O5 | `ecat-data-redis` | 各メソッドの `self.conn.clone()` は不要 — `MultiplexedConnection` は共有をサポートするよう `Clone` 設計 |

## 指標まとめ

| 指標 | 数値 |
|------|------|
| 総 crate 数 | 50 |
| Rust ソースファイル総行数 | 7,968 |
| 非テストコードの `expect()` | 12 |
| 非テストコードの `unwrap()` | 0 |
| `unsafe` ブロック | 0 |
| 非テストコードの `panic!` | 0 |
| `#[allow(dead_code)]` | 4 |
| TODO/FIXME/HACK | 0 |
| 非同期コードの std Mutex | 1 (memcached) |

## 結論

コードベースは良好な状態です——コンパイル、テスト、clippy がすべて通過し、unsafe コードも panic マクロもありません。最も重要な 2 つの問題は **XOR「暗号化」**（セキュリティが偽物）と **パラメータ化クエリのデフォルト実装がパラメータを静かに破棄**（セキュリティ脆弱性）です。Header ルーティング機能も完全に使用不可です。その他の問題は比較的小さく、保守性レベルの最適化です。

**推奨修正優先順位:**
1. `execute_with`/`query_with` デフォルト実装 → パラメータを静かに破棄せずエラーを返す
2. XOR 暗号化 → 本物の AEAD 暗号化、または「難読化」に名称変更
3. Header バージョンルーティング → 実際の header ルーティングを実装
4. `from_config()` → expect-panic ではなく Result を返す
5. Redis TTL 切り捨て → サブ秒 TTL は最低 1 秒を使用

## 修正ステータス (R6 → R6.1)

| # | 問題 | ステータス | 変更 |
|---|------|------|------|
| 1 | XOR "暗号化" | 修正済み | `EncryptedSource` → `ObfuscatedSource`、`decrypt` → `deobfuscate`、プレフィックス `enc:` → `obfs:`、難読化であり暗号化ではないというドキュメントを追加 |
| 2 | `execute_with`/`query_with` がパラメータを静かに破棄 | 修正済み | デフォルト実装を `"parameterized ... not supported by this backend"` エラーを返すように変更 |
| 3 | パスワードが URL に平文埋め込み | 修正済み | `connect_with_auth` メソッドで `percent_encode()` により認証情報をエンコード |
| 4 | TLS `expect()` panic | 修正済み | 9 個の crate の `from_config()` が `Result` を返すように変更、`RdbmsError` に `Config` 変種を追加 |
| 5 | Header ルーティングが機能しない | 修正済み | `from_fn_with_state` ミドルウェアでバージョン検証を実装、新規テスト `header_versioned_router_builds` を追加 |
| 6 | Redis TTL 切り捨て | 修正済み | `set_ex` → `pset_ex`、ミリ秒精度でサブ秒 TTL が無期限に切り捨てられるのを防止 |
| 7 | `StaticResolver` のロック競合 panic | 修正済み | `try_write()` → `blocking_write()` |
| 8 | `RandomBalancer` がランダムでない | 修正済み | `Instant::now()` ハッシュを `RandomState::new().build_hasher()` に置き換え |
| 9 | 非同期コンテキストの `std::sync::Mutex` | 修正済み | `tokio::sync::Mutex` に置き換え |
| 10 | 手書き base64 | 修正済み | `base64` crate 0.22 に置き換え |
| 11 | `Arc<Vec<String>>` のオーバーヘッド | 修正済み | 普通の `Vec<String>` に置き換え、不要な Arc ラップを削除 |
| 12 | QuestDB の GET 方式 SQL 送信 | 修正済み | POST + body に変更、Content-Type header を追加 |
| 13 | `#[allow(dead_code)]` | 修正済み | memcached フィールドに `_` プレフィックス；consul フィールドに `_` プレフィックスを付け allow を削除；auth で `Rsa` → `RsaReserved` |
| 14 | Content-Type の欠如 | 修正済み | InfluxDB、ClickHouse、IoTDB のリクエストに明示的な Content-Type を追加 |
| 15 | `ecat-auth` が 500 行超過 | 修正済み | `claims.rs`(31) + `jwt.rs`(139) + `apikey.rs`(96) + `oauth2.rs`(173) + `helpers.rs`(28) + `lib.rs`(98) に分割 |

### 影響を受けた Crate

| Crate | 変更種別 |
|-------|----------|
| `ecat-data` | trait デフォルト実装、`RdbmsError::Config` 変種 |
| `ecat-config` | `EncryptedSource` → `ObfuscatedSource` |
| `ecat-versioning` | Header ルーティングミドルウェアの実装 |
| `ecat-data-redis` | TTL ミリ秒精度、認証情報の URL エンコード |
| `ecat-data-sqlx` | 認証情報の URL エンコード、Arc オーバーヘッド削除 |
| `ecat-data-clickhouse` | `from_config` → `Result`、Content-Type header |
| `ecat-data-questdb` | `from_config` → `Result`、GET → POST |
| `ecat-data-elasticsearch` | `from_config` → `Result` |
| `ecat-data-opensearch` | `from_config` → `Result` |
| `ecat-data-arangodb` | `from_config` → `Result` |
| `ecat-data-neo4j` | `from_config` → `Result` |
| `ecat-data-nebulagraph` | `from_config` → `Result` |
| `ecat-data-influxdb` | `from_config` → `Result`、Content-Type header |
| `ecat-data-iotdb` | `from_config` → `Result`、Content-Type header |
| `ecat-data-memcached` | `std::sync::Mutex` → `tokio::sync::Mutex`、dead_code クリーンアップ |
| `ecat-client` | `StaticResolver`、`RandomBalancer` の修正 |
| `ecat-registry-etcd` | base64 を crate に置き換え |
| `ecat-registry-consul` | dead_code クリーンアップ |
| `ecat-auth` | 6 モジュールに分割、dead_code クリーンアップ |

### 最終検証 (R6.2)

| 次元 | ステータス |
|------|------|
| Build | 通過、ゼロエラーゼロ警告 |
| Test | すべて通過、ゼロ失敗 |
| Clippy (`-D warnings`) | 通過、ゼロ警告 |
| ファイル規模 | すべて ≤ 300 行 |
