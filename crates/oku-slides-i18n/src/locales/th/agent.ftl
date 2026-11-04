# เอเจนต์: พื้นผิว การยืนยัน เธรด

agent-surface-cancel = ยกเลิก
agent-surface-confirm = ยืนยัน
agent-surface-confirm-danger = ใช่ ดำเนินการ
agent-surface-heard = ได้ยินว่า: “{ $text }”
agent-surface-say-or-click = พูดคำตอบของคุณหรือคลิกปุ่ม
agent-surface-said-too-early = พูดเร็วเกินไป: รอให้หน้าต่างแสดงก่อน แล้วพูดอีกครั้ง
agent-surface-click-to-confirm = คลิกปุ่มเพื่อยืนยัน
agent-surface-verb-replace = แทนที่
agent-surface-verb-merge = รวม
agent-surface-verb-discard = ทิ้ง
agent-surface-verb-delete = ลบ
agent-surface-verb-use-api = ใช้ api
agent-surface-verb-install = ติดตั้ง
agent-surface-verb-quit = ปิดแอป
agent-surface-verb-close = ปิด
agent-surface-verb-import = นำเข้า
agent-surface-rewrite = การเรียบเรียงใหม่
agent-surface-pick = เลือก { $n }
agent-surface-keep = เก็บต้นฉบับไว้
agent-surface-edit = แก้ไขก่อนนำไปใช้
agent-surface-close = ปิด
agent-surface-themes = ธีม
agent-surface-variants = ตัวเลือกอื่น
agent-surface-permission-always = ตลอดไป
agent-surface-permission-title = เอเจนต์ขอสิทธิ์
agent-surface-permission-guides = เทิร์นนี้มีคู่มือที่ได้รับของ { $names }
agent-undo-turn = เอเจนต์: { $text }

agent-tools-unavailable = ไม่สามารถเริ่มเครื่องมือของเอเจนต์ได้

agent-undo-apply = การเปลี่ยนแปลงของเอเจนต์
agent-undo-variant = ตัวเลือก: { $label }
agent-undo-param = การตั้งค่า: { $label }
agent-undo-rewrite = การเรียบเรียงใหม่

agent-confirm-save = บันทึกงานนำเสนอเป็น { $path } ไหม
agent-confirm-save-over = แทนที่ { $path } ไหม ไฟล์เดิมจะถูกเขียนทับ
agent-confirm-open = เปิด { $path } ไหม
agent-open-elsewhere = ไฟล์นี้เปิดอยู่ในแท็บอื่นแล้ว
agent-confirm-new = เริ่มงานนำเสนอใหม่ไหม
agent-confirm-new-dirty = เริ่มงานนำเสนอใหม่ไหม การเปลี่ยนแปลงที่ยังไม่ได้บันทึกจะหายไป
agent-confirm-copy-pack = บันทึกสำเนาของแพ็ก “{ $name }” ไหม ตัวแพ็กเองจะไม่เปลี่ยนแปลง
agent-confirm-copy-edited-theme = บันทึกสำเนาของธีม “{ $name }” ไหม ตัวธีมเองจะไม่เปลี่ยนแปลง
agent-confirm-delete-pack = ลบแพ็ก “{ $name }” ไหม การกระทำนี้ไม่สามารถย้อนกลับได้
agent-confirm-edit-theme = บันทึกเวอร์ชันใหม่ของธีม “{ $name }” หรือไม่
agent-confirm-copy-theme = สร้างรูปแบบย่อยของธีมในตัว “{ $name }” หรือไม่ ธีมในตัวจะไม่เปลี่ยนแปลง
agent-confirm-setting = เปลี่ยน { $key }: { $old } → { $new } ไหม
agent-confirm-move = ย้ายงานนำเสนอไปที่ { $path } หรือไม่ ไฟล์เดิมจะถูกลบ
agent-confirm-merge = รวม { $path } เข้ากับงานนำเสนอนี้หรือไม่ การกระทำนี้ย้อนกลับไม่ได้
agent-confirm-save-copy = บันทึกสำเนางานนำเสนอเป็น { $path } หรือไม่
agent-confirm-dictation-api = ส่งสิ่งที่คุณพูดไปที่ { $host } ({ $url }) โมเดล { $model } พร้อมความลับจาก { $var } หรือไม่?
agent-confirm-dictation-download = ใช้ { $name } และดาวน์โหลดตอนนี้ ({ $size }) หรือไม่?
agent-confirm-dictation-language = ใช้การเขียนตามคำบอกของ Apple และดาวน์โหลดภาษาของมันตอนนี้หรือไม่?
agent-confirm-agent = สลับเอเจนต์ของแอปเป็น { $name } ({ $command }) เมื่อไม่มีคำตอบที่กำลังทำงานอยู่หรือไม่?
agent-confirm-agent-builtin = สลับเอเจนต์ของแอปเป็น { $name } เมื่อไม่มีคำตอบที่กำลังทำงานอยู่หรือไม่?
agent-confirm-agent-model = ดาวน์โหลดโมเดลของเอเจนต์ในเครื่อง ({ $size }) หรือไม่?

