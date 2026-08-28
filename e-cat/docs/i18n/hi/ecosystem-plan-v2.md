# e-cat इकोसिस्टम योजना v2 — पूर्ण और आगे

**संस्करण:** 2.1.7  
**दिनांक:** 2026-08-01  
**स्थिति:** सभी योजनाएँ पूर्ण, 47 crates

---

## एक、पूर्ण (सभी डिलीवर)

| फेज़ | Crate | क्षमता | परीक्षण |
|------|-------|------|------|
| फेज़ 1 | `ecat-health` | हेल्थ चेक (/health、/ready) | 4 |
| फेज़ 1 | `ecat-client` | HTTP/gRPC क्लाइंट + सेवा खोज + लोड बैलेंसिंग | 7 |
| फेज़ 1 | `ecat-circuit-breaker` | त्रि-अवस्था सर्किट ब्रेकर (Tower Layer) | 4 |
| फेज़ 1 | `ecat-auth` | JWT + API Key + OAuth2 प्रमाणीकरण मिडलवेयर | 8 |
| फेज़ 1 | `ecat-registry-consul` | Consul सेवा रजिस्ट्री | 2 |
| फेज़ 2 | `ecat-data-redis` | Redis कैश (Cache trait) | 1 |
| फेज़ 2 | `ecat-mq` | मैसेज क्यू एब्स्ट्रैक्शन + InMemoryMq | 2 |
| फेज़ 2 | `ecat-events` | स्थानीय + रिमोट इवेंट बस | 2 |
| फेज़ 2 | `ecat-config-remote` | Consul KV रिमोट कॉन्फ़िगरेशन | 2 |
| फेज़ 3 | `ecat-testing` | MockServer + ChaosConfig | 5 |
| फेज़ 3 | `ecat-openapi` | OpenAPI 3.0 spec जनरेशन | 2 |
| फेज़ 3 | `ecat-bench` | समवर्ती प्रदर्शन बेंचमार्क | 2 |
| फेज़ 3 | `ecat-deploy` | Dockerfile + K8s + Helm | — |
| फेज़ 4 | `ecat-tracing` | वितरित ट्रेसिंग (span + trace_id) | 2 |
| फेज़ 4 | `ecat-client` विस्तार | GrpcClient + TlsConfig | — |
| फेज़ 4 | `ecat-auth` विस्तार | OAuth2Layer | — |
| फेज़ 5 | `ecat-registry-etcd` | etcd सेवा रजिस्ट्री | 4 |
| फेज़ 5 | `ecat-mq-kafka` | Kafka मैसेज क्यू | 1 |
| फेज़ 5 | `ecat-data-opensearch` | OpenSearch खोज | 1 |
| फेज़ 5 | `ecat-data-influxdb` | InfluxDB टाइम-सीरीज़ | 2 |
| फेज़ 5 | `ecat-data-elasticsearch` | Elasticsearch खोज | 2 |
| फेज़ 5 | `ecat-data-clickhouse` | ClickHouse OLAP | 1 |
| फेज़ 5 | `ecat-data-memcached` | Memcached कैश | 3 |
| फेज़ 5 | `ecat-data-neo4j` | Neo4j ग्राफ डेटाबेस | 1 |
| फेज़ 5 | `ecat-data-nebulagraph` | NebulaGraph ग्राफ डेटाबेस | 1 |
| फेज़ 5 | `ecat-data-arangodb` | ArangoDB ग्राफ डेटाबेस | 1 |
| फेज़ 5 | `ecat-data-iotdb` | IoTDB टाइम-सीरीज़ | 1 |
| फेज़ 5 | `ecat-data-questdb` | QuestDB टाइम-सीरीज़ | 1 |
| फेज़ 6 | `ecat-transport-ws` | WebSocket समर्थन | 2 |
| फेज़ 6 | `ecat-versioning` | API संस्करण रूटिंग | 2 |
| फेज़ 6 | `ecat-graphql` | GraphQL endpoint | 9 |
| फेज़ 6 | CI/CD टेम्पलेट | GitHub Actions | — |

---

## दो、शेष अंतराल (3)

| # | अंतराल | कार्य-मात्रा |
|---|------|--------|
| 1 | **transport में mTLS एकीकरण** | छोटा |
| 2 | **Redis रेट-लिमिट बैकएंड** | छोटा |
| 3 | **GitLab CI टेम्पलेट** | छोटा |

---

## तीन、संस्करण रोडमैप

```
v1.0.x  कोर स्केलेटन (18 crates)                    ✅ पूर्ण
v2.0.x  इकोसिस्टम फेज़ 1～3 (+13 crates = 31 total)   ✅ पूर्ण
v2.1.x  संचार और सुरक्षा + डेटा बैकएंड + संचालन अनुभव             ✅ पूर्ण (वर्तमान 47 crates)
```

## चार、इकोसिस्टम में शामिल नहीं

| आवश्यकता | समाधान | कारण |
|------|------|------|
| API गेटवे | Kong / Envoy | भाषा-स्वतंत्र |
| सर्विस मेश | Linkerd | Rust में कोई परिपक्व समाधान नहीं |
| कंटेनर ऑर्केस्ट्रेशन | Kubernetes | उद्योग मानक |
| लॉग संग्रह | Vector | Rust नेटिव |
