<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# E-CAT ऑडिट रिपोर्ट — r5

**तिथि**: 2026-08-01  
**शाखा**: main  
**संस्करण**: 2.1.7  
**Crate संख्या**: 47 (workspace members)
**स्थिति**: ✅ सभी मरम्मत योग्य समस्याएँ हल + डेटा बैकएंड का पूर्ण कॉन्फ़िगरेशन फ़ाइल समर्थन

---

## 0. मरम्मत रिकॉर्ड (2026-08-01)

| # | समस्या | फ़ाइल | मरम्मत |
|---|------|------|------|
| 1 | अप्रयुक्त import `axum::routing::get` | `ecat-versioning/src/lib.rs:3` | शीर्ष-स्तरीय import हटाया, `#[cfg(test)]` में ले गए |
| 2 | अप्रयुक्त चर `version` | `ecat-versioning/src/lib.rs:61` | `_version` में बदला |
| 3 | dead code `extract_version` | `ecat-versioning/src/lib.rs:68` | `pub fn` में बदला |
| 4 | `useless_format!` | `ecat-versioning/src/lib.rs:62` | सीधे `"/api"` में बदला |
| 5 | `unnecessary_to_owned` | `ecat-data-questdb/src/lib.rs:39` | `"true".to_string()` → `"true"` |
| 6 | त्रुटि संदेश निगला गया | `ecat-data-questdb/src/lib.rs:30` | `unwrap_or_default()` → `unwrap_or_else(...)` |
| 7 | `derivable_impls` | `ecat-client/src/lib.rs:249` | `GrpcClientBuilder` में `#[derive(Default)]` उपयोग |
| 8 | `manual_is_multiple_of` | `ecat-config/src/encrypted.rs:60` | `s.len() % 2 != 0` → `!s.len().is_multiple_of(2)` |
| 9 | `collapsible_if` | `ecat-registry-etcd/src/lib.rs:92` | नेस्टेड `if let` मर्ज |
| 10 | `collapsible_if` | `ecat-data-clickhouse/src/lib.rs:56` | नेस्टेड `if let` मर्ज |
| 11 | `type_complexity` | `ecat-data-memcached/src/lib.rs:9` | `type CacheEntry` उपनाम जोड़ा |

**अंतिम परिणाम**: `cargo build` शून्य warning, `cargo clippy --all-targets` शून्य warning, `cargo test` सभी पास (0 विफल)।

### 12 ─ डेटा बैकएंड का पूर्ण कॉन्फ़िगरेशन फ़ाइल समर्थन (Cargo + lib.rs)

12 डेटा बैकएंड crates के लिए `Config` संरचना (`#[derive(Deserialize)]`) और `from_config()` कंस्ट्रक्टर जोड़ा, JSON/YAML कॉन्फ़िगरेशन फ़ाइलों से कनेक्शन जानकारी लोड करना समर्थन करता है, हार्डकोडिंग की आवश्यकता नहीं।

| Crate | Config संरचना | फ़ील्ड |
|-------|--------------|------|
| `ecat-data-redis` | `RedisConfig` | `url` |
| `ecat-data-sqlx` | `SqlxConfig` | `url` |
| `ecat-data-clickhouse` | `ClickhouseConfig` | `base_url`, `database` (डिफ़ॉल्ट "default") |
| `ecat-data-questdb` | `QuestdbConfig` | `base_url` |
| `ecat-data-elasticsearch` | `ElasticsearchConfig` | `base_url` |
| `ecat-data-opensearch` | `OpenSearchConfig` | `base_url` |
| `ecat-data-influxdb` | `InfluxConfig` | `base_url`, `org`, `bucket`, `token` |
| `ecat-data-memcached` | `MemcachedConfig` | (खाली — मेमोरी कार्यान्वयन) |
| `ecat-data-neo4j` | `Neo4jConfig` | `base_url`, `username`, `password` |
| `ecat-data-nebulagraph` | `NebulaGraphConfig` | `base_url`, `space` |
| `ecat-data-arangodb` | `ArangoConfig` | `base_url`, `db`, `username`, `password` |
| `ecat-data-iotdb` | `IotdbConfig` | `base_url`, `username`, `password` |

