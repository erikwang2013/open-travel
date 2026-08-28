# Tutorial Konfigurasi Database

**Versi:** 2.4.2 · **Tanggal:** 2026-08-01

Ke-14 backend data e-cat semuanya mendukung pemuatan info koneksi melalui file konfigurasi, tanpa perlu hardcode di kode. `username` / `password` keduanya kolom opsional, jika dihilangkan maka autentikasi dilewati.

---

## Memulai Cepat

### 1. Membuat File Konfigurasi

Salin template contoh dan sesuaikan dengan lingkungan aktual:

```bash
cp config/databases.example.yaml databases.yaml
```

Edit `databases.yaml`, isi dengan info koneksi yang sebenarnya:

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

### 2. Menambahkan Dependensi

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
ecat-data-sqlx = { path = "../ecat-data-sqlx" }
ecat-data-redis = { path = "../ecat-data-redis" }
ecat-data-clickhouse = { path = "../ecat-data-clickhouse" }
```

### 3. Memuat dan Menggunakan

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

## Referensi Konfigurasi Lengkap

### Mendefinisikan Struct Konfigurasi Tingkat Atas

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

### Contoh YAML Lengkap

Lihat `config/databases.example.yaml`.

---

## Referensi Cepat Kolom Config per Backend

### RDBMS — SqlxConfig

```yaml
sql:
  url: "postgres://host:5432/dbname"
  # username: "app_user"    # 可选
  # password: "secret"      # 可选
```

| Kolom | Tipe | Keterangan |
|------|------|------|
| `url` | `String` | String koneksi sqlx, mendukung SQLite/PG/MySQL/TiDB |
| `username` | `Option<String>` | Opsional: autentikasi tertanam di URL (berpasangan dengan password) |
| `password` | `Option<String>` | Opsional: autentikasi tertanam di URL (berpasangan dengan username) |

### Redis — RedisConfig

```yaml
redis:
  url: "redis://host:6379"
  # password: "auth_token"  # 可选
```

| Kolom | Tipe | Keterangan |
|------|------|------|
| `url` | `String` | URL koneksi Redis |
| `password` | `Option<String>` | Opsional: kata sandi AUTH Redis |

### Memcached — MemcachedConfig

```yaml
memcached:
  # username: "memcache"    # 可选: 保留字段（当前为内存实现）
  # password: "secret"      # 可选: 保留字段
  {}
```

| Kolom | Tipe | Keterangan |
|------|------|------|
| `username` | `Option<String>` | Opsional: kolom cadangan |
| `password` | `Option<String>` | Opsional: kolom cadangan |

Saat ini merupakan implementasi memori, kolom autentikasi dicadangkan.

### ClickHouse — ClickhouseConfig

```yaml
clickhouse:
  base_url: "http://host:8123"
  database: "default"
  # username: "default"   # 可选
  # password: "secret"    # 可选
```

| Kolom | Tipe | Nilai default | Keterangan |
|------|------|--------|------|
| `base_url` | `String` | — | Alamat antarmuka HTTP |
| `database` | `String` | `"default"` | Nama database |
| `username` | `Option<String>` | `None` | Opsional: nama pengguna HTTP Basic Auth |
| `password` | `Option<String>` | `None` | Opsional: kata sandi HTTP Basic Auth |

### QuestDB — QuestdbConfig

```yaml
questdb:
  base_url: "http://host:9000"
  # username: "admin"     # 可选
  # password: "quest"     # 可选
```

| Kolom | Tipe | Keterangan |
|------|------|------|
| `base_url` | `String` | Alamat HTTP API |
| `username` | `Option<String>` | Opsional: nama pengguna HTTP Basic Auth |
| `password` | `Option<String>` | Opsional: kata sandi HTTP Basic Auth |

### Elasticsearch — ElasticsearchConfig

```yaml
elasticsearch:
  base_url: "http://host:9200"
  # username: "elastic"   # 可选
  # password: "secret"    # 可选
```

| Kolom | Tipe | Keterangan |
|------|------|------|
| `base_url` | `String` | Alamat REST API |
| `username` | `Option<String>` | Opsional: nama pengguna HTTP Basic Auth |
| `password` | `Option<String>` | Opsional: kata sandi HTTP Basic Auth |

### OpenSearch — OpenSearchConfig

```yaml
opensearch:
  base_url: "http://host:9200"
  # username: "admin"     # 可选
  # password: "secret"    # 可选
```

| Kolom | Tipe | Keterangan |
|------|------|------|
| `base_url` | `String` | Alamat REST API |
| `username` | `Option<String>` | Opsional: nama pengguna HTTP Basic Auth |
| `password` | `Option<String>` | Opsional: kata sandi HTTP Basic Auth |

### InfluxDB — InfluxConfig

```yaml
influxdb:
  base_url: "http://host:8086"
  org: "myorg"
  bucket: "mybucket"
  token: "my-token"
