# واجهة التطبيق (React). يحمّلها ui/src/i18n.ts عبر الطلب «messages».
ui-loading = جارٍ التحميل…

## المستند

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · القوالب
ui-title-rename = إعادة تسمية العرض التقديمي
ui-title-field = عنوان العرض التقديمي
ui-status-unsaved = تغييرات غير محفوظة
ui-status-saved = محفوظ
ui-status-never-saved = لم يُحفظ بعد
ui-caption-templates = { $count ->
    [zero] لا قوالب
    [one] قالب واحد
    [two] قالبان
    [few] { $count } قوالب
    [many] { $count } قالبًا
   *[other] { $count } قالب
} · { $status }
ui-caption-slides = { $count ->
    [zero] لا شرائح
    [one] شريحة واحدة
    [two] شريحتان
    [few] { $count } شرائح
    [many] { $count } شريحة
   *[other] { $count } شريحة
} · { $status }
ui-template-default-name = قالب
ui-template-variant-name = { $name } (متغيّر)
ui-confirm-quit-templates = هل تريد الخروج دون حفظ القوالب المعدّلة؟

## الإشعارات

ui-toast-templates-saved = حُفظت القوالب: { $name }
ui-toast-moved = تم نقل العرض التقديمي
ui-toast-copied = تم حفظ نسخة
ui-toast-select-zone = حدّد منطقة لوضع: «{ $text }»
ui-toast-field-gone = الحقل الذي كنت تعدّله لم يعد موجودًا: فُقد ما أدخلته.

## شريط الأدوات

ui-toolbar-undo = تراجع
ui-toolbar-undo-label = تراجع: { $label }
ui-toolbar-redo = إعادة
ui-toolbar-redo-label = إعادة: { $label }
ui-toolbar-grid-hide = إخفاء الشبكة
ui-toolbar-grid-show = إظهار الشبكة
ui-toolbar-ghosts-hide = إخفاء الأشباح
ui-toolbar-ghosts-show = إظهار الأشباح
ui-divider-pile = عرض كومة الشرائح
ui-divider-notes = عرض سلسلة الوكيل
ui-divider-code = حجم الشيفرة
ui-divider-band = ارتفاع الإعدادات
ui-toolbar-theme = السمة
ui-toolbar-theme-embedded = سمة الملف
ui-toolbar-save-copy = حفظ نسخة
ui-toolbar-finish = تم
ui-menu-workshop = الورشة
ui-toolbar-open = فتح
ui-merge = دمج مع ملف PDF…
ui-menu = القائمة
ui-menu-move = نقل…
ui-language = اللغة
ui-language-system = لغة النظام
ui-scheme = المظهر
ui-scheme-system = النظام
ui-scheme-light = فاتح
ui-scheme-dark = داكن
ui-toolbar-add-label = إضافة إلى الشريحة
ui-add-kind = { $kind ->
    [card] بطاقة
    [shape] شكل
    [chart] مخطط
    [table] جدول
   *[text] نص
}
ui-add-widget = أداة الحزمة
ui-toolbar-remove-template = حذف القالب
ui-toolbar-remove-slide = حذف الشريحة
ui-toolbar-remove-hint = الشريحة { $n } · تراجع باستخدام ⌘Z

## تسميات التراجع

ui-undo-add = { $kind ->
    [card] إضافة بطاقة
    [shape] إضافة شكل
    [chart] إضافة مخطط
    [table] إضافة جدول
   *[text] إضافة نص
}
ui-undo-add-widget = إضافة «{ $name }»
ui-undo-new-template = قالب جديد: { $name }
ui-undo-new-slide = شريحة جديدة: { $name }
ui-undo-templates = القوالب: { $name }
ui-undo-move-template = نقل القالب
ui-undo-move-slide = نقل الشريحة
ui-undo-remove-slide = حذف الشريحة
ui-undo-remove = حذف
ui-undo-paste = لصق
ui-undo-place-on-grid = وضع على الشبكة
ui-undo-move-on-grid = نقل على الشبكة
ui-undo-resize-on-grid = تغيير الحجم على الشبكة
ui-undo-change-anchor = تغيير التثبيت
ui-undo-move = نقل
ui-undo-resize = تغيير الحجم
ui-undo-edit-slide = تعديل الشريحة
ui-undo-dictation = الإملاء
ui-undo-rename-set = إعادة تسمية المجموعة
ui-undo-rename-deck = إعادة تسمية العرض التقديمي
ui-undo-chart-kind = تغيير نوع المخطط
ui-undo-chart-data = تعديل البيانات
ui-undo-chart-color = تلوين سلسلة
ui-undo-chart-stack = { $stacked ->
    [yes] تكديس السلاسل
   *[no] فصل السلاسل
}

