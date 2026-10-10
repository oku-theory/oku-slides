# الوكيل: السطوح، التأكيدات، سلسلة المحادثة.

agent-surface-cancel = إلغاء
agent-surface-confirm = تأكيد
agent-surface-confirm-danger = نعم، نفّذ ذلك
agent-surface-rewrite = إعادة الصياغة
agent-surface-pick = اختيار { $n }
agent-surface-keep = الاحتفاظ بالأصل
agent-surface-edit = التعديل قبل التطبيق
agent-surface-close = إغلاق
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
agent-confirm-setting = هل تريد تغيير { $key }: { $old } → { $new }؟

agent-not-found = لم يتم العثور على الوكيل: { $command }. إعداد [agent] command.
agent-no-http = هذا الوكيل لا يستطيع الاتصال بأدوات Oku Slides.
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
agent-error-denied = رفض المستخدم.
agent-error-no-turn = لا يوجد دور جارٍ: لا يُعرض { $tool } إلا أثناء دور.
agent-error-turn-over = انتهى دور هذا الاستدعاء: لم يُعرض شيء.
agent-error-anchor-scope = لا يمكن ربط سؤال في محادثة عنصر واجهة بشريحة: اطرحه دون ربط.
agent-error-no-file = ليس للعرض التقديمي ملف بعد: أعطِ مسارًا مطلقًا.
agent-error-path-relative = يجب أن يكون المسار مطلقًا.
agent-error-path-parent = يجب ألا يحتوي المسار على «..».
agent-error-setting-locked = لا يمكن للوكيل تغيير الإعداد { $key }.
agent-error-setting = تعذّر تغيير الإعداد { $key }: تحقّق من قيمته.
agent-error-rewrite-text = عادت إعادة الصياغة دون نص.
agent-error-paragraphs = فقرات إعادة الصياغة غير صالحة.
agent-error-draft-too-long = المسودة طويلة جدًا.
agent-error-draft-origin = رُفضت المسودة: عرض غير معروف.
agent-error-draft-no-agent = لم يبدأ وكيل هذا التبويب بعد: رُفضت المسودة.