```

| Kolom | Tipe | Keterangan |
|------|------|------|
| `base_url` | `String` | Alamat API InfluxDB 2.x |
| `org` | `String` | Nama organisasi |
| `bucket` | `String` | Nama bucket |
| `token` | `String` | Token autentikasi |

### Neo4j — Neo4jConfig

```yaml
neo4j:
  base_url: "http://host:7474"
  username: "neo4j"
  password: "secret"
```

| Kolom | Tipe | Keterangan |
|------|------|------|
| `base_url` | `String` | Alamat REST API |
| `username` | `String` | Nama pengguna |
| `password` | `String` | Kata sandi |

### NebulaGraph — NebulaGraphConfig

```yaml
nebulagraph:
  base_url: "http://host:19669"
  space: "my_space"
  # username: "root"      # 可选
  # password: "nebula"    # 可选
```

| Kolom | Tipe | Keterangan |
|------|------|------|
| `base_url` | `String` | Alamat API |
| `space` | `String` | Nama graph space |
| `username` | `Option<String>` | Opsional: nama pengguna HTTP Basic Auth |
| `password` | `Option<String>` | Opsional: kata sandi HTTP Basic Auth |

### ArangoDB — ArangoConfig

```yaml
arangodb:
  base_url: "http://host:8529"
  db: "mydb"
  username: "root"
  password: "secret"
```

| Kolom | Tipe | Keterangan |
|------|------|------|
| `base_url` | `String` | Alamat API |
| `db` | `String` | Nama database |
| `username` | `String` | Nama pengguna |
| `password` | `String` | Kata sandi |

### IoTDB — IotdbConfig

```yaml
iotdb:
  base_url: "http://host:18080"
  username: "root"
  password: "root"
```

| Kolom | Tipe | Keterangan |
|------|------|------|
| `base_url` | `String` | Alamat REST API |
| `username` | `String` | Nama pengguna |
| `password` | `String` | Kata sandi |

---

## Pembuatan Programatik

### Tanpa Autentikasi

```rust
let es = ElasticsearchClient::new("http://localhost:9200");
let ch = ClickhouseClient::new("http://localhost:8123", "default");
```

### Dengan Autentikasi

```rust
let es = ElasticsearchClient::with_auth("http://es:9200", "elastic", "secret");
let ch = ClickhouseClient::with_auth("http://ch:8123", "default", "admin", "pass");
let qdb = QuestdbClient::with_auth("http://qdb:9000", "admin", "quest");
let ng = NebulaGraphClient::with_auth("http://ng:19669", "space1", "root", "nebula");
```

---

---

## Konfigurasi Sertifikat TLS

Semua backend data mendukung autentikasi klien TLS opsional (kolom `tls`).

### Contoh Konfigurasi

```yaml
clickhouse:
  base_url: "https://ch.internal:8443"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"
    # skip_verify: true  # 仅测试环境
```

### Pembuatan Sertifikat Otomatis (ecat-tls)

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

### Pembuatan Manual (OpenSSL)

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

### Keterangan Kolom TLS

| Kolom | Tipe | Keterangan |
|------|------|------|
| `ca_cert` | `Option<String>` | Jalur PEM sertifikat CA (memverifikasi server) |
| `client_cert` | `Option<String>` | Jalur PEM sertifikat klien (mTLS) |
| `client_key` | `Option<String>` | Jalur PEM kunci privat klien (mTLS) |
| `skip_verify` | `Option<bool>` | Lewati verifikasi sertifikat (hanya pengujian) |

---

## Penggunaan Lanjutan

### Override Variabel Lingkungan

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

### Menggabungkan dengan Framework ecat-config

```rust
use ecat_config::{Config, FileSource};

let mut app_config = Config::new();
app_config.load(&FileSource::new("databases.yaml")).await?;

let redis_cfg: RedisConfig = serde_json::from_value(
    app_config.get::<serde_json::Value>("redis").unwrap()
)?;
let cache = RedisCache::from_config(redis_cfg).await?;
```

### Konfigurasi Sesuai Kebutuhan

Database yang tidak digunakan dihilangkan di YAML, struct Rust ditandai dengan `Option`:

```rust
#[derive(Deserialize)]
struct AppConfig {
    sql: SqlxConfig,
    redis: Option<RedisConfig>,
    clickhouse: Option<ClickhouseConfig>,
}
```

---

## Dokumen Terkait

- [Laporan Audit r5](audit-report-2026-08-01-r5.md)
- [Tutorial Sertifikat TLS](tls-certificate-tutorial.md)
- [File konfigurasi contoh](../../../config/databases.example.yaml)