## العناصر الجديدة

ui-new-text = نص جديد
ui-new-card-title = عنوان البطاقة
ui-new-card-detail = تفاصيل

## كومة الشرائح

ui-pile-slides = الشرائح
ui-pile-templates = القوالب
ui-pile-add-slide = + شريحة جديدة
ui-pile-add-template = + قالب جديد
ui-pile-drag-hint = اسحب للنقل · ⌥↑ ⌥↓
ui-slide-alt = الشريحة { $n }

## المسرح

ui-stage-rendering = جارٍ العرض…
ui-stage-elements = عناصر الشريحة
ui-stage-fields = حقول الشريحة
ui-stage-edit-text = تعديل النص
ui-stage-hint-plain = Enter أو Tab للانتقال إلى التالي · Esc للإلغاء
ui-stage-hint-rich = Tab للانتقال إلى التالي · ⌘ Enter للتأكيد · «- » لنقطة تعداد · ⌘B عريض، ⌘I مائل، ⌘K رابط
ui-field = { $kind ->
    [title] العنوان
    [subtitle] العنوان الفرعي
    [kicker] العنوان التمهيدي
    [text] النص
    [body] المتن
    [author] المؤلف
    [date] التاريخ
    [organization] المؤسسة
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] نص
    [card] بطاقة
    [image] صورة
    [shape] شكل
    [group] مجموعة
   *[other] { $kind }
}
ui-grid-cell = عمود { $cols } · صف { $rows }
ui-ghost-zone = المنطقة

## ملاحظات الشريحة

ui-notes = أدوات الشريحة
ui-stepper-less = { $label }: واحد أقل
ui-stepper-more = { $label }: واحد أكثر
ui-list-item = عنصر
ui-list-add = إضافة { $item }
ui-list-remove = إزالة { $item }
ui-list-move = نقل { $item }
ui-list-full = القائمة ممتلئة ({ $max })
ui-undo-list-move = نقل { $item }
ui-agent-color-other = لون آخر
ui-agent-thread = الوكيل
ui-agent-new-conversation = محادثة جديدة
ui-agent-detach = فتح الوكيل في نافذة منفصلة
ui-agent-attach = إعادة الوكيل إلى النافذة الرئيسية
ui-agent-window-empty = لا يوجد وكيل لهذا التبويب
ui-agent-kept-in-draft = لا يوجد مكان للكتابة: تم الاحتفاظ بالنص في المسودة
ui-agent-busy-elsewhere = الوكيل يجيب في { $place }…
ui-agent-scope-slides = الشرائح
ui-agent-scope-atelier = الورشة
ui-draft = مسودة
ui-draft-insert = إدراج
ui-draft-insert-title = إدراج كما هو (⌘⇧↩)
ui-draft-send = إرسال إلى الوكيل
ui-draft-send-title = إرسال إلى الوكيل (⌘↩)
ui-draft-discard = تجاهل
ui-agent-state-idle = الوكيل
ui-agent-state-starting = الوكيل يبدأ…
ui-agent-state-working = الوكيل يعمل…
ui-agent-dock = قيد التنفيذ
ui-agent-dock-settings = الإعدادات
ui-agent-jump = ↓ الدور الحالي
ui-agent-sheet = سؤال الوكيل
ui-agent-sheet-keys = 1–4 اختيار · Enter تأكيد · E تعديل · Esc إلغاء
ui-agent-variants = البدائل
ui-agent-drawer-keys = انقر أو 1–4 · Esc يغلق
ui-agent-confirm = تأكيد الوكيل

## الإملاء

