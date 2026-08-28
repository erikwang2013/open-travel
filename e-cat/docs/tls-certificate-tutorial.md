# TLS 证书配置与认证教程

**版本:** 2.4.2 · **日期:** 2026-08-01

e-cat 的 14 个数据后端全部支持 TLS 客户端证书认证（mTLS）。本教程涵盖证书生成、配置、和连接到所有数据库后端的完整流程。

---

## 一、证书生成

### 方式 1：ecat-tls 自动生成（推荐）

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

### 方式 2：OpenSSL 手动生成

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

## 二、TLS 配置

### 通用 TLS 字段

所有后端 Config 均支持以下可选字段（`#[serde(default)]`）：

| 字段 | 类型 | 说明 |
|------|------|------|
| `tls.ca_cert` | `Option<String>` | CA 证书 PEM 路径（验证服务端证书） |
| `tls.client_cert` | `Option<String>` | 客户端证书 PEM 路径（mTLS） |
| `tls.client_key` | `Option<String>` | 客户端私钥 PEM 路径（mTLS） |
| `tls.skip_verify` | `Option<bool>` | 跳过证书验证（仅测试环境） |

> ⚠️ 互斥：`skip_verify=true` 与 `ca_cert` 同时配置会在构建时直接报错（`ecat-tls` 拒绝矛盾配置——跳过校验却配置信任锚，防止误配静默关闭证书校验）。

### YAML 配置示例

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

## 三、各后端 TLS 配置

### HTTP 后端（9 个）

Elasticsearch, OpenSearch, ClickHouse, QuestDB, InfluxDB, Neo4j, NebulaGraph, ArangoDB, IoTDB — 统一通过 `TlsClientConfig::build_reqwest_client()` 构建 TLS Client。

```yaml
# 所有 HTTP 后端使用相同格式
backend:
  base_url: "https://host:port"
  tls:
    ca_cert: "/path/to/ca.pem"
    client_cert: "/path/to/client.pem"   # mTLS 需要
    client_key: "/path/to/client-key.pem" # mTLS 需要
```

### Redis — 自动 URL scheme 切换

```yaml
redis:
  url: "redis://cache.internal:6379"    # 启用 TLS → 自动切换 rediss://
  tls:
    ca_cert: "/etc/ecat/ca.pem"
```

### RDBMS (Sqlx) — URL 参数配置

```yaml
sql:
  url: "postgres://db.internal:5432/mydb?sslmode=require"
  tls: {}  # 保留字段
```

| 数据库 | TLS URL 参数 |
|--------|------------|
| PostgreSQL | `?sslmode=require` 或 `?sslmode=verify-full` |
| MySQL | `?ssl-mode=VERIFY_CA&ssl-ca=/path/to/ca.pem` |
| TiDB | `?ssl-mode=VERIFY_IDENTITY&ssl-ca=/path/to/ca.pem` |
| SQLite | 无需 TLS |

---

## 四、Rust 代码加载

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

## 五、程序化创建（TLS + 认证）

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

## 六、安全建议

1. **生产环境必须验证证书** — 禁用 `skip_verify`
2. **CA 私钥安全存储** — 不纳入版本控制
3. **证书有效期管理** — 到期前续签并轮换
4. **mTLS 增强安全** — 生产建议同时配置客户端证书

---

## 相关文档

- [数据库配置教程](database-config-tutorial.md)
- [审计报告 r5](audit-report-2026-08-01-r5.md)
- [配置示例文件](../config/databases.example.yaml)
