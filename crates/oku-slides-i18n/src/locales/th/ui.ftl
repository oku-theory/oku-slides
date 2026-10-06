# อินเทอร์เฟซของแอป (React) โหลดโดย ui/src/i18n.ts ผ่านคำขอ « messages »
ui-loading = กำลังโหลด…

## เอกสาร

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · เทมเพลต
ui-title-rename = เปลี่ยนชื่องานนำเสนอ
ui-title-field = ชื่องานนำเสนอ
ui-status-unsaved = มีการเปลี่ยนแปลงที่ยังไม่บันทึก
ui-status-saved = บันทึกแล้ว
ui-status-never-saved = ยังไม่ได้บันทึก
ui-caption-templates = { $count ->
   *[other] { $count } เทมเพลต
} · { $status }
ui-caption-slides = { $count ->
   *[other] { $count } สไลด์
} · { $status }
ui-template-default-name = เทมเพลต
ui-template-variant-name = { $name } (รูปแบบอื่น)
ui-confirm-quit-templates = ออกโดยไม่บันทึกเทมเพลตที่แก้ไขหรือไม่

## การแจ้งเตือน

ui-toast-templates-saved = บันทึกเทมเพลตแล้ว: { $name }
ui-toast-moved = ย้ายงานนำเสนอแล้ว
ui-toast-copied = บันทึกสำเนาแล้ว
ui-toast-select-zone = เลือกพื้นที่สำหรับวาง: “{ $text }”
ui-toast-field-gone = ช่องที่คุณกำลังแก้ไขไม่มีอยู่แล้ว: ข้อความที่พิมพ์สูญหาย

## แถบเครื่องมือ

ui-toolbar-undo = เลิกทำ
ui-toolbar-undo-label = เลิกทำ: { $label }
ui-toolbar-redo = ทำซ้ำ
ui-toolbar-redo-label = ทำซ้ำ: { $label }
ui-toolbar-grid-hide = ซ่อนเส้นตาราง
ui-toolbar-grid-show = แสดงเส้นตาราง
ui-toolbar-ghosts-hide = ซ่อนโครงร่าง
ui-toolbar-ghosts-show = แสดงโครงร่าง
ui-divider-pile = ความกว้างของกองสไลด์
ui-divider-notes = ความกว้างของเธรดเอเจนต์
ui-divider-code = ขนาดโค้ด
ui-toolbar-theme = ธีม
ui-toolbar-theme-default = ค่าเริ่มต้น
ui-toolbar-save-copy = บันทึกสำเนา
ui-toolbar-finish = เสร็จสิ้น
ui-menu-workshop = เวิร์กช็อป
ui-toolbar-open = เปิด
ui-merge = รวมกับ PDF…
ui-menu = เมนู
ui-menu-move = ย้าย…
ui-language = ภาษา
ui-language-system = ภาษาของระบบ
ui-scheme = ลักษณะที่ปรากฏ
ui-scheme-system = ตามระบบ
ui-scheme-light = สว่าง
ui-scheme-dark = มืด
ui-toolbar-add-label = เพิ่มลงในสไลด์
ui-add-kind = { $kind ->
    [card] การ์ด
    [shape] รูปร่าง
    [chart] แผนภูมิ
   *[text] ข้อความ
}
ui-add-widget = วิดเจ็ตของแพ็ก
ui-toolbar-remove-template = ลบเทมเพลต
ui-toolbar-remove-slide = ลบสไลด์
ui-toolbar-remove-hint = สไลด์ { $n } · เลิกทำด้วย ⌘Z

## ป้ายกำกับการเลิกทำ

ui-undo-add = { $kind ->
    [card] เพิ่มการ์ด
    [shape] เพิ่มรูปร่าง
    [chart] เพิ่มแผนภูมิ
   *[text] เพิ่มข้อความ
}
ui-undo-add-widget = เพิ่ม “{ $name }”
ui-undo-new-template = เทมเพลตใหม่: { $name }
ui-undo-new-slide = สไลด์ใหม่: { $name }
ui-undo-templates = เทมเพลต: { $name }
ui-undo-move-template = ย้ายเทมเพลต
ui-undo-move-slide = ย้ายสไลด์
ui-undo-remove-slide = ลบสไลด์
ui-undo-remove = ลบ
ui-undo-cut = ตัด
ui-undo-paste = วาง
ui-undo-place-on-grid = วางบนเส้นตาราง
ui-undo-move-on-grid = ย้ายบนเส้นตาราง
ui-undo-resize-on-grid = ปรับขนาดบนเส้นตาราง
ui-undo-change-anchor = เปลี่ยนจุดยึด
ui-undo-move = ย้าย
ui-undo-resize = ปรับขนาด
ui-undo-edit-slide = แก้ไขสไลด์
ui-undo-dictation = พิมพ์ด้วยเสียง
ui-undo-rename-set = เปลี่ยนชื่อชุด
ui-undo-rename-deck = เปลี่ยนชื่องานนำเสนอ
ui-undo-theme = เปลี่ยนธีม
ui-undo-chart-kind = เปลี่ยนประเภทแผนภูมิ
ui-undo-chart-data = แก้ไขข้อมูล
ui-undo-chart-color = กำหนดสีชุดข้อมูล
ui-undo-chart-stack = { $stacked ->
    [yes] ซ้อนชุดข้อมูล
   *[no] แยกชุดข้อมูล
}

