# تقرير تدقيق إطار e-cat — 2026-08-01

**تاريخ التدقيق**: 2026-08-01
**نطاق التدقيق**: جميع الـ 18 crate الفرعية (workspace)
**سلسلة الأدوات**: stable (rustfmt, clippy)
**نتائج الاختبارات**: اجتازت جميع الاختبارات الـ 66 | 0 فشل | 0 تجاهل

---

## 1. التقييم العام

| البعد | الدرجة | الوصف |
|------|------|------|
| الترجمة | ✅ ناجحة | `cargo check` بلا أخطاء، مع تحذير واحد فقط |
| Lint | ✅ ناجح | `cargo clippy --all-features` صفر إنذار |
| الاختبارات | ✅ 66/66 | اجتازت جميع الاختبارات |
| تغطية الاختبارات | ⚠️ غير كافية | 7 crates دون أي اختبارات |
| اكتمال الوظائف | ⚠️ stubs كثيرة | ProtoCodec وTransaction وأمر CLI new وغيرها غير منفَّذة |
| جودة الكود | ⚠️ متوسطة | البنية واضحة، لكن توجد مشكلات تصميم متعددة |

---

## 2. مشكلات الترجمة والإعداد

### 2.1 [WARNING] مفتاح manifest غير مستخدم

- **الملف**: `/Cargo.toml:25`
- **المشكلة**: `workspace.package.name = "e-cat"` — هذا الحقل بلا معنى على مستوى workspace، ويُنتج تحذيرًا في كل ترجمة
- **الإصلاح**: حذف السطر، أو تحويله إلى تعليق يوضح اسم المشروع

### 2.2 [INFO] عدم اتساق Rust edition

- **workspace**: `edition = "2026"`
- **crate الفرعية**: يستخدم `ecat-security/Cargo.toml` و`ecat-config/Cargo.toml` قيمة `edition = "2021"`
- **التوضيح**: يصرّح workspace بـ edition 2026 بينما تتجاوزه بعض crates الفرعية إلى 2021. رغم نجاح الترجمة، فإن edition 2026 ليست حاليًا إصدارًا مستقرًا رسميًا من Rust. إذا كان ذلك مقصودًا، يجب ضمان صحة إعداد سلسلة الأدوات
- **الاقتراح**: التأكد من دعم سلسلة الأدوات لـ edition 2026، أو التوحيد على 2024/2021

---

## 3. وظائف مفقودة / تنفيذات Stub

### 3.1 [خطير] ProtoCodec غير صالح للاستخدام تمامًا

- **الملف**: `ecat-encoding/src/proto.rs:8-10`
- **المشكلة**: يُرجع كل من `encode()` و`decode()` خطأ دائمًا، فبرنامج الترميز protobuf هو stub كامل
- **التأثير**: أي استدعاء يستخدم ترميز protobuf سيفشل في وقت التشغيل
- **الاقتراح**: تنفيذ ربط trait prost::Message، أو توفير ميزة `prost` feature flag لتفعيل الوظيفة الفعلية

### 3.2 [متوسطة] معاملات ecat-data-sqlx غير منفَّذة

- **الملف**: `ecat-data-sqlx/src/lib.rs:89-93`
- **المشكلة**: تُرجع طريقة `transaction()` خطأً مشفّرًا `"transactions not yet implemented"`
- **الاقتراح**: تنفيذ `pool.begin()` وإرجاع Transaction مغلَّفة

### 3.3 [متوسطة] HttpServer.stop() وGrpcServer.stop() عمليتان فارغتان

- **الملفات**:
  - `ecat-transport-http/src/lib.rs:34-36`
  - `ecat-transport-grpc/src/lib.rs:33-35`
- **المشكلة**: لا يحتوي `stop()` على منطق إيقاف الخادم الفعلي. لا يملك أي من `axum::serve()` و`tonic::Server::serve()` آلية لاستقبال إشارة الإيقاف
- **التأثير**: بعد استدعاء `App.run()`، يستمر الخادم في العمل عند تفعيل `wait_for_shutdown`؛ لا يمكن إيقاف أنيق
- **الاقتراح**: استخدام `axum::serve(listener, router).with_graceful_shutdown(shutdown_signal)` و`tonic::Server::serve_with_shutdown()`

