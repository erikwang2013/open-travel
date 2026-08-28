# 専門監査レポート（セキュリティと性能）— 2026-08-14

監査範囲：55 crate workspace（v2.3.5）。方法：Cargo.lock の手動確認（cargo-audit 未インストール）、認証/TLS パスのソース監査、並行性とリソースライフサイクルの検査。コードは未提出。

## 依存 CVE の確認

- コア依存のバージョンはすべて比較的新しく、既知の未修正 CVE はなし：rustls 0.23.43、ring 0.17.14、aws-lc-rs 1.17.3、jsonwebtoken 9.3.1、tokio 1.53.1、h2 0.4.15、quinn 0.11.11、sqlx 0.8.6、zerocopy 0.8.55、time 0.3.54、openssl 0.10.81。
- hyper 0.14.32（rust-s3 0.35.1 のみから、hyper-tls 0.5 経由）は 0.14.28 の修正ラインを超えています。
- 注意：CI に cargo-audit が未インストール。ワークフローへの自動化確認の追加を提案します。

## 発見項目（深刻度順）

### S1 [中] HTTP TLS ハンドシェイクの直列化 → スローハンドシェイク DoS
- 位置：`ecat-transport-http/src/lib.rs:134-150`（TlsListener::accept）
- 現象：TLS ハンドシェイクが `accept()` 内で同期的に完了し、axum::serve が accept を直列に呼び出す——ハンドシェイクを完了しない接続が accept ループ全体をブロックします。
- 影響：攻撃者が低速/ゾンビ TCP 接続を大量に確立するだけで、サービスが新しい接続を受け付けなくなります（gRPC 側の tonic は接続ごとにハンドシェイクを spawn するため影響なし）。
- 提案：accept 後に `tokio::spawn` でハンドシェイクを行い、`tokio::time::timeout(10s)` を付け、失敗時は接続を閉じます。

### S2 [中] OAuth2 イントロスペクションキャッシュの無制限成長 → メモリ DoS
- 位置：`ecat-auth/src/oauth2.rs:45,84-92`
- 現象：`HashMap<String,(String,Instant)>` は token をキーとし、TTL は鮮度のみを制御し、容量上限も退去もありません。
- 影響：大量の一意 token リクエストでメモリが無限に増加します（miss のたびに上流の introspection もトリガー）。
- 提案：容量上限（例 10k）+ 定期的なクリーンアップ、または容量と TTL 退去を持つ moka/LRU に変更。

### S3 [低-中] ecat-data-s3 が旧版 rust-s3 0.35.1 を使用（hyper 0.14 + native-tls/openssl）
- 位置：`ecat-data-s3/Cargo.toml` → rust-s3 0.35.1
- 現象：S3 クライアントが独立に hyper-tls/openssl スタックを使用し、ecat-tls::TlsClientConfig（カスタム CA、クライアント証明書、skip_verify）が S3 に無効。TLS 設定面が不整合。
- 影響：エンタープライズ環境の S3 プライベート CA/mTLS を設定不可。2023 年以降メンテナンスが遅い依存。
- 提案：rust-s3 のアップグレードを評価するか、統一 reqwest/rustls クライアントに変更。

### S4 [低] JWT のデフォルト検証が iss/aud を含まない
- 位置：`ecat-auth/src/jwt.rs:125` — `Validation::new(HS256)` は署名+exp のみ。
- 影響：HS256 共有鍵の下では、あるサービスの token が別のサービスに受け入れられます（発行者分離なし）。
- 提案：ドキュメントで本番設定に issuer/audience を明示的に要求。またはデフォルトで iss 検証の入口を追加。

### S5 [低] TlsClientConfig.skip_verify 単独でも is_enabled() が真になる
- 位置：`ecat-tls/src/lib.rs:23-29`
- 現象：`skip_verify: true` のみを設定すると TLS が「有効」とみなされ、証明書を検証せず、静かに検証がオフになります。
- 提案：skip_verify と ca_cert の排他検証を行うか、明示的な二重確認を要求。

## 性能とリソース

