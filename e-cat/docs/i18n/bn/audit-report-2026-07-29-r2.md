<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat কোড রিভিউ রিপোর্ট (দ্বিতীয় রাউন্ড)

**তারিখ**: 2026-07-29  
**ব্রাঞ্চ**: main  
**প্রকল্প**: e-cat (Rust workspace, 17টি crate)

---

## এক、রিভিউ সারাংশ

প্রথম রাউন্ডের clippy মেরামত ও টেস্ট সংযোজনের ভিত্তিতে, এই রাউন্ডে গভীর কোড লজিক রিভিউ করা হয়েছে, রানটাইম সঠিকতা、কনকারেন্সি সেফটি、API সেমান্টিক ধারাবাহিকতার উপর ফোকাস। মোট 32টি সোর্স ফাইল রিভিউ করা হয়েছে।

### ভেরিফিকেশন বেসলাইন

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

---

## দুই、আবিষ্কৃত বাগ ও মেরামত

### বাগ 1：[গুরুত্বপূর্ণ] TracingLayer span guard লাইফসাইকেল ভুল

- **ফাইল**: `ecat-middleware/src/tracing.rs:37`
- **গুরুতরতা**: **উচ্চ**
- **প্রভাব**: TracingLayer-এর মধ্য দিয়ে যাওয়া সব রিকোয়েস্ট tracing span-এর আওতায় আসবে না

**রুট কজ বিশ্লেষণ**:

```rust
// মেরামতের আগে
fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let _guard = span.enter();  // guard call() ফেরার সময় drop হয়
    let fut = self.inner.call(req);
    Box::pin(fut)               // future পরবর্তী poll-এ এক্সিকিউট হয়
}
```

`span.enter()` ফেরত দেওয়া guard শুধুমাত্র বর্তমান সিঙ্ক্রোনাস কনটেক্সটে span সক্রিয় রাখে। `call()` ফেরত দেয় এখনও poll না হওয়া future, আসল অ্যাসিংক এক্সিকিউশন পরবর্তী poll পর্যায়ে ঘটে — তখন guard ইতিমধ্যে drop হয়ে গেছে, span কার্যকর হবে না। TracingLayer-এর মধ্য দিয়ে যাওয়া সব রিকোয়েস্ট tracing আউটপুটে দেখা যাবে না।

**মেরামত**:

```rust
// মেরামতের পরে
use tracing::Instrument;

fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let fut = self.inner.call(req);
    Box::pin(fut.instrument(span))  // span future-এর লাইফসাইকেলে সংযুক্ত
}
```

`tracing::Instrument::instrument()` দিয়ে span-কে future-এর সাথে সংযুক্ত করা হয়, নিশ্চিত করে span future-এর সম্পূর্ণ poll লাইফসাইকেল জুড়ে সক্রিয় থাকে।

---

### বাগ 2：[গুরুত্বপূর্ণ] LifecycleHook ক্লোজার ইমপ্লিমেন্টেশন ত্রুটি — on_stop কখনো চালানো হয় না

- **ফাইল**: `ecat/src/hook.rs:14-23`、`ecat/src/lib.rs:11-16`
- **গুরুতরতা**: **উচ্চ**
- **প্রভাব**: `.on_stop()` দিয়ে রেজিস্টার করা ক্লোজার hook shutdown-এ কিছুই করে না

**রুট কজ বিশ্লেষণ**:

আসল ডিজাইনে, `on_start()` ও `on_stop()` মেথড দুটোই hook-কে একই `lifecycle_hooks` Vec-এ পুশ করত। `run()`-এ, সব hook ক্রমান্বয়ে `on_start()` কল করে, shutdown-এ সব hook ক্রমান্বয়ে `on_stop()` কল করে।

সমস্যাটি `LifecycleHook` trait-এর ক্লোজার `Fn() -> Fut`-এর blanket impl-এ: **শুধুমাত্র `on_start()` কভার করে, `on_stop()` trait-এর ডিফল্ট ইমপ্লিমেন্টেশন (no-op) ব্যবহার করে**।

