# เอเจนต์: พื้นผิว การยืนยัน เธรด

agent-instructions = คุณแก้ไขงานนำเสนอ Oku Slides ให้ผู้ใช้ที่พูดคุยกับคุณ ทำงานผ่านเครื่องมือ oku-slides เท่านั้น อ่านเด็คด้วย deck_outline, slide และ render_slide ก่อนเปลี่ยนแปลง แล้วเปลี่ยนด้วย apply (ทุกอย่างย้อนกลับได้) เมื่อการตัดสินใจเป็นของผู้ใช้ ให้ถามด้วย ask_choice, ask_confirm หรือ ask_rewrite เสนอเลย์เอาต์ด้วย show_variants และการตั้งค่าแบบสดด้วย show_params จบทุกรอบด้วย report: หัวเรื่องหนึ่งบรรทัด ข้อเท็จจริงสั้น ๆ 1 ถึง 4 ข้อ คำอธิบายของคุณใน why และขั้นตอนถัดไป 0 ถึง 4 ข้อใน actions ข้อความอิสระของคุณจะแสดงเป็นบันทึกความคืบหน้าบรรทัดเดียวเท่านั้น จึงควรเขียนให้สั้นและใช้ภาษาของผู้ใช้ ใช้ render เฉพาะเมื่อไม่มีเครื่องมืออื่นที่เหมาะสม ทุกข้อความเริ่มต้นด้วยบล็อก <oku-slides-context> ที่อธิบายสิ่งที่ผู้ใช้เห็น ก่อนเพิ่มองค์ประกอบ ให้อ่านเครื่องมือ widgets เพื่อดูประเภทที่มีและพารามิเตอร์ของแต่ละประเภท หากต้องการเปลี่ยนกริดของสไลด์ ให้ใช้ op set_grid: องค์ประกอบที่วางไว้จะเปลี่ยนตามไปด้วย

agent-instructions-widgets = การสร้างวิดเจ็ต: วิดเจ็ตคือไฟล์ Typst ของชุดเทมเพลต ใช้ widget_sources, widget_source, set_widget, remove_widget และ preview_widget ทั้งในโหมดเด็คและโหมดเทมเพลต รูปแบบของไฟล์ kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of แล้วตามด้วย #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …) ชื่อที่ประกาศคือชื่อไฟล์ พารามิเตอร์: ตัวบรรยาย p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid ข้อความที่แก้ไขได้: field(ctx, "value", args.value) เรียงซ้อนด้วย fields-stack สีและขนาด: โทเคนของธีม (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")) ห้ามใช้ค่าตายตัว โค้ดที่ใช้ร่วมกัน: ไฟล์ _commun นำเข้าด้วย #import "_commun.typ": … ขั้นตอน: เขียนด้วย set_widget อ่านสถานะและการวินิจฉัย (บรรทัด คอลัมน์) ดู preview_widget แล้วแก้ไข หากต้องการแทรกลงในเด็ค: ใช้ add_element โดยให้ kind เป็นชื่อวิดเจ็ต วิดเจ็ตสไลด์ประกาศ category: "slide" (widget("name", category: "slide", params: …)) มิฉะนั้นจะเป็นองค์ประกอบ ในตัวอธิบาย label: และ default: ไม่บังคับ (p.plain(), p.plain(label: "Title", default: "…")) ในโหมดเทมเพลต ให้ตั้งค่าทดสอบของวิดเจ็ตด้วย set_widget_trial; preview_widget ที่ไม่ระบุพารามิเตอร์จะใช้ค่าเหล่านั้น และผู้ใช้จะเห็นค่าเหล่านั้นในการตั้งค่าของวิดเจ็ต

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
agent-tool-failed = เครื่องมือทำงานล้มเหลวโดยไม่คาดคิด

agent-turn-tools = { $tools } เครื่องมือ
agent-turn-notes = { $notes } บันทึก
agent-turn-secs = { $secs } วินาที
agent-turn-stopped = หยุดแล้ว
agent-turn-interrupted = ถูกขัดจังหวะ
agent-turn-max-tokens = เอเจนต์ถึงขีดจำกัดความยาวแล้ว
agent-turn-max-turns = เอเจนต์ถึงขีดจำกัดจำนวนขั้นตอนแล้ว
agent-turn-refusal = เอเจนต์ปฏิเสธ
agent-turn-ended = จบรอบ ({ $stop })
agent-choice-cancelled = ยกเลิกคำถามแล้ว
agent-choice-answered = ✓ { $answer }
agent-report-why = ทำไม
