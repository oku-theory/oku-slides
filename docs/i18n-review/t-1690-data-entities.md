# Translation review: strings added or changed by t-1690

Data entities, phase 1 (commit `2459f300e`). Every Fluent message this commit added or
changed, in the 18 machine-translated locales (en and fr are written by hand).
Files: chart.ftl, core.ftl, data.ftl, document.ftl, mcp.ftl, session.ftl, ui.ftl under `crates/oku-slides-i18n/src/locales/<locale>/`.

For a human translator: check each translation against the English; keep `{ $variables }`,
code tokens and JSON field names unchanged. Fix wording in the locale's `.ftl` file directly.
Generated from the commit diff; nothing was retranslated. Entries: 1656.

## ar


### ar/chart.ftl

| Key | English | ar |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | يتطلب `columns` وجود `data` ويسرد من 1 إلى 64 معرّف عمود صالحًا ومختلفًا |
| `chart-ids-duplicate` | a row or column id is repeated | معرّف صف أو عمود مكرر |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | يتكون معرّف الصف أو العمود من 1 إلى 64 حرفًا أو رقمًا أو - أو _ |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | يتطلب dataset.ids معرّفًا لكل صف ويتطلب dataset.columns معرّفًا لكل عمود |
| `chart-no-dataset` | the chart has no data table (dataset) | لا يحتوي الرسم البياني على جدول بيانات (dataset) |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | لا يمكن لتصحيح الخيارات تغيير `dataset`: استخدم edit_data |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | مع وجود dataset، يجب أن تبقى البيانات فيه: عدّلها باستخدام edit_data |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | لا يمكن لخيار أن يحمل مرجع بيانات وبيانات مضمّنة معًا: احتفظ بأحدهما |
| `chart-ref-null` | `data` cannot be null: remove the key instead | لا يمكن أن تكون `data` قيمة null: احذف المفتاح بدلًا من ذلك |

### ar/core.ftl

| Key | English | ar |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | يخزّن العنصر { $element } بياناته داخل خياراته: ملف العرض هذا ليس بالتنسيق الحالي |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | البيانات «{ $data }»: ملفها الأساسي مفقود، أُبقيت للقراءة فقط |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | البيانات «{ $data }»: العمود «{ $column }» يحتوي على قيم ليست من النوع { $type } |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | البيانات «{ $data }»: فُقدت عند الدمج تعديلات على جدول مستبدَل |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | العنصر { $element }: خزّن إصدار أقدم بيانات مخططه أو جدوله داخل خياراته؛ أُزيلت |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | يشير { $element } إلى البيانات «{ $data }» غير الموجودة في العرض التقديمي |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | اسم البيانات «{ $data }» مستخدم بالفعل |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | بيانات كثيرة جدًا في العرض ({ $count })؛ لم تُقرأ: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | البيانات «{ $data }» كبيرة جدًا، اقتُطعت عند القراءة وأُبقيت للقراءة فقط |
| `issue-more` | more issues not shown: { $count } | مشكلات أخرى غير معروضة: { $count } |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | لم يُرسم { $element }: طرق عرض العرض التقديمي تقرأ بيانات أكثر مما يسمح به العرض |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | يقرأ { $element } العمود «{ $column }» غير الموجود في بياناته |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element } يعرض جزءًا فقط من بياناته ({ $what -><br>    [rows] الصفوف<br>   *[columns] الأعمدة<br>}: { $shown } من { $total }) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | لم يُرسم { $element }: لا يمكن قراءة خياراته |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | الدفعة كبيرة جدًا: عدد الخلايا { $count }، والحد الأقصى { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | يعرض العمود «{ $column }» كل من { $views }؛ اربطها بأعمدة أخرى أولًا (bind_data مع columns) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | لا تحتوي البيانات «{ $data }» على العمود «{ $column }» |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | ستتجاوز بيانات العرض حدّها: { $max } على الأكثر من { $what -><br>    [row-ids] معرّفات الصفوف<br>    [overlay] الخلايا المعدّلة<br>    [text] بايتات النص<br>   *[data] البيانات<br>} |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | تعرض هذه البيانات { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}و{ $more } عرض آخر<br>   *[other] {" "}و{ $more } عروض أخرى<br>}؛ احذفها أو اربطها ببيانات أخرى أولًا |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | يحمل العنصر { $subject } بياناته داخل خياراته؛ أضفها كبيانات للعرض باستخدام add_data واربطها باستخدام bind_data |
| `op-data-internal` | A batch cannot set a table's base or overlay | لا يمكن لدفعة أن تعيّن أساس جدول أو طبقته |
| `op-data-missing` | Data “{ $data }” not found | لم يُعثر على البيانات «{ $data }» |
| `op-data-name-taken` | The data name “{ $name }” is already taken | اسم البيانات «{ $name }» مستخدم بالفعل |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | لا يمكن للمعامل «{ $param }» عرض البيانات: وحده خيار مخطط أو جدول يمكنه ذلك |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | العنصر { $subject } يعرض بيانات أخرى بالفعل؛ استخدم bind_data لتغييرها |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | اقتُطعت البيانات «{ $data }» عند القراءة وهي للقراءة فقط؛ استبدل جدولها كاملًا لتعديلها |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | يقترب سجل العرض من { $max } ميبيبايت، وهو الحد الأقصى لإعادة فتحه: أزل محتوى قبل إضافة المزيد. |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | تشير بيانات الحزمة «{ $data }» إلى ملف خارجي؛ بيانات الحزمة تحمل خلاياها الخاصة. |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | تتجاوز خلايا بيانات الحزمة الحد الأقصى { $max }، العدد: { $count }. |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | تتجاوز بيانات الحزمة الحد الأقصى { $max }، العدد: { $count }. |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | ملف بيانات في هذا العرض تالف أو تعذّر حفظه |

### ar/data.ftl

| Key | English | ar |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | الملف الأساسي للبيانات تالف أو لا يطابق جدولها |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | الجدول الذي له ملف أساسي يعطي معرّفات صفوفه فقط، دون خلايا |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | قيمة لا تناسب العمود «{ $column }» الذي يحتوي على { $type -><br>    [number] أرقام<br>    [integer] أعداد صحيحة<br>    [bool] صواب أو خطأ<br>    [date] تواريخ (YYYY-MM-DD)<br>   *[text] نص لا يتجاوز 16 KB<br>} |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | لا يمكن تحويل العمود «{ $column }» إلى { $type -><br>    [number] عمود أرقام<br>    [integer] عمود أعداد صحيحة<br>    [bool] عمود صواب/خطأ<br>    [date] عمود تواريخ<br>   *[text] عمود نص<br>} دون فقدان، راجع الصفوف { $rows }{ $more -><br>    [0] {""}<br>   *[other] {" "}وغيرها (العدد: { $more })<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | قائمة الأعمدة فارغة أو تذكر عمودًا مرتين |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | المعرّف «{ $id }» غير صالح: استخدم من 1 إلى 64 حرفًا لاتينيًا أو رقمًا أو - أو _ |
| `data-id-missing` | a new row, column or table has no id | صف أو عمود أو جدول جديد بلا معرّف |
| `data-id-taken` | id { $id } is already used in this table | المعرّف { $id } مستخدم بالفعل في هذا الجدول |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | «{ $name }» ليس اسم بيانات صالحًا: استخدم أحرفًا لاتينية صغيرة وأرقامًا و_ دون البدء برقم |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | تحفظ هذه البيانات صفوفها في ملف أساسي لا يستطيع هذا التعديل قراءته بعد |
| `data-no-column` | column { $column } does not exist | العمود { $column } غير موجود |
| `data-no-row` | row { $row } does not exist | الصف { $row } غير موجود |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | أرقام غير منتهية (NaN أو لانهاية)، العدد: { $count } |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | صفوف فيها خلايا أكثر من الترويسة، وحُذفت الخلايا الزائدة، العدد: { $count } |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | يحتوي صف على خلايا أكثر من أعمدة الجدول (العدد: { $max }) |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] ستتجاوز البيانات حدّ الصفوف (الحد: { $max })<br>    [columns] ستتجاوز البيانات حدّ الأعمدة (الحد: { $max })<br>    [text] ستتجاوز خلية نصية حدّها بالبايت (الحد: { $max })<br>   *[overlay] ستتجاوز البيانات حدّ الخلايا المعدّلة (الحد: { $max })<br>} |

### ar/document.ftl

| Key | English | ar |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | قيم كثيرة جدًا للرسم ({ $count }، الحد الأقصى { $max }) |

### ar/mcp.ftl

| Key | English | ar |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | لا توجد بيانات بالمعرّف «{ $data }»؛ تسردها data_list. |

### ar/session.ftl

| Key | English | ar |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | لا يمكن لمجلد بيانات التطبيق حفظ ملفات بيانات العروض |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | لا يمكن تغيير نوع { $subject }: تعرض عروض أخرى بياناته، وهي تفتقر إلى الأعمدة التي يحتاجها هذا النوع. |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | تغيّرت البيانات «{ $data }» أثناء قراءة صفوفها المحفوظة؛ أعد المحاولة |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | مخزن البيانات ممتلئ: تعذّر ضغط { $data }. حرّر مساحة ضمن [sources] max_store_mib. |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | استبدل الدمج تعديلات على { $data }: تم مسح سجل التراجع |
| `session-data-user` | { $element } on slide { $slide } | { $element } في الشريحة { $slide } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | لا يمكن إضافة أعمدة أو إزالتها هنا: لا يمكن للعرض { $view } متابعتها |
| `session-example-base-chart-category` | Category | الفئة |
| `session-example-base-chart-value` | Value | القيمة |
| `session-request-too-large` | The request is larger than { $mib } MiB. | الطلب أطول من { $mib } ميغابايت. |
| `session-undo-data-add` | Add data | إضافة بيانات |
| `session-undo-data-bind` | Bind data | ربط البيانات |
| `session-undo-data-edit` | Edit data | تعديل البيانات |
| `session-undo-data-remove` | Remove data | حذف البيانات |
| `session-undo-data-rename` | Rename data | إعادة تسمية البيانات |
| `session-undo-data-type` | Change the column type | تغيير نوع العمود |

### ar/ui.ftl

| Key | English | ar |
|---|---|---|
| `ui-data-add` | Add data | إضافة بيانات |
| `ui-data-column-type` | Column type | نوع العمود |
| `ui-data-next` | Next rows | الصفوف التالية |
| `ui-data-open` | Open the data | فتح البيانات |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | الصفوف { $first }–{ $last } من { $total } |
| `ui-data-previous` | Previous rows | الصفوف السابقة |
| `ui-data-remove` | Remove | إزالة |
| `ui-data-rename` | Rename | إعادة التسمية |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | بيانات مشتركة · { $count -><br>    [zero] { $count } عرض<br>    [one] { $count } عرض<br>    [two] { $count } عرضان<br>    [few] { $count } عروض<br>    [many] { $count } عرضًا<br>   *[other] { $count } عرض<br>} |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | الأعمدة: { $columns } · الصفوف: { $rows } |
| `ui-data-tab` | Data | البيانات |
| `ui-data-type-bool` | Yes/no | نعم/لا |
| `ui-data-type-date` | Date (YYYY-MM-DD) | تاريخ |
| `ui-data-type-integer` | Integer | عدد صحيح |
| `ui-data-type-number` | Number | رقم |
| `ui-data-type-text` | Text | نص |
| `ui-data-unused` | Unused | غير مستخدمة |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count -><br>    [zero] { $count } عروض<br>    [one] عرض واحد<br>    [two] عرضان<br>    [few] { $count } عروض<br>    [many] { $count } عرضًا<br>   *[other] { $count } عرض<br>} |

## bn


### bn/chart.ftl

| Key | English | bn |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns`-এর জন্য `data` লাগে এবং এটি ১ থেকে ৬৪টি বৈধ, ভিন্ন কলাম আইডি তালিকাভুক্ত করে |
| `chart-ids-duplicate` | a row or column id is repeated | একটি সারি বা কলামের id পুনরাবৃত্ত হয়েছে |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | সারি বা কলামের id-তে 1 থেকে 64টি অক্ষর, অঙ্ক, - বা _ থাকে |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids-এ প্রতি সারির জন্য একটি id এবং dataset.columns-এ প্রতি কলামের জন্য একটি id লাগে |
| `chart-no-dataset` | the chart has no data table (dataset) | চার্টে কোনো ডেটা টেবিল (dataset) নেই |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | অপশন প্যাচ `dataset` পরিবর্তন করতে পারে না: edit_data ব্যবহার করুন |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | dataset থাকলে ডেটা dataset-এই থাকতে হবে: edit_data দিয়ে সম্পাদনা করুন |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | একটি অপশনে ডেটা রেফারেন্স ও ইনলাইন ডেটা দুটোই থাকতে পারে না: একটি রাখুন |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` null হতে পারে না: বরং কী-টি সরান |

### bn/core.ftl

| Key | English | bn |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | উপাদান { $element } তার ডেটা তার বিকল্পে রাখে: এই ডেক ফাইলটি বর্তমান ফরম্যাটে নেই |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | ডেটা “{ $data }”: এর মূল ফাইল নেই, শুধু-পঠনযোগ্য রাখা হয়েছে |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | ডেটা “{ $data }”: কলাম “{ $column }”-এ এমন মান আছে যা { $type } ধরনের নয় |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | ডেটা “{ $data }”: প্রতিস্থাপিত টেবিলের সম্পাদনা একত্রীকরণে হারিয়ে গেছে |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | উপাদান { $element }: একটি পুরোনো সংস্করণ এর চার্ট বা টেবিলের ডেটা এর বিকল্পে রেখেছিল; সেগুলি সরানো হয়েছে |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element } “{ $data }” ডেটার নাম দেয়, যা ডেকে নেই |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | ডেটার নাম “{ $data }” আগে থেকেই ব্যবহৃত |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | ডেকে অনেক বেশি ডেটা ({ $count }); পড়া হয়নি: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | ডেটা “{ $data }” খুব বড়, পড়ার সময় ছেঁটে শুধু-পঠনযোগ্য রাখা হয়েছে |
| `issue-more` | more issues not shown: { $count } | আরও সমস্যা দেখানো হয়নি: { $count } |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | { $element } আঁকা হয়নি: ডেকের ভিউগুলো একটি রেন্ডারের অনুমতির চেয়ে বেশি ডেটা পড়ে |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element } “{ $column }” কলাম পড়ে, যা এর ডেটায় নেই |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element } তার ডেটার শুধু একটি অংশ দেখায় ({ $what -><br>    [rows] সারি<br>   *[columns] কলাম<br>}: { $total }টির মধ্যে { $shown }টি) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | { $element } আঁকা হয়নি: এর অপশনগুলো পড়া যায় না |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | ব্যাচটি খুব বড়: সেলের সংখ্যা { $count }, সর্বোচ্চ { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | কলাম “{ $column }” দেখায় { $views }; আগে সেগুলোকে অন্য কলামে যুক্ত করুন (columns সহ bind_data) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | ডেটা “{ $data }”-তে কলাম “{ $column }” নেই |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | ডেকের ডেটা তার সীমা ছাড়িয়ে যাবে: সর্বোচ্চ { $max } { $what -><br>    [row-ids] সারি আইডি<br>    [overlay] সম্পাদিত ঘর<br>    [text] বাইট লেখা<br>   *[data] ডেটা<br>} |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | এই ডেটা দেখায় { $views }{ $more -><br>    [0] {""}<br>   *[other] {" "}এবং আরও { $more }টি ভিউ<br>}; আগে সেগুলো সরান বা অন্য ডেটার সঙ্গে যুক্ত করুন |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | উপাদান { $subject } তার ডেটা তার বিকল্পে বহন করে; add_data দিয়ে সেগুলি ডেকের ডেটা হিসেবে যোগ করুন এবং bind_data দিয়ে যুক্ত করুন |
| `op-data-internal` | A batch cannot set a table's base or overlay | একটি ব্যাচ কোনো টেবিলের ভিত্তি বা ওভারলে সেট করতে পারে না |
| `op-data-missing` | Data “{ $data }” not found | ডেটা “{ $data }” পাওয়া যায়নি |
| `op-data-name-taken` | The data name “{ $name }” is already taken | ডেটার নাম “{ $name }” আগে থেকেই ব্যবহৃত |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | প্যারামিটার “{ $param }” ডেটা দেখাতে পারে না: শুধু চার্ট বা টেবিলের অপশন পারে |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | উপাদান { $subject } ইতিমধ্যে অন্য ডেটা দেখাচ্ছে; বদলাতে bind_data ব্যবহার করুন |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | ডেটা “{ $data }” পড়ার সময় ছেঁটে ফেলা হয়েছে এবং শুধু পড়ার জন্য; সম্পাদনা করতে পুরো টেবিলটি বদলান |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | উপস্থাপনার ইতিহাস প্রায় { $max } MiB, যা আবার খোলার সর্বোচ্চ সীমা: আরও যোগ করার আগে বিষয়বস্তু সরান। |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | প্যাকের ডেটা “{ $data }” একটি বাহ্যিক ফাইল নির্দেশ করে; প্যাকের ডেটা নিজস্ব সেল ধারণ করে। |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | প্যাকের ডেটায় { $count }টি সেল আছে, সর্বোচ্চ { $max }। |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | প্যাকে { $count }টি ডেটা আছে, সর্বোচ্চ { $max }। |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | এই ডেকের একটি ডেটা ফাইল ক্ষতিগ্রস্ত বা সংরক্ষণ করা যায়নি |

### bn/data.ftl

| Key | English | bn |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | ডেটার ভিত্তি ফাইলটি ক্ষতিগ্রস্ত অথবা তার টেবিলের সাথে মেলে না |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | বেস ফাইলযুক্ত টেবিল তার সারিগুলোর শুধু আইডি দেয়, ঘর নয় |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | একটি মান কলাম “{ $column }”-এ খাপ খায় না, যাতে থাকে { $type -><br>    [number] সংখ্যা<br>    [integer] পূর্ণসংখ্যা<br>    [bool] সত্য বা মিথ্যা<br>    [date] তারিখ (YYYY-MM-DD)<br>   *[text] সর্বোচ্চ 16 KB লেখা<br>} |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | কলাম “{ $column }”-কে ক্ষতি ছাড়া { $type -><br>    [number] সংখ্যার কলাম<br>    [integer] পূর্ণসংখ্যার কলাম<br>    [bool] সত্য/মিথ্যা কলাম<br>    [date] তারিখের কলাম<br>   *[text] লেখার কলাম<br>} করা যায় না, সারিগুলো দেখুন: { $rows }{ $more -><br>    [0] {""}<br>   *[other] {" "}এবং আরও { $more }টি<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | কলামের তালিকা খালি বা একটি কলাম দুবার উল্লেখ করে |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | আইডি “{ $id }” বৈধ নয়: 1 থেকে 64টি অক্ষর, অঙ্ক, - বা _ ব্যবহার করুন |
| `data-id-missing` | a new row, column or table has no id | নতুন সারি, কলাম বা টেবিলের আইডি নেই |
| `data-id-taken` | id { $id } is already used in this table | আইডি { $id } এই টেবিলে আগেই ব্যবহৃত |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | “{ $name }” বৈধ ডেটা নাম নয়: ছোট হাতের অক্ষর, অঙ্ক ও _ ব্যবহার করুন, অঙ্ক দিয়ে শুরু করবেন না |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | এই ডেটা তার সারিগুলো একটি বেস ফাইলে রাখে, যা এই সম্পাদনা এখনও পড়তে পারে না |
| `data-no-column` | column { $column } does not exist | কলাম { $column } নেই |
| `data-no-row` | row { $row } does not exist | সারি { $row } নেই |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | { $count }টি সংখ্যা সসীম নয় (NaN বা অসীম) |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | { $count }টি সারিতে শিরোনামের চেয়ে বেশি ঘর ছিল; বাড়তি ঘরগুলো সরানো হয়েছে |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | একটি সারিতে টেবিলের { $max }টি কলামের চেয়ে বেশি ঘর আছে |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] ডেটা সারির সীমা ছাড়িয়ে যাবে (সীমা: { $max })<br>    [columns] ডেটা কলামের সীমা ছাড়িয়ে যাবে (সীমা: { $max })<br>    [text] একটি লেখার ঘর বাইটের সীমা ছাড়িয়ে যাবে (সীমা: { $max })<br>   *[overlay] ডেটা সম্পাদিত ঘরের সীমা ছাড়িয়ে যাবে (সীমা: { $max })<br>} |

### bn/document.ftl

| Key | English | bn |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | আঁকার জন্য অনেক বেশি মান ({ $count }, সর্বোচ্চ { $max }) |

### bn/mcp.ftl

| Key | English | bn |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | “{ $data }” আইডির কোনো ডেটা নেই; data_list সেগুলোর তালিকা দেয়। |

### bn/session.ftl

| Key | English | bn |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | অ্যাপের ডেটা ফোল্ডারে ডেকের ডেটা ফাইল রাখা যাচ্ছে না |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | { $subject }-এর ধরন বদলানো যায় না: এর ডেটা অন্য ভিউতেও দেখানো হয়, আর তাতে এই ধরনের দরকারি কলাম নেই। |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | সংরক্ষিত সারি পড়ার সময় ডেটা “{ $data }” বদলে গেছে; আবার চেষ্টা করুন |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | ডেটা ভান্ডার পূর্ণ: { $data } সংকুচিত করা যায়নি। [sources] max_store_mib-এর অধীনে জায়গা খালি করুন। |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | মার্জ { $data }-এর সম্পাদনাগুলি প্রতিস্থাপন করেছে: পূর্বাবস্থার ইতিহাস মুছে ফেলা হয়েছে |
| `session-data-user` | { $element } on slide { $slide } | স্লাইড { $slide }-এ { $element } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | এখানে কলাম যোগ বা সরানো যাবে না: ভিউ { $view } সেগুলো অনুসরণ করতে পারে না |
| `session-example-base-chart-category` | Category | ক্যাটাগরি |
| `session-example-base-chart-value` | Value | মান |
| `session-request-too-large` | The request is larger than { $mib } MiB. | অনুরোধটি { $mib } MiB-এর চেয়ে বড়। |
| `session-undo-data-add` | Add data | ডেটা যোগ করুন |
| `session-undo-data-bind` | Bind data | ডেটা যুক্ত করুন |
| `session-undo-data-edit` | Edit data | ডেটা সম্পাদনা করুন |
| `session-undo-data-remove` | Remove data | ডেটা সরান |
| `session-undo-data-rename` | Rename data | ডেটার নাম বদলান |
| `session-undo-data-type` | Change the column type | কলামের ধরন বদলান |

### bn/ui.ftl

| Key | English | bn |
|---|---|---|
| `ui-data-add` | Add data | ডেটা যোগ করুন |
| `ui-data-column-type` | Column type | কলামের ধরন |
| `ui-data-next` | Next rows | পরের সারি |
| `ui-data-open` | Open the data | ডেটা খুলুন |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | { $total }টির মধ্যে সারি { $first }–{ $last } |
| `ui-data-previous` | Previous rows | আগের সারি |
| `ui-data-remove` | Remove | সরান |
| `ui-data-rename` | Rename | নাম পরিবর্তন |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count }টি ভিউ দ্বারা শেয়ার করা |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | কলাম: { $columns } · সারি: { $rows } |
| `ui-data-tab` | Data | ডেটা |
| `ui-data-type-bool` | Yes/no | হ্যাঁ/না |
| `ui-data-type-date` | Date (YYYY-MM-DD) | তারিখ |
| `ui-data-type-integer` | Integer | পূর্ণসংখ্যা |
| `ui-data-type-number` | Number | সংখ্যা |
| `ui-data-type-text` | Text | টেক্সট |
| `ui-data-unused` | Unused | ব্যবহৃত নয় |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count }টি ভিউ |