### P1 [低] OAuth2 キャッシュヒットパスでリクエストごとに JSON 逆シリアル化
- 位置：`ecat-auth/src/oauth2.rs:87` — キャッシュはシリアル化された文字列を保存し、ヒット後も `serde_json::from_str` を実行。
- 提案：キャッシュに `AuthClaims` 構造体を直接保存し、リクエストごとの parse を省く。

### P2 [低] ecat-bench にウォームアップと定常状態の判定がない
- 位置：`ecat-bench/src/lib.rs:run_bench` — 直接計時し、warmup なし。コールドスタート/コネクションプールの初回割り当てが p99 に混入。
- 提案：ウォームアップラウンドと定常状態収束判定を追加し、結果をより信頼できるものにする。

### P3 [低] Kafka コンシューマの 100ms poll + 100ms sleep の直列
- 位置：`ecat-mq-kafka/src/lib.rs:84-92` — メッセージのエンドツーエンド遅延上限は約 200ms。
- 提案：poll 後に sleep は不要。低スループットのシナリオでは poll 間隔を短縮可能。

## 良好なプラクティスの確認

- プロダクションパスに unwrap/expect panic なし（transport/auth/middleware はテストのみ）。
- API key のクエリパラメータフォールバックは漏洩警告ログ付き。HashMap は SipHash で衝突防止。
- SQL 層は呼び出し側の SQL を透過（フレームワークの性質上）。接続文字列の user:pass は百分号エンコードが正しい。
- Kafka コンシュームチャネルは満杯時に破棄ではなくブロックで背圧。rx drop 後に poll タスクは正常終了。
- config-remote の取得はタイムアウト付き（5s/30s）。ブロッキングクエリは index 欠落エラーでビジーループを防止。

---

## コアドメインの正確性監査（補足、上記のセキュリティ/性能専門監査と相互補完）

監査方法：workspace 全体のプロダクションコードスキャン（unwrap/expect/panic の位置特定、サイレントエラー握りつぶし、非同期停止、並行状態）+ `cargo test --workspace` の全量再検証（初回は全緑。S1 修正中に transport-http が途中でコンパイル警告を出したため、仕上げ後に再実行が必要）。コードは未提出。

### N1 [中] ecat-events のコンシュームタスク終了後も handle が残留 → イベントのサイレント消失
- 位置：`ecat-events/src/lib.rs:97-101`（コンシュームループ 89-95 行 `None => break`）
- 現象：mq stream が None を返す（kafka broadcast channel のクローズ等）かタスクが panic するとコンシュームループが終了しますが、`consumers` map に JoinHandle が残留。その後同じイベントタイプで再 `subscribe()` しても 68 行の `contains_key` が常に真のためコンシュームタスクが再起動されず → そのタイプのイベントが永久にサイレント消失します。
- 影響：リモートイベントストリームの中断後に自己修復できず、復旧にはプロセス再起動が必要。
- 提案：タスク終了パスで map から handle を削除（spawn ウォッチャーまたは `handle.is_finished()` の遅延クリーンアップ）。

### N2 [中] ecat-mq-kafka subscribe の group_id セマンティクス誤り
- 位置：`ecat-mq-kafka/src/lib.rs:71-84`
- a. `group_id` がデフォルト None の場合、rdkafka の `consumer.subscribe()` は group.id を要求（librdkafka が INVALID_ARG を報告）、デフォルト設定での購読は高い確率で直接失敗します（実機検証が必要）。
- b. group_id を設定した場合（ecat-events はイベントタイプごとに 1 回 subscribe、同一 group）、Kafka は同一 group の複数コンシューマ間でパーティションごとに topic を分配 → あるイベントタイプが他のタイプのコンシュームタスクに落ちてサイレントに破棄される可能性（auto.offset.reset=latest かつコミットしない）。
- 影響：イベントバスが kafka バックエンドでイベントを消失。
- 提案：group_id がない場合はランダムな一意 group.id を生成。またはコンシューム側で assign() により明示的にパーティションを割り当て。ドキュメントで複数購読は独立 group を必須と明示。

