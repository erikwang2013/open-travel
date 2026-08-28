# Руководство по настройке баз данных

**Версия:** 2.4.2 · **Дата:** 2026-08-01

Все 14 бэкендов данных e-cat поддерживают загрузку параметров подключения из конфигурационного файла — без хардкода в коде. Поля `username` / `password` опциональны: если опущены, аутентификация пропускается.

---

## Быстрый старт

### 1. Создание конфигурационного файла

Скопируйте пример шаблона и измените под своё окружение:

```bash
cp config/databases.example.yaml databases.yaml
```

Отредактируйте `databases.yaml`, вписав реальные параметры подключения:

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

### 2. Добавление зависимостей

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
ecat-data-sqlx = { path = "../ecat-data-sqlx" }
ecat-data-redis = { path = "../ecat-data-redis" }
ecat-data-clickhouse = { path = "../ecat-data-clickhouse" }
```

### 3. Загрузка и использование

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

## Полный справочник конфигурации

### Определение структуры конфигурации верхнего уровня

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

### Полный пример YAML

См. `config/databases.example.yaml`.

---

## Справочник полей Config по бэкендам

### RDBMS — SqlxConfig

```yaml
sql:
  url: "postgres://host:5432/dbname"
  # username: "app_user"    # 可选
  # password: "secret"      # 可选
```

| Поле | Тип | Описание |
|------|------|------|
| `url` | `String` | Строка подключения sqlx, поддерживает SQLite/PG/MySQL/TiDB |
| `username` | `Option<String>` | Опционально: встраивание аутентификации в URL (совместно с password) |
| `password` | `Option<String>` | Опционально: встраивание аутентификации в URL (совместно с username) |

### Redis — RedisConfig

```yaml
redis:
  url: "redis://host:6379"
  # password: "auth_token"  # 可选
```

| Поле | Тип | Описание |
|------|------|------|
| `url` | `String` | URL подключения к Redis |
| `password` | `Option<String>` | Опционально: пароль Redis AUTH |

### Memcached — MemcachedConfig

```yaml
memcached:
  # username: "memcache"    # 可选: 保留字段（当前为内存实现）
  # password: "secret"      # 可选: 保留字段
  {}
```

| Поле | Тип | Описание |
|------|------|------|
| `username` | `Option<String>` | Опционально: зарезервированное поле |
| `password` | `Option<String>` | Опционально: зарезервированное поле |

В настоящее время это реализация в памяти; поля аутентификации зарезервированы.

### ClickHouse — ClickhouseConfig

```yaml
clickhouse:
  base_url: "http://host:8123"
  database: "default"
  # username: "default"   # 可选
  # password: "secret"    # 可选
```

| Поле | Тип | Значение по умолчанию | Описание |
|------|------|--------|------|
| `base_url` | `String` | — | Адрес HTTP-интерфейса |
| `database` | `String` | `"default"` | Имя базы данных |
| `username` | `Option<String>` | `None` | Опционально: имя пользователя HTTP Basic Auth |
| `password` | `Option<String>` | `None` | Опционально: пароль HTTP Basic Auth |

### QuestDB — QuestdbConfig

```yaml
questdb:
  base_url: "http://host:9000"
  # username: "admin"     # 可选
  # password: "quest"     # 可选
```

| Поле | Тип | Описание |
|------|------|------|
| `base_url` | `String` | Адрес HTTP API |
| `username` | `Option<String>` | Опционально: имя пользователя HTTP Basic Auth |
| `password` | `Option<String>` | Опционально: пароль HTTP Basic Auth |

### Elasticsearch — ElasticsearchConfig

```yaml
elasticsearch:
  base_url: "http://host:9200"
  # username: "elastic"   # 可选
  # password: "secret"    # 可选
```

| Поле | Тип | Описание |
|------|------|------|
| `base_url` | `String` | Адрес REST API |
| `username` | `Option<String>` | Опционально: имя пользователя HTTP Basic Auth |
| `password` | `Option<String>` | Опционально: пароль HTTP Basic Auth |

### OpenSearch — OpenSearchConfig

```yaml
opensearch:
  base_url: "http://host:9200"
  # username: "admin"     # 可选
  # password: "secret"    # 可选
```

| Поле | Тип | Описание |
|------|------|------|
| `base_url` | `String` | Адрес REST API |
| `username` | `Option<String>` | Опционально: имя пользователя HTTP Basic Auth |
| `password` | `Option<String>` | Опционально: пароль HTTP Basic Auth |

### InfluxDB — InfluxConfig

```yaml
influxdb:
  base_url: "http://host:8086"
  org: "myorg"
  bucket: "mybucket"
  token: "my-token"