## de


### de/chart.ftl

| Key | English | de |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns` braucht `data` und listet 1 bis 64 gültige, verschiedene Spalten-IDs |
| `chart-ids-duplicate` | a row or column id is repeated | eine Zeilen- oder Spalten-ID kommt doppelt vor |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | eine Zeilen- oder Spalten-ID hat 1 bis 64 Buchstaben, Ziffern, - oder _ |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids braucht eine ID pro Zeile und dataset.columns eine pro Spalte |
| `chart-no-dataset` | the chart has no data table (dataset) | das Diagramm hat keine Datentabelle (dataset) |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | ein Options-Patch kann `dataset` nicht ändern: edit_data verwenden |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | mit einem Dataset müssen die Daten im Dataset bleiben: mit edit_data bearbeiten |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | eine Option kann nicht zugleich einen Datenverweis und eingebettete Daten enthalten: nur eines behalten |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` darf nicht null sein: stattdessen den Schlüssel entfernen |

### de/core.ftl

| Key | English | de |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | Element { $element } speichert seine Daten in seinen Optionen: Diese Deck-Datei hat nicht das aktuelle Format |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | Daten „{ $data }“: ihre Basisdatei fehlt, schreibgeschützt behalten |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | Daten „{ $data }“: Spalte „{ $column }“ enthält Werte, die nicht vom Typ { $type } sind |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | Daten „{ $data }“: Änderungen an einer ersetzten Tabelle gingen beim Zusammenführen verloren |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | Element { $element }: Die Daten seines Diagramms oder seiner Tabelle wurden von einer älteren Version in seinen Optionen gespeichert; sie wurden entfernt |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element } nennt die Daten „{ $data }“, die nicht im Deck sind |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | der Datenname „{ $data }“ ist bereits vergeben |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | zu viele Daten im Deck ({ $count }); nicht gelesen: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | Daten „{ $data }“ zu groß, beim Lesen gekürzt und schreibgeschützt behalten |
| `issue-more` | more issues not shown: { $count } | weitere, nicht angezeigte Probleme: { $count } |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | { $element } wird nicht gezeichnet: die Ansichten des Decks lesen mehr Daten, als ein Rendern erlaubt |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element } liest die Spalte „{ $column }“, die seine Daten nicht haben |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element } zeigt nur einen Teil seiner Daten ({ $what -><br>    [rows] Zeilen<br>   *[columns] Spalten<br>}: { $shown } von { $total }) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | { $element } wird nicht gezeichnet: seine Optionen sind nicht lesbar |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | Der Stapel ist zu groß: Zellenanzahl { $count }, höchstens { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | Die Spalte „{ $column }“ wird von { $views } angezeigt; binden Sie diese zuerst an andere Spalten (bind_data mit columns) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | Die Daten „{ $data }“ haben keine Spalte „{ $column }“ |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | Die Daten des Decks würden ihr Limit überschreiten: höchstens { $max } { $what -><br>    [row-ids] Zeilen-IDs<br>    [overlay] geänderte Zellen<br>    [text] Byte Text<br>   *[data] Daten<br>} |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | Diese Daten werden von { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}und { $more } weiteren Ansicht<br>   *[other] {" "}und { $more } weiteren Ansichten<br>}{" "}angezeigt; entfernen Sie diese oder binden Sie sie zuerst an andere Daten |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | Element { $subject } trägt seine Daten in seinen Optionen; fügen Sie sie mit add_data als Deck-Daten hinzu und binden Sie sie mit bind_data |
| `op-data-internal` | A batch cannot set a table's base or overlay | Ein Stapel kann Basis oder Überlagerung einer Tabelle nicht setzen |
| `op-data-missing` | Data “{ $data }” not found | Daten „{ $data }“ nicht gefunden |
| `op-data-name-taken` | The data name “{ $name }” is already taken | Der Datenname „{ $name }“ ist bereits vergeben |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | Der Parameter „{ $param }“ kann keine Daten anzeigen: nur eine Diagramm- oder Tabellenoption kann das |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | Das Element { $subject } zeigt bereits andere Daten; zum Ändern bind_data verwenden |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | Die Daten „{ $data }“ wurden beim Lesen gekürzt und sind schreibgeschützt; zum Bearbeiten die ganze Tabelle ersetzen |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | Der Verlauf der Präsentation erreicht fast { $max } MiB, das Maximum zum erneuten Öffnen: Entfernen Sie Inhalte, bevor Sie weitere hinzufügen. |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | Die Daten „{ $data }“ des Packs verweisen auf eine externe Datei; die Daten eines Packs enthalten ihre eigenen Zellen. |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | Die Daten des Packs enthalten { $count -><br>    [one] { $count } Zelle<br>   *[other] { $count } Zellen<br>}, höchstens { $max }. |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | Das Pack enthält { $count } Daten, höchstens { $max }. |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | Eine Datendatei dieses Decks ist beschädigt oder konnte nicht gespeichert werden |

### de/data.ftl

| Key | English | de |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | die Basisdatei der Daten ist beschädigt oder passt nicht zu ihrer Tabelle |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | eine Tabelle mit Basisdatei gibt für ihre Zeilen nur IDs an, keine Zellen |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | ein Wert passt nicht in die Spalte „{ $column }“, die { $type -><br>    [number] Zahlen<br>    [integer] ganze Zahlen<br>    [bool] wahr oder falsch<br>    [date] Datumsangaben (JJJJ-MM-TT)<br>   *[text] Text bis 16 KB<br>} enthält |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | die Spalte „{ $column }“ kann nicht verlustfrei { $type -><br>    [number] eine Zahlenspalte<br>    [integer] eine Ganzzahlspalte<br>    [bool] eine Wahr-/Falsch-Spalte<br>    [date] eine Datumsspalte<br>   *[text] eine Textspalte<br>} werden, siehe Zeilen { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}und { $more } weitere Zeile<br>   *[other] {" "}und { $more } weitere Zeilen<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | die Spaltenliste ist leer oder nennt eine Spalte zweimal |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | die ID „{ $id }“ ist ungültig: 1 bis 64 Buchstaben, Ziffern, - oder _ verwenden |
| `data-id-missing` | a new row, column or table has no id | eine neue Zeile, Spalte oder Tabelle hat keine ID |
| `data-id-taken` | id { $id } is already used in this table | die ID { $id } wird in dieser Tabelle bereits verwendet |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | „{ $name }“ ist kein gültiger Datenname: Kleinbuchstaben, Ziffern und _ verwenden, nicht mit einer Ziffer beginnen |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | diese Daten halten ihre Zeilen in einer Basisdatei, die diese Bearbeitung noch nicht lesen kann |
| `data-no-column` | column { $column } does not exist | die Spalte { $column } existiert nicht |
| `data-no-row` | row { $row } does not exist | die Zeile { $row } existiert nicht |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | { $count -><br>    [one] { $count } Zahl ist nicht endlich (NaN oder unendlich)<br>   *[other] { $count } Zahlen sind nicht endlich (NaN oder unendlich)<br>} |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | { $count -><br>    [one] { $count } Zeile hatte mehr Zellen als die Kopfzeile; die überzähligen Zellen wurden entfernt<br>   *[other] { $count } Zeilen hatten mehr Zellen als die Kopfzeile; die überzähligen Zellen wurden entfernt<br>} |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | eine Zeile hat mehr Zellen als die { $max } { $max -><br>    [one] Spalte<br>   *[other] Spalten<br>} der Tabelle |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] die Daten würden ihr Limit von { $max } { $max -><br>        [one] Zeile<br>       *[other] Zeilen<br>    } überschreiten<br>    [columns] die Daten würden ihr Limit von { $max } { $max -><br>        [one] Spalte<br>       *[other] Spalten<br>    } überschreiten<br>    [text] eine Textzelle würde ihr Limit von { $max } { $max -><br>        [one] Byte<br>       *[other] Bytes<br>    } überschreiten<br>   *[overlay] die Daten würden ihr Limit von { $max } { $max -><br>        [one] bearbeiteten Zelle<br>       *[other] bearbeiteten Zellen<br>    } überschreiten<br>} |

### de/document.ftl

| Key | English | de |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | zu viele Werte zum Zeichnen ({ $count }, höchstens { $max }) |

### de/mcp.ftl

| Key | English | de |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | Keine Daten haben die ID „{ $data }“; data_list listet sie auf. |

### de/session.ftl

| Key | English | de |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | Der Datenordner der App kann die Datendateien der Decks nicht aufnehmen |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | Der Typ von { $subject } lässt sich nicht ändern: Andere Ansichten zeigen seine Daten, denen die Spalten fehlen, die dieser Typ braucht. |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | Die Daten „{ $data }“ haben sich beim Lesen ihrer gespeicherten Zeilen geändert; versuchen Sie es erneut |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | Der Datenspeicher ist voll: { $data } konnte nicht verdichtet werden. Geben Sie Platz unter [sources] max_store_mib frei. |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | Die Zusammenführung hat Änderungen an { $data } ersetzt: der Rückgängig-Verlauf wurde geleert |
| `session-data-user` | { $element } on slide { $slide } | { $element } auf Folie { $slide } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | Spalten können hier nicht hinzugefügt oder entfernt werden: Die Ansicht { $view } kann ihnen nicht folgen |
| `session-example-base-chart-category` | Category | Kategorie |
| `session-example-base-chart-value` | Value | Wert |
| `session-request-too-large` | The request is larger than { $mib } MiB. | Die Anfrage ist größer als { $mib } MiB. |
| `session-undo-data-add` | Add data | Daten hinzufügen |
| `session-undo-data-bind` | Bind data | Daten verknüpfen |
| `session-undo-data-edit` | Edit data | Daten bearbeiten |
| `session-undo-data-remove` | Remove data | Daten entfernen |
| `session-undo-data-rename` | Rename data | Daten umbenennen |
| `session-undo-data-type` | Change the column type | Spaltentyp ändern |

### de/ui.ftl

| Key | English | de |
|---|---|---|
| `ui-data-add` | Add data | Daten hinzufügen |
| `ui-data-column-type` | Column type | Spaltentyp |
| `ui-data-next` | Next rows | Nächste Zeilen |
| `ui-data-open` | Open the data | Daten öffnen |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | Zeilen { $first }–{ $last } von { $total } |
| `ui-data-previous` | Previous rows | Vorherige Zeilen |
| `ui-data-remove` | Remove | Entfernen |
| `ui-data-rename` | Rename | Umbenennen |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | Geteilt von { $count -><br>    [one] { $count } Ansicht<br>   *[other] { $count } Ansichten<br>} |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | Spalten: { $columns } · Zeilen: { $rows } |
| `ui-data-tab` | Data | Daten |
| `ui-data-type-bool` | Yes/no | Ja/Nein |
| `ui-data-type-date` | Date (YYYY-MM-DD) | Datum |
| `ui-data-type-integer` | Integer | Ganzzahl |
| `ui-data-type-number` | Number | Zahl |
| `ui-data-type-text` | Text | Text |
| `ui-data-unused` | Unused | Nicht verwendet |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count -><br>    [one] { $count } Ansicht<br>   *[other] { $count } Ansichten<br>} |

## es


### es/chart.ftl

| Key | English | es |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns` requiere `data` y enumera de 1 a 64 identificadores de columna válidos y distintos |
| `chart-ids-duplicate` | a row or column id is repeated | un id de fila o de columna está repetido |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | un id de fila o de columna tiene de 1 a 64 letras, cifras, - o _ |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids necesita un id por fila y dataset.columns uno por columna |
| `chart-no-dataset` | the chart has no data table (dataset) | el gráfico no tiene tabla de datos (dataset) |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | un parche de opciones no puede cambiar `dataset`: usa edit_data |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | con un dataset, los datos deben quedarse en el dataset: edítalos con edit_data |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | una opción no puede tener una referencia de datos y datos en línea: conserva solo una |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` no puede ser null: elimina la clave en su lugar |

### es/core.ftl

| Key | English | es |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | El elemento { $element } guarda sus datos dentro de sus opciones: este archivo de deck no tiene el formato actual |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | datos «{ $data }»: falta su archivo base, conservados en solo lectura |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | datos «{ $data }»: la columna «{ $column }» contiene valores que no son de tipo { $type } |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | datos «{ $data }»: se perdieron en la fusión cambios hechos a una tabla reemplazada |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | Elemento { $element }: una versión anterior guardó los datos de su gráfico o tabla dentro de sus opciones; se han eliminado |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element } nombra los datos «{ $data }», que no están en la presentación |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | el nombre de datos «{ $data }» ya está en uso |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | demasiados datos en la presentación ({ $count }); no leídos: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | datos «{ $data }» demasiado grandes, truncados al leer y conservados en solo lectura |
| `issue-more` | more issues not shown: { $count } | otros problemas no mostrados: { $count } |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | { $element } no se dibuja: las vistas de la presentación leen más datos de los que permite un renderizado |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element } lee la columna «{ $column }», que sus datos no tienen |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element } solo muestra parte de sus datos ({ $what -><br>    [rows] filas<br>   *[columns] columnas<br>}: { $shown } de { $total }) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | { $element } no se dibuja: sus opciones no se pueden leer |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | El lote es demasiado grande: número de celdas { $count }, como máximo { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | La columna «{ $column }» la muestran { $views }; vincúlalos primero a otras columnas (bind_data con columns) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | Los datos «{ $data }» no tienen la columna «{ $column }» |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | Los datos de la presentación superarían su límite: como máximo { $max } { $what -><br>    [row-ids] identificadores de fila<br>    [overlay] celdas modificadas<br>    [text] bytes de texto<br>   *[data] datos<br>} |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | Estos datos los muestran { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}y { $more } vista más<br>   *[other] {" "}y { $more } vistas más<br>}; elimínalas o vincúlalas primero a otros datos |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | El elemento { $subject } lleva sus datos dentro de sus opciones; añádelos como datos del deck con add_data y vincúlalos con bind_data |
| `op-data-internal` | A batch cannot set a table's base or overlay | Un lote no puede fijar la base ni la capa de cambios de una tabla |
| `op-data-missing` | Data “{ $data }” not found | Datos «{ $data }» no encontrados |
| `op-data-name-taken` | The data name “{ $name }” is already taken | El nombre de datos «{ $name }» ya está en uso |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | El parámetro «{ $param }» no puede mostrar datos: solo una opción de gráfico o de tabla puede |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | El elemento { $subject } ya muestra otros datos; usa bind_data para cambiarlos |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | Los datos «{ $data }» se truncaron al leerse y son de solo lectura; reemplaza toda su tabla para editarlos |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | El historial de la presentación casi alcanza { $max } MiB, el máximo para volver a abrirla: quita contenido antes de añadir más. |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | Los datos «{ $data }» del paquete remiten a un archivo externo; los datos de un paquete contienen sus propias celdas. |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | Los datos del paquete contienen { $count -><br>    [one] { $count } celda<br>   *[other] { $count } celdas<br>}, como máximo { $max }. |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | El paquete contiene { $count -><br>    [one] { $count } dato<br>   *[other] { $count } datos<br>}, como máximo { $max }. |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | Un archivo de datos de esta presentación está dañado o no se pudo guardar |

### es/data.ftl

| Key | English | es |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | el archivo base de los datos está dañado o no coincide con su tabla |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | una tabla con archivo base solo da los id de sus filas, no celdas |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | un valor no encaja en la columna «{ $column }», que contiene { $type -><br>    [number] números<br>    [integer] números enteros<br>    [bool] verdadero o falso<br>    [date] fechas (AAAA-MM-DD)<br>   *[text] texto de 16 KB como máximo<br>} |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | la columna «{ $column }» no puede convertirse sin pérdida en { $type -><br>    [number] una columna de números<br>    [integer] una columna de enteros<br>    [bool] una columna de verdadero o falso<br>    [date] una columna de fechas<br>   *[text] una columna de texto<br>}, ver las filas { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}y { $more } fila más<br>   *[other] {" "}y { $more } filas más<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | la lista de columnas está vacía o nombra una columna dos veces |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | el identificador «{ $id }» no es válido: usa de 1 a 64 letras, dígitos, - o _ |
| `data-id-missing` | a new row, column or table has no id | una nueva fila, columna o tabla no tiene identificador |
| `data-id-taken` | id { $id } is already used in this table | el identificador { $id } ya se usa en esta tabla |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | «{ $name }» no es un nombre de datos válido: usa minúsculas, dígitos y _, sin empezar por un dígito |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | estos datos guardan sus filas en un archivo base que esta edición aún no sabe leer |
| `data-no-column` | column { $column } does not exist | la columna { $column } no existe |
| `data-no-row` | row { $row } does not exist | la fila { $row } no existe |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | { $count -><br>    [one] { $count } número no es finito (NaN o infinito)<br>   *[other] { $count } números no son finitos (NaN o infinito)<br>} |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | { $count -><br>    [one] { $count } fila tenía más celdas que el encabezado; se quitaron las celdas sobrantes<br>   *[other] { $count } filas tenían más celdas que el encabezado; se quitaron las celdas sobrantes<br>} |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | una fila tiene más celdas que las { $max } { $max -><br>    [one] columna<br>   *[other] columnas<br>} de la tabla |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] los datos superarían su límite de { $max } { $max -><br>        [one] fila<br>       *[other] filas<br>    }<br>    [columns] los datos superarían su límite de { $max } { $max -><br>        [one] columna<br>       *[other] columnas<br>    }<br>    [text] una celda de texto superaría su límite de { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] los datos superarían su límite de { $max } { $max -><br>        [one] celda editada<br>       *[other] celdas editadas<br>    }<br>} |

### es/document.ftl

| Key | English | es |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | demasiados valores para dibujar ({ $count }, como máximo { $max }) |

### es/mcp.ftl

| Key | English | es |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | Ningún dato tiene el id «{ $data }»: data_list los enumera. |

### es/session.ftl

| Key | English | es |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | La carpeta de datos de la app no puede guardar los archivos de datos de los decks |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | No se puede cambiar el tipo de { $subject }: otras vistas muestran sus datos, a los que les faltan las columnas que este tipo necesita. |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | Los datos «{ $data }» cambiaron mientras se leían sus filas guardadas; inténtalo de nuevo |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | El almacén de datos está lleno: no se pudo compactar { $data }. Libera espacio bajo [sources] max_store_mib. |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | La fusión reemplazó cambios en { $data }: se borró el historial de deshacer |
| `session-data-user` | { $element } on slide { $slide } | { $element } en la diapositiva { $slide } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | No se pueden añadir ni quitar columnas aquí: la vista { $view } no puede seguirlas |
| `session-example-base-chart-category` | Category | Categoría |
| `session-example-base-chart-value` | Value | Valor |
| `session-request-too-large` | The request is larger than { $mib } MiB. | La solicitud supera los { $mib } MiB. |
| `session-undo-data-add` | Add data | Añadir datos |
| `session-undo-data-bind` | Bind data | Vincular datos |
| `session-undo-data-edit` | Edit data | Editar datos |
| `session-undo-data-remove` | Remove data | Eliminar datos |
| `session-undo-data-rename` | Rename data | Renombrar datos |
| `session-undo-data-type` | Change the column type | Cambiar el tipo de columna |

### es/ui.ftl

| Key | English | es |
|---|---|---|
| `ui-data-add` | Add data | Añadir datos |
| `ui-data-column-type` | Column type | Tipo de columna |
| `ui-data-next` | Next rows | Filas siguientes |
| `ui-data-open` | Open the data | Abrir los datos |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | Filas { $first }–{ $last } de { $total } |
| `ui-data-previous` | Previous rows | Filas anteriores |
| `ui-data-remove` | Remove | Eliminar |
| `ui-data-rename` | Rename | Cambiar nombre |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | Compartidos por { $count -><br>    [one] { $count } vista<br>   *[other] { $count } vistas<br>} |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | Columnas: { $columns } · Filas: { $rows } |
| `ui-data-tab` | Data | Datos |
| `ui-data-type-bool` | Yes/no | Sí/no |
| `ui-data-type-date` | Date (YYYY-MM-DD) | Fecha |
| `ui-data-type-integer` | Integer | Entero |
| `ui-data-type-number` | Number | Número |
| `ui-data-type-text` | Text | Texto |
| `ui-data-unused` | Unused | Sin usar |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count -><br>    [one] { $count } vista<br>   *[other] { $count } vistas<br>} |

## hi


### hi/chart.ftl

| Key | English | hi |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns` को `data` चाहिए और यह 1 से 64 मान्य, अलग-अलग कॉलम पहचानकर्ता सूचीबद्ध करता है |
| `chart-ids-duplicate` | a row or column id is repeated | किसी पंक्ति या स्तंभ की id दोहराई गई है |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | पंक्ति या स्तंभ की id में 1 से 64 अक्षर, अंक, - या _ होते हैं |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids में हर पंक्ति के लिए एक id और dataset.columns में हर स्तंभ के लिए एक id चाहिए |
| `chart-no-dataset` | the chart has no data table (dataset) | चार्ट में डेटा तालिका (dataset) नहीं है |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | विकल्प पैच `dataset` नहीं बदल सकता: edit_data का उपयोग करें |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | dataset होने पर डेटा dataset में ही रहना चाहिए: इसे edit_data से संपादित करें |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | एक विकल्प में डेटा संदर्भ और इनलाइन डेटा दोनों नहीं हो सकते: केवल एक रखें |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` null नहीं हो सकता: इसके बजाय कुंजी हटाएँ |

### hi/core.ftl

| Key | English | hi |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | तत्व { $element } अपना डेटा अपने विकल्पों में रखता है: यह डेक फ़ाइल वर्तमान प्रारूप में नहीं है |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | डेटा “{ $data }”: इसकी आधार फ़ाइल नहीं मिली, केवल-पठन रखा गया |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | डेटा “{ $data }”: कॉलम “{ $column }” में ऐसे मान हैं जो { $type } प्रकार के नहीं हैं |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | डेटा “{ $data }”: बदली गई तालिका के संपादन मर्ज में खो गए |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | तत्व { $element }: एक पुराने संस्करण ने उसके चार्ट या तालिका का डेटा उसके विकल्पों में रखा था; उसे हटा दिया गया |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element } डेटा “{ $data }” का नाम लेता है, जो डेक में नहीं है |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | डेटा नाम “{ $data }” पहले से उपयोग में है |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | डेक में बहुत अधिक डेटा ({ $count }); नहीं पढ़े गए: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | डेटा “{ $data }” बहुत बड़ा है, पढ़ते समय काटा गया और केवल-पठन रखा गया |
| `issue-more` | more issues not shown: { $count } | और समस्याएँ नहीं दिखाई गईं: { $count } |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | { $element } नहीं बनाया गया: डेक के व्यू एक रेंडर की अनुमति से अधिक डेटा पढ़ते हैं |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element } कॉलम “{ $column }” पढ़ता है, जो उसके डेटा में नहीं है |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element } अपने डेटा का केवल एक भाग दिखाता है ({ $what -><br>    [rows] पंक्तियाँ<br>   *[columns] कॉलम<br>}: { $total } में से { $shown }) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | { $element } नहीं बनाया गया: उसके विकल्प पढ़े नहीं जा सकते |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | बैच बहुत बड़ा है: सेल की संख्या { $count }, अधिकतम { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | कॉलम “{ $column }” को { $views } दिखाते हैं; पहले उन्हें दूसरे कॉलमों से जोड़ें (columns के साथ bind_data) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | डेटा “{ $data }” में कॉलम “{ $column }” नहीं है |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | डेक का डेटा अपनी सीमा पार कर जाएगा: अधिकतम { $max } { $what -><br>    [row-ids] पंक्ति आईडी<br>    [overlay] संपादित सेल<br>    [text] बाइट टेक्स्ट<br>   *[data] डेटा<br>} |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | यह डेटा { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}और { $more } अन्य दृश्य<br>   *[other] {" "}और { $more } अन्य दृश्य<br>}{" "}दिखाते हैं; पहले उन्हें हटाएँ या किसी दूसरे डेटा से जोड़ें |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | तत्व { $subject } अपना डेटा अपने विकल्पों में रखता है; उसे add_data से डेक डेटा के रूप में जोड़ें और bind_data से जोड़ें |
| `op-data-internal` | A batch cannot set a table's base or overlay | एक बैच किसी तालिका का आधार या ओवरले सेट नहीं कर सकता |
| `op-data-missing` | Data “{ $data }” not found | डेटा “{ $data }” नहीं मिला |
| `op-data-name-taken` | The data name “{ $name }” is already taken | डेटा नाम “{ $name }” पहले से उपयोग में है |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | पैरामीटर “{ $param }” डेटा नहीं दिखा सकता: यह केवल चार्ट या तालिका विकल्प कर सकता है |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | एलिमेंट { $subject } पहले से अन्य डेटा दिखाता है; बदलने के लिए bind_data का उपयोग करें |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | डेटा “{ $data }” पढ़ते समय छोटा किया गया और केवल पढ़ने योग्य है; संपादित करने के लिए उसकी पूरी तालिका बदलें |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | प्रस्तुति का इतिहास लगभग { $max } MiB तक पहुँच गया है, जो इसे फिर से खोलने की अधिकतम सीमा है: और जोड़ने से पहले सामग्री हटाएँ। |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | पैक का डेटा “{ $data }” किसी बाहरी फ़ाइल को संदर्भित करता है; पैक के डेटा में उनके अपने सेल होते हैं। |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | पैक के डेटा में { $count } सेल हैं, अधिकतम { $max }। |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | पैक में { $count } डेटा हैं, अधिकतम { $max }। |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | इस डेक की एक डेटा फ़ाइल क्षतिग्रस्त है या सहेजी नहीं जा सकी |

