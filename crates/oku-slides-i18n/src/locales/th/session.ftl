session-read = อ่าน { $path } ไม่ได้
session-write = เขียน { $path } ไม่ได้
session-not-oku-slides = PDF นี้ไม่ได้สร้างด้วย Oku Slides จึงแก้ไขไม่ได้
session-invalid-deck = JSON ของงานนำเสนอไม่ถูกต้อง
session-not-rendered = งานนำเสนอยังไม่ได้เรนเดอร์
session-pack-disabled = วิดเจ็ตของแพ็ก “{ $pack }” ถูกปิดใช้งานในเซสชันนี้: { $reason } งานนำเสนอแสดงด้วยวิดเจ็ตพื้นฐาน
session-pack-missing = ไม่พบแพ็กเทมเพลตของงานนำเสนอนี้ในไฟล์ จึงแสดงด้วยวิดเจ็ตพื้นฐาน
session-no-path = ยังไม่มีไฟล์: เลือกตำแหน่งที่จะบันทึก
session-no-slide = ไม่พบสไลด์: { $id }
session-no-chart = ไม่มีแผนภูมิที่นี่: { $subject }
session-builtin-template-set = ลบชุดเทมเพลตในตัวไม่ได้
session-no-templates-dir = ไม่มีโฟลเดอร์สำหรับเทมเพลตของคุณ
session-unknown-pack = ไม่พบแพ็กเทมเพลต: { $id }
session-pack-version-missing = ไม่พบเวอร์ชัน { $version } ของแพ็กเทมเพลต { $id }
session-invalid-pack = อ่านแพ็กเทมเพลตไม่ได้: { $id }
session-pack-id-mismatch = pack.json นี้เป็นของแพ็กเทมเพลต { $found } ไม่ใช่ { $id }
session-editing-templates = แก้ไขเทมเพลตให้เสร็จก่อน
session-not-editing-templates = ไม่มีชุดเทมเพลตที่กำลังแก้ไข
session-response = เขียนคำตอบไม่ได้
session-unknown-tab = แท็บนี้ไม่ได้เปิดอยู่แล้ว
session-unknown-window = หน้าต่างนี้ไม่ได้เปิดอยู่แล้ว
session-single-document = ที่นี่เปิดงานนำเสนอได้เพียงรายการเดียว
session-last-window = หน้าต่างสุดท้ายจะยังคงเปิดอยู่ ให้ออกจากแอปแทน
session-unsaved = “{ $title }” มีการเปลี่ยนแปลงที่ยังไม่บันทึก
session-bad-request = คำขอไม่ถูกต้อง: { $reason }
session-history-not-open = ไม่ได้เปิดประวัติ
session-in-history = ปิดประวัติก่อนแก้ไขงานนำเสนอ
session-agent-form = คำขอของเอเจนต์ไม่ถูกต้อง: { $reason }

deck-untitled = ไม่มีชื่อ
deck-new-title = งานนำเสนอใหม่
deck-new-slide-title = ชื่อสไลด์
deck-new-slide-text = บอกสิ่งที่คุณอยากนำเสนอ
templates-default-name = เทมเพลตของฉัน
templates-copy-name = { $name } (สำเนา)
templates-adopt-label = เทมเพลต: { $name }
session-widgets-broken = ละวิดเจ็ตเหล่านี้เพราะคอมไพล์ไม่ผ่าน: { $names } วิดเจ็ตอื่นในแพ็กยังคงอยู่
session-invalid-widget-name = ชื่อวิดเจ็ตไม่ถูกต้อง: “{ $name }” ใช้อักษรละตินตัวพิมพ์เล็ก ตัวเลข และขีดกลาง หรือ _commun
session-widget-too-large = วิดเจ็ต “{ $name }” มีขนาดเกิน { $max } KiB
session-widget-nul = วิดเจ็ต “{ $name }” มีอักขระ NUL
session-too-many-widgets = แพ็กหนึ่งมีไฟล์วิดเจ็ตได้ไม่เกิน { $max } ไฟล์
session-widget-in-use = วิดเจ็ต “{ $name }” ถูกใช้โดยเทมเพลตเหล่านี้: { $templates }
session-unknown-widget = ไม่พบวิดเจ็ต: “{ $name }”
session-widget-exists = มีวิดเจ็ต “{ $name }” อยู่แล้ว
widgets-edit-label = วิดเจ็ต: { $name }
widget-preview-plain = ข้อความตัวอย่าง
widget-preview-rich = ย่อหน้าตัวอย่าง เพื่อดูวิดเจ็ตในบริบทจริง
pack-notice-older = งานนำเสนอนี้ใช้ “{ $name }” เวอร์ชันที่เก่ากว่าของคุณ
pack-notice-newer = งานนำเสนอนี้ใช้ “{ $name }” เวอร์ชันที่ใหม่กว่าของคุณ
pack-notice-parallel = งานนำเสนอนี้ใช้ “{ $name }” เวอร์ชันที่สร้างขึ้นคู่ขนานกับของคุณ
pack-notice-use-mine = เปลี่ยนเป็นเวอร์ชันของฉัน
pack-notice-make-current = ตั้งเวอร์ชันของเขาเป็นเวอร์ชันปัจจุบันของฉัน
template-default-name = เทมเพลต
history-restore-label = กู้คืนเวอร์ชันของ { $date }
notice-merged = รวมกับ { $file } แล้ว — การเปลี่ยนแปลงที่นำเข้า: { $count } ({ $authors })
notice-merged-anonymous = รวมกับ { $file } แล้ว — การเปลี่ยนแปลงที่นำเข้า: { $count }
notice-same-deck-open = งานนำเสนอนี้เปิดอยู่แล้วในแท็บ “{ $title }” หากต้องการรวม ให้ใช้ “รวมกับ PDF…” ในแท็บนั้น
notice-modified-elsewhere = PDF นี้ถูกแก้ไขโดยเครื่องมืออื่นหลังจาก Oku Slides: ซอร์สอาจไม่ตรงกับสิ่งที่โปรแกรมอ่าน PDF แสดงอีกต่อไป
notice-open-failed = ไม่สามารถเปิด { $path } ได้: { $reason }
notice-tab-resume-failed = ไม่สามารถกู้คืนสถานะของแท็บนี้ได้: สถานะถูกแยกเก็บไว้ และแท็บจะเริ่มใหม่จากงานนำเสนอเปล่า
notice-unreadable-parts = บางส่วนของงานนำเสนอนี้อ่านไม่ได้และถูกแยกไว้ ({ $count })

log-document-opened = เปิดเอกสารแล้ว
log-document-saved = บันทึกเอกสารแล้ว

## Panneau de droite (surface A2UI)

session-panel-template = เทมเพลตนี้
session-panel-name = ชื่อ
session-panel-description = คำอธิบาย
session-panel-grid = เส้นตาราง
session-panel-columns = คอลัมน์
session-panel-rows = แถว

## Menu

menu-edit = แก้ไข
menu-window = หน้าต่าง
menu-quit = ออก
