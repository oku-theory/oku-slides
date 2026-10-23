# อินเทอร์เฟซของแอป (React) โหลดโดย ui/src/i18n.ts ผ่านคำขอ « messages »
ui-loading = กำลังโหลด…

## เอกสาร

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · ชุดเลย์เอาต์
   *[other] { $name } · ชุดวิดเจ็ต
}
ui-title-rename = เปลี่ยนชื่องานนำเสนอ
ui-title-field = ชื่องานนำเสนอ
ui-status-unsaved = มีการเปลี่ยนแปลงที่ยังไม่บันทึก
ui-status-saved = บันทึกแล้ว
ui-status-never-saved = ยังไม่ได้บันทึก
ui-caption-pack = { $kind ->
    [layouts] { $count ->
       *[other] { $count } เลย์เอาต์
    }
   *[other] { $count ->
       *[other] { $count } วิดเจ็ต
    }
} · { $status }
ui-caption-slides = { $count ->
   *[other] { $count } สไลด์
} · { $status }
ui-confirm-quit-pack = ออกโดยไม่บันทึกชุดที่แก้ไขหรือไม่

## การแจ้งเตือน

ui-toast-copied = บันทึกสำเนาแล้ว
ui-toast-field-gone = ช่องที่คุณกำลังแก้ไขไม่มีอยู่แล้ว: ข้อความที่พิมพ์สูญหาย
ui-toast-edit-replaced = ข้อความนี้เปลี่ยนไประหว่างที่คุณแก้ไข: ฉบับของคุณแทนที่แล้ว เลิกทำจะคืนอีกฉบับ

## แถบเครื่องมือ

ui-toolbar-undo = เลิกทำ
ui-toolbar-undo-label = เลิกทำ: { $label }
ui-toolbar-redo = ทำซ้ำ
ui-toolbar-redo-label = ทำซ้ำ: { $label }
ui-divider-pile = ความกว้างของกองสไลด์
ui-divider-notes = ความกว้างของเธรดเอเจนต์
ui-divider-code = ขนาดโค้ด
ui-toolbar-theme = ธีม
ui-toolbar-theme-embedded = ธีมของไฟล์
ui-toolbar-finish = เสร็จสิ้น
ui-pack-deck-only = บันทึกไว้ในงานนำเสนอนี้เท่านั้น
ui-toolbar-open = เปิด
ui-menu = เมนู
ui-toolbar-add-label = เพิ่มลงในสไลด์
ui-toolbar-packs = แพ็กของงานนำเสนอ
ui-toolbar-packs-update = แพ็กของงานนำเสนอ — มีการอัปเดต
ui-toolbar-thumbs-show = แสดงภาพย่อ
ui-toolbar-thumbs-hide = ซ่อนภาพย่อ
ui-toolbar-remove-slide = ลบสไลด์
ui-toolbar-remove-hint = สไลด์ { $n } · เลิกทำด้วย ⌘Z

## ป้ายกำกับการเลิกทำ

ui-undo-add-widget = เพิ่ม “{ $name }”
ui-undo-new-slide = สไลด์ใหม่: { $name }
ui-undo-move-slide = ย้ายสไลด์
ui-undo-remove-slide = ลบสไลด์
ui-undo-remove = ลบ
ui-undo-paste = วาง
ui-undo-place-on-grid = วางบนเส้นตาราง
ui-undo-move-on-grid = ย้ายบนเส้นตาราง
ui-undo-resize-on-grid = ปรับขนาดบนเส้นตาราง
ui-undo-change-anchor = เปลี่ยนจุดยึด
ui-undo-move = ย้าย
ui-undo-resize = ปรับขนาด
ui-undo-edit-slide = แก้ไขสไลด์
ui-undo-rename-pack = เปลี่ยนชื่อชุด
ui-undo-rename-deck = เปลี่ยนชื่องานนำเสนอ

## รายการสไลด์

ui-pile-slides = สไลด์
ui-pile-layouts = เลย์เอาต์
ui-pile-add-slide = สไลด์ใหม่
ui-pile-drag-hint = ลากเพื่อย้าย · ⌥↑ ⌥↓
ui-slide-alt = สไลด์ { $n }

