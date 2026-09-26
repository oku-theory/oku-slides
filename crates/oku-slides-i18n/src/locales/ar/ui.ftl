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
ui-status-builtin = مدمج: الحفظ ينشئ نسخة
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

ui-toast-saved = حُفظ عرضك التقديمي
ui-toast-templates-saved = حُفظت القوالب: { $name }
ui-toast-select-zone = حدّد منطقة لوضع: «{ $text }»

## شريط الأدوات

ui-toolbar-undo = تراجع
ui-toolbar-undo-label = تراجع: { $label }
ui-toolbar-redo = إعادة
ui-toolbar-redo-label = إعادة: { $label }
ui-toolbar-grid-hide = إخفاء الشبكة
ui-toolbar-grid-show = إظهار الشبكة
ui-toolbar-grid-hide-title = إخفاء الشبكة (⌘')
ui-toolbar-grid-show-title = إظهار الشبكة (⌘')
ui-toolbar-theme = السمة
ui-toolbar-theme-default = الافتراضية
ui-toolbar-save-copy = حفظ نسخة
ui-toolbar-save = حفظ
ui-toolbar-finish = تم
ui-toolbar-templates = القوالب
ui-toolbar-templates-title = إنشاء قوالب الشرائح وتعديلها
ui-toolbar-open = فتح
ui-toolbar-add-label = إضافة إلى الشريحة
ui-toolbar-add = إضافة
ui-add-kind = { $kind ->
    [card] بطاقة
    [shape] شكل
    [chart] مخطط
   *[text] نص
}
ui-toolbar-remove-template = حذف القالب
ui-toolbar-remove-slide = حذف الشريحة
ui-toolbar-remove-hint = الشريحة { $n } · تراجع باستخدام ⌘Z

## تسميات التراجع

ui-undo-add = { $kind ->
    [card] إضافة بطاقة
    [shape] إضافة شكل
    [chart] إضافة مخطط
   *[text] إضافة نص
}
ui-undo-new-template = قالب جديد: { $name }
ui-undo-new-slide = شريحة جديدة: { $name }
ui-undo-grid = شبكة { $columns } × { $rows }
ui-undo-templates = القوالب: { $name }
ui-undo-move-template = نقل القالب
ui-undo-move-slide = نقل الشريحة
ui-undo-remove-slide = حذف الشريحة
ui-undo-remove = حذف
ui-undo-cut = قص
ui-undo-paste = لصق
ui-undo-place-on-grid = وضع على الشبكة
ui-undo-move-on-grid = نقل على الشبكة
ui-undo-resize-on-grid = تغيير الحجم على الشبكة
ui-undo-change-anchor = تغيير التثبيت
ui-undo-move = نقل
ui-undo-resize = تغيير الحجم
ui-undo-edit-text = تعديل النص
ui-undo-dictation = الإملاء
ui-undo-rename-set = إعادة تسمية المجموعة
ui-undo-rename-deck = إعادة تسمية العرض التقديمي
ui-undo-theme = تغيير السمة
ui-undo-rename-template = إعادة تسمية القالب
ui-undo-describe-template = وصف القالب
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
ui-stage-edit-text = تعديل النص
ui-stage-hint-field = Enter للتأكيد · Esc للإلغاء
ui-stage-hint-block = ⌘ Enter للتأكيد · سطر لكل فقرة، و«- » لنقطة تعداد
ui-field = { $kind ->
    [title] العنوان
    [subtitle] العنوان الفرعي
    [kicker] العنوان التمهيدي
    [text] النص
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

## ملاحظات الشريحة

ui-notes = أدوات الشريحة
ui-stepper-less = { $label }: واحد أقل
ui-stepper-more = { $label }: واحد أكثر

## الإملاء

ui-dictation = الإملاء
ui-dictation-start = إملاء
ui-dictation-start-title = إملاء (⌘D)
ui-dictation-finish = إنهاء الإملاء
ui-dictation-finish-title = إنهاء (⌘D)
ui-dictation-cancel = إلغاء
ui-dictation-preparing = جارٍ تجهيز الإملاء…
ui-dictation-heard-nothing = لم يُسمع شيء: تحدّث أقرب قليلًا إلى الميكروفون
ui-dictation-to-cursor = عند المؤشر
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
ui-dictation-settings = إعدادات الإملاء
ui-dictation-model-title = نموذج التعرّف
ui-dictation-model-intro = Whisper، على هذا الحاسوب (CTranslate2). كل النماذج تفهم عدة لغات.
ui-dictation-note-turbo = موصى به، سريع ودقيق
ui-dictation-note-large = الأكثر دقة، وأبطأ
ui-dictation-note-base = خفيف، للأجهزة الصغيرة
ui-dictation-installed = مثبّت
ui-dictation-not-installed = للتنزيل
ui-dictation-language = اللغة المنطوقة
ui-dictation-language-auto = اكتشاف تلقائي
ui-dictation-auto-stop-label = التوقف تلقائيًا بعد صمت مدته
ui-dictation-silence = مدة الصمت
ui-dictation-phrases-note = لا تُفرَّغ إلا المقاطع المنطوقة، جملةً جملة، أثناء حديثك.
ui-dictation-download-now = التنزيل الآن ({ $size })
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
ui-chart-add-row = + صف
ui-chart-add-column = + عمود
ui-chart-series = السلسلة { $n }

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
ui-close-unsaved = هل تريد حفظ التغييرات على «{ $name }» قبل الإغلاق؟
ui-close-save = حفظ
ui-close-discard = عدم الحفظ
ui-close-cancel = إلغاء