agent-not-found = ไม่พบเอเจนต์: { $command } ไม่มีทั้งที่พาธนั้นและใน PATH ของแอป ระบุพาธเต็มใน [agent.presets]
agent-spawn-failed = เริ่มเอเจนต์ไม่ได้: { $command } การตั้งค่า [agent.presets]
agent-no-http = เอเจนต์นี้ไม่สามารถเชื่อมต่อกับเครื่องมือของ Oku Slides ได้
agent-local-no-program = เอเจนต์ในเครื่องทำงานได้เฉพาะภายในแอป Oku Slides เท่านั้น
agent-local-needs-download = เอเจนต์ในเครื่องต้องดาวน์โหลดโมเดลของตัวเองก่อน ({ $gigabytes } GB)
agent-local-intel-mac = เอเจนต์ในเครื่องต้องใช้ Mac ที่มีชิป Apple
agent-local-unsupported-os = เอเจนต์ในเครื่องยังใช้งานบนระบบนี้ไม่ได้
agent-local-apple-off = เอเจนต์ในเครื่องต้องใช้ Apple Intelligence: เปิดใช้ได้ในการตั้งค่าระบบ
agent-local-apple-not-ready = โมเดลของ Apple ยังเตรียมพร้อมอยู่: ลองใช้เอเจนต์ในเครื่องอีกครั้งในอีกไม่กี่นาที
agent-local-apple-unsupported = โมเดลของ Apple ต้องใช้ Mac ที่รองรับ Apple Intelligence และใช้ macOS 26 ขึ้นไป
agent-model-download = ดาวน์โหลดโมเดล
agent-model-downloading = กำลังดาวน์โหลดโมเดล… { $percent }%
agent-model-cancel = ยกเลิก
agent-model-download-failed = ดาวน์โหลดโมเดลไม่สำเร็จ
agent-model-download-integrity = โมเดลที่ดาวน์โหลดเสียหายและถูกลบแล้ว
agent-model-download-incomplete = ดาวน์โหลดไม่ครบ: ได้ { $got } ไบต์ จาก { $expected } ไบต์
agent-model-download-cancelled = ยกเลิกการดาวน์โหลดแล้ว
agent-fallback-notice = เอเจนต์ในเครื่อง — { $reason }
agent-fallback-download = { $reason } เอเจนต์ในเครื่องรับช่วงต่อได้เมื่อดาวน์โหลดโมเดล ({ $model }, { $gigabytes } GB) เสร็จแล้ว ดาวน์โหลดตอนนี้เลยไหม
agent-fallback-unusable = ไม่มีเอเจนต์ใดทำงานได้ เอเจนต์หลัก: { $main } เอเจนต์ในเครื่อง: { $local }
local-agent-no-route = เอเจนต์ในเครื่องไม่เข้าใจว่าต้องทำอะไร ลองพูดด้วยวิธีอื่น
local-agent-tool-limit = เอเจนต์ในเครื่องหยุดหลังจาก { $n } ขั้นตอนโดยยังทำไม่เสร็จ
local-agent-invalid-call = เอเจนต์ในเครื่องใช้เครื่องมือไม่ถูกต้อง
local-agent-engine-failed = โมเดลของเอเจนต์ในเครื่องล้มเหลว
local-agent-retry = ลองอีกครั้ง หรือเลือกเอเจนต์ตัวอื่นในแผงเอเจนต์
agent-stopped = เอเจนต์หยุดทำงานแล้ว
agent-failed = เอเจนต์ปฏิเสธคำขอ: { $message }
agent-error-work-dir = ไม่สามารถเตรียมโฟลเดอร์ทำงานส่วนตัวของเอเจนต์สำหรับแท็บนี้ได้
agent-tool-failed = เครื่องมือทำงานล้มเหลวโดยไม่คาดคิด

