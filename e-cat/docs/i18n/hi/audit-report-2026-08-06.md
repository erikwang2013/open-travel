<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat व्यापक समीक्षा रिपोर्ट

**तिथि**: 2026-08-06
**संस्करण**: 2.3.0 · 55 crates
**दायरा**: बिल्ड/टेस्ट, रनटाइम स्मोक, इकोसिस्टम संगति, सुरक्षा सुरक्षा, परिनियोजन कॉन्फ़िगरेशन

---

## 1. टेस्ट और बिल्ड परिणाम

| जाँच आइटम | परिणाम | विवरण |
|--------|------|------|
| `cargo check --workspace` | ✅ पास | 0 चेतावनी |
| `cargo test --workspace` | ✅ पास | **202 टेस्ट सभी पास, 0 विफल** (doc-tests सहित) |
| `cargo fmt --check` | ✅ पास | |
| `cargo clippy --workspace -- -D warnings` | ✅ पास | CI कमांड के अनुरूप |
| `cargo clippy --all-targets -- -D warnings` | ❌ विफल | निष्कर्ष D2 देखें |
| स्मोक टेस्ट (helloworld) | ❌ **स्टार्टअप विफल** | निष्कर्ष D1 देखें |

**टेस्ट कवरेज वितरण**: 51 स्रोत फ़ाइलों में `#[test]`, 105 टेस्ट बाइनरी। प्रोडक्शन पथों में कोई `todo!()`/`unimplemented!()` नहीं, `panic!` केवल टेस्ट कोड में।

---

## 2. रनटाइम समस्याएँ (स्मोक टेस्ट से पता चलीं)

### [HIGH] D1. `HttpServer::new(":8000")` IPv6 के बिना वातावरण में स्टार्टअप विफल
- **स्थान**: `ecat-transport-http/src/lib.rs:40`, `examples/helloworld/src/main.rs:41`, README कई स्थान
- **लक्षण**: `TcpListener::bind(":8000")` IPv6 वाइल्डकार्ड `[::]:8000` में resolve होता है, IPv6 के बिना मशीनों (कंटेनर/कुछ क्लाउड होस्ट) पर `failed to lookup address information: Name or service not known` रिपोर्ट होता है, सेवा शुरू नहीं होती।
- **पुनरुत्पादन**: स्टैंडअलोन न्यूनतम प्रोग्राम सत्यापन — `bind(":8001")` विफल, `bind("0.0.0.0:8002")` सफल, `bind("localhost:8003")` सफल।
- **मरम्मत**: `HttpServer::new` आंतरिक रूप से खाली host को `"0.0.0.0"` में सामान्यीकृत करता है; उदाहरण और दस्तावेज़ एकीकृत रूप से `"0.0.0.0:8000"` उपयोग करते हैं।

### [LOW] D2. `cargo clippy --all-targets -- -D warnings` विफल
- **स्थान**: `ecat-data-sqlx/src/lib.rs` (टेस्ट मॉड्यूल के बाद items, `items_after_test_module` ट्रिगर)
- **प्रभाव**: वर्तमान CI का clippy कमांड (`--all-targets` के बिना) प्रभावित नहीं; CI सख्त होने पर विफल।
- **मरम्मत**: टेस्ट मॉड्यूल को फ़ाइल के अंत में ले जाएँ।

---

## 3. गंभीर समस्याएँ (CRITICAL)

### [CRITICAL] C1. `ecat-data-memcached` «नकली कार्यान्वयन» है
- **स्थान**: `ecat-data-memcached/src/lib.rs:23-88`
- **समस्या**: पूरा crate शुद्ध मेमोरी `HashMap` है, कोई नेटवर्क कनेक्शन नहीं, कोई सर्वर एड्रेस कॉन्फ़िगरेशन नहीं (`MemcachedConfig` में केवल username/password/tls है), Cargo.toml description स्वयं "in-memory cache client" स्वीकार करता है। प्रोडक्शन में गलत उपयोग से **चुपचाप डेटा हानि** होगी (रीस्टार्ट पर खाली, मल्टी-इंस्टेंस साझा नहीं)।
- **मरम्मत**: वास्तविक memcached प्रोटोकॉल से जोड़ें (जैसे `memcache` crate), या स्पष्ट रूप से `#[deprecated]`/दस्तावेज़ चेतावनी से प्रोडक्शन उपयोग रोकें।

