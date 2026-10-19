# الوكيل: السطوح، التأكيدات، سلسلة المحادثة.

agent-surface-cancel = إلغاء
agent-surface-confirm = تأكيد
agent-surface-confirm-danger = نعم، نفّذ ذلك
agent-surface-rewrite = إعادة الصياغة
agent-surface-pick = اختيار { $n }
agent-surface-keep = الاحتفاظ بالأصل
agent-surface-edit = التعديل قبل التطبيق
agent-surface-close = إغلاق
agent-surface-themes = السمات
agent-surface-variants = البدائل
agent-surface-permission-always = بشكل دائم
agent-surface-permission-title = يطلب الوكيل إذنًا
agent-undo-turn = الوكيل: { $text }

agent-tools-unavailable = تعذّر تشغيل أدوات الوكيل.

agent-undo-apply = تغيير الوكيل
agent-undo-variant = بديل: { $label }
agent-undo-param = إعداد: { $label }
agent-undo-rewrite = إعادة الصياغة

agent-confirm-save = هل تريد حفظ العرض التقديمي باسم { $path }؟
agent-confirm-save-over = هل تريد استبدال { $path }؟ سيتم الكتابة فوق الملف الحالي.
agent-confirm-open = هل تريد فتح { $path }؟
agent-open-elsewhere = هذا الملف مفتوح بالفعل في علامة تبويب أخرى.
agent-confirm-new = هل تريد بدء عرض تقديمي جديد؟
agent-confirm-new-dirty = هل تريد بدء عرض تقديمي جديد؟ ستُفقد التغييرات غير المحفوظة.
agent-confirm-save-templates = هل تريد حفظ مجموعة القوالب «{ $name }»؟
agent-confirm-delete-set = هل تريد حذف مجموعة القوالب «{ $name }»؟ لا يمكن التراجع عن هذا.
agent-confirm-edit-theme = هل تريد حفظ إصدار جديد من السمة «{ $name }»؟
agent-confirm-copy-theme = هل تريد حفظ نسخة معدّلة من السمة المدمجة «{ $name }»؟
agent-confirm-setting = هل تريد تغيير { $key }: { $old } → { $new }؟
agent-confirm-move = هل تريد نقل العرض إلى { $path }؟ سيُحذف الملف القديم.
agent-confirm-merge = هل تريد دمج { $path } في هذا العرض؟ لا يمكن التراجع عن ذلك.
agent-confirm-save-copy = هل تريد حفظ نسخة من العرض باسم { $path }؟

agent-not-found = لم يتم العثور على الوكيل: { $command }. إعداد [agent.presets].
agent-spawn-failed = تعذّر تشغيل الوكيل: { $command }. إعداد [agent.presets].
agent-no-http = هذا الوكيل لا يستطيع الاتصال بأدوات Oku Slides.
agent-local-no-program = لا يعمل الوكيل المحلي إلا داخل تطبيق Oku Slides.
agent-local-needs-download = يحتاج الوكيل المحلي أولاً إلى تنزيل نموذجه ({ $gigabytes } غيغابايت).
agent-local-intel-mac = يحتاج الوكيل المحلي إلى جهاز Mac بمعالج Apple.
agent-local-unsupported-os = الوكيل المحلي غير متاح على هذا النظام بعد.
agent-model-download = تنزيل النموذج
agent-model-downloading = جارٍ تنزيل النموذج… { $percent }%
agent-model-cancel = إلغاء
agent-model-download-failed = تعذّر تنزيل النموذج.
agent-model-download-integrity = كان النموذج الذي تم تنزيله تالفاً وقد حُذف.
agent-model-download-incomplete = تنزيل غير مكتمل: { $got } بايت من أصل { $expected }.
agent-model-download-cancelled = أُلغي التنزيل.
local-agent-no-route = لم يفهم الوكيل المحلي ما يجب فعله. حاول قوله بطريقة أخرى.
local-agent-tool-limit = توقف الوكيل المحلي بعد { $n } خطوات دون أن ينتهي.
local-agent-invalid-call = لم يتمكن الوكيل المحلي من استخدام الأدوات بشكل صحيح.
local-agent-engine-failed = فشل نموذج الوكيل المحلي.
local-agent-retry = حاول مرة أخرى، أو اختر وكيلًا آخر في لوحة الوكيل.
agent-stopped = توقف الوكيل.
agent-failed = رفض الوكيل الطلب: { $message }
agent-tool-failed = تعذّر تنفيذ الأداة بشكل غير متوقع.

