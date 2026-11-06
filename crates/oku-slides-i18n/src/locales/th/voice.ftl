# Spoken commands: comma-separated forms, matched whole, never with placeables.

voice-yes = ใช่, ครับ, ค่ะ, ได้, ตกลง, โอเค, ok, แน่นอน, ยืนยัน, ถูกต้อง
voice-no = ไม่, ไม่ใช่, ไม่เอา, ยกเลิก, ปฏิเสธ
voice-negation = อย่า, ห้าม, ไม่เคย, ไม่ต้อง

voice-stop = หยุด, พอ, พอแล้ว, เงียบ, หยุดฟัง, สต็อป
voice-ordinal-1 = ที่หนึ่ง, แรก, หนึ่ง, อันแรก
voice-ordinal-2 = ที่สอง, สอง, อันที่สอง
voice-ordinal-3 = ที่สาม, สาม
voice-ordinal-4 = ที่สี่, สี่
voice-ordinal-5 = ที่ห้า, ห้า
voice-ordinal-6 = ที่หก, หก
voice-ordinal-7 = ที่เจ็ด, เจ็ด
voice-ordinal-8 = ที่แปด, แปด
voice-ordinal-9 = ที่เก้า, เก้า

voice-next = ถัดไป, ต่อไป, สไลด์ถัดไป, สไลด์ต่อไป, ไปข้างหน้า, ไปต่อ
voice-previous = ก่อนหน้า, สไลด์ก่อนหน้า, ย้อนกลับ, ถอยกลับ, กลับไป
voice-first = สไลด์แรก, ไปสไลด์แรก, กลับไปเริ่มต้น, เริ่มต้น, ต้น
voice-last = สไลด์สุดท้าย, สไลด์ท้ายสุด, สุดท้าย, ท้ายสุด, ไปท้ายสุด, จบ
voice-exit = ออก, ออกจากการนำเสนอ, ออกจากพรีเซนต์, เอสเคป, esc

voice-wake = oku, hey oku, โอคุ, เฮ้ โอคุ, สวัสดี โอคุ
voice-filler = กรุณา, ช่วย, ขอบคุณ, เอ่อ, อืม, ตอนนี้, งั้น, หน่อย
voice-option = ตัวเลือก, หมายเลข, ข้อ

voice-verb-replace = แทนที่, เขียนทับ, ทับ
voice-verb-merge = รวม, รวมกัน, ผสาน, เมิร์จ
voice-verb-discard = ทิ้ง, ละทิ้ง, ทิ้งไป, ทิ้งการเปลี่ยนแปลง
voice-verb-delete = ลบ, ลบออก, ลบทิ้ง, เอาออก
voice-verb-use-api = ใช้ api, ใช้ เอพีไอ, ใช้เอพีไอ, ใช้ api แทน
voice-verb-install = ติดตั้ง, อัปเดต, อัพเดต, ติดตั้งอัปเดต
voice-verb-quit = ปิดแอป, ปิดโปรแกรม, ออกจากแอป, ปิดแอปพลิเคชัน
voice-verb-close = ปิด, ปิดแท็บ
voice-verb-import = นำเข้า, อิมพอร์ต, นำเข้าไฟล์

voice-say-question = { $question }
voice-say-option = ตัวเลือก { $n }: { $option }
voice-say-confirm = หากต้องการยืนยัน ให้พูดว่า "{ $verb }" หรือพูดว่าไม่
voice-say-confirm-yes = หากต้องการยืนยัน ให้พูดว่าใช่ หรือพูดว่าไม่
voice-say-confirm-action = หากต้องการ{ $action } ให้พูดว่าใช่ หรือพูดว่าไม่
voice-action-open = เปิดไฟล์
voice-action-save = บันทึกงานนำเสนอ
voice-action-save-copy = บันทึกสำเนา
voice-action-move = ย้ายงานนำเสนอ
voice-action-new-deck = สร้างงานนำเสนอใหม่
voice-action-copy-pack = คัดลอกแพ็ก
voice-action-copy-theme = คัดลอกธีม
voice-action-edit-theme = แก้ไขธีม
voice-action-change-setting = เปลี่ยนการตั้งค่า
voice-action-change-dictation = เปลี่ยนการป้อนด้วยเสียง
voice-action-download = ดาวน์โหลดโมเดล
voice-action-select-agent = เปลี่ยนเอเจนต์
voice-say-permission = เอเจนต์ขออนุญาต: พูดว่า "ใช่" เพื่ออนุญาตหนึ่งครั้ง หรือ "ไม่"
voice-say-agent-refused = เอเจนต์ปฏิเสธคำขอ
voice-say-agent-stopped = เอเจนต์หยุดทำงานแล้ว
voice-say-agent-no-start = เอเจนต์เริ่มทำงานไม่ได้
voice-say-agent-no-tools = เครื่องมือของแท็บนี้ใช้งานไม่ได้
