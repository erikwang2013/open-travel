# TLS 인증서 설정과 인증 튜토리얼

**버전:** 2.4.2 · **날짜:** 2026-08-01

e-cat의 14개 데이터 백엔드는 모두 TLS 클라이언트 인증서 인증(mTLS)을 지원합니다. 이 튜토리얼은 인증서 생성, 설정, 그리고 모든 데이터베이스 백엔드 연결까지의 전체 과정을 다룹니다.

---

## 1. 인증서 생성

### 방법 1: ecat-tls 자동 생성 (권장)

```rust
use ecat_tls::{generate_ca, generate_server_cert, generate_client_cert};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("certs")?;

    // 1. CA 인증서 생성
    let ca = generate_ca("MyOrganization")?;
    fs::write("certs/ca.pem", &ca.cert_pem)?;
    fs::write("certs/ca-key.pem", &ca.key_pem)?;

    // 2. 서버 인증서 생성 (데이터베이스 서버에 배포)
    let server = generate_server_cert("db.internal")?;
    fs::write("certs/server.pem", &server.cert_pem)?;
    fs::write("certs/server-key.pem", &server.key_pem)?;

    // 3. 클라이언트 인증서 생성 (애플리케이션 측 사용, mTLS)
    let client = generate_client_cert("myapp")?;
    fs::write("certs/client.pem", &client.cert_pem)?;
    fs::write("certs/client-key.pem", &client.key_pem)?;

    Ok(())
}
```

### 방법 2: OpenSSL 수동 생성

```bash
mkdir -p certs && cd certs

# CA 생성
openssl req -x509 -newkey rsa:4096 \
  -keyout ca-key.pem -out ca.pem -days 3650 -nodes \
  -subj "/O=MyOrg/CN=MyOrg CA"

# 서버 인증서 생성
openssl req -new -newkey rsa:4096 \
  -keyout server-key.pem -out server.csr -nodes \
  -subj "/CN=db.internal"
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca-key.pem \
  -out server.pem -days 365

# 클라이언트 인증서 생성 (mTLS)
openssl req -new -newkey rsa:4096 \
  -keyout client-key.pem -out client.csr -nodes \
  -subj "/CN=myapp"
openssl x509 -req -in client.csr -CA ca.pem -CAkey ca-key.pem \
  -out client.pem -days 365

rm -f *.csr
```

---

## 2. TLS 설정

### 공통 TLS 필드

모든 백엔드 Config는 다음 선택 필드를 지원합니다 (`#[serde(default)]`):

| 필드 | 타입 | 설명 |
|------|------|------|
| `tls.ca_cert` | `Option<String>` | CA 인증서 PEM 경로(서버 인증서 검증) |
| `tls.client_cert` | `Option<String>` | 클라이언트 인증서 PEM 경로(mTLS) |
| `tls.client_key` | `Option<String>` | 클라이언트 개인키 PEM 경로(mTLS) |
| `tls.skip_verify` | `Option<bool>` | 인증서 검증 건너뛰기(테스트 환경 전용) |

> ⚠️ 상호 배타: `skip_verify=true`와 `ca_cert`를 동시에 설정하면 빌드 시 바로 오류가 발생합니다(`ecat-tls`가 모순 설정을 거부 — 검증 건너뛰기인데 신뢰 앵커를 설정하는 경우, 잘못된 설정으로 인증서 검증이 조용히 꺼지는 것을 방지).

### YAML 설정 예시

```yaml
# 서버 인증서만 검증
elasticsearch:
  base_url: "https://es.internal:9200"
  tls:
    ca_cert: "/etc/ecat/ca.pem"

# mTLS (양방향 인증)
clickhouse:
  base_url: "https://ch.internal:8443"
  database: "analytics"
  tls:
    ca_cert: "/etc/ecat/ca.pem"
    client_cert: "/etc/ecat/client.pem"
    client_key: "/etc/ecat/client-key.pem"

# 테스트 환경 (검증 건너뛰기)
questdb:
  base_url: "https://localhost:9000"
  tls:
    skip_verify: true
```

---

## 3. 각 백엔드 TLS 설정

### HTTP 백엔드 (9개)

Elasticsearch, OpenSearch, ClickHouse, QuestDB, InfluxDB, Neo4j, NebulaGraph, ArangoDB, IoTDB — 모두 `TlsClientConfig::build_reqwest_client()`로 TLS Client를 통일 구축합니다.

```yaml
# 모든 HTTP 백엔드는 동일한 형식 사용
backend:
  base_url: "https://host:port"
  tls:
    ca_cert: "/path/to/ca.pem"
    client_cert: "/path/to/client.pem"   # mTLS 필요
    client_key: "/path/to/client-key.pem" # mTLS 필요
```

### Redis — 자동 URL scheme 전환

```yaml
redis:
  url: "redis://cache.internal:6379"    # TLS 활성화 → 자동으로 rediss:// 전환
  tls:
    ca_cert: "/etc/ecat/ca.pem"
```

### RDBMS (Sqlx) — URL 파라미터 설정

```yaml
sql:
  url: "postgres://db.internal:5432/mydb?sslmode=require"
  tls: {}  # 예약 필드
```

| 데이터베이스 | TLS URL 파라미터 |
|--------|------------|
| PostgreSQL | `?sslmode=require` 또는 `?sslmode=verify-full` |
| MySQL | `?ssl-mode=VERIFY_CA&ssl-ca=/path/to/ca.pem` |
| TiDB | `?ssl-mode=VERIFY_IDENTITY&ssl-ca=/path/to/ca.pem` |
| SQLite | TLS 불필요 |

---

## 4. Rust 코드 로드

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

    // from_config 내부에서 tls.build_reqwest_client() 호출 — TLS 자동 적용
    let es = ElasticsearchClient::from_config(cfg.elasticsearch);
    let ch = ClickhouseClient::from_config(cfg.clickhouse);

    let results = es.search("logs", &serde_json::json!({"match_all": {}})).await?;
    Ok(())
}
```

---

## 5. 프로그래밍 방식 생성 (TLS + 인증)

```rust
use ecat_tls::TlsClientConfig;

// 수동으로 TLS 클라이언트 구축
let tls = TlsClientConfig {
    ca_cert: Some("/etc/ecat/ca.pem".into()),
    client_cert: Some("/etc/ecat/client.pem".into()),
    client_key: Some("/etc/ecat/client-key.pem".into()),
    skip_verify: None,
};
let client = tls.build_reqwest_client()?;

// 또는 with_auth + TLS 설정 사용
let es = ElasticsearchClient::with_auth(
    "https://es.internal:9200", "elastic", "secret"
);
```

---

## 6. 보안 권장 사항

1. **프로덕션 환경에서는 인증서 검증 필수** — `skip_verify` 비활성화
2. **CA 개인키 안전 보관** — 버전 관리에 포함 금지
3. **인증서 유효 기간 관리** — 만료 전 갱신 및 교체
4. **mTLS 보안 강화** — 프로덕션에서는 클라이언트 인증서 동시 설정 권장

---

## 관련 문서

- [데이터베이스 설정 튜토리얼](database-config-tutorial.md)
- [감사 보고서 r5](audit-report-2026-08-01-r5.md)
- [설정 예시 파일](../../../config/databases.example.yaml)
