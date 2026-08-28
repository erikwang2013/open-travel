<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat फ्रेमवर्क ऑडिट रिपोर्ट — 2026-08-01

**ऑडिट तिथि**: 2026-08-01
**ऑडिट दायरा**: सभी 18 उप-crates (workspace)
**टूलचेन**: stable (rustfmt, clippy)
**परीक्षण परिणाम**: 66 टेस्ट सभी पास | 0 विफल | 0 अनदेखा

---

## 1. समग्र मूल्यांकन

| आयाम | स्कोर | विवरण |
|------|------|------|
| कंपाइल | ✅ पास | `cargo check` कोई त्रुटि नहीं, केवल 1 warning |
| Lint | ✅ पास | `cargo clippy --all-features` शून्य चेतावनी |
| टेस्ट | ✅ 66/66 | सभी टेस्ट पास |
| टेस्ट कवरेज | ⚠️ अपर्याप्त | 7 crates में कोई टेस्ट नहीं |
| फीचर पूर्णता | ⚠️ अधिक stub | ProtoCodec、Transaction、CLI new जैसे फीचर लागू नहीं |
| कोड गुणवत्ता | ⚠️ सामान्य | संरचना स्पष्ट, लेकिन कई डिज़ाइन समस्याएँ |

---

## 2. कंपाइल और कॉन्फ़िगरेशन समस्याएँ

### 2.1 [WARNING] अप्रयुक्त manifest key

- **फ़ाइल**: `/Cargo.toml:25`
- **समस्या**: `workspace.package.name = "e-cat"` — यह फ़ील्ड workspace स्तर पर अर्थहीन है, हर कंपाइल पर warning उत्पन्न करता है
- **मरम्मत**: यह पंक्ति हटाएँ, या प्रोजेक्ट नाम समझाने वाली टिप्पणी में बदलें

### 2.2 [INFO] Rust edition असंगतता

- **workspace**: `edition = "2026"`
- **उप-crates**: `ecat-security/Cargo.toml` और `ecat-config/Cargo.toml` में `edition = "2021"` उपयोग
- **विवरण**: workspace 2026 edition घोषित करता है लेकिन कुछ उप-crates 2021 में ओवरराइड करते हैं। कंपाइल पास होता है, लेकिन 2026 edition वर्तमान में Rust आधिकारिक रूप से जारी स्थिर edition नहीं है। यदि जानबूझकर किया गया है, तो toolchain कॉन्फ़िगरेशन सही सुनिश्चित करें
- **सुझाव**: पुष्टि करें कि toolchain 2026 edition समर्थन करता है, या 2024/2021 में एकीकृत करें

---

## 3. फीचर की कमी / Stub कार्यान्वयन

### 3.1 [गंभीर] ProtoCodec पूरी तरह अनुपयोगी

- **फ़ाइल**: `ecat-encoding/src/proto.rs:8-10`
- **समस्या**: `encode()` और `decode()` हमेशा त्रुटि लौटाते हैं, protobuf codec पूरी तरह stub है
- **प्रभाव**: protobuf एन्कोडिंग का उपयोग करने वाली कोई भी कॉल रनटाइम पर विफल होगी
- **सुझाव**: prost::Message trait बाइंडिंग लागू करें, या वास्तविक कार्यक्षमता सक्षम करने के लिए `prost` feature flag प्रदान करें

### 3.2 [मध्यम] ecat-data-sqlx ट्रांज़ैक्शन लागू नहीं

- **फ़ाइल**: `ecat-data-sqlx/src/lib.rs:89-93`
- **समस्या**: `transaction()` विधि हार्डकोडेड `"transactions not yet implemented"` त्रुटि लौटाती है
- **सुझाव**: `pool.begin()` लागू करें और लपेटा गया Transaction लौटाएँ

### 3.3 [मध्यम] HttpServer.stop() और GrpcServer.stop() नो-ऑप हैं

- **फ़ाइलें**:
  - `ecat-transport-http/src/lib.rs:34-36`
  - `ecat-transport-grpc/src/lib.rs:33-35`
- **समस्या**: `stop()` विधि में सर्वर रोकने का वास्तविक तर्क नहीं है। `axum::serve()` और `tonic::Server::serve()` दोनों में shutdown सिग्नल प्राप्त करने की कोई व्यवस्था नहीं
- **प्रभाव**: `App.run()` कॉल करने के बाद, `wait_for_shutdown` ट्रिगर होने पर भी सर्वर चलता रहता है; सुचारू बंद असंभव
- **सुझाव**: `axum::serve(listener, router).with_graceful_shutdown(shutdown_signal)` और `tonic::Server::serve_with_shutdown()` उपयोग करें