**उपयोग उदाहरण**:
```rust
// YAML कॉन्फ़िगरेशन फ़ाइल से लोड करें
let cfg: ClickhouseConfig = serde_json::from_str(r#"{"base_url":"http://localhost:8123"}"#)?;
let client = ClickhouseClient::from_config(cfg);
```

### 13 ─ HTTP बैकएंड में वैकल्पिक प्रमाणीकरण समर्थन (5 crates)

5 शुद्ध HTTP बैकएंड के लिए वैकल्पिक `username` / `password` फ़ील्ड और `with_auth()` कंस्ट्रक्टर जोड़ा। सभी `Option<String>` (`#[serde(default)]`) हैं, कॉन्फ़िगर न करने पर कोई प्रमाणीकरण नहीं।

| Crate | नए Config फ़ील्ड | नया कंस्ट्रक्टर |
|-------|-----------------|-------------|
| `ecat-data-elasticsearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-opensearch` | `username?`, `password?` | `with_auth()` |
| `ecat-data-clickhouse` | `username?`, `password?` | `with_auth()` |
| `ecat-data-questdb` | `username?`, `password?` | `with_auth()` |
| `ecat-data-nebulagraph` | `username?`, `password?` | `with_auth()` |

सभी HTTP अनुरोध `apply_auth()` सहायक विधि से स्वचालित रूप से Basic Auth जोड़ते हैं (केवल जब दोनों None न हों)।

### 14 ─ Redis / RDBMS / Memcached में वैकल्पिक प्रमाणीकरण फ़ील्ड (3 crates)

| Crate | नए Config फ़ील्ड | नया कंस्ट्रक्टर | प्रमाणीकरण विधि |
|-------|-----------------|-------------|----------|
| `ecat-data-redis` | `password?` | `connect_with_password()` | URL में एम्बेडेड पासवर्ड |
| `ecat-data-sqlx` | `username?`, `password?` | `connect_with_auth()` | URL में एम्बेडेड प्रमाणीकरण |
| `ecat-data-memcached` | `username?`, `password?` | `with_auth()` | फ़ील्ड रखे गए (मेमोरी कार्यान्वयन) |

Sqlx SQLite / PostgreSQL / MySQL / TiDB चार RDBMS कवर करता है। Auth फ़ील्ड `replacen("://", "://user:pass@")` से कनेक्शन URL में एम्बेड होते हैं, केवल URL में `@` न होने पर प्रभावी।

### 15 ─ TLS प्रमाणपत्र प्रमाणीकरण समर्थन + ecat-tls crate (सभी 12 बैकएंड)

नया `ecat-tls` crate, प्रदान करता है:
- `TlsClientConfig` — वैकल्पिक TLS कॉन्फ़िगरेशन (ca_cert, client_cert, client_key, skip_verify)
- `generate_ca()` — सेल्फ-साइन्ड CA प्रमाणपत्र जनरेशन
- `generate_server_cert()` — सर्वर प्रमाणपत्र जनरेशन
- `generate_client_cert()` — क्लाइंट प्रमाणपत्र जनरेशन (mTLS)

सभी 12 डेटा बैकएंड Config में `#[serde(default)] tls: Option<TlsClientConfig>` फ़ील्ड जोड़ा।

| बैकएंड प्रकार | TLS विधि |
|----------|----------|
| 9 HTTP बैकएंड | `tls.build_reqwest_client()` से TLS reqwest Client निर्माण |
| Redis | URL scheme स्विच `redis://` → `rediss://` |
| Sqlx | फ़ील्ड रखा गया (TLS URL पैरामीटर `?sslmode=require` से) |
| Memcached | फ़ील्ड रखा गया (नेटवर्क कार्यान्वयन के लिए आरक्षित) |

---

## 1. सिंहावलोकन