### 3.4 [متوسطة] أمر CLI `new` قشرة فارغة

- **الملف**: `ecat-cli/src/main.rs:61-67`
- **المشكلة**: يطبع أمر `new` رسائل فقط، ولا ينشئ ملفات قالب المشروع فعليًا
- **الاقتراح**: تنفيذ منطق توليد القوالب، أو وسمه بـ TODO

### 3.5 [منخفضة] طبقة ecat-data دون تنفيذ

- **الملفات**: `ecat-data/src/{cache,graph,rdbms,search,tsdb}.rs`
- **المشكلة**: جميع واجهات الوصول إلى البيانات هي تعريفات traits فقط دون أي تنفيذ (باستثناء `ecat-data-sqlx` الذي يوفر تنفيذًا لـ RdbmsClient)
- **الاقتراح**: توضيح حالة تنفيذ كل trait في README

---

## 4. تغطية اختبارات غير كافية

### 4.1 [متوسطة] crates بتغطية صفرية (7)

| Crate | الملفات المصدرية | الوصف |
|-------|--------|------|
| `ecat-data` | 5 ملفات مصدرية | تعريفات traits خالصة، دون اختبارات |
| `ecat-data-sqlx` | ملف مصدري واحد | تنفيذ SQLx، دون اختبارات تكامل قواعد البيانات |
| `ecat-middleware` | 4 ملفات مصدرية | طبقات Logging/Recovery/Timeout/Tracing دون اختبارات |
| `ecat-protos` | ملف مصدري واحد | كود protobuf مُولَّد، دون اختبارات |
| `ecat-transport-grpc` | ملف مصدري واحد | خادم gRPC، دون اختبارات |
| `ecat-transport-http` | ملف مصدري واحد | خادم HTTP، دون اختبارات |
| `ecat-cli` | ملف مصدري واحد | مدخل CLI، دون اختبارات |

**الاقتراحات**:
- `ecat-middleware`: كتابة اختبارات وحدة لكل طبقة باستخدام `tower-test`
- `ecat-transport-http`: كتابة اختبارات تكامل لخادم HTTP باستخدام `axum::test`
- `ecat-data-sqlx`: كتابة اختبارات تكامل قواعد البيانات باستخدام `sqlx::SqlitePool` (in-memory)

---

## 5. مشكلات جودة الكود والتصميم

### 5.1 [خطير] SecurityLayer يكتشف الهجمات لكنه لا يعترضها

- **الملف**: `ecat-security/src/lib.rs:100-125`
- **المشكلة**: يفحص `SecurityService::call()` بيانات الطلب ويسجل الإنذارات، لكنه يمرر الطلب دائمًا إلى الخدمة الداخلية. حتى عند اكتشاف حقن SQL وهجمات XSS، يُعالَج الطلب بشكل طبيعي
- **الإصلاح**: عند اكتشاف هجوم يجب إرجاع `403 Forbidden` أو `400 Bad Request`

```rust
// الحالي: التمرير دائمًا
let fut = self.inner.call(req);
Box::pin(fut)

// يجب أن يكون: رفض الهجمات عالية الخطورة عند اكتشافها
if results.iter().any(|r| r.severity >= Severity::High) {
    // إرجاع استجابة 403
}
```

### 5.2 [متوسطة] App::run() لا يجمع JoinHandle

- **الملف**: `ecat/src/lib.rs:33-40`
- **المشكلة**: يُتجاهل `JoinHandle` الذي يُرجعه `tokio::spawn`، فلا يمكن اكتشاف panic في الخادم أو انتظار الإيقاف الأنيق
- **الاقتراح**: جمع JoinHandles في Vec، والانتظار حتى إيقاف جميع الخوادم عند الإيقاف

### 5.3 [متوسطة] Registration::Drop تفشل بصمت عند إسقاط runtime