## องค์ประกอบใหม่

ui-new-text = ข้อความใหม่
ui-new-card-title = ชื่อการ์ด
ui-new-card-detail = รายละเอียด

## รายการสไลด์

ui-pile-slides = สไลด์
ui-pile-templates = เทมเพลต
ui-pile-add-slide = + สไลด์ใหม่
ui-pile-add-template = + เทมเพลตใหม่
ui-pile-drag-hint = ลากเพื่อย้าย · ⌥↑ ⌥↓
ui-slide-alt = สไลด์ { $n }

## พื้นที่ทำงาน

ui-stage-rendering = กำลังเรนเดอร์…
ui-stage-elements = องค์ประกอบของสไลด์
ui-stage-fields = ช่องของสไลด์
ui-stage-edit-text = แก้ไขข้อความ
ui-stage-hint-plain = Enter หรือ Tab เพื่อไปช่องถัดไป · Esc เพื่อยกเลิก
ui-stage-hint-rich = Tab เพื่อไปช่องถัดไป · ⌘ Enter เพื่อยืนยัน · ใช้ “- ” สำหรับหัวข้อย่อย · ⌘B ตัวหนา ⌘I ตัวเอียง ⌘K ลิงก์
ui-field = { $kind ->
    [title] ชื่อเรื่อง
    [subtitle] ชื่อรอง
    [kicker] คำนำ
    [text] ข้อความ
    [body] เนื้อหา
    [author] ผู้เขียน
    [date] วันที่
    [organization] องค์กร
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] ข้อความ
    [card] การ์ด
    [image] รูปภาพ
    [shape] รูปร่าง
    [group] กลุ่ม
   *[other] { $kind }
}
ui-grid-cell = คอลัมน์ { $cols } · แถว { $rows }
ui-ghost-zone = โซน

## บันทึกของสไลด์

ui-notes = เครื่องมือสไลด์
ui-stepper-less = { $label }: ลดหนึ่ง
ui-stepper-more = { $label }: เพิ่มหนึ่ง
ui-list-item = รายการ
ui-list-add = เพิ่ม{ $item }
ui-list-remove = ลบ{ $item }
ui-list-move = ย้าย{ $item }
ui-list-full = รายการเต็มแล้ว ({ $max })
ui-undo-list-remove = ลบ{ $item }
ui-undo-list-move = ย้าย{ $item }
ui-agent-color-other = สีอื่น
ui-agent-thread = เอเจนต์
ui-agent-new-conversation = บทสนทนาใหม่
ui-draft = ฉบับร่าง
ui-draft-insert = แทรก
ui-draft-insert-title = แทรกตามเดิม (⌘⇧↩)
ui-draft-send = ส่งถึงเอเจนต์
ui-draft-send-title = ส่งถึงเอเจนต์ (⌘↩)
ui-draft-discard = ทิ้ง
ui-agent-state-idle = เอเจนต์
ui-agent-state-starting = เอเจนต์กำลังเริ่ม…
ui-agent-state-working = เอเจนต์กำลังทำงาน…
ui-agent-dock = กำลังดำเนินการ
ui-agent-dock-settings = การตั้งค่า
ui-agent-jump = ↓ รอบปัจจุบัน
ui-agent-sheet = คำถามของเอเจนต์
ui-agent-sheet-keys = 1–4 เลือก · Enter ยืนยัน · E แก้ไข · Esc ยกเลิก
ui-agent-variants = รูปแบบอื่น
ui-agent-drawer-keys = คลิกหรือ 1–4 · Esc ปิด
ui-agent-confirm = การยืนยันของเอเจนต์

## การพิมพ์ด้วยเสียง