এর অর্থ, ব্যবহারকারী ক্লোজার সিনট্যাক্স `.on_stop(|| async { ... })` ব্যবহার করলে, ক্লোজার hooks তালিকায় যোগ হলেও, shutdown-এ শুধুমাত্র ডিফল্ট খালি `on_stop()` চালানো হবে, ব্যবহারকারীর লজিক কখনো চলবে না।

**মেরামত (দুই অংশ)**:

1. **start_hooks ও stop_hooks আলাদা করা**（`ecat/src/lib.rs`）：

```rust
// App স্ট্রাক্ট — দুটি স্বাধীন Vec
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

2. **ক্লোজার blanket impl সম্পূর্ণ করা**（`ecat/src/hook.rs`）：

```rust
impl<F, Fut> LifecycleHook for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Result<...>> + Send,
{
    async fn on_start(&self) -> ... { (self)().await }
    async fn on_stop(&self) -> ...  { (self)().await }  // নতুন
}
```

এখন ক্লোজার একসাথে `on_start` ও `on_stop` ইমপ্লিমেন্ট করে, আলাদা Vec-এর সাথে, প্রতিটি hook শুধুমাত্র সঠিক লাইফসাইকেল পর্যায়ে কল হয়।

---

### বাগ 3：[মাঝারি] SqlxClient Row মান টাইপ এক্সট্রাকশন অগ্রাধিকার ভুল

- **ফাইল**: `ecat-data-sqlx/src/lib.rs:53-68`
- **গুরুতরতা**: মাঝারি
- **প্রভাব**: ডেটাবেসের পূর্ণসংখ্যা ও ফ্লোট মান JSON স্ট্রিং হিসেবে এক্সট্রাক্ট হবে, সংখ্যা নয়

**রুট কজ বিশ্লেষণ**:

`try_get::<String>()` প্রথম অবস্থানে চেষ্টা করা হয়। বেশিরভাগ ডেটাবেস ড্রাইভার সংখ্যা কলামে `try_get::<String>()` সফলভাবে চালাতে পারে (ইমপ্লিসিট কনভার্সন), ফলে পূর্ণসংখ্যা `42`-কে `"42"` হিসেবে এক্সট্রাক্ট করা হয়, `42` নয়।

**মেরামত**: `try_get` চেষ্টার ক্রম `i64 → f64 → String → Null`-এ সামঞ্জস্য করা হয়েছে, সংখ্যা টাইপ অগ্রাধিকার সংরক্ষণ করে।

---

## তিন、অন্যান্য রিভিউ আবিষ্কার (অপরিবর্তিত / পরিচিত সীমাবদ্ধতা)

| ক্যাটাগরি | ফাইল | ব্যাখ্যা | পরামর্শ |
|------|------|------|------|
| ফিচার অসম্পূর্ণ | `ecat-transport-http/src/lib.rs:30` | `axum::serve().await` ব্লক করে কখনো ফেরে না, `stop()` খালি অপারেশন | graceful shutdown বাস্তবায়ন |
| ফিচার অসম্পূর্ণ | `ecat-transport-grpc/src/lib.rs:29` | একই | graceful shutdown বাস্তবায়ন |
| ফিচার অসম্পূর্ণ | `ecat-data-sqlx/src/lib.rs:79` | `transaction()` অবাস্তবায়িত এরর ফেরত দেয় | ট্রানজেকশন সাপোর্ট বাস্তবায়ন |
| কোড স্টাইল | `ecat-middleware/src/logging.rs:42` | `elapsed.as_millis() as u64` u128→u64 তাত্ত্বিক ট্রাঙ্কেশন | বাস্তবে কোনো প্রভাব নেই |
| টেস্ট ঘাটতি | `ecat-middleware/` | 4টি Tower Service-এর ইউনিট টেস্ট নেই | ইন্টিগ্রেশন টেস্ট প্রয়োজন |
| টেস্ট ঘাটতি | `ecat-data/` | বিশুদ্ধ trait সংজ্ঞা | বর্তমানে গ্রহণযোগ্য |
| RwLock ব্লকিং | `ecat-registry/src/memory.rs` | সিঙ্ক্রোনাস RwLock অ্যাসিংক কনটেক্সটে ব্লক করতে পারে | tokio::sync::RwLock বিবেচনা |

---

## চার、টেস্ট ফলাফল

```
cargo test → 60 passed, 0 failed