### 3.4 [मध्यम] CLI `new` कमांड खोखला है

- **फ़ाइल**: `ecat-cli/src/main.rs:61-67`
- **समस्या**: `new` कमांड केवल संदेश प्रिंट करता है, प्रोजेक्ट टेम्पलेट फ़ाइलें वास्तव में नहीं बनाता
- **सुझाव**: टेम्पलेट जनरेशन तर्क लागू करें, या TODO के रूप में चिह्नित करें

### 3.5 [कम] ecat-data परत में कोई कार्यान्वयन नहीं

- **फ़ाइलें**: `ecat-data/src/{cache,graph,rdbms,search,tsdb}.rs`
- **समस्या**: सभी डेटा एक्सेस इंटरफ़ेस केवल trait परिभाषाएँ हैं, कोई कार्यान्वयन नहीं (`ecat-data-sqlx` द्वारा RdbmsClient का एक कार्यान्वयन प्रदान किया गया है)
- **सुझाव**: README में प्रत्येक trait की कार्यान्वयन स्थिति बताएँ

---

## 4. अपर्याप्त टेस्ट कवरेज

### 4.1 [मध्यम] शून्य टेस्ट कवरेज वाले crates (7)

| Crate | स्रोत फ़ाइलें | विवरण |
|-------|--------|------|
| `ecat-data` | 5 स्रोत फ़ाइलें | शुद्ध trait परिभाषाएँ, कोई टेस्ट नहीं |
| `ecat-data-sqlx` | 1 स्रोत फ़ाइल | SQLx कार्यान्वयन, कोई डेटाबेस इंटीग्रेशन टेस्ट नहीं |
| `ecat-middleware` | 4 स्रोत फ़ाइलें | Logging/Recovery/Timeout/Tracing layer में कोई टेस्ट नहीं |
| `ecat-protos` | 1 स्रोत फ़ाइल | जनरेटेड protobuf कोड, कोई टेस्ट नहीं |
| `ecat-transport-grpc` | 1 स्रोत फ़ाइल | gRPC सर्वर, कोई टेस्ट नहीं |
| `ecat-transport-http` | 1 स्रोत फ़ाइल | HTTP सर्वर, कोई टेस्ट नहीं |
| `ecat-cli` | 1 स्रोत फ़ाइल | CLI प्रवेश बिंदु, कोई टेस्ट नहीं |

**सुझाव**:
- `ecat-middleware`: `tower-test` का उपयोग करके प्रत्येक layer के लिए यूनिट टेस्ट लिखें
- `ecat-transport-http`: `axum::test` का उपयोग करके HTTP सर्वर इंटीग्रेशन टेस्ट लिखें
- `ecat-data-sqlx`: `sqlx::SqlitePool` (in-memory) का उपयोग करके डेटाबेस इंटीग्रेशन टेस्ट लिखें

---

## 5. कोड गुणवत्ता और डिज़ाइन समस्याएँ

### 5.1 [गंभीर] SecurityLayer हमलों का पता लगाता है लेकिन रोकता नहीं

- **फ़ाइल**: `ecat-security/src/lib.rs:100-125`
- **समस्या**: `SecurityService::call()` अनुरोध डेटा स्कैन करता है और चेतावनी लॉग करता है, लेकिन अनुरोध को हमेशा आंतरिक सेवा को अग्रेषित करता है। SQL इंजेक्शन और XSS हमलों का पता चलने पर भी अनुरोध सामान्य रूप से संसाधित होता है
- **मरम्मत**: हमले का पता चलने पर `403 Forbidden` या `400 Bad Request` लौटाएँ

```rust
// वर्तमान: हमेशा अग्रेषित
let fut = self.inner.call(req);
Box::pin(fut)

// इसे बदलकर: उच्च जोखिम वाले हमले का पता चलने पर अस्वीकार करें
if results.iter().any(|r| r.severity >= Severity::High) {
    // 403 प्रतिक्रिया लौटाएँ
}
```

### 5.2 [मध्यम] App::run() JoinHandle एकत्र नहीं करता

- **फ़ाइल**: `ecat/src/lib.rs:33-40`
- **समस्या**: `tokio::spawn` द्वारा लौटाया गया `JoinHandle` छोड़ दिया जाता है, server panic का पता लगाना या सुचारू बंद का इंतज़ार करना असंभव
- **सुझाव**: JoinHandles को Vec में एकत्र करें, shutdown पर सभी servers के बंद होने का इंतज़ार करें