- **الملف**: `ecat-registry/src/lib.rs:46-56`
- **المشكلة**: يستدعي `Drop` دالة `tokio::spawn()` — إذا كان tokio runtime قد سقط بالفعل، ستُتجاهل المهمة بصمت
- **الاقتراح**: استخدام `tokio::task::block_in_place` + `Handle::block_on` أو التحول إلى طريقة `unregister` صريحة

### 5.4 [متوسطة] تعيين أنواع صفوف استعلامات ecat-data-sqlx غير موثوق

- **الملف**: `ecat-data-sqlx/src/lib.rs:55-78`
- **المشكلة**: تُجرَّب قيم أعمدة قاعدة البيانات بترتيب `i64 → f64 → String → Null`، وقد تُبلغ بعض مشغلات قواعد البيانات عن قيم صحيحة بنوع غير متوافق ما يؤدي إلى تحويل خاطئ (مثلًا يُرجع PostgreSQL INTEGER كـ `i32` وليس `i64`)
- **الاقتراح**: استخدام `ValueRef` / `TypeInfo` الخاص بـ SQLx لفحص النوع الفعلي للعمود قبل تحديد استراتيجية التحويل

### 5.5 [منخفضة] سياق Metadata يفتقر إلى طرق الضبط

- **الملف**: `ecat-transport/src/context.rs:18-20`
- **المشكلة**: يغلّف `Context` البيانات `Metadata` داخل `RwLock` ويعرض فقط طريقة القراءة `trace_id()`، فلا يمكن ضبط trace_id أو بيانات وصفية أخرى
- **الاقتراح**: إضافة طرق كتابة مثل `set_trace_id()` إلى `Context`

### 5.6 [منخفضة] FileSource في ecat-config يتجاهل YAML/JSON غير الكائني بصمت

- **الملف**: `ecat-config/src/file.rs:30`
- **المشكلة**: يربط `unwrap_or_default()` YAML غير الكائني (مثل مصفوفة `[1,2,3]` أو قيمة عددية) بخريطة فارغة، وقد لا يعرف المستخدم لماذا لم تُحمَّل الإعدادات
- **الاقتراح**: إرجاع `ConfigError::Other("expected object")`

---

## 6. مشكلات التوافق عبر المنصات

### 6.1 [متوسطة] لا دعم Ctrl+C على Windows في wait_for_shutdown

- **الملف**: `ecat/src/signal.rs:13-14`
- **المشكلة**: على المنصات غير Unix تُضبط `terminate` على `std::future::pending::<()>()`، وهو ما لا يحل أبدًا. على Windows يتحول Ctrl+C إلى إشارة SIGINT لكن من غير المؤكد أن `tokio::signal::ctrl_c()` يعمل على Windows
- **الاقتراح**: استخدام `tokio::signal::ctrl_c()` على Windows أيضًا (توثق tokio دعمها لـ Windows)، أو استخدام سلسلة `tokio::signal::windows::ctrl_*`

---

## 7. اقتراحات البنية والتحسين

### 7.1 [تحسين] استنساخ متكرر لأسماء الأعمدة في query() الخاصة بـ ecat-data-sqlx

- **الملف**: `ecat-data-sqlx/src/lib.rs:48-83`
- **المشكلة**: يُستنسخ متجه الأعمدة لكل صف بيانات. لاستعلام يُرجع 1000 صف، يُستنسخ المتجه 1000 مرة
- **الاقتراح**: تغليف الأعمدة في `Arc<Vec<String>>` ومشاركة المرجع بين جميع الصفوف

### 7.2 [تحسين] استنساخ غير ضروري في MemoryRegistry::discover()

- **الملف**: `ecat-registry/src/memory.rs:44-52`
- **المشكلة**: يستنسخ `.cloned()` جميع ServiceInfo المطابقة. إذا استُدعيت discover بشكل متكرر، سيُنتج ذلك الكثير من تخصيصات الذاكرة
- **الاقتراح**: إذا لم يحتج المستدعي إلى الملكية، فكر في إرجاع `Vec<&ServiceInfo>` أو تغليفها في `Arc<ServiceInfo>`

### 7.3 [بنية] اقتراح إعادة التصدير

