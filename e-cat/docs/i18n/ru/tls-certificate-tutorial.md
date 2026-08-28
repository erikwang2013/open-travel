# Руководство по настройке и аутентификации TLS-сертификатов

**Версия:** 2.4.2 · **Дата:** 2026-08-01

Все 14 бэкендов данных e-cat полностью поддерживают TLS-аутентификацию клиентским сертификатом (mTLS). Руководство охватывает весь процесс: генерация сертификатов, настройка и подключение ко всем бэкендам баз данных.

---

## 1. Генерация сертификатов

### Способ 1: автогенерация ecat-tls (рекомендуется)

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

### Способ 2: ручная генерация OpenSSL

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

## 2. Настройка TLS

### Общие поля TLS

Все Config бэкендов поддерживают следующие опциональные поля (`#[serde(default)]`):

| Поле | Тип | Описание |
|------|------|------|
| `tls.ca_cert` | `Option<String>` | Путь к PEM-файлу CA-сертификата (проверка сертификата сервера) |
| `tls.client_cert` | `Option<String>` | Путь к PEM-файлу клиентского сертификата (mTLS) |
| `tls.client_key` | `Option<String>` | Путь к PEM-файлу приватного ключа клиента (mTLS) |
| `tls.skip_verify` | `Option<bool>` | Пропустить проверку сертификатов (только тестовое окружение) |

> ⚠️ Взаимоисключение: одновременная настройка `skip_verify=true` и `ca_cert` приводит к ошибке на этапе построения (`ecat-tls` отвергает противоречивую конфигурацию — пропуск проверки при заданном доверенном якоре, чтобы ошибка конфигурации не отключила проверку сертификатов молча).

### Пример YAML-конфигурации

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

## 3. TLS-конфигурация по бэкендам

### HTTP-бэкенды (9 шт.)

Elasticsearch, OpenSearch, ClickHouse, QuestDB, InfluxDB, Neo4j, NebulaGraph, ArangoDB, IoTDB — единообразно строят TLS Client через `TlsClientConfig::build_reqwest_client()`.

```yaml
# 所有 HTTP 后端使用相同格式
backend:
  base_url: "https://host:port"
  tls:
    ca_cert: "/path/to/ca.pem"
    client_cert: "/path/to/client.pem"   # mTLS 需要
    client_key: "/path/to/client-key.pem" # mTLS 需要
```

### Redis — автоматическое переключение scheme URL

```yaml
redis:
  url: "redis://cache.internal:6379"    # 启用 TLS → 自动切换 rediss://
  tls:
    ca_cert: "/etc/ecat/ca.pem"
```

### RDBMS (Sqlx) — настройка через параметры URL

```yaml
sql:
  url: "postgres://db.internal:5432/mydb?sslmode=require"
  tls: {}  # 保留字段
```

| База данных | Параметры TLS в URL |
|--------|------------|
| PostgreSQL | `?sslmode=require` или `?sslmode=verify-full` |
| MySQL | `?ssl-mode=VERIFY_CA&ssl-ca=/path/to/ca.pem` |
| TiDB | `?ssl-mode=VERIFY_IDENTITY&ssl-ca=/path/to/ca.pem` |
| SQLite | TLS не требуется |

---

## 4. Загрузка из кода Rust

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

## 5. Программное создание (TLS + аутентификация)

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

## 6. Рекомендации по безопасности

1. **В продакшене сертификаты обязательно проверяются** — отключайте `skip_verify`
2. **Приватный ключ CA храните в безопасности** — не включайте в систему контроля версий
3. **Управление сроком действия сертификатов** — продлевайте и ротируйте до истечения
4. **mTLS усиливает безопасность** — в продакшене рекомендуется настраивать и клиентский сертификат

---

## Связанные документы

- [Руководство по настройке баз данных](database-config-tutorial.md)
- [Отчёт об аудите r5](audit-report-2026-08-01-r5.md)
- [Пример файла конфигурации](../../../config/databases.example.yaml)