### N3 [低] GrpcServer/WsServer の空ホストが未正規化（D1 修正が不完全）
- 位置：`ecat-transport-grpc/src/lib.rs:52`、`ecat-transport-ws/src/lib.rs:58`
- 現象：`GrpcServer::new(":8000")` の `addr.parse::<SocketAddr>()` は AddrParseError を返します（実測検証済み）。WsServer の `TcpListener::bind(":8000")` は IPv6 ワイルドカードに解決され、IPv6 のない環境では起動失敗。HttpServer は 0.0.0.0 正規化済みで、3 つの server API の挙動が不整合。
- 提案：new 内で空ホストを統一正規化。

### N4 [低] TracingLayer が trace_id を注入せず、CHANGELOG 2.3.3 の宣言と不一致
- 位置：`ecat-tracing/src/lib.rs:72-84`（span は service フィールドのみを含み、コードコメントも汎用 Req ではヘッダーを取得できないと自認）；`inject_trace_id()` は毎回新しい UUID を生成し、上流の extract で得た trace_id を引き継ぎません。
- 影響：ドキュメント通りに設定した分散トレーシングがサービス間で関連付けられない。
- 提案：span フィールドを遅延バインドするか、http::Request<B> に特化。inject が上流の id を引き継ぐようにする。

### N5 [低] ecat-scheduler の job panic がサイレントに停止
- 位置：`ecat-scheduler/src/lib.rs:53-57,83`（`run()` 内の `let _ = handle.await`）
- 現象：定期タスクが panic するとタスクが死亡し、再起動もログもなし。`run()` は JoinHandle のエラーを破棄。
- 提案：panic を捕捉してログを出し、オプションで再起動ポリシーを追加。

### N6 [低] プロダクションコードに残留 unwrap（毒化/panic パス）
- `ecat-events/src/lib.rs:68,98` の std `Mutex::lock().unwrap()`（毒化すると panic）。`ecat-versioning/src/lib.rs:86` の Response builder unwrap（失敗しないが panic パスに該当）。`ecat-mq/src/lib.rs:110` の expect は is_none ガード済み（安全）。
- 提案：events の 2 箇所を `unwrap_or_else(|e| e.into_inner())` に変更。

### N7 [情報] WsServer::stop() がアップグレード済み WebSocket 接続を待たない
- 位置：`ecat-transport-ws/src/lib.rs:63-87`
- axum の on_upgrade 接続は独立タスクで実行され、グレースフルシャットダウンの対象外。長接続ハンドラは stop() 後も残留し、プロセス終了がクリーンでない（App::stop のセマンティクスが不完全）。

### N8 [情報] テストゼロの crate：ecat-data / ecat-lock / ecat-protos
- いずれも trait/定義型 crate。デフォルトメソッドは fail-loud（エラーを返しサイレントにはしない）であることを検証済みだが、trait 契約（Transaction drop ロールバックセマンティクス、ロック token 検証）に単体テストがありません。
- 提案：RdbmsError/Transaction と DistributedLock のセマンティクスに最小限の単体テストを追加。

### N9 [情報] graphql のパラメータとネストフィールドが依然として破棄される
- `ecat-graphql/src/lib.rs` の execute は `variables` のみを resolver に渡し、`{ hello(name: "x") }` のフィールドパラメータやネスト selection を渡しません。README はこの制限を明記していません（旧レポート L8 がドキュメント化を要求、2.3.3 の書き直し後も未対応）。

### N10 [情報] circuit-breaker がトランスポート層エラーのみを集計
- `ecat-circuit-breaker/src/lib.rs:203-209` は inner の Err のみを失敗として記録し、HTTP 5xx は成功とみなす → サービス不可（5xx ストーム）に対するサーキットブレーカーが機能しない。ドキュメントに説明なし。

**検証ステータス**：初回の `cargo test --workspace` は全緑（doc-tests 含む、末尾出力に失敗なし）。S1 修正 agent の編集期間中、transport-http にコンパイルエラーと 2 箇所の警告（unused import `ensure_crypto_provider`、`shutdown_tx` 未読）が発生——中間状態であり、S1 の仕上げ後にテストと `clippy --all-targets -D warnings` を全量再実行する必要があります。

