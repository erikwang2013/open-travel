# データベース設定チュートリアル

**バージョン:** 2.4.2 · **日付:** 2026-08-01

e-cat の 14 個のデータバックエンドはすべて設定ファイルからの接続情報ロードに対応しており、コードへのハードコードは不要です。`username` / `password` はどちらもオプションフィールドで、省略すると認証をスキップします。

---

## クイックスタート

### 1. 設定ファイルの作成

サンプルテンプレートをコピーし、実際の環境に合わせて修正します：

```bash
cp config/databases.example.yaml databases.yaml
```

`databases.yaml` を編集し、実際の接続情報を入力します：

```yaml
# databases.yaml
sql:
  url: "postgres://myapp:secret@db.internal:5432/myapp"

redis:
  url: "redis://cache.internal:6379"

clickhouse:
  base_url: "http://ch.internal:8123"
  database: "analytics"
```

### 2. 依存関係の追加

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
ecat-data-sqlx = { path = "../ecat-data-sqlx" }
ecat-data-redis = { path = "../ecat-data-redis" }
ecat-data-clickhouse = { path = "../ecat-data-clickhouse" }
```

### 3. ロードして使用

```rust
use ecat_data_redis::{RedisCache, RedisConfig};
use ecat_data_sqlx::{SqlxClient, SqlxConfig};
use ecat_data_clickhouse::{ClickhouseClient, ClickhouseConfig};
use serde::Deserialize;

#[derive(Deserialize)]
struct AppConfig {
    sql: SqlxConfig,
    redis: RedisConfig,
    clickhouse: ClickhouseConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载 YAML 配置
    let yaml = std::fs::read_to_string("databases.yaml")?;
    let cfg: AppConfig = serde_yaml::from_str(&yaml)?;

    // 创建数据库客户端 — 无硬编码连接信息
    let db = SqlxClient::from_config(cfg.sql).await?;
    let cache = RedisCache::from_config(cfg.redis).await?;
    let ch = ClickhouseClient::from_config(cfg.clickhouse);

    // 使用
    let rows = db.query("SELECT id, name FROM users LIMIT 10").await?;
    cache.set("health", b"ok", std::time::Duration::from_secs(30)).await?;

    Ok(())
}
```

---

## 完全な設定リファレンス

### トップレベルの設定構造体の定義

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct DatabasesConfig {
    pub sql: ecat_data_sqlx::SqlxConfig,
    pub redis: ecat_data_redis::RedisConfig,
    pub memcached: ecat_data_memcached::MemcachedConfig,
    pub clickhouse: ecat_data_clickhouse::ClickhouseConfig,
    pub questdb: ecat_data_questdb::QuestdbConfig,
    pub elasticsearch: ecat_data_elasticsearch::ElasticsearchConfig,
    pub opensearch: ecat_data_opensearch::OpenSearchConfig,
    pub neo4j: ecat_data_neo4j::Neo4jConfig,
    pub nebulagraph: ecat_data_nebulagraph::NebulaGraphConfig,
    pub arangodb: ecat_data_arangodb::ArangoConfig,
    pub influxdb: ecat_data_influxdb::InfluxConfig,
    pub iotdb: ecat_data_iotdb::IotdbConfig,
}
```

### YAML 完全サンプル

`config/databases.example.yaml` を参照してください。

---

## 各バックエンドの Config フィールド早見表

### RDBMS — SqlxConfig

```yaml
sql:
  url: "postgres://host:5432/dbname"
  # username: "app_user"    # 可选
  # password: "secret"      # 可选
```

| フィールド | 型 | 説明 |
|------|------|------|
| `url` | `String` | sqlx 接続文字列、SQLite/PG/MySQL/TiDB に対応 |
| `username` | `Option<String>` | オプション：URL 埋め込み認証（password と併用） |
| `password` | `Option<String>` | オプション：URL 埋め込み認証（username と併用） |

### Redis — RedisConfig

```yaml
redis:
  url: "redis://host:6379"
  # password: "auth_token"  # 可选
```

| フィールド | 型 | 説明 |
|------|------|------|
| `url` | `String` | Redis 接続 URL |
| `password` | `Option<String>` | オプション：Redis AUTH パスワード |

### Memcached — MemcachedConfig

```yaml
memcached:
  # username: "memcache"    # 可选: 保留字段（当前为内存实现）
  # password: "secret"      # 可选: 保留字段
  {}
```

