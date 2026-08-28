# تقرير الاختبارات — 2026-08-26

استكمال شامل لاختبارات الوحدة (تغطية كاملة لـ 51 crate)، عبر 4 مجموعات من مهندسي اختبار Rust كبار بالتوازي.

## نظرة عامة

| المجموعة | crates | سابق | جديد | حالي | البوابة |
|---|---|---|---|---|---|
| core/الإطار | 12 | 102 | +40 | 142 | ✅ test أخضر بالكامل + clippy 0 تحذير |
| data | 14 | 87 | +66 | 153 | ✅ كما سبق |
| mq/transport | 12 | 82 | +54 | 136 | ✅ كما سبق |
| طبقة تطبيق app | 13 | ~178 | +46 | ~224 | ✅ كما سبق |
| **الإجمالي** | **51** | **~449** | **+206** | **~655** | ✅ |

ملاحظة: أعداد طبقة التطبيق السابقة تشمل ecat-auth 24 / ecat-graphql 35 / ecat-bench 11 / ecat-scheduler 6 / ecat-events 7 / ecat-cli 12 / ecat-middleware 34 / ecat-circuit-breaker 10 / ecat-health 4 / ecat-client 7 / ecat-security 12 / ecat-versioning 4 / ecat 4. اجتاز كل crate اختبارات `cargo test -p` المستقلة و`cargo clippy -p --all-targets -- -D warnings`، مع عزل CARGO_TARGET_DIR للتشغيل المتوازي.

## التفاصيل لكل crate

### مجموعة core/الإطار (test-core، +40)

| crate | سابق→جديد | نقاط التغطية |
|---|---|---|
| ecat-protos | 4→8 | مطابقة ErrorCode الكاملة مع proto؛ فك ترميز buffer مقطوع؛ رسالة افتراضية لـ buffer فارغ؛ roundtrip metadata |
| ecat-errors | 4→9 | تعيين http_status الكامل (409/429/500)؛ from_status؛ غير المُعيَّن→Internal؛ cause source() |
| ecat-metadata | 9→12 | استخراج trace_id من رؤوس HTTP؛ تصغير حالة المفاتيح؛ خريطة رؤوس فارغة |
| ecat-encoding | 18→22 | NaN→null (افتراضي serde_json، موثق)؛ فك ترميز بايتات فارغة؛ CodecBox JSON غير صالح؛ roundtrip proto |
| ecat-lock | 7→9 | خطأ عند release دون امتلاك القفل؛ مفتاح فارغ |
| ecat-logging | 1→1 | shim التوافق لا يسبب panic |
| ecat-tracing | 9→12 | تخطي رأس trace غير UTF-8؛ الرأس المتعارف عليه؛ تمرير الاستجابة |
| ecat-tls | 7→12 | basic_auth بحقل واحد/حقلين؛ غياب ملف ca؛ is_enabled؛ العميل الافتراضي |
| ecat-config | 14→26 | فلترة بادئة env + حدود تحليل الأنواع (hex/سلسلة فارغة/-0/1e3)؛ دمج مصادر متعددة بالتراكب؛ مسارات خطأ obfs؛ ملف مفقود/YAML غير صالح |
| ecat-config-remote | 6→9 | حدود ConsulKvEntry؛ خطأ عند غياب X-Consul-Index؛ مفاتيح متداخلة |
| ecat-openapi | 4→11 | components/schema_ref؛ تراكب متكرر؛ 200 افتراضي؛ tags |
| ecat-metrics | 8→11 | نص المقاييس المسجلة؛ 404/405 |

### مجموعة data (test-data، +66)

| crate | سابق→جديد | نقاط التغطية |
|---|---|---|
| ecat-data | 12→14 | تحليل صيغة البحث |
| ecat-data-sqlx | 7→14 | SQLite في الذاكرة من النهاية إلى النهاية؛ ربط المعاملات بكل الأنواع؛ Blob→base64؛ config |
| ecat-data-redis | 6→12 | بناء URL redis:///rediss://؛ auth؛ مسارات خطأ config |
| ecat-data-opensearch | 4→10 | HTTP مُحاكى: percent-encode، Basic auth، تمرير الأخطاء |
| ecat-data-elasticsearch | 6→11 | كما سبق |
| ecat-data-influxdb | 5→10 | تهريب بروتوكول line؛ رأس Token؛ تمرير الأخطاء |
| ecat-data-clickhouse | 12→22 | SQL إنشاء الجداول؛ JSONEachRow؛ عدد الصفوف المكتوبة؛ التجميع |
| ecat-data-memcached | 4→8 | تحويل TTL من ثوانٍ إلى ملي ثانية؛ تعبئة flag |
| ecat-data-nebulagraph | 6→7 | تحليل config |
| ecat-data-arangodb | 5→7 | config/URL |
| ecat-data-iotdb | 5→10 | HTTP مُحاكى: معاملات مسار session |
| ecat-data-questdb | 4→9 | بروتوكول line؛ المعاملات غير مدعومة |
| ecat-data-tdengine | 6→11 | توليد INSERT؛ تقسيم دفعات 100 |
| ecat-data-mongodb | 5→8 | roundtrip bson؛ URI |

### مجموعة mq/transport/registry (test-mq، +54)

