# テストレポート — 2026-08-26

全面単体テスト補完（51 crate 全カバー）、4 組のシニア Rust テストエンジニアが並行実施。

## 総覧

| グループ | crates | 既存 | 追加 | 現有 | ゲート |
|---|---|---|---|---|---|
| core/フレームワーク | 12 | 102 | +40 | 142 | ✅ test 全緑 + clippy 0 警告 |
| data | 14 | 87 | +66 | 153 | ✅ 同上 |
| mq/transport | 12 | 82 | +54 | 136 | ✅ 同上 |
| app アプリケーション層 | 13 | ~178 | +46 | ~224 | ✅ 同上 |
| **合計** | **51** | **~449** | **+206** | **~655** | ✅ |

注：アプリケーション層の既存数は ecat-auth 24 / ecat-graphql 35 / ecat-bench 11 / ecat-scheduler 6 / ecat-events 7 / ecat-cli 12 / ecat-middleware 34 / ecat-circuit-breaker 10 / ecat-health 4 / ecat-client 7 / ecat-security 12 / ecat-versioning 4 / ecat 4。各 crate 独立の `cargo test -p` + `cargo clippy -p --all-targets -- -D warnings` はすべて通過、CARGO_TARGET_DIR 分離で並行実行。

## crate 別明細

### core/フレームワークグループ（test-core、+40）

| crate | 既存→新規 | カバー要点 |
|---|---|---|
| ecat-protos | 4→8 | ErrorCode 全列挙と proto の対照。切り詰め buffer の decode。空 buffer のデフォルトメッセージ。metadata roundtrip |
| ecat-errors | 4→9 | http_status 全マッピング（409/429/500）。from_status。未マッピング→Internal。cause source() |
| ecat-metadata | 9→12 | HTTP header からの trace_id 抽出。key の小文字化。空 header map |
| ecat-encoding | 18→22 | NaN→null（serde_json デフォルト、ドキュメント化済み）。空バイト decode。CodecBox の不正 JSON。proto roundtrip |
| ecat-lock | 7→9 | ロック未保持の release でエラー。空 key |
| ecat-logging | 1→1 | 互換 shim が panic しない |
| ecat-tracing | 9→12 | 非 UTF-8 trace ヘッダーのスキップ。canonical ヘッダー。レスポンス透過 |
| ecat-tls | 7→12 | basic_auth 単一/二重フィールド。ca ファイル欠落。is_enabled。デフォルトクライアント |
| ecat-config | 14→26 | env プレフィックスフィルタ + 型解析の境界（hex/空文字列/-0/1e3）。複数 source のマージ上書き。obfs エラーパス。ファイル欠落/不正 YAML |
| ecat-config-remote | 6→9 | ConsulKvEntry の境界。X-Consul-Index 欠落でエラー。ネスト key |
| ecat-openapi | 4→11 | components/schema_ref。重複上書き。デフォルト 200。tags |
| ecat-metrics | 8→11 | 登録済み指標のテキスト。404/405 |

### data グループ（test-data、+66）

| crate | 既存→新規 | カバー要点 |
|---|---|---|
| ecat-data | 12→14 | 検索構文の解析 |
| ecat-data-sqlx | 7→14 | インメモリ SQLite のエンドツーエンド。パラメータバインド全型。Blob→base64。config |
| ecat-data-redis | 6→12 | redis:///rediss:// URL 構築。auth。config エラーパス |
| ecat-data-opensearch | 4→10 | mock HTTP：percent-encode、Basic auth、エラー透過 |
| ecat-data-elasticsearch | 6→11 | 同上 |
| ecat-data-influxdb | 5→10 | line protocol のエスケープ。Token ヘッダー。エラー透過 |
| ecat-data-clickhouse | 12→22 | テーブル作成 SQL。JSONEachRow。書き込み行数。グループ化 |
| ecat-data-memcached | 4→8 | TTL 秒→ミリ秒。flag パッキング |
| ecat-data-nebulagraph | 6→7 | config 解析 |
| ecat-data-arangodb | 5→7 | config/URL |
| ecat-data-iotdb | 5→10 | mock HTTP：session パスパラメータ |
| ecat-data-questdb | 4→9 | line protocol。トランザクション非対応 |
| ecat-data-tdengine | 6→11 | INSERT 生成。100 件バッチの分割 |
| ecat-data-mongodb | 5→8 | bson 往復。URI |

### mq/transport/registry グループ（test-mq、+54）

