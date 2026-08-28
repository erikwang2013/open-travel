# TLS 証明書設定と認証チュートリアル

**バージョン:** 2.4.2 · **日付:** 2026-08-01

e-cat の 14 個のデータバックエンドはすべて TLS クライアント証明書認証（mTLS）に対応しています。本チュートリアルでは、証明書の生成、設定、すべてのデータベースバックエンドへの接続の完全な流れを説明します。

---

## 一、証明書の生成

### 方法 1：ecat-tls による自動生成（推奨）

```rust
use ecat_tls::{generate_ca, generate_server_cert, generate_client_cert};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("certs")?;

    // 1. 生成 CA 证书
    let ca = generate_ca("MyOrganization")?;
    fs::write("certs/ca.pem", &ca.cert_pem)?;
    fs::write("certs/ca-key.pem", &ca.key_pem)?;

    // 2. 生成服务端证书（部署到数据库服务器）
    let server = generate_server_cert("db.internal")?;
    fs::write("certs/server.pem", &server.cert_pem)?;
    fs::write("certs/server-key.pem", &server.key_pem)?;

    // 3. 生成客户端证书（应用侧使用，mTLS）
    let client = generate_client_cert("myapp")?;
    fs::write("certs/client.pem", &client.cert_pem)?;
    fs::write("certs/client-key.pem", &client.key_pem)?;

    Ok(())
}
```

### 方法 2：OpenSSL による手動生成

```bash
mkdir -p certs && cd certs

# 生成 CA
openssl req -x509 -newkey rsa:4096 \
  -keyout ca-key.pem -out ca.pem -days 3650 -nodes \
  -subj "/O=MyOrg/CN=MyOrg CA"

# 生成服务端证书
openssl req -new -newkey rsa:4096 \
  -keyout server-key.pem -out server.csr -nodes \
  -subj "/CN=db.internal"
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca-key.pem \
  -out server.pem -days 365

# 生成客户端证书 (mTLS)
openssl req -new -newkey rsa:4096 \
  -keyout client-key.pem -out client.csr -nodes \
  -subj "/CN=myapp"
openssl x509 -req -in client.csr -CA ca.pem -CAkey ca-key.pem \
  -out client.pem -days 365

rm -f *.csr
```

---

## 二、TLS 設定

### 共通 TLS フィールド

すべてのバックエンド Config は以下のオプションフィールドをサポートします（`#[serde(default)]`）：

| フィールド | 型 | 説明 |
|------|------|------|
| `tls.ca_cert` | `Option<String>` | CA 証明書 PEM パス（サーバー証明書の検証用） |
| `tls.client_cert` | `Option<String>` | クライアント証明書 PEM パス（mTLS） |
| `tls.client_key` | `Option<String>` | クライアント秘密鍵 PEM パス（mTLS） |
| `tls.skip_verify` | `Option<bool>` | 証明書検証のスキップ（テスト環境のみ） |

> ⚠️ 排他制約：`skip_verify=true` と `ca_cert` を同時に設定するとビルド時にエラーになります（`ecat-tls` は矛盾する設定を拒否します — 検証スキップと信頼アンカーの同時設定を防ぎ、誤設定による証明書検証のサイレント無効化を防ぎます）。

### YAML 設定例

```yaml
# 仅验证服务端证书
elasticsearch:
  base_url: "https://es.internal:9200"
  tls:
    ca_cert: "/etc/ecat/ca.pem"

# mTLS（双向认证）
clickhouse:
  base_url: "https://ch.internal:8443"
  database: "analytics"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"

# 测试环境（跳过验证）
questdb:
  base_url: "https://localhost:9000"
  tls:
    skip_verify: true
```

---

## 三、各バックエンドの TLS 設定

### HTTP バックエンド（9 個）

Elasticsearch, OpenSearch, ClickHouse, QuestDB, InfluxDB, Neo4j, NebulaGraph, ArangoDB, IoTDB — 統一して `TlsClientConfig::build_reqwest_client()` で TLS クライアントを構築します。

```yaml
# 所有 HTTP 后端使用相同格式
backend:
  base_url: "https://host:port"
  tls:
    ca_cert: "/path/to/ca.pem"
    client_cert: "/path/to/client.pem"   # mTLS 需要
    client_key: "/path/to/client-key.pem" # mTLS 需要
```

### Redis — URL scheme の自動切替

```yaml
redis:
  url: "redis://cache.internal:6379"    # 启用 TLS → 自动切换 rediss://
  tls:
    ca_cert: "/etc/ecat/ca.pem"
```

### RDBMS (Sqlx) — URL パラメータによる設定

```yaml
sql:
  url: "postgres://db.internal:5432/mydb?sslmode=require"
  tls: {}  # 保留字段
```

| データベース | TLS URL パラメータ |
|--------|------------|
| PostgreSQL | `?sslmode=require` または `?sslmode=verify-full` |
| MySQL | `?ssl-mode=VERIFY_CA&ssl-ca=/path/to/ca.pem` |
| TiDB | `?ssl-mode=VERIFY_IDENTITY&ssl-ca=/path/to/ca.pem` |
| SQLite | TLS 不要 |

---

## 四、Rust コードでのロード

```rust
use serde::Deserialize;
use ecat_data_elasticsearch::{ElasticsearchClient, ElasticsearchConfig};
use ecat_data_clickhouse::{ClickhouseClient, ClickhouseConfig};

#[derive(Deserialize)]
struct AppConfig {
    elasticsearch: ElasticsearchConfig,
    clickhouse: ClickhouseConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = std::fs::read_to_string("databases.yaml")?;
    let cfg: AppConfig = serde_yaml::from_str(&yaml)?;

    // from_config 内部调用 tls.build_reqwest_client() — TLS 自动应用
    let es = ElasticsearchClient::from_config(cfg.elasticsearch);
    let ch = ClickhouseClient::from_config(cfg.clickhouse);

    let results = es.search("logs", &serde_json::json!({"match_all": {}})).await?;
    Ok(())
}
```

---

## 五、プログラムによる作成（TLS + 認証）

```rust
use ecat_tls::TlsClientConfig;

// 手动构建 TLS 客户端
let tls = TlsClientConfig {
    ca_cert: Some("/etc/ecat/ca.pem".into()),
    client_cert: Some("/etc/ecat/client.pem".into()),
    client_key: Some("/etc/ecat/client-key.pem".into()),
    skip_verify: None,
};
let client = tls.build_reqwest_client()?;

// 或使用 with_auth + TLS 配置
let es = ElasticsearchClient::with_auth(
    "https://es.internal:9200", "elastic", "secret"
);
```

---

## 六、セキュリティ推奨事項

1. **本番環境では証明書を必ず検証する** — `skip_verify` を無効化
2. **CA 秘密鍵を安全に保管する** — バージョン管理に含めない
3. **証明書の有効期限管理** — 期限切れ前に更新・ローテーション
4. **mTLS でセキュリティ強化** — 本番ではクライアント証明書の同時設定を推奨

---

## 関連ドキュメント

- [データベース設定チュートリアル](database-config-tutorial.md)
- [監査レポート r5](audit-report-2026-08-01-r5.md)
- [設定サンプルファイル](../../../config/databases.example.yaml)