agent-turn-tools = { $tools ->
   [zero] لا أدوات
   [one] أداة واحدة
   [two] أداتان
   [few] { $tools } أدوات
   [many] { $tools } أداة
  *[other] { $tools } أداة
}
agent-turn-notes = { $notes ->
   [zero] لا ملاحظات
   [one] ملاحظة واحدة
   [two] ملاحظتان
   [few] { $notes } ملاحظات
   [many] { $notes } ملاحظة
  *[other] { $notes } ملاحظة
}
agent-turn-secs = { $secs } ث
agent-turn-stopped = تم الإيقاف.
agent-turn-interrupted = تمت المقاطعة.
agent-memory-lost = لم يعد الوكيل يتذكر هذه المحادثة: سيبدأ من جديد.
agent-turn-max-tokens = بلغ الوكيل حدّ الطول.
agent-turn-max-turns = بلغ الوكيل حدّ الخطوات.
agent-turn-refusal = رفض الوكيل.
agent-turn-ended = انتهى الدور ({ $stop }).
agent-choice-cancelled = أُلغي السؤال
agent-choice-answered = ✓ { $answer }
agent-report-why = لماذا

agent-error-file = تعذّرت قراءة الملف أو الكتابة فيه.
agent-error-unexpected-action = إجراء غير متوقع: { $action }.
agent-error-unknown-surface = لم تعد هذه البطاقة مفتوحة.
agent-error-expired = انتهت صلاحية إجراءات دور سابق.
agent-error-incomplete = إجراء هذه البطاقة غير مكتمل.
agent-error-no-field = لم يعد هذا الإعداد موجودًا.
agent-error-no-variant = لم يعد هذا البديل موجودًا.
agent-error-arguments = وسائط الأداة غير صالحة ({ $detail }): تحقّق منها وفق مخططها.
agent-error-unknown-tool = أداة غير معروفة: { $name }.
agent-error-pack-id = هذا ليس معرّف مجموعة قوالب: استدعِ templates لعرض المجموعات ومعرّفاتها.
agent-error-denied = رفض المستخدم.
agent-error-no-turn = لا يوجد دور جارٍ: لا يُعرض { $tool } إلا أثناء دور.
agent-error-turn-over = انتهى دور هذا الاستدعاء: لم يُعرض شيء.
agent-error-anchor-scope = لا يمكن ربط سؤال في محادثة عنصر واجهة بشريحة: اطرحه دون ربط.
agent-error-no-file = ليس للعرض التقديمي ملف بعد: أعطِ مسارًا مطلقًا.
agent-error-path-relative = يجب أن يكون المسار مطلقًا.
agent-error-path-parent = يجب ألا يحتوي المسار على «..».
agent-error-setting-locked = لا يمكن للوكيل تغيير الإعداد { $key }.
agent-error-setting = تعذّر تغيير الإعداد { $key }: تحقّق من قيمته.
agent-error-history-index = يلزم رقم إصدار للتنقل في السجل.
agent-error-no-pack = لا يثبّت هذا العرض أي حزمة قوالب: أعطِ set (راجع templates).
agent-error-history-not-opened = لم تفتح علامة التبويب السجل في الوقت المناسب: اطلب مجددًا.
agent-error-present = لا شيء لعرضه هنا: لا توجد شرائح، أو أن علامة التبويب في وضع القوالب أو السجل أو القراءة فقط.
agent-error-locale = رمز لغة غير معروف. أعطِ رمز لغة متاحة أو system.
agent-error-empty-interface = لا شيء لتغييره: أعطِ scheme أو locale أو grid أو guides.
agent-error-not-shown = علامة التبويب هذه ليست المعروضة في نافذتها: اطلب مجددًا عندما يعرضها المستخدم.
agent-error-themes-deck-mode = لا يُفتح منتقي السمات إلا أثناء تحرير الشرائح.
agent-error-rewrite-text = عادت إعادة الصياغة دون نص.
agent-error-paragraphs = فقرات إعادة الصياغة غير صالحة.
agent-error-draft-too-long = المسودة طويلة جدًا.
agent-error-draft-origin = رُفضت المسودة: عرض غير معروف.
agent-error-draft-no-agent = لم يبدأ وكيل هذا التبويب بعد: رُفضت المسودة.
agent-error-unknown-agent = لم يعد هذا الوكيل موجودًا في الإعدادات.
agent-error-agent-locked = اختيار الوكيل تفرضه البيئة.
agent-error-agent-not-saved = تعذّر حفظ اختيار الوكيل.
agent-picker-title = الوكيل
agent-picker-busy = تبديل الوكيل سيوقف الدور الجاري.
agent-picker-locked = الاختيار مفروض من البيئة (OKU_SLIDES_AGENT__SELECTED).
agent-tune-prompt = اضبط العرض التقديمي المستورد بدقة
