# Tutoriel de configuration des bases de données

**Version :** 2.4.2 · **Date :** 2026-08-01

Les 14 backends de données d'e-cat prennent tous en charge le chargement des informations de connexion depuis un fichier de configuration, sans codage en dur dans le code. `username` / `password` sont des champs optionnels ; s'ils sont omis, l'authentification est ignorée.

---

## Démarrage rapide

### 1. Créer le fichier de configuration

Copiez le modèle d'exemple et adaptez-le à votre environnement réel :

```bash
cp config/databases.example.yaml databases.yaml
```

Modifiez `databases.yaml` et renseignez les véritables informations de connexion :

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

### 2. Ajouter les dépendances

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
ecat-data-sqlx = { path = "../ecat-data-sqlx" }
ecat-data-redis = { path = "../ecat-data-redis" }
ecat-data-clickhouse = { path = "../ecat-data-clickhouse" }
```

### 3. Charger et utiliser

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

## Référence complète de la configuration

### Définir la structure de configuration de niveau supérieur

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

### Exemple YAML complet

Voir `config/databases.example.yaml`.

---

## Référence rapide des champs Config par backend

### RDBMS — SqlxConfig

```yaml
sql:
  url: "postgres://host:5432/dbname"
  # username: "app_user"    # 可选
  # password: "secret"      # 可选
```

| Champ | Type | Description |
|------|------|------|
| `url` | `String` | Chaîne de connexion sqlx, prend en charge SQLite/PG/MySQL/TiDB |
| `username` | `Option<String>` | Optionnel : authentification intégrée à l'URL (avec password) |
| `password` | `Option<String>` | Optionnel : authentification intégrée à l'URL (avec username) |

### Redis — RedisConfig

```yaml
redis:
  url: "redis://host:6379"
  # password: "auth_token"  # 可选
```

| Champ | Type | Description |
|------|------|------|
| `url` | `String` | URL de connexion Redis |
| `password` | `Option<String>` | Optionnel : mot de passe Redis AUTH |

### Memcached — MemcachedConfig

```yaml
memcached:
  # username: "memcache"    # 可选: 保留字段（当前为内存实现）
  # password: "secret"      # 可选: 保留字段
  {}
```

| Champ | Type | Description |
|------|------|------|
| `username` | `Option<String>` | Optionnel : champ réservé |
| `password` | `Option<String>` | Optionnel : champ réservé |

Il s'agit actuellement d'une implémentation en mémoire ; les champs d'authentification sont réservés pour une utilisation future.

### ClickHouse — ClickhouseConfig

```yaml
clickhouse:
  base_url: "http://host:8123"
  database: "default"
  # username: "default"   # 可选
  # password: "secret"    # 可选
```

| Champ | Type | Valeur par défaut | Description |
|------|------|--------|------|
| `base_url` | `String` | — | Adresse de l'interface HTTP |
| `database` | `String` | `"default"` | Nom de la base de données |
| `username` | `Option<String>` | `None` | Optionnel : nom d'utilisateur HTTP Basic Auth |
| `password` | `Option<String>` | `None` | Optionnel : mot de passe HTTP Basic Auth |

### QuestDB — QuestdbConfig

```yaml
questdb:
  base_url: "http://host:9000"
  # username: "admin"     # 可选
  # password: "quest"     # 可选
```

| Champ | Type | Description |
|------|------|------|
| `base_url` | `String` | Adresse de l'API HTTP |
| `username` | `Option<String>` | Optionnel : nom d'utilisateur HTTP Basic Auth |
| `password` | `Option<String>` | Optionnel : mot de passe HTTP Basic Auth |

### Elasticsearch — ElasticsearchConfig

```yaml
elasticsearch:
  base_url: "http://host:9200"
  # username: "elastic"   # 可选
  # password: "secret"    # 可选
```

| Champ | Type | Description |
|------|------|------|
| `base_url` | `String` | Adresse de l'API REST |
| `username` | `Option<String>` | Optionnel : nom d'utilisateur HTTP Basic Auth |
| `password` | `Option<String>` | Optionnel : mot de passe HTTP Basic Auth |

### OpenSearch — OpenSearchConfig

```yaml
opensearch:
  base_url: "http://host:9200"
  # username: "admin"     # 可选
  # password: "secret"    # 可选
```

| Champ | Type | Description |
|------|------|------|
| `base_url` | `String` | Adresse de l'API REST |
| `username` | `Option<String>` | Optionnel : nom d'utilisateur HTTP Basic Auth |
| `password` | `Option<String>` | Optionnel : mot de passe HTTP Basic Auth |

### InfluxDB — InfluxConfig

```yaml
influxdb:
  base_url: "http://host:8086"
  org: "myorg"
  bucket: "mybucket"
  token: "my-token"
