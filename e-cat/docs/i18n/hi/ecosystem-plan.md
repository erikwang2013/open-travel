# e-cat इकोसिस्टम योजना

**संस्करण:** 2.1.7  
**दिनांक:** 2026-08-01  
**स्थिति:** सभी पूर्ण · 47 crates

| क्षेत्र | कवरेज | स्थिति |
|------|--------|------|
| ट्रांसपोर्ट परत | HTTP (axum), gRPC (tonic), WebSocket | ✅ |
| एन्कोडिंग | JSON, Protobuf | ✅ |
| मिडलवेयर | Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, Auth (JWT/API Key/OAuth2) | ✅ |
| कॉन्फ़िगरेशन | env, file (JSON/YAML), Consul KV रिमोट, एन्क्रिप्शन | ✅ |
| रजिस्ट्री | memory, Consul, etcd | ✅ |
| सुरक्षा | हमले की पहचान, JWT, API Key, OAuth2, TlsConfig | ✅ |
| डेटा | RDBMS (sqlx), Redis, Memcached, OpenSearch, Elasticsearch, ClickHouse, Neo4j, NebulaGraph, ArangoDB, InfluxDB, IoTDB, QuestDB | ✅ |
| अवलोकनीयता | tracing, Prometheus, Health, वितरित ट्रेसिंग | ✅ |
| संचार | HTTP/gRPC क्लाइंट, MessageQueue (InMemory/Kafka), EventBus | ✅ |
| DevOps | CLI, Dockerfile, K8s, Helm, GitHub Actions, Bench, Testing | ✅ |
| API उपकरण | OpenAPI, Versioning, GraphQL | ✅ |

## शेष अंतराल (3 छोटे अनुकूलन)

1. **transport में mTLS एकीकरण** — TlsConfig मौजूद है, HttpServer/GrpcServer में नहीं जुड़ा
2. **Redis रेट-लिमिट बैकएंड** — RateLimitLayer केवल मेमोरी, मल्टी-इंस्टेंस के लिए साझा करना आवश्यक
3. **GitLab CI टेम्पलेट** — वर्तमान में केवल GitHub Actions

## संस्करण विकास

```
v1.0.x  कोर स्केलेटन (18 crates)                    ✅
v2.0.x  इकोसिस्टम फेज़ 1～3 (+13 crates)              ✅
v2.1.x  संचार और सुरक्षा सुदृढ़ीकरण + डेटा बैकएंड पूर्ति + संचालन अनुभव   ✅ (वर्तमान)
```

## इकोसिस्टम में शामिल नहीं

| आवश्यकता | समाधान | कारण |
|------|------|------|
| API गेटवे | Kong / Envoy | भाषा-स्वतंत्र |
| सर्विस मेश | Linkerd | Rust में कोई परिपक्व समाधान नहीं |
| कंटेनर ऑर्केस्ट्रेशन | Kubernetes | उद्योग मानक |
| लॉग संग्रह | Vector | Rust नेटिव |