```

| Поле | Тип | Описание |
|------|------|------|
| `base_url` | `String` | Адрес API InfluxDB 2.x |
| `org` | `String` | Имя организации |
| `bucket` | `String` | Имя bucket-а |
| `token` | `String` | Токен аутентификации |

### Neo4j — Neo4jConfig

```yaml
neo4j:
  base_url: "http://host:7474"
  username: "neo4j"
  password: "secret"
```

| Поле | Тип | Описание |
|------|------|------|
| `base_url` | `String` | Адрес REST API |
| `username` | `String` | Имя пользователя |
| `password` | `String` | Пароль |

### NebulaGraph — NebulaGraphConfig

```yaml
nebulagraph:
  base_url: "http://host:19669"
  space: "my_space"
  # username: "root"      # 可选
  # password: "nebula"    # 可选
```

| Поле | Тип | Описание |
|------|------|------|
| `base_url` | `String` | Адрес API |
| `space` | `String` | Имя graph space |
| `username` | `Option<String>` | Опционально: имя пользователя HTTP Basic Auth |
| `password` | `Option<String>` | Опционально: пароль HTTP Basic Auth |

### ArangoDB — ArangoConfig

```yaml
arangodb:
  base_url: "http://host:8529"
  db: "mydb"
  username: "root"
  password: "secret"
```

| Поле | Тип | Описание |
|------|------|------|
| `base_url` | `String` | Адрес API |
| `db` | `String` | Имя базы данных |
| `username` | `String` | Имя пользователя |
| `password` | `String` | Пароль |

### IoTDB — IotdbConfig

```yaml
iotdb:
  base_url: "http://host:18080"
  username: "root"
  password: "root"
```

| Поле | Тип | Описание |
|------|------|------|
| `base_url` | `String` | Адрес REST API |
| `username` | `String` | Имя пользователя |
| `password` | `String` | Пароль |

---

## Программное создание

### Без аутентификации

```rust
let es = ElasticsearchClient::new("http://localhost:9200");
let ch = ClickhouseClient::new("http://localhost:8123", "default");
```

### С аутентификацией

```rust
let es = ElasticsearchClient::with_auth("http://es:9200", "elastic", "secret");
let ch = ClickhouseClient::with_auth("http://ch:8123", "default", "admin", "pass");
let qdb = QuestdbClient::with_auth("http://qdb:9000", "admin", "quest");
let ng = NebulaGraphClient::with_auth("http://ng:19669", "space1", "root", "nebula");
```

---

---

## Настройка TLS-сертификатов

Все бэкенды данных поддерживают опциональную TLS-аутентификацию клиента (поле `tls`).

### Пример конфигурации

```yaml
clickhouse:
  base_url: "https://ch.internal:8443"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"
    # skip_verify: true  # 仅测试环境
```

### Автогенерация сертификатов (ecat-tls)

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

### Ручная генерация (OpenSSL)

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

### Описание полей TLS

| Поле | Тип | Описание |
|------|------|------|
| `ca_cert` | `Option<String>` | Путь к PEM-файлу CA-сертификата (проверка сервера) |
| `client_cert` | `Option<String>` | Путь к PEM-файлу клиентского сертификата (mTLS) |
| `client_key` | `Option<String>` | Путь к PEM-файлу приватного ключа клиента (mTLS) |
| `skip_verify` | `Option<bool>` | Пропустить проверку сертификатов (только тест) |

---

## Продвинутое использование

### Переопределение через переменные окружения

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

### Совместно с фреймворком ecat-config

```rust
use ecat_config::{Config, FileSource};

let mut app_config = Config::new();
app_config.load(&FileSource::new("databases.yaml")).await?;

let redis_cfg: RedisConfig = serde_json::from_value(
    app_config.get::<serde_json::Value>("redis").unwrap()
)?;
let cache = RedisCache::from_config(redis_cfg).await?;
```

### Конфигурация по необходимости

Неиспользуемые базы данных опускаются в YAML, в Rust-структуре помечаются `Option`:

```rust
#[derive(Deserialize)]
struct AppConfig {
    sql: SqlxConfig,
    redis: Option<RedisConfig>,
    clickhouse: Option<ClickhouseConfig>,
}
```

---

## Связанные документы

- [Отчёт об аудите r5](audit-report-2026-08-01-r5.md)
- [Руководство по TLS-сертификатам](tls-certificate-tutorial.md)
- [Пример файла конфигурации](../../../config/databases.example.yaml)