```

| Champ | Type | Description |
|------|------|------|
| `base_url` | `String` | Adresse de l'API InfluxDB 2.x |
| `org` | `String` | Nom de l'organisation |
| `bucket` | `String` | Nom du bucket |
| `token` | `String` | Jeton d'authentification |

### Neo4j — Neo4jConfig

```yaml
neo4j:
  base_url: "http://host:7474"
  username: "neo4j"
  password: "secret"
```

| Champ | Type | Description |
|------|------|------|
| `base_url` | `String` | Adresse de l'API REST |
| `username` | `String` | Nom d'utilisateur |
| `password` | `String` | Mot de passe |

### NebulaGraph — NebulaGraphConfig

```yaml
nebulagraph:
  base_url: "http://host:19669"
  space: "my_space"
  # username: "root"      # 可选
  # password: "nebula"    # 可选
```

| Champ | Type | Description |
|------|------|------|
| `base_url` | `String` | Adresse de l'API |
| `space` | `String` | Nom de l'espace de graphe |
| `username` | `Option<String>` | Optionnel : nom d'utilisateur HTTP Basic Auth |
| `password` | `Option<String>` | Optionnel : mot de passe HTTP Basic Auth |

### ArangoDB — ArangoConfig

```yaml
arangodb:
  base_url: "http://host:8529"
  db: "mydb"
  username: "root"
  password: "secret"
```

| Champ | Type | Description |
|------|------|------|
| `base_url` | `String` | Adresse de l'API |
| `db` | `String` | Nom de la base de données |
| `username` | `String` | Nom d'utilisateur |
| `password` | `String` | Mot de passe |

### IoTDB — IotdbConfig

```yaml
iotdb:
  base_url: "http://host:18080"
  username: "root"
  password: "root"
```

| Champ | Type | Description |
|------|------|------|
| `base_url` | `String` | Adresse de l'API REST |
| `username` | `String` | Nom d'utilisateur |
| `password` | `String` | Mot de passe |

---

## Création programmatique

### Sans authentification

```rust
let es = ElasticsearchClient::new("http://localhost:9200");
let ch = ClickhouseClient::new("http://localhost:8123", "default");
```

### Avec authentification

```rust
let es = ElasticsearchClient::with_auth("http://es:9200", "elastic", "secret");
let ch = ClickhouseClient::with_auth("http://ch:8123", "default", "admin", "pass");
let qdb = QuestdbClient::with_auth("http://qdb:9000", "admin", "quest");
let ng = NebulaGraphClient::with_auth("http://ng:19669", "space1", "root", "nebula");
```

---

---

## Configuration TLS des certificats

Tous les backends de données prennent en charge l'authentification client TLS optionnelle (champ `tls`).

### Exemple de configuration

```yaml
clickhouse:
  base_url: "https://ch.internal:8443"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"
    # skip_verify: true  # 仅测试环境
```

### Génération automatique des certificats (ecat-tls)

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

### Génération manuelle (OpenSSL)

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

### Description des champs TLS

| Champ | Type | Description |
|------|------|------|
| `ca_cert` | `Option<String>` | Chemin du certificat CA PEM (validation du serveur) |
| `client_cert` | `Option<String>` | Chemin du certificat client PEM (mTLS) |
| `client_key` | `Option<String>` | Chemin de la clé privée client PEM (mTLS) |
| `skip_verify` | `Option<bool>` | Ignorer la validation des certificats (test uniquement) |

---

## Utilisation avancée

### Surcharge par variable d'environnement

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

### Intégration avec le framework ecat-config

```rust
use ecat_config::{Config, FileSource};

let mut app_config = Config::new();
app_config.load(&FileSource::new("databases.yaml")).await?;

let redis_cfg: RedisConfig = serde_json::from_value(
    app_config.get::<serde_json::Value>("redis").unwrap()
)?;
let cache = RedisCache::from_config(redis_cfg).await?;
```

### Configuration à la demande

Les bases de données non utilisées sont omises dans le YAML ; les structures Rust utilisent `Option` :

```rust
#[derive(Deserialize)]
struct AppConfig {
    sql: SqlxConfig,
    redis: Option<RedisConfig>,
    clickhouse: Option<ClickhouseConfig>,
}
```

---

## Documents associés

- [Rapport d'audit r5](audit-report-2026-08-01-r5.md)
- [Tutoriel d'authentification par certificat TLS](tls-certificate-tutorial.md)
- [Exemple de fichier de configuration](../../../config/databases.example.yaml)
