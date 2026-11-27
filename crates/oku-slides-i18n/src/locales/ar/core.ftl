op-slide-not-found = لم يُعثر على الشريحة: { $subject }
op-element-not-found = لم يُعثر على العنصر: { $subject }
op-duplicate-id = المعرّف مستخدم بالفعل: { $subject }
op-index-out-of-range = الموضع خارج النطاق
op-invalid-value = قيمة غير صالحة لـ { $subject }
op-invalid-value-because = قيمة غير صالحة لـ { $subject }: { $reason }
op-value-no-value = لا قيمة للمعامل
op-value-grid-tracks = تضم الشبكة من 1 إلى 24 عمودًا وصفًا
op-value-bad-item-id = معرّف البند نص غير فارغ بلا «/»
op-value-missing-item-id = يحتاج كل بند إلى معرّف
op-value-too-deep = العناصر متداخلة بعمق مفرط
op-value-empty-path = المسار فارغ
op-value-not-text = المتوقع نص أو null
op-value-empty-frame = يجب أن يكون العرض والارتفاع أكبر من 0
op-value-blocks-off-grid = الشريحة ترتّب خانات التدفق والشبكة تخفيها؛ انقل أولًا كل عنصر إلى خانة فارغة
op-value-outside-grid = الخلية خارج الشبكة
op-value-not-tokens = المتوقع كائن أو null
op-value-not-a-slot = هذا المعامل ليس خانة
op-value-out-of-bounds = العدد خارج النطاق المسموح
op-value-unreadable = القيمة ليست بالشكل المتوقع
op-value-valign = المتوقع top أو middle أو bottom
op-value-path = يتكوّن المسار من أوامر M وL وC وQ وZ تتبعها أرقامها، ويبدأ بـ M
knowledge-path-empty = المسار فارغ
knowledge-path-not-absolute = يجب أن يبدأ المسار بـ /
knowledge-path-bad-segment = جزء من المسار فارغ أو ينتهي بنقطة أو يستخدم حرفًا غير a-z و0-9 والنقطة والشرطة السفلية والواصلة
knowledge-path-dot-segment = يجب ألا يبدأ جزء من المسار بنقطة
knowledge-path-reserved = جزء من المسار اسم نظام محجوز
knowledge-path-too-deep = يحتوي المسار على أجزاء كثيرة جدًا
knowledge-path-too-long = المسار طويل جدًا
knowledge-path-not-markdown = يجب أن ينتهي اسم الملف بـ .md
knowledge-frontmatter-missing = يجب أن يبدأ الملف بكتلة ترويسة محاطة بـ ---
knowledge-frontmatter-unterminated = كتلة الترويسة ليس فيها --- ختامي
knowledge-frontmatter-too-large = كتلة الترويسة كبيرة جدًا
knowledge-frontmatter-not-yaml = الترويسة ليست YAML صالحًا
knowledge-frontmatter-not-mapping = يجب أن تكون الترويسة خريطة مفاتيحها نصوص
knowledge-frontmatter-duplicate-key = المفتاح "{ $key }" يظهر مرتين في الترويسة
knowledge-frontmatter-alias = لا يجوز أن تستخدم الترويسة مراسي YAML أو أسماءها المستعارة
knowledge-frontmatter-tag = لا يجوز أن تستخدم الترويسة وسوم YAML
knowledge-frontmatter-too-deep = الترويسة متداخلة بعمق زائد
knowledge-frontmatter-after-document-end = الترويسة تحتوي على نص بعد نهاية المستند ...
knowledge-frontmatter-flow-mapping = يجب أن تكون الترويسة تعيينًا كتليًا واحدًا، مفتاحًا واحدًا في كل سطر، دون نهاية `...`
knowledge-frontmatter-type-missing = تحتاج الترويسة إلى نوع غير فارغ
knowledge-frontmatter-bad-timestamp = يجب أن تكون قيمة "{ $key }" في الترويسة تاريخًا ووقتًا بصيغة RFC 3339 مع إزاحة المنطقة الزمنية
knowledge-frontmatter-not-allowed = المفتاح "{ $key }" غير مسموح به في ترويسة هذا الملف
knowledge-reason-not-found = لا يوجد ملف في هذا المسار
knowledge-reason-exists = يوجد ملف بالفعل في هذا المسار
knowledge-reason-id-missing = يحتاج الملف الجديد إلى معرّف
knowledge-reason-id-invalid = معرّف الملف هو 32 رقمًا ست عشريًا بأحرف صغيرة
knowledge-reason-id-taken = ملف آخر يستخدم هذا المعرّف بالفعل
knowledge-reason-id-mismatch = الملف في هذا المسار له معرّف آخر
knowledge-reason-edit-empty-old = يجب ألا يكون النص المراد استبداله فارغًا
knowledge-reason-edit-no-match = النص المراد استبداله غير موجود في الملف
knowledge-reason-edit-ambiguous = النص المراد استبداله يظهر في الملف أكثر من مرة
knowledge-reason-too-many-files = الحزمة تحتوي على عدد كبير جدًا من الملفات
knowledge-reason-file-too-large = الملف كبير جدًا
knowledge-reason-total-too-large = الحزمة كبيرة جدًا
knowledge-reason-over-limit = تجاوزت قاعدة المعرفة حدودها: حذف الملفات وحده يعيدها تحتها
knowledge-no-text-layer = لا يحتوي ملف PDF هذا على طبقة نص (إنه مسح ضوئي): صدّره مع النص أو الصق نصه
knowledge-legacy-format = لا يمكن قراءة تنسيق Office القديم هذا: احفظه بصيغة .pptx أو .docx أو .xlsx
knowledge-document-encrypted = هذا المستند محمي بكلمة مرور: احفظ نسخة غير محمية
knowledge-unsupported-document = لا يمكن استيراد هذا النوع من المستندات: PDF أو DOCX أو PPTX أو XLSX أو ODS أو Markdown أو نص
knowledge-document-damaged = هذا المستند تالف ولا يمكن قراءته
knowledge-document-type-mismatch = محتوى هذا الملف لا يطابق اسمه
knowledge-document-too-large = هذا المستند أكبر من أن يُستورد
op-knowledge = ملف المعرفة { $path }: { $reason }
op-unknown-widget = أداة غير معروفة: { $subject }
op-unknown-namespace = لا يحتوي هذا العرض على حزمة { $pack }.
op-pack-required = حزمة { $pack } تحتاجها { $by }؛ أزل تلك أولًا.
op-pack-in-use = لا تزال حزمة { $pack } مستخدمة في { $count ->
    [zero] { $count } عنصر
    [one] عنصر واحد
    [two] عنصرين
    [few] { $count } عناصر
    [many] { $count } عنصرًا
   *[other] { $count } عنصر
}.
op-pack-requirement-missing = تحتاج حزمة { $pack } إلى { $required }، وهي غير موجودة في هذا العرض.
op-base-required = تحتفظ كل عروض التقديم بحزمة الأدوات الأساسية.
op-pack-kind-conflict = يحتوي هذا العرض بالفعل على حزمة { $pack } من النوع الآخر (أدوات أو تخطيطات).
op-theme-cycle = السمات يمدّ بعضها بعضًا في حلقة: { $chain }.
op-theme-closure = السمات المثبّتة في هذا العرض ليست سمته وآباءها بالضبط ({ $namespace }).
op-unknown-param = معامل غير معروف: { $subject }
op-param-type-because = قيمة غير صالحة لـ { $subject }: { $reason }
op-param-shape-text = { $nullable ->
    [yes] المتوقع: نص أو null
   *[no] المتوقع: نص
}
op-param-shape-choice = { $nullable ->
    [yes] المتوقع: إحدى قيمه أو null
   *[no] المتوقع: إحدى قيمه
}
op-param-shape-paragraphs = { $nullable ->
    [yes] المتوقع: قائمة فقرات أو null
   *[no] المتوقع: قائمة فقرات
}
op-param-shape-elements = { $nullable ->
    [yes] المتوقع: قائمة عناصر أو null
   *[no] المتوقع: قائمة عناصر
}
op-param-shape-flag = { $nullable ->
    [yes] المتوقع: قيمة منطقية أو null
   *[no] المتوقع: قيمة منطقية
}
op-param-shape-number = { $nullable ->
    [yes] المتوقع: عدد أو null
   *[no] المتوقع: عدد
}
op-param-shape-bounded-number = { $nullable ->
    [yes] المتوقع: عدد ضمن حدوده أو null
   *[no] المتوقع: عدد ضمن حدوده
}
op-param-shape-grid = { $nullable ->
    [yes] المتوقع: شبكة أو null
   *[no] المتوقع: شبكة
}
op-param-shape-chart-option = { $nullable ->
    [yes] المتوقع: خيار مخطط أو null
   *[no] المتوقع: خيار مخطط
}
op-param-shape-table-option = { $nullable ->
    [yes] المتوقع: خيار جدول أو null
   *[no] المتوقع: خيار جدول
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] المتوقع: خيار مخطط أو جدول أو null
   *[no] المتوقع: خيار مخطط أو جدول
}
op-param-shape-items = { $nullable ->
    [yes] المتوقع: قائمة بنود أو null
   *[no] المتوقع: قائمة بنود
}
op-param-shape-list = { $nullable ->
    [yes] المتوقع: قائمة أو null
   *[no] المتوقع: قائمة
}
op-param-shape-add-element-only = { $nullable ->
    [yes] تُضاف العناصر عبر add_element, أو null
   *[no] تُضاف العناصر عبر add_element
}
op-param-shape-set-param-only = { $nullable ->
    [yes] تُحدَّد القيمة عبر set_param, أو null
   *[no] تُحدَّد القيمة عبر set_param
}
op-param-shape-path-option = { $nullable ->
    [yes] المتوقع مسار شكل (كائن بالمفتاح d) أو null
   *[no] المتوقع مسار شكل (كائن بالمفتاح d)
}
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
deck-inline-data = يخزّن العنصر { $element } بياناته داخل خياراته: ملف العرض هذا ليس بالتنسيق الحالي
sources-not-sources = هذا ليس أرشيف مصادر Oku Slides
sources-unsupported-version = كُتبت هذه المصادر بإصدار أحدث من Oku Slides
sources-bad-name = اسم ملف مرفوض في المصادر
sources-missing-deck = العرض التقديمي غير موجود في المصادر
sources-too-large = مصادر هذا الملف أكبر من أن تُفتح
sources-too-deep = هذا العرض يضمّن العناصر بعمق مفرط
sources-base-refused = ملف بيانات في هذا العرض تالف أو تعذّر حفظه

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
issue-invalid-style = قيمة نمط غير صالحة: { $field } ({ $reason ->
        [valign] { op-value-valign }
        [out-of-bounds] { op-value-out-of-bounds }
       *[other] { op-value-unreadable }
    })
