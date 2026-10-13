cli-reading = قراءة { $path }
cli-invalid-deck = عرض تقديمي غير صالح: { $path }
cli-deck-valid = عرض تقديمي صالح: { $slides } شريحة
cli-deck-issue = مشكلة: { $issue }
cli-rendered = { $pages } صفحة → { $out } (الترجمة البرمجية { $compile }، SVG { $svg })
cli-asset-added = المورد { $path } → { $id }
cli-saved = { $path }: { $pages } صفحة، PDF { $pdf } كيلوبايت، المصادر { $sources } كيلوبايت
cli-opened = { $path }: { $slides } شريحة، { $assets } مورد، { $fonts } خط
cli-extracted = استُخرج إلى { $dir }
cli-stripped = { $path }: نسخة بلا مصادر
cli-pdf-modified = عدّلت أداة أخرى ملف PDF بعد Oku Slides
cli-iterations = يجب أن تكون قيمة --iterations أكبر من 0
cli-no-block = لا يحتوي العرض التقديمي على كتلة لتعديلها
cli-blob-missing = لم يُعثر على كتلة المصادر
cli-roundtrip-mismatch = العرض التقديمي المُعاد قراءته يختلف عن المحفوظ
cli-bench-world = بناء البيئة (الخطوط)              { $time }
cli-bench-first = الترجمة الأولى                    { $time }  ({ $pages } صفحة)
cli-bench-same = إعادة الترجمة دون تغيير            { $time }
cli-bench-edited-text = نص معدَّل { $n }
cli-bench-edit = إعادة الترجمة بعد set_text         { $time }
cli-bench-svg = SVG لكل شريحة                       { $time }  ({ $each } لكل شريحة)
cli-bench-pdf = التصدير إلى PDF                     { $time }  ({ $size } كيلوبايت)
cli-bench-embed = تضمين كتلة المصادر                { $time }
cli-bench-extract = إعادة قراءة الكتلة              { $time }
cli-bench-roundtrip = دورة العرض التقديمي الكاملة     مطابق
cli-bench-history = خطوة في السجل                    { $mean } ms بالمتوسط، p50 { $p50 } ms
cli-serving = Oku Slides (تطوير) على http://{ $host }:{ $port }
cli-models-dir = نماذج الإملاء: { $dir }
cli-stopped = توقف الخادم
cli-not-found = غير موجود
cli-bad-request = طلب غير صالح: { $reason }
cli-sse-full = عدد كبير جدًا من تدفقات الأحداث مفتوحة بالفعل
cli-forbidden = تم رفض الطلب: لم يصدر عن الواجهة المحلية
cli-exposed = الاستماع على { $host }: الخادم بلا مصادقة ويمكن الوصول إليه من الشبكة؛ يُخدَم أي عميل يرسل ترويسة Host محلية. أبقِ [serve] host على 127.0.0.1
cli-imported = { $path }: { $slides ->
       [zero] لا شرائح
       [one] شريحة واحدة
       [two] شريحتان
       [few] { $slides } شرائح
       [many] { $slides } شريحة
      *[other] { $slides } شريحة
    }، { $pages ->
       [zero] لا صفحات
       [one] صفحة واحدة
       [two] صفحتان
       [few] { $pages } صفحات
       [many] { $pages } صفحة
      *[other] { $pages } صفحة
    }، { $losses ->
       [zero] لا فقد
       [one] فقد واحد
       [two] فقدان
       [few] { $losses } حالات فقد
       [many] { $losses } حالة فقد
      *[other] { $losses } حالة فقد
    }، { $requests ->
       [zero] لا طلبات أدوات
       [one] طلب أداة واحد
       [two] طلبا أداة
       [few] { $requests } طلبات أدوات
       [many] { $requests } طلب أداة
      *[other] { $requests } طلب أداة
    }
cli-import-label = استيراد من PowerPoint
cli-import-pack-required = الخيار --pack مطلوب في وضع retrofit: معرّف حزمة القوالب أو مجلدها أو اسمها
cli-import-unknown-pack = حزمة قوالب غير معروفة: { $pack }
cli-import-bad-pack = حزمة قوالب غير مقروءة { $pack }: ملف pack.json الخاص بها لا يصف حزمة قوالب
cli-import-write-failed = تعذّرت كتابة العرض المستورد { $path }
cli-import-hifi-no-target = --pack و--theme خاصّان بوضع retrofit: الاستيراد عالي الدقة (--mode hifi) يولّد حزمته وسمته
cli-import-probe-mismatch = الحزمة المولَّدة لا تُفحص كما أُعلنت (الأداة { $widget }): هذا خلل في المستورِد
cli-import-probe-limit = تتجاوز الحزمة المولَّدة حدود الفحص (الأداة { $widget }): ارفع max_widgets أو max_widget_kib أو probe_budget_ms في القسم [render] من الإعدادات

cli-about = عرض عروض Oku Slides التقديمية وحفظها
cli-arg-fonts = مجلد خطوط إضافي (TTF/OTF)؛ الافتراضي: paths.fonts في الإعدادات
cli-cmd-check = فحص عرض تقديمي
cli-arg-deck = العرض التقديمي (JSON)
cli-cmd-render = عرض كل شريحة بصيغة SVG مع مربعات العناصر
cli-arg-input = ملف deck.json أو .oku-slides.pdf
cli-arg-out = مجلد الإخراج
cli-arg-theme = استبدال سمة العرض التقديمي: سمة مدمجة أو من المكتبة، باسمها
cli-cmd-save = حفظ عرض تقديمي بصيغة .oku-slides.pdf (PDF + كتلة المصادر)
cli-arg-output = الملف المراد كتابته
cli-arg-assets = مجلد الموارد: يصبح كل ملف موردًا sha256:…
cli-arg-embed-fonts = تضمين خطوط --fonts في المصادر أيضًا
cli-cmd-open = إعادة فتح ملف .oku-slides.pdf: استخراج العرض التقديمي وموارده
cli-arg-extract = المجلد الذي تُكتب فيه deck.json و assets/ و fonts/
cli-cmd-strip = نسخ ملف .oku-slides.pdf دون كتلة مصادره
cli-cmd-bench = قياس أزمنة الترجمة والتصدير
cli-arg-iterations = عدد القياسات لكل خطوة
cli-cmd-schema = طباعة مخطط JSON للعمليات
cli-cmd-serve = خادم التطوير: الواجهة في متصفح، دون Tauri
cli-arg-serve-input = العرض التقديمي المراد فتحه (.oku-slides.pdf أو .json)
cli-arg-port = المنفذ؛ الافتراضي: serve.port في الإعدادات
cli-arg-ui = مجلد الواجهة المبنية (ui/dist)
cli-cmd-import = استيراد عرض PowerPoint (.pptx) بصيغة .oku-slides.pdf
cli-arg-pptx = عرض PowerPoint (.pptx)
cli-arg-mode = retrofit: المحتوى على حزمة قوالب وسمة موجودتين؛ hifi: نسخة مطابقة، على حزمة وسمة مولَّدتين لها
cli-arg-pack = حزمة القوالب الهدف: المعرّف أو مجلد الحزمة (pack.json، widgets/) أو الاسم
cli-arg-import-theme = السمة الهدف: سمة مدمجة أو من المكتبة، بمعرّفها أو اسمها؛ الافتراضي: السمة الافتراضية
cli-arg-report = كتابة تقرير الاستيراد (JSON) أيضًا: المفقودات والبدائل والتغطية
cli-arg-no-system-fonts = في وضع hifi، لا تبحث عن خطوط العرض بين خطوط النظام