## พื้นที่ทำงาน

ui-stage-rendering = กำลังเรนเดอร์…
ui-stage-widget-pack = เปิดวิดเจ็ตจากกอง หรือสร้างใหม่
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
    [chart] แผนภูมิ
    [table] ตาราง
    [stack] สแตก
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
ui-undo-list-move = ย้าย{ $item }
ui-agent-color-other = สีอื่น
ui-agent-thread = เอเจนต์
ui-agent-new-conversation = บทสนทนาใหม่
ui-agent-undo-turn = เลิกทำรอบนี้
ui-agent-detach = เปิดเอเจนต์ในหน้าต่างแยก
ui-agent-attach = นำเอเจนต์กลับมาที่หน้าต่างหลัก
ui-agent-window-empty = ไม่มีเอเจนต์สำหรับแท็บนี้
ui-agent-kept-in-draft = ไม่มีที่ให้เขียน: เก็บข้อความไว้ในฉบับร่าง
ui-agent-busy-elsewhere = เอเจนต์กำลังตอบใน { $place }…
ui-agent-scope-slides = สไลด์
ui-agent-scope-atelier = เวิร์กชอป
ui-draft = ฉบับร่าง
ui-draft-send = ส่งถึงเอเจนต์
ui-draft-send-title = ส่งถึงเอเจนต์ (⌘↩)
ui-draft-discard = ทิ้ง
ui-agent-state-idle = เอเจนต์
ui-agent-state-starting = เอเจนต์กำลังเริ่ม…
ui-agent-state-working = เอเจนต์กำลังทำงาน…
ui-agent-dock = กำลังดำเนินการ
ui-agent-dock-settings = การตั้งค่า
ui-agent-jump = ↓ รอบปัจจุบัน
ui-agent-tool-input = อาร์กิวเมนต์
ui-agent-tool-output = ผลลัพธ์
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
ui-dictation-downloading = กำลังดาวน์โหลดโมเดล { $name }
ui-dictation-download-progress = { $percent }% · { $received } จาก { $total } · ครั้งเดียวเท่านั้น
ui-dictation-listening = กำลังฟัง · { $time }
ui-dictation-transcribing = กำลังถอดเสียงประโยคสุดท้าย…
ui-dictation-stop-in = หยุดใน { $duration }
ui-dictation-speak = พูดได้เลย ข้อความจะปรากฏที่นี่…
ui-dictation-auto-stop = หยุดเมื่อเงียบ { $duration }
ui-dictation-to-download = ต้องดาวน์โหลด { $size }
ui-dictation-settings = การตั้งค่า
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
ui-picker-set-mine = { $name } · ของฉัน
ui-picker-set-received = { $name } · ที่ได้รับ
ui-picker-keys = ← → เพื่อเลื่อนดู · Enter เพื่อเพิ่ม · Esc เพื่อปิด
ui-picker-close = ปิด
ui-picker-list = เลย์เอาต์
ui-widget-picker-filter = กรองตามชื่อ ป้ายกำกับ หรือคำอธิบาย
ui-widget-picker-empty = ไม่มีวิดเจ็ตที่ตรงกับ “{ $query }”
ui-widget-picker-list = วิดเจ็ต
ui-widget-picker-keys = ลูกศรเพื่อเลื่อนดู · Enter เพื่อเพิ่ม · Esc เพื่อปิด

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

## ตาราง

ui-table-panel = ตาราง
ui-table-options = ตัวเลือกตาราง
ui-table-header-row = แถวส่วนหัว
ui-table-header-column = คอลัมน์แรกเป็นส่วนหัว
ui-table-striped = แถวสลับสี
ui-table-alignment = การจัดแนวคอลัมน์
ui-table-align-of = การจัดแนวของ { $name }
ui-table-align = { $align ->
    [start] ชิดต้น
    [center] กึ่งกลาง
    [end] ชิดท้าย
   *[auto] อัตโนมัติ
}
ui-table-column = คอลัมน์ { $n }
ui-table-no-data = ตารางนี้ไม่มีข้อมูล
ui-show-as = แสดงเป็น
ui-show-as-table = แสดงเป็นตาราง
ui-show-as-chart = แสดงเป็นแผนภูมิ