agent-turn-tools = เครื่องมือ { $tools } รายการ
agent-turn-notes = บันทึก { $notes } รายการ
agent-turn-secs = { $secs } วินาที
agent-turn-stopped = หยุดแล้ว
agent-turn-interrupted = ถูกขัดจังหวะ
agent-memory-lost = เอเจนต์จำการสนทนานี้ไม่ได้แล้ว จึงเริ่มต้นใหม่
agent-turn-max-tokens = เอเจนต์ถึงขีดจำกัดความยาวแล้ว
agent-turn-max-turns = เอเจนต์ถึงขีดจำกัดจำนวนขั้นตอนแล้ว
agent-turn-refusal = เอเจนต์ปฏิเสธ
agent-turn-ended = จบรอบ ({ $stop })
agent-choice-cancelled = ยกเลิกคำถามแล้ว
agent-choice-answered = ✓ { $answer }
agent-report-why = ทำไม
agent-trace = { $name }: { $count ->
   *[other] การเปลี่ยนแปลง { $count } รายการ
}
agent-trace-data = { $name }: เปลี่ยนข้อมูลแล้ว
agent-trace-removed = { $name }: ไม่มีอยู่แล้ว
agent-trace-more = { $count ->
   *[other] และอีก { $count } รายการ
}
agent-trace-title = องค์ประกอบที่เปลี่ยนแปลง

