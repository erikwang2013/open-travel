# Tutoriel de configuration et d'authentification des certificats TLS

**Version :** 2.4.2 · **Date :** 2026-08-01

Les 14 backends de données d'e-cat prennent tous en charge l'authentification par certificat client TLS (mTLS). Ce tutoriel couvre le processus complet de génération des certificats, de configuration et de connexion à tous les backends de données.

---

## I. Génération des certificats

### Méthode 1 : génération automatique avec ecat-tls (recommandé)

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

### Méthode 2 : génération manuelle avec OpenSSL

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

## II. Configuration TLS

### Champs TLS génériques

Tous les Config de backends prennent en charge les champs optionnels suivants (`#[serde(default)]`) :

| Champ | Type | Description |
|------|------|------|
| `tls.ca_cert` | `Option<String>` | Chemin du certificat CA PEM (validation du certificat serveur) |
| `tls.client_cert` | `Option<String>` | Chemin du certificat client PEM (mTLS) |
| `tls.client_key` | `Option<String>` | Chemin de la clé privée client PEM (mTLS) |
| `tls.skip_verify` | `Option<bool>` | Ignorer la validation des certificats (environnement de test uniquement) |

> ⚠️ Mutuellement exclusifs : configurer `skip_verify=true` et `ca_cert` simultanément provoque une erreur de compilation (`ecat-tls` rejette les configurations contradictoires — ignorer la validation tout en configurant une ancre de confiance — pour éviter qu'une erreur de configuration ne désactive silencieusement la validation des certificats).

### Exemple de configuration YAML

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

## III. Configuration TLS par backend

### Backends HTTP (9)

Elasticsearch, OpenSearch, ClickHouse, QuestDB, InfluxDB, Neo4j, NebulaGraph, ArangoDB, IoTDB — tous construisent le client TLS via `TlsClientConfig::build_reqwest_client()`.

```yaml
# 所有 HTTP 后端使用相同格式
backend:
  base_url: "https://host:port"
  tls:
    ca_cert: "/path/to/ca.pem"
    client_cert: "/path/to/client.pem"   # mTLS 需要
    client_key: "/path/to/client-key.pem" # mTLS 需要
```

### Redis — bascule automatique du schéma d'URL

```yaml
redis:
  url: "redis://cache.internal:6379"    # 启用 TLS → 自动切换 rediss://
  tls:
    ca_cert: "/etc/ecat/ca.pem"
```

### RDBMS (Sqlx) — configuration par paramètres d'URL

```yaml
sql:
  url: "postgres://db.internal:5432/mydb?sslmode=require"
  tls: {}  # 保留字段
```

| Base de données | Paramètres TLS de l'URL |
|--------|------------|
| PostgreSQL | `?sslmode=require` ou `?sslmode=verify-full` |
| MySQL | `?ssl-mode=VERIFY_CA&ssl-ca=/path/to/ca.pem` |
| TiDB | `?ssl-mode=VERIFY_IDENTITY&ssl-ca=/path/to/ca.pem` |
| SQLite | TLS inutile |

---

## IV. Chargement dans le code Rust

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

## V. Création programmatique (TLS + authentification)

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

## VI. Recommandations de sécurité

1. **La validation des certificats est obligatoire en production** — désactivez `skip_verify`
2. **Stockage sécurisé de la clé privée du CA** — ne la mettez pas sous contrôle de version
3. **Gestion de la validité des certificats** — renouvelez et faites la rotation avant expiration
4. **mTLS renforce la sécurité** — en production, configurez également le certificat client

---

## Documents associés

- [Tutoriel de configuration des bases de données](database-config-tutorial.md)
- [Rapport d'audit r5](audit-report-2026-08-01-r5.md)
- [Exemple de fichier de configuration](../../../config/databases.example.yaml)
