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
ui-status-builtin = ในตัว: การบันทึกจะสร้างสำเนา
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

ui-toast-saved = บันทึกงานนำเสนอของคุณแล้ว
ui-toast-templates-saved = บันทึกเทมเพลตแล้ว: { $name }
ui-toast-select-zone = เลือกพื้นที่สำหรับวาง: “{ $text }”

## แถบเครื่องมือ

ui-toolbar-undo = เลิกทำ
ui-toolbar-undo-label = เลิกทำ: { $label }
ui-toolbar-redo = ทำซ้ำ
ui-toolbar-redo-label = ทำซ้ำ: { $label }
ui-toolbar-grid-hide = ซ่อนเส้นตาราง
ui-toolbar-grid-show = แสดงเส้นตาราง
ui-toolbar-grid-hide-title = ซ่อนเส้นตาราง (⌘')
ui-toolbar-grid-show-title = แสดงเส้นตาราง (⌘')
ui-toolbar-theme = ธีม
ui-toolbar-theme-default = ค่าเริ่มต้น
ui-toolbar-save-copy = บันทึกสำเนา
ui-toolbar-save = บันทึก
ui-toolbar-finish = เสร็จสิ้น
ui-toolbar-templates = เทมเพลต
ui-toolbar-templates-title = สร้างและแก้ไขเทมเพลตสไลด์
ui-toolbar-open = เปิด
ui-toolbar-add-label = เพิ่มลงในสไลด์
ui-toolbar-add = เพิ่ม
ui-add-kind = { $kind ->
    [card] การ์ด
    [shape] รูปร่าง
    [chart] แผนภูมิ
   *[text] ข้อความ
}
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
ui-undo-new-template = เทมเพลตใหม่: { $name }
ui-undo-new-slide = สไลด์ใหม่: { $name }
ui-undo-grid = เส้นตาราง { $columns } × { $rows }
ui-undo-templates = เทมเพลต: { $name }
ui-undo-move-template = ย้ายเทมเพลต
ui-undo-move-slide = ย้ายสไลด์
ui-undo-remove-slide = ลบสไลด์
ui-undo-remove = ลบ
ui-undo-place-on-grid = วางบนเส้นตาราง
ui-undo-move-on-grid = ย้ายบนเส้นตาราง
ui-undo-resize-on-grid = ปรับขนาดบนเส้นตาราง
ui-undo-change-anchor = เปลี่ยนจุดยึด
ui-undo-move = ย้าย
ui-undo-resize = ปรับขนาด
ui-undo-edit-text = แก้ไขข้อความ
ui-undo-dictation = พิมพ์ด้วยเสียง
ui-undo-rename-set = เปลี่ยนชื่อชุด
ui-undo-rename-deck = เปลี่ยนชื่องานนำเสนอ
ui-undo-theme = เปลี่ยนธีม
ui-undo-rename-template = เปลี่ยนชื่อเทมเพลต
ui-undo-describe-template = อธิบายเทมเพลต
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
ui-stage-edit-text = แก้ไขข้อความ
ui-stage-hint-field = Enter เพื่อยืนยัน · Esc เพื่อยกเลิก
ui-stage-hint-block = ⌘ Enter เพื่อยืนยัน · หนึ่งบรรทัดต่อย่อหน้า ใช้ “- ” สำหรับหัวข้อย่อย
ui-field = { $kind ->
    [title] ชื่อเรื่อง
    [subtitle] ชื่อรอง
    [kicker] คำนำ
    [text] ข้อความ
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

## บันทึกของสไลด์

ui-notes = เครื่องมือสไลด์
ui-stepper-less = { $label }: ลดหนึ่ง
ui-stepper-more = { $label }: เพิ่มหนึ่ง

## การพิมพ์ด้วยเสียง

ui-dictation = การพิมพ์ด้วยเสียง
ui-dictation-start = พิมพ์ด้วยเสียง
ui-dictation-start-title = พิมพ์ด้วยเสียง (⌘D)
ui-dictation-finish = จบการพิมพ์ด้วยเสียง
ui-dictation-finish-title = จบ (⌘D)
ui-dictation-cancel = ยกเลิก
ui-dictation-preparing = กำลังเตรียมการพิมพ์ด้วยเสียง…
ui-dictation-heard-nothing = ไม่ได้ยินเสียง: ลองพูดใกล้ไมโครโฟนขึ้นอีกนิด
ui-dictation-to-cursor = ที่เคอร์เซอร์
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
ui-dictation-auto-stop-label = หยุดเองเมื่อเงียบนาน
ui-dictation-silence = ระยะเวลาเงียบ
ui-dictation-phrases-note = ระบบจะถอดเสียงเฉพาะช่วงที่มีการพูด ทีละประโยค ขณะที่คุณพูด
ui-dictation-download-now = ดาวน์โหลดเลย ({ $size })
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
ui-chart-add-row = + แถว
ui-chart-add-column = + คอลัมน์
ui-chart-series = ชุดข้อมูล { $n }

## ไฟล์

ui-file-open-prompt = พาธของไฟล์ที่จะเปิด (.oku-slides.pdf หรือ .json)
ui-file-save-prompt = บันทึกเป็น (พาธแบบเต็ม)
