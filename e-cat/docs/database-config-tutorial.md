# 数据库配置教程

**版本:** 2.4.2 · **日期:** 2026-08-01

e-cat 的 14 个数据后端均支持通过配置文件加载连接信息，无需在代码中硬编码。`username` / `password` 均为可选字段，省略则跳过认证。

---

## 快速开始

### 1. 创建配置文件

复制示例模板并根据实际环境修改：

```bash
cp config/databases.example.yaml databases.yaml
```

编辑 `databases.yaml`，填入真实的连接信息：

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

### 2. 引入依赖

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
ecat-data-sqlx = { path = "../ecat-data-sqlx" }
ecat-data-redis = { path = "../ecat-data-redis" }
ecat-data-clickhouse = { path = "../ecat-data-clickhouse" }
```

### 3. 加载并使用

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

## 完整配置参考

### 定义顶层配置结构体

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

### YAML 完整示例

见 `config/databases.example.yaml`。

---

## 各后端 Config 字段速查

### RDBMS — SqlxConfig

```yaml
sql:
  url: "postgres://host:5432/dbname"
  # username: "app_user"    # 可选
  # password: "secret"      # 可选
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `url` | `String` | sqlx 连接串，支持 SQLite/PG/MySQL/TiDB |
| `username` | `Option<String>` | 可选：嵌入 URL 认证（与 password 配合） |
| `password` | `Option<String>` | 可选：嵌入 URL 认证（与 username 配合） |

### Redis — RedisConfig

```yaml
redis:
  url: "redis://host:6379"
  # password: "auth_token"  # 可选
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `url` | `String` | Redis 连接 URL |
| `password` | `Option<String>` | 可选：Redis AUTH 密码 |

### Memcached — MemcachedConfig

```yaml
memcached:
  # username: "memcache"    # 可选: 保留字段（当前为内存实现）
  # password: "secret"      # 可选: 保留字段
  {}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `username` | `Option<String>` | 可选：保留字段 |
| `password` | `Option<String>` | 可选：保留字段 |

当前为内存实现，认证字段预留。

### ClickHouse — ClickhouseConfig

```yaml
clickhouse:
  base_url: "http://host:8123"
  database: "default"
  # username: "default"   # 可选
  # password: "secret"    # 可选
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `base_url` | `String` | — | HTTP 接口地址 |
| `database` | `String` | `"default"` | 数据库名 |
| `username` | `Option<String>` | `None` | 可选：HTTP Basic Auth 用户名 |
| `password` | `Option<String>` | `None` | 可选：HTTP Basic Auth 密码 |

### QuestDB — QuestdbConfig

```yaml
questdb:
  base_url: "http://host:9000"
  # username: "admin"     # 可选
  # password: "quest"     # 可选
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `base_url` | `String` | HTTP API 地址 |
| `username` | `Option<String>` | 可选：HTTP Basic Auth 用户名 |
| `password` | `Option<String>` | 可选：HTTP Basic Auth 密码 |

### Elasticsearch — ElasticsearchConfig

```yaml
elasticsearch:
  base_url: "http://host:9200"
  # username: "elastic"   # 可选
  # password: "secret"    # 可选
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `base_url` | `String` | REST API 地址 |
| `username` | `Option<String>` | 可选：HTTP Basic Auth 用户名 |
| `password` | `Option<String>` | 可选：HTTP Basic Auth 密码 |

### OpenSearch — OpenSearchConfig

```yaml
opensearch:
  base_url: "http://host:9200"
  # username: "admin"     # 可选
  # password: "secret"    # 可选
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `base_url` | `String` | REST API 地址 |
| `username` | `Option<String>` | 可选：HTTP Basic Auth 用户名 |
| `password` | `Option<String>` | 可选：HTTP Basic Auth 密码 |

### InfluxDB — InfluxConfig