### hi/data.ftl

| Key | English | hi |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | डेटा की आधार फ़ाइल क्षतिग्रस्त है या उसकी तालिका से मेल नहीं खाती |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | बेस फ़ाइल वाली तालिका अपनी पंक्तियों की केवल आईडी देती है, सेल नहीं |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | एक मान कॉलम “{ $column }” में फ़िट नहीं होता, जिसमें { $type -><br>    [number] संख्याएँ<br>    [integer] पूर्णांक<br>    [bool] सही या गलत<br>    [date] तारीखें (YYYY-MM-DD)<br>   *[text] अधिकतम 16 KB टेक्स्ट<br>} होते हैं |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | कॉलम “{ $column }” को बिना नुकसान { $type -><br>    [number] संख्या कॉलम<br>    [integer] पूर्णांक कॉलम<br>    [bool] सही/गलत कॉलम<br>    [date] तारीख कॉलम<br>   *[text] टेक्स्ट कॉलम<br>} नहीं बनाया जा सकता, पंक्तियाँ देखें: { $rows }{ $more -><br>    [0] {""}<br>   *[other] {" "}और { $more } अन्य<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | कॉलम सूची खाली है या किसी कॉलम का नाम दो बार लेती है |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | आईडी “{ $id }” मान्य नहीं है: 1 से 64 अक्षर, अंक, - या _ उपयोग करें |
| `data-id-missing` | a new row, column or table has no id | किसी नई पंक्ति, कॉलम या तालिका की आईडी नहीं है |
| `data-id-taken` | id { $id } is already used in this table | आईडी { $id } इस तालिका में पहले से उपयोग में है |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | “{ $name }” मान्य डेटा नाम नहीं है: छोटे अक्षर, अंक और _ उपयोग करें, अंक से शुरू न करें |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | यह डेटा अपनी पंक्तियाँ एक बेस फ़ाइल में रखता है, जिसे यह संपादन अभी नहीं पढ़ सकता |
| `data-no-column` | column { $column } does not exist | कॉलम { $column } मौजूद नहीं है |
| `data-no-row` | row { $row } does not exist | पंक्ति { $row } मौजूद नहीं है |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | परिमित न होने वाली संख्याएँ (NaN या अनंत): { $count } |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | शीर्षक से अधिक सेल वाली पंक्तियाँ, अतिरिक्त सेल हटा दिए गए: { $count } |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | एक पंक्ति में तालिका के कॉलम से अधिक सेल हैं (कॉलम: { $max }) |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] डेटा पंक्तियों की सीमा पार कर जाएगा (सीमा: { $max })<br>    [columns] डेटा कॉलम की सीमा पार कर जाएगा (सीमा: { $max })<br>    [text] एक टेक्स्ट सेल बाइट की सीमा पार कर जाएगा (सीमा: { $max })<br>   *[overlay] डेटा संपादित सेल की सीमा पार कर जाएगा (सीमा: { $max })<br>} |

### hi/document.ftl

| Key | English | hi |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | बनाने के लिए बहुत अधिक मान ({ $count }, अधिकतम { $max }) |

### hi/mcp.ftl

| Key | English | hi |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | “{ $data }” आईडी वाला कोई डेटा नहीं है; data_list उनकी सूची देता है। |

### hi/session.ftl

| Key | English | hi |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | ऐप का डेटा फ़ोल्डर डेक की डेटा फ़ाइलें नहीं रख सकता |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | { $subject } का प्रकार नहीं बदला जा सकता: इसका डेटा दूसरे दृश्यों में दिखता है, और उसमें इस प्रकार के लिए ज़रूरी कॉलम नहीं हैं। |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | सहेजी गई पंक्तियाँ पढ़ते समय डेटा “{ $data }” बदल गया; फिर से प्रयास करें |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | डेटा भंडार भर गया है: { $data } को संकुचित नहीं किया जा सका। [sources] max_store_mib के अंतर्गत जगह खाली करें। |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | मर्ज ने { $data } के संपादनों को बदल दिया: पूर्ववत इतिहास साफ़ कर दिया गया |
| `session-data-user` | { $element } on slide { $slide } | स्लाइड { $slide } पर { $element } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | यहाँ कॉलम जोड़े या हटाए नहीं जा सकते: व्यू { $view } उनका अनुसरण नहीं कर सकता |
| `session-example-base-chart-category` | Category | श्रेणी |
| `session-example-base-chart-value` | Value | मान |
| `session-request-too-large` | The request is larger than { $mib } MiB. | अनुरोध { $mib } MiB से बड़ा है। |
| `session-undo-data-add` | Add data | डेटा जोड़ें |
| `session-undo-data-bind` | Bind data | डेटा बाँधें |
| `session-undo-data-edit` | Edit data | डेटा संपादित करें |
| `session-undo-data-remove` | Remove data | डेटा हटाएँ |
| `session-undo-data-rename` | Rename data | डेटा का नाम बदलें |
| `session-undo-data-type` | Change the column type | कॉलम का प्रकार बदलें |

### hi/ui.ftl

| Key | English | hi |
|---|---|---|
| `ui-data-add` | Add data | डेटा जोड़ें |
| `ui-data-column-type` | Column type | कॉलम का प्रकार |
| `ui-data-next` | Next rows | अगली पंक्तियाँ |
| `ui-data-open` | Open the data | डेटा खोलें |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | { $total } में से पंक्तियाँ { $first }–{ $last } |
| `ui-data-previous` | Previous rows | पिछली पंक्तियाँ |
| `ui-data-remove` | Remove | हटाएँ |
| `ui-data-rename` | Rename | नाम बदलें |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count -><br>    [one] { $count } दृश्य द्वारा साझा<br>   *[other] { $count } दृश्यों द्वारा साझा<br>} |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | कॉलम: { $columns } · पंक्तियाँ: { $rows } |
| `ui-data-tab` | Data | डेटा |
| `ui-data-type-bool` | Yes/no | हाँ/नहीं |
| `ui-data-type-date` | Date (YYYY-MM-DD) | तारीख |
| `ui-data-type-integer` | Integer | पूर्णांक |
| `ui-data-type-number` | Number | संख्या |
| `ui-data-type-text` | Text | टेक्स्ट |
| `ui-data-unused` | Unused | अप्रयुक्त |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count } व्यू |

## it


### it/chart.ftl

| Key | English | it |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns` richiede `data` ed elenca da 1 a 64 identificatori di colonna validi e distinti |
| `chart-ids-duplicate` | a row or column id is repeated | un id di riga o di colonna è ripetuto |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | un id di riga o di colonna ha da 1 a 64 lettere, cifre, - o _ |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids richiede un id per riga e dataset.columns uno per colonna |
| `chart-no-dataset` | the chart has no data table (dataset) | il grafico non ha una tabella di dati (dataset) |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | una patch di opzioni non può modificare `dataset`: usa edit_data |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | con un dataset, i dati devono restare nel dataset: modificali con edit_data |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | un'opzione non può avere un riferimento ai dati e dati in linea: tienine uno solo |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` non può essere null: rimuovi invece la chiave |

### it/core.ftl

| Key | English | it |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | L’elemento { $element } salva i suoi dati nelle sue opzioni: questo file di deck non è nel formato attuale |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | dati «{ $data }»: manca il loro file di base, tenuti in sola lettura |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | dati «{ $data }»: la colonna «{ $column }» contiene valori che non sono di tipo { $type } |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | dati «{ $data }»: modifiche a una tabella sostituita sono andate perse nell'unione |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | Elemento { $element }: una versione precedente ha salvato i dati del suo grafico o tabella nelle sue opzioni; sono stati rimossi |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element } indica i dati «{ $data }», che non sono nella presentazione |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | il nome di dati «{ $data }» è già in uso |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | troppi dati nella presentazione ({ $count }); non letti: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | dati «{ $data }» troppo grandi, troncati in lettura e tenuti in sola lettura |
| `issue-more` | more issues not shown: { $count } | altri problemi non mostrati: { $count } |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | { $element } non viene disegnato: le viste della presentazione leggono più dati di quanti un rendering ne consenta |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element } legge la colonna «{ $column }», che i suoi dati non hanno |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element } mostra solo una parte dei suoi dati ({ $what -><br>    [rows] righe<br>   *[columns] colonne<br>}: { $shown } su { $total }) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | { $element } non viene disegnato: le sue opzioni non sono leggibili |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | Il lotto è troppo grande: numero di celle { $count }, al massimo { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | La colonna «{ $column }» è mostrata da { $views }; collegali prima ad altre colonne (bind_data con columns) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | I dati «{ $data }» non hanno la colonna «{ $column }» |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | I dati della presentazione supererebbero il loro limite: al massimo { $max } { $what -><br>    [row-ids] ID di riga<br>    [overlay] celle modificate<br>    [text] byte di testo<br>   *[data] dati<br>} |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | Questi dati sono mostrati da { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}e da { $more } altra vista<br>   *[other] {" "}e da altre { $more } viste<br>}; rimuovile o collegale prima ad altri dati |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | L’elemento { $subject } porta i suoi dati nelle sue opzioni; aggiungili come dati del deck con add_data e collegali con bind_data |
| `op-data-internal` | A batch cannot set a table's base or overlay | Un lotto non può impostare la base o la sovrapposizione di una tabella |
| `op-data-missing` | Data “{ $data }” not found | Dati «{ $data }» non trovati |
| `op-data-name-taken` | The data name “{ $name }” is already taken | Il nome di dati «{ $name }» è già in uso |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | Il parametro «{ $param }» non può mostrare dati: solo un'opzione di grafico o tabella può farlo |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | L'elemento { $subject } mostra già altri dati; usa bind_data per cambiarli |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | I dati «{ $data }» sono stati troncati in lettura e sono di sola lettura; sostituisci l'intera tabella per modificarli |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | La cronologia della presentazione sta per raggiungere { $max } MiB, il massimo per riaprirla: rimuovi contenuti prima di aggiungerne altri. |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | I dati «{ $data }» del pacchetto rimandano a un file esterno; i dati di un pacchetto contengono le proprie celle. |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | I dati del pacchetto contengono { $count -><br>    [one] { $count } cella<br>   *[other] { $count } celle<br>}, al massimo { $max }. |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | Il pacchetto contiene { $count -><br>    [one] { $count } dato<br>   *[other] { $count } dati<br>}, al massimo { $max }. |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | Un file di dati di questa presentazione è danneggiato o non è stato possibile salvarlo |

### it/data.ftl

| Key | English | it |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | il file di base dei dati è danneggiato o non corrisponde alla sua tabella |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | una tabella con un file di base indica solo gli id delle righe, non le celle |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | un valore non è adatto alla colonna «{ $column }», che contiene { $type -><br>    [number] numeri<br>    [integer] numeri interi<br>    [bool] vero o falso<br>    [date] date (AAAA-MM-GG)<br>   *[text] testo di al massimo 16 KB<br>} |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | la colonna «{ $column }» non può diventare senza perdite { $type -><br>    [number] una colonna di numeri<br>    [integer] una colonna di interi<br>    [bool] una colonna vero o falso<br>    [date] una colonna di date<br>   *[text] una colonna di testo<br>}, vedi le righe { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}e { $more } altra riga<br>   *[other] {" "}e altre { $more } righe<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | l’elenco delle colonne è vuoto o nomina due volte una colonna |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | l’identificatore «{ $id }» non è valido: usa da 1 a 64 lettere, cifre, - o _ |
| `data-id-missing` | a new row, column or table has no id | una nuova riga, colonna o tabella non ha un identificatore |
| `data-id-taken` | id { $id } is already used in this table | l’identificatore { $id } è già usato in questa tabella |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | «{ $name }» non è un nome di dati valido: usa lettere minuscole, cifre e _, senza iniziare con una cifra |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | questi dati tengono le righe in un file di base che questa modifica non sa ancora leggere |
| `data-no-column` | column { $column } does not exist | la colonna { $column } non esiste |
| `data-no-row` | row { $row } does not exist | la riga { $row } non esiste |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | { $count -><br>    [one] { $count } numero non è finito (NaN o infinito)<br>   *[other] { $count } numeri non sono finiti (NaN o infinito)<br>} |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | { $count -><br>    [one] { $count } riga aveva più celle dell’intestazione; le celle in eccesso sono state rimosse<br>   *[other] { $count } righe avevano più celle dell’intestazione; le celle in eccesso sono state rimosse<br>} |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | una riga ha più celle delle { $max } { $max -><br>    [one] colonna<br>   *[other] colonne<br>} della tabella |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] i dati supererebbero il limite di { $max } { $max -><br>        [one] riga<br>       *[other] righe<br>    }<br>    [columns] i dati supererebbero il limite di { $max } { $max -><br>        [one] colonna<br>       *[other] colonne<br>    }<br>    [text] una cella di testo supererebbe il limite di { $max } { $max -><br>        [one] byte<br>       *[other] byte<br>    }<br>   *[overlay] i dati supererebbero il limite di { $max } { $max -><br>        [one] cella modificata<br>       *[other] celle modificate<br>    }<br>} |

### it/document.ftl

| Key | English | it |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | troppi valori da disegnare ({ $count }, al massimo { $max }) |

### it/mcp.ftl

| Key | English | it |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | Nessun dato ha l’id «{ $data }»: data_list li elenca. |

### it/session.ftl

| Key | English | it |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | La cartella dati dell’app non può contenere i file di dati dei deck |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | Impossibile cambiare il tipo di { $subject }: altre viste mostrano i suoi dati, privi delle colonne richieste da questo tipo. |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | I dati «{ $data }» sono cambiati durante la lettura delle righe salvate; riprova |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | L'archivio dati è pieno: impossibile compattare { $data }. Libera spazio sotto [sources] max_store_mib. |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | L’unione ha sostituito modifiche a { $data }: la cronologia di annullamento è stata cancellata |
| `session-data-user` | { $element } on slide { $slide } | { $element } nella diapositiva { $slide } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | Qui non è possibile aggiungere o rimuovere colonne: la vista { $view } non può seguirle |
| `session-example-base-chart-category` | Category | Categoria |
| `session-example-base-chart-value` | Value | Valore |
| `session-request-too-large` | The request is larger than { $mib } MiB. | La richiesta supera { $mib } MiB. |
| `session-undo-data-add` | Add data | Aggiungi dati |
| `session-undo-data-bind` | Bind data | Collega dati |
| `session-undo-data-edit` | Edit data | Modifica dati |
| `session-undo-data-remove` | Remove data | Rimuovi dati |
| `session-undo-data-rename` | Rename data | Rinomina dati |
| `session-undo-data-type` | Change the column type | Cambia il tipo di colonna |

### it/ui.ftl

| Key | English | it |
|---|---|---|
| `ui-data-add` | Add data | Aggiungi dati |
| `ui-data-column-type` | Column type | Tipo di colonna |
| `ui-data-next` | Next rows | Righe successive |
| `ui-data-open` | Open the data | Apri i dati |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | Righe { $first }–{ $last } di { $total } |
| `ui-data-previous` | Previous rows | Righe precedenti |
| `ui-data-remove` | Remove | Rimuovi |
| `ui-data-rename` | Rename | Rinomina |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | Condivisi da { $count -><br>    [one] { $count } vista<br>   *[other] { $count } viste<br>} |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | Colonne: { $columns } · Righe: { $rows } |
| `ui-data-tab` | Data | Dati |
| `ui-data-type-bool` | Yes/no | Sì/no |
| `ui-data-type-date` | Date (YYYY-MM-DD) | Data |
| `ui-data-type-integer` | Integer | Intero |
| `ui-data-type-number` | Number | Numero |
| `ui-data-type-text` | Text | Testo |
| `ui-data-unused` | Unused | Non usati |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count -><br>    [one] { $count } vista<br>   *[other] { $count } viste<br>} |

## ja


### ja/chart.ftl

| Key | English | ja |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns` には `data` が必要で、有効で重複しない列 ID を 1〜64個並べます |
| `chart-ids-duplicate` | a row or column id is repeated | 行または列の id が重複しています |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | 行または列の id は 1〜64 文字の英数字、- または _ です |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids には行ごとに 1つ、dataset.columns には列ごとに 1つの id が必要です |
| `chart-no-dataset` | the chart has no data table (dataset) | グラフにデータテーブル (dataset) がありません |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | オプションのパッチでは `dataset` を変更できません：edit_data を使用してください |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | dataset がある場合、データは dataset 内に置く必要があります：edit_data で編集してください |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | オプションにデータ参照とインラインデータを両方持たせることはできません。どちらか一方にしてください |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` を null にはできません。代わりにキーを削除してください |

### ja/core.ftl

| Key | English | ja |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | 要素 { $element } はデータをオプション内に保存しています：このデッキファイルは現在の形式ではありません |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | データ「{ $data }」: ベースファイルがないため、読み取り専用にしました |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | データ「{ $data }」: 列「{ $column }」に { $type } 型でない値があります |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | データ「{ $data }」: 置き換えられた表への編集がマージで失われました |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | 要素 { $element }：古いバージョンがグラフや表のデータをオプション内に保存していました。削除しました |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element }はデータ「{ $data }」を指定していますが、デッキにありません |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | データ名「{ $data }」はすでに使われています |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | デッキのデータが多すぎます（{ $count }）。読み込まなかったもの: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | データ「{ $data }」が大きすぎるため、読み込み時に切り詰めて読み取り専用にしました |
| `issue-more` | more issues not shown: { $count } | 表示していない問題があと{ $count }件あります |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | { $element }は描画されません：デッキのビューが1回の描画で許される量を超えるデータを読み込みます |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element }は列「{ $column }」を読み込みますが、そのデータにはありません |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element }はデータの一部だけを表示しています（{ $what -><br>    [rows] 行<br>   *[columns] 列<br>}：{ $total }中{ $shown }） |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | { $element }は描画されません：オプションを読み取れません |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | バッチが大きすぎます: セル数 { $count }、上限 { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | 列「{ $column }」は { $views } が表示しています。先にそれらを別の列に紐付けてください（columns を指定した bind_data） |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | データ「{ $data }」に列「{ $column }」はありません |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | デッキのデータが上限を超えます: { $what -><br>    [row-ids] 行 ID<br>    [overlay] 編集済みセル<br>    [text] テキストのバイト数<br>   *[data] データ<br>} は最大 { $max } です |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | このデータは { $views }{ $more -><br>    [0] {""}<br>   *[other] {" "}ほか { $more }件のビュー<br>}{" "}が表示しています。先にそれらを削除するか、別のデータに紐付けてください |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | 要素 { $subject } はデータをオプション内に持っています。add_data でデッキのデータとして追加し、bind_data で結び付けてください |
| `op-data-internal` | A batch cannot set a table's base or overlay | バッチでは表のベースやオーバーレイを設定できません |
| `op-data-missing` | Data “{ $data }” not found | データ「{ $data }」が見つかりません |
| `op-data-name-taken` | The data name “{ $name }” is already taken | データ名「{ $name }」はすでに使われています |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | パラメーター「{ $param }」はデータを表示できません。表示できるのはグラフまたは表のオプションだけです |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | 要素 { $subject } はすでに別のデータを表示しています。変更するには bind_data を使ってください |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | データ「{ $data }」は読み込み時に切り詰められ、読み取り専用です。編集するには表全体を置き換えてください |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | プレゼンテーションの履歴が再び開ける上限の { $max } MiB に近づいています：追加する前に内容を削除してください。 |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | パックのデータ「{ $data }」は外部ファイルを参照しています。パックのデータは自身のセルを持つ必要があります。 |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | パックのデータのセルは { $count }個です（最大 { $max }個）。 |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | パックのデータは { $count }件です（最大 { $max }件）。 |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | このデッキのデータファイルが破損しているか、保存できませんでした |

### ja/data.ftl

| Key | English | ja |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | データのベースファイルが破損しているか、表と一致しません |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | ベースファイルを持つ表は、行の ID だけを指定し、セルは指定しません |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | 値が列「{ $column }」に合いません（この列の内容：{ $type -><br>    [number] 数値<br>    [integer] 整数<br>    [bool] 真または偽<br>    [date] 日付、YYYY-MM-DD<br>   *[text] 最大16KBのテキスト<br>}） |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | 列「{ $column }」は損失なしに{ $type -><br>    [number] 数値の列<br>    [integer] 整数の列<br>    [bool] 真偽の列<br>    [date] 日付の列<br>   *[text] テキストの列<br>}にできません。該当する行：{ $rows }{ $more -><br>    [0] {""}<br>   *[other] {""}、ほか{ $more }行<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | 列のリストが空か、同じ列を2回指定しています |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | ID「{ $id }」は無効です：英字、数字、- または _ を1～64文字で使用してください |
| `data-id-missing` | a new row, column or table has no id | 新しい行、列、または表に ID がありません |
| `data-id-taken` | id { $id } is already used in this table | ID { $id } はこの表ですでに使われています |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | 「{ $name }」は有効なデータ名ではありません：小文字の英字、数字、_ を使い、数字で始めないでください |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | このデータは行をベースファイルに保存しており、この編集ではまだ読み取れません |
| `data-no-column` | column { $column } does not exist | 列 { $column } は存在しません |
| `data-no-row` | row { $row } does not exist | 行 { $row } は存在しません |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | { $count }個の数値が有限ではありません（NaN または無限大） |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | { $count }行のセル数が見出しより多かったため、余分なセルを削除しました |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | 行のセル数が表の{ $max }列を超えています |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] データが行数の上限（{ $max }行）を超えます<br>    [columns] データが列数の上限（{ $max }列）を超えます<br>    [text] テキストのセルが上限（{ $max }バイト）を超えます<br>   *[overlay] データが編集済みセルの上限（{ $max }件）を超えます<br>} |

### ja/document.ftl

| Key | English | ja |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | 描画する値が多すぎます（{ $count }、最大{ $max }） |

### ja/mcp.ftl

| Key | English | ja |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | ID「{ $data }」のデータはありません。data_list で一覧できます。 |

