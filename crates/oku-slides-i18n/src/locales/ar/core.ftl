op-slide-not-found = لم يُعثر على الشريحة: { $subject }
op-element-not-found = لم يُعثر على العنصر: { $subject }
op-duplicate-id = المعرّف مستخدم بالفعل: { $subject }
op-index-out-of-range = الموضع خارج النطاق
op-invalid-value = قيمة غير صالحة لـ { $subject }
op-unknown-widget = أداة غير معروفة: { $subject }
op-unknown-param = معامل غير معروف: { $subject }
op-param-type = قيمة غير صالحة لـ { $subject }
op-item-not-found = لا يوجد بند { $subject }
op-list-full = القائمة { $subject } ممتلئة ({ $max ->
   [zero] { $max } بند
   [one] بند واحد
   [two] بندان
   [few] { $max } بنود
   [many] { $max } بندًا
  *[other] { $max } بند
} على الأكثر)
op-list-minimum = يجب أن تحتوي القائمة { $subject } على { $min ->
   [zero] { $min } بند
   [one] بند واحد
   [two] بندين
   [few] { $min } بنود
   [many] { $min } بندًا
  *[other] { $min } بند
} على الأقل
op-list-cell-too-long = إحدى خلايا القائمة { $subject } طويلة جدًا (العمود { $column })
op-list-id-too-long = معرّف أحد بنود القائمة { $subject } طويل جدًا
op-list-too-many-items = تحتوي القائمة { $subject } على عدد كبير جدًا من البنود ({ $max } على الأكثر)
op-slot-rejects = { $subject } غير مقبول هنا
op-wrong-category = لا يمكن استخدام { $subject } هنا
op-choice-out-of-range = القيمة غير مسموح بها لـ { $subject }
transaction-refused = رُفضت العملية { $index }: { $reason }

sources-unreadable = تعذّرت قراءة المصادر
sources-not-sources = هذا ليس أرشيف مصادر Oku Slides
sources-unsupported-version = كُتبت هذه المصادر بإصدار أحدث من Oku Slides
sources-bad-name = اسم ملف مرفوض في المصادر
sources-missing-deck = العرض التقديمي غير موجود في المصادر
sources-too-large = مصادر هذا الملف أكبر من أن تُفتح
sources-too-deep = هذا العرض يضمّن العناصر بعمق مفرط

issue-empty-slide-id = معرّف شريحة فارغ
issue-duplicate-slide-id = معرّف شريحة مكرر
issue-duplicate-element-id = معرّف عنصر مكرر
issue-empty-element-id = معرّف فارغ
issue-cell-outside-grid = خلية خارج الشبكة
issue-empty-frame = إطار بحجم صفري
issue-free-without-frame = عنصر حر بلا إطار
issue-unknown-widget = عنصر واجهة غير معروف: { $kind }
issue-wrong-category = لا يمكن استخدام { $kind } هنا
issue-unknown-param = معامل غير معروف: { $param }
issue-param-type = قيمة غير صالحة لـ { $param }
issue-choice-out-of-range = قيمة غير مسموح بها لـ { $param }
issue-slot-rejects = { $kind } غير مقبول هنا
issue-placed-in-flow = عنصر موضوع بإطار في قائمة منسّقة
issue-unknown-bind = { $param } مرتبط بإعداد عرض غير معروف
issue-slot-too-deep = العناصر متداخلة بعمق مفرط
issue-missing-asset = مورد مفقود: { $asset }
issue-pack-version-mismatch = أُهملت نسخة حزمة معدَّلة: { $version }
issue-pack-version-missing = نسخة الحزمة مفقودة من المصادر: { $version }
issue-theme-version-mismatch = أُهملت نسخة سمة معدَّلة: { $version }
issue-theme-version-missing = نسخة السمة مفقودة من المصادر: { $version }
issue-unsupported-link = رابط غير مدعوم: { $link }
issue-highlight-zero = تُرقَّم ألوان التمييز بدءًا من 1
issue-unreadable-element = عنصر غير مقروء، نُحّي جانبًا
issue-unreadable-item = بند قائمة غير مقروء، نُحّي جانبًا
issue-data-truncated = بيانات كبيرة جدًا، اقتُطعت عند القراءة
issue-unreadable-slide = شريحة غير مقروءة، نُحّيت جانبًا
issue-unreadable-field = حقل العرض { $field } غير مقروء، أُعيد تعيينه
op-invalid-chart = المخطط { $subject }: { $reason }
op-invalid-table = الجدول { $subject }: { $reason }

replica-corrupted = سجل العرض التقديمي في هذا الملف تالف
replica-too-large = سجل العرض التقديمي في هذا الملف أكبر من أن يُفتح
replica-write = تعذّر تحديث سجل العرض التقديمي
replica-unrelated = لا يشترك هذان العرضان التقديميان في أصل واحد
replica-unsupported-version = يستخدم هذا العرض التقديمي تنسيقًا لا يستطيع هذا الإصدار من Oku Slides قراءته

widget-unreadable = تعذّرت قراءة تعريفات الودجات
widget-fault-empty-name = الودجة { $widget }: ودجة أو معلمة بلا اسم
widget-fault-duplicate-widget = الودجة { $widget } معرَّفة مرتين
widget-fault-duplicate-param = الودجة { $widget } تعرّف نفس المعلمة مرتين
widget-fault-empty-choice = الودجة { $widget }: خيار بلا قيم
widget-fault-default-not-in-choice = الودجة { $widget }: القيمة الافتراضية للخيار ليست ضمن قيمه
widget-fault-default-wrong-type = الودجة { $widget }: القيمة الافتراضية لا تطابق نوع مُعامِلها
widget-fault-bind-not-plain = الودجة { $widget }: فقط معلمات النص البسيط يمكن ربطها بالعرض التقديمي
widget-fault-unknown-slot-kind = الودجة { $widget }: تقبل الفتحة ودجة عنصر غير معروفة
widget-fault-empty-list = الودجة { $widget }: تحتاج القائمة إلى عمود واحد على الأقل
widget-fault-column-type = الودجة { $widget }: يمكن أن يكون عمود القائمة plain أو rich أو number أو flag أو choice أو asset؛ للمحتوى الحر لكل بند، استخدم p.slot مع ودجة فرعية
widget-fault-reserved-name = الودجة { $widget }: اسم محجوز: id أو kind أو اسم يحتوي على /
widget-fault-list-bounds = الودجة { $widget }: الحد الأقصى للقائمة أقل من حدها الأدنى
widget-fault-list-default-too-large = الودجة { $widget }: القيمة الافتراضية لقائمة تتجاوز حدود حجم القوائم
widget-incompatible-override = الودجة { $widget } تستبدل الودجة الأساسية لكنها تغيّر أو تحذف المعلمة { $param }
widget-incompatible-category = الودجة { $widget } تستبدل الودجة الأساسية بفئة مختلفة
issue-empty-item-id = القائمة { $param } تحتوي على بند بلا معرّف
issue-duplicate-item-id = القائمة { $param } تحتوي على معرّف بند مكرر
issue-list-bounds = القائمة { $param } تحتوي على عدد بنود قليل جدًا أو كثير جدًا