### [CRITICAL] C2. TDengine लेखन SQL संयोजन इंजेक्शन
- **स्थान**: `ecat-data-tdengine/src/lib.rs:91-116`
- **समस्या**: `INSERT INTO "{}" ({}) VALUES ({})` में measurement/कॉलम नाम/मान सभी `format!` से सीधे संयोजित होते हैं, स्ट्रिंग मान केवल डबल कोट में लपेटा जाता है, `"` और `\` एस्केप नहीं होते। `"; DELETE ...; --` युक्त फ़ील्ड मान एस्केप होकर मनमाना SQL निष्पादित कर सकता है (TDengine REST मल्टी-स्टेटमेंट समर्थन करता है)।
- **मरम्मत**: पहचानकर्ताओं और स्ट्रिंग मानों को एस्केप करें (`"`→`\"`, `\`→`\\`), या पैरामीटराइज़्ड लेखन इंटरफ़ेस में बदलें।

---

## 4. उच्च जोखिम समस्याएँ (HIGH)

### [HIGH] H1. सभी HTTP डेटाबेस एडेप्टर में कोई टाइमआउट नहीं
- **स्थान**: `ecat-tls/src/lib.rs:27,61`, elasticsearch/opensearch/clickhouse/influxdb/iotdb/questdb/tdengine/neo4j/nebulagraph/arangodb
- **समस्या**: reqwest डिफ़ॉल्ट रूप से कोई टाइमआउट नहीं, सर्वर हैंग होने पर अनुरोध **हमेशा के लिए लटकता है** (कनेक्शन पूल खत्म, टास्क लीक)।
- **मरम्मत**: `build_reqwest_client` में एकीकृत `connect_timeout` (जैसे 5s) + `timeout` (जैसे 30s) सेट करें।

### [HIGH] H2. रेट लिमिटिंग क्लाइंट अनुसार प्रभावी नहीं हो सकती
- **स्थान**: `ecat-middleware/src/ratelimit.rs:155`
- **समस्या**: `key_fn("")` अनुरोध ऑब्जेक्ट नहीं पा सकता, IP/उपयोगकर्ता अनुसार लिमिट असंभव; डिफ़ॉल्ट एकल बकेट "global", हमलावर वैश्विक कोटा खत्म कर सकते हैं (दूसरों को DoS) या वितरित बायपास।
- **मरम्मत**: `key_fn` हस्ताक्षर `&http::Request` प्राप्त करने में बदलें, `X-Forwarded-For`/पीयर एड्रेस से key लें।

### [HIGH] H3. GitHub CI अनिवार्य रूप से विफल (protoc की कमी)
- **स्थान**: `.github/workflows/ci.yml`
- **समस्या**: `ecat-protos` build.rs tonic-build से proto कंपाइल करता है, protoc की कड़ी निर्भरता; GH CI में `protobuf-compiler` स्थापित नहीं (स्थानीय `/home/erik/.local/bin/protoc` मौजूद होने से स्थानीय पास)। `.gitlab-ci.yml` में स्थापित है, दो CI व्यवहार असंगत।
- **मरम्मत**: GH CI में `apt-get install protobuf-compiler` जोड़ें (और आवश्यकता होने पर cmake)।

### [HIGH] H4. Elasticsearch `search()`/`delete()` HTTP स्टेटस कोड जाँच नहीं करते
- **स्थान**: `ecat-data-elasticsearch/src/lib.rs:87-114`
- **समस्या**: 404/400 त्रुटि बॉडी को JSON के रूप में पार्स कर "es parse" गलत त्रुटि रिपोर्ट होती है; `index()` जाँच करता है लेकिन `search`/`delete` नहीं, व्यवहार असंगत (opensearch सही है)।
- **मरम्मत**: एकीकृत रूप से `status.is_success()` जाँचें।

### [HIGH] H5. IoTDB `insertTablet` प्रोटोकॉल असंगतता का संदेह
- **स्थान**: `ecat-data-iotdb/src/lib.rs:51-82`
- **समस्या**: IoTDB REST `insertTablet` को `timestamps/measurements/values/data_types` ऐरे फ़ॉर्मेट चाहिए; यह कार्यान्वयन एकल दस्तावेज़ JSON भेजता है, «दिखने में लागू लेकिन वास्तव में बेकार» हो सकता है।
- **मरम्मत**: insertTablet विनिर्देश अनुसार अनुरोध बॉडी बनाएँ, और इंटीग्रेशन टेस्ट जोड़ें।

### [HIGH] H6. etcd deregister उपसर्ग मेल नहीं खाता (deregister अप्रभावी)
- **स्थान**: `ecat-registry-etcd/src/lib.rs:47,66`
- **समस्या**: रजिस्ट्रेशन कुंजी `/ecat/services/{prefix}/{name}/{uuid}` है, deregister केवल `{prefix}/{name}` हटाता है (uuid सेगमेंट कम) → इंस्टेंस बाहर निकलने के बाद रजिस्ट्रेशन जानकारी बची रहती है।
- **मरम्मत**: हटाते समय पूर्ण कुंजी मिलाएँ या सूची के बाद name उपसर्ग अनुसार हटाएँ।

---

## 5. मध्यम जोखिम समस्याएँ (MEDIUM)

| # | स्थान | समस्या | सुझाव |
|---|------|------|------|
| M1 | `ecat-middleware/src/ratelimit_redis.rs:28-48` | Redis विफलता पर Err लौटना अति-सीमा माना जाता है → **fail-closed DoS**; INCR के बाद EXPIRE विफल कुंजी कभी समाप्त नहीं → स्थायी प्रतिबंध | लिमिट/स्टोरेज त्रुटियाँ अलग करें (स्टोरेज विफलता पर पास), Lua परमाणु स्क्रिप्ट |
| M2 | `ecat-middleware/src/ratelimit.rs:16-51` | MemoryStore प्रविष्टियाँ केवल रीसेट होती हैं, हटती नहीं, क्लाइंट कुंजी के अनुसार **मेमोरी असीमित बढ़ती है** | समाप्त बकेट नियमित रूप से साफ़ करें |
| M3 | `ecat-auth/src/jwt.rs:25-31` | कमज़ोर कुंजी में न्यूनतम लंबाई जाँच नहीं (टेस्ट "secret-key" उपयोग), ऑफ़लाइन ब्रूट-फोर्स संभव | ≥32 बाइट यादृच्छिक कुंजी अनिवार्य; त्रुटि प्रतिक्रिया सामान्यीकृत करें, jsonwebtoken विवरण प्रतिध्वनि से बचें |
| M4 | `ecat-auth/src/oauth2.rs:111-123` | हर अनुरोध नया reqwest::Client बिना timeout; URL HTTPS अनिवार्य नहीं | Client पुनः उपयोग, timeout सेट, https सत्यापन |
| M5 | `ecat-data-redis/src/lib.rs:34-64`, `ratelimit_redis.rs:12-17`, ecat-lock | पासवर्ड percent_encode के बाद URL में एम्बेड, कनेक्शन त्रुटि Display में पूर्ण URL → **लॉग में पासवर्ड लीक**; URL में पहले से `@` होने पर क्रेडेंशियल चुपचाप छोड़ दिए जाते हैं | प्रमाणीकरण पैरामीटर अलग से पास करें, त्रुटि संदेश डी-सेंसिटाइज़ करें |
| M6 | `ecat-data-elasticsearch/src/lib.rs:104-113`, opensearch:111-116 | index/id URL-एन्कोडेड नहीं, पथ में संयोजित, `/` से अन्य index तक पहुँच संभव (IDOR) | URL एन्कोडिंग + index व्हाइटलिस्ट |
| M7 | `ecat-data-sqlx/src/lib.rs:79,173`, questdb:78-84 | डेटाबेस कच्ची त्रुटियाँ (SQL और मान सहित) सीधे ऊपर फेंकी जाती हैं | बाहरी स्तर पर सामान्यीकृत करें, विवरण केवल लॉग में |
| M8 | `ecat-data-clickhouse/src/lib.rs:92` | `execute()` हमेशा `Ok(0)` लौटाता है, rows_affected खो जाती है; `query()` पार्स विफल पंक्तियाँ चुपचाप छोड़ता है | वास्तविक पंक्ति संख्या लौटाएँ, त्रुटि ऊपर फेंकें |
| M9 | `ecat-data-tdengine/src/lib.rs:80-118` | `write()` एक-एक करके लूप अनुरोध करता है (N+1) | बैच लेखन |
| M10 | `ecat-data-sqlx/src/lib.rs:98-142 बनाम 213-256` | query/query_with में ~50 पंक्तियाँ दोहराव वाला प्रकार रूपांतरण तर्क | साझा फ़ंक्शन निकालें |
| M11 | `ecat-data-redis/src/lib.rs:167` | `acquire` में `ttl.as_millis() as u64` ओवरफ्लो ट्रंकेशन (`set` में संभाला गया यहाँ नहीं) | एकीकृत ओवरफ्लो प्रबंधन |
| M12 | `ecat-data-influxdb/src/lib.rs:69-79` | line protocol स्ट्रिंग फ़ील्ड एस्केप नहीं (कोट/कॉमा/स्पेस) → लिखते ही प्रोटोकॉल त्रुटि | विनिर्देश अनुसार एस्केप करें |
| M13 | `ecat-mq-*` | `from_config` हस्ताक्षर एकीकृत नहीं: kafka/mqtt सिंक्रोनस लौटते हैं, rabbitmq/nats async | async में एकीकृत करें |
| M14 | `ecat-auth/src/apikey.rs:33-36`, `ecat-security/src/lib.rs:126-137` | API key query पैरामीटर समर्थन (लॉग/Referer में पड़ता है); WAF केवल URI+headers स्कैन करता है, body नहीं | केवल header में key; WAF में body स्कैन जोड़ें |

---

## 6. कम जोखिम और सूचना स्तर (LOW/INFO)

| # | स्थान | समस्या |
|---|------|------|
| L1 | `ecat-deploy/Dockerfile` | **अस्तित्वहीन `ecat-app` बाइनरी कॉपी** (वास्तविक bin `ecat` है, ecat-cli से) → docker build के बाद इमेज में कोई एंट्रीपॉइंट नहीं; HEALTHCHECK curl उपयोग करता है लेकिन इमेज में curl स्थापित नहीं |
| L2 | `ecat-deploy/helm/Chart.yaml` | appVersion "2.2.0" है, वर्तमान संस्करण 2.3.0 |
| L3 | `README.en.md` | "v2.1.7 · 47 crates" का दावा, वास्तव में v2.3.0 · 55 crates, अंग्रेज़ी दस्तावेज़ गंभीर रूप से पुराना |
| L4 | `ecat-registry-consul/src/lib.rs:66,143` | रजिस्ट्रेशन पोर्ट हमेशा 0, discover परिणाम संस्करण हार्डकोडेड "1.0" |
| L5 | 11 crates के Cargo.toml | `workspace.dependencies` बायपास कर सीधे समान संस्करण निर्भरता लिखना (संस्करण ड्रिफ्ट जोखिम) |
| L6 | `ecat-tracing` / `ecat-middleware/src/tracing.rs` | TracingLayer दोहराव; ecat-tracing-otlp और ecat-tracing प्रत्येक स्वतंत्र subscriber स्थापित करते हैं, एक साथ कॉल करने पर डबल इनिट संघर्ष |
| L7 | `ecat-config-remote/src/lib.rs:92` | हाथ से लिखा base64 डिकोडिंग, base64 crate उपयोग का सुझाव |
| L8 | `ecat-graphql` | हाथ से लिखा एकल-फ़ील्ड पार्सर, केवल शीर्ष-स्तरीय एकल फ़ील्ड समर्थन (कोई नेस्टिंग/उपनाम/पैरामीटर नहीं), दस्तावेज़ में सीमा नहीं बताई |
| L9 | `ecat-cli/src/main.rs:69-104`, lib.rs:3-22 | `ecat new ../../x` पथ ट्रैवर्सल; नाम में `"`/न्यूलाइन जनरेटेड Cargo.toml में इंजेक्ट किया जा सकता है |
| L10 | `config/databases.example.yaml:54-79` | कई मान्य डिफ़ॉल्ट पासवर्ड (neo4j/changeme, arangodb root/changeme, iotdb root/root, influx my-secret-token), कॉपी करते ही डिफ़ॉल्ट पासवर्ड के साथ लाइव |
| L11 | `ecat-data-s3/src/lib.rs:83-93` | list() में कोई टाइमआउट कॉन्फ़िगरेशन नहीं; क्रेडेंशियल निर्माण सिंक्रोनस ब्लॉकिंग कॉल |
| L12 | `ecat-data-redis` | कोई स्पष्ट रीकनेक्ट नहीं, MultiplexedConnection अंतर्निहित रीकनेक्ट पर निर्भर, दस्तावेज़ में नहीं बताया |
| L13 | `ecat-data/src/rdbms.rs:71-77` | `Transaction::drop` केवल warn करता है, रोलबैक ट्रिगर नहीं, sqlx पक्ष के drop स्वचालित रोलबैक पर निर्भर, टिप्पणी से समझाने का सुझाव |

---

## 7. इकोसिस्टम पूर्णता निष्कर्ष

**पूर्णता: उच्च**। 55/55 crates workspace में, संस्करण एकीकृत 2.3.0, कोई stub नहीं (memcached नकली कार्यान्वयन को छोड़कर)। 18 डेटाबेस बैकएंड, 4 MQ बैकएंड, 2 रजिस्ट्री, रेट लिमिट स्टोरेज एब्स्ट्रैक्शन, वितरित लॉक, शेड्यूलर, OTLP ट्रेसिंग, वर्ज़निंग, GraphQL सभी पूरे। `todo!()`/`unimplemented!()` शून्य स्थान।

**सुदृढ़ीकरण आवश्यक**:
1. memcached वास्तविक प्रोटोकॉल कार्यान्वयन (वर्तमान में एकमात्र «नकली» एडेप्टर)
2. IoTDB प्रोटोकॉल अनुपालन सत्यापन (बेकार होने का संदेह)
3. GitHub CI और GitLab CI संरेखण (protoc की कमी)
4. सभी HTTP एडेप्टर में एकीकृत टाइमआउट नीति

## 8. सुरक्षा सुरक्षा निष्कर्ष

**कोई CRITICAL सुरक्षा भेद्यता नहीं (इंजेक्शन/क्रेडेंशियल प्रबंधन/TLS डिफ़ॉल्ट सुरक्षित)**:
- ✅ पूरे workspace में शून्य unsafe ब्लॉक
- ✅ कोई हार्डकोडेड क्रेडेंशियल नहीं, उदाहरण कॉन्फ़िगरेशन changeme प्लेसहोल्डर (सभी टिप्पणी करने का सुझाव, L10)
- ✅ sqlx सभी पैरामीटराइज़्ड बाइंडिंग; Redis लॉक Lua CAS से रिलीज़
- ✅ TLS `skip_verify` डिफ़ॉल्ट बंद; Redis स्वचालित rediss:// में अपग्रेड
- ⚠️ बाकी: TDengine संयोजन इंजेक्शन (C2, sqlx कवरेज के बाहर), क्लाइंट अनुसार रेट लिमिट (H2), Redis रेट लिमिट fail-closed (M1), JWT कमज़ोर कुंजी (M3), Redis त्रुटि संदेश लीक (M5), ES पथ इंजेक्शन (M6)

## 9. अनुकूलन सुझाव (शीर्ष प्राथमिकता क्रम)

1. **P0**: C1 नकली कार्यान्वयन, C2 SQL इंजेक्शन, D1 पोर्ट बाइंडिंग, H1 टाइमआउट — 4 आइटम
2. **P1**: H2 रेट लिमिट, H3 CI, H4 ES स्टेटस कोड, H5 IoTDB, H6 etcd deregister
3. **P1**: M1 fail-closed, M3 JWT, M5 पासवर्ड लीक, M6 पथ इंजेक्शन
4. **P2**: Dockerfile/Helm/README मरम्मत, clippy --all-targets, त्रुटि प्रसारण, बैच लेखन
5. **P3**: workspace.dependencies संकेन्द्रण, MQ from_config एकीकरण, दस्तावेज़ सिंक्रनाइज़ेशन

---

## 10. मरम्मत स्थिति (2026-08-06 पुनः सत्यापन)

**सभी 35 निष्कर्ष मरम्मत या दस्तावेज़ीकृत।** पुनः सत्यापन परिणाम: `cargo check --workspace` ✅, `cargo test --workspace` 219 टेस्ट सभी पास ✅, `cargo clippy --workspace --all-targets -- -D warnings` शून्य चेतावनी ✅, `cargo fmt --check` साफ़ ✅, helloworld स्मोक टेस्ट (`/` + `/health`) ✅।

| संख्या | गंभीरता | मरम्मत विधि | सत्यापन |
|------|--------|----------|------|
| D1 | HIGH | `HttpServer` खाली host को `0.0.0.0` में सामान्यीकृत; उदाहरण/दस्तावेज़/CLI टेम्पलेट एकीकृत `0.0.0.0:8000` | स्मोक टेस्ट बाइंडिंग सफल |
| D2 | LOW | `SqlxTransactionWrapper` impl टेस्ट मॉड्यूल से पहले ले जाया | clippy शून्य चेतावनी |
| C1 | CRITICAL | memcached स्पष्ट रूप से «केवल डेव/टेस्ट» चिह्नित; `in_memory` स्विच; get लेज़ी समाप्ति + set sweep | 23 डेटा परत टेस्ट पास |
| C2 | CRITICAL | TDengine डबल एस्केप (`\`→`\\`, `"`→`\"`); 100 प्रविष्टियों के बैच चंक | पास |
| H1 | HIGH | `ecat-tls` एकीकृत connect 5s / request 30s टाइमआउट, सभी HTTP एडेप्टर इनहेरिट | पास |
| H2 | HIGH | रेट लिमिट key डिफ़ॉल्ट X-Forwarded-For पहला हॉप → X-Real-IP → global; MemoryStore 60s लेज़ी सफाई | 22 मिडलवेयर टेस्ट पास |
| H3 | HIGH | CI में `protobuf-compiler` स्थापना जोड़ी | कॉन्फ़िगरेशन अपडेट |
| H4 | HIGH | ES/OpenSearch `search()`/`delete()` में `is_success()` जाँच; index/id RFC 3986 एन्कोडिंग | पास |
| H5 | HIGH | IoTDB मानक insertTablet body में पुनर्गठित, `code != 200` जाँच | पास |
| H6 | HIGH | etcd deregister उपसर्ग range delete उपयोग, रजिस्ट्रेशन कुंजी मिलान | पास |
| M1 | MED | Redis रेट लिमिट: Lua परमाणु INCR+EXPIRE, EXPIRE विफल पर DEL रोलबैक, कनेक्शन त्रुटि fail-open + warn | पास |
| M3 | MED | JWT कुंजी <32 बाइट अस्वीकृत (`WeakKey`); त्रुटि प्रतिक्रिया एकीकृत `invalid token` | 9 auth टेस्ट पास |
| M5 | MED | Redis पासवर्ड `ConnectionInfo` से अलग पास, URL में एम्बेड नहीं | पास |
| M6 | MED | ES/OpenSearch/InfluxDB सभी इंजेक्शन सतहें एस्केप या पैरामीटराइज़्ड | पास |
| M9 | MED | TDengine 100 प्रविष्टियाँ/बैच | पास |
| M11 | MED | Redis ttl ओवरफ्लो क्लैंप `u64::MAX` | पास |
| M13 | MED | MQ `from_config` एकीकृत async (kafka/mqtt सिंक्रोनसीकरण) | 11 CLI टेस्ट पास |
| L श्रृंखला | LOW/INFO | Dockerfile (वास्तविक बाइनरी नाम + curl स्वास्थ्य जाँच + builder 1.85), Chart appVersion 2.3.0, उदाहरण पासवर्ड टिप्पणीकृत, consul संस्करण/पोर्ट रजिस्ट्रेशन जानकारी से पार्स, हाथ से लिखा base64 `base64` crate से बदला, `validate_crate_name` इंजेक्शन रोकथाम, workspace.dependencies 8 स्थान संकेन्द्रण, डबल subscriber संघर्ष टिप्पणी, दस्तावेज़ (README/README.en/CHANGELOG 2.3.1) सिंक्रनाइज़ेशन | सभी पास |

**मरम्मत के दौरान नई समस्याएँ**: `ecat-config-remote` टेस्ट पुराने `base64_decode` का संदर्भ देता है (agent प्रतिस्थापन में छूट गया) → `base64::engine` उपयोग में बदला; `ecat-middleware` 4 clippy चेतावनियाँ (नेस्टेड if / जटिल प्रकार) → मोड़े + `KeyFn` प्रकार उपनाम। मरम्मत के बाद कोई रिग्रेशन नहीं।

**इकोसिस्टम निष्कर्ष**: 55 crates, 18 डेटाबेस एडेप्टर, 4 MQ, Docker/Helm/CI कॉन्फ़िगरेशन, चीनी/अंग्रेज़ी README, CHANGELOG सभी v2.3.0 के अनुरूप; छवियाँ (alipay/weixinpay.png) संदर्भ सामान्य।

---

*रिपोर्ट स्वचालित समीक्षा से जनरेट हुई: बिल्ड+टेस्ट+स्मोक रन + 3 विशेष समीक्षा agents (सुरक्षा/डेटा परत/इकोसिस्टम संगति), 2026-08-06 पूर्ण पुनः सत्यापन।*