| フィールド | 型 | 説明 |
|------|------|------|
| `username` | `Option<String>` | オプション：保留フィールド |
| `password` | `Option<String>` | オプション：保留フィールド |

現在はメモリ実装のため、認証フィールドは予約のみです。

### ClickHouse — ClickhouseConfig

```yaml
clickhouse:
  base_url: "http://host:8123"
  database: "default"
  # username: "default"   # 可选
  # password: "secret"    # 可选
```

| フィールド | 型 | デフォルト値 | 説明 |
|------|------|--------|------|
| `base_url` | `String` | — | HTTP インターフェースのアドレス |
| `database` | `String` | `"default"` | データベース名 |
| `username` | `Option<String>` | `None` | オプション：HTTP Basic Auth ユーザー名 |
| `password` | `Option<String>` | `None` | オプション：HTTP Basic Auth パスワード |

### QuestDB — QuestdbConfig

```yaml
questdb:
  base_url: "http://host:9000"
  # username: "admin"     # 可选
  # password: "quest"     # 可选
```

| フィールド | 型 | 説明 |
|------|------|------|
| `base_url` | `String` | HTTP API アドレス |
| `username` | `Option<String>` | オプション：HTTP Basic Auth ユーザー名 |
| `password` | `Option<String>` | オプション：HTTP Basic Auth パスワード |

### Elasticsearch — ElasticsearchConfig

```yaml
elasticsearch:
  base_url: "http://host:9200"
  # username: "elastic"   # 可选
  # password: "secret"    # 可选
```

| フィールド | 型 | 説明 |
|------|------|------|
| `base_url` | `String` | REST API アドレス |
| `username` | `Option<String>` | オプション：HTTP Basic Auth ユーザー名 |
| `password` | `Option<String>` | オプション：HTTP Basic Auth パスワード |

### OpenSearch — OpenSearchConfig

```yaml
opensearch:
  base_url: "http://host:9200"
  # username: "admin"     # 可选
  # password: "secret"    # 可选
```

| フィールド | 型 | 説明 |
|------|------|------|
| `base_url` | `String` | REST API アドレス |
| `username` | `Option<String>` | オプション：HTTP Basic Auth ユーザー名 |
| `password` | `Option<String>` | オプション：HTTP Basic Auth パスワード |

### InfluxDB — InfluxConfig

```yaml
influxdb:
  base_url: "http://host:8086"
  org: "myorg"
  bucket: "mybucket"
  token: "my-token"
```

| フィールド | 型 | 説明 |
|------|------|------|
| `base_url` | `String` | InfluxDB 2.x API アドレス |
| `org` | `String` | 組織名 |
| `bucket` | `String` | バケット名 |
| `token` | `String` | 認証トークン |

### Neo4j — Neo4jConfig

```yaml
neo4j:
  base_url: "http://host:7474"
  username: "neo4j"
  password: "secret"
```

| フィールド | 型 | 説明 |
|------|------|------|
| `base_url` | `String` | REST API アドレス |
| `username` | `String` | ユーザー名 |
| `password` | `String` | パスワード |

### NebulaGraph — NebulaGraphConfig

```yaml
nebulagraph:
  base_url: "http://host:19669"
  space: "my_space"
  # username: "root"      # 可选
  # password: "nebula"    # 可选
```

| フィールド | 型 | 説明 |
|------|------|------|
| `base_url` | `String` | API アドレス |
| `space` | `String` | グラフスペース名 |
| `username` | `Option<String>` | オプション：HTTP Basic Auth ユーザー名 |
| `password` | `Option<String>` | オプション：HTTP Basic Auth パスワード |

### ArangoDB — ArangoConfig

```yaml
arangodb:
  base_url: "http://host:8529"
  db: "mydb"
  username: "root"
  password: "secret"
```

| フィールド | 型 | 説明 |
|------|------|------|
| `base_url` | `String` | API アドレス |
| `db` | `String` | データベース名 |
| `username` | `String` | ユーザー名 |
| `password` | `String` | パスワード |

### IoTDB — IotdbConfig

```yaml
iotdb:
  base_url: "http://host:18080"
  username: "root"
  password: "root"
```

| フィールド | 型 | 説明 |
|------|------|------|
| `base_url` | `String` | REST API アドレス |
| `username` | `String` | ユーザー名 |
| `password` | `String` | パスワード |

