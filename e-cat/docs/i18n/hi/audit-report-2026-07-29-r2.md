<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat कोड समीक्षा रिपोर्ट (दूसरा दौर)

**दिनांक**: 2026-07-29  
**शाखा**: main  
**प्रोजेक्ट**: e-cat (Rust workspace, 17 crates)

---

## एक、समीक्षा सारांश

पहले दौर के clippy मरम्मत और परीक्षण पूर्ति के आधार पर, इस दौर में गहन कोड लॉजिक समीक्षा की गई, मुख्य ध्यान रनटाइम सहीता, समवर्ती सुरक्षा, API सिमेंटिक्स स्थिरता पर। कुल 32 स्रोत फ़ाइलों की समीक्षा की गई।

### सत्यापन बेसलाइन

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

---

## दो、पाए गए Bugs और मरम्मत

### Bug 1: [महत्वपूर्ण] TracingLayer span गार्ड लाइफसाइकिल त्रुटि

- **फ़ाइल**: `ecat-middleware/src/tracing.rs:37`
- **गंभीरता**: **उच्च**
- **प्रभाव**: TracingLayer से गुज़रने वाले सभी अनुरोध tracing span द्वारा कवर नहीं होते

**मूल कारण विश्लेषण**:

```rust
// मरम्मत से पहले
fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let _guard = span.enter();  // guard call() लौटते ही drop हो जाता है
    let fut = self.inner.call(req);
    Box::pin(fut)               // future बाद के poll में ही निष्पादित होता है
}
```

`span.enter()` द्वारा लौटाया गया guard केवल वर्तमान सिंक्रोनस संदर्भ में span को सक्रिय रखता है। `call()` एक ऐसा future लौटाता है जिसे अभी poll नहीं किया गया, वास्तविक एसिंक निष्पादन बाद के poll चरण में होता है — उस समय तक guard पहले ही drop हो चुका होता है, span प्रभावी नहीं होता। TracingLayer से गुज़रने वाले सभी अनुरोध tracing आउटपुट में दिखाई नहीं देते।

**मरम्मत**:

```rust
// मरम्मत के बाद
use tracing::Instrument;

fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let fut = self.inner.call(req);
    Box::pin(fut.instrument(span))  // span future के लाइफसाइकिल से जुड़ा
}
```

`tracing::Instrument::instrument()` से span को future पर चिपकाएँ, सुनिश्चित करें कि span future के पूरे poll लाइफसाइकिल में सक्रिय रहे।

---

### Bug 2: [महत्वपूर्ण] LifecycleHook क्लोजर कार्यान्वयन दोष — on_stop कभी निष्पादित नहीं होता

- **फ़ाइल**: `ecat/src/hook.rs:14-23`、`ecat/src/lib.rs:11-16`
- **गंभीरता**: **उच्च**
- **प्रभाव**: `.on_stop()` से पंजीकृत क्लोजर hook shutdown पर कुछ भी नहीं करता

**मूल कारण विश्लेषण**:

मूल डिज़ाइन में, `on_start()` और `on_stop()` दोनों विधियाँ hook को एक ही `lifecycle_hooks` Vec में धकेलती थीं। `run()` में, सभी hooks क्रमशः `on_start()` कॉल करते हैं, shutdown पर सभी hooks क्रमशः `on_stop()` कॉल करते हैं।

समस्या `LifecycleHook` trait के क्लोजर `Fn() -> Fut` के blanket impl में है: **केवल `on_start()` कवर किया गया था, `on_stop()` trait के डिफ़ॉल्ट कार्यान्वयन (no-op) का उपयोग करता है**।

इसका मतलब है कि उपयोगकर्ता क्लोजर सिंटैक्स `.on_stop(|| async { ... })` का उपयोग करते समय, क्लोजर hooks सूची में जुड़ जाता है, लेकिन shutdown पर केवल डिफ़ॉल्ट खाली `on_stop()` निष्पादित होता है, उपयोगकर्ता का तर्क कभी नहीं चलता।

**मरम्मत (दो भाग)**:

1. **start_hooks और stop_hooks अलग करें** (`ecat/src/lib.rs`)：

```rust
// App स्ट्रक्चर — दो स्वतंत्र Vecs
pub struct App {
    start_hooks: Vec<Box<dyn LifecycleHook>>,
    stop_hooks: Vec<Box<dyn LifecycleHook>>,
    // ...
}

// on_start() → start_hooks, on_stop() → stop_hooks
pub fn on_start(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.start_hooks.push(Box::new(hook));
    self
}
pub fn on_stop(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.stop_hooks.push(Box::new(hook));
    self
}
```

2. **क्लोजर blanket impl पूरा करें** (`ecat/src/hook.rs`)：

```rust
impl<F, Fut> LifecycleHook for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Result<...>> + Send,
{
    async fn on_start(&self) -> ... { (self)().await }
    async fn on_stop(&self) -> ...  { (self)().await }  // नया
}
```

अब क्लोजर एक साथ `on_start` और `on_stop` कार्यान्वित करता है, अलग किए गए Vecs के साथ, प्रत्येक hook केवल सही लाइफसाइकिल चरण में कॉल होता है।

---

### Bug 3: [मध्यम] SqlxClient Row मान प्रकार निष्कर्षण प्राथमिकता त्रुटि

- **फ़ाइल**: `ecat-data-sqlx/src/lib.rs:53-68`
- **गंभीरता**: मध्यम
- **प्रभाव**: डेटाबेस में पूर्णांक और फ्लोट मान JSON स्ट्रिंग के रूप में निष्कर्षित होते हैं, संख्या के रूप में नहीं

