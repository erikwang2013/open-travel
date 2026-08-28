<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat कोड समीक्षा रिपोर्ट — 2026-08-01 (चौथा दौर · सभी मरम्मत)

**प्रोजेक्ट संस्करण:** 2.1.0  
**अंतिम स्थिति:** 0 warnings, ~116 tests, clippy clean, fmt clean

**पाँचवें दौर की सफाई:** 12 अप्रयुक्त निर्भरताएँ हटाईं (ecat-health/reqwest, ecat-circuit-breaker/tokio, ecat-bench/tracing, ecat-mq/serde+serde_json, ecat-events/async-trait, ecat-config-remote/tracing, ecat-testing/transport-http+axum, ecat-client/serde+serde_json)
**समीक्षा दायरा:** सभी 18 crates

## अंतिम स्थिति

| उपकरण | स्थिति |
|------|------|
| `cargo build` | पास (0 warnings) |
| `cargo test` | 77 passed, 0 failed, 1 ignored |
| `cargo clippy` | पास (0 warnings) |
| `cargo fmt` | पास |

---

## मरम्मत सूची (सभी)

### मध्यम जोखिम

1. **[मरम्मत]** `Mutex::lock().unwrap()` → `ecat-transport-http/lib.rs`, `ecat-transport-grpc/lib.rs`
2. **[मरम्मत]** CLI `fs::write().unwrap()` → `ecat-cli/src/main.rs`

### कम जोखिम

3. **[मरम्मत]** ProtoCodec doc-test → `ecat-encoding/src/proto.rs`
4. **[मरम्मत]** शून्य यूनिट टेस्ट वाले crates → transport-http/grpc में प्रत्येक में 3 नए टेस्ट
5. **[मरम्मत]** `Transaction::commit()` नो-ऑप → नया `TransactionInner` trait
6. **[मरम्मत]** `SecurityScanner::new()` टिप्पणी सुधार
7. **[मरम्मत]** अप्रयुक्त `opentelemetry` निर्भरता → `ecat-logging` और workspace रूट Cargo.toml
8. **[मरम्मत]** Doc-test फ़ॉर्मेटिंग

### अनुकूलन

9. **[मरम्मत]** `scan_parts` प्री-आवंटन → `Vec::with_capacity`
10. **[मरम्मत]** `serde_yaml` 0.9 अप्रचलित → `yaml_serde` 0.10 में माइग्रेट
11. **[मरम्मत]** `Transaction::commit()` अब नो-ऑप नहीं → `SqlxTransactionWrapper` के माध्यम से वास्तविक commit/rollback

### मरम्मत की आवश्यकता नहीं (डिज़ाइन निर्णय)

- **`ecat` crate अतिरिक्त निर्भरताएँ** — जानबूझकर किया गया «meta crate» पैटर्न, डाउनस्ट्रीम को सुविधाजनक ट्रांज़िटिव निर्भरताएँ प्रदान करता है
- **ProtoCodec Codec trait त्रुटि लौटाता है** — serde और prost::Message की मौलिक प्रकार असंगतता, `encode_message()`/`decode_message()` पृथक API और स्पष्ट दस्तावेज़ीकरण से हल
- **`ecat-data` में कोई ठोस कार्यान्वयन नहीं** — trait इंटरफ़ेस डिज़ाइन, कार्यान्वयन `ecat-data-sqlx` में

---

## परिवर्तित फ़ाइलें सारांश

| फ़ाइल | परिवर्तन |
|------|------|
| `ecat-transport-http/src/lib.rs` | Mutex विषाक्तता सुरक्षा + 3 नए टेस्ट |
| `ecat-transport-grpc/src/lib.rs` | Mutex विषाक्तता सुरक्षा + 3 नए टेस्ट |
| `ecat-cli/src/main.rs` | एकीकृत त्रुटि प्रबंधन |
| `ecat-security/src/lib.rs` | टिप्पणी सुधार + प्री-आवंटन अनुकूलन |
| `ecat-logging/Cargo.toml` | अप्रयुक्त opentelemetry हटाया |
| `ecat-encoding/src/proto.rs` | doc-test सुधार |
| `ecat-data/src/lib.rs` | TransactionInner निर्यात |
| `ecat-data/src/rdbms.rs` | नया TransactionInner trait |
| `ecat-data-sqlx/src/lib.rs` | SqlxTransactionWrapper TransactionInner लागू करता है |
| `ecat-config/Cargo.toml` | serde_yaml → yaml_serde |
| `ecat-config/src/file.rs` | serde_yaml → yaml_serde |
| `Cargo.toml` | orphaned opentelemetry workspace निर्भरता हटाई |
| `README.md` | संस्करण संख्या अपडेट, अवलोकनीयता विवरण सुधार, इकोसिस्टम प्लान लिंक जोड़ा |
| `docs/ecosystem-plan.md` | नया इकोसिस्टम प्लान दस्तावेज़ (तीन चरणों में 15 crates) |