| आइटम | स्थिति | विवरण |
|------|------|------|
| `cargo build` | ✅ पास | 3 कंपाइलर warnings, 19.85s |
| `cargo test` | ✅ पास | ~137 यूनिट टेस्ट सभी पास, 0 विफल, 1 ignored |
| `cargo clippy` | ⚠️ warning सहित | 3 crates में कुल 5 lint warnings |
| `cargo fmt` | ✅ पास | कोई फ़ॉर्मेटिंग समस्या नहीं |
| `cargo audit` | ❌ स्थापित नहीं | ज्ञात CVE स्कैन असंभव |

---

## 2. कंपाइलर Warnings (मरम्मत आवश्यक)

### 2.1 ecat-versioning (3 warnings)

**फ़ाइल**: `ecat-versioning/src/lib.rs`

| # | Warning | पंक्ति | गंभीरता |
|---|---------|------|----------|
| 1 | `unused import: axum::routing::get` | 3 | कम |
| 2 | `unused variable: version` | 61 | कम |
| 3 | `function extract_version is never used` | 68 | कम |

**सुझाव**: अप्रयुक्त import हटाएँ, `version` को `_version` में बदलें, `extract_version` को `pub` बनाएँ या `#[allow(dead_code)]` चिह्नित करें।

### 2.2 ecat-data-questdb (1 clippy warning)

**फ़ाइल**: `ecat-data-questdb/src/lib.rs:39`

```rust
// वर्तमान:
.query(&[("query", sql), ("count", &"true".to_string())])

// इसे बदलकर:
.query(&[("query", sql), ("count", &"true")])
```

### 2.3 ecat-client (1 clippy warning)

**फ़ाइल**: `ecat-client/src/lib.rs:249`

`GrpcClientBuilder` ने मैन्युअल रूप से `Default` लागू किया है, सीधे `#[derive(Default)]` से बदला जा सकता है।

---

## 3. Clippy Lint Warnings सारांश

| Crate | Warning | प्रकार |
|-------|---------|------|
| ecat-versioning | `useless_format!` — `"/api".to_string()` उपयोग करें | प्रदर्शन |
| ecat-versioning | unused import / dead code | सफाई |
| ecat-data-questdb | `unnecessary_to_owned` | प्रदर्शन |
| ecat-client | `derivable_impls` — derive Default उपयोग | सरलीकरण |

---

## 4. टेस्ट कवरेज विश्लेषण

### 4.1 आँकड़े

| मीट्रिक | मान |
|------|------|
| यूनिट टेस्ट कुल | ~137 |
| विफल | 0 |
| Ignored | 1 |
| टेस्ट वाले crates | ~24 / 48 |
| **0 टेस्ट वाले crates** | **~24 / 48 (50%)** |

### 4.2 टेस्ट की कमी वाले Crates (0 या केवल कंस्ट्रक्शन टेस्ट)

निम्नलिखित crates में टेस्ट कमज़ोर:

- ecat-data-arangodb, ecat-data-clickhouse, ecat-data-elasticsearch
- ecat-data-influxdb, ecat-data-iotdb, ecat-data-nebulagraph
- ecat-data-neo4j, ecat-data-opensearch, ecat-data-questdb
- ecat-data-redis, ecat-data-sqlx, ecat-data-memcached
- ecat-mq, ecat-mq-kafka, ecat-graphql, ecat-openapi
- ecat-transport, ecat-transport-grpc, ecat-transport-http
- ecat-transport-ws, ecat-tracing, ecat-logging
- ecat-middleware, ecat-registry-consul, ecat-registry-etcd

### 4.3 Doc-tests

सभी **48 crates के doc-tests 0 हैं**। कोड में कोई `/// ````rust` दस्तावेज़ उदाहरण नहीं।

---

## 5. निर्भरता समस्याएँ

### 5.1 ⚠️ yaml_serde बनाम serde_yaml (मध्यम जोखिम)

**फ़ाइल**: `ecat-config/Cargo.toml:9`

```toml
yaml_serde = "0.10"
```

Rust इकोसिस्टम की मानक YAML लाइब्रेरी `serde_yaml` (नवीनतम `0.9.34+`) है, जबकि `yaml_serde` एक **अलग और कम रखरखाव वाला crate** है।