ui-dictation = การพิมพ์ด้วยเสียง
ui-dictation-start = พิมพ์ด้วยเสียง
ui-dictation-start-title = พิมพ์ด้วยเสียง (⌘D)
ui-dictation-type = พิมพ์ถึงเอเจนต์
ui-dictation-type-title = พิมพ์ถึงเอเจนต์ (⌘K)
ui-dictation-agent-stop = หยุดเอเจนต์ (⌘⇧D หรือ Esc)
ui-dictation-agent-working = เอเจนต์กำลังทำงาน…
ui-dictation-agent-working-hint = ⌘⇧D หรือ Esc เพื่อหยุด
ui-dictation-finish = จบการพิมพ์ด้วยเสียง
ui-dictation-finish-title = จบ (⌘D)
ui-dictation-cancel = ยกเลิก
ui-dictation-preparing = กำลังเตรียมการพิมพ์ด้วยเสียง…
ui-dictation-heard-nothing = ไม่ได้ยินเสียง: ลองพูดใกล้ไมโครโฟนขึ้นอีกนิด
ui-dictation-to-cursor = ที่เคอร์เซอร์
ui-dictation-to-draft = ลงในฉบับร่าง
ui-dictation-to-field = แทนที่{ $field }
ui-dictation-to-block = หลังบล็อกที่เลือก
ui-dictation-new-block = บล็อกข้อความใหม่
ui-dictation-select-zone = เลือกพื้นที่
ui-dictation-downloading = กำลังดาวน์โหลดโมเดล { $name }
ui-dictation-download-progress = { $percent }% · { $received } จาก { $total } · ครั้งเดียวเท่านั้น
ui-dictation-listening = กำลังฟัง · { $time }
ui-dictation-transcribing = กำลังถอดเสียงประโยคสุดท้าย…
ui-dictation-stop-in = หยุดใน { $duration }
ui-dictation-speak = พูดได้เลย ข้อความจะปรากฏที่นี่…
ui-dictation-auto-stop = หยุดเมื่อเงียบ { $duration }
ui-dictation-to-download = ต้องดาวน์โหลด { $size }
ui-dictation-settings = การตั้งค่าการพิมพ์ด้วยเสียง
ui-dictation-model-title = โมเดลการรู้จำเสียง
ui-dictation-model-intro = Whisper บนคอมพิวเตอร์เครื่องนี้ (CTranslate2) ทุกโมเดลเข้าใจได้หลายภาษา
ui-dictation-note-turbo = แนะนำ รวดเร็วและแม่นยำ
ui-dictation-note-large = แม่นยำที่สุด แต่ช้ากว่า
ui-dictation-note-base = เบา สำหรับเครื่องสเปกต่ำ
ui-dictation-installed = ติดตั้งแล้ว
ui-dictation-not-installed = ต้องดาวน์โหลด
ui-dictation-language = ภาษาที่พูด
ui-dictation-language-auto = ตรวจหาอัตโนมัติ
ui-dictation-note-apple = เร็วที่สุด ในตัวระบบ
ui-dictation-model-unavailable = ไม่รองรับภาษานี้
ui-dictation-model-intro-apple = Apple ในตัวระบบ หรือ Whisper บนคอมพิวเตอร์เครื่องนี้ (CTranslate2) ทั้งหมดเข้าใจหลายภาษา
ui-dictation-language-auto-resolved = อัตโนมัติ ({ $language })
ui-dictation-to-download-language = ต้องดาวน์โหลดภาษา
ui-dictation-auto-stop-label = หยุดเองเมื่อเงียบนาน
ui-dictation-silence = ระยะเวลาเงียบ
ui-dictation-phrases-note = ระบบจะถอดเสียงเฉพาะช่วงที่มีการพูด ทีละประโยค ขณะที่คุณพูด
ui-dictation-download-now = ดาวน์โหลดเลย ({ $size })
ui-dictation-download-language = ดาวน์โหลดภาษา
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } วินาที
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## ตัวเลือกเทมเพลต

ui-picker-new-slide = สไลด์ใหม่
ui-picker-new-template = เทมเพลตใหม่ โดยอิงจาก…
ui-picker-set = เทมเพลต
ui-picker-set-mine = { $name } · ของฉัน
ui-picker-keys = ← → เพื่อเลื่อนดู · Enter เพื่อเพิ่ม · Esc เพื่อปิด
ui-picker-close = ปิด
ui-picker-previous = เทมเพลตก่อนหน้า
ui-picker-next = เทมเพลตถัดไป
ui-picker-list = เทมเพลตสไลด์

## แผนภูมิ

ui-chart-panel = แผนภูมิ
ui-chart-kinds = ประเภทแผนภูมิ
ui-chart-kind = { $kind ->
    [line] เส้น
    [pie] วงกลม
    [scatter] กระจาย
    [radar] เรดาร์
    [waterfall] น้ำตก
   *[bar] แท่ง
}
ui-chart-no-data = แผนภูมินี้ไม่มีตารางข้อมูล
ui-chart-options = ตัวเลือกแผนภูมิ
ui-chart-stack = ซ้อนกัน
ui-chart-waterfall-hint = เลือก «รวม» เพื่อแสดงแท่งผลรวม
ui-chart-colors = สีของชุดข้อมูล
ui-chart-color-of = สีของ { $name }
ui-chart-color-auto = อัตโนมัติ
ui-chart-color-pick = สี { $n }
ui-chart-column-name = ชื่อคอลัมน์ { $col }
ui-chart-remove-column = ลบคอลัมน์
ui-chart-cell = แถว { $row }, คอลัมน์ { $col }
ui-chart-remove-row = ลบแถว
ui-chart-add-row = แถว
ui-chart-add-column = คอลัมน์
ui-chart-series = ชุดข้อมูล { $n }

