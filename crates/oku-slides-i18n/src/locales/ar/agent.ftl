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
agent-confirm-save-pack = هل تريد حفظ الحزمة «{ $name }»؟
agent-confirm-delete-pack = هل تريد حذف الحزمة «{ $name }»؟ لا يمكن التراجع عن هذا.
agent-confirm-edit-theme = هل تريد حفظ إصدار جديد من السمة «{ $name }»؟
agent-confirm-copy-theme = هل تريد إنشاء متغيّر من السمة المدمجة «{ $name }»؟ تبقى السمة المدمجة دون تغيير.
agent-confirm-setting = هل تريد تغيير { $key }: { $old } → { $new }؟
agent-confirm-move = هل تريد نقل العرض إلى { $path }؟ سيُحذف الملف القديم.
agent-confirm-merge = هل تريد دمج { $path } في هذا العرض؟ لا يمكن التراجع عن ذلك.
agent-confirm-save-copy = هل تريد حفظ نسخة من العرض باسم { $path }؟
agent-confirm-dictation-api = هل تريد إرسال ما تمليه إلى { $host } ({ $url }) بالنموذج { $model } مع سر { $var }؟
agent-confirm-dictation-download = هل تريد استخدام { $name } وتنزيله الآن ({ $size })؟
agent-confirm-dictation-language = هل تريد استخدام إملاء Apple وتنزيل لغته الآن؟
agent-confirm-agent = هل تريد تبديل وكيل التطبيق إلى { $name } ({ $command }) عندما لا يكون أي رد قيد التشغيل؟
agent-confirm-agent-builtin = هل تريد تبديل وكيل التطبيق إلى { $name } عندما لا يكون أي رد قيد التشغيل؟
agent-confirm-agent-model = هل تريد تنزيل نموذج الوكيل المحلي ({ $size })؟

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
agent-error-pack-id = هذا ليس معرّف حزمة: استدعِ packs لعرض الحزم ومعرّفاتها.
agent-error-asset-unknown = هذه الصورة ليست في العرض التقديمي ولا ضمن الصور الملصقة: استخدم معرّف صورة ملصقة.
agent-error-denied = رفض المستخدم.
agent-error-no-turn = لا يوجد دور جارٍ: لا يُعرض { $tool } إلا أثناء دور.
agent-error-pack-tab-no-turn = لا تفتح علامة تبويب حزمة ولا تنتقل إليها إلا أثناء الرد على المستخدم.
agent-error-pack-tab-limit = فتحتَ بالفعل علامة تبويب حزمة في هذا الدور؛ أكمل هناك، أو اسأل المستخدم قبل فتح أخرى.
agent-error-continue-in-pack-tab = يتابع هذا الطلب في علامة تبويب الحزمة نفسها: اطلب من المستخدم المتابعة هناك؛ لا تعدّل علامة التبويب هذه أي عنصر واجهة الآن.
agent-error-pack-open-in-tab = هذه الحزمة قيد التعديل في علامة تبويب أخرى: اطلب من المستخدم أن ينهي هناك أولًا.
agent-pack-opened-in-tab = فُتحت الحزمة في علامة التبويب { $tab }، وأصبح التركيز عليها الآن. أخبر المستخدم بالمتابعة هناك: فوكيل علامة التبويب تلك لا يعرف هذا الطلب. تبقى علامة التبويب هذه على عرضها التقديمي.
agent-error-turn-over = انتهى دور هذا الاستدعاء: لم يُعرض شيء.
agent-error-no-file = ليس للعرض التقديمي ملف بعد: أعطِ مسارًا مطلقًا.
agent-error-path-relative = يجب أن يكون المسار مطلقًا.
agent-error-path-parent = يجب ألا يحتوي المسار على «..».
agent-error-setting-locked = لا يمكن للوكيل تغيير الإعداد { $key }.
agent-error-setting = تعذّر تغيير الإعداد { $key }: تحقّق من قيمته.
agent-error-history-index = يلزم رقم إصدار للتنقل في السجل.
agent-error-no-pack = لا يثبّت هذا العرض أي حزمة: أعطِ pack (راجع packs).
agent-error-history-not-opened = لم تفتح علامة التبويب السجل في الوقت المناسب: اطلب مجددًا.
agent-error-present = لا شيء لعرضه هنا: لا توجد شرائح، أو أن علامة التبويب في وضع الحزمة أو السجل أو القراءة فقط.
agent-error-locale = رمز لغة غير معروف. أعطِ رمز لغة متاحة أو system.
agent-error-empty-interface = لا شيء لتغييره: أعطِ scheme أو locale أو grid أو guides.
agent-error-empty-dictation = لا شيء لتغييره: أعطِ إعدادًا واحدًا على الأقل للإملاء.
agent-error-not-shown = علامة التبويب هذه ليست المعروضة في نافذتها: اطلب مجددًا عندما يعرضها المستخدم.
agent-error-themes-deck-mode = لا يُفتح منتقي السمات إلا أثناء تحرير الشرائح.
agent-error-packs-deck-mode = لا تُفتح لوحة حزم العرض التقديمي إلا أثناء تحرير الشرائح.
agent-error-rewrite-text = عادت إعادة الصياغة دون نص.
agent-error-paragraphs = فقرات إعادة الصياغة غير صالحة.
agent-error-draft-too-long = المسودة طويلة جدًا.
agent-error-draft-origin = رُفضت المسودة: عرض غير معروف.
agent-error-draft-no-agent = لم يبدأ وكيل هذا التبويب بعد: رُفضت المسودة.
agent-error-too-many-images = المسودة تحتوي بالفعل على كل صورها.
agent-error-image-not-base64 = لم تصل الصورة الملصقة كاملة.
agent-error-image-gone = لم تعد صورة ملصقة متاحة: الصقها مرة أخرى.
agent-error-unknown-agent = لم يعد هذا الوكيل موجودًا في الإعدادات.
agent-error-agent-locked = اختيار الوكيل تفرضه البيئة.
agent-error-agent-not-saved = تعذّر حفظ اختيار الوكيل.
voice-error-no-dictation = الإملاء غير متاح هنا
voice-error-busy = الإملاء قيد التشغيل: أعد المحاولة عندما يتوقف
voice-error-unknown-model = نموذج إملاء غير معروف
voice-error-unavailable = هذا النموذج لا يدعم اللغة المختارة
voice-error-installed = هذا النموذج مثبّت بالفعل
voice-error-not-downloadable = هذا النموذج لا يُنزَّل
voice-error-nothing-to-cancel = لا يوجد تنزيل جارٍ
voice-error-bad-silence = يجب أن يدوم الصمت من 0.5 إلى 10 ثوانٍ
voice-error-bad-language = لغة إملاء غير معروفة
voice-error-bad-url = يجب أن يبدأ عنوان الواجهة البرمجية بـ https:// (و http:// لهذا الحاسوب فقط)، دون اسم مستخدم أو معاملات أو محارف خاصة
voice-error-bad-key-env = يجب أن يكون متغير المفتاح اسمًا مثل OPENAI_API_KEY: أحرف كبيرة وأرقام و _
voice-error-bad-api-model = لا يقبل اسم نموذج الواجهة البرمجية إلا الأحرف والأرقام و . _ : / -
voice-error-bad-api-query = تُكتب المعلمات بالشكل name=value&name=value، بأحرف ASCII مرئية دون % أو #، و8 على الأكثر
voice-error-bad-api-headers = تُكتب الترويسات بالشكل Name: value; Name: $VARIABLE، و8 على الأكثر. يلزم متغير مفتاح أو ترويسة $VARIABLE؛ وتُرفض Host, Content-Type, Content-Length, Transfer-Encoding, Connection والأسماء المكررة و Authorization مع متغير مفتاح
voice-error-api-reserved = المستخدم وحده يضبط عنوان الواجهة البرمجية ومتغير المفتاح ومعلمات الاستعلام والترويسات: افتح المربع بـ show_dictation واطلب منه ذلك
voice-error-changed = تغيّرت الإعدادات منذ السؤال: اسأل مجددًا
voice-error-nothing-to-download = لا شيء لتنزيله لهذا الوكيل
agent-tune-prompt = اضبط العرض التقديمي المستورد بدقة
agent-settings-unknown-selected = الإعداد agent.selected = { $selected } لا يشير إلى أي وكيل في [agent.presets] (المعروفة: { $known })
agent-settings-preset-launch = يحتاج الوكيل [agent.presets.{ $id }] إلى واحد فقط من command أو builtin