```yaml
influxdb:
  base_url: "http://host:8086"
  org: "myorg"
  bucket: "mybucket"
  token: "my-token"
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `base_url` | `String` | InfluxDB 2.x API 地址 |
| `org` | `String` | 组织名 |
| `bucket` | `String` | 桶名 |
| `token` | `String` | 认证令牌 |

### Neo4j — Neo4jConfig

```yaml
neo4j:
  base_url: "http://host:7474"
  username: "neo4j"
  password: "secret"
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `base_url` | `String` | REST API 地址 |
| `username` | `String` | 用户名 |
| `password` | `String` | 密码 |

### NebulaGraph — NebulaGraphConfig

```yaml
nebulagraph:
  base_url: "http://host:19669"
  space: "my_space"
  # username: "root"      # 可选
  # password: "nebula"    # 可选
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `base_url` | `String` | API 地址 |
| `space` | `String` | 图空间名 |
| `username` | `Option<String>` | 可选：HTTP Basic Auth 用户名 |
| `password` | `Option<String>` | 可选：HTTP Basic Auth 密码 |

### ArangoDB — ArangoConfig

```yaml
arangodb:
  base_url: "http://host:8529"
  db: "mydb"
  username: "root"
  password: "secret"
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `base_url` | `String` | API 地址 |
| `db` | `String` | 数据库名 |
| `username` | `String` | 用户名 |
| `password` | `String` | 密码 |

### IoTDB — IotdbConfig

```yaml
iotdb:
  base_url: "http://host:18080"
  username: "root"
  password: "root"
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `base_url` | `String` | REST API 地址 |
| `username` | `String` | 用户名 |
| `password` | `String` | 密码 |

---

## 程序化创建

### 无需认证

```rust
let es = ElasticsearchClient::new("http://localhost:9200");
let ch = ClickhouseClient::new("http://localhost:8123", "default");
```

### 带认证

```rust
let es = ElasticsearchClient::with_auth("http://es:9200", "elastic", "secret");
let ch = ClickhouseClient::with_auth("http://ch:8123", "default", "admin", "pass");
let qdb = QuestdbClient::with_auth("http://qdb:9000", "admin", "quest");
let ng = NebulaGraphClient::with_auth("http://ng:19669", "space1", "root", "nebula");
```

---

---

## TLS 证书配置

所有数据后端均支持可选的 TLS 客户端认证（`tls` 字段）。

### 配置示例

```yaml
clickhouse:
  base_url: "https://ch.internal:8443"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"
    # skip_verify: true  # 仅测试环境
```

### 证书自动生成（ecat-tls）

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

### 手动生成（OpenSSL）

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

### TLS 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `ca_cert` | `Option<String>` | CA 证书 PEM 路径（验证服务端） |
| `client_cert` | `Option<String>` | 客户端证书 PEM 路径（mTLS） |
| `client_key` | `Option<String>` | 客户端私钥 PEM 路径（mTLS） |
| `skip_verify` | `Option<bool>` | 跳过证书验证（仅测试） |

---

## 进阶用法

### 环境变量覆盖

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

### 结合 ecat-config 框架

```rust
use ecat_config::{Config, FileSource};

let mut app_config = Config::new();
app_config.load(&FileSource::new("databases.yaml")).await?;

let redis_cfg: RedisConfig = serde_json::from_value(
    app_config.get::<serde_json::Value>("redis").unwrap()
)?;
let cache = RedisCache::from_config(redis_cfg).await?;
```

### 按需配置

不用的数据库在 YAML 中省略，Rust 结构体用 `Option` 标记：

```rust
#[derive(Deserialize)]
struct AppConfig {
    sql: SqlxConfig,
    redis: Option<RedisConfig>,
    clickhouse: Option<ClickhouseConfig>,
}
```

---

## 相关文档

- [审计报告 r5](audit-report-2026-08-01-r5.md)
- [TLS 证书认证教程](tls-certificate-tutorial.md)
- [示例配置文件](../config/databases.example.yaml)
