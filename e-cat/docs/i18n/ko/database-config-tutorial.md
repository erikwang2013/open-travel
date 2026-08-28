# 데이터베이스 설정 튜토리얼

**버전:** 2.4.2 · **날짜:** 2026-08-01

e-cat의 14개 데이터 백엔드는 모두 설정 파일에서 연결 정보를 로드할 수 있으며, 코드에 하드코딩할 필요가 없습니다. `username` / `password`는 모두 선택 필드이며, 생략하면 인증을 건너뜁니다.

---

## 빠른 시작

### 1. 설정 파일 생성

예시 템플릿을 복사한 후 실제 환경에 맞게 수정합니다:

```bash
cp config/databases.example.yaml databases.yaml
```

`databases.yaml`을 편집하여 실제 연결 정보를 입력합니다:

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

### 2. 의존성 추가

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
ecat-data-sqlx = { path = "../ecat-data-sqlx" }
ecat-data-redis = { path = "../ecat-data-redis" }
ecat-data-clickhouse = { path = "../ecat-data-clickhouse" }
```

### 3. 로드하여 사용

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
    // YAML 설정 로드
    let yaml = std::fs::read_to_string("databases.yaml")?;
    let cfg: AppConfig = serde_yaml::from_str(&yaml)?;

    // 데이터베이스 클라이언트 생성 — 하드코딩된 연결 정보 없음
    let db = SqlxClient::from_config(cfg.sql).await?;
    let cache = RedisCache::from_config(cfg.redis).await?;
    let ch = ClickhouseClient::from_config(cfg.clickhouse);

    // 사용
    let rows = db.query("SELECT id, name FROM users LIMIT 10").await?;
    cache.set("health", b"ok", std::time::Duration::from_secs(30)).await?;

    Ok(())
}
```

---

## 전체 설정 참조

### 최상위 설정 구조체 정의

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

### YAML 전체 예시

`config/databases.example.yaml`을 참조하세요.

---

## 각 백엔드 Config 필드 속성 참조

### RDBMS — SqlxConfig

```yaml
sql:
  url: "postgres://host:5432/dbname"
  # username: "app_user"    # 선택
  # password: "secret"      # 선택
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `url` | `String` | sqlx 연결 문자열, SQLite/PG/MySQL/TiDB 지원 |
| `username` | `Option<String>` | 선택: URL 내장 인증(password와 함께 사용) |
| `password` | `Option<String>` | 선택: URL 내장 인증(username과 함께 사용) |

### Redis — RedisConfig

```yaml
redis:
  url: "redis://host:6379"
  # password: "auth_token"  # 선택
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `url` | `String` | Redis 연결 URL |
| `password` | `Option<String>` | 선택: Redis AUTH 비밀번호 |

### Memcached — MemcachedConfig

```yaml
memcached:
  # username: "memcache"    # 선택: 예약 필드 (현재 메모리 구현)
  # password: "secret"      # 선택: 예약 필드
  {}
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `username` | `Option<String>` | 선택: 예약 필드 |
| `password` | `Option<String>` | 선택: 예약 필드 |

현재는 메모리 구현이며, 인증 필드는 예약되어 있습니다.

### ClickHouse — ClickhouseConfig

```yaml
clickhouse:
  base_url: "http://host:8123"
  database: "default"
  # username: "default"   # 선택
  # password: "secret"    # 선택
```

| 필드 | 타입 | 기본값 | 설명 |
|------|------|--------|------|
| `base_url` | `String` | — | HTTP 인터페이스 주소 |
| `database` | `String` | `"default"` | 데이터베이스 이름 |
| `username` | `Option<String>` | `None` | 선택: HTTP Basic Auth 사용자 이름 |
| `password` | `Option<String>` | `None` | 선택: HTTP Basic Auth 비밀번호 |

### QuestDB — QuestdbConfig

```yaml
questdb:
  base_url: "http://host:9000"
  # username: "admin"     # 선택
  # password: "quest"     # 선택
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `base_url` | `String` | HTTP API 주소 |
| `username` | `Option<String>` | 선택: HTTP Basic Auth 사용자 이름 |
| `password` | `Option<String>` | 선택: HTTP Basic Auth 비밀번호 |

### Elasticsearch — ElasticsearchConfig

```yaml
elasticsearch:
  base_url: "http://host:9200"
  # username: "elastic"   # 선택
  # password: "secret"    # 선택
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `base_url` | `String` | REST API 주소 |
| `username` | `Option<String>` | 선택: HTTP Basic Auth 사용자 이름 |
| `password` | `Option<String>` | 선택: HTTP Basic Auth 비밀번호 |

### OpenSearch — OpenSearchConfig

```yaml
opensearch:
  base_url: "http://host:9200"
  # username: "admin"     # 선택
  # password: "secret"    # 선택
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `base_url` | `String` | REST API 주소 |
| `username` | `Option<String>` | 선택: HTTP Basic Auth 사용자 이름 |
| `password` | `Option<String>` | 선택: HTTP Basic Auth 비밀번호 |

### InfluxDB — InfluxConfig

```yaml
influxdb:
  base_url: "http://host:8086"
  org: "myorg"
  bucket: "mybucket"
  token: "my-token"
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `base_url` | `String` | InfluxDB 2.x API 주소 |
| `org` | `String` | 조직 이름 |
| `bucket` | `String` | 버킷 이름 |
| `token` | `String` | 인증 토큰 |

### Neo4j — Neo4jConfig

```yaml
neo4j:
  base_url: "http://host:7474"
  username: "neo4j"
  password: "secret"
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `base_url` | `String` | REST API 주소 |
| `username` | `String` | 사용자 이름 |
| `password` | `String` | 비밀번호 |

