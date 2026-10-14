pdf-unreadable = تعذّرت قراءة ملف PDF
pdf-unsupported-version = كُتب ملف PDF هذا بإصدار أحدث من Oku Slides
pdf-corrupted = المصادر في ملف PDF هذا تالفة
pdf-write = تعذّرت كتابة ملف PDF

render-compile = تعذّر عرض العرض التقديمي
render-pdf = فشل التصدير إلى PDF
render-no-page = هذه الصفحة غير موجودة
render-image = صورة الشريحة لم تتمكن من الإنتاج

render-setup = تعذّر تشغيل محرك العرض
render-missing-image = صورة مفقودة
diagnostic-error = خطأ
diagnostic-warning = تحذير
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = تلميح: { $hint }
render-chart-failed = تعذّر رسم المخطط

registry-bad-pack-id = معرّف حزمة القوالب غير صالح: { $id }
registry-compile = تعذّر ترجمة عناصر واجهة حزمة القوالب
registry-missing = حزمة القوالب لا تُعلن عن عناصر واجهتها
render-timeout = استغرق العرض أكثر من { $seconds } ثانية وتم إيقافه
render-out-of-memory = احتاج العرض إلى أكثر من { $mib } ميبيبايت من الذاكرة وتم إيقافه
render-worker-lost = توقفت عملية العرض
render-frame = رسالة غير مقروءة من عملية العرض
render-page-too-large = هذه الشريحة أكبر من أن تُعرض معاينتها (أكثر من { $mib } ميبيبايت، غالبًا بسبب صورة كبيرة جدًا). لا يزال بالإمكان تصديرها بصيغة PDF أو PNG.
render-stale = تم استبدال العرض؛ حاول مرة أخرى
render-pack-disabled = عناصر واجهة الحزمة «{ $pack }» معطّلة في هذه الجلسة
registry-schema-remote = سجل عناصر الواجهة غير صالح: { $message }
probe-not-probed = لم يُفحص بعد: انتهى الوقت المخصص لعناصر واجهة الحزمة. سيُفحص عند العرض التالي.
probe-too-many-files = تحتوي الحزمة على أكثر من { $max } ملفًا لعناصر الواجهة: لم يُحمَّل هذا الملف.
probe-too-large = أكبر من { $max } كيلوبايت: لم يُحمَّل.