### 5.3 [मध्यम] Registration::Drop रनटाइम छोड़ने पर चुपचाप विफल होता है

- **फ़ाइल**: `ecat-registry/src/lib.rs:46-56`
- **समस्या**: `Drop` में `tokio::spawn()` कॉल — यदि tokio runtime पहले ही drop हो चुका है, तो कार्य चुपचाप छोड़ दिया जाएगा
- **सुझाव**: `tokio::task::block_in_place` + `Handle::block_on` उपयोग करें या स्पष्ट `unregister` विधि में बदलें

### 5.4 [मध्यम] ecat-data-sqlx क्वेरी पंक्ति प्रकार मैपिंग अविश्वसनीय

- **फ़ाइल**: `ecat-data-sqlx/src/lib.rs:55-78`
- **समस्या**: डेटाबेस कॉलम मान `i64 → f64 → String → Null` क्रम में प्रयास किए जाते हैं, कुछ डेटाबेस ड्राइवर पूर्णांक मानों को असंगत प्रकार के रूप में रिपोर्ट कर सकते हैं जिससे गलत रूपांतरण होता है (जैसे PostgreSQL INTEGER को `i32` के रूप में लौटाता है, `i64` नहीं)
- **सुझाव**: रूपांतरण रणनीति तय करने से पहले SQLx के `ValueRef` / `TypeInfo` से कॉलम के वास्तविक डेटाबेस प्रकार की जाँच करें

### 5.5 [कम] Metadata संदर्भ में सेट विधियों की कमी

- **फ़ाइल**: `ecat-transport/src/context.rs:18-20`
- **समस्या**: `Context` `Metadata` को `RwLock` में लपेटता है और केवल `trace_id()` रीड विधि उजागर करता है, trace_id या अन्य मेटाडेटा सेट करना असंभव
- **सुझाव**: `Context` के लिए `set_trace_id()` जैसी राइट विधियाँ जोड़ें

### 5.6 [कम] ecat-config FileSource गैर-ऑब्जेक्ट YAML/JSON चुपचाप छोड़ देता है

- **फ़ाइल**: `ecat-config/src/file.rs:30`
- **समस्या**: `unwrap_or_default()` गैर-ऑब्जेक्ट YAML (जैसे ऐरे `[1,2,3]` या स्केलर मान) को खाली HashMap में बदल देता है, उपयोगकर्ता को पता नहीं चलता कि कॉन्फ़िगरेशन लोड क्यों नहीं हुआ
- **सुझाव**: `ConfigError::Other("expected object")` लौटाएँ

---

## 6. क्रॉस-प्लेटफ़ॉर्म संगतता समस्याएँ

### 6.1 [मध्यम] Windows पर wait_for_shutdown में Ctrl+C समर्थन नहीं

- **फ़ाइल**: `ecat/src/signal.rs:13-14`
- **समस्या**: गैर-Unix प्लेटफ़ॉर्म पर `terminate` को `std::future::pending::<()>()` पर सेट किया गया है, जो कभी resolve नहीं होता। Windows पर Ctrl+C SIGINT सिग्नल में बदल जाता है लेकिन यह अनिश्चित है कि `tokio::signal::ctrl_c()` Windows पर कारगर है या नहीं
- **सुझाव**: Windows पर भी `tokio::signal::ctrl_c()` उपयोग करें (tokio दस्तावेज़ कहता है कि यह Windows समर्थन करता है), या `tokio::signal::windows::ctrl_*` श्रृंखला उपयोग करें

---

## 7. आर्किटेक्चर और अनुकूलन सुझाव

### 7.1 [अनुकूलन] ecat-data-sqlx query() में कॉलम नामों का बार-बार clone

- **फ़ाइल**: `ecat-data-sqlx/src/lib.rs:48-83`
- **समस्या**: हर पंक्ति डेटा पर columns वेक्टर एक बार clone होता है। 1000 पंक्तियाँ लौटाने वाली क्वेरी के लिए, columns 1000 बार clone होता है
- **सुझाव**: columns को `Arc<Vec<String>>` में लपेटें, सभी पंक्तियाँ संदर्भ साझा करें

### 7.2 [अनुकूलन] MemoryRegistry::discover() में अनावश्यक clone