ui-dictation = الإملاء
ui-dictation-start = إملاء
ui-dictation-start-title = إملاء (⌘D)
ui-dictation-type = اكتب إلى الوكيل
ui-dictation-type-title = اكتب إلى الوكيل (⌘K)
ui-dictation-agent-stop = أوقف الوكيل (⌘⇧D أو Esc)
ui-dictation-agent-working = الوكيل يعمل…
ui-dictation-agent-working-hint = ⌘⇧D أو Esc يوقفه
ui-dictation-finish = إنهاء الإملاء
ui-dictation-finish-title = إنهاء (⌘D)
ui-dictation-cancel = إلغاء
ui-dictation-preparing = جارٍ تجهيز الإملاء…
ui-dictation-heard-nothing = لم يُسمع شيء: تحدّث أقرب قليلًا إلى الميكروفون
ui-dictation-to-cursor = عند المؤشر
ui-dictation-to-draft = في المسودة
ui-dictation-to-field = استبدال { $field }
ui-dictation-to-block = بعد الكتلة المحددة
ui-dictation-new-block = كتلة نص جديدة
ui-dictation-select-zone = حدّد منطقة
ui-dictation-downloading = جارٍ تنزيل النموذج { $name }
ui-dictation-download-progress = { $percent }% · { $received } من { $total } · مرة واحدة فقط
ui-dictation-listening = جارٍ الاستماع · { $time }
ui-dictation-transcribing = جارٍ تفريغ الجملة الأخيرة…
ui-dictation-stop-in = التوقف خلال { $duration }
ui-dictation-speak = تحدّث، وسيظهر النص هنا…
ui-dictation-auto-stop = يتوقف بعد { $duration } من الصمت
ui-dictation-to-download = { $size } للتنزيل
ui-dictation-settings = الإعدادات
ui-dictation-model-title = نموذج التعرّف
ui-dictation-model-intro = Whisper، على هذا الحاسوب (CTranslate2). كل النماذج تفهم عدة لغات.
ui-dictation-note-turbo = موصى به، سريع ودقيق
ui-dictation-note-large = الأكثر دقة، وأبطأ
ui-dictation-note-base = خفيف، للأجهزة الصغيرة
ui-dictation-installed = مثبّت
ui-dictation-not-installed = للتنزيل
ui-dictation-language = اللغة المنطوقة
ui-dictation-language-auto = اكتشاف تلقائي
ui-dictation-note-apple = الأسرع، مدمج في النظام
ui-dictation-model-unavailable = اللغة غير مدعومة
ui-dictation-model-intro-apple = Apple، المدمج في النظام، أو Whisper، على هذا الحاسوب (CTranslate2). جميعها تفهم عدة لغات.
ui-dictation-language-auto-resolved = تلقائي ({ $language })
ui-dictation-to-download-language = لغة للتنزيل
ui-dictation-auto-stop-label = التوقف تلقائيًا بعد صمت مدته
ui-dictation-silence = مدة الصمت
ui-dictation-phrases-note = لا تُفرَّغ إلا المقاطع المنطوقة، جملةً جملة، أثناء حديثك.
ui-dictation-download-now = التنزيل الآن ({ $size })
ui-dictation-download-language = تنزيل اللغة
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } ث
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } غيغابايت
ui-size-mb = { $value } ميغابايت

## منتقي القوالب

ui-picker-new-slide = شريحة جديدة
ui-picker-new-template = قالب جديد، مبني على…
ui-picker-set = القوالب
ui-picker-set-mine = { $name } · خاصتي
ui-picker-keys = ← → للتصفح · Enter للإضافة · Esc للإغلاق
ui-picker-close = إغلاق
ui-picker-previous = القالب السابق
ui-picker-next = القالب التالي
ui-picker-list = قوالب الشرائح

## المخطط

ui-chart-panel = المخطط
ui-chart-kinds = نوع المخطط
ui-chart-kind = { $kind ->
    [line] خطوط
    [pie] دائري
    [scatter] نقاط
    [radar] رادار
    [waterfall] شلال
   *[bar] أعمدة
}
ui-chart-no-data = لا يحتوي هذا المخطط على جدول بيانات.
ui-chart-options = خيارات المخطط
ui-chart-stack = مكدّس
ui-chart-waterfall-hint = حدّد «الإجمالي» لعرض شريط المجموع.
ui-chart-colors = ألوان السلاسل
ui-chart-color-of = لون { $name }
ui-chart-color-auto = تلقائي
ui-chart-color-pick = اللون { $n }
ui-chart-column-name = اسم العمود { $col }
ui-chart-remove-column = حذف العمود
ui-chart-cell = الصف { $row }، العمود { $col }
ui-chart-remove-row = حذف الصف
ui-chart-add-row = صف
ui-chart-add-column = عمود
ui-chart-series = السلسلة { $n }

## جدول

ui-table-panel = جدول
ui-table-options = خيارات الجدول
ui-table-header-row = صف العنوان
ui-table-header-column = العمود الأول كعنوان
ui-table-striped = صفوف مخططة
ui-table-alignment = محاذاة الأعمدة
ui-table-align-of = محاذاة { $name }
ui-table-align = { $align ->
    [start] البداية
    [center] المنتصف
    [end] النهاية
   *[auto] تلقائي
}
ui-table-column = العمود { $n }
ui-table-no-data = لا توجد بيانات في هذا الجدول.
ui-undo-table-option = خيارات الجدول
ui-undo-table-align = محاذاة العمود
ui-undo-table-data = تعديل الجدول
ui-show-as = عرض على شكل
ui-show-as-table = عرض على شكل جدول
ui-show-as-chart = عرض على شكل رسم بياني
ui-undo-show-as-table = عرض على شكل جدول
ui-undo-show-as-chart = عرض على شكل رسم بياني