## ไฟล์

ui-file-open-prompt = พาธของไฟล์ที่จะเปิด (.oku-slides.pdf, .json หรือ .pptx)
ui-file-filter-presentations = งานนำเสนอ
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = บันทึกเป็น (พาธแบบเต็ม)

## แท็บ

ui-tabs = งานนำเสนอที่เปิดอยู่
ui-tab-new = แท็บใหม่
ui-tab-close = ปิดแท็บ
ui-tab-unsaved = การเปลี่ยนแปลงที่ยังไม่บันทึก
ui-tab-pack = ชุด
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
ui-slide-previous = สไลด์ก่อนหน้า
ui-slide-next = สไลด์ถัดไป
ui-present = งานนำเสนอ
ui-present-slide = สไลด์ { $index } จาก { $count }
ui-present-screen = นำเสนอแบบเต็มหน้าจอ
ui-help-close = ปิดความช่วยเหลือ
ui-help-search = ค้นหาในความช่วยเหลือ
ui-help-search-placeholder = ค้นหา…
ui-help-search-empty = ไม่พบผลลัพธ์
ui-help-search-count = { $count } ผลลัพธ์
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
ui-pack-tabs = แท็บของชุด
ui-pack-layouts-tab = เลย์เอาต์
ui-theme-tab = ธีม
ui-theme-picker = ธีม
ui-theme-name = ชื่อธีม
ui-theme-unsaved = ยังไม่ได้บันทึกการเปลี่ยนแปลงธีม
ui-widgets-close-tab = ปิด { $name }
ui-widgets-removed-by-agent = เอเจนต์ลบวิดเจ็ต { $name } แล้ว

## นำเข้า

ui-import-title = นำเข้างานนำเสนอ PowerPoint
ui-import-mode = โหมดการนำเข้า
ui-import-hifi = สำเนาตรงต้นฉบับ
ui-import-hifi-note = คงรูปลักษณ์ของต้นฉบับไว้ ด้วยธีมและเลย์เอาต์ที่สร้างขึ้นสำหรับงานนำเสนอนี้
ui-import-retrofit = ใช้เลย์เอาต์ของฉัน
ui-import-retrofit-note = สร้างสไลด์ใหม่ด้วยชุดเลย์เอาต์และธีมจากคลัง
ui-import-layout-packs = ชุดเลย์เอาต์
ui-import-theme-default = ธีมเริ่มต้น
ui-import-no-packs = ไม่มีชุดเลย์เอาต์ในคลัง
ui-import-tune = ปรับแต่งอย่างละเอียดกับเอเจนต์
ui-import-tune-note = เอเจนต์เทียบแต่ละสไลด์กับต้นฉบับและแก้ส่วนที่ต่างกัน
ui-import-tune-note-retrofit = เอเจนต์เทียบแต่ละสไลด์กับต้นฉบับ และออกแบบวิดเจ็ตสำหรับรูปทรงที่เลย์เอาต์ของคุณแสดงไม่ได้
ui-import-tune-unavailable = ไม่มีเอเจนต์ที่ใช้งานได้: { $reason }
ui-import-submit = นำเข้า
ui-import-busy = กำลังนำเข้า…
ui-update-available = Oku Slides { $version } พร้อมใช้งานแล้ว
ui-update-install = ติดตั้งและรีสตาร์ท
ui-update-see-release = ดูรุ่นที่เผยแพร่
ui-update-later = ภายหลัง
ui-update-installing = กำลังดาวน์โหลดการอัปเดต… { $percent }%
ui-update-downloading = กำลังดาวน์โหลดการอัปเดต…
ui-update-up-to-date = Oku Slides เป็นเวอร์ชันล่าสุดแล้ว

ui-shortcut-theme = ธีม, theme, themes