| crate | سابق→جديد | نقاط التغطية |
|---|---|---|
| ecat-mq | 5→9 | إطارات خطأ تأخر عند امتلاء المخزن؛ إغلاق التدفق عند إسقاط الكل؛ مشتركون متعددون؛ publish دون مشتركين |
| ecat-mq-kafka | 12→14 | قيم افتراضية config؛ حقول SASL تعمل بشكل مستقل |
| ecat-mq-rabbitmq | 2→5 | قيم افتراضية exchange؛ مسار خطأ url |
| ecat-mq-mqtt | 5→9 | التحقق من اقتران cert/key؛ ملف مفقود؛ منفذ افتراضي 1883/8883؛ تراجع عند منفذ غير صالح |
| ecat-mq-nats | 6→9 | نص عادي افتراضي؛ مسارات خطأ غياب ca/cert |
| ecat-transport | 4→7 | افتراضيات TlsConfig/with_client_auth؛ حدود normalize_addr |
| ecat-transport-http | 17→20 | اختبار تكامل: stop عملية فارغة، فشل عند احتلال المنفذ، استقبال وإرسال حقيقي |
| ecat-transport-grpc | 7→13 | TLS بملف مفقود؛ دورة حياة نص عادي؛ رفض mTLS |
| ecat-transport-ws | 4→8 | فشل دون handler؛ احتلال المنفذ؛ صدى إطار RFC 6455 masked |
| ecat-registry | 5→8 | discover متعدد المثيلات؛ إلغاء تسجيل تلقائي عند drop؛ افتراضيات builder |
| ecat-registry-consul | 10→24 | percent-encode؛ متغيرات التسجيل؛ استجابات خطأ؛ X-Consul-Token؛ تحليل agent/services؛ تراجع node |
| ecat-registry-etcd | 5→10 | تخطي القيم التالفة في discover؛ جسم طلب kv؛ منح lease؛ keepalive |

### مجموعة طبقة تطبيق app (test-app، +46)

| crate | سابق→جديد | نقاط التغطية |
|---|---|---|
| ecat-auth | 20→46 | قائمة بيضاء لذاكرة oauth2 المؤقتة/مفتاح SHA-256/إخلاء FIFO؛ ثلاث حالات apikey؛ فرض jwt iss/aud؛ انتهاء الصلاحية/توقيع خاطئ |
| ecat-health | 4→8 | تجميع الجاهزية (الكل ok/أي fail/سجل فارغ)؛ liveness |
| ecat-versioning | 4→7 | توجيه استراتيجية path؛ حدود extract_version |
| ecat-security | 12→20 | من النهاية إلى النهاية على طبقة الرؤوس؛ شكل JSON لاعتراض الهجمات |
| ecat-middleware | 34→37 | انتهاء صلاحية نافذة MemoryStore؛ panic داخلي→Err |
| ecat-circuit-breaker | 10→12 | استنفاد مجسات half-open؛ تراجع classify |
| ecat-client | 7→10 | خطأ نقطة نهاية grpc غير صالحة دون اتصال بالشبكة |
| ecat-graphql | 35→35 | التغطية السابقة كافية، لا فجوات |
| ecat-scheduler / ecat-bench / ecat-events / ecat-cli / ecat | التغطية السابقة كافية | لا فجوات |

## العيوب المكتشفة

| المستوى | الموقع | الوصف | الحالة |
|---|---|---|---|
| P1 | ecat-events/Cargo.toml | تفتقر dev-dependencies إلى ميزات tokio macros/rt/time، فتفشل أهداف اختبار ترجمة هذا crate منفردًا حتمًا (تغطيها بناءات workspace الكاملة بفضل اتحاد الميزات) | ✅ تم الإصلاح (إضافة الميزات + تعليق) |
| P2 | ecat-security src/lib.rs:118-127 | حقن SQLi بترميز percent في URI (`?q=SELECT%20*%20...`) يمكنه تجاوز فحص طبقة الرؤوس (يتطلب المكتشف مسافة حرفية، ويفحص URI الخام دون فك الترميز أولًا)؛ فحص الجسم غير متأثر | ⏳ بانتظار الإصلاح |
| P3 | ecat-data-sqlx | تستخدم `connect()/from_config()` AnyPool دون تثبيت مشغلات، فتصاب sqlx 0.8.6 بالـ panic "No drivers installed" عند أول اتصال | ⏳ بانتظار الإصلاح |
| P3 | ecat-data-influxdb | حقول السلسلة تهرب المسافات (`\ `)، بينما مواصفات line protocol تتطلب تهريب `"` و`\` فقط؛ ترتيب tag/field غير حتمي | ⏳ بانتظار الإصلاح |
| P3 | ecat-data-clickhouse | ذاكرة إنشاء الجداول المؤقتة لا تنتهي صلاحيتها أبدًا، فلا يُعاد CREATE بعد drop/تعديل خارجي للجدول | ⏳ بانتظار الإصلاح |
| P3 | ecat-circuit-breaker | حد half_open_probes غير قابل للوصول في الفحص التسلسلي (يمكن الوصول إليه فقط مع التزامن الجاري)، تغطيه اختبارات الصندوق الأبيض | ℹ️ معروف، ليس عيبًا |
| P3 | ecat-health | يستخدم `with_check` دالة blocking_write()، فتسبب panic عند الاستدعاء من سياق async؛ حاليًا usable فقط في السياقات المتزامنة | ℹ️ معروف، قيد API |

## الوحدات المتخطاة (تتطلب بيئة تكامل، لم تُحاكَ)

- roundtrip وسطاء حقيقيين: publish-subscribe لـ kafka/rabbitmq/mqtt/nats (الإعدادات ومسارات الأخطاء مغطاة)
- مجموعات حقيقية: دورة حياة تسجيل-اكتشاف consul/etcd (محاكاة axum تغطي شكل الطلبات)
- قواعد بيانات حقيقية: عمليات redis/memcached، mongod، تحقق خادم influxdb، مشغلات sqlx postgres/mysql، واجهات nebulagraph/arangodb API
- خدمات خارجية حقيقية: فحص OAuth2 (محاكاة محلية تغطيه)، roundtrip gRPC/HTTP (محاكاة محلية تغطي عدم تتبع 302)