### NebulaGraph — NebulaGraphConfig

```yaml
nebulagraph:
  base_url: "http://host:19669"
  space: "my_space"
  # username: "root"      # 선택
  # password: "nebula"    # 선택
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `base_url` | `String` | API 주소 |
| `space` | `String` | 그래프 스페이스 이름 |
| `username` | `Option<String>` | 선택: HTTP Basic Auth 사용자 이름 |
| `password` | `Option<String>` | 선택: HTTP Basic Auth 비밀번호 |

### ArangoDB — ArangoConfig

```yaml
arangodb:
  base_url: "http://host:8529"
  db: "mydb"
  username: "root"
  password: "secret"
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `base_url` | `String` | API 주소 |
| `db` | `String` | 데이터베이스 이름 |
| `username` | `String` | 사용자 이름 |
| `password` | `String` | 비밀번호 |

### IoTDB — IotdbConfig

```yaml
iotdb:
  base_url: "http://host:18080"
  username: "root"
  password: "root"
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `base_url` | `String` | REST API 주소 |
| `username` | `String` | 사용자 이름 |
| `password` | `String` | 비밀번호 |

---

## 프로그래밍 방식 생성

### 인증 없이

```rust
let es = ElasticsearchClient::new("http://localhost:9200");
let ch = ClickhouseClient::new("http://localhost:8123", "default");
```

### 인증 포함

```rust
let es = ElasticsearchClient::with_auth("http://es:9200", "elastic", "secret");
let ch = ClickhouseClient::with_auth("http://ch:8123", "default", "admin", "pass");
let qdb = QuestdbClient::with_auth("http://qdb:9000", "admin", "quest");
let ng = NebulaGraphClient::with_auth("http://ng:19669", "space1", "root", "nebula");
```

---

---

## TLS 인증서 설정

모든 데이터 백엔드는 선택적 TLS 클라이언트 인증(`tls` 필드)을 지원합니다.

### 설정 예시

```yaml
clickhouse:
  base_url: "https://ch.internal:8443"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"
    # skip_verify: true  # 테스트 환경 전용
```

### 인증서 자동 생성 (ecat-tls)

```rust
use ecat_tls::{generate_ca, generate_server_cert, generate_client_cert};

// 1. CA 생성
let ca = generate_ca("MyOrg")?;
std::fs::write("ca.pem", &ca.cert_pem)?;
std::fs::write("ca-key.pem", &ca.key_pem)?;

// 2. 서버 인증서 생성
let srv = generate_server_cert("db.example.com")?;
std::fs::write("server.pem", &srv.cert_pem)?;
std::fs::write("server-key.pem", &srv.key_pem)?;

// 3. 클라이언트 인증서 생성 (mTLS)
let client = generate_client_cert("myapp")?;
std::fs::write("client.pem", &client.cert_pem)?;
std::fs::write("client-key.pem", &client.key_pem)?;
```

### 수동 생성 (OpenSSL)

```bash
# CA
openssl req -x509 -newkey rsa:4096 -keyout ca-key.pem -out ca.pem -days 3650 -nodes

# 서버 인증서
openssl req -new -newkey rsa:4096 -keyout server-key.pem -out server.csr -nodes -subj "/CN=db.example.com"
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca-key.pem -out server.pem -days 365

# 클라이언트 인증서 (mTLS)
openssl req -new -newkey rsa:4096 -keyout client-key.pem -out client.csr -nodes -subj "/CN=myapp"
openssl x509 -req -in client.csr -CA ca.pem -CAkey ca-key.pem -out client.pem -days 365
```

### TLS 필드 설명

| 필드 | 타입 | 설명 |
|------|------|------|
| `ca_cert` | `Option<String>` | CA 인증서 PEM 경로(서버 검증) |
| `client_cert` | `Option<String>` | 클라이언트 인증서 PEM 경로(mTLS) |
| `client_key` | `Option<String>` | 클라이언트 개인키 PEM 경로(mTLS) |
| `skip_verify` | `Option<bool>` | 인증서 검증 건너뛰기(테스트 전용) |

---

## 고급 사용법

### 환경 변수 오버라이드

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

### ecat-config 프레임워크와 결합

```rust
use ecat_config::{Config, FileSource};

let mut app_config = Config::new();
app_config.load(&FileSource::new("databases.yaml")).await?;

let redis_cfg: RedisConfig = serde_json::from_value(
    app_config.get::<serde_json::Value>("redis").unwrap()
)?;
let cache = RedisCache::from_config(redis_cfg).await?;
```

### 필요에 따른 설정

사용하지 않는 데이터베이스는 YAML에서 생략하고, Rust 구조체는 `Option`으로 표시합니다:

```rust
#[derive(Deserialize)]
struct AppConfig {
    sql: SqlxConfig,
    redis: Option<RedisConfig>,
    clickhouse: Option<ClickhouseConfig>,
}
```

---

## 관련 문서

- [감사 보고서 r5](audit-report-2026-08-01-r5.md)
- [TLS 인증서 인증 튜토리얼](tls-certificate-tutorial.md)
- [설정 예시 파일](../../../config/databases.example.yaml)