---

## プログラムによる作成

### 認証なし

```rust
let es = ElasticsearchClient::new("http://localhost:9200");
let ch = ClickhouseClient::new("http://localhost:8123", "default");
```

### 認証あり

```rust
let es = ElasticsearchClient::with_auth("http://es:9200", "elastic", "secret");
let ch = ClickhouseClient::with_auth("http://ch:8123", "default", "admin", "pass");
let qdb = QuestdbClient::with_auth("http://qdb:9000", "admin", "quest");
let ng = NebulaGraphClient::with_auth("http://ng:19669", "space1", "root", "nebula");
```

---

---

## TLS 証明書設定

すべてのデータバックエンドはオプションの TLS クライアント認証（`tls` フィールド）をサポートします。

### 設定例

```yaml
clickhouse:
  base_url: "https://ch.internal:8443"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"
    # skip_verify: true  # 仅测试环境
```

### 証明書の自動生成（ecat-tls）

```rust
use ecat_tls::{generate_ca, generate_server_cert, generate_client_cert};

// 1. 生成 CA
let ca = generate_ca("MyOrg")?;
std::fs::write("ca.pem", &ca.cert_pem)?;
std::fs::write("ca-key.pem", &ca.key_pem)?;

// 2. 生成服务端证书
let srv = generate_server_cert("db.example.com")?;
std::fs::write("server.pem", &srv.cert_pem)?;
std::fs::write("server-key.pem", &srv.key_pem)?;

// 3. 生成客户端证书（mTLS）
let client = generate_client_cert("myapp")?;
std::fs::write("client.pem", &client.cert_pem)?;
std::fs::write("client-key.pem", &client.key_pem)?;
```

### 手動生成（OpenSSL）

```bash
# CA
openssl req -x509 -newkey rsa:4096 -keyout ca-key.pem -out ca.pem -days 3650 -nodes

# 服务端证书
openssl req -new -newkey rsa:4096 -keyout server-key.pem -out server.csr -nodes -subj "/CN=db.example.com"
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca-key.pem -out server.pem -days 365

# 客户端证书 (mTLS)
openssl req -new -newkey rsa:4096 -keyout client-key.pem -out client.csr -nodes -subj "/CN=myapp"
openssl x509 -req -in client.csr -CA ca.pem -CAkey ca-key.pem -out client.pem -days 365
```

### TLS フィールドの説明

| フィールド | 型 | 説明 |
|------|------|------|
| `ca_cert` | `Option<String>` | CA 証明書 PEM パス（サーバー検証用） |
| `client_cert` | `Option<String>` | クライアント証明書 PEM パス（mTLS） |
| `client_key` | `Option<String>` | クライアント秘密鍵 PEM パス（mTLS） |
| `skip_verify` | `Option<bool>` | 証明書検証のスキップ（テストのみ） |

---

## 高度な使い方

### 環境変数によるオーバーライド

```rust
use std::env;

fn load_config() -> Result<SqlxConfig, Box<dyn std::error::Error>> {
    let mut cfg: SqlxConfig = serde_yaml::from_str(
        &std::fs::read_to_string("databases.yaml")?
    )?;
    if let Ok(url) = env::var("DATABASE_URL") {
        cfg.url = url;
    }
    Ok(cfg)
}
```

### ecat-config フレームワークとの連携

```rust
use ecat_config::{Config, FileSource};

let mut app_config = Config::new();
app_config.load(&FileSource::new("databases.yaml")).await?;

let redis_cfg: RedisConfig = serde_json::from_value(
    app_config.get::<serde_json::Value>("redis").unwrap()
)?;
let cache = RedisCache::from_config(redis_cfg).await?;
```

### 必要なものだけ設定

使わないデータベースは YAML で省略し、Rust 構造体では `Option` でマークします：

```rust
#[derive(Deserialize)]
struct AppConfig {
    sql: SqlxConfig,
    redis: Option<RedisConfig>,
    clickhouse: Option<ClickhouseConfig>,
}
```

---

## 関連ドキュメント

- [監査レポート r5](audit-report-2026-08-01-r5.md)
- [TLS 証明書認証チュートリアル](tls-certificate-tutorial.md)
- [設定サンプルファイル](../../../config/databases.example.yaml)