### ja/session.ftl

| Key | English | ja |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | アプリのデータフォルダーにデッキのデータファイルを保存できません |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | { $subject } の種類を変更できません：そのデータは他のビューにも表示されており、この種類に必要な列がありません。 |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | 保存された行の読み込み中にデータ「{ $data }」が変更されました。もう一度お試しください |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | データストアがいっぱいです：{ $data } を圧縮できませんでした。[sources] max_store_mib の下で空き容量を確保してください。 |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | 結合により { $data } への編集が置き換えられました: 取り消し履歴は消去されました |
| `session-data-user` | { $element } on slide { $slide } | スライド { $slide } の { $element } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | ここでは列を追加・削除できません：ビュー { $view } が追従できません |
| `session-example-base-chart-category` | Category | カテゴリ |
| `session-example-base-chart-value` | Value | 値 |
| `session-request-too-large` | The request is larger than { $mib } MiB. | リクエストが { $mib } MiB を超えています。 |
| `session-undo-data-add` | Add data | データを追加 |
| `session-undo-data-bind` | Bind data | データを紐付け |
| `session-undo-data-edit` | Edit data | データを編集 |
| `session-undo-data-remove` | Remove data | データを削除 |
| `session-undo-data-rename` | Rename data | データの名前を変更 |
| `session-undo-data-type` | Change the column type | 列の型を変更 |

### ja/ui.ftl

| Key | English | ja |
|---|---|---|
| `ui-data-add` | Add data | データを追加 |
| `ui-data-column-type` | Column type | 列の型 |
| `ui-data-next` | Next rows | 次の行 |
| `ui-data-open` | Open the data | データを開く |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | { $total }行中 { $first }–{ $last }行 |
| `ui-data-previous` | Previous rows | 前の行 |
| `ui-data-remove` | Remove | 削除 |
| `ui-data-rename` | Rename | 名前を変更 |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count }個のビューで共有 |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | 列: { $columns } · 行: { $rows } |
| `ui-data-tab` | Data | データ |
| `ui-data-type-bool` | Yes/no | はい/いいえ |
| `ui-data-type-date` | Date (YYYY-MM-DD) | 日付 |
| `ui-data-type-integer` | Integer | 整数 |
| `ui-data-type-number` | Number | 数値 |
| `ui-data-type-text` | Text | テキスト |
| `ui-data-unused` | Unused | 未使用 |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count }件のビュー |

## ko


### ko/chart.ftl

| Key | English | ko |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns`에는 `data`가 필요하며 유효하고 서로 다른 열 ID를 1~64개 나열합니다 |
| `chart-ids-duplicate` | a row or column id is repeated | 행 또는 열 id가 중복되었습니다 |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | 행 또는 열 id는 1~64자의 문자, 숫자, - 또는 _입니다 |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids에는 행마다 id가 하나, dataset.columns에는 열마다 하나가 필요합니다 |
| `chart-no-dataset` | the chart has no data table (dataset) | 차트에 데이터 표(dataset)가 없습니다 |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | 옵션 패치로는 `dataset`을 변경할 수 없습니다. edit_data를 사용하세요 |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | dataset이 있으면 데이터는 dataset에 있어야 합니다. edit_data로 편집하세요 |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | 옵션에 데이터 참조와 인라인 데이터를 함께 둘 수 없습니다. 하나만 남기세요 |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data`는 null일 수 없습니다. 대신 키를 제거하세요 |

### ko/core.ftl

| Key | English | ko |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | 요소 { $element }이(가) 데이터를 옵션 안에 저장합니다: 이 덱 파일은 현재 형식이 아닙니다 |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | 데이터 “{ $data }”: 기본 파일이 없어 읽기 전용으로 유지합니다 |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | 데이터 “{ $data }”: 열 “{ $column }”에 { $type } 형식이 아닌 값이 있습니다 |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | 데이터 “{ $data }”: 교체된 표에 대한 편집이 병합 중 손실되었습니다 |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | 요소 { $element }: 이전 버전이 차트나 표의 데이터를 옵션 안에 저장했습니다. 삭제되었습니다 |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element }이(가) 덱에 없는 데이터 “{ $data }”를 지정합니다 |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | 데이터 이름 “{ $data }”은(는) 이미 사용 중입니다 |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | 덱에 데이터가 너무 많습니다({ $count }). 읽지 않은 항목: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | 데이터 “{ $data }”가 너무 커서 읽을 때 잘랐으며 읽기 전용으로 유지합니다 |
| `issue-more` | more issues not shown: { $count } | 표시되지 않은 문제가 { $count }개 더 있습니다 |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | { $element }을(를) 그리지 않음: 덱의 보기가 한 번의 렌더링에서 허용되는 것보다 많은 데이터를 읽습니다 |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element }이(가) 데이터에 없는 열 “{ $column }”을(를) 읽습니다 |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element }에 데이터의 일부만 표시됩니다 ({ $what -><br>    [rows] 행<br>   *[columns] 열<br>}: { $total }개 중 { $shown }개) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | { $element }을(를) 그리지 않음: 옵션을 읽을 수 없습니다 |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | 일괄 작업이 너무 큽니다: 셀 수 { $count }, 최대 { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | 열 “{ $column }”을(를) { $views }이(가) 표시합니다. 먼저 다른 열에 연결하세요(columns를 지정한 bind_data) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | 데이터 “{ $data }”에 열 “{ $column }”이(가) 없습니다 |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | 덱 데이터가 한도를 넘게 됩니다: { $what -><br>    [row-ids] 행 ID<br>    [overlay] 편집된 셀<br>    [text] 텍스트 바이트<br>   *[data] 데이터<br>} 최대 { $max } |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | 이 데이터는 { $views }{ $more -><br>    [0] {""}<br>   *[other] {" "}외 { $more }개 보기<br>}이(가) 표시합니다. 먼저 삭제하거나 다른 데이터에 연결하세요 |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | 요소 { $subject }의 데이터가 옵션 안에 있습니다. add_data로 덱 데이터로 추가하고 bind_data로 연결하세요 |
| `op-data-internal` | A batch cannot set a table's base or overlay | 일괄 작업으로 표의 기반이나 오버레이를 설정할 수 없습니다 |
| `op-data-missing` | Data “{ $data }” not found | 데이터 “{ $data }”을(를) 찾을 수 없습니다 |
| `op-data-name-taken` | The data name “{ $name }” is already taken | 데이터 이름 “{ $name }”은(는) 이미 사용 중입니다 |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | 매개변수 “{ $param }”은(는) 데이터를 표시할 수 없습니다. 차트나 표 옵션만 가능합니다 |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | 요소 { $subject }은(는) 이미 다른 데이터를 표시합니다. 바꾸려면 bind_data를 사용하세요 |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | 데이터 “{ $data }”은(는) 읽을 때 잘려 읽기 전용입니다. 편집하려면 표 전체를 바꾸세요 |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | 프레젠테이션 기록이 다시 열 수 있는 최대 크기인 { $max } MiB에 가까워졌습니다: 더 추가하기 전에 콘텐츠를 제거하세요. |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | 팩의 데이터 “{ $data }”가 외부 파일을 참조합니다. 팩의 데이터는 자체 셀을 가져야 합니다. |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | 팩의 데이터에 셀이 { $count }개 있습니다(최대 { $max }개). |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | 팩에 데이터가 { $count }개 있습니다(최대 { $max }개). |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | 이 덱의 데이터 파일이 손상되었거나 저장할 수 없습니다 |

### ko/data.ftl

| Key | English | ko |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | 데이터의 기본 파일이 손상되었거나 표와 일치하지 않습니다 |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | 기본 파일이 있는 표는 행의 ID만 지정하고 셀은 지정하지 않습니다 |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | 값이 “{ $column }” 열에 맞지 않습니다 (이 열의 내용: { $type -><br>    [number] 숫자<br>    [integer] 정수<br>    [bool] 참 또는 거짓<br>    [date] 날짜, YYYY-MM-DD<br>   *[text] 최대 16KB 텍스트<br>}) |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | “{ $column }” 열은 손실 없이 { $type -><br>    [number] 숫자 열로<br>    [integer] 정수 열로<br>    [bool] 참/거짓 열로<br>    [date] 날짜 열로<br>   *[text] 텍스트 열로<br>} 바꿀 수 없습니다. 해당 행: { $rows }{ $more -><br>    [0] {""}<br>   *[other] {""}, 그 외 { $more }개<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | 열 목록이 비어 있거나 같은 열을 두 번 지정합니다 |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | ID “{ $id }”는 유효하지 않습니다: 영문자, 숫자, - 또는 _를 1~64자로 사용하세요 |
| `data-id-missing` | a new row, column or table has no id | 새 행, 열 또는 표에 ID가 없습니다 |
| `data-id-taken` | id { $id } is already used in this table | ID { $id }는 이 표에서 이미 사용 중입니다 |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | “{ $name }”는 유효한 데이터 이름이 아닙니다: 영문 소문자, 숫자, _를 사용하고 숫자로 시작하지 마세요 |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | 이 데이터는 행을 기본 파일에 보관하며, 이 편집은 아직 그 파일을 읽을 수 없습니다 |
| `data-no-column` | column { $column } does not exist | { $column } 열이 없습니다 |
| `data-no-row` | row { $row } does not exist | { $row } 행이 없습니다 |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | 유한하지 않은 숫자가 { $count }개 있습니다 (NaN 또는 무한대) |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | 머리글보다 셀이 많은 행 { $count }개에서 남는 셀을 삭제했습니다 |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | 행의 셀 수가 표의 열 { $max }개보다 많습니다 |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] 데이터가 행 한도({ $max }개)를 넘게 됩니다<br>    [columns] 데이터가 열 한도({ $max }개)를 넘게 됩니다<br>    [text] 텍스트 셀이 한도({ $max }바이트)를 넘게 됩니다<br>   *[overlay] 데이터가 편집된 셀 한도({ $max }개)를 넘게 됩니다<br>} |

### ko/document.ftl

| Key | English | ko |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | 그릴 값이 너무 많습니다 ({ $count }개, 최대 { $max }개) |

### ko/mcp.ftl

| Key | English | ko |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | ID “{ $data }”인 데이터가 없습니다. data_list가 목록을 보여 줍니다. |

### ko/session.ftl

| Key | English | ko |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | 앱의 데이터 폴더에 덱의 데이터 파일을 저장할 수 없습니다 |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | { $subject }의 유형을 바꿀 수 없습니다: 데이터가 다른 보기에도 표시되며, 이 유형에 필요한 열이 없습니다. |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | 저장된 행을 읽는 동안 데이터 “{ $data }”이(가) 변경되었습니다. 다시 시도하세요 |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | 데이터 저장소가 가득 찼습니다: { $data }을(를) 압축할 수 없습니다. [sources] max_store_mib에서 공간을 확보하세요. |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | 병합으로 { $data }의 편집 내용이 대체되었습니다: 실행 취소 기록이 지워졌습니다 |
| `session-data-user` | { $element } on slide { $slide } | 슬라이드 { $slide }의 { $element } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | 여기서는 열을 추가하거나 제거할 수 없습니다: 보기 { $view }이(가) 따라갈 수 없습니다 |
| `session-example-base-chart-category` | Category | 범주 |
| `session-example-base-chart-value` | Value | 값 |
| `session-request-too-large` | The request is larger than { $mib } MiB. | 요청이 { $mib } MiB보다 큽니다. |
| `session-undo-data-add` | Add data | 데이터 추가 |
| `session-undo-data-bind` | Bind data | 데이터 연결 |
| `session-undo-data-edit` | Edit data | 데이터 편집 |
| `session-undo-data-remove` | Remove data | 데이터 삭제 |
| `session-undo-data-rename` | Rename data | 데이터 이름 바꾸기 |
| `session-undo-data-type` | Change the column type | 열 형식 변경 |

### ko/ui.ftl

| Key | English | ko |
|---|---|---|
| `ui-data-add` | Add data | 데이터 추가 |
| `ui-data-column-type` | Column type | 열 유형 |
| `ui-data-next` | Next rows | 다음 행 |
| `ui-data-open` | Open the data | 데이터 열기 |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | { $total }행 중 { $first }–{ $last }행 |
| `ui-data-previous` | Previous rows | 이전 행 |
| `ui-data-remove` | Remove | 삭제 |
| `ui-data-rename` | Rename | 이름 바꾸기 |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | 뷰 { $count }개에서 공유 |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | 열: { $columns } · 행: { $rows } |
| `ui-data-tab` | Data | 데이터 |
| `ui-data-type-bool` | Yes/no | 예/아니요 |
| `ui-data-type-date` | Date (YYYY-MM-DD) | 날짜 |
| `ui-data-type-integer` | Integer | 정수 |
| `ui-data-type-number` | Number | 숫자 |
| `ui-data-type-text` | Text | 텍스트 |
| `ui-data-unused` | Unused | 사용 안 함 |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | 보기 { $count }개 |

## nl


### nl/chart.ftl

| Key | English | nl |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns` vereist `data` en noemt 1 tot 64 geldige, verschillende kolom-id's |
| `chart-ids-duplicate` | a row or column id is repeated | een rij- of kolom-id komt dubbel voor |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | een rij- of kolom-id heeft 1 tot 64 letters, cijfers, - of _ |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids heeft één id per rij nodig en dataset.columns één per kolom |
| `chart-no-dataset` | the chart has no data table (dataset) | de grafiek heeft geen gegevenstabel (dataset) |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | een optiepatch kan `dataset` niet wijzigen: gebruik edit_data |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | met een dataset moeten de gegevens in de dataset blijven: bewerk ze met edit_data |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | een optie kan niet zowel een gegevensverwijzing als ingesloten gegevens bevatten: houd er één |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` mag niet null zijn: verwijder in plaats daarvan de sleutel |

### nl/core.ftl

| Key | English | nl |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | Element { $element } bewaart zijn gegevens in zijn opties: dit deckbestand heeft niet het huidige formaat |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | gegevens ‘{ $data }’: het basisbestand ontbreekt, alleen-lezen bewaard |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | gegevens ‘{ $data }’: kolom ‘{ $column }’ bevat waarden die niet van het type { $type } zijn |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | gegevens ‘{ $data }’: wijzigingen aan een vervangen tabel zijn bij het samenvoegen verloren gegaan |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | Element { $element }: een oudere versie bewaarde de gegevens van zijn grafiek of tabel in zijn opties; ze zijn verwijderd |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element } noemt de gegevens ‘{ $data }’, die niet in het deck staan |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | de gegevensnaam ‘{ $data }’ is al in gebruik |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | te veel gegevens in de presentatie ({ $count }); niet gelezen: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | gegevens ‘{ $data }’ te groot, bij het lezen ingekort en alleen-lezen bewaard |
| `issue-more` | more issues not shown: { $count } | meer problemen niet getoond: { $count } |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | { $element } wordt niet getekend: de weergaven van het deck lezen meer gegevens dan een weergave toestaat |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element } leest de kolom ‘{ $column }’, die niet in zijn gegevens staat |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element } toont maar een deel van zijn gegevens ({ $what -><br>    [rows] rijen<br>   *[columns] kolommen<br>}: { $shown } van { $total }) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | { $element } wordt niet getekend: de opties zijn niet leesbaar |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | De batch is te groot: aantal cellen { $count }, hoogstens { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | Kolom ‘{ $column }’ wordt getoond door { $views }; koppel die eerst aan andere kolommen (bind_data met columns) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | Gegevens ‘{ $data }’ hebben geen kolom ‘{ $column }’ |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | De gegevens van de presentatie zouden hun limiet overschrijden: hoogstens { $max } { $what -><br>    [row-ids] rij-id's<br>    [overlay] gewijzigde cellen<br>    [text] bytes tekst<br>   *[data] gegevens<br>} |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | Deze gegevens worden getoond door { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}en { $more } andere weergave<br>   *[other] {" "}en { $more } andere weergaven<br>}; verwijder ze of koppel ze eerst aan andere gegevens |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | Element { $subject } draagt zijn gegevens in zijn opties; voeg ze toe als deckgegevens met add_data en koppel ze met bind_data |
| `op-data-internal` | A batch cannot set a table's base or overlay | Een batch kan de basis of de overlay van een tabel niet instellen |
| `op-data-missing` | Data “{ $data }” not found | Gegevens ‘{ $data }’ niet gevonden |
| `op-data-name-taken` | The data name “{ $name }” is already taken | De gegevensnaam ‘{ $name }’ is al in gebruik |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | Parameter ‘{ $param }’ kan geen gegevens tonen: alleen een grafiek- of tabeloptie kan dat |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | Element { $subject } toont al andere gegevens; gebruik bind_data om ze te wijzigen |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | Gegevens ‘{ $data }’ zijn bij het lezen ingekort en alleen-lezen; vervang de hele tabel om ze te bewerken |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | De geschiedenis van de presentatie nadert { $max } MiB, het maximum om haar te heropenen: verwijder inhoud voordat je meer toevoegt. |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | De gegevens ‘{ $data }’ van het pakket verwijzen naar een extern bestand; de gegevens van een pakket bevatten hun eigen cellen. |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | De gegevens van het pakket bevatten { $count -><br>    [one] { $count } cel<br>   *[other] { $count } cellen<br>}, hoogstens { $max }. |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | Het pakket bevat { $count } gegevens, hoogstens { $max }. |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | Een gegevensbestand van dit deck is beschadigd of kon niet worden opgeslagen |

### nl/data.ftl

| Key | English | nl |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | het basisbestand van de gegevens is beschadigd of past niet bij de tabel |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | een tabel met een basisbestand geeft voor zijn rijen alleen id’s, geen cellen |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | een waarde past niet in kolom ‘{ $column }’, die { $type -><br>    [number] getallen<br>    [integer] gehele getallen<br>    [bool] waar of onwaar<br>    [date] datums (JJJJ-MM-DD)<br>   *[text] tekst van hoogstens 16 KB<br>} bevat |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | kolom ‘{ $column }’ kan niet zonder verlies { $type -><br>    [number] een getallenkolom<br>    [integer] een kolom met gehele getallen<br>    [bool] een waar/onwaar-kolom<br>    [date] een datumkolom<br>   *[text] een tekstkolom<br>} worden, zie rijen { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}en nog { $more } rij<br>   *[other] {" "}en nog { $more } rijen<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | de kolomlijst is leeg of noemt een kolom twee keer |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | id ‘{ $id }’ is ongeldig: gebruik 1 tot 64 letters, cijfers, - of _ |
| `data-id-missing` | a new row, column or table has no id | een nieuwe rij, kolom of tabel heeft geen id |
| `data-id-taken` | id { $id } is already used in this table | id { $id } wordt al gebruikt in deze tabel |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | ‘{ $name }’ is geen geldige gegevensnaam: gebruik kleine letters, cijfers en _, niet beginnend met een cijfer |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | deze gegevens bewaren hun rijen in een basisbestand dat deze bewerking nog niet kan lezen |
| `data-no-column` | column { $column } does not exist | kolom { $column } bestaat niet |
| `data-no-row` | row { $row } does not exist | rij { $row } bestaat niet |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | { $count -><br>    [one] { $count } getal is niet eindig (NaN of oneindig)<br>   *[other] { $count } getallen zijn niet eindig (NaN of oneindig)<br>} |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | { $count -><br>    [one] { $count } rij had meer cellen dan de kopregel; de extra cellen zijn verwijderd<br>   *[other] { $count } rijen hadden meer cellen dan de kopregel; de extra cellen zijn verwijderd<br>} |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | een rij heeft meer cellen dan de { $max } { $max -><br>    [one] kolom<br>   *[other] kolommen<br>} van de tabel |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] de gegevens zouden hun limiet van { $max } { $max -><br>        [one] rij<br>       *[other] rijen<br>    } overschrijden<br>    [columns] de gegevens zouden hun limiet van { $max } { $max -><br>        [one] kolom<br>       *[other] kolommen<br>    } overschrijden<br>    [text] een tekstcel zou haar limiet van { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    } overschrijden<br>   *[overlay] de gegevens zouden hun limiet van { $max } { $max -><br>        [one] bewerkte cel<br>       *[other] bewerkte cellen<br>    } overschrijden<br>} |

### nl/document.ftl

| Key | English | nl |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | te veel waarden om te tekenen ({ $count }, hoogstens { $max }) |

### nl/mcp.ftl

| Key | English | nl |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | Geen gegevens hebben de id “{ $data }”; data_list somt ze op. |

### nl/session.ftl

| Key | English | nl |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | De gegevensmap van de app kan de gegevensbestanden van de decks niet bevatten |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | Het type van { $subject } kan niet veranderen: andere weergaven tonen de gegevens, die de kolommen missen die dit type nodig heeft. |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | De gegevens ‘{ $data }’ zijn gewijzigd tijdens het lezen van de opgeslagen rijen; probeer het opnieuw |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | De gegevensopslag is vol: { $data } kon niet worden gecomprimeerd. Maak ruimte vrij onder [sources] max_store_mib. |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | Het samenvoegen heeft wijzigingen in { $data } vervangen: de geschiedenis voor ongedaan maken is gewist |
| `session-data-user` | { $element } on slide { $slide } | { $element } op dia { $slide } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | Hier kunnen geen kolommen worden toegevoegd of verwijderd: de weergave { $view } kan ze niet volgen |
| `session-example-base-chart-category` | Category | Categorie |
| `session-example-base-chart-value` | Value | Waarde |
| `session-request-too-large` | The request is larger than { $mib } MiB. | Het verzoek is groter dan { $mib } MiB. |
| `session-undo-data-add` | Add data | Gegevens toevoegen |
| `session-undo-data-bind` | Bind data | Gegevens koppelen |
| `session-undo-data-edit` | Edit data | Gegevens bewerken |
| `session-undo-data-remove` | Remove data | Gegevens verwijderen |
| `session-undo-data-rename` | Rename data | Gegevens hernoemen |
| `session-undo-data-type` | Change the column type | Kolomtype wijzigen |

### nl/ui.ftl

| Key | English | nl |
|---|---|---|
| `ui-data-add` | Add data | Gegevens toevoegen |
| `ui-data-column-type` | Column type | Kolomtype |
| `ui-data-next` | Next rows | Volgende rijen |
| `ui-data-open` | Open the data | Gegevens openen |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | Rijen { $first }–{ $last } van { $total } |
| `ui-data-previous` | Previous rows | Vorige rijen |
| `ui-data-remove` | Remove | Verwijderen |
| `ui-data-rename` | Rename | Naam wijzigen |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | Gedeeld door { $count -><br>    [one] { $count } weergave<br>   *[other] { $count } weergaven<br>} |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | Kolommen: { $columns } · Rijen: { $rows } |
| `ui-data-tab` | Data | Gegevens |
| `ui-data-type-bool` | Yes/no | Ja/nee |
| `ui-data-type-date` | Date (YYYY-MM-DD) | Datum |
| `ui-data-type-integer` | Integer | Geheel getal |
| `ui-data-type-number` | Number | Getal |
| `ui-data-type-text` | Text | Tekst |
| `ui-data-unused` | Unused | Ongebruikt |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count -><br>    [one] { $count } weergave<br>   *[other] { $count } weergaven<br>} |

## pl


### pl/chart.ftl

| Key | English | pl |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns` wymaga `data` i wymienia od 1 do 64 poprawnych, różnych identyfikatorów kolumn |
| `chart-ids-duplicate` | a row or column id is repeated | id wiersza lub kolumny się powtarza |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | id wiersza lub kolumny ma od 1 do 64 liter, cyfr, - lub _ |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids wymaga jednego id na wiersz, a dataset.columns jednego na kolumnę |
| `chart-no-dataset` | the chart has no data table (dataset) | wykres nie ma tabeli danych (dataset) |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | łatka opcji nie może zmieniać `dataset`: użyj edit_data |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | przy zbiorze danych dane muszą pozostać w zbiorze: edytuj je za pomocą edit_data |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | opcja nie może mieć jednocześnie odwołania do danych i danych wbudowanych: zostaw jedno |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` nie może być null: zamiast tego usuń klucz |