issue-missing-asset = مورد مفقود: { $asset }
issue-pack-version-mismatch = أُهملت نسخة حزمة معدَّلة: { $version }
issue-pack-version-missing = نسخة الحزمة مفقودة من المصادر: { $version }
issue-pack-pin-mismatch = الحزمة المضمّنة { $pack } لا تطابق ما يتوقعه العرض.
issue-pack-tokens-invalid = تحتوي الحزمة المضمّنة { $pack } على رمز مكوّن غير صالح: { $reason }
issue-pack-kind-mismatch = الحزمة المضمّنة { $pack } ليست من النوع الذي يتوقعه العرض (أدوات أو تخطيطات).
issue-pack-kind-conflict = مساحة الأسماء { $pack } مثبّتة كحزمة أدوات وكحزمة تخطيطات في آن واحد: لا يمكن استخدام إلا واحدة.
issue-pack-requirement-missing = تحتاج حزمة { $pack } إلى { $required }، وهي غير موجودة في هذا العرض.
issue-theme-version-mismatch = أُهملت نسخة سمة معدَّلة: { $version }
issue-theme-version-missing = نسخة السمة مفقودة من المصادر: { $version }
issue-theme-pin-mismatch = السمة المضمّنة { $namespace } لا تطابق ما يتوقعه العرض.
issue-theme-closure = السمات المثبّتة في هذا العرض ليست سمته وآباءها بالضبط ({ $namespace }).
issue-usage-line-invalid = السطر { $line } من سجل الاستخدام غير قابل للقراءة، تم تجاهله
issue-usage-line-forged = دورة الاستخدام { $turn } سُجّلت مرتين بمحتوى مختلف، تم تجاهل إحداهما
issue-usage-too-large = سجل الاستخدام كبير جدًا، تم تجاهله
issue-usage-unreadable = سجل الاستخدام غير قابل للقراءة، تم تجاهله
issue-usage-lines-dropped = { $count ->
    [zero] لم يُتجاهل أي سطر آخر من سجل الاستخدام
    [one] تم تجاهل { $count } سطر آخر من سجل الاستخدام
    [two] تم تجاهل { $count } سطرين آخرين من سجل الاستخدام
    [few] تم تجاهل { $count } أسطر أخرى من سجل الاستخدام
    [many] تم تجاهل { $count } سطرًا آخر من سجل الاستخدام
   *[other] تم تجاهل { $count } سطر آخر من سجل الاستخدام
}
usage-range-bad-date = «{ $date }» ليس تاريخًا: استخدم YYYY-MM-DD.
usage-range-reversed = تاريخ البداية { $since } بعد تاريخ النهاية { $until }.
pack-token-entry-invalid = رمز مكوّن غير صالح «{ $path }» في بيان الحزمة.
pack-manifest-json = تعذّرت قراءة بيان الحزمة: { $reason }
pack-manifest-order = ترتيب عناصر الحزمة غير صالح عند «{ $name }».
pack-manifest-data-count = تتجاوز بيانات الحزمة الحد الأقصى { $max }، العدد: { $count }.
pack-manifest-data-base = تشير بيانات الحزمة «{ $data }» إلى ملف خارجي؛ بيانات الحزمة تحمل خلاياها الخاصة.
pack-manifest-data-cells = تتجاوز خلايا بيانات الحزمة الحد الأقصى { $max }، العدد: { $count }.
issue-unsupported-link = رابط غير مدعوم: { $link }
issue-highlight-zero = تُرقَّم ألوان التمييز بدءًا من 1
issue-unreadable-element = عنصر غير مقروء، نُحّي جانبًا
issue-slot-repaired = عنصر بلا خانة معروفة، وُضع في { $slot }
issue-unreadable-item = بند قائمة غير مقروء، نُحّي جانبًا
issue-unreadable-row = صف جدول غير مقروء، نُحّي جانبًا
issue-data-truncated = البيانات «{ $data }» كبيرة جدًا، اقتُطعت عند القراءة وأُبقيت للقراءة فقط
issue-data-name-taken = اسم البيانات «{ $data }» مستخدم بالفعل
issue-data-cell-type = البيانات «{ $data }»: العمود «{ $column }» يحتوي على قيم ليست من النوع { $type }
issue-data-too-many = بيانات كثيرة جدًا في العرض ({ $count })؛ لم تُقرأ: { $dropped }
issue-data-base-missing = البيانات «{ $data }»: ملفها الأساسي مفقود، أُبقيت للقراءة فقط
issue-data-edits-lost = البيانات «{ $data }»: فُقدت عند الدمج تعديلات على جدول مستبدَل
issue-data-inline-dropped = العنصر { $element }: خزّن إصدار أقدم بيانات مخططه أو جدوله داخل خياراته؛ أُزيلت
issue-more = مشكلات أخرى غير معروضة: { $count }
op-data-deck-full = ستتجاوز بيانات العرض حدّها: { $max } على الأكثر من { $what ->
    [row-ids] معرّفات الصفوف
    [overlay] الخلايا المعدّلة
    [text] بايتات النص
   *[data] البيانات
}
op-data-missing = لم يُعثر على البيانات «{ $data }»
op-data-name-taken = اسم البيانات «{ $name }» مستخدم بالفعل
op-data-column-in-use = يعرض العمود «{ $column }» كل من { $views }؛ اربطها بأعمدة أخرى أولًا (bind_data مع columns)
op-data-in-use = تعرض هذه البيانات { $views }{ $more ->
    [0] {""}
    [one] {" "}و{ $more } عرض آخر
   *[other] {" "}و{ $more } عروض أخرى
}؛ احذفها أو اربطها ببيانات أخرى أولًا
op-data-column-missing = لا تحتوي البيانات «{ $data }» على العمود «{ $column }»
op-data-not-view = لا يمكن للمعامل «{ $param }» عرض البيانات: وحده خيار مخطط أو جدول يمكنه ذلك
op-data-truncated = اقتُطعت البيانات «{ $data }» عند القراءة وهي للقراءة فقط؛ استبدل جدولها كاملًا لتعديلها
op-data-rebind = العنصر { $subject } يعرض بيانات أخرى بالفعل؛ استخدم bind_data لتغييرها
op-data-internal = لا يمكن لدفعة أن تعيّن أساس جدول أو طبقته
op-data-batch-too-large = الدفعة كبيرة جدًا: عدد الخلايا { $count }، والحد الأقصى { $max }
op-data-inline = يحمل العنصر { $subject } بياناته داخل خياراته؛ أضفها كبيانات للعرض باستخدام add_data واربطها باستخدام bind_data
issue-unreadable-slide = شريحة غير مقروءة، نُحّيت جانبًا
issue-unreadable-field = حقل العرض { $field } غير مقروء، أُعيد تعيينه
issue-knowledge-unreadable = إدخال معرفة غير مقروء { $id }، نُحّي جانبًا
issue-knowledge-entries-dropped = إدخالات معرفة أُسقطت دون قراءة: { $count } ({ $cause ->
        [files] ملفات أكثر من أن تُقرأ
        [file-bytes] ملف أكبر من أن يُقرأ
        [total-bytes] مجموعة أكبر من أن تُقرأ
       *[unreadable] إدخالات أخرى غير مقروءة
    })