- **फ़ाइल**: `ecat-registry/src/memory.rs:44-52`
- **समस्या**: `.cloned()` सभी मिलान वाले ServiceInfo को clone करता है। यदि discover उच्च आवृत्ति पर कॉल होता है, तो बहुत सारे मेमोरी आवंटन होंगे
- **सुझाव**: यदि कॉलर को ownership की आवश्यकता नहीं है, तो `Vec<&ServiceInfo>` लौटाने या `Arc<ServiceInfo>` में लपेटने पर विचार करें

### 7.3 [आर्किटेक्चर] Re-export संरचना सुझाव

`ecat-transport` crate में `Request` और `Response` के जेनेरिक पैरामीटर `T` का डिफ़ॉल्ट `()` है, उपयोग करते समय आमतौर पर विशिष्ट प्रकार निर्दिष्ट करना पड़ता है। प्रकार उपनाम प्रदान करने का सुझाव:
```rust
pub type HttpRequest = Request<hyper::Body>;
pub type JsonRequest<T> = Request<T>;
```

### 7.4 [सुरक्षा] रेट लिमिटिंग मिडलवेयर की कमी

वर्तमान middleware परत में रेट लिमिटिंग (Rate Limiting) फीचर की कमी है। DoS हमलों को रोकने के लिए `RateLimitLayer` जोड़ने का सुझाव।

---

## 8. टेस्ट आँकड़े

```
टेस्ट सिंहावलोकन:
  कुल: 66 tests
  पास: 66
  विफल: 0
  अनदेखा: 0

crate अनुसार वितरण:
  ecat:              4 tests ✅
  ecat-config:       9 tests ✅
  ecat-data:         0 tests ⚠️
  ecat-data-sqlx:    0 tests ⚠️
  ecat-encoding:    15 tests ✅
  ecat-errors:       4 tests ✅
  ecat-logging:      1 test  ✅
  ecat-metadata:     9 tests ✅
  ecat-metrics:      2 tests ✅
  ecat-middleware:   0 tests ⚠️
  ecat-protos:       0 tests ⚠️
  ecat-registry:     5 tests ✅
  ecat-security:     6 tests ✅
  ecat-transport:   11 tests ✅
  ecat-transport-grpc: 0 tests ⚠️
  ecat-transport-http: 0 tests ⚠️
  ecat-cli:          0 tests ⚠️
```

---

## 9. समस्या प्राथमिकता सारांश

| # | गंभीरता | समस्या | फ़ाइल |
|---|--------|------|------|
| 1 | 🔴 गंभीर | SecurityLayer हमलों का पता लगाता है लेकिन रोकता नहीं | `ecat-security/src/lib.rs` |
| 2 | 🔴 गंभीर | ProtoCodec पूरी तरह अनुपयोगी | `ecat-encoding/src/proto.rs` |
| 3 | 🟠 मध्यम | HttpServer/GrpcServer stop() नो-ऑप है | `ecat-transport-http/src/lib.rs`, `ecat-transport-grpc/src/lib.rs` |
| 4 | 🟠 मध्यम | 7 crates में शून्य टेस्ट कवरेज | 4.1 तालिका देखें |
| 5 | 🟠 मध्यम | App::run() JoinHandle एकत्र नहीं करता | `ecat/src/lib.rs` |
| 6 | 🟠 मध्यम | Transaction लागू नहीं | `ecat-data-sqlx/src/lib.rs` |
| 7 | 🟠 मध्यम | Registration::Drop tokio बंद होने पर अप्रभावी | `ecat-registry/src/lib.rs` |
| 8 | 🟠 मध्यम | ecat-data-sqlx कॉलम प्रकार मैपिंग अविश्वसनीय | `ecat-data-sqlx/src/lib.rs` |
| 9 | 🟠 मध्यम | CLI new कमांड खोखला है | `ecat-cli/src/main.rs` |
| 10 | 🟡 कम | अप्रयुक्त manifest key warning | `/Cargo.toml` |
| 11 | 🟡 कम | Edition असंगतता (2026 vs 2021) | `/Cargo.toml`, `ecat-security/Cargo.toml`, `ecat-config/Cargo.toml` |
| 12 | 🟡 कम | FileSource गैर-ऑब्जेक्ट मान चुपचाप छोड़ता है | `ecat-config/src/file.rs` |
| 13 | 🟡 कम | Context में set_trace_id विधि की कमी | `ecat-transport/src/context.rs` |
| 14 | 🟡 कम | discover() में अनावश्यक clone | `ecat-registry/src/memory.rs` |
| 15 | 🟡 कम | query() columns का बार-बार clone | `ecat-data-sqlx/src/lib.rs` |
| 16 | 🟡 कम | रेट लिमिटिंग मिडलवेयर की कमी | — |