### pl/core.ftl

| Key | English | pl |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | Element { $element } przechowuje dane w swoich opcjach: ten plik decku nie ma bieżącego formatu |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | dane „{ $data }”: brak pliku bazowego, zachowane tylko do odczytu |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | dane „{ $data }”: kolumna „{ $column }” zawiera wartości, które nie są typu { $type } |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | dane „{ $data }”: zmiany w zastąpionej tabeli zostały utracone przy scalaniu |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | Element { $element }: starsza wersja zapisała dane jego wykresu lub tabeli w jego opcjach; zostały usunięte |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element } wskazuje dane „{ $data }”, których nie ma w prezentacji |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | nazwa danych „{ $data }” jest już zajęta |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | zbyt wiele danych w prezentacji ({ $count }); nieodczytane: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | dane „{ $data }” zbyt duże, przycięte przy odczycie i zachowane tylko do odczytu |
| `issue-more` | more issues not shown: { $count } | pozostałe niewyświetlone problemy: { $count } |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | { $element } nie jest rysowany: widoki prezentacji czytają więcej danych, niż pozwala renderowanie |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element } czyta kolumnę „{ $column }”, której nie ma w jego danych |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element } pokazuje tylko część swoich danych ({ $what -><br>    [rows] wiersze<br>   *[columns] kolumny<br>}: { $shown } z { $total }) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | { $element } nie jest rysowany: nie można odczytać jego opcji |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | Partia jest za duża: liczba komórek { $count }, najwyżej { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | Kolumnę „{ $column }” pokazują { $views }; najpierw powiąż je z innymi kolumnami (bind_data z columns) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | Dane „{ $data }” nie mają kolumny „{ $column }” |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | Dane prezentacji przekroczyłyby swój limit: najwyżej { $max } { $what -><br>    [row-ids] identyfikatorów wierszy<br>    [overlay] zmienionych komórek<br>    [text] bajtów tekstu<br>   *[data] danych<br>} |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | Te dane pokazują { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}i { $more } inny widok<br>    [few] {" "}i { $more } inne widoki<br>    [many] {" "}i { $more } innych widoków<br>   *[other] {" "}i { $more } innego widoku<br>}; najpierw je usuń lub powiąż z innymi danymi |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | Element { $subject } przenosi dane w swoich opcjach; dodaj je jako dane decku przez add_data i powiąż przez bind_data |
| `op-data-internal` | A batch cannot set a table's base or overlay | Partia nie może ustawić bazy ani nakładki tabeli |
| `op-data-missing` | Data “{ $data }” not found | Nie znaleziono danych „{ $data }” |
| `op-data-name-taken` | The data name “{ $name }” is already taken | Nazwa danych „{ $name }” jest już zajęta |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | Parametr „{ $param }” nie może pokazywać danych: potrafi to tylko opcja wykresu lub tabeli |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | Element { $subject } pokazuje już inne dane; użyj bind_data, aby je zmienić |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | Dane „{ $data }” zostały obcięte przy odczycie i są tylko do odczytu; aby je edytować, zastąp całą tabelę |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | Historia prezentacji zbliża się do { $max } MiB, maksimum pozwalającego ją ponownie otworzyć: usuń zawartość, zanim dodasz więcej. |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | Dane „{ $data }” pakietu odwołują się do pliku zewnętrznego; dane pakietu zawierają własne komórki. |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | Dane pakietu zawierają { $count -><br>    [one] { $count } komórkę<br>    [few] { $count } komórki<br>   *[many] { $count } komórek<br>}, najwyżej { $max }. |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | Pakiet zawiera { $count -><br>    [one] { $count } zbiór danych<br>    [few] { $count } zbiory danych<br>   *[many] { $count } zbiorów danych<br>}, najwyżej { $max }. |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | Plik danych tej prezentacji jest uszkodzony lub nie można go zapisać |

### pl/data.ftl

| Key | English | pl |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | plik bazowy danych jest uszkodzony lub nie pasuje do ich tabeli |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | tabela z plikiem bazowym podaje dla wierszy tylko identyfikatory, bez komórek |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | wartość nie pasuje do kolumny „{ $column }”, która zawiera { $type -><br>    [number] liczby<br>    [integer] liczby całkowite<br>    [bool] prawdę lub fałsz<br>    [date] daty (RRRR-MM-DD)<br>   *[text] tekst do 16 KB<br>} |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | kolumny „{ $column }” nie można bez strat zmienić na { $type -><br>    [number] kolumnę liczb<br>    [integer] kolumnę liczb całkowitych<br>    [bool] kolumnę prawda/fałsz<br>    [date] kolumnę dat<br>   *[text] kolumnę tekstu<br>}, zobacz wiersze { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}i jeszcze { $more } wiersz<br>    [few] {" "}i jeszcze { $more } wiersze<br>   *[many] {" "}i jeszcze { $more } wierszy<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | lista kolumn jest pusta lub wymienia kolumnę dwa razy |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | identyfikator „{ $id }” jest nieprawidłowy: użyj od 1 do 64 liter łacińskich, cyfr, - lub _ |
| `data-id-missing` | a new row, column or table has no id | nowy wiersz, kolumna lub tabela nie ma identyfikatora |
| `data-id-taken` | id { $id } is already used in this table | identyfikator { $id } jest już używany w tej tabeli |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | „{ $name }” nie jest prawidłową nazwą danych: użyj małych liter łacińskich, cyfr i _, nie zaczynając od cyfry |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | te dane trzymają wiersze w pliku bazowym, którego ta zmiana jeszcze nie umie odczytać |
| `data-no-column` | column { $column } does not exist | kolumna { $column } nie istnieje |
| `data-no-row` | row { $row } does not exist | wiersz { $row } nie istnieje |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | { $count -><br>    [one] { $count } liczba nie jest skończona (NaN lub nieskończoność)<br>    [few] { $count } liczby nie są skończone (NaN lub nieskończoność)<br>   *[many] { $count } liczb nie jest skończonych (NaN lub nieskończoność)<br>} |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | { $count -><br>    [one] { $count } wiersz miał więcej komórek niż nagłówek; nadmiarowe komórki usunięto<br>    [few] { $count } wiersze miały więcej komórek niż nagłówek; nadmiarowe komórki usunięto<br>   *[many] { $count } wierszy miało więcej komórek niż nagłówek; nadmiarowe komórki usunięto<br>} |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | wiersz ma więcej komórek niż { $max } { $max -><br>    [one] kolumna<br>    [few] kolumny<br>   *[many] kolumn<br>} tabeli |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] dane przekroczyłyby limit wierszy: { $max } { $max -><br>        [one] wiersz<br>        [few] wiersze<br>       *[many] wierszy<br>    }<br>    [columns] dane przekroczyłyby limit kolumn: { $max } { $max -><br>        [one] kolumna<br>        [few] kolumny<br>       *[many] kolumn<br>    }<br>    [text] komórka tekstowa przekroczyłaby limit: { $max } { $max -><br>        [one] bajt<br>        [few] bajty<br>       *[many] bajtów<br>    }<br>   *[overlay] dane przekroczyłyby limit edytowanych komórek: { $max } { $max -><br>        [one] komórka<br>        [few] komórki<br>       *[many] komórek<br>    }<br>} |

### pl/document.ftl

| Key | English | pl |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | za dużo wartości do narysowania ({ $count }, najwyżej { $max }) |

### pl/mcp.ftl

| Key | English | pl |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | Żadne dane nie mają identyfikatora „{ $data }”; data_list je wyświetla. |

### pl/session.ftl

| Key | English | pl |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | Folder danych aplikacji nie może przechowywać plików danych prezentacji |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | Nie można zmienić typu { $subject }: inne widoki pokazują jego dane, którym brakuje kolumn potrzebnych temu typowi. |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | Dane „{ $data }” zmieniły się podczas odczytu zapisanych wierszy; spróbuj ponownie |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | Magazyn danych jest pełny: nie udało się skompaktować { $data }. Zwolnij miejsce w [sources] max_store_mib. |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | Scalenie zastąpiło zmiany w { $data }: historia cofania została wyczyszczona |
| `session-data-user` | { $element } on slide { $slide } | { $element } na slajdzie { $slide } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | Nie można tu dodać ani usunąć kolumn: widok { $view } nie może za nimi podążać |
| `session-example-base-chart-category` | Category | Kategoria |
| `session-example-base-chart-value` | Value | Wartość |
| `session-request-too-large` | The request is larger than { $mib } MiB. | Żądanie przekracza { $mib } MiB. |
| `session-undo-data-add` | Add data | Dodaj dane |
| `session-undo-data-bind` | Bind data | Powiąż dane |
| `session-undo-data-edit` | Edit data | Edytuj dane |
| `session-undo-data-remove` | Remove data | Usuń dane |
| `session-undo-data-rename` | Rename data | Zmień nazwę danych |
| `session-undo-data-type` | Change the column type | Zmień typ kolumny |

### pl/ui.ftl

| Key | English | pl |
|---|---|---|
| `ui-data-add` | Add data | Dodaj dane |
| `ui-data-column-type` | Column type | Typ kolumny |
| `ui-data-next` | Next rows | Następne wiersze |
| `ui-data-open` | Open the data | Otwórz dane |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | Wiersze { $first }–{ $last } z { $total } |
| `ui-data-previous` | Previous rows | Poprzednie wiersze |
| `ui-data-remove` | Remove | Usuń |
| `ui-data-rename` | Rename | Zmień nazwę |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | Dane wspólne · { $count -><br>    [one] { $count } widok<br>    [few] { $count } widoki<br>    [many] { $count } widoków<br>   *[other] { $count } widoku<br>} |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | Kolumny: { $columns } · Wiersze: { $rows } |
| `ui-data-tab` | Data | Dane |
| `ui-data-type-bool` | Yes/no | Tak/nie |
| `ui-data-type-date` | Date (YYYY-MM-DD) | Data |
| `ui-data-type-integer` | Integer | Liczba całkowita |
| `ui-data-type-number` | Number | Liczba |
| `ui-data-type-text` | Text | Tekst |
| `ui-data-unused` | Unused | Nieużywane |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count -><br>    [one] { $count } widok<br>    [few] { $count } widoki<br>    [many] { $count } widoków<br>   *[other] { $count } widoku<br>} |

## pt


### pt/chart.ftl

| Key | English | pt |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns` exige `data` e lista de 1 a 64 identificadores de coluna válidos e distintos |
| `chart-ids-duplicate` | a row or column id is repeated | um id de linha ou de coluna está repetido |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | um id de linha ou de coluna tem de 1 a 64 letras, dígitos, - ou _ |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids precisa de um id por linha e dataset.columns de um por coluna |
| `chart-no-dataset` | the chart has no data table (dataset) | o gráfico não tem tabela de dados (dataset) |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | um patch de opções não pode alterar `dataset`: use edit_data |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | com um dataset, os dados devem ficar no dataset: edite-os com edit_data |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | uma opção não pode ter uma referência de dados e dados em linha: mantenha só uma |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` não pode ser null: remova a chave em vez disso |

### pt/core.ftl

| Key | English | pt |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | O elemento { $element } guarda os seus dados nas suas opções: este ficheiro de deck não está no formato atual |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | dados «{ $data }»: falta o arquivo base, mantidos somente leitura |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | dados «{ $data }»: a coluna «{ $column }» contém valores que não são do tipo { $type } |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | dados «{ $data }»: alterações em uma tabela substituída foram perdidas na mesclagem |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | Elemento { $element }: uma versão anterior guardou os dados do seu gráfico ou tabela nas suas opções; foram removidos |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element } nomeia os dados «{ $data }», que não estão na apresentação |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | o nome de dados «{ $data }» já está em uso |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | dados demais na apresentação ({ $count }); não lidos: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | dados «{ $data }» grandes demais, truncados na leitura e mantidos somente leitura |
| `issue-more` | more issues not shown: { $count } | outros problemas não exibidos: { $count } |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | { $element } não é desenhado: as vistas da apresentação leem mais dados do que uma renderização permite |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element } lê a coluna «{ $column }», que os seus dados não têm |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element } mostra só parte dos seus dados ({ $what -><br>    [rows] linhas<br>   *[columns] colunas<br>}: { $shown } de { $total }) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | { $element } não é desenhado: as suas opções não podem ser lidas |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | O lote é grande demais: número de células { $count }, no máximo { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | A coluna «{ $column }» é mostrada por { $views }; vincule-os primeiro a outras colunas (bind_data com columns) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | Os dados «{ $data }» não têm a coluna «{ $column }» |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | Os dados da apresentação passariam do limite: no máximo { $max } { $what -><br>    [row-ids] IDs de linha<br>    [overlay] células alteradas<br>    [text] bytes de texto<br>   *[data] dados<br>} |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | Estes dados são mostrados por { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}e mais { $more } vista<br>   *[other] {" "}e mais { $more } vistas<br>}; remova-as ou vincule-as primeiro a outros dados |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | O elemento { $subject } traz os seus dados nas suas opções; adicione-os como dados do deck com add_data e ligue-os com bind_data |
| `op-data-internal` | A batch cannot set a table's base or overlay | Um lote não pode definir a base nem a sobreposição de uma tabela |
| `op-data-missing` | Data “{ $data }” not found | Dados «{ $data }» não encontrados |
| `op-data-name-taken` | The data name “{ $name }” is already taken | O nome de dados «{ $name }» já está em uso |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | O parâmetro «{ $param }» não pode mostrar dados: só uma opção de gráfico ou de tabela pode |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | O elemento { $subject } já mostra outros dados; use bind_data para os mudar |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | Os dados «{ $data }» foram truncados na leitura e são só de leitura; substitua toda a tabela para os editar |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | O histórico da apresentação está quase em { $max } MiB, o máximo para reabri-la: remova conteúdo antes de adicionar mais. |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | Os dados “{ $data }” do pacote remetem a um arquivo externo; os dados de um pacote contêm as próprias células. |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | Os dados do pacote contêm { $count -><br>    [one] { $count } célula<br>   *[other] { $count } células<br>}, no máximo { $max }. |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | O pacote contém { $count -><br>    [one] { $count } dado<br>   *[other] { $count } dados<br>}, no máximo { $max }. |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | Um ficheiro de dados desta apresentação está danificado ou não pôde ser guardado |

### pt/data.ftl

| Key | English | pt |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | o arquivo base dos dados está danificado ou não corresponde à sua tabela |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | uma tabela com arquivo base só indica os ids das linhas, não células |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | um valor não cabe na coluna “{ $column }”, que contém { $type -><br>    [number] números<br>    [integer] números inteiros<br>    [bool] verdadeiro ou falso<br>    [date] datas (AAAA-MM-DD)<br>   *[text] texto de no máximo 16 KB<br>} |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | a coluna “{ $column }” não pode virar sem perda { $type -><br>    [number] uma coluna de números<br>    [integer] uma coluna de inteiros<br>    [bool] uma coluna de verdadeiro ou falso<br>    [date] uma coluna de datas<br>   *[text] uma coluna de texto<br>}, veja as linhas { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}e mais { $more } linha<br>   *[other] {" "}e mais { $more } linhas<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | a lista de colunas está vazia ou cita uma coluna duas vezes |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | o identificador “{ $id }” não é válido: use de 1 a 64 letras, dígitos, - ou _ |
| `data-id-missing` | a new row, column or table has no id | uma nova linha, coluna ou tabela não tem identificador |
| `data-id-taken` | id { $id } is already used in this table | o identificador { $id } já é usado nesta tabela |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | “{ $name }” não é um nome de dados válido: use minúsculas, dígitos e _, sem começar por um dígito |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | estes dados guardam as linhas em um arquivo base que esta edição ainda não consegue ler |
| `data-no-column` | column { $column } does not exist | a coluna { $column } não existe |
| `data-no-row` | row { $row } does not exist | a linha { $row } não existe |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | { $count -><br>    [one] { $count } número não é finito (NaN ou infinito)<br>   *[other] { $count } números não são finitos (NaN ou infinito)<br>} |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | { $count -><br>    [one] { $count } linha tinha mais células que o cabeçalho; as células a mais foram removidas<br>   *[other] { $count } linhas tinham mais células que o cabeçalho; as células a mais foram removidas<br>} |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | uma linha tem mais células que as { $max } { $max -><br>    [one] coluna<br>   *[other] colunas<br>} da tabela |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] os dados passariam do limite de { $max } { $max -><br>        [one] linha<br>       *[other] linhas<br>    }<br>    [columns] os dados passariam do limite de { $max } { $max -><br>        [one] coluna<br>       *[other] colunas<br>    }<br>    [text] uma célula de texto passaria do limite de { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] os dados passariam do limite de { $max } { $max -><br>        [one] célula editada<br>       *[other] células editadas<br>    }<br>} |

### pt/document.ftl

| Key | English | pt |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | valores demais para desenhar ({ $count }, no máximo { $max }) |

### pt/mcp.ftl

| Key | English | pt |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | Nenhum dado tem o id “{ $data }”; data_list os lista. |

### pt/session.ftl

| Key | English | pt |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | A pasta de dados do app não pode guardar os arquivos de dados dos decks |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | Não é possível mudar o tipo de { $subject }: outras vistas mostram os seus dados, que não têm as colunas de que este tipo precisa. |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | Os dados «{ $data }» mudaram enquanto as linhas guardadas eram lidas; tente novamente |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | O armazenamento de dados está cheio: não foi possível compactar { $data }. Libere espaço em [sources] max_store_mib. |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | A mesclagem substituiu alterações em { $data }: o histórico de desfazer foi apagado |
| `session-data-user` | { $element } on slide { $slide } | { $element } no slide { $slide } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | Não é possível adicionar ou remover colunas aqui: a vista { $view } não as pode acompanhar |
| `session-example-base-chart-category` | Category | Categoria |
| `session-example-base-chart-value` | Value | Valor |
| `session-request-too-large` | The request is larger than { $mib } MiB. | A solicitação ultrapassa { $mib } MiB. |
| `session-undo-data-add` | Add data | Adicionar dados |
| `session-undo-data-bind` | Bind data | Vincular dados |
| `session-undo-data-edit` | Edit data | Editar dados |
| `session-undo-data-remove` | Remove data | Remover dados |
| `session-undo-data-rename` | Rename data | Renomear dados |
| `session-undo-data-type` | Change the column type | Alterar o tipo da coluna |

### pt/ui.ftl

| Key | English | pt |
|---|---|---|
| `ui-data-add` | Add data | Adicionar dados |
| `ui-data-column-type` | Column type | Tipo de coluna |
| `ui-data-next` | Next rows | Próximas linhas |
| `ui-data-open` | Open the data | Abrir os dados |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | Linhas { $first }–{ $last } de { $total } |
| `ui-data-previous` | Previous rows | Linhas anteriores |
| `ui-data-remove` | Remove | Remover |
| `ui-data-rename` | Rename | Renomear |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | Partilhados por { $count -><br>    [one] { $count } vista<br>   *[other] { $count } vistas<br>} |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | Colunas: { $columns } · Linhas: { $rows } |
| `ui-data-tab` | Data | Dados |
| `ui-data-type-bool` | Yes/no | Sim/não |
| `ui-data-type-date` | Date (YYYY-MM-DD) | Data |
| `ui-data-type-integer` | Integer | Inteiro |
| `ui-data-type-number` | Number | Número |
| `ui-data-type-text` | Text | Texto |
| `ui-data-unused` | Unused | Não usados |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count -><br>    [one] { $count } visualização<br>   *[other] { $count } visualizações<br>} |

## ru


### ru/chart.ftl

| Key | English | ru |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns` требует `data` и перечисляет от 1 до 64 допустимых различных идентификаторов столбцов |
| `chart-ids-duplicate` | a row or column id is repeated | id строки или столбца повторяется |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | id строки или столбца содержит от 1 до 64 букв, цифр, - или _ |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids требует один id на строку, а dataset.columns — один на столбец |
| `chart-no-dataset` | the chart has no data table (dataset) | у диаграммы нет таблицы данных (dataset) |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | изменение параметров не может менять `dataset`: используйте edit_data |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | при наличии dataset данные должны оставаться в dataset: меняйте их через edit_data |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | параметр не может содержать одновременно ссылку на данные и встроенные данные: оставьте одно |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` не может быть null: вместо этого удалите ключ |

### ru/core.ftl

| Key | English | ru |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | Элемент { $element } хранит данные в своих параметрах: этот файл презентации не в текущем формате |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | данные «{ $data }»: отсутствует базовый файл, доступны только для чтения |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | данные «{ $data }»: столбец «{ $column }» содержит значения не типа { $type } |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | данные «{ $data }»: изменения заменённой таблицы потеряны при слиянии |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | Элемент { $element }: старая версия сохранила данные его диаграммы или таблицы в его параметрах; они удалены |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element } ссылается на данные «{ $data }», которых нет в презентации |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | имя данных «{ $data }» уже занято |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | слишком много данных в презентации ({ $count }); не прочитаны: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | данные «{ $data }» слишком велики, усечены при чтении и доступны только для чтения |
| `issue-more` | more issues not shown: { $count } | другие непоказанные проблемы: { $count } |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | { $element } не отрисован: представления презентации читают больше данных, чем допускает отрисовка |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element } читает столбец «{ $column }», которого нет в его данных |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element } показывает лишь часть своих данных ({ $what -><br>    [rows] строки<br>   *[columns] столбцы<br>}: { $shown } из { $total }) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | { $element } не отрисован: его параметры не читаются |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | Пакет слишком велик: число ячеек { $count }, не более { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | Столбец «{ $column }» показывают { $views }; сначала привяжите их к другим столбцам (bind_data с columns) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | В данных «{ $data }» нет столбца «{ $column }» |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | Данные презентации превысили бы лимит: не более { $max } { $what -><br>    [row-ids] идентификаторов строк<br>    [overlay] изменённых ячеек<br>    [text] байт текста<br>   *[data] данных<br>} |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | Эти данные показывают { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}и ещё { $more } представление<br>    [few] {" "}и ещё { $more } представления<br>    [many] {" "}и ещё { $more } представлений<br>   *[other] {" "}и ещё { $more } представления<br>}; сначала удалите их или привяжите к другим данным |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | Элемент { $subject } содержит данные в своих параметрах; добавьте их как данные презентации через add_data и привяжите через bind_data |
| `op-data-internal` | A batch cannot set a table's base or overlay | Пакет не может задавать основу или наложение таблицы |
| `op-data-missing` | Data “{ $data }” not found | Данные «{ $data }» не найдены |
| `op-data-name-taken` | The data name “{ $name }” is already taken | Имя данных «{ $name }» уже занято |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | Параметр «{ $param }» не может показывать данные: это умеет только параметр диаграммы или таблицы |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | Элемент { $subject } уже показывает другие данные; чтобы их сменить, используйте bind_data |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | Данные «{ $data }» обрезаны при чтении и доступны только для чтения; чтобы их изменить, замените всю таблицу |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | История презентации почти достигла { $max } МиБ — максимума, при котором её можно снова открыть: удалите содержимое, прежде чем добавлять новое. |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | Данные «{ $data }» пакета ссылаются на внешний файл; данные пакета содержат собственные ячейки. |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | Данные пакета содержат { $count -><br>    [one] { $count } ячейку<br>    [few] { $count } ячейки<br>   *[many] { $count } ячеек<br>}, не более { $max }. |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | Пакет содержит { $count -><br>    [one] { $count } набор данных<br>    [few] { $count } набора данных<br>   *[many] { $count } наборов данных<br>}, не более { $max }. |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | Файл данных этой презентации повреждён или не может быть сохранён |

