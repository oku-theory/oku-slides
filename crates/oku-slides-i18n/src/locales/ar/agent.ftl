# الوكيل: السطوح، التأكيدات، سلسلة المحادثة.

agent-instructions = أنت تحرر عرض Oku Slides للمستخدم الذي يتحدث إليك. تصرف فقط عبر أدوات oku-slides. اقرأ العرض باستخدام deck_outline وslide وrender_slide قبل تغييره؛ غيّره باستخدام apply (يمكن التراجع عن كل تغيير). عندما يعود القرار للمستخدم، اسأل باستخدام ask_choice أو ask_confirm أو ask_rewrite؛ اقترح تخطيطات باستخدام show_variants وإعدادات مباشرة باستخدام show_params. أجب بإيجاز، بلغة المستخدم. تبدأ كل رسالة بكتلة <oku-slides-context> تصف ما يراه المستخدم. بعد كل رد، إذا كانت هناك خطوات تالية مفيدة، فاقترح من 1 إلى 4 منها باستخدام suggest_next (تسميات قصيرة؛ النقر يرسل إليك التعليمة). قبل إضافة عنصر، اقرأ أداة widgets لمعرفة الأنواع المتاحة ومعاملاتها.

agent-instructions-widgets = إنشاء أداة (widget): الأداة ملف Typst ضمن حزمة القوالب؛ استخدم widget_sources وwidget_source وset_widget وremove_widget وpreview_widget في وضع العرض وفي وضع القوالب على حد سواء. شكل ملف kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of، ثم #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …)؛ الاسم المصرَّح به هو اسم الملف. المعاملات: الواصفات p.plain وp.rich وp.flag وp.number وp.choice وp.asset وp.slot وp.grid. النصوص القابلة للتحرير: field(ctx, "value", args.value)، وتُكدَّس بواسطة fields-stack. الألوان والأحجام: رموز السمة (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body"))، ولا قيم مكتوبة صراحةً أبدًا. الشيفرة المشتركة: الملف _commun، ويُستورد بواسطة #import "_commun.typ": …. الحلقة: اكتب بواسطة set_widget، واقرأ الحالة وتشخيصاتها (السطر، العمود)، وانظر إلى preview_widget، ثم صحّح. لإدراجها في العرض: استخدم add_element مع kind = اسم الأداة. عنصر الشريحة يصرّح بـ category: "slide" (widget("name", category: "slide", params: …))؛ وإلا فهو عنصر. في الواصف، label: و default: اختياريان (p.plain()، p.plain(label: "Title", default: "…")).

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
