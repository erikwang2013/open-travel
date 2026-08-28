<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# تقرير مراجعة الكود لـ e-cat (الجولة الثانية)

**التاريخ**: 2026-07-29  
**الفرع**: main  
**المشروع**: e-cat (Rust workspace، 17 crate)

---

## ١. ملخص المراجعة

بناءً على إصلاحات clippy واستكمال الاختبارات في الجولة الأولى، أجرت هذه الجولة مراجعة منطقية عميقة للكود، مع التركيز على صحة وقت التشغيل، وسلامة التزامن، واتساق دلالات واجهات API. رُوجعت 32 ملف مصدر إجمالًا.

### خط الأساس للتحقق

```
cargo check  → 0 errors, 0 warnings
cargo clippy → 0 warnings (all targets)
cargo test   → 60 passed, 0 failed
```

---

## ٢. الأخطاء المكتشفة وإصلاحاتها

### الخطأ 1: [حرج] خطأ دورة حياة حارس span في TracingLayer

- **الملف**: `ecat-middleware/src/tracing.rs:37`
- **الخطورة**: **عالية**
- **التأثير**: لن تغطي spans التتبع أي طلب يمر عبر TracingLayer

**تحليل السبب الجذري**:

```rust
// قبل الإصلاح
fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let _guard = span.enter();  // guard يُسقط عند عودة call()
    let fut = self.inner.call(req);
    Box::pin(fut)               // future يُنفَّذ فقط في مرحلة poll اللاحقة
}
```

يبقي guard الذي يُرجعه `span.enter()` الـ span نشطًا في السياق المتزامن الحالي فقط. تُرجع `call()` future لم يُفحص بعد، ويحدث التنفيذ غير المتزامن الفعلي في مرحلة poll لاحقة — وبحلولها يكون guard قد سقط منذ زمن، فلا يعمل الـ span. لن تظهر أي طلبات تمر عبر TracingLayer في مخرجات tracing.

**الإصلاح**:

```rust
// بعد الإصلاح
use tracing::Instrument;

fn call(&mut self, req: Req) -> Self::Future {
    let span = tracing::info_span!("request");
    let fut = self.inner.call(req);
    Box::pin(fut.instrument(span))  // span مرتبط بدورة حياة future
}
```

يُرافق استخدام `tracing::Instrument::instrument()` الـ span مع الـ future، ما يضمن بقاء الـ span نشطًا طوال دورة حياة poll الكاملة للـ future.

---

### الخطأ 2: [حرج] عيب في تنفيذ إغلاق LifecycleHook — on_stop لا يُنفَّذ أبدًا

- **الملف**: `ecat/src/hook.rs:14-23`، `ecat/src/lib.rs:11-16`
- **الخطورة**: **عالية**
- **التأثير**: لا تفعل خطافات الإغلاق المسجلة عبر `.on_stop()` شيئًا عند إيقاف التشغيل

**تحليل السبب الجذري**:

في التصميم الأصلي، تدفع كل من `on_start()` و`on_stop()` الخطاف في نفس Vec `lifecycle_hooks`. أثناء `run()`، تُستدعى `on_start()` لكل الخطافات تباعًا، وعند الإيقاف تُستدعى `on_stop()` للكل تباعًا.

المشكلة في التنفيذ الشامل (blanket impl) لـ trait `LifecycleHook` على الإغلاق `Fn() -> Fut`: **يغطي `on_start()` فقط، بينما يستخدم `on_stop()` التنفيذ الافتراضي للـ trait (no-op)**.

يعني هذا أنه عندما يستخدم المستخدم صيغة الإغلاق `.on_stop(|| async { ... })`، يُضاف الإغلاق إلى قائمة الخطافات فعلًا، لكن عند الإيقاف يُنفَّذ فقط `on_stop()` الافتراضي الفارغ، فلا يعمل منطق المستخدم أبدًا.

**الإصلاح (جزآن)**:

1. **فصل start_hooks عن stop_hooks** (`ecat/src/lib.rs`):

```rust
// بنية App — Vec مستقلان
pub struct App {
    start_hooks: Vec<Box<dyn LifecycleHook>>,
    stop_hooks: Vec<Box<dyn LifecycleHook>>,
    // ...
}

// on_start() → start_hooks، on_stop() → stop_hooks
pub fn on_start(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.start_hooks.push(Box::new(hook));
    self
}
pub fn on_stop(mut self, hook: impl LifecycleHook + 'static) -> Self {
    self.stop_hooks.push(Box::new(hook));
    self
}
```

2. **استكمال blanket impl للإغلاق** (`ecat/src/hook.rs`):

```rust
impl<F, Fut> LifecycleHook for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Result<...>> + Send,
{
    async fn on_start(&self) -> ... { (self)().await }
    async fn on_stop(&self) -> ...  { (self)().await }  // جديد
}
```

الآن ينفذ الإغلاق كلا `on_start` و`on_stop`، ومع فصل الـ Vec، يُستدعى كل خطاف في مرحلة دورة الحياة الصحيحة فقط.

---

### الخطأ 3: [متوسط] أولوية خاطئة في استخراج أنواع قيم Row في SqlxClient