---

## 10. सारांश

फ्रेमवर्क संरचना डिज़ाइन उचित, परतें स्पष्ट, कंपाइल और lint गुणवत्ता अच्छी। मुख्य जोखिम यहाँ केंद्रित हैं:
1. **SecurityLayer कागज़ी बाघ है** — पता लगाता है लेकिन रोकता नहीं, सबसे तत्काल मरम्मत की आवश्यकता
2. **ProtoCodec अनुपयोगी** — यदि protobuf समर्थन का दावा है, तो लागू करना ही होगा
3. **सर्वर सुचारू बंद काम नहीं करता** — प्रोडक्शन परिनियोजन को प्रभावित करता है
4. **बहुत सारे stub और शून्य टेस्ट कवरेज** — समग्र परिपक्वता प्रारंभिक चरण में

प्राथमिकता क्रम (गंभीर → मध्यम → कम) के अनुसार उपरोक्त समस्याओं को धीरे-धीरे मरम्मत करने का सुझाव।

---

## 11. मरम्मत रिकॉर्ड (2026-08-01)

निम्नलिखित सभी समस्याएँ इस कमिट में मरम्मत की गईं:

| # | समस्या | मरम्मत विधि | स्थिति |
|---|------|----------|------|
| 1 | SecurityLayer नहीं रोकता | `SecurityError` त्रुटि प्रकार + `matches!` से उच्च जोखिम हमलों को रोकना | ✅ मरम्मत |
| 2 | ProtoCodec अनुपयोगी | `prost-codec` feature flag + `encode_message`/`decode_message` API जोड़ा | ✅ मरम्मत |
| 3 | Server stop() नो-ऑप | `watch::channel` + `with_graceful_shutdown` / `serve_with_shutdown` | ✅ मरम्मत |
| 4 | 7 crates में शून्य टेस्ट | RateLimitLayer में 4 नए टेस्ट; middleware में अब 4 tests | ✅ आंशिक मरम्मत |
| 5 | JoinHandle एकत्र नहीं | `Vec<JoinHandle>` एकत्र कर shutdown पर await | ✅ मरम्मत |
| 6 | Transaction लागू नहीं | `pool.begin()` से ट्रांज़ैक्शन समर्थन लागू | ✅ मरम्मत |
| 7 | Registration::Drop | `tokio::runtime::Handle::try_current()` सुरक्षित जाँच | ✅ मरम्मत |
| 8 | SQL कॉलम प्रकार मैपिंग | `bool` + `i32` समर्थन पथ जोड़ा | ✅ मरम्मत |
| 9 | CLI new खोखला | Cargo.toml, src/main.rs, proto/service.proto वास्तव में जनरेट करता है | ✅ मरम्मत |
| 10 | manifest key warning | `workspace.package.name` हटाया | ✅ मरम्मत |
| 11 | Edition असंगतता | `edition.workspace = true` (2024) में एकीकृत | ✅ मरम्मत |
| 12 | FileSource चुपचाप छोड़ता है | `ok_or_else` से स्पष्ट त्रुटि लौटाता है | ✅ मरम्मत |
| 13 | Context में विधियों की कमी | `set_trace_id`, `set_meta`, `get_meta` जोड़े | ✅ मरम्मत |
| 14 | discover() clone | `Arc<ServiceInfo>` से clone कम | ✅ मरम्मत |
| 15 | query() columns clone | `Arc<Vec<String>>` साझा संदर्भ | ✅ मरम्मत |
| 16 | रेट लिमिटिंग की कमी | नया `RateLimitLayer` (token-bucket) + 4 टेस्ट | ✅ मरम्मत |

### नए टेस्ट

- `ecat-middleware`: 4 RateLimitLayer टेस्ट (अनुमति, रोकना, पृथक key, निर्माण)
- कुल टेस्ट संख्या: 66 → 70

### संस्करण एकीकरण

- मूल workspace: `version = "1.0.3"`, `edition = "2024"`
- सभी उप-crates: `version.workspace = true`, `edition.workspace = true`

### अंतिम कंपाइल स्थिति

- `cargo check --workspace`: ✅ पास, शून्य warning
- `cargo clippy --workspace --all-features`: ✅ पास
- `cargo test --workspace`: ✅ 70/70 पास