## الملفات

ui-file-open-prompt = مسار الملف المراد فتحه (.oku-slides.pdf أو .json)
ui-file-save-prompt = حفظ باسم (المسار الكامل)

## علامات التبويب

ui-tabs = العروض المفتوحة
ui-tab-new = علامة تبويب جديدة
ui-tab-close = إغلاق علامة التبويب
ui-tab-unsaved = تغييرات غير محفوظة
ui-tab-templates = القوالب
ui-tab-move-new-window = نقل إلى نافذة جديدة
ui-tab-move-to = نقل إلى النافذة { $n }
ui-tab-close-others = إغلاق علامات التبويب الأخرى
ui-close-cancel = إلغاء
ui-disk-replaced-question = تم استبدال { $file } على القرص بعرض تقديمي آخر.
ui-disk-replaced-overwrite = استبدال
ui-disk-replaced-save-as = حفظ باسم…
ui-same-deck-question = { $file } نسخة من العرض التقديمي المفتوح في علامة التبويب «{ $title }».
ui-same-deck-merge = الدمج في علامة التبويب تلك
ui-same-deck-apart = فتح بشكل منفصل
ui-history = السجل
ui-history-version = نسخة { $date }
ui-history-restore = استعادة
ui-history-back = رجوع
ui-history-anonymous = مجهول
ui-history-slider = نسخ العرض التقديمي
ui-history-removed-since = حُذفت منذ ذلك الحين
ui-history-changed-since = عُدّلت منذ ذلك الحين
ui-history-added-since = الشرائح المضافة منذ ذلك الحين: { $count }
ui-help = مساعدة
ui-help-caption = الشريحة { $index } من { $count } · للقراءة فقط
ui-slide-previous = الشريحة السابقة
ui-slide-next = الشريحة التالية
ui-present = العرض التقديمي
ui-present-slide = الشريحة { $index } من { $count }
ui-present-screen = العرض بملء الشاشة
ui-present-window = العرض داخل النافذة
ui-toolbar-params-hide = إخفاء الإعدادات
ui-toolbar-params-show = إظهار الإعدادات
ui-help-close = إغلاق المساعدة
ui-help-search = البحث في التعليمات
ui-help-search-placeholder = ابحث…
ui-help-search-empty = لا توجد نتائج
ui-help-search-count = { $count } نتيجة
ui-pack-notice-close = إغلاق
ui-widgets-files = ملفات عناصر الواجهة
ui-widgets-common = مشترك
ui-widgets-new = ودجة جديدة
ui-widgets-name = اسم عنصر الواجهة (أحرف صغيرة وأرقام وشرطات)
ui-widgets-name-taken = هذا الاسم مستخدم بالفعل
ui-widgets-name-letter = يجب أن يبدأ الاسم بحرف
ui-widgets-valid = صالح
ui-widgets-errors = أخطاء
ui-widgets-empty = لا تحتوي هذه الحزمة على أي عنصر واجهة بعد.
ui-widgets-preview = معاينة عنصر الواجهة
ui-widgets-no-preview = لا توجد معاينة
ui-widgets-field-bound = مرتبط بالعرض: للقراءة فقط هنا
ui-widgets-source = الشيفرة المصدرية
ui-widgets-unsaved = غير محفوظ
ui-widgets-in-error = الودجات التي بها أخطاء
ui-widgets-none-in-error = لا توجد ودجات بها أخطاء
ui-widgets-section = الودجات
ui-widgets-open = فتح الودجة { $name }
ui-widgets-delete-named = حذف { $name }
ui-widgets-confirm-delete = هل تريد حذف الودجة «{ $name }»؟
ui-widgets-rename-hint = انقر نقرًا مزدوجًا أو اضغط F2 لإعادة التسمية
ui-widgets-tabs = علامات تبويب القوالب
ui-widgets-templates-tab = القوالب
ui-theme-tab = السمة
ui-theme-picker = السمة
ui-theme-name = اسم السمة
ui-theme-unsaved = تغييرات السمة غير محفوظة
ui-widgets-close-tab = إغلاق { $name }
ui-widgets-removed-by-agent = حذف الوكيل الودجة { $name }.
