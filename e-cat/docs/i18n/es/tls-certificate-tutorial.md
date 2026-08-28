# Tutorial de configuración y autenticación de certificados TLS

**Versión:** 2.4.2 · **Fecha:** 2026-08-01

Los 14 backends de datos de e-cat admiten autenticación TLS de cliente (mTLS). Este tutorial cubre el flujo completo de generación de certificados, configuración y conexión a todos los backends de bases de datos.

---

## 一、Generación de certificados

### Método 1: generación automática con ecat-tls (recomendado)

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

### Método 2: generación manual con OpenSSL

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

## 二、Configuración TLS

### Campos TLS comunes

Todos los backends Config admiten los siguientes campos opcionales (`#[serde(default)]`):

| Campo | Tipo | Descripción |
|------|------|------|
| `tls.ca_cert` | `Option<String>` | Ruta del certificado CA PEM (verifica el certificado del servidor) |
| `tls.client_cert` | `Option<String>` | Ruta del certificado de cliente PEM (mTLS) |
| `tls.client_key` | `Option<String>` | Ruta de la clave privada de cliente PEM (mTLS) |
| `tls.skip_verify` | `Option<bool>` | Omite la verificación de certificados (solo entornos de prueba) |

> ⚠️ Mutuamente excluyentes: configurar `skip_verify=true` junto con `ca_cert` produce un error en tiempo de construcción (`ecat-tls` rechaza configuraciones contradictorias: omitir la verificación pero configurar un ancla de confianza, evitando que una mala configuración desactive silenciosamente la verificación de certificados).

### Ejemplo de configuración YAML

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

## 三、Configuración TLS por backend

### Backends HTTP (9)

Elasticsearch, OpenSearch, ClickHouse, QuestDB, InfluxDB, Neo4j, NebulaGraph, ArangoDB, IoTDB — construyen el cliente TLS de forma unificada mediante `TlsClientConfig::build_reqwest_client()`.

```yaml
# 所有 HTTP 后端使用相同格式
backend:
  base_url: "https://host:port"
  tls:
    ca_cert: "/path/to/ca.pem"
    client_cert: "/path/to/client.pem"   # mTLS 需要
    client_key: "/path/to/client-key.pem" # mTLS 需要
```

### Redis — cambio automático de scheme en la URL

```yaml
redis:
  url: "redis://cache.internal:6379"    # 启用 TLS → 自动切换 rediss://
  tls:
    ca_cert: "/etc/ecat/ca.pem"
```

### RDBMS (Sqlx) — configuración mediante parámetros de URL

```yaml
sql:
  url: "postgres://db.internal:5432/mydb?sslmode=require"
  tls: {}  # 保留字段
```

| Base de datos | Parámetros TLS de URL |
|--------|------------|
| PostgreSQL | `?sslmode=require` o `?sslmode=verify-full` |
| MySQL | `?ssl-mode=VERIFY_CA&ssl-ca=/path/to/ca.pem` |
| TiDB | `?ssl-mode=VERIFY_IDENTITY&ssl-ca=/path/to/ca.pem` |
| SQLite | No requiere TLS |

---

## 四、Carga desde código Rust

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

## 五、Creación programática (TLS + autenticación)

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

## 六、Recomendaciones de seguridad

1. **En producción se debe verificar el certificado** — desactiva `skip_verify`
2. **Almacena la clave privada de la CA de forma segura** — no la incluyas en el control de versiones
3. **Gestiona la validez de los certificados** — renueva y rota antes de la expiración
4. **mTLS refuerza la seguridad** — en producción se recomienda configurar también el certificado de cliente

---

## Documentos relacionados

- [Tutorial de configuración de base de datos](database-config-tutorial.md)
- [Informe de auditoría r5](audit-report-2026-08-01-r5.md)
- [Archivo de ejemplo de configuración](../../../config/databases.example.yaml)