---

## 第 3 ラウンド：動的検証 + CVE 再確認 + panic 面（専門監査、2026-08-14）

### CVE 再確認（新発見、深刻度順）

1. **[中] rustls-webpki 0.102.8 が依存ツリーに残留**（RUSTSEC-2026-0049/0098/0099/0104：CRL distributionPoint バイパス、URI/wildcard name-constraints、修正版 0.103.10）。主チェーンは 0.103.13（rustls 0.23.43 経由、安全）。0.102.8 は async-nats 0.38.0 / rumqttc 0.25.1 経由で導入され、NATS/MQTT の TLS クライアントチェーンをカバー。上流は rustls 0.23 に移行しておらず、修正バージョンなし——管理されたリスクであり、コメントで追跡を提案。
2. **[中-低] rdkafka 0.36.2 内蔵の librdkafka が cJSON 1.7.14 を同梱**（CVE-2023-53154 および cJSON シリーズ。CVE-2025-57052 は CVSS 9.8 だが、影響を受けるファイル cJSON_utils.c は librdkafka が使用しておらず、適用性に疑義）。上流の修正は librdkafka 2.10+（2026-03 PR #5346）。ecat-mq-kafka は静的リンクのため、librdkafka-sys のパッケージ版を照合してアップグレードを追跡する必要があります。
3. **[低] rustls-pemfile 2.2.0 が未メンテナンス**（RUSTSEC-2025-0134）— ecat-transport-http の起動期にローカルファイルを解析するもので、攻撃者入力ではない。
4. **[低] rsa 0.9.10**（RUSTSEC-2023-0071 Marvin タイミングサイドチャネル）— sqlx-mysql の TLS 経由で導入。MySQL + RSA 鍵交換のシナリオのみ関連。
5. async-nats 0.38.0 は RUSTSEC-2023-0027（CN 検証バイパス）の修正ラインを超えており問題なし。

### 動的検証（examples/helloworld、debug ビルド、一時ポート 18080、クリーンアップ済み）

- /health 200、/（JSON シリアル化）200（27B）、404 正常。Logging ミドルウェアがリクエストを正常に記録。
- **/metrics はマウントされているが 200 + 空ボディ（0 バイト）を返す**：指標が登録されていないと出力がなく、モニタリング側で「正常/指標なし」を区別できない。空 registry にコメント行または 503 を出力することを提案。
- 不正なリクエスト（ヘッダーに 0x01/0x02 を含む）→ 400 Bad Request、サービスは生存し、その後の /health も 200、panic なし。
- TLS/mTLS パスとサーキットブレーカー/レート制限ミドルウェア：ecat-transport-http/grpc、ecat-middleware のテストでカバー（mTLS 競合修正後に全緑、匿名/誤ったクライアント証明書の拒否ケース通過）。

### bench ベースライン

- ecat-bench に [[bench]]/bin ターゲットがなく、cargo bench の入口なし。run_bench_with_warmup は既にウォームアップ付き（P2 修正済み）、harness テストは全緑。
- 実測は debug ビルドのスモーク：/ は約 1.3ms、/health は約 1.8ms（curl プロセスオーバーヘッドを含み、ベースラインの意味なし）。release ビルド + wrk/hey での負荷テストによる実ベースライン取得を提案。

### panic 面の再確認（workspace 全体、テストモジュール除外）

- 計 31 箇所の unwrap/expect/panic、すべて低リスク：Response::builder().body().unwrap()（jwt/apikey/oauth2 の失敗しない分岐）、ロック毒化フォールバック（etcd/testing）、clickhouse の serde_json::to_string().unwrap()（極端な NaN/inf 入力で理論上 panic）。
- **1 箇所要注意**：`ecat-transport-http/src/tls_listener.rs:234` — バックグラウンド accept ループが異常終了する際に `accept()` 内で panic! し、サービススレッドが死亡（発火条件は厳しい：リスナー致命的エラーのみ）。エラーを返してログを出す形に格下げすることを提案。