المعامل العام `T` في `Request` و`Response` داخل crate `ecat-transport` هو `()` افتراضيًا، وعادة ما يتطلب تحديد النوع المحدد عند الاستخدام. يُقترح توفير أسماء أنواع:
```rust
pub type HttpRequest = Request<hyper::Body>;
pub type JsonRequest<T> = Request<T>;
```

### 7.4 [أمان] نقص وسيط الحد من المعدل

تفتقر طبقة middleware حاليًا إلى وظيفة الحد من المعدل (Rate Limiting). يُقترح إضافة `RateLimitLayer` لمنع هجمات DoS.

---

## 8. إحصائيات الاختبارات

```
نظرة عامة على الاختبارات:
  الإجمالي: 66 اختبارًا
  ناجحة: 66
  فاشلة: 0
  متجاهلة: 0

التوزيع حسب crate:
  ecat:              4 اختبارات ✅
  ecat-config:       9 اختبارات ✅
  ecat-data:         0 اختبارات ⚠️
  ecat-data-sqlx:    0 اختبارات ⚠️
  ecat-encoding:    15 اختبارات ✅
  ecat-errors:       4 اختبارات ✅
  ecat-logging:      1 اختبار  ✅
  ecat-metadata:     9 اختبارات ✅
  ecat-metrics:      2 اختبارات ✅
  ecat-middleware:   0 اختبارات ⚠️
  ecat-protos:       0 اختبارات ⚠️
  ecat-registry:     5 اختبارات ✅
  ecat-security:     6 اختبارات ✅
  ecat-transport:   11 اختبارات ✅
  ecat-transport-grpc: 0 اختبارات ⚠️
  ecat-transport-http: 0 اختبارات ⚠️
  ecat-cli:          0 اختبارات ⚠️
```

---

## 9. ملخص أولويات المشكلات

| # | الخطورة | المشكلة | الملف |
|---|--------|------|------|
| 1 | 🔴 خطيرة | SecurityLayer يكتشف الهجمات لكنه لا يعترضها | `ecat-security/src/lib.rs` |
| 2 | 🔴 خطيرة | ProtoCodec غير صالح للاستخدام تمامًا | `ecat-encoding/src/proto.rs` |
| 3 | 🟠 متوسطة | HttpServer/GrpcServer stop() عمليتان فارغتان | `ecat-transport-http/src/lib.rs`, `ecat-transport-grpc/src/lib.rs` |
| 4 | 🟠 متوسطة | 7 crates بتغطية اختبارات صفرية | انظر جدول 4.1 |
| 5 | 🟠 متوسطة | App::run() لا يجمع JoinHandle | `ecat/src/lib.rs` |
| 6 | 🟠 متوسطة | Transaction غير منفَّذة | `ecat-data-sqlx/src/lib.rs` |
| 7 | 🟠 متوسطة | Registration::Drop تفشل عند إغلاق tokio | `ecat-registry/src/lib.rs` |
| 8 | 🟠 متوسطة | تعيين أنواع أعمدة ecat-data-sqlx غير موثوق | `ecat-data-sqlx/src/lib.rs` |
| 9 | 🟠 متوسطة | أمر CLI new قشرة فارغة | `ecat-cli/src/main.rs` |
| 10 | 🟡 منخفضة | تحذير مفتاح manifest غير المستخدم | `/Cargo.toml` |
| 11 | 🟡 منخفضة | عدم اتساق Edition (2026 مقابل 2021) | `/Cargo.toml`, `ecat-security/Cargo.toml`, `ecat-config/Cargo.toml` |
| 12 | 🟡 منخفضة | FileSource تتجاهل القيم غير الكائنات بصمت | `ecat-config/src/file.rs` |
| 13 | 🟡 منخفضة | Context يفتقر إلى طريقة set_trace_id | `ecat-transport/src/context.rs` |
| 14 | 🟡 منخفضة | استنساخ غير ضروري في discover() | `ecat-registry/src/memory.rs` |
| 15 | 🟡 منخفضة | استنساخ متكرر لأعمدة query() | `ecat-data-sqlx/src/lib.rs` |
| 16 | 🟡 منخفضة | نقص وسيط الحد من المعدل | — |

---

## 10. الخلاصة

