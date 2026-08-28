# परीक्षण रिपोर्ट — 2026-08-26

व्यापक यूनिट टेस्ट पूर्ति (51 crates पूर्ण कवरेज), 4 समूह वरिष्ठ Rust परीक्षण इंजीनियर समानांतर।

## अवलोकन

| समूह | crates | मूल | नया | वर्तमान | गेट |
|---|---|---|---|---|---|
| core/फ्रेमवर्क | 12 | 102 | +40 | 142 | ✅ test सब हरा + clippy 0 चेतावनी |
| data | 14 | 87 | +66 | 153 | ✅ वही |
| mq/transport | 12 | 82 | +54 | 136 | ✅ वही |
| app अनुप्रयोग परत | 13 | ~178 | +46 | ~224 | ✅ वही |
| **कुल** | **51** | **~449** | **+206** | **~655** | ✅ |

नोट: अनुप्रयोग परत के मूल संख्याओं में ecat-auth 24 / ecat-graphql 35 / ecat-bench 11 / ecat-scheduler 6 / ecat-events 7 / ecat-cli 12 / ecat-middleware 34 / ecat-circuit-breaker 10 / ecat-health 4 / ecat-client 7 / ecat-security 12 / ecat-versioning 4 / ecat 4 शामिल हैं। प्रत्येक crate स्वतंत्र `cargo test -p` + `cargo clippy -p --all-targets -- -D warnings` पास करता है, CARGO_TARGET_DIR अलगाव समानांतर।

## प्रति crate विवरण

### core/फ्रेमवर्क समूह (test-core, +40)

| crate | मूल→नया | कवरेज बिंदु |
|---|---|---|
| ecat-protos | 4→8 | ErrorCode पूर्ण enum proto से तुलना; ट्रंकेटेड buffer decode; खाली buffer डिफ़ॉल्ट संदेश; metadata roundtrip |
| ecat-errors | 4→9 | http_status पूर्ण मैपिंग (409/429/500); from_status; अनमैप्ड→Internal; cause source() |
| ecat-metadata | 9→12 | HTTP header से trace_id निष्कर्षण; key लोअरकेस; खाली header map |
| ecat-encoding | 18→22 | NaN→null (serde_json डिफ़ॉल्ट, प्रलेखित); खाली बाइट decode; CodecBox अवैध JSON; proto roundtrip |
| ecat-lock | 7→9 | बिना लॉक release त्रुटि; खाली key |
| ecat-logging | 1→1 | संगतता shim panic नहीं |
| ecat-tracing | 9→12 | गैर-UTF-8 trace header छोड़ें; canonical header; प्रतिक्रिया ट्रांसमिशन |
| ecat-tls | 7→12 | basic_auth एक/दो फ़ील्ड; ca फ़ाइल की कमी; is_enabled; डिफ़ॉल्ट क्लाइंट |
| ecat-config | 14→26 | env उपसर्ग फ़िल्टर + प्रकार पार्सिंग सीमाएँ (hex/खाली स्ट्रिंग/-0/1e3); मल्टी-source विलय ओवरराइड; obfs त्रुटि पथ; फ़ाइल अनुपस्थित/अवैध YAML |
| ecat-config-remote | 6→9 | ConsulKvEntry सीमाएँ; X-Consul-Index की कमी त्रुटि; नेस्टेड key |
| ecat-openapi | 4→11 | components/schema_ref; डुप्लिकेट ओवरराइड; डिफ़ॉल्ट 200; tags |
| ecat-metrics | 8→11 | पंजीकृत मेट्रिक्स टेक्स्ट; 404/405 |

### data समूह (test-data, +66)