### ru/data.ftl

| Key | English | ru |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | базовый файл данных повреждён или не соответствует их таблице |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | таблица с базовым файлом задаёт для строк только идентификаторы, без ячеек |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | значение не подходит для столбца «{ $column }», в котором { $type -><br>    [number] числа<br>    [integer] целые числа<br>    [bool] истина или ложь<br>    [date] даты (ГГГГ-ММ-ДД)<br>   *[text] текст не длиннее 16 КБ<br>} |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | столбец «{ $column }» нельзя без потерь сделать { $type -><br>    [number] числовым<br>    [integer] целочисленным<br>    [bool] логическим<br>    [date] столбцом дат<br>   *[text] текстовым<br>}, см. строки { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}и ещё { $more } строка<br>    [few] {" "}и ещё { $more } строки<br>   *[many] {" "}и ещё { $more } строк<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | список столбцов пуст или называет столбец дважды |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | идентификатор «{ $id }» недопустим: используйте от 1 до 64 латинских букв, цифр, - или _ |
| `data-id-missing` | a new row, column or table has no id | у новой строки, столбца или таблицы нет идентификатора |
| `data-id-taken` | id { $id } is already used in this table | идентификатор { $id } уже используется в этой таблице |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | «{ $name }» — недопустимое имя данных: используйте строчные латинские буквы, цифры и _, не начиная с цифры |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | эти данные хранят строки в базовом файле, который это изменение пока не умеет читать |
| `data-no-column` | column { $column } does not exist | столбца { $column } не существует |
| `data-no-row` | row { $row } does not exist | строки { $row } не существует |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | { $count -><br>    [one] { $count } число не является конечным (NaN или бесконечность)<br>    [few] { $count } числа не являются конечными (NaN или бесконечность)<br>   *[many] { $count } чисел не являются конечными (NaN или бесконечность)<br>} |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | { $count -><br>    [one] { $count } строка содержала больше ячеек, чем заголовок; лишние ячейки удалены<br>    [few] { $count } строки содержали больше ячеек, чем заголовок; лишние ячейки удалены<br>   *[many] { $count } строк содержали больше ячеек, чем заголовок; лишние ячейки удалены<br>} |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | в строке больше ячеек, чем { $max } { $max -><br>    [one] столбец<br>    [few] столбца<br>   *[many] столбцов<br>} таблицы |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] данные превысили бы предел строк — { $max } { $max -><br>        [one] строка<br>        [few] строки<br>       *[many] строк<br>    }<br>    [columns] данные превысили бы предел столбцов — { $max } { $max -><br>        [one] столбец<br>        [few] столбца<br>       *[many] столбцов<br>    }<br>    [text] текстовая ячейка превысила бы предел — { $max } { $max -><br>        [one] байт<br>        [few] байта<br>       *[many] байтов<br>    }<br>   *[overlay] данные превысили бы предел изменённых ячеек — { $max } { $max -><br>        [one] ячейка<br>        [few] ячейки<br>       *[many] ячеек<br>    }<br>} |

### ru/document.ftl

| Key | English | ru |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | слишком много значений для отрисовки ({ $count }, не более { $max }) |

### ru/mcp.ftl

| Key | English | ru |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | Нет данных с идентификатором «{ $data }»; их перечисляет data_list. |

### ru/session.ftl

| Key | English | ru |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | Папка данных приложения не может хранить файлы данных презентаций |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | Нельзя сменить тип { $subject }: его данные показаны в других видах, и в них нет столбцов, нужных этому типу. |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | Данные «{ $data }» изменились во время чтения сохранённых строк; повторите попытку |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | Хранилище данных заполнено: не удалось сжать { $data }. Освободите место в [sources] max_store_mib. |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | Слияние заменило изменения в { $data }: история отмены очищена |
| `session-data-user` | { $element } on slide { $slide } | { $element } на слайде { $slide } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | Здесь нельзя добавить или удалить столбцы: представление { $view } не может за ними следовать |
| `session-example-base-chart-category` | Category | Категория |
| `session-example-base-chart-value` | Value | Значение |
| `session-request-too-large` | The request is larger than { $mib } MiB. | Запрос больше { $mib } МиБ. |
| `session-undo-data-add` | Add data | Добавить данные |
| `session-undo-data-bind` | Bind data | Привязать данные |
| `session-undo-data-edit` | Edit data | Изменить данные |
| `session-undo-data-remove` | Remove data | Удалить данные |
| `session-undo-data-rename` | Rename data | Переименовать данные |
| `session-undo-data-type` | Change the column type | Изменить тип столбца |

### ru/ui.ftl

| Key | English | ru |
|---|---|---|
| `ui-data-add` | Add data | Добавить данные |
| `ui-data-column-type` | Column type | Тип столбца |
| `ui-data-next` | Next rows | Следующие строки |
| `ui-data-open` | Open the data | Открыть данные |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | Строки { $first }–{ $last } из { $total } |
| `ui-data-previous` | Previous rows | Предыдущие строки |
| `ui-data-remove` | Remove | Удалить |
| `ui-data-rename` | Rename | Переименовать |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | Общие данные · { $count -><br>    [one] { $count } представление<br>    [few] { $count } представления<br>    [many] { $count } представлений<br>   *[other] { $count } представления<br>} |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | Столбцы: { $columns } · Строки: { $rows } |
| `ui-data-tab` | Data | Данные |
| `ui-data-type-bool` | Yes/no | Да/нет |
| `ui-data-type-date` | Date (YYYY-MM-DD) | Дата |
| `ui-data-type-integer` | Integer | Целое число |
| `ui-data-type-number` | Number | Число |
| `ui-data-type-text` | Text | Текст |
| `ui-data-unused` | Unused | Не используются |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count -><br>    [one] { $count } представление<br>    [few] { $count } представления<br>    [many] { $count } представлений<br>   *[other] { $count } представления<br>} |

## th


### th/chart.ftl

| Key | English | th |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns` ต้องมี `data` และระบุรหัสคอลัมน์ที่ถูกต้องและไม่ซ้ำกัน 1 ถึง 64 รายการ |
| `chart-ids-duplicate` | a row or column id is repeated | id ของแถวหรือคอลัมน์ซ้ำกัน |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | id ของแถวหรือคอลัมน์มีตัวอักษร ตัวเลข - หรือ _ 1 ถึง 64 ตัว |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids ต้องมี id หนึ่งรายการต่อแถว และ dataset.columns หนึ่งรายการต่อคอลัมน์ |
| `chart-no-dataset` | the chart has no data table (dataset) | แผนภูมิไม่มีตารางข้อมูล (dataset) |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | แพตช์ตัวเลือกเปลี่ยน `dataset` ไม่ได้: ให้ใช้ edit_data |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | เมื่อมี dataset ข้อมูลต้องอยู่ใน dataset: แก้ไขด้วย edit_data |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | ตัวเลือกหนึ่งมีทั้งการอ้างอิงข้อมูลและข้อมูลในบรรทัดไม่ได้ ให้เก็บไว้อย่างเดียว |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` เป็น null ไม่ได้ ให้ลบคีย์ออกแทน |

### th/core.ftl

| Key | English | th |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | องค์ประกอบ { $element } เก็บข้อมูลไว้ในตัวเลือก: ไฟล์เด็คนี้ไม่ใช่รูปแบบปัจจุบัน |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | ข้อมูล “{ $data }”: ไม่พบไฟล์ฐาน เก็บไว้แบบอ่านอย่างเดียว |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | ข้อมูล “{ $data }”: คอลัมน์ “{ $column }” มีค่าที่ไม่ใช่ชนิด { $type } |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | ข้อมูล “{ $data }”: การแก้ไขตารางที่ถูกแทนที่สูญหายระหว่างการผสาน |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | องค์ประกอบ { $element }: เวอร์ชันเก่าเก็บข้อมูลของแผนภูมิหรือตารางไว้ในตัวเลือก ข้อมูลนั้นถูกนำออกแล้ว |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element } อ้างถึงข้อมูล “{ $data }” ซึ่งไม่มีในงานนำเสนอ |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | ชื่อข้อมูล “{ $data }” ถูกใช้แล้ว |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | ข้อมูลในเด็คมากเกินไป ({ $count }) ที่ไม่ได้อ่าน: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | ข้อมูล “{ $data }” ใหญ่เกินไป ถูกตัดทอนขณะอ่านและเก็บไว้แบบอ่านอย่างเดียว |
| `issue-more` | more issues not shown: { $count } | ปัญหาอื่นที่ไม่ได้แสดง: { $count } |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | ไม่ได้วาด { $element }: มุมมองของงานนำเสนออ่านข้อมูลมากกว่าที่การเรนเดอร์อนุญาต |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element } อ่านคอลัมน์ “{ $column }” ซึ่งไม่มีในข้อมูลของมัน |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element } แสดงข้อมูลเพียงบางส่วน ({ $what -><br>    [rows] แถว<br>   *[columns] คอลัมน์<br>}: { $shown } จาก { $total }) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | ไม่ได้วาด { $element }: อ่านตัวเลือกของมันไม่ได้ |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | ชุดคำสั่งใหญ่เกินไป: จำนวนเซลล์ { $count } สูงสุด { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | คอลัมน์ “{ $column }” แสดงโดย { $views } ให้ผูกกับคอลัมน์อื่นก่อน (bind_data พร้อม columns) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | ข้อมูล “{ $data }” ไม่มีคอลัมน์ “{ $column }” |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | ข้อมูลของเด็คจะเกินขีดจำกัด: { $what -><br>    [row-ids] รหัสแถว<br>    [overlay] เซลล์ที่แก้ไข<br>    [text] ไบต์ของข้อความ<br>   *[data] ข้อมูล<br>} ได้ไม่เกิน { $max } |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | ข้อมูลนี้แสดงโดย { $views }{ $more -><br>    [0] {""}<br>   *[other] {" "}และอีก { $more } มุมมอง<br>}{" "}ให้ลบหรือผูกกับข้อมูลอื่นก่อน |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | องค์ประกอบ { $subject } เก็บข้อมูลไว้ในตัวเลือก ให้เพิ่มเป็นข้อมูลของเด็คด้วย add_data และผูกด้วย bind_data |
| `op-data-internal` | A batch cannot set a table's base or overlay | ชุดคำสั่งไม่สามารถตั้งค่าฐานหรือชั้นซ้อนของตารางได้ |
| `op-data-missing` | Data “{ $data }” not found | ไม่พบข้อมูล “{ $data }” |
| `op-data-name-taken` | The data name “{ $name }” is already taken | ชื่อข้อมูล “{ $name }” ถูกใช้แล้ว |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | พารามิเตอร์ “{ $param }” แสดงข้อมูลไม่ได้ ทำได้เฉพาะตัวเลือกของแผนภูมิหรือตาราง |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | องค์ประกอบ { $subject } แสดงข้อมูลอื่นอยู่แล้ว ใช้ bind_data เพื่อเปลี่ยน |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | ข้อมูล “{ $data }” ถูกตัดทอนตอนอ่านและอ่านได้อย่างเดียว ให้แทนที่ตารางทั้งหมดเพื่อแก้ไข |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | ประวัติของงานนำเสนอใกล้ถึง { $max } MiB ซึ่งเป็นขนาดสูงสุดที่เปิดใหม่ได้: โปรดลบเนื้อหาก่อนเพิ่มอีก |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | ข้อมูล “{ $data }” ของแพ็กอ้างอิงไฟล์ภายนอก ข้อมูลของแพ็กต้องมีเซลล์ของตัวเอง |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | ข้อมูลของแพ็กมี { $count } เซลล์ สูงสุด { $max } เซลล์ |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | แพ็กมีข้อมูล { $count } รายการ สูงสุด { $max } รายการ |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | ไฟล์ข้อมูลของเด็คนี้เสียหายหรือไม่สามารถจัดเก็บได้ |

### th/data.ftl

| Key | English | th |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | ไฟล์ฐานของข้อมูลเสียหายหรือไม่ตรงกับตาราง |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | ตารางที่มีไฟล์ฐานระบุเฉพาะรหัสของแถว ไม่ระบุเซลล์ |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | ค่าหนึ่งไม่เข้ากับคอลัมน์ “{ $column }” ซึ่งเก็บ{ $type -><br>    [number] ตัวเลข<br>    [integer] จำนวนเต็ม<br>    [bool] จริงหรือเท็จ<br>    [date] วันที่ (YYYY-MM-DD)<br>   *[text] ข้อความไม่เกิน 16 KB<br>} |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | เปลี่ยนคอลัมน์ “{ $column }” เป็น{ $type -><br>    [number] คอลัมน์ตัวเลข<br>    [integer] คอลัมน์จำนวนเต็ม<br>    [bool] คอลัมน์จริง/เท็จ<br>    [date] คอลัมน์วันที่<br>   *[text] คอลัมน์ข้อความ<br>}โดยไม่สูญเสียข้อมูลไม่ได้ ดูแถว { $rows }{ $more -><br>    [0] {""}<br>   *[other] {" "}และแถวอื่นอีก { $more } รายการ<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | รายการคอลัมน์ว่างหรือระบุคอลัมน์ซ้ำ |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | รหัส “{ $id }” ไม่ถูกต้อง: ใช้ตัวอักษร ตัวเลข - หรือ _ 1 ถึง 64 ตัว |
| `data-id-missing` | a new row, column or table has no id | แถว คอลัมน์ หรือตารางใหม่ไม่มีรหัส |
| `data-id-taken` | id { $id } is already used in this table | รหัส { $id } ถูกใช้ในตารางนี้แล้ว |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | “{ $name }” ไม่ใช่ชื่อข้อมูลที่ถูกต้อง: ใช้อักษรตัวเล็ก ตัวเลข และ _ โดยไม่ขึ้นต้นด้วยตัวเลข |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | ข้อมูลนี้เก็บแถวไว้ในไฟล์ฐานซึ่งการแก้ไขนี้ยังอ่านไม่ได้ |
| `data-no-column` | column { $column } does not exist | ไม่มีคอลัมน์ { $column } |
| `data-no-row` | row { $row } does not exist | ไม่มีแถว { $row } |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | ตัวเลข { $count } รายการไม่เป็นค่าจำกัด (NaN หรืออนันต์) |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | แถว { $count } รายการมีเซลล์มากกว่าส่วนหัว จึงลบเซลล์ที่เกินออกแล้ว |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | แถวหนึ่งมีเซลล์มากกว่าคอลัมน์ของตาราง { $max } รายการ |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] ข้อมูลจะเกินขีดจำกัดแถว { $max } รายการ<br>    [columns] ข้อมูลจะเกินขีดจำกัดคอลัมน์ { $max } รายการ<br>    [text] เซลล์ข้อความจะเกินขีดจำกัด { $max } ไบต์<br>   *[overlay] ข้อมูลจะเกินขีดจำกัดเซลล์ที่แก้ไข { $max } รายการ<br>} |

### th/document.ftl

| Key | English | th |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | มีค่าที่จะวาดมากเกินไป ({ $count } สูงสุด { $max }) |

### th/mcp.ftl

| Key | English | th |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | ไม่มีข้อมูลที่มีรหัส “{ $data }” ใช้ data_list เพื่อดูรายการ |

### th/session.ftl

| Key | English | th |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | โฟลเดอร์ข้อมูลของแอปไม่สามารถเก็บไฟล์ข้อมูลของเด็คได้ |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | เปลี่ยนชนิดของ { $subject } ไม่ได้: มุมมองอื่นแสดงข้อมูลนี้อยู่ และข้อมูลไม่มีคอลัมน์ที่ชนิดนี้ต้องใช้ |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | ข้อมูล “{ $data }” เปลี่ยนไประหว่างอ่านแถวที่บันทึกไว้ โปรดลองอีกครั้ง |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | ที่เก็บข้อมูลเต็ม: ไม่สามารถบีบอัด { $data } ได้ โปรดเพิ่มพื้นที่ว่างภายใต้ [sources] max_store_mib |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | การรวมได้แทนที่การแก้ไขใน { $data }: ล้างประวัติการเลิกทำแล้ว |
| `session-data-user` | { $element } on slide { $slide } | { $element } ในสไลด์ { $slide } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | เพิ่มหรือลบคอลัมน์ที่นี่ไม่ได้ มุมมอง { $view } ตามคอลัมน์ไม่ได้ |
| `session-example-base-chart-category` | Category | หมวดหมู่ |
| `session-example-base-chart-value` | Value | ค่า |
| `session-request-too-large` | The request is larger than { $mib } MiB. | คำขอมีขนาดเกิน { $mib } MiB |
| `session-undo-data-add` | Add data | เพิ่มข้อมูล |
| `session-undo-data-bind` | Bind data | ผูกข้อมูล |
| `session-undo-data-edit` | Edit data | แก้ไขข้อมูล |
| `session-undo-data-remove` | Remove data | ลบข้อมูล |
| `session-undo-data-rename` | Rename data | เปลี่ยนชื่อข้อมูล |
| `session-undo-data-type` | Change the column type | เปลี่ยนชนิดคอลัมน์ |

### th/ui.ftl

| Key | English | th |
|---|---|---|
| `ui-data-add` | Add data | เพิ่มข้อมูล |
| `ui-data-column-type` | Column type | ชนิดคอลัมน์ |
| `ui-data-next` | Next rows | แถวถัดไป |
| `ui-data-open` | Open the data | เปิดข้อมูล |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | แถว { $first }–{ $last } จาก { $total } |
| `ui-data-previous` | Previous rows | แถวก่อนหน้า |
| `ui-data-remove` | Remove | ลบ |
| `ui-data-rename` | Rename | เปลี่ยนชื่อ |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | ใช้ร่วมกันโดย { $count } มุมมอง |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | คอลัมน์: { $columns } · แถว: { $rows } |
| `ui-data-tab` | Data | ข้อมูล |
| `ui-data-type-bool` | Yes/no | ใช่/ไม่ใช่ |
| `ui-data-type-date` | Date (YYYY-MM-DD) | วันที่ |
| `ui-data-type-integer` | Integer | จำนวนเต็ม |
| `ui-data-type-number` | Number | ตัวเลข |
| `ui-data-type-text` | Text | ข้อความ |
| `ui-data-unused` | Unused | ไม่ได้ใช้ |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count } มุมมอง |

## tr


### tr/chart.ftl

| Key | English | tr |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns`, `data` gerektirir ve 1 ile 64 arasında geçerli, farklı sütun kimliği listeler |
| `chart-ids-duplicate` | a row or column id is repeated | bir satır veya sütun id'si tekrarlanıyor |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | bir satır veya sütun id'si 1 ile 64 arası harf, rakam, - veya _ içerir |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids her satır için bir id, dataset.columns her sütun için bir id gerektirir |
| `chart-no-dataset` | the chart has no data table (dataset) | grafiğin veri tablosu (dataset) yok |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | bir seçenek yaması `dataset` değerini değiştiremez: edit_data kullanın |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | dataset varken veriler dataset içinde kalmalı: edit_data ile düzenleyin |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | bir seçenek hem veri başvurusu hem satır içi veri içeremez: yalnızca birini tutun |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` null olamaz: bunun yerine anahtarı kaldırın |

### tr/core.ftl

| Key | English | tr |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | { $element } öğesi verilerini seçeneklerinde saklıyor: bu sunum dosyası güncel biçimde değil |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | “{ $data }” verisi: temel dosyası eksik, salt okunur tutuldu |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | “{ $data }” verisi: “{ $column }” sütunu { $type } türünde olmayan değerler içeriyor |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | “{ $data }” verisi: değiştirilen bir tabloya yapılan düzenlemeler birleştirmede kayboldu |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | { $element } öğesi: eski bir sürüm grafiğinin veya tablosunun verilerini seçeneklerinde saklamış; bunlar kaldırıldı |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element } sunumda olmayan “{ $data }” verisini adlandırıyor |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | “{ $data }” veri adı zaten kullanılıyor |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | sunuda çok fazla veri var ({ $count }); okunmayanlar: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | “{ $data }” verisi çok büyük, okunurken kırpıldı ve salt okunur tutuldu |
| `issue-more` | more issues not shown: { $count } | gösterilmeyen diğer sorunlar: { $count } |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | { $element } çizilmedi: sunumun görünümleri bir işlemenin izin verdiğinden fazla veri okuyor |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element }, verisinde olmayan “{ $column }” sütununu okuyor |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element } verilerinin yalnızca bir kısmını gösteriyor ({ $what -><br>    [rows] satırlar<br>   *[columns] sütunlar<br>}: { $total } içinden { $shown }) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | { $element } çizilmedi: seçenekleri okunamıyor |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | Toplu işlem çok büyük: hücre sayısı { $count }, en fazla { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | “{ $column }” sütununu { $views } gösteriyor; önce onları başka sütunlara bağlayın (columns ile bind_data) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | “{ $data }” verisinde “{ $column }” sütunu yok |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | Sununun verileri sınırını aşacaktı: en fazla { $max } { $what -><br>    [row-ids] satır kimliği<br>    [overlay] düzenlenmiş hücre<br>    [text] bayt metin<br>   *[data] veri<br>} |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | Bu veriyi { $views }{ $more -><br>    [0] {""}<br>   *[other] {" "}ve { $more } görünüm daha<br>}{" "}gösteriyor; önce bunları kaldırın veya başka bir veriye bağlayın |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | { $subject } öğesi verilerini seçeneklerinde taşıyor; bunları add_data ile sunum verisi olarak ekleyin ve bind_data ile bağlayın |
| `op-data-internal` | A batch cannot set a table's base or overlay | Bir toplu işlem bir tablonun tabanını veya katmanını ayarlayamaz |
| `op-data-missing` | Data “{ $data }” not found | “{ $data }” verisi bulunamadı |
| `op-data-name-taken` | The data name “{ $name }” is already taken | “{ $name }” veri adı zaten kullanılıyor |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | “{ $param }” parametresi veri gösteremez: bunu yalnızca bir grafik veya tablo seçeneği yapabilir |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | { $subject } öğesi zaten başka verileri gösteriyor; değiştirmek için bind_data kullanın |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | “{ $data }” verisi okunurken kırpıldı ve salt okunur; düzenlemek için tablosunun tamamını değiştirin |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | Sunumun geçmişi neredeyse { $max } MiB'a ulaştı, yeniden açılabileceği en büyük boyut: daha fazla eklemeden önce içerik kaldırın. |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | Paketin “{ $data }” verisi harici bir dosyaya başvuruyor; bir paketin verileri kendi hücrelerini içerir. |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | Paketin verileri { $count } hücre içeriyor, en fazla { $max }. |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | Paket { $count } veri içeriyor, en fazla { $max }. |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | Bu sunumun bir veri dosyası bozuk veya kaydedilemedi |

### tr/data.ftl

| Key | English | tr |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | verilerin temel dosyası bozuk veya tablosuyla eşleşmiyor |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | temel dosyası olan bir tablo satırları için yalnızca kimlik verir, hücre vermez |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | bir değer “{ $column }” sütununa uymuyor; bu sütun { $type -><br>    [number] sayılar<br>    [integer] tam sayılar<br>    [bool] doğru veya yanlış<br>    [date] tarihler (YYYY-AA-GG)<br>   *[text] en fazla 16 KB metin<br>} içerir |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | “{ $column }” sütunu kayıpsız olarak { $type -><br>    [number] sayı sütununa<br>    [integer] tam sayı sütununa<br>    [bool] doğru/yanlış sütununa<br>    [date] tarih sütununa<br>   *[text] metin sütununa<br>} dönüştürülemez, bkz. satırlar { $rows }{ $more -><br>    [0] {""}<br>   *[other] {" "}ve { $more } satır daha<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | sütun listesi boş ya da bir sütunu iki kez adlandırıyor |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | “{ $id }” kimliği geçersiz: 1 ile 64 arası harf, rakam, - veya _ kullanın |
| `data-id-missing` | a new row, column or table has no id | yeni bir satırın, sütunun veya tablonun kimliği yok |
| `data-id-taken` | id { $id } is already used in this table | { $id } kimliği bu tabloda zaten kullanılıyor |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | “{ $name }” geçerli bir veri adı değil: küçük harf, rakam ve _ kullanın, rakamla başlamayın |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | bu veriler satırlarını bir temel dosyada tutuyor; bu düzenleme onu henüz okuyamıyor |
| `data-no-column` | column { $column } does not exist | { $column } sütunu yok |
| `data-no-row` | row { $row } does not exist | { $row } satırı yok |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | { $count } sayı sonlu değil (NaN veya sonsuz) |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | { $count } satırda başlıktan fazla hücre vardı; fazla hücreler kaldırıldı |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | bir satırda tablonun { $max } sütunundan daha fazla hücre var |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] veriler { $max } satırlık sınırını aşardı<br>    [columns] veriler { $max } sütunluk sınırını aşardı<br>    [text] bir metin hücresi { $max } baytlık sınırını aşardı<br>   *[overlay] veriler { $max } düzenlenmiş hücrelik sınırını aşardı<br>} |