**सुझाव**: पुष्टि करें कि `yaml_serde` इच्छित निर्भरता है या नहीं। यदि इरादा `serde_yaml` था, तो बदलें।

### 5.2 cargo-audit की कमी

`cargo audit` स्थापित नहीं है। `cargo install cargo-audit` करके CI में जोड़ने का सुझाव।

### 5.3 description फ़ील्ड की कमी

`[workspace.package]` में `description` नहीं है, सभी उप-crates ने भी description परिभाषित नहीं किया।

---

## 6. कोड गुणवत्ता समस्याएँ

### 6.1 प्रोडक्शन कोड में unwrap/expect

| फ़ाइल | पंक्ति | कॉल | जोखिम |
|------|------|------|------|
| `ecat-client/src/lib.rs` | 28 | `.expect("StaticResolver poisoned")` | कम — उचित |
| `ecat/src/signal.rs` | 8 | `.expect("failed to install SIGTERM handler")` | मध्यम — स्टार्टअप पर panic |
| `ecat-protos/build.rs` | 5 | `.unwrap()` | कम — build script |

### 6.2 ecat-versioning का extract_version

`extract_version` फ़ंक्शन (पंक्ति 68) Accept header से संस्करण संख्या निकालने का कार्यान्वयन करता है, लेकिन `build_header_router()` द्वारा कॉल नहीं किया जाता।

### 6.3 ecat-data-questdb त्रुटि प्रबंधन

```rust
// पंक्ति 30: नेटवर्क प्रतिक्रिया बॉडी पढ़ते समय unwrap_or_default उपयोग
Err(RdbmsError::Database(resp.text().await.unwrap_or_default()))
```

`resp.text()` विफल होने पर त्रुटि संदेश चुपचाप निगल जाता है। `unwrap_or_else(|e| format!("questdb parse: {e}"))` में बदलने का सुझाव।

---

## 7. आर्किटेक्चर मूल्यांकन

### लाभ

- 48 crates की ज़िम्मेदारी पृथक्करण स्पष्ट
- workspace एकीकृत संस्करण `version.workspace = true`
- निर्भरताएँ संक्षिप्त, कोई भारी फ्रेमवर्क नहीं
- कोई TODO/FIXME/HACK नहीं

### सुधार की आवश्यकता

| समस्या | प्राथमिकता |
|------|--------|
| 50% crates में कोई टेस्ट नहीं | उच्च |
| yaml_serde बनाम serde_yaml भ्रम | मध्यम |
| cargo-audit की कमी | मध्यम |
| ecat-versioning dead code | कम |
| कोई doc-tests नहीं | कम |

---

## 8. सुरक्षा सिंहावलोकन

| जाँच आइटम | परिणाम |
|--------|------|
| हार्डकोडेड कुंजियाँ | नहीं मिलीं |
| .env फ़ाइल लीक | नहीं मिला |
| खतरनाक unwrap (प्रोडक्शन कोड) | 2 स्थान (signal.rs, client.rs) |
| CVE स्कैन | निष्पादित नहीं (cargo-audit स्थापना आवश्यक) |

---

## 9. कार्य योजना

### P0 — तुरंत मरम्मत
1. ecat-versioning के 3 कंपाइलर warnings साफ़ करें
2. ecat-data-questdb clippy मरम्मत
3. ecat-client derivable_impls मरम्मत

### P1 — अल्पकालिक
4. निर्भरता भेद्यता स्कैन के लिए `cargo-audit` स्थापित करें
5. `yaml_serde` बनाम `serde_yaml` चयन की पुष्टि करें
6. कोर crates के लिए doc-tests पूरा करें

### P2 — मध्यकालिक
7. transport/data/security crates के लिए टेस्ट पूरा करें
8. सभी crates के लिए `description` फ़ील्ड जोड़ें
9. `extract_version` एकीकृत करें या हटाएँ

### P3 — दीर्घकालिक
10. CI स्थापित करें: build → test → clippy → audit → coverage

---

*रिपोर्ट 2026-08-01 को जनरेट हुई। टूलचेन: cargo 1.92.0, rustc 1.92.0, clippy 1.92.0*
