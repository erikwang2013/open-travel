# Tutorial Konfigurasi dan Autentikasi Sertifikat TLS

**Versi:** 2.4.2 · **Tanggal:** 2026-08-01

Ke-14 backend data e-cat semuanya mendukung autentikasi sertifikat klien TLS (mTLS). Tutorial ini mencakup alur lengkap pembuatan sertifikat, konfigurasi, dan koneksi ke semua backend database.

---

## 1. Pembuatan Sertifikat

### Cara 1: Pembuatan Otomatis ecat-tls (Direkomendasikan)

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

### Cara 2: Pembuatan Manual dengan OpenSSL

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

## 2. Konfigurasi TLS

### Kolom TLS Umum

Semua Config backend mendukung kolom opsional berikut (`#[serde(default)]`):

| Kolom | Tipe | Keterangan |
|------|------|------|
| `tls.ca_cert` | `Option<String>` | Jalur PEM sertifikat CA (memverifikasi sertifikat server) |
| `tls.client_cert` | `Option<String>` | Jalur PEM sertifikat klien (mTLS) |
| `tls.client_key` | `Option<String>` | Jalur PEM kunci privat klien (mTLS) |
| `tls.skip_verify` | `Option<bool>` | Lewati verifikasi sertifikat (hanya lingkungan pengujian) |

> ⚠️ Saling eksklusif: konfigurasi `skip_verify=true` bersama `ca_cert` akan langsung error saat build (`ecat-tls` menolak konfigurasi yang kontradiktif — melewati verifikasi namun mengonfigurasi trust anchor, mencegah miskonfigurasi yang diam-diam menonaktifkan verifikasi sertifikat).

### Contoh Konfigurasi YAML

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

## 3. Konfigurasi TLS per Backend

### Backend HTTP (9 buah)

Elasticsearch, OpenSearch, ClickHouse, QuestDB, InfluxDB, Neo4j, NebulaGraph, ArangoDB, IoTDB — terpadu melalui `TlsClientConfig::build_reqwest_client()` untuk membangun TLS Client.

```yaml
# 所有 HTTP 后端使用相同格式
backend:
  base_url: "https://host:port"
  tls:
    ca_cert: "/path/to/ca.pem"
    client_cert: "/path/to/client.pem"   # mTLS 需要
    client_key: "/path/to/client-key.pem" # mTLS 需要
```

### Redis — Beralih URL Scheme Otomatis

```yaml
redis:
  url: "redis://cache.internal:6379"    # 启用 TLS → 自动切换 rediss://
  tls:
    ca_cert: "/etc/ecat/ca.pem"
```

### RDBMS (Sqlx) — Konfigurasi Parameter URL

```yaml
sql:
  url: "postgres://db.internal:5432/mydb?sslmode=require"
  tls: {}  # 保留字段
```

| Database | Parameter URL TLS |
|--------|------------|
| PostgreSQL | `?sslmode=require` atau `?sslmode=verify-full` |
| MySQL | `?ssl-mode=VERIFY_CA&ssl-ca=/path/to/ca.pem` |
| TiDB | `?ssl-mode=VERIFY_IDENTITY&ssl-ca=/path/to/ca.pem` |
| SQLite | Tidak perlu TLS |

---

## 4. Memuat dengan Kode Rust

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

## 5. Pembuatan Programatik (TLS + Autentikasi)

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

## 6. Rekomendasi Keamanan

1. **Produksi wajib memverifikasi sertifikat** — nonaktifkan `skip_verify`
2. **Simpan kunci privat CA dengan aman** — tidak masuk ke version control
3. **Kelola masa berlaku sertifikat** — perpanjang dan rotasi sebelum kedaluwarsa
4. **mTLS memperkuat keamanan** — produksi disarankan mengonfigurasi sertifikat klien secara bersamaan

---

## Dokumen Terkait

- [Tutorial Konfigurasi Database](database-config-tutorial.md)
- [Laporan Audit r5](audit-report-2026-08-01-r5.md)
- [File konfigurasi contoh](../../../config/databases.example.yaml)