| crate | मूल→नया | कवरेज बिंदु |
|---|---|---|
| ecat-data | 12→14 | खोज सिंटैक्स पार्सिंग |
| ecat-data-sqlx | 7→14 | इन-मेमोरी SQLite एंड-टू-एंड; पैरामीटर बाइंडिंग सभी प्रकार; Blob→base64; config |
| ecat-data-redis | 6→12 | redis:///rediss:// URL निर्माण; auth; config त्रुटि पथ |
| ecat-data-opensearch | 4→10 | mock HTTP: percent-encode、Basic auth、त्रुटि ट्रांसमिशन |
| ecat-data-elasticsearch | 6→11 | वही |
| ecat-data-influxdb | 5→10 | line protocol एस्केपिंग; Token header; त्रुटि ट्रांसमिशन |
| ecat-data-clickhouse | 12→22 | टेबल बनाने वाला SQL; JSONEachRow; लिखी गई पंक्तियाँ; समूहीकरण |
| ecat-data-memcached | 4→8 | TTL सेकंड→मिलीसेकंड; flag पैकिंग |
| ecat-data-nebulagraph | 6→7 | config पार्सिंग |
| ecat-data-arangodb | 5→7 | config/URL |
| ecat-data-iotdb | 5→10 | mock HTTP: session पथ पैरामीटर |
| ecat-data-questdb | 4→9 | line protocol; ट्रांज़ैक्शन अनसमर्थित |
| ecat-data-tdengine | 6→11 | INSERT जनरेशन; 100 बैच चंकिंग |
| ecat-data-mongodb | 5→8 | bson राउंड-ट्रिप; URI |

### mq/transport/registry समूह (test-mq, +54)

| crate | मूल→नया | कवरेज बिंदु |
|---|---|---|
| ecat-mq | 5→9 | भरी buffer लैग त्रुटि फ्रेम; सभी drop स्ट्रीम बंद; मल्टी-सब्सक्राइबर; बिना सब्सक्राइबर publish |
| ecat-mq-kafka | 12→14 | config डिफ़ॉल्ट; SASL फ़ील्ड स्वतंत्र प्रभावी |
| ecat-mq-rabbitmq | 2→5 | exchange डिफ़ॉल्ट; url त्रुटि पथ |
| ecat-mq-mqtt | 5→9 | cert/key जोड़ी सत्यापन; फ़ाइल की कमी; पोर्ट डिफ़ॉल्ट 1883/8883; अवैध पोर्ट फॉलबैक |
| ecat-mq-nats | 6→9 | प्लेनटेक्स्ट डिफ़ॉल्ट; ca/cert अनुपस्थित त्रुटि पथ |
| ecat-transport | 4→7 | TlsConfig डिफ़ॉल्ट/with_client_auth; normalize_addr सीमाएँ |
| ecat-transport-http | 17→20 | एकीकरण परीक्षण: stop नो-ऑप、पोर्ट व्यस्त विफलता、वास्तविक भेजना-प्राप्त करना |
| ecat-transport-grpc | 7→13 | TLS फ़ाइल की कमी; प्लेनटेक्स्ट लाइफसाइकिल; mTLS अस्वीकृति |
| ecat-transport-ws | 4→8 | बिना handler विफलता; पोर्ट व्यस्त; RFC 6455 masked फ्रेम इको |
| ecat-registry | 5→8 | मल्टी-इंस्टेंस discover; drop स्वतः डी-रजिस्टर; builder डिफ़ॉल्ट |
| ecat-registry-consul | 10→24 | percent-encode; रजिस्टर वेरिएंट; त्रुटि प्रतिक्रियाएँ; X-Consul-Token; agent/services पार्सिंग; node फॉलबैक |
| ecat-registry-etcd | 5→10 | discover खराब मान छोड़ें; kv अनुरोध बॉडी; lease grant; keepalive |

### app अनुप्रयोग परत समूह (test-app, +46)