## ไฟล์

ui-file-open-prompt = พาธของไฟล์ที่จะเปิด (.oku-slides.pdf หรือ .json)
ui-file-save-prompt = บันทึกเป็น (พาธแบบเต็ม)

## แท็บ

ui-tabs = งานนำเสนอที่เปิดอยู่
ui-tab-new = แท็บใหม่
ui-tab-close = ปิดแท็บ
ui-tab-unsaved = การเปลี่ยนแปลงที่ยังไม่บันทึก
ui-tab-templates = เทมเพลต
ui-tab-move-new-window = ย้ายไปหน้าต่างใหม่
ui-tab-move-to = ย้ายไปหน้าต่าง { $n }
ui-tab-close-others = ปิดแท็บอื่น
ui-close-cancel = ยกเลิก
ui-disk-replaced-question = { $file } บนดิสก์ถูกแทนที่ด้วยงานนำเสนออื่น
ui-disk-replaced-overwrite = เขียนทับ
ui-disk-replaced-save-as = บันทึกเป็น…
ui-same-deck-question = { $file } เป็นอีกเวอร์ชันหนึ่งของงานนำเสนอที่เปิดอยู่ในแท็บ “{ $title }”
ui-same-deck-merge = รวมเข้ากับแท็บนั้น
ui-same-deck-apart = เปิดแยก
ui-history = ประวัติ
ui-history-version = เวอร์ชันของ { $date }
ui-history-restore = กู้คืน
ui-history-back = กลับ
ui-history-anonymous = ไม่ระบุชื่อ
ui-history-slider = เวอร์ชันของงานนำเสนอ
ui-history-removed-since = ถูกลบตั้งแต่นั้น
ui-history-changed-since = ถูกแก้ไขตั้งแต่นั้น
ui-history-added-since = สไลด์ที่เพิ่มตั้งแต่นั้น: { $count }
ui-help = ความช่วยเหลือ
ui-help-caption = สไลด์ { $index } จาก { $count } · อ่านอย่างเดียว
ui-help-previous = สไลด์ก่อนหน้า
ui-help-next = สไลด์ถัดไป
ui-help-close = ปิดความช่วยเหลือ
ui-help-search = ค้นหาในความช่วยเหลือ
ui-help-search-placeholder = ค้นหา…
ui-help-search-empty = ไม่พบผลลัพธ์
ui-help-search-count = { $count } ผลลัพธ์
ui-pack-notice-close = ปิด
ui-widgets-files = ไฟล์วิดเจ็ต
ui-widgets-common = ใช้ร่วมกัน
ui-widgets-new = วิดเจ็ตใหม่
ui-widgets-name = ชื่อวิดเจ็ต (ตัวพิมพ์เล็ก ตัวเลข และขีดกลาง)
ui-widgets-name-taken = ชื่อนี้ถูกใช้แล้ว
ui-widgets-name-letter = ชื่อต้องขึ้นต้นด้วยตัวอักษร
ui-widgets-valid = ถูกต้อง
ui-widgets-errors = ข้อผิดพลาด
ui-widgets-empty = แพ็กนี้ยังไม่มีวิดเจ็ต
ui-widgets-preview = ตัวอย่างวิดเจ็ต
ui-widgets-no-preview = ไม่มีตัวอย่าง
ui-widgets-field-bound = เชื่อมกับงานนำเสนอ: อ่านอย่างเดียวที่นี่
ui-widgets-source = ซอร์สโค้ด
ui-widgets-unsaved = ยังไม่ได้บันทึก
ui-widgets-in-error = วิดเจ็ตที่มีข้อผิดพลาด
ui-widgets-none-in-error = ไม่มีวิดเจ็ตที่มีข้อผิดพลาด
ui-widgets-section = วิดเจ็ต
ui-widgets-open = เปิดวิดเจ็ต { $name }
ui-widgets-delete-named = ลบ { $name }
ui-widgets-confirm-delete = ลบวิดเจ็ต “{ $name }” หรือไม่?
ui-widgets-rename-hint = ดับเบิลคลิกหรือกด F2 เพื่อเปลี่ยนชื่อ
ui-widgets-tabs = แท็บเทมเพลต
ui-widgets-templates-tab = เทมเพลต
ui-widgets-close-tab = ปิด { $name }
ui-widgets-removed-by-agent = เอเจนต์ลบวิดเจ็ต { $name } แล้ว