### tr/document.ftl

| Key | English | tr |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | çizilecek çok fazla değer ({ $count }, en çok { $max }) |

### tr/mcp.ftl

| Key | English | tr |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | “{ $data }” kimliğine sahip veri yok; data_list bunları listeler. |

### tr/session.ftl

| Key | English | tr |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | Uygulamanın veri klasörü destelerin veri dosyalarını tutamıyor |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | { $subject } türü değiştirilemiyor: verileri başka görünümlerde gösteriliyor ve bu türün gerektirdiği sütunlar yok. |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | “{ $data }” verisi kayıtlı satırları okunurken değişti; yeniden deneyin |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | Veri deposu dolu: { $data } sıkıştırılamadı. [sources] max_store_mib altında yer açın. |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | Birleştirme { $data } üzerindeki düzenlemelerin yerini aldı: geri alma geçmişi temizlendi |
| `session-data-user` | { $element } on slide { $slide } | Slayt { $slide } üzerindeki { $element } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | Burada sütun eklenemez veya kaldırılamaz: { $view } görünümü bunları izleyemez |
| `session-example-base-chart-category` | Category | Kategori |
| `session-example-base-chart-value` | Value | Değer |
| `session-request-too-large` | The request is larger than { $mib } MiB. | İstek { $mib } MiB’den büyük. |
| `session-undo-data-add` | Add data | Veri ekle |
| `session-undo-data-bind` | Bind data | Veriyi bağla |
| `session-undo-data-edit` | Edit data | Veriyi düzenle |
| `session-undo-data-remove` | Remove data | Veriyi kaldır |
| `session-undo-data-rename` | Rename data | Veriyi yeniden adlandır |
| `session-undo-data-type` | Change the column type | Sütun türünü değiştir |

### tr/ui.ftl

| Key | English | tr |
|---|---|---|
| `ui-data-add` | Add data | Veri ekle |
| `ui-data-column-type` | Column type | Sütun türü |
| `ui-data-next` | Next rows | Sonraki satırlar |
| `ui-data-open` | Open the data | Verileri aç |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | { $total } satırdan { $first }–{ $last } |
| `ui-data-previous` | Previous rows | Önceki satırlar |
| `ui-data-remove` | Remove | Kaldır |
| `ui-data-rename` | Rename | Yeniden adlandır |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count } görünüm tarafından paylaşılıyor |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | Sütun: { $columns } · Satır: { $rows } |
| `ui-data-tab` | Data | Veriler |
| `ui-data-type-bool` | Yes/no | Evet/hayır |
| `ui-data-type-date` | Date (YYYY-MM-DD) | Tarih |
| `ui-data-type-integer` | Integer | Tam sayı |
| `ui-data-type-number` | Number | Sayı |
| `ui-data-type-text` | Text | Metin |
| `ui-data-unused` | Unused | Kullanılmıyor |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count } görünüm |

## uk


### uk/chart.ftl

| Key | English | uk |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns` потребує `data` і перелічує від 1 до 64 дійсних різних ідентифікаторів стовпців |
| `chart-ids-duplicate` | a row or column id is repeated | id рядка або стовпця повторюється |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | id рядка або стовпця містить від 1 до 64 літер, цифр, - або _ |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids потребує один id на рядок, а dataset.columns — один на стовпець |
| `chart-no-dataset` | the chart has no data table (dataset) | у діаграми немає таблиці даних (dataset) |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | зміна параметрів не може змінювати `dataset`: використовуйте edit_data |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | за наявності dataset дані мають залишатися в dataset: змінюйте їх через edit_data |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | параметр не може містити водночас посилання на дані та вбудовані дані: залиште одне |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` не може бути null: натомість видаліть ключ |

### uk/core.ftl

| Key | English | uk |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | Елемент { $element } зберігає дані у своїх параметрах: цей файл презентації не в поточному форматі |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | дані «{ $data }»: бракує базового файлу, залишено лише для читання |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | дані «{ $data }»: стовпець «{ $column }» містить значення не типу { $type } |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | дані «{ $data }»: зміни заміненої таблиці втрачено під час злиття |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | Елемент { $element }: стара версія зберегла дані його діаграми або таблиці в його параметрах; їх вилучено |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element } посилається на дані «{ $data }», яких немає в презентації |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | назва даних «{ $data }» уже зайнята |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | забагато даних у презентації ({ $count }); не прочитано: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | дані «{ $data }» завеликі, обрізано під час читання й залишено лише для читання |
| `issue-more` | more issues not shown: { $count } | інші непоказані проблеми: { $count } |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | { $element } не намальовано: подання презентації читають більше даних, ніж дозволяє рендеринг |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element } читає стовпець «{ $column }», якого немає в його даних |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element } показує лише частину своїх даних ({ $what -><br>    [rows] рядки<br>   *[columns] стовпці<br>}: { $shown } з { $total }) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | { $element } не намальовано: його параметри не читаються |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | Пакет завеликий: кількість клітинок { $count }, не більше { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | Стовпець «{ $column }» показують { $views }; спершу прив’яжіть їх до інших стовпців (bind_data з columns) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | У даних «{ $data }» немає стовпця «{ $column }» |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | Дані презентації перевищили б ліміт: щонайбільше { $max } { $what -><br>    [row-ids] ідентифікаторів рядків<br>    [overlay] змінених клітинок<br>    [text] байтів тексту<br>   *[data] даних<br>} |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | Ці дані показують { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}і ще { $more } подання<br>    [few] {" "}і ще { $more } подання<br>    [many] {" "}і ще { $more } подань<br>   *[other] {" "}і ще { $more } подання<br>}; спочатку видаліть їх або прив’яжіть до інших даних |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | Елемент { $subject } містить дані у своїх параметрах; додайте їх як дані презентації через add_data і прив’яжіть через bind_data |
| `op-data-internal` | A batch cannot set a table's base or overlay | Пакет не може задавати основу чи накладку таблиці |
| `op-data-missing` | Data “{ $data }” not found | Дані «{ $data }» не знайдено |
| `op-data-name-taken` | The data name “{ $name }” is already taken | Назва даних «{ $name }» уже зайнята |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | Параметр «{ $param }» не може показувати дані: це вміє лише параметр діаграми чи таблиці |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | Елемент { $subject } уже показує інші дані; щоб змінити їх, використайте bind_data |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | Дані «{ $data }» обрізано під час читання, вони лише для читання; щоб змінити їх, замініть усю таблицю |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | Історія презентації майже досягла { $max } МіБ — максимуму, за якого її можна знову відкрити: видаліть вміст, перш ніж додавати новий. |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | Дані «{ $data }» пакета посилаються на зовнішній файл; дані пакета містять власні клітинки. |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | Дані пакета містять { $count -><br>    [one] { $count } клітинку<br>    [few] { $count } клітинки<br>   *[many] { $count } клітинок<br>}, не більше { $max }. |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | Пакет містить { $count -><br>    [one] { $count } набір даних<br>    [few] { $count } набори даних<br>   *[many] { $count } наборів даних<br>}, не більше { $max }. |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | Файл даних цієї презентації пошкоджено або його не вдалося зберегти |

### uk/data.ftl

| Key | English | uk |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | базовий файл даних пошкоджено або він не відповідає їхній таблиці |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | таблиця з базовим файлом задає для рядків лише ідентифікатори, без клітинок |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | значення не підходить для стовпця «{ $column }», у якому { $type -><br>    [number] числа<br>    [integer] цілі числа<br>    [bool] істина або хиба<br>    [date] дати (РРРР-ММ-ДД)<br>   *[text] текст не довший за 16 КБ<br>} |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | стовпець «{ $column }» не можна без втрат зробити { $type -><br>    [number] числовим<br>    [integer] цілочисельним<br>    [bool] логічним<br>    [date] стовпцем дат<br>   *[text] текстовим<br>}, див. рядки { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}і ще { $more } рядок<br>    [few] {" "}і ще { $more } рядки<br>   *[many] {" "}і ще { $more } рядків<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | список стовпців порожній або називає стовпець двічі |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | ідентифікатор «{ $id }» недійсний: використовуйте від 1 до 64 латинських літер, цифр, - або _ |
| `data-id-missing` | a new row, column or table has no id | новий рядок, стовпець або таблиця не має ідентифікатора |
| `data-id-taken` | id { $id } is already used in this table | ідентифікатор { $id } уже використовується в цій таблиці |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | «{ $name }» — недійсна назва даних: використовуйте малі латинські літери, цифри та _, не починаючи з цифри |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | ці дані зберігають рядки в базовому файлі, який ця зміна поки не вміє читати |
| `data-no-column` | column { $column } does not exist | стовпця { $column } не існує |
| `data-no-row` | row { $row } does not exist | рядка { $row } не існує |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | { $count -><br>    [one] { $count } число не є скінченним (NaN або нескінченність)<br>    [few] { $count } числа не є скінченними (NaN або нескінченність)<br>   *[many] { $count } чисел не є скінченними (NaN або нескінченність)<br>} |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | { $count -><br>    [one] { $count } рядок мав більше клітинок, ніж заголовок; зайві клітинки вилучено<br>    [few] { $count } рядки мали більше клітинок, ніж заголовок; зайві клітинки вилучено<br>   *[many] { $count } рядків мали більше клітинок, ніж заголовок; зайві клітинки вилучено<br>} |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | у рядку більше клітинок, ніж { $max } { $max -><br>    [one] стовпець<br>    [few] стовпці<br>   *[many] стовпців<br>} таблиці |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] дані перевищили б межу рядків — { $max } { $max -><br>        [one] рядок<br>        [few] рядки<br>       *[many] рядків<br>    }<br>    [columns] дані перевищили б межу стовпців — { $max } { $max -><br>        [one] стовпець<br>        [few] стовпці<br>       *[many] стовпців<br>    }<br>    [text] текстова клітинка перевищила б межу — { $max } { $max -><br>        [one] байт<br>        [few] байти<br>       *[many] байтів<br>    }<br>   *[overlay] дані перевищили б межу змінених клітинок — { $max } { $max -><br>        [one] клітинка<br>        [few] клітинки<br>       *[many] клітинок<br>    }<br>} |

### uk/document.ftl

| Key | English | uk |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | забагато значень для малювання ({ $count }, щонайбільше { $max }) |

### uk/mcp.ftl

| Key | English | uk |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | Немає даних з ідентифікатором «{ $data }»; їх перелічує data_list. |

### uk/session.ftl

| Key | English | uk |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | Тека даних застосунку не може зберігати файли даних презентацій |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | Не вдається змінити тип { $subject }: його дані показано в інших поданнях, і в них немає стовпців, потрібних цьому типу. |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | Дані «{ $data }» змінилися під час читання збережених рядків; спробуйте ще раз |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | Сховище даних заповнене: не вдалося стиснути { $data }. Звільніть місце в [sources] max_store_mib. |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | Злиття замінило зміни в { $data }: історію скасування очищено |
| `session-data-user` | { $element } on slide { $slide } | { $element } на слайді { $slide } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | Тут не можна додати чи видалити стовпці: подання { $view } не може за ними стежити |
| `session-example-base-chart-category` | Category | Категорія |
| `session-example-base-chart-value` | Value | Значення |
| `session-request-too-large` | The request is larger than { $mib } MiB. | Запит більший за { $mib } МіБ. |
| `session-undo-data-add` | Add data | Додати дані |
| `session-undo-data-bind` | Bind data | Прив’язати дані |
| `session-undo-data-edit` | Edit data | Змінити дані |
| `session-undo-data-remove` | Remove data | Видалити дані |
| `session-undo-data-rename` | Rename data | Перейменувати дані |
| `session-undo-data-type` | Change the column type | Змінити тип стовпця |

### uk/ui.ftl

| Key | English | uk |
|---|---|---|
| `ui-data-add` | Add data | Додати дані |
| `ui-data-column-type` | Column type | Тип стовпця |
| `ui-data-next` | Next rows | Наступні рядки |
| `ui-data-open` | Open the data | Відкрити дані |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | Рядки { $first }–{ $last } з { $total } |
| `ui-data-previous` | Previous rows | Попередні рядки |
| `ui-data-remove` | Remove | Видалити |
| `ui-data-rename` | Rename | Перейменувати |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | Спільні дані · { $count -><br>    [one] { $count } подання<br>    [few] { $count } подання<br>    [many] { $count } подань<br>   *[other] { $count } подання<br>} |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | Стовпці: { $columns } · Рядки: { $rows } |
| `ui-data-tab` | Data | Дані |
| `ui-data-type-bool` | Yes/no | Так/ні |
| `ui-data-type-date` | Date (YYYY-MM-DD) | Дата |
| `ui-data-type-integer` | Integer | Ціле число |
| `ui-data-type-number` | Number | Число |
| `ui-data-type-text` | Text | Текст |
| `ui-data-unused` | Unused | Не використовуються |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count -><br>    [one] { $count } подання<br>    [few] { $count } подання<br>    [many] { $count } подань<br>   *[other] { $count } подання<br>} |

## vi


### vi/chart.ftl

| Key | English | vi |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns` cần `data` và liệt kê từ 1 đến 64 mã cột hợp lệ, khác nhau |
| `chart-ids-duplicate` | a row or column id is repeated | một id hàng hoặc cột bị lặp lại |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | một id hàng hoặc cột gồm 1 đến 64 chữ cái, chữ số, - hoặc _ |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids cần một id cho mỗi hàng và dataset.columns một id cho mỗi cột |
| `chart-no-dataset` | the chart has no data table (dataset) | biểu đồ không có bảng dữ liệu (dataset) |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | bản vá tùy chọn không thể thay đổi `dataset`: hãy dùng edit_data |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | khi có dataset, dữ liệu phải nằm trong dataset: hãy sửa bằng edit_data |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | một tùy chọn không thể vừa có tham chiếu dữ liệu vừa có dữ liệu nội tuyến: chỉ giữ một |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` không thể là null: hãy xóa khóa thay vào đó |

### vi/core.ftl

| Key | English | vi |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | Phần tử { $element } lưu dữ liệu trong tùy chọn của nó: tệp deck này không ở định dạng hiện tại |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | dữ liệu “{ $data }”: thiếu tệp gốc, giữ ở chế độ chỉ đọc |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | dữ liệu “{ $data }”: cột “{ $column }” chứa giá trị không thuộc kiểu { $type } |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | dữ liệu “{ $data }”: các chỉnh sửa trên một bảng đã bị thay thế bị mất khi hợp nhất |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | Phần tử { $element }: một phiên bản cũ đã lưu dữ liệu biểu đồ hoặc bảng trong tùy chọn của nó; dữ liệu đó đã bị xóa |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element } nêu dữ liệu “{ $data }”, không có trong bản trình bày |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | tên dữ liệu “{ $data }” đã được dùng |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | quá nhiều dữ liệu trong bản trình bày ({ $count }); chưa đọc: { $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | dữ liệu “{ $data }” quá lớn, đã cắt bớt khi đọc và giữ ở chế độ chỉ đọc |
| `issue-more` | more issues not shown: { $count } | các vấn đề khác không hiển thị: { $count } |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | { $element } không được vẽ: các chế độ xem của bản trình bày đọc nhiều dữ liệu hơn mức một lần kết xuất cho phép |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element } đọc cột “{ $column }”, không có trong dữ liệu của nó |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element } chỉ hiển thị một phần dữ liệu ({ $what -><br>    [rows] hàng<br>   *[columns] cột<br>}: { $shown } trên { $total }) |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | { $element } không được vẽ: không đọc được các tùy chọn của nó |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | Lô quá lớn: số ô { $count }, tối đa { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | Cột “{ $column }” đang được { $views } hiển thị; hãy liên kết chúng với các cột khác trước (bind_data với columns) |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | Dữ liệu “{ $data }” không có cột “{ $column }” |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | Dữ liệu của bản trình bày sẽ vượt giới hạn: tối đa { $max } { $what -><br>    [row-ids] mã dòng<br>    [overlay] ô đã sửa<br>    [text] byte văn bản<br>   *[data] dữ liệu<br>} |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | Dữ liệu này được hiển thị bởi { $views }{ $more -><br>    [0] {""}<br>   *[other] {" "}và { $more } chế độ xem khác<br>}; hãy xóa chúng hoặc liên kết chúng với dữ liệu khác trước |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | Phần tử { $subject } mang dữ liệu trong tùy chọn của nó; hãy thêm chúng làm dữ liệu của deck bằng add_data và liên kết bằng bind_data |
| `op-data-internal` | A batch cannot set a table's base or overlay | Một lô không thể đặt phần nền hay lớp phủ của bảng |
| `op-data-missing` | Data “{ $data }” not found | Không tìm thấy dữ liệu “{ $data }” |
| `op-data-name-taken` | The data name “{ $name }” is already taken | Tên dữ liệu “{ $name }” đã được dùng |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | Tham số “{ $param }” không thể hiển thị dữ liệu: chỉ tùy chọn biểu đồ hoặc bảng mới làm được |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | Phần tử { $subject } đã hiển thị dữ liệu khác; dùng bind_data để thay đổi |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | Dữ liệu “{ $data }” đã bị cắt khi đọc và chỉ đọc; hãy thay toàn bộ bảng để chỉnh sửa |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | Lịch sử của bản trình bày gần đạt { $max } MiB, mức tối đa để mở lại: hãy xóa bớt nội dung trước khi thêm. |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | Dữ liệu “{ $data }” của gói tham chiếu đến một tệp bên ngoài; dữ liệu của gói phải chứa ô của riêng nó. |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | Dữ liệu của gói chứa { $count } ô, tối đa { $max }. |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | Gói chứa { $count } dữ liệu, tối đa { $max }. |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | Một tệp dữ liệu của bản trình bày này bị hỏng hoặc không thể lưu |

### vi/data.ftl

| Key | English | vi |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | tệp cơ sở của dữ liệu bị hỏng hoặc không khớp với bảng của nó |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | bảng có tệp nền chỉ ghi mã của các hàng, không ghi ô |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | một giá trị không hợp với cột “{ $column }”, cột này chứa { $type -><br>    [number] số<br>    [integer] số nguyên<br>    [bool] đúng hoặc sai<br>    [date] ngày (YYYY-MM-DD)<br>   *[text] văn bản tối đa 16 KB<br>} |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | không thể đổi cột “{ $column }” thành { $type -><br>    [number] cột số<br>    [integer] cột số nguyên<br>    [bool] cột đúng/sai<br>    [date] cột ngày<br>   *[text] cột văn bản<br>} mà không mất dữ liệu, xem các hàng { $rows }{ $more -><br>    [0] {""}<br>   *[other] {" "}và { $more } hàng khác<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | danh sách cột trống hoặc nêu một cột hai lần |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | mã “{ $id }” không hợp lệ: dùng từ 1 đến 64 chữ cái, chữ số, - hoặc _ |
| `data-id-missing` | a new row, column or table has no id | một hàng, cột hoặc bảng mới chưa có mã |
| `data-id-taken` | id { $id } is already used in this table | mã { $id } đã được dùng trong bảng này |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | “{ $name }” không phải tên dữ liệu hợp lệ: dùng chữ thường, chữ số và _, không bắt đầu bằng chữ số |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | dữ liệu này lưu các hàng trong một tệp nền mà thao tác sửa này chưa đọc được |
| `data-no-column` | column { $column } does not exist | cột { $column } không tồn tại |
| `data-no-row` | row { $row } does not exist | hàng { $row } không tồn tại |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | { $count } số không hữu hạn (NaN hoặc vô cực) |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | { $count } hàng có nhiều ô hơn tiêu đề; các ô thừa đã bị xóa |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | một hàng có nhiều ô hơn { $max } cột của bảng |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] dữ liệu sẽ vượt giới hạn { $max } hàng<br>    [columns] dữ liệu sẽ vượt giới hạn { $max } cột<br>    [text] một ô văn bản sẽ vượt giới hạn { $max } byte<br>   *[overlay] dữ liệu sẽ vượt giới hạn { $max } ô đã sửa<br>} |

### vi/document.ftl

| Key | English | vi |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | quá nhiều giá trị để vẽ ({ $count }, tối đa { $max }) |

### vi/mcp.ftl

| Key | English | vi |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | Không có dữ liệu nào có id “{ $data }”; data_list liệt kê chúng. |

### vi/session.ftl

| Key | English | vi |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | Thư mục dữ liệu của ứng dụng không thể chứa các tệp dữ liệu của bản trình chiếu |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | Không thể đổi loại của { $subject }: các chế độ xem khác hiển thị dữ liệu của nó, và dữ liệu thiếu các cột loại này cần. |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | Dữ liệu “{ $data }” đã thay đổi trong khi đọc các hàng đã lưu; hãy thử lại |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | Kho dữ liệu đã đầy: không thể nén { $data }. Hãy giải phóng dung lượng trong [sources] max_store_mib. |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | Việc hợp nhất đã thay thế các chỉnh sửa trong { $data }: lịch sử hoàn tác đã bị xóa |
| `session-data-user` | { $element } on slide { $slide } | { $element } trên trang chiếu { $slide } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | Không thể thêm hoặc xóa cột ở đây: chế độ xem { $view } không thể theo kịp |
| `session-example-base-chart-category` | Category | Danh mục |
| `session-example-base-chart-value` | Value | Giá trị |
| `session-request-too-large` | The request is larger than { $mib } MiB. | Yêu cầu lớn hơn { $mib } MiB. |
| `session-undo-data-add` | Add data | Thêm dữ liệu |
| `session-undo-data-bind` | Bind data | Liên kết dữ liệu |
| `session-undo-data-edit` | Edit data | Sửa dữ liệu |
| `session-undo-data-remove` | Remove data | Xóa dữ liệu |
| `session-undo-data-rename` | Rename data | Đổi tên dữ liệu |
| `session-undo-data-type` | Change the column type | Đổi kiểu cột |

### vi/ui.ftl

| Key | English | vi |
|---|---|---|
| `ui-data-add` | Add data | Thêm dữ liệu |
| `ui-data-column-type` | Column type | Kiểu cột |
| `ui-data-next` | Next rows | Các hàng sau |
| `ui-data-open` | Open the data | Mở dữ liệu |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | Hàng { $first }–{ $last } trên { $total } |
| `ui-data-previous` | Previous rows | Các hàng trước |
| `ui-data-remove` | Remove | Xóa |
| `ui-data-rename` | Rename | Đổi tên |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | Được chia sẻ bởi { $count } chế độ xem |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | Cột: { $columns } · Hàng: { $rows } |
| `ui-data-tab` | Data | Dữ liệu |
| `ui-data-type-bool` | Yes/no | Có/không |
| `ui-data-type-date` | Date (YYYY-MM-DD) | Ngày |
| `ui-data-type-integer` | Integer | Số nguyên |
| `ui-data-type-number` | Number | Số |
| `ui-data-type-text` | Text | Văn bản |
| `ui-data-unused` | Unused | Chưa dùng |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count } chế độ xem |