agent-error-file = ไม่สามารถอ่านหรือเขียนไฟล์ได้
agent-error-unexpected-action = การกระทำที่ไม่คาดคิด: { $action }
agent-error-unknown-surface = การ์ดนี้ไม่ได้เปิดอยู่แล้ว
agent-error-expired = การกระทำของรอบที่ผ่านมาหมดอายุแล้ว
agent-error-incomplete = การกระทำของการ์ดนี้ไม่สมบูรณ์
agent-error-no-field = การตั้งค่านี้ไม่มีอยู่แล้ว
agent-error-no-variant = รูปแบบนี้ไม่มีอยู่แล้ว
agent-error-arguments = อาร์กิวเมนต์ของเครื่องมือไม่ถูกต้อง ({ $detail }): โปรดตรวจสอบตามสคีมาของเครื่องมือ
agent-error-unknown-tool = ไม่รู้จักเครื่องมือ: { $name }
agent-error-pack-id = นี่ไม่ใช่ id ของแพ็ก: เรียก packs เพื่อดูรายการแพ็กและ id ของแต่ละแพ็ก
agent-error-asset-unknown = รูปภาพนี้ไม่ได้อยู่ในงานนำเสนอและไม่อยู่ในรูปภาพที่วางไว้: ใช้รหัสของรูปภาพที่วางไว้
agent-error-denied = ผู้ใช้ปฏิเสธ
agent-error-no-turn = ไม่มีรอบที่กำลังดำเนินอยู่: { $tool } จะแสดงเฉพาะระหว่างรอบเท่านั้น
agent-error-tab-no-turn = เปิด แสดง หรือย้ายแท็บเฉพาะตอนตอบผู้ใช้เท่านั้น
agent-error-tab-limit = รอบนี้คุณเปิด แสดง หรือย้ายแท็บไปแล้ว ทำต่อที่นั่น หรือถามผู้ใช้ก่อนแท็บอื่น
agent-error-continue-in-other-tab = คำขอนี้ดำเนินต่อในแท็บที่คุณเปิดหรือแสดง โปรดบอกผู้ใช้ให้ทำต่อที่นั่น แท็บนี้จะไม่แก้ไขวิดเจ็ตใดในตอนนี้
agent-error-pack-open-in-tab = แพ็กนี้กำลังถูกแก้ไขในอีกแท็บหนึ่ง โปรดบอกผู้ใช้ให้ทำให้เสร็จที่นั่นก่อน
agent-pack-opened-in-tab = { $shown ->
   [yes] แพ็กเปิดในแท็บ { $tab } และกำลังแสดงอยู่ตอนนี้
  *[no] แพ็กเปิดในแท็บ { $tab } อยู่ด้านหลัง: หน้าต่างของผู้ใช้กำลังแสดงแท็บอื่น
} { $draft ->
   [set] คำขอรออยู่ในแบบร่างของแท็บนั้น โปรดบอกผู้ใช้ให้ตรวจดูที่นั่นก่อนแล้วจึงส่ง
   [kept] แบบร่างของแท็บนั้นมีเนื้อหาอยู่แล้ว จึงคงไว้ตามเดิม โปรดบอกผู้ใช้ให้ทำต่อที่นั่น เพราะเอเจนต์ของแท็บนั้นไม่รู้จักคำขอนี้
   [too_long] คำขอยาวเกินไปสำหรับแบบร่างของแท็บนั้น โปรดบอกผู้ใช้ให้ทำต่อที่นั่น เพราะเอเจนต์ของแท็บนั้นไม่รู้จักคำขอนี้
   [unavailable] ไม่สามารถส่งคำขอไปยังแบบร่างของแท็บนั้นได้ โปรดบอกผู้ใช้ให้ทำต่อที่นั่น เพราะเอเจนต์ของแท็บนั้นไม่รู้จักคำขอนี้
  *[none] โปรดบอกผู้ใช้ให้ทำต่อที่นั่น เพราะเอเจนต์ของแท็บนั้นไม่รู้จักคำขอนี้
} แท็บนี้ยังคงเหมือนเดิม
agent-theme-opened-in-tab = { $shown ->
   [yes] ธีมเปิดในแท็บ { $tab } และกำลังแสดงอยู่ตอนนี้
  *[no] ธีมเปิดในแท็บ { $tab } อยู่ด้านหลัง: หน้าต่างของผู้ใช้กำลังแสดงแท็บอื่น
} { $draft ->
   [set] คำขอรออยู่ในแบบร่างของแท็บนั้น โปรดบอกผู้ใช้ให้ตรวจดูที่นั่นก่อนแล้วจึงส่ง
   [kept] แบบร่างของแท็บนั้นมีเนื้อหาอยู่แล้ว จึงคงไว้ตามเดิม โปรดบอกผู้ใช้ให้ทำต่อที่นั่น เพราะเอเจนต์ของแท็บนั้นไม่รู้จักคำขอนี้
   [too_long] คำขอยาวเกินไปสำหรับแบบร่างของแท็บนั้น โปรดบอกผู้ใช้ให้ทำต่อที่นั่น เพราะเอเจนต์ของแท็บนั้นไม่รู้จักคำขอนี้
   [unavailable] ไม่สามารถส่งคำขอไปยังแบบร่างของแท็บนั้นได้ โปรดบอกผู้ใช้ให้ทำต่อที่นั่น เพราะเอเจนต์ของแท็บนั้นไม่รู้จักคำขอนี้
  *[none] โปรดบอกผู้ใช้ให้ทำต่อที่นั่น เพราะเอเจนต์ของแท็บนั้นไม่รู้จักคำขอนี้
} แท็บนี้ยังคงเหมือนเดิม
agent-error-turn-over = รอบของการเรียกนี้สิ้นสุดแล้ว: ไม่มีสิ่งใดแสดง
agent-error-no-box-turn = ไม่มีรอบให้ผูกกล่องนี้: ไม่มีสิ่งใดแสดง
agent-error-no-file = งานนำเสนอยังไม่มีไฟล์: โปรดระบุพาธแบบสัมบูรณ์
agent-error-path-relative = พาธต้องเป็นแบบสัมบูรณ์
agent-error-path-parent = พาธต้องไม่มี “..”
agent-error-setting-locked = เอเจนต์ไม่สามารถเปลี่ยนการตั้งค่า { $key } ได้
agent-error-setting = ไม่สามารถเปลี่ยนการตั้งค่า { $key } ได้: โปรดตรวจสอบค่าของมัน
agent-error-history-index = การเลื่อนดูประวัติต้องระบุลำดับของเวอร์ชัน
agent-error-no-pack = งานนำเสนอนี้ไม่ได้ปักหมุดแพ็กใด โปรดระบุ pack (ดู packs)
agent-error-history-not-opened = แท็บไม่ได้เปิดประวัติทันเวลา โปรดขอใหม่
agent-error-locale = ไม่รู้จักรหัสภาษา โปรดระบุรหัสภาษาที่มีให้หรือ system
agent-error-empty-interface = ไม่มีอะไรให้เปลี่ยน โปรดระบุ scheme, locale, guides, folded, pile, notes หรือ code
agent-error-layout-range = { $field } ต้องอยู่ระหว่าง { $min } ถึง { $max } px
agent-error-layout-min = { $field } ต้องมีอย่างน้อย { $min } px
agent-error-empty-dictation = ไม่มีอะไรให้เปลี่ยน: โปรดระบุการตั้งค่าการเขียนตามคำบอกอย่างน้อยหนึ่งอย่าง
agent-error-empty-speech = ไม่มีอะไรต้องเปลี่ยน: ระบุการตั้งค่าการอ่านออกเสียงอย่างน้อยหนึ่งรายการ
agent-error-not-shown = แท็บนี้ไม่ใช่แท็บที่แสดงในหน้าต่าง โปรดขอใหม่เมื่อผู้ใช้แสดงแท็บนี้
agent-error-not-presenting = เด็คนี้ไม่ได้กำลังนำเสนออยู่ที่นี่: เริ่มการนำเสนอด้วยการกระทำ start ก่อน
agent-error-present-slide = การแสดงสไลด์ต้องใช้ id ของสไลด์ในเด็คนี้ (ดู deck_outline)
agent-error-undo-open-turn = undo_turn ไม่เลิกทำรอบที่กำลังทำอยู่ ใช้ undo สำหรับการเปลี่ยนแปลงของรอบนี้
agent-error-app-unavailable = มีเพียงแอป Oku Slides ที่ทำสิ่งนี้ได้ ที่นี่ใช้ไม่ได้
agent-error-app-closing = Oku Slides กำลังปิดอยู่แล้ว
agent-error-app-no-offer = ที่นี่ไม่มีการอัปเดตให้เลื่อนหรือติดตั้ง: ตรวจหาการอัปเดตก่อน
agent-error-app-failed = การอัปเดต Oku Slides ล้มเหลว: ตรวจสอบการเชื่อมต่อ หรือใช้แถบการอัปเดต
agent-error-after-turn-taken = คำตอบนี้จะจบด้วยการออกจากแอป การติดตั้งการอัปเดต หรือการปิดแท็บของตัวเองอยู่แล้ว
agent-error-no-pack-update = แผงแพ็กไม่ได้เสนอการอัปเดตแบบนี้สำหรับแพ็กนี้: ดู show_packs
agent-confirm-update-install = ติดตั้ง Oku Slides { $version } ไหม แอปจะรีสตาร์ทเมื่อคำตอบนี้จบ
agent-after-turn-quit-unsaved = Oku Slides ไม่ได้ปิด: มีแท็บที่มีการเปลี่ยนแปลงที่ยังไม่ได้บันทึกตั้งแต่ที่คุณขอ
agent-error-workspace-unavailable = มีเพียงแอป Oku Slides ที่เปลี่ยนแท็บและหน้าต่างได้ ที่นี่ใช้ไม่ได้
agent-error-workspace-unknown-tab = หน้าต่างนี้ไม่มีแท็บหรือหน้าต่างนั้น
agent-error-workspace-unknown-window = ไม่มีหน้าต่างนั้น
agent-error-workspace-index = ตำแหน่งนั้นเลยท้ายแถบแท็บไปแล้ว
agent-error-workspace-closing = Oku Slides กำลังปิด: แท็บและหน้าต่างไม่เปลี่ยนอีกแล้ว
agent-confirm-close-tab = ปิดแท็บ “{ $tab }” ไหม
agent-confirm-discard-tab = ปิดแท็บ “{ $tab }” ไหม การเปลี่ยนแปลงที่ยังไม่ได้บันทึกของแท็บนี้จะหายไป
agent-confirm-close-window = ปิดหน้าต่างนี้และแท็บของหน้าต่างไหม
agent-confirm-discard-window = ปิดหน้าต่างนี้ไหม การเปลี่ยนแปลงที่ยังไม่ได้บันทึกใน { $tabs } จะหายไป
agent-error-window-last = นี่เป็นหน้าต่างเดียว: ให้ปิดแท็บของหน้าต่างแทน หรือออกจาก Oku Slides
agent-error-tab-index = บอกว่าจะวางแท็บไว้ที่ไหน: การจัดลำดับใหม่ต้องมีตำแหน่ง
agent-error-tab-unused = มีเพียงการจัดลำดับใหม่และการย้ายที่รับตำแหน่ง และมีเพียงการย้ายที่รับหน้าต่าง
agent-after-turn-close-unsaved = ไม่ได้ปิดแท็บ: แท็บเปลี่ยนไปตั้งแต่ที่คุณขอ
agent-after-turn-close-not-shown = ไม่ได้ปิดแท็บ: เมื่อคำตอบจบ หน้าต่างของแท็บกำลังแสดงแท็บอื่นอยู่
agent-confirm-quit = ออกจาก Oku Slides ไหม แอปจะปิดเมื่อคำตอบนี้จบ
agent-error-quit-unsaved = แท็บที่มีงานยังไม่บันทึก: { $count } Oku Slides ยังไม่ได้ออก: ผู้ใช้ต้องบันทึกหรือปิดแท็บเหล่านั้นก่อน
agent-error-tune-unavailable = เริ่มการปรับละเอียดไม่ได้: เอเจนต์ไม่พร้อมใช้งาน
agent-error-tune-not-started = การปรับละเอียดไม่ได้เริ่ม: เอเจนต์ของแท็บนั้นยังใช้ไม่ได้ในตอนนี้ แท็บบอกเหตุผลไว้
agent-confirm-import = นำเข้า { $path } ในแท็บใหม่หรือไม่
agent-import-opened-in-tab = { $shown ->
   [yes] งานนำเสนอเปิดในแท็บ { $tab } และแสดงอยู่ตอนนี้
  *[no] งานนำเสนอเปิดในแท็บ { $tab } อยู่ด้านหลัง: หน้าต่างของผู้ใช้แสดงแท็บอื่นอยู่
} { $tune ->
   [started] การปรับละเอียดเริ่มที่นั่น ในบทสนทนาของแท็บนั้นเอง
   [unavailable] เริ่มการปรับละเอียดไม่ได้: บอกผู้ใช้ให้ขอในแท็บนั้น
  *[no] บอกผู้ใช้ให้ทำต่อที่นั่น
} แท็บนี้คงเดิม
agent-error-import-hifi-target = การนำเข้าแบบไฮไฟไม่รับแพ็กหรือธีม: เอาออก หรือนำเข้าในโหมด retrofit
agent-error-import-extension = นำเข้าได้เฉพาะไฟล์ PowerPoint .pptx
agent-error-a2ui-unknown = { $name } ไม่ใช่คอมโพเนนต์ที่ render รับ คอมโพเนนต์: { $known }
agent-error-a2ui-elsewhere = render ไม่แสดง { $name }: ใช้ { $tool }
agent-error-a2ui-host-only = { $name } สงวนไว้สำหรับแอป: render แสดงไม่ได้
agent-error-themes-deck-mode = ตัวเลือกธีมเปิดได้เฉพาะขณะแก้ไขสไลด์เท่านั้น
agent-error-packs-deck-mode = แผงแพ็กของงานนำเสนอเปิดได้เฉพาะขณะแก้ไขสไลด์เท่านั้น
agent-error-usage-no-deck = การใช้งานของแท็บนี้ไม่ได้เป็นของงานนำเสนอใด: เฉพาะแท็บแพ็กหรือแท็บธีมที่เปิดจากงานนำเสนอ (โดยผู้ใช้หรือเอเจนต์) หรือแพ็กของงานนำเสนอนั้นเอง เท่านั้นที่นับรวมในงานนำเสนอนั้น
agent-error-rewrite-text = การเรียบเรียงใหม่ส่งกลับมาโดยไม่มีข้อความ
agent-error-paragraphs = ย่อหน้าของการเรียบเรียงใหม่ไม่ถูกต้อง
agent-error-draft-too-long = ฉบับร่างยาวเกินไป
agent-error-report-elsewhere = หน้าต่างนี้ไม่ได้แสดงแท็บนั้น
agent-error-ui-too-large = บริบทของหน้ามีขนาดใหญ่เกินไป
agent-error-seq-exhausted = หมายเลขรายงานของหน้าอยู่นอกช่วง
agent-error-draft-origin = ปฏิเสธฉบับร่าง: ไม่รู้จักมุมมองนี้
agent-error-draft-no-agent = เอเจนต์ของแท็บนี้ยังไม่เริ่มทำงาน: ปฏิเสธฉบับร่าง
agent-error-too-many-images = แบบร่างมีรูปภาพครบแล้ว
agent-error-image-not-base64 = รูปภาพที่วางมาไม่ครบสมบูรณ์
agent-error-image-gone = รูปภาพที่วางไว้ใช้ไม่ได้แล้ว: โปรดวางอีกครั้ง
agent-error-unknown-agent = เอเจนต์นี้ไม่อยู่ในการกำหนดค่าแล้ว
agent-error-agent-locked = การเลือกเอเจนต์ถูกกำหนดโดยสภาพแวดล้อม
agent-error-agent-not-saved = ไม่สามารถบันทึกการเลือกเอเจนต์ได้
agent-error-unknown-model = เอเจนต์นี้ไม่มีโมเดลนั้นให้เลือก
agent-error-model-not-saved = ไม่สามารถบันทึกการเลือกโมเดลได้
voice-error-no-dictation = ไม่สามารถใช้การป้อนด้วยเสียงที่นี่
voice-error-busy = กำลังป้อนด้วยเสียงอยู่: ลองอีกครั้งเมื่อหยุดแล้ว
voice-error-unknown-model = ไม่รู้จักโมเดลการป้อนด้วยเสียง
voice-error-unavailable = โมเดลนี้ไม่รองรับภาษาที่เลือก
voice-error-installed = ติดตั้งโมเดลนี้แล้ว
voice-error-not-downloadable = โมเดลนี้ไม่ต้องดาวน์โหลด
voice-error-nothing-to-cancel = ไม่มีการดาวน์โหลดที่กำลังทำงาน
voice-error-bad-silence = ความเงียบต้องยาว 0.5 ถึง 10 วินาที
voice-error-bad-language = ไม่รู้จักภาษาของการป้อนด้วยเสียง
voice-error-bad-url = ที่อยู่ API ต้องขึ้นต้นด้วย https:// (http:// สำหรับคอมพิวเตอร์เครื่องนี้เท่านั้น) โดยไม่มีชื่อผู้ใช้ พารามิเตอร์ หรืออักขระพิเศษ
voice-error-bad-key-env = ตัวแปรของคีย์ต้องเป็นชื่ออย่าง OPENAI_API_KEY: ตัวพิมพ์ใหญ่ ตัวเลข และ _
voice-error-bad-api-model = ชื่อโมเดล API ใช้ได้เฉพาะตัวอักษร ตัวเลข และ . _ : / - ส่วนชื่อการปรับใช้ของ Azure ใช้ได้เฉพาะตัวอักษร ตัวเลข และ . _ -
voice-error-bad-api-version = เวอร์ชัน API ของ Azure ใช้ได้เฉพาะตัวอักษร ตัวเลข . และ - ไม่เกิน 32 อักขระ เช่น 2025-03-01-preview
voice-error-bad-api-headers = ส่วนหัวเขียนแบบ Name: value; Name: $VARIABLE ไม่เกิน 8 รายการ Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection และชื่อซ้ำจะถูกปฏิเสธ
voice-error-api-reserved = มีเพียงผู้ใช้ที่ตั้งค่าที่อยู่ ตัวแปรของคีย์ เวอร์ชัน Azure และส่วนหัวของ API ได้: เปิดกล่องด้วย show_dictation แล้วขอให้ผู้ใช้ตั้งค่า
voice-error-hands-free-reserved = มีเพียงผู้ใช้เท่านั้นที่เปิดการฟังแบบแฮนด์ฟรีได้: เปิดกล่องด้วย show_dictation แล้วขอให้ผู้ใช้ทำ
voice-error-changed = การตั้งค่าเปลี่ยนไปหลังคำถาม: โปรดถามอีกครั้ง
voice-error-nothing-to-download = ไม่มีอะไรให้ดาวน์โหลดสำหรับเอเจนต์นี้
voice-error-bad-rate = ความเร็วในการพูดต้องอยู่ระหว่าง 0.5 ถึง 2
voice-error-unknown-voice = ไม่รู้จักเสียงสำหรับภาษาของอินเทอร์เฟซ
voice-error-speech-unavailable = ไม่สามารถใช้การอ่านออกเสียงบนคอมพิวเตอร์เครื่องนี้
agent-tune-prompt = ปรับแต่งงานนำเสนอที่นำเข้าอย่างละเอียด
agent-settings-unknown-selected = การตั้งค่า agent.selected = { $selected } ไม่ตรงกับเอเจนต์ใดใน [agent.presets] (ที่รู้จัก: { $known })
agent-settings-preset-launch = เอเจนต์ [agent.presets.{ $id }] ต้องมี command หรือ builtin อย่างใดอย่างหนึ่งเท่านั้น