crate অনুযায়ী বণ্টন:
  ecat                  4   (Builder/ডিফল্ট মান/লাইফসাইকেল hook)
  ecat-config           9   (env parse ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (HTTP ম্যাপিং/gRPC রূপান্তর/metadata/Display)
  ecat-logging          1   (init স্মোক)
  ecat-metadata         9   (存取/From HeaderMap/From MetadataMap/ইটারেটর)
  ecat-metrics          2   (সিঙ্গেলটন/text panic নয়)
  ecat-registry         5   (রেজিস্ট্রি/ডিসকভারি/ডিরেজিস্টার/তালিকা/ফিল্টার)
  ecat-transport       11   (Context/Request/Response/Server trait)
  অন্যান্য 8 crate          0   (বিশুদ্ধ trait/কোড জেনারেশন/ইন্টিগ্রেশন টেস্ট প্রয়োজন/বিশুদ্ধ প্রিন্ট)
```

---

## পাঁচ、পরিবর্তিত ফাইল তালিকা

| ফাইল | পরিবর্তন টাইপ | পরিবর্তন ব্যাখ্যা |
|------|----------|----------|
| `ecat/src/lib.rs` | বাগ মেরামত | App-এ start_hooks/stop_hooks আলাদা; AppBuilder সামঞ্জস্যপূর্ণ আপডেট; টেস্ট অভিযোজন |
| `ecat/src/hook.rs` | বাগ মেরামত | ক্লোজার blanket impl-এ on_stop() ইমপ্লিমেন্টেশন সম্পূর্ণ |
| `ecat-middleware/src/tracing.rs` | বাগ মেরামত | span guard → `fut.instrument(span)` |
| `ecat-data-sqlx/src/lib.rs` | বাগ মেরামত | Row মান এক্সট্রাকশন ক্রম i64→f64→String→Null |

---

## ছয়、সারাংশ

এই রাউন্ডের রিভিউতে 2টি উচ্চ-গুরুতরতা রানটাইম বাগ এবং 1টি মাঝারি-গুরুতরতা ডেটা সঠিকতা সমস্যা আবিষ্কৃত হয়েছে:

1. **TracingLayer span নিষ্ক্রিয়** — সব রিকোয়েস্টের অবজারভেবিলিটি প্রভাবিত
2. **LifecycleHook on_stop চালানো হয় না** — সব shutdown লজিকের সঠিকতা প্রভাবিত
3. **Row সংখ্যা টাইপ হারানো** — ডেটাবেস কোয়েরি ফলাফলের টাইপ সঠিকতা প্রভাবিত

তিনটি সমস্যাই মেরামত করা হয়েছে, মেরামতের পরে সব 60টি টেস্ট পাস, কম্পাইল শূন্য এরর শূন্য ওয়ার্নিং।

### পরবর্তী পরামর্শ

- HTTP/gRPC সার্ভারের জন্য graceful shutdown বাস্তবায়ন
- `ecat-middleware`-এর জন্য ইন্টিগ্রেশন টেস্ট যোগ (mock Service + span/টাইমআউট/রিকভারি আচরণ যাচাই)
- `ecat-data-sqlx`-এর জন্য ইন্টিগ্রেশন টেস্ট যোগ (SQLite মেমরি ডেটাবেস ব্যবহার)
- `ecat-registry/memory.rs`-এর সিঙ্ক্রোনাস RwLock `tokio::sync::RwLock` দিয়ে প্রতিস্থাপন