issue-knowledge-over-limit = مجموعة المعرفة تتجاوز حدًا: { $limit ->
        [knowledge-reason-too-many-files] { knowledge-reason-too-many-files }
        [knowledge-reason-file-too-large] { knowledge-reason-file-too-large }
       *[knowledge-reason-total-too-large] { knowledge-reason-total-too-large }
    } (الملفات المتأثرة: { $count }، أولها { $id })؛ تُحفظ كل الملفات؛ حذف الملفات وحده يعيد الحزمة تحت الحد (تُرفض الكتابات والتعديلات حتى ذلك الحين)
issue-knowledge-path-invalid = نُقل ملف المعرفة { $id } إلى /recovered: { $reason ->
        [knowledge-path-empty] { knowledge-path-empty }
        [knowledge-path-not-absolute] { knowledge-path-not-absolute }
        [knowledge-path-bad-segment] { knowledge-path-bad-segment }
        [knowledge-path-dot-segment] { knowledge-path-dot-segment }
        [knowledge-path-reserved] { knowledge-path-reserved }
        [knowledge-path-too-deep] { knowledge-path-too-deep }
        [knowledge-path-too-long] { knowledge-path-too-long }
       *[knowledge-path-not-markdown] { knowledge-path-not-markdown }
    }
issue-knowledge-path-collision = كانت عدة ملفات معرفة في { $path }: أُعيدت تسمية الأخرى
issue-knowledge-invalid = ملف المعرفة { $path } غير صالح: { $reason ->
        [knowledge-frontmatter-missing] { knowledge-frontmatter-missing }
        [knowledge-frontmatter-unterminated] { knowledge-frontmatter-unterminated }
        [knowledge-frontmatter-too-large] { knowledge-frontmatter-too-large }
        [knowledge-frontmatter-not-yaml] { knowledge-frontmatter-not-yaml }
        [knowledge-frontmatter-not-mapping] { knowledge-frontmatter-not-mapping }
        [knowledge-frontmatter-duplicate-key] { knowledge-frontmatter-duplicate-key }
        [knowledge-frontmatter-alias] { knowledge-frontmatter-alias }
        [knowledge-frontmatter-tag] { knowledge-frontmatter-tag }
        [knowledge-frontmatter-too-deep] { knowledge-frontmatter-too-deep }
        [knowledge-frontmatter-after-document-end] { knowledge-frontmatter-after-document-end }
        [knowledge-frontmatter-flow-mapping] { knowledge-frontmatter-flow-mapping }
        [knowledge-frontmatter-type-missing] { knowledge-frontmatter-type-missing }
        [knowledge-frontmatter-bad-timestamp] { knowledge-frontmatter-bad-timestamp }
       *[knowledge-frontmatter-not-allowed] { knowledge-frontmatter-not-allowed }
    }