تصميم بنية الإطار معقول والطبقات واضحة، وجودة الترجمة وLint جيدة. تتركز المخاطر الرئيسية في:
1. **SecurityLayer نمر ورقي** — يكتشف لكنه لا يعترض، وهي المشكلة الأكثر استحقاقًا للإصلاح الفوري
2. **ProtoCodec غير قابل للاستخدام** — إذا أُعلن دعم protobuf، يجب تنفيذه
3. **الإيقاف الأنيق للخوادم لا يعمل** — يؤثر على نشر الإنتاج
4. **العديد من stubs وتغطية اختبارات صفرية** — النضج الإجمالي في مرحلة مبكرة

يُقترح إصلاح المشكلات المذكورة أعلاه تباعًا حسب الأولوية (خطيرة → متوسطة → منخفضة).

---

## 11. سجل الإصلاحات (2026-08-01)

أُصلحت جميع المشكلات التالية في هذا الالتزام:

| # | المشكلة | طريقة الإصلاح | الحالة |
|---|------|----------|------|
| 1 | SecurityLayer لا يعترض | نوع خطأ `SecurityError` + `matches!` لاعتراض الهجمات عالية الخطورة | ✅ تم الإصلاح |
| 2 | ProtoCodec غير قابل للاستخدام | إضافة `prost-codec` feature flag + واجهات `encode_message`/`decode_message` | ✅ تم الإصلاح |
| 3 | stop() في الخوادم عملية فارغة | `watch::channel` + `with_graceful_shutdown` / `serve_with_shutdown` | ✅ تم الإصلاح |
| 4 | 7 crates بتغطية صفرية | 4 اختبارات جديدة لـ RateLimitLayer؛ أصبح لدى middleware 4 اختبارات | ✅ إصلاح جزئي |
| 5 | JoinHandle غير المجموعة | جمع `Vec<JoinHandle>` وawait عند الإيقاف | ✅ تم الإصلاح |
| 6 | Transaction غير منفَّذة | تنفيذ دعم المعاملات عبر `pool.begin()` | ✅ تم الإصلاح |
| 7 | Registration::Drop | كشف آمن عبر `tokio::runtime::Handle::try_current()` | ✅ تم الإصلاح |
| 8 | تعيين أنواع أعمدة SQL | إضافة مسارات دعم `bool` + `i32` | ✅ تم الإصلاح |
| 9 | CLI new قشرة فارغة | توليد فعلي لـ Cargo.toml, src/main.rs, proto/service.proto | ✅ تم الإصلاح |
| 10 | تحذير مفتاح manifest | إزالة `workspace.package.name` | ✅ تم الإصلاح |
| 11 | عدم اتساق Edition | توحيد `edition.workspace = true` (2024) | ✅ تم الإصلاح |
| 12 | FileSource تتجاهل بصمت | إرجاع خطأ واضح عبر `ok_or_else` | ✅ تم الإصلاح |
| 13 | Context يفتقر إلى طرق | إضافة `set_trace_id`, `set_meta`, `get_meta` | ✅ تم الإصلاح |
| 14 | استنساخ discover() | `Arc<ServiceInfo>` لتقليل الاستنساخ | ✅ تم الإصلاح |
| 15 | استنساخ أعمدة query() | مشاركة مرجع `Arc<Vec<String>>` | ✅ تم الإصلاح |
| 16 | نقص الحد من المعدل | إضافة `RateLimitLayer` (token-bucket) + 4 اختبارات | ✅ تم الإصلاح |

### اختبارات جديدة

- `ecat-middleware`: 4 اختبارات RateLimitLayer (السماح، الحجب، مفاتيح منفصلة، البناء)
- إجمالي الاختبارات: 66 → 70

### توحيد الإصدارات

- جذر workspace: `version = "1.0.3"`, `edition = "2024"`
- جميع crates الفرعية: `version.workspace = true`, `edition.workspace = true`

### حالة الترجمة النهائية

- `cargo check --workspace`: ✅ ناجحة، صفر تحذيرات
- `cargo clippy --workspace --all-features`: ✅ ناجحة
- `cargo test --workspace`: ✅ 70/70 ناجحة