## zh-Hans


### zh-Hans/chart.ftl

| Key | English | zh-Hans |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns` 需要 `data`，并列出 1 到 64个有效且互不相同的列标识 |
| `chart-ids-duplicate` | a row or column id is repeated | 行或列的 id 重复 |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | 行或列的 id 由 1 到 64个字母、数字、- 或 _ 组成 |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids 需要每行一个 id，dataset.columns 需要每列一个 |
| `chart-no-dataset` | the chart has no data table (dataset) | 图表没有数据表 (dataset) |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | 选项补丁不能修改 `dataset`：请使用 edit_data |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | 使用 dataset 时，数据必须保留在 dataset 中：请用 edit_data 编辑 |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | 选项不能同时包含数据引用和内联数据：只保留其一 |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` 不能为 null：请改为删除该键 |

### zh-Hans/core.ftl

| Key | English | zh-Hans |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | 元素 { $element } 把数据存放在选项中：此演示文稿文件不是当前格式 |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | 数据“{ $data }”：缺少其基础文件，已设为只读 |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | 数据“{ $data }”：列“{ $column }”包含不属于 { $type } 类型的值 |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | 数据“{ $data }”：对已替换表格的编辑在合并时丢失 |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | 元素 { $element }：旧版本把其图表或表格的数据存放在选项中；这些数据已被移除 |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element }引用的数据“{ $data }”不在演示文稿中 |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | 数据名称“{ $data }”已被使用 |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | 演示文稿中的数据过多（{ $count }）；未读取：{ $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | 数据“{ $data }”过大，读取时已截断并设为只读 |
| `issue-more` | more issues not shown: { $count } | 另有{ $count }个问题未显示 |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | 未绘制{ $element }：演示文稿的视图读取的数据超过一次渲染允许的量 |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element }读取的列“{ $column }”不在其数据中 |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element }只显示部分数据（{ $what -><br>    [rows] 行<br>   *[columns] 列<br>}：{ $total }中的{ $shown }） |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | 未绘制{ $element }：无法读取其选项 |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | 批处理过大：单元格数 { $count }，最多 { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | 列“{ $column }”正由 { $views } 显示；请先将它们绑定到其他列（使用 columns 的 bind_data） |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | 数据“{ $data }”没有列“{ $column }” |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | 演示文稿的数据将超出上限：{ $what -><br>    [row-ids] 行 ID<br>    [overlay] 已编辑单元格<br>    [text] 文本字节<br>   *[data] 数据<br>}最多 { $max } |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | 此数据由 { $views }{ $more -><br>    [0] {""}<br>   *[other] {" "}等另外 { $more }个视图<br>}显示；请先删除它们或将其绑定到其他数据 |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | 元素 { $subject } 的数据放在其选项中；请用 add_data 将其添加为演示文稿数据，并用 bind_data 绑定 |
| `op-data-internal` | A batch cannot set a table's base or overlay | 批处理不能设置表格的基础或覆盖层 |
| `op-data-missing` | Data “{ $data }” not found | 未找到数据“{ $data }” |
| `op-data-name-taken` | The data name “{ $name }” is already taken | 数据名称“{ $name }”已被使用 |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | 参数“{ $param }”无法显示数据：只有图表或表格选项可以 |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | 元素 { $subject } 已显示其他数据；如需更改，请使用 bind_data |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | 数据“{ $data }”在读取时被截断，为只读；如需编辑，请替换其整个表格 |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | 演示文稿的历史接近 { $max } MiB，即可重新打开的上限：请先删除内容再添加。 |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | 包中的数据“{ $data }”引用了外部文件；包的数据应包含自己的单元格。 |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | 包的数据包含 { $count }个单元格，最多 { $max }个。 |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | 该包包含 { $count }个数据，最多 { $max }个。 |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | 此演示文稿的一个数据文件已损坏或无法存储 |

### zh-Hans/data.ftl

| Key | English | zh-Hans |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | 数据的基础文件已损坏或与其表格不符 |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | 带有基础文件的表格只给出行的 ID，不给出单元格 |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | 有一个值不适合列“{ $column }”（该列包含{ $type -><br>    [number] 数字<br>    [integer] 整数<br>    [bool] 真或假<br>    [date] 日期，YYYY-MM-DD<br>   *[text] 最多16KB的文本<br>}） |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | 列“{ $column }”无法无损地改为{ $type -><br>    [number] 数字列<br>    [integer] 整数列<br>    [bool] 真假列<br>    [date] 日期列<br>   *[text] 文本列<br>}，请查看以下行：{ $rows }{ $more -><br>    [0] {""}<br>   *[other] {""}，另有{ $more }行<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | 列列表为空或重复指定了某一列 |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | ID“{ $id }”无效：请使用1到64个字母、数字、- 或 _ |
| `data-id-missing` | a new row, column or table has no id | 新的行、列或表没有 ID |
| `data-id-taken` | id { $id } is already used in this table | ID { $id } 已在此表中使用 |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | “{ $name }”不是有效的数据名称：请使用小写字母、数字和 _，且不以数字开头 |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | 此数据将行保存在基础文件中，此编辑暂时无法读取 |
| `data-no-column` | column { $column } does not exist | 列 { $column } 不存在 |
| `data-no-row` | row { $row } does not exist | 行 { $row } 不存在 |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | 有{ $count }个数字不是有限值（NaN 或无穷大） |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | 有{ $count }行的单元格多于表头，多余的单元格已删除 |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | 某行的单元格多于表格的{ $max }列 |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] 数据将超过行数上限（{ $max }行）<br>    [columns] 数据将超过列数上限（{ $max }列）<br>    [text] 文本单元格将超过上限（{ $max }字节）<br>   *[overlay] 数据将超过已编辑单元格上限（{ $max }个）<br>} |

### zh-Hans/document.ftl

| Key | English | zh-Hans |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | 要绘制的值过多（{ $count }，最多{ $max }） |

### zh-Hans/mcp.ftl

| Key | English | zh-Hans |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | 没有 id 为“{ $data }”的数据；data_list 会列出它们。 |

### zh-Hans/session.ftl

| Key | English | zh-Hans |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | 应用的数据文件夹无法存放演示文稿的数据文件 |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | 无法更改 { $subject } 的类型：其他视图也显示这些数据，而数据缺少此类型需要的列。 |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | 读取已存储的行时数据“{ $data }”已更改；请重试 |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | 数据存储已满：无法压缩 { $data }。请在 [sources] max_store_mib 下释放空间。 |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | 合并替换了对 { $data } 的编辑：撤销历史已清除 |
| `session-data-user` | { $element } on slide { $slide } | 幻灯片 { $slide } 上的 { $element } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | 此处无法添加或删除列：视图 { $view } 无法跟随 |
| `session-example-base-chart-category` | Category | 类别 |
| `session-example-base-chart-value` | Value | 值 |
| `session-request-too-large` | The request is larger than { $mib } MiB. | 请求超过 { $mib } MiB。 |
| `session-undo-data-add` | Add data | 添加数据 |
| `session-undo-data-bind` | Bind data | 绑定数据 |
| `session-undo-data-edit` | Edit data | 编辑数据 |
| `session-undo-data-remove` | Remove data | 删除数据 |
| `session-undo-data-rename` | Rename data | 重命名数据 |
| `session-undo-data-type` | Change the column type | 更改列类型 |

### zh-Hans/ui.ftl

| Key | English | zh-Hans |
|---|---|---|
| `ui-data-add` | Add data | 添加数据 |
| `ui-data-column-type` | Column type | 列类型 |
| `ui-data-next` | Next rows | 下一页 |
| `ui-data-open` | Open the data | 打开数据 |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | 第 { $first }–{ $last }行，共 { $total }行 |
| `ui-data-previous` | Previous rows | 上一页 |
| `ui-data-remove` | Remove | 删除 |
| `ui-data-rename` | Rename | 重命名 |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | 由 { $count }个视图共享 |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | 列：{ $columns } · 行：{ $rows } |
| `ui-data-tab` | Data | 数据 |
| `ui-data-type-bool` | Yes/no | 是/否 |
| `ui-data-type-date` | Date (YYYY-MM-DD) | 日期 |
| `ui-data-type-integer` | Integer | 整数 |
| `ui-data-type-number` | Number | 数字 |
| `ui-data-type-text` | Text | 文本 |
| `ui-data-unused` | Unused | 未使用 |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count }个视图 |

## zh-Hant


### zh-Hant/chart.ftl

| Key | English | zh-Hant |
|---|---|---|
| `chart-columns-invalid` | `columns` needs `data` and lists 1 to 64 valid, distinct column ids | `columns` 需要 `data`，並列出 1 到 64個有效且互不相同的欄識別碼 |
| `chart-ids-duplicate` | a row or column id is repeated | 列或欄的 id 重複 |
| `chart-ids-invalid` | a row or column id has 1 to 64 letters, digits, - or _ | 列或欄的 id 由 1 到 64個字母、數字、- 或 _ 組成 |
| `chart-ids-length` | dataset.ids needs one id per row and dataset.columns one per column | dataset.ids 需要每列一個 id，dataset.columns 需要每欄一個 |
| `chart-no-dataset` | the chart has no data table (dataset) | 圖表沒有資料表 (dataset) |
| `chart-patch-dataset` | an option patch cannot change `dataset`: use edit_data | 選項修補不能修改 `dataset`：請使用 edit_data |
| `chart-patch-inline-data` | with a dataset, data must stay in the dataset: edit it with edit_data | 使用 dataset 時，資料必須保留在 dataset 中：請用 edit_data 編輯 |
| `chart-ref-and-inline` | an option cannot hold a data reference and inline data: keep one of them | 選項不能同時包含資料參照與內嵌資料：只保留其一 |
| `chart-ref-null` | `data` cannot be null: remove the key instead | `data` 不能為 null：請改為刪除該鍵 |

### zh-Hant/core.ftl

| Key | English | zh-Hant |
|---|---|---|
| `deck-inline-data` | Element { $element } stores its data inside its options: this deck file is not in the current format | 元素 { $element } 把資料存放在選項中：此簡報檔案不是目前格式 |
| `issue-data-base-missing` | data “{ $data }”: its base file is missing, kept read-only | 資料「{ $data }」：缺少其基礎檔案，已設為唯讀 |
| `issue-data-cell-type` | data “{ $data }”: column “{ $column }” holds values that are not { $type } | 資料「{ $data }」：欄「{ $column }」包含不屬於 { $type } 類型的值 |
| `issue-data-edits-lost` | data “{ $data }”: edits to a replaced table were lost in the merge | 資料「{ $data }」：對已取代表格的編輯在合併時遺失 |
| `issue-data-inline-dropped` | Element { $element }: its chart or table data were stored inside its options by an older version; they were removed | 元素 { $element }：舊版本把其圖表或表格的資料存放在選項中；這些資料已被移除 |
| `issue-data-missing` | { $element } names the data “{ $data }”, which is not in the deck | { $element }引用的資料「{ $data }」不在簡報中 |
| `issue-data-name-taken` | the data name “{ $data }” is already taken | 資料名稱「{ $data }」已被使用 |
| `issue-data-too-many` | too many data in the deck ({ $count }); not read: { $dropped } | 簡報中的資料過多（{ $count }）；未讀取：{ $dropped } |
| `issue-data-truncated` | data “{ $data }” too large, truncated on read and kept read-only | 資料「{ $data }」過大，讀取時已截斷並設為唯讀 |
| `issue-more` | more issues not shown: { $count } | 另有{ $count }個問題未顯示 |
| `issue-render-budget` | { $element } is not drawn: the deck's views read more data than a render allows | 未繪製{ $element }：簡報的檢視讀取的資料超過一次算繪允許的量 |
| `issue-view-column-missing` | { $element } reads the column “{ $column }”, which its data does not have | { $element }讀取的欄「{ $column }」不在其資料中 |
| `issue-view-truncated` | { $element } shows only part of its data ({ $what -><br>    [rows] rows<br>   *[columns] columns<br>}: { $shown } of { $total }) | { $element }只顯示部分資料（{ $what -><br>    [rows] 列<br>   *[columns] 欄<br>}：{ $total }中的{ $shown }） |
| `issue-view-unreadable` | { $element } is not drawn: its options cannot be read | 未繪製{ $element }：無法讀取其選項 |
| `op-data-batch-too-large` | The batch is too large: cell count { $count }, at most { $max } | 批次過大：儲存格數 { $count }，最多 { $max } |
| `op-data-column-in-use` | Column “{ $column }” is shown by { $views }; bind them to other columns first (bind_data with columns) | 欄「{ $column }」正由 { $views } 顯示；請先將它們繫結到其他欄（使用 columns 的 bind_data） |
| `op-data-column-missing` | Data “{ $data }” has no column “{ $column }” | 資料「{ $data }」沒有欄「{ $column }」 |
| `op-data-deck-full` | The deck's data would pass their limit: at most { $max } { $what -><br>    [row-ids] row ids<br>    [overlay] edited cells<br>    [text] bytes of text<br>   *[data] data<br>} | 簡報的資料將超出上限：{ $what -><br>    [row-ids] 列 ID<br>    [overlay] 已編輯儲存格<br>    [text] 文字位元組<br>   *[data] 資料<br>}最多 { $max } |
| `op-data-in-use` | This data is shown by { $views }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more view<br>   *[other] {" "}and { $more } more views<br>}; remove them or bind them to another data first | 此資料由 { $views }{ $more -><br>    [0] {""}<br>   *[other] {" "}等另外 { $more }個檢視<br>}顯示；請先刪除它們或將其繫結到其他資料 |
| `op-data-inline` | Element { $subject } carries its data inside its options; add them as deck data with add_data and bind them with bind_data | 元素 { $subject } 的資料放在其選項中；請用 add_data 將其新增為簡報資料，並用 bind_data 繫結 |
| `op-data-internal` | A batch cannot set a table's base or overlay | 批次不能設定表格的基礎或覆蓋層 |
| `op-data-missing` | Data “{ $data }” not found | 找不到資料「{ $data }」 |
| `op-data-name-taken` | The data name “{ $name }” is already taken | 資料名稱「{ $name }」已被使用 |
| `op-data-not-view` | Parameter “{ $param }” cannot show data: only a chart or table option can | 參數「{ $param }」無法顯示資料：只有圖表或表格選項可以 |
| `op-data-rebind` | Element { $subject } already shows other data; use bind_data to change them | 元素 { $subject } 已顯示其他資料；如需變更，請使用 bind_data |
| `op-data-truncated` | Data “{ $data }” was truncated on read and is read-only; replace its whole table to edit it | 資料「{ $data }」在讀取時被截斷，為唯讀；如需編輯，請取代其整個表格 |
| `op-replica-full` | The deck's history is nearly { $max } MiB, the most it can reopen: remove content before adding more. | 簡報的歷史接近 { $max } MiB，即可重新開啟的上限：請先移除內容再新增。 |
| `pack-manifest-data-base` | The pack's data “{ $data }” refers to an external file; a pack's data hold their own cells. | 套件中的資料「{ $data }」參照了外部檔案；套件的資料應包含自己的儲存格。 |
| `pack-manifest-data-cells` | The pack's data hold { $count -><br>    [one] { $count } cell<br>   *[other] { $count } cells<br>}, at most { $max }. | 套件的資料包含 { $count }個儲存格，最多 { $max }個。 |
| `pack-manifest-data-count` | The pack holds { $count -><br>    [one] { $count } data<br>   *[other] { $count } data<br>}, at most { $max }. | 此套件包含 { $count }個資料，最多 { $max }個。 |
| `sources-base-refused` | A data file of this deck is damaged or could not be stored | 此簡報的一個資料檔已損毀或無法儲存 |

### zh-Hant/data.ftl

| Key | English | zh-Hant |
|---|---|---|
| `data-base-invalid` | the data's base file is damaged or does not match its table | 資料的基礎檔案已損毀或與其表格不符 |
| `data-base-row-cells` | a table with a base file gives its rows ids only, not cells | 具有基礎檔案的表格只提供列的 ID，不提供儲存格 |
| `data-cell-type` | a value does not fit column “{ $column }”, which holds { $type -><br>    [number] numbers<br>    [integer] whole numbers<br>    [bool] true or false<br>    [date] dates (YYYY-MM-DD)<br>   *[text] text of at most 16 KB<br>} | 有一個值不適合欄「{ $column }」（該欄包含{ $type -><br>    [number] 數字<br>    [integer] 整數<br>    [bool] 真或假<br>    [date] 日期，YYYY-MM-DD<br>   *[text] 最多16KB的文字<br>}） |
| `data-column-type` | column “{ $column }” cannot become { $type -><br>    [number] a number column<br>    [integer] a whole-number column<br>    [bool] a true-or-false column<br>    [date] a date column<br>   *[text] a text column<br>} without loss, see rows { $rows }{ $more -><br>    [0] {""}<br>    [one] {" "}and { $more } more row<br>   *[other] {" "}and { $more } more rows<br>} | 欄「{ $column }」無法無損地改為{ $type -><br>    [number] 數字欄<br>    [integer] 整數欄<br>    [bool] 真假欄<br>    [date] 日期欄<br>   *[text] 文字欄<br>}，請查看以下列：{ $rows }{ $more -><br>    [0] {""}<br>   *[other] {""}，另有{ $more }列<br>} |
| `data-columns-invalid` | the column list is empty or names a column twice | 欄清單為空或重複指定了某一欄 |
| `data-id-invalid` | id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _ | ID「{ $id }」無效：請使用1到64個字母、數字、- 或 _ |
| `data-id-missing` | a new row, column or table has no id | 新的列、欄或表格沒有 ID |
| `data-id-taken` | id { $id } is already used in this table | ID { $id } 已在此表格中使用 |
| `data-invalid-name` | “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit | 「{ $name }」不是有效的資料名稱：請使用小寫字母、數字和 _，且不以數字開頭 |
| `data-needs-base` | this data keeps its rows in a base file, which this edit cannot read yet | 此資料將列儲存在基礎檔案中，此編輯暫時無法讀取 |
| `data-no-column` | column { $column } does not exist | 欄 { $column } 不存在 |
| `data-no-row` | row { $row } does not exist | 列 { $row } 不存在 |
| `data-non-finite` | { $count -><br>    [one] { $count } number is not finite (NaN or infinity)<br>   *[other] { $count } numbers are not finite (NaN or infinity)<br>} | 有{ $count }個數字不是有限值（NaN 或無限大） |
| `data-row-cut` | { $count -><br>    [one] { $count } row had more cells than the header; the extra cells were dropped<br>   *[other] { $count } rows had more cells than the header; the extra cells were dropped<br>} | 有{ $count }列的儲存格多於表頭，多餘的儲存格已刪除 |
| `data-row-width` | a row has more cells than the table’s { $max } { $max -><br>    [one] column<br>   *[other] columns<br>} | 某列的儲存格多於表格的{ $max }欄 |
| `data-too-large` | { $what -><br>    [rows] the data would pass its limit of { $max } { $max -><br>        [one] row<br>       *[other] rows<br>    }<br>    [columns] the data would pass its limit of { $max } { $max -><br>        [one] column<br>       *[other] columns<br>    }<br>    [text] a text cell would pass its limit of { $max } { $max -><br>        [one] byte<br>       *[other] bytes<br>    }<br>   *[overlay] the data would pass its limit of { $max } { $max -><br>        [one] edited cell<br>       *[other] edited cells<br>    }<br>} | { $what -><br>    [rows] 資料將超過列數上限（{ $max }列）<br>    [columns] 資料將超過欄數上限（{ $max }欄）<br>    [text] 文字儲存格將超過上限（{ $max }位元組）<br>   *[overlay] 資料將超過已編輯儲存格上限（{ $max }個）<br>} |

### zh-Hant/document.ftl

| Key | English | zh-Hant |
|---|---|---|
| `render-chart-too-many-values` | too many values to draw ({ $count }, at most { $max }) | 要繪製的值過多（{ $count }，最多{ $max }） |

### zh-Hant/mcp.ftl

| Key | English | zh-Hant |
|---|---|---|
| `mcp-data-unknown` | No data has the id “{ $data }”; data_list lists them. | 沒有 id 為「{ $data }」的資料；data_list 會列出它們。 |

### zh-Hant/session.ftl

| Key | English | zh-Hant |
|---|---|---|
| `session-bases-unavailable` | The app's data folder cannot hold the deck data files | 應用程式的資料資料夾無法存放簡報的資料檔案 |
| `session-chart-kind-shared` | Cannot change the type of { $subject }: other views show its data, which lacks the columns this type needs. | 無法變更 { $subject } 的類型：其他檢視也顯示這些資料，而資料缺少此類型需要的欄。 |
| `session-data-base-stale` | The data “{ $data }” changed while its stored rows were read; try again | 讀取已儲存的列時資料「{ $data }」已變更；請重試 |
| `session-data-store-full` | The data store is full: { $data } could not be compacted. Free space under [sources] max_store_mib. | 資料儲存區已滿：無法壓縮 { $data }。請在 [sources] max_store_mib 下釋出空間。 |
| `session-data-undo-cut` | The merge replaced edits to { $data }: undo history was cleared | 合併取代了對 { $data } 的編輯：復原記錄已清除 |
| `session-data-user` | { $element } on slide { $slide } | 投影片 { $slide } 上的 { $element } |
| `session-data-view-unfollowed` | Columns cannot be added or removed here: the view { $view } cannot follow them | 此處無法新增或刪除欄：檢視 { $view } 無法跟隨 |
| `session-example-base-chart-category` | Category | 類別 |
| `session-example-base-chart-value` | Value | 值 |
| `session-request-too-large` | The request is larger than { $mib } MiB. | 請求超過 { $mib } MiB。 |
| `session-undo-data-add` | Add data | 新增資料 |
| `session-undo-data-bind` | Bind data | 繫結資料 |
| `session-undo-data-edit` | Edit data | 編輯資料 |
| `session-undo-data-remove` | Remove data | 刪除資料 |
| `session-undo-data-rename` | Rename data | 重新命名資料 |
| `session-undo-data-type` | Change the column type | 變更欄類型 |

### zh-Hant/ui.ftl

| Key | English | zh-Hant |
|---|---|---|
| `ui-data-add` | Add data | 新增資料 |
| `ui-data-column-type` | Column type | 欄類型 |
| `ui-data-next` | Next rows | 下一頁 |
| `ui-data-open` | Open the data | 開啟資料 |
| `ui-data-page` | Rows { $first }–{ $last } of { $total } | 第 { $first }–{ $last }列，共 { $total }列 |
| `ui-data-previous` | Previous rows | 上一頁 |
| `ui-data-remove` | Remove | 刪除 |
| `ui-data-rename` | Rename | 重新命名 |
| `ui-data-shared-by` | Shared by { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | 由 { $count }個檢視共用 |
| `ui-data-size` | Columns: { $columns } · Rows: { $rows } | 欄：{ $columns } · 列：{ $rows } |
| `ui-data-tab` | Data | 資料 |
| `ui-data-type-bool` | Yes/no | 是/否 |
| `ui-data-type-date` | Date (YYYY-MM-DD) | 日期 |
| `ui-data-type-integer` | Integer | 整數 |
| `ui-data-type-number` | Number | 數字 |
| `ui-data-type-text` | Text | 文字 |
| `ui-data-unused` | Unused | 未使用 |
| `ui-data-views` | { $count -><br>    [one] { $count } view<br>   *[other] { $count } views<br>} | { $count }個檢視 |