**मूल कारण विश्लेषण**:

`try_get::<String>()` को पहले प्रयास में रखा गया था। अधिकांश डेटाबेस ड्राइवर संख्यात्मक कॉलम पर `try_get::<String>()` सफलतापूर्वक निष्पादित कर सकते हैं (अंतर्निहित रूपांतरण), जिससे पूर्णांक मान `42` संख्या के बजाय `"42"` के रूप में निष्कर्षित होता है।

**मरम्मत**: `try_get` प्रयास क्रम `i64 → f64 → String → Null` में समायोजित करें, संख्यात्मक प्रकारों को प्राथमिकता दें।

---

## तीन、अन्य समीक्षा निष्कर्ष (संशोधित नहीं / ज्ञात सीमाएँ)

| श्रेणी | फ़ाइल | स्पष्टीकरण | सुझाव |
|------|------|------|------|
| कार्य अधूरा | `ecat-transport-http/src/lib.rs:30` | `axum::serve().await` ब्लॉक होकर कभी लौटता नहीं, `stop()` नो-ऑप है | graceful shutdown कार्यान्वित करें |
| कार्य अधूरा | `ecat-transport-grpc/src/lib.rs:29` | वही | graceful shutdown कार्यान्वित करें |
| कार्य अधूरा | `ecat-data-sqlx/src/lib.rs:79` | `transaction()` अपर्याप्त त्रुटि लौटाता है | ट्रांज़ैक्शन समर्थन कार्यान्वित करें |
| कोड शैली | `ecat-middleware/src/logging.rs:42` | `elapsed.as_millis() as u64` u128→u64 सैद्धांतिक ट्रंकेशन | वास्तव में कोई प्रभाव नहीं |
| परीक्षण की कमी | `ecat-middleware/` | 4 Tower Services में कोई यूनिट टेस्ट नहीं | एकीकरण परीक्षण आवश्यक |
| परीक्षण की कमी | `ecat-data/` | शुद्ध trait परिभाषाएँ | वर्तमान में स्वीकार्य |
| RwLock ब्लॉकिंग | `ecat-registry/src/memory.rs` | सिंक्रोनस RwLock एसिंक संदर्भ में ब्लॉक कर सकता है | tokio::sync::RwLock पर विचार करें |

---

## चार、परीक्षण परिणाम

```
cargo test → 60 passed, 0 failed

crate के अनुसार वितरण:
  ecat                  4   (Builder/डिफ़ॉल्ट मान/लाइफसाइकिल hook)
  ecat-config           9   (env parse ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (HTTP मैपिंग/gRPC रूपांतरण/metadata/Display)
  ecat-logging          1   (init स्मोक)
  ecat-metadata         9   (संग्रहण/From HeaderMap/From MetadataMap/इटरेटर)
  ecat-metrics          2   (सिंगलटन/text panic नहीं)
  ecat-registry         5   (रजिस्टर/डिस्कवर/डी-रजिस्टर/सूची/फ़िल्टर)
  ecat-transport       11   (Context/Request/Response/Server trait)
  अन्य 8 crates         0   (शुद्ध trait/कोड जनरेशन/एकीकरण परीक्षण आवश्यक/शुद्ध प्रिंट)
```

---

## पाँच、संशोधित फ़ाइल सूची

| फ़ाइल | परिवर्तन प्रकार | परिवर्तन स्पष्टीकरण |
|------|----------|----------|
| `ecat/src/lib.rs` | Bug मरम्मत | App में start_hooks/stop_hooks अलग; AppBuilder संगत अपडेट; परीक्षण अनुकूलन |
| `ecat/src/hook.rs` | Bug मरम्मत | क्लोजर blanket impl में on_stop() कार्यान्वयन पूरा |
| `ecat-middleware/src/tracing.rs` | Bug मरम्मत | span गार्ड → `fut.instrument(span)` |
| `ecat-data-sqlx/src/lib.rs` | Bug मरम्मत | Row मान निष्कर्षण क्रम i64→f64→String→Null |

---

## छह、सारांश

इस दौर की समीक्षा में 2 उच्च-गंभीरता रनटाइम Bugs और 1 मध्यम-गंभीरता डेटा सहीता समस्या मिली:

1. **TracingLayer span अप्रभावी** — सभी अनुरोधों की अवलोकनीयता को प्रभावित करता है
2. **LifecycleHook on_stop निष्पादित नहीं** — सभी shutdown तर्क की सहीता को प्रभावित करता है
3. **Row संख्यात्मक प्रकार हानि** — डेटाबेस क्वेरी परिणामों की प्रकार सहीता को प्रभावित करता है

तीनों मुद्दे मरम्मत किए गए, मरम्मत के बाद सभी 60 परीक्षण पास, कंपाइल शून्य त्रुटि शून्य चेतावनी।

### आगे के सुझाव

- HTTP/gRPC server के लिए graceful shutdown कार्यान्वित करें
- `ecat-middleware` के लिए एकीकरण परीक्षण जोड़ें (mock Service + span/टाइमआउट/रिकवरी व्यवहार सत्यापन)
- `ecat-data-sqlx` के लिए एकीकरण परीक्षण जोड़ें (SQLite इन-मेमोरी डेटाबेस उपयोग करके)
- `ecat-registry/memory.rs` के सिंक्रोनस RwLock को `tokio::sync::RwLock` से बदलें