op-invalid-chart = المخطط { $subject }: { $reason }
op-invalid-table = الجدول { $subject }: { $reason }

replica-corrupted = سجل العرض التقديمي في هذا الملف تالف
replica-too-large = سجل العرض التقديمي في هذا الملف أكبر من أن يُفتح
replica-write = تعذّر تحديث سجل العرض التقديمي
replica-unrelated = لا يشترك هذان العرضان التقديميان في أصل واحد
replica-unsupported-version = يستخدم هذا العرض التقديمي تنسيقًا لا يستطيع هذا الإصدار من Oku Slides قراءته
replica-too-deep = تعذّر حفظ هذا العرض التقديمي: توجد عناصر محذوفة متداخلة بعمق كبير في سجلّه. تراجع عن آخر عمليات النقل، ثم احفظ مرة أخرى.

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
widget-fault-column-type = الودجة { $widget }: يمكن أن يكون عمود القائمة plain أو rich أو number أو flag أو choice أو asset (أو p.list متداخلة، مستوى واحد)؛ للمحتوى الحر لكل بند، استخدم p.slot مع ودجة فرعية
widget-fault-reserved-name = الودجة { $widget }: اسم محجوز: id أو kind أو اسم يحتوي على /
widget-fault-list-bounds = الودجة { $widget }: الحد الأقصى للقائمة أقل من حدها الأدنى
widget-fault-list-default-too-large = الودجة { $widget }: القيمة الافتراضية لقائمة تتجاوز حدود حجم القوائم
widget-fault-list-depth = الودجة { $widget }: لا يمكن أن تحتوي قائمة متداخلة على عمود قائمة؛ تتداخل القوائم بمستوى واحد فقط
widget-fault-list-default-below-min = الودجة { $widget }: القيمة الافتراضية لقائمة متداخلة تضم بنودًا أقل من حدها الأدنى
widget-fault-slide-sizing = الودجة { $widget } شريحة: ودجات العناصر وحدها تصرّح بـ grow أو typography
widget-fault-pack-category = الودجة { $widget } لا تطابق حزمتها: حزم التخطيطات تحوي ودجات الشرائح وحزم الودجات تحوي ودجات العناصر
widget-fault-unqualified = الودجة { $widget } بلا مساحة أسماء حزمة
widget-fault-too-long = الودجة { $widget }: اسم أو تسمية أو قيمة أو وصف أو قائمة تتجاوز الحد المسموح
issue-empty-item-id = القائمة { $param } تحتوي على بند بلا معرّف
issue-duplicate-item-id = القائمة { $param } تحتوي على معرّف بند مكرر
issue-invalid-item-id = القائمة { $param } فيها معرّف بند يحتوي على شرطة مائلة
issue-list-bounds = القائمة { $param } تحتوي على عدد بنود قليل جدًا أو كثير جدًا
issue-view-truncated = { $element } يعرض جزءًا فقط من بياناته ({ $what ->
    [rows] الصفوف
   *[columns] الأعمدة
}: { $shown } من { $total })
issue-render-budget = لم يُرسم { $element }: طرق عرض العرض التقديمي تقرأ بيانات أكثر مما يسمح به العرض
issue-data-missing = يشير { $element } إلى البيانات «{ $data }» غير الموجودة في العرض التقديمي
issue-view-column-missing = يقرأ { $element } العمود «{ $column }» غير الموجود في بياناته
issue-view-unreadable = لم يُرسم { $element }: لا يمكن قراءة خياراته
op-replica-full = يقترب سجل العرض من { $max } ميبيبايت، وهو الحد الأقصى لإعادة فتحه: أزل محتوى قبل إضافة المزيد.