| crate | 既存→新規 | カバー要点 |
|---|---|---|
| ecat-mq | 5→9 | 満杯バッファの遅延エラーフレーム。全 drop でストリーム終了。複数サブスクライバ。サブスクライバなしの publish |
| ecat-mq-kafka | 12→14 | config のデフォルト。SASL フィールドの独立有効性 |
| ecat-mq-rabbitmq | 2→5 | exchange のデフォルト。url エラーパス |
| ecat-mq-mqtt | 5→9 | cert/key のペア検証。ファイル欠落。ポートデフォルト 1883/8883。不正ポートのフォールバック |
| ecat-mq-nats | 6→9 | 平文デフォルト。ca/cert 欠落のエラーパス |
| ecat-transport | 4→7 | TlsConfig デフォルト/with_client_auth。normalize_addr の境界 |
| ecat-transport-http | 17→20 | 統合テスト：stop 空操作、ポート占有失敗、実際の送受信 |
| ecat-transport-grpc | 7→13 | TLS ファイル欠落。プレーンテキストのライフサイクル。mTLS 拒否 |
| ecat-transport-ws | 4→8 | handler なしで失敗。ポート占有。RFC 6455 masked フレームのエコー |
| ecat-registry | 5→8 | マルチインスタンス discover。drop で自動登録解除。builder デフォルト |
| ecat-registry-consul | 10→24 | percent-encode。登録バリアント。エラーレスポンス。X-Consul-Token。agent/services 解析。node フォールバック |
| ecat-registry-etcd | 5→10 | discover の不正値スキップ。kv リクエストボディ。lease grant。keepalive |

### app アプリケーション層グループ（test-app、+46）

| crate | 既存→新規 | カバー要点 |
|---|---|---|
| ecat-auth | 20→46 | oauth2 キャッシュのホワイトリスト/SHA-256 key/FIFO 退去。apikey の三状態。jwt iss/aud 強制。期限切れ/誤署名 |
| ecat-health | 4→8 | readiness 集約（全 ok/いずれか fail/空レジストリ）。liveness |
| ecat-versioning | 4→7 | path ポリシールーティング。extract_version の境界 |
| ecat-security | 12→20 | header 層のエンドツーエンド。攻撃遮断の JSON 形状 |
| ecat-middleware | 34→37 | MemoryStore のウィンドウ期限切れ。内層 panic→Err |
| ecat-circuit-breaker | 10→12 | half-open プローブ枯渇。classify の格下げ |
| ecat-client | 7→10 | grpc の不正エンドポイントでネットワーク接続なしにエラー |
| ecat-graphql | 35→35 | 既存カバレッジが十分でギャップなし |
| ecat-scheduler / ecat-bench / ecat-events / ecat-cli / ecat | 既存カバレッジが十分 | ギャップなし |

## 発見された欠陥

| レベル | 位置 | 説明 | ステータス |
|---|---|---|---|
| P1 | ecat-events/Cargo.toml | dev-dependencies に tokio macros/rt/time features が欠落し、この crate 単体でテストターゲットをコンパイルすると必ず失敗（workspace 全量ビルドでは feature の和集合で隠れる） | ✅ 修正済み（features 補完 + コメント） |
| P2 | ecat-security src/lib.rs:118-127 | URI 百分号エンコードの SQLi（`?q=SELECT%20*%20...`）が header 層スキャンを迂回可能（検出器はリテラル空白を要求し、生の URI を先にデコードしない）。ボディスキャンは影響なし | ⏳ 要修正 |
| P3 | ecat-data-sqlx | `connect()/from_config()` が AnyPool を使用するがドライバ未インストール。sqlx 0.8.6 は初回接続で "No drivers installed" panic | ⏳ 要修正 |
| P3 | ecat-data-influxdb | 文字列 field が空白もエスケープ（`\ `）、line protocol 仕様では `"` と `\` のみエスケープすればよい。tag/field の順序が非決定的 | ⏳ 要修正 |
| P3 | ecat-data-clickhouse | テーブル作成キャッシュが永久に失効せず、外部で drop/ALTER した後に CREATE を再試行しない | ⏳ 要修正 |
| P3 | ecat-circuit-breaker | half_open_probes の上限は順次プローブでは到達不可（並行実行中のみ到達可）。ホワイトボックステストでカバー済み | ℹ️ 既知、欠陥ではない |
| P3 | ecat-health | `with_check` は blocking_write() を使用し、async コンテキストから呼ぶと panic。現在は同期コンテキストのみ使用可 | ℹ️ 既知、API 制限 |

## スキップしたモジュール（統合環境が必要、mock なし）

- 実 broker の往復：kafka/rabbitmq/mqtt/nats の publish-subscribe（設定とエラーパスはカバー済み）
- 実クラスタ：consul/etcd の登録-発見ライフサイクル（axum mock でリクエスト形状をカバー）
- 実データベース：redis/memcached の操作、mongod、influxdb サーバー側検証、sqlx postgres/mysql ドライバ、nebulagraph/arangodb API
- 実外部サービス：OAuth2 introspection（ローカル mock でカバー）、gRPC/HTTP の往復（ローカル mock で 302 非フォローをカバー）