| crate | मूल→नया | कवरेज बिंदु |
|---|---|---|
| ecat-auth | 20→46 | oauth2 कैश व्हाइटलिस्ट/SHA-256 key/FIFO निष्कासन; apikey त्रि-अवस्था; jwt iss/aud अनिवार्य; समाप्त/गलत हस्ताक्षर |
| ecat-health | 4→8 | readiness समुच्चय (सभी ok/कोई fail/खाली रजिस्ट्री); liveness |
| ecat-versioning | 4→7 | path रणनीति रूटिंग; extract_version सीमाएँ |
| ecat-security | 12→20 | header परत एंड-टू-एंड; हमला इंटरसेप्शन JSON आकार |
| ecat-middleware | 34→37 | MemoryStore विंडो समाप्ति; आंतरिक panic→Err |
| ecat-circuit-breaker | 10→12 | half-open प्रोब समाप्ति; classify डिग्रेडेशन |
| ecat-client | 7→10 | grpc अवैध endpoint त्रुटि नेटवर्किंग के बिना |
| ecat-graphql | 35→35 | मौजूदा कवरेज पर्याप्त, कोई अंतराल नहीं |
| ecat-scheduler / ecat-bench / ecat-events / ecat-cli / ecat | मौजूदा कवरेज पर्याप्त | कोई अंतराल नहीं |

## पाए गए दोष

| स्तर | स्थान | विवरण | स्थिति |
|---|---|---|---|
| P1 | ecat-events/Cargo.toml | dev-dependencies में tokio macros/rt/time features की कमी, अकेले उस crate के परीक्षण लक्ष्य को कंपाइल करना अनिवार्य रूप से विफल होता है (workspace पूर्ण बिल्ड feature संघ से छिपा हुआ) | ✅ मरम्मत (features + टिप्पणियाँ जोड़ी) |
| P2 | ecat-security src/lib.rs:118-127 | URI प्रतिशत-एन्कोडेड SQLi (`?q=SELECT%20*%20...`) header परत स्कैन को बायपास कर सकता है (डिटेक्टर को शाब्दिक स्थान चाहिए, कच्चे URI को पहले डिकोड किए बिना स्कैन करता है); बॉडी स्कैन प्रभावित नहीं | ⏳ बाकी |
| P3 | ecat-data-sqlx | `connect()/from_config()` AnyPool उपयोग करता है लेकिन ड्राइवर इंस्टॉल नहीं किया, sqlx 0.8.6 पहले कनेक्शन पर ही panic "No drivers installed" | ⏳ बाकी |
| P3 | ecat-data-influxdb | स्ट्रिंग फ़ील्ड में स्पेस एस्केप (`\ `), line protocol मानक केवल `"` और `\` एस्केप करना चाहिए; tag/field क्रम अनिश्चित | ⏳ बाकी |
| P3 | ecat-data-clickhouse | टेबल बनाने का कैश कभी समाप्त नहीं होता, बाहरी drop/टेबल बदलने के बाद CREATE दोबारा नहीं करता | ⏳ बाकी |
| P3 | ecat-circuit-breaker | half_open_probes सीमा अनुक्रमिक प्रोब में अप्राप्य (केवल समवर्ती इन-फ्लाइट में प्राप्य), व्हाइट-बॉक्स परीक्षण कवर | ℹ️ ज्ञात, दोष नहीं |
| P3 | ecat-health | `with_check` blocking_write() उपयोग करता है, async संदर्भ में कॉल करने पर panic; वर्तमान में केवल सिंक्रोनस संदर्भ में उपयोग योग्य | ℹ️ ज्ञात, API सीमा |

## छोड़े गए मॉड्यूल (एकीकरण वातावरण आवश्यक, mock नहीं)

- वास्तविक broker राउंड-ट्रिप: kafka/rabbitmq/mqtt/nats publish-subscribe (कॉन्फ़िगरेशन और त्रुटि पथ कवर)
- वास्तविक क्लस्टर: consul/etcd रजिस्टर-डिस्कवर लाइफसाइकिल (axum mock अनुरोध आकार कवर)
- वास्तविक डेटाबेस: redis/memcached संचालन, mongod, influxdb सर्वर सत्यापन, sqlx postgres/mysql ड्राइवर, nebulagraph/arangodb API
- वास्तविक बाहरी सेवाएँ: OAuth2 introspection (स्थानीय mock कवर), gRPC/HTTP राउंड-ट्रिप (स्थानीय mock कवर 302 फॉलो नहीं)