import-unreadable = تعذّرت قراءة هذا العرض التقديمي
import-not-a-presentation = هذا الملف ليس عرضًا تقديميًا من PowerPoint
import-protected = هذا العرض التقديمي مشفّر أو بتنسيق .ppt القديم: احفظه أولًا بتنسيق .pptx
import-too-large = هذا العرض التقديمي أكبر من أن يُستورد
import-output-too-large = سيتجاوز العرض الناتج الحجم المسموح به للاستيراد
import-loss-list-levels = تم دمج مستويات القائمة في مستوى واحد
import-loss-merged-cells = تم فصل الخلايا المدمجة
import-loss-percent-stacked = عُرض المخطط المكدس بنسبة 100% كمخطط مكدس
import-loss-run-style = { $effect ->
    [underline] فُقد التسطير
    [strike] فُقد الشطب
    [baseline] فُقد النص المرتفع أو المنخفض
    [caps] فُقدت الأحرف الكبيرة
    [font] استُبدل خط مقطع بخط المربع
    [tracking] تباعد أحرف مقطع استُبدل بتباعد المربع
    [weight] عُرض مقطع عادي بخط عريض مثل المربع
    [slant] عُرض مقطع مستقيم بخط مائل مثل المربع
   *[highlight] فُقد التمييز
}
import-loss-transform = فُقد التدوير أو القلب أو الاقتصاص
import-loss-color-dropped = لا يوجد للون { $color } لون قريب في السمة: استُخدم لون السمة
import-loss-text-truncated = اختُصر النص ليبقى ضمن الحدود
import-loss-table-truncated = اختُصر الجدول ليبقى ضمن الحدود
import-loss-opaque = { $reason ->
    [media] لم يُستورد الفيديو أو الصوت (لا توجد صورة بديلة)
    [ole] لم يُستورد الكائن المضمّن (لا توجد صورة بديلة)
    [math] لم تُستورد المعادلة (لا توجد صورة بديلة)
    [smartart] لم يُستورد SmartArt (لا توجد صورة بديلة)
   *[unknown] لم يُستورد الكائن غير المدعوم (لا توجد صورة بديلة)
}
import-loss-unsupported-image = لم تُستورد الصورة بتنسيق { $format }
import-loss-hidden-slide = استُوردت الشريحة المخفية كشريحة ظاهرة
import-loss-effects = فُقدت الظلال والانعكاسات والتأثيرات ثلاثية الأبعاد
import-loss-series-dropped = يعرض المخطط الدائري سلسلة واحدة: تم الاحتفاظ بـ { $kept } واستبعاد { $dropped }
import-loss-repeated-decor = { $slides ->
   [zero] حُذف الرأس أو التذييل «{ $text }» المكرر في { $slides } شريحة: يرسم النسق رأسه وتذييله
   [one] حُذف الرأس أو التذييل «{ $text }» المكرر في شريحة واحدة: يرسم النسق رأسه وتذييله
   [two] حُذف الرأس أو التذييل «{ $text }» المكرر في شريحتين: يرسم النسق رأسه وتذييله
   [few] حُذف الرأس أو التذييل «{ $text }» المكرر في { $slides } شرائح: يرسم النسق رأسه وتذييله
   [many] حُذف الرأس أو التذييل «{ $text }» المكرر في { $slides } شريحة: يرسم النسق رأسه وتذييله
  *[other] حُذف الرأس أو التذييل «{ $text }» المكرر في { $slides } شريحة: يرسم النسق رأسه وتذييله
}
import-loss-no-slot = حُذف المحتوى: لا مكان له في القالب المختار
import-loss-page-fitted = صفحة المصدر { $width } × { $height } نقطة ليست بنسبة 16:9: غُيّر حجم المحتوى ووُسِّط
import-loss-paragraph-style = وُحّدت محاذاة فقرة أو تباعد أسطرها أو تباعد الفقرات مع المربع
import-loss-shape-style = { $effect ->
    [gradient] عُرض التدرج بلون واحد
    [picture] مُدّت صورة التعبئة على إطار الشكل
    [dash] عُرض الخط المتقطع متصلاً
   *[geometry] عُرض شكل غير معروف كمستطيل
}
import-loss-layout-dropped = تخطيطات كثيرة جدًا لحزمة واحدة: «{ $layout }» بلا قالب؛ تستخدم شرائحه الأداة الفارغة وتتلقى زخرفته كعناصر
import-loss-decor-truncated = قُصّت الزخرفة لتلائم حجم ملف الحزمة: عناصر محذوفة: { $omitted }
import-loss-table-style = قُرّب نمط الجدول: خلفيات الخلايا أو الحدود أو عروض الأعمدة أو أحجام النص الخاصة به فُقدت
import-font-substituted = استُبدل { $family } بـ { $by } ذي المقاييس المتوافقة
import-font-missing = لم يُعثر على { $family }: يستخدم النص { $by }
import-font-restricted = { $family }: يمنع ترخيصه تضمين بعض أنماطه في العرض التقديمي
import-font-restricted-substituted = لا يمكن تضمين { $family } في العرض التقديمي (الترخيص): استُبدل بـ { $by } ذي المقاييس المتوافقة
import-font-restricted-missing = لا يمكن تضمين { $family } في العرض التقديمي (الترخيص): يستخدم النص { $by }
import-font-too-large = { $family }: الملفات أكبر من أن تُضمَّن في العرض؛ يستخدم النص { $by }
import-font-missing-no-fallback = لم يُعثر على { $family } ولا على خط متن السمة: يُعرض النص بالخط الافتراضي للمحرك
import-warning-effect-ignored = لم يُستورد التأثير: { $detail }
import-warning-approximated = عرض تقريبي: { $detail }
import-warning-unreadable-item = تم تخطي عنصر غير مقروء: { $detail }
import-warning-unreadable-part = جزء غير مقروء: { $detail }
import-warning-missing-part = جزء مفقود: { $detail }
import-warning-unsupported-chart = مخطط غير مدعوم: { $detail }
import-warning-unsupported-image = تنسيق صورة غير مدعوم: { $detail }
import-warning-external-image = صورة مرتبطة خارج الملف، لم تُستورد: { $detail }
import-warning-unsafe-svg = صورة SVG نُظّفت من المحتوى غير الآمن، أو استُبعدت: { $detail }
import-warning-unknown-table-style = نمط جدول غير معروف: { $detail }
import-warning-unsafe-link = تم رفض الرابط: { $detail }
import-warning-nesting-too-deep = تداخل عميق جدًا، تم قطعه: { $detail }
import-warning-font-embedded-unreadable = خط مضمَّن غير مقروء: { $detail }
import-warning-truncated = تحذيرات كثيرة جدًا: تم حذف التحذيرات التالية
