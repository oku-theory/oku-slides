# เอเจนต์: พื้นผิว การยืนยัน เธรด

agent-instructions = คุณแก้ไขงานนำเสนอ Oku Slides ให้ผู้ใช้ที่กำลังพูดกับคุณ กระทำผ่านเครื่องมือ oku-slides เท่านั้น อ่านเด็คด้วย deck_outline, slide และ render_slide ก่อนเปลี่ยนแปลง เปลี่ยนแปลงด้วย apply (ทุกการเปลี่ยนแปลงย้อนกลับได้) เมื่อการตัดสินใจเป็นของผู้ใช้ ให้ถามด้วย ask_choice, ask_confirm หรือ ask_rewrite เสนอเลย์เอาต์ด้วย show_variants และการตั้งค่าแบบสดด้วย show_params ตอบสั้น ๆ เป็นภาษาของผู้ใช้ แต่ละข้อความเริ่มต้นด้วยบล็อก <oku-slides-context> ที่อธิบายสิ่งที่ผู้ใช้เห็น หลังตอบทุกครั้ง หากมีขั้นตอนถัดไปที่เป็นประโยชน์ ให้เสนอ 1 ถึง 4 ขั้นด้วย suggest_next (ป้ายสั้น ๆ คลิกแล้วคำสั่งจะถูกส่งถึงคุณ)

agent-surface-cancel = ยกเลิก
agent-surface-confirm = ยืนยัน
agent-surface-confirm-danger = ใช่ ดำเนินการ
agent-surface-rewrite = การเรียบเรียงใหม่
agent-surface-pick = เลือก { $n }
agent-surface-keep = เก็บต้นฉบับไว้
agent-surface-edit = แก้ไขก่อนนำไปใช้
agent-surface-close = ปิด
agent-surface-variants = ตัวเลือกอื่น
agent-surface-permission-always = ตลอดไป
agent-surface-permission-title = เอเจนต์ขอสิทธิ์
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
agent-confirm-save-templates = บันทึกชุดเทมเพลต “{ $name }” ไหม
agent-confirm-delete-set = ลบชุดเทมเพลต “{ $name }” ไหม การกระทำนี้ไม่สามารถย้อนกลับได้
agent-confirm-setting = เปลี่ยน { $key }: { $old } → { $new } ไหม

agent-not-found = ไม่พบเอเจนต์: { $command } การตั้งค่า [agent] command
agent-no-http = เอเจนต์นี้ไม่สามารถเชื่อมต่อกับเครื่องมือของ Oku Slides ได้
agent-stopped = เอเจนต์หยุดทำงานแล้ว
agent-failed = เอเจนต์ปฏิเสธคำขอ: { $message }