- **الملف**: `ecat-data-sqlx/src/lib.rs:53-68`
- **الخطورة**: متوسطة
- **التأثير**: تُستخرج القيم الصحيحة والعشرية في قاعدة البيانات كسلاسل JSON بدلًا من أرقام

**تحليل السبب الجذري**:

وُضع `try_get::<String>()` في المحاولة الأولى. ينجح معظم مشغلات قواعد البيانات في تنفيذ `try_get::<String>()` على الأعمدة الرقمية (تحويل ضمني)، فيُستخرج العدد الصحيح `42` كـ `"42"` بدلًا من `42`.

**الإصلاح**: ضبط ترتيب محاولات `try_get` إلى `i64 → f64 → String → Null`، مع إعطاء الأولوية للحفاظ على الأنواع الرقمية.

---

## ٣. نتائج مراجعة أخرى (لم تُعدَّل / قيود معروفة)

| الفئة | الملف | الوصف | الاقتراح |
|------|------|------|------|
| وظيفة غير مكتملة | `ecat-transport-http/src/lib.rs:30` | `axum::serve().await` يمنع العودة إلى الأبد، و`stop()` عملية فارغة | تنفيذ graceful shutdown |
| وظيفة غير مكتملة | `ecat-transport-grpc/src/lib.rs:29` | كما سبق | تنفيذ graceful shutdown |
| وظيفة غير مكتملة | `ecat-data-sqlx/src/lib.rs:79` | تُرجع `transaction()` خطأ غير منفَّذ | تنفيذ دعم المعاملات |
| نمط الكود | `ecat-middleware/src/logging.rs:42` | `elapsed.as_millis() as u64` اقتطاع نظري u128→u64 | لا تأثير فعلي |
| نقص اختبارات | `ecat-middleware/` | لا اختبارات وحدة لـ 4 Tower Services | تتطلب اختبارات تكامل |
| نقص اختبارات | `ecat-data/` | تعريفات traits خالصة | مقبول حاليًا |
| حجب RwLock | `ecat-registry/src/memory.rs` | قد يحجب RwLock المتزامن في سياقات غير متزامنة | النظر في tokio::sync::RwLock |

---

## ٤. نتائج الاختبارات

```
cargo test → 60 passed, 0 failed

التوزيع حسب crate:
  ecat                  4   (Builder/قيم افتراضية/خطافات دورة الحياة)
  ecat-config           9   (env parse ×4 + config ×5)
  ecat-encoding        15   (JSON/Proto/CodecBox/codec_for/from_ct)
  ecat-errors           4   (تعيين HTTP/تحويل gRPC/metadata/Display)
  ecat-logging          1   (دخان init)
  ecat-metadata         9   (تخزين/From HeaderMap/From MetadataMap/مكرر)
  ecat-metrics          2   (مفرد/text دون panic)
  ecat-registry         5   (تسجيل/اكتشاف/إلغاء/قائمة/فلترة)
  ecat-transport       11   (Context/Request/Response/trait Server)
  8 crates أخرى        0   (traits خالصة/توليد كود/يتطلب تكامل/طباعة خالصة)
```

---

## ٥. قائمة الملفات المعدلة

| الملف | نوع التغيير | وصف التغيير |
|------|----------|----------|
| `ecat/src/lib.rs` | إصلاح خطأ | فصل App إلى start_hooks/stop_hooks؛ تحديث AppBuilder المقابل؛ تكييف الاختبارات |
| `ecat/src/hook.rs` | إصلاح خطأ | استكمال تنفيذ on_stop() في blanket impl للإغلاق |
| `ecat-middleware/src/tracing.rs` | إصلاح خطأ | حارس span → `fut.instrument(span)` |
| `ecat-data-sqlx/src/lib.rs` | إصلاح خطأ | ترتيب استخراج قيم Row: i64→f64→String→Null |

---

## ٦. الخلاصة

اكتشفت هذه الجولة خطأين في وقت التشغيل بخطورة عالية ومشكلة صحة بيانات بخطورة متوسطة:

1. **تعطل span في TracingLayer** — يؤثر على قابلية مراقبة جميع الطلبات
2. **عدم تنفيذ LifecycleHook on_stop** — يؤثر على صحة جميع منطقيات الإيقاف
3. **فقدان الأنواع الرقمية للـ Row** — يؤثر على صحة أنواع نتائج استعلامات قاعدة البيانات

أُصلحت المشكلات الثلاث، وبعد الإصلاح اجتازت جميع الاختبارات الستين، والترجمة صفر أخطاء صفر تحذيرات.

### توصيات لاحقة

- تنفيذ graceful shutdown لخادمي HTTP/gRPC
- إضافة اختبارات تكامل لـ `ecat-middleware` (mock Service + التحقق من سلوك span/المهلة/الاسترداد)
- إضافة اختبارات تكامل لـ `ecat-data-sqlx` (باستخدام قاعدة بيانات SQLite في الذاكرة)
- استبدال RwLock المتزامن في `ecat-registry/memory.rs` بـ `tokio::sync::RwLock`
