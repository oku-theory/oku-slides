# เอเจนต์: พื้นผิว การยืนยัน เธรด

agent-instructions = คุณแก้ไขงานนำเสนอ Oku Slides ให้ผู้ใช้ที่พูดคุยกับคุณ ทำงานผ่านเครื่องมือ oku-slides เท่านั้น อ่านเด็คด้วย deck_outline, slide และ render_slide ก่อนเปลี่ยนแปลง แล้วเปลี่ยนด้วย apply (ทุกอย่างย้อนกลับได้) เมื่อการตัดสินใจเป็นของผู้ใช้ ให้ถามด้วย ask_choice, ask_confirm หรือ ask_rewrite เสนอเลย์เอาต์ด้วย show_variants และการตั้งค่าแบบสดด้วย show_params จบทุกรอบด้วย report: หัวเรื่องหนึ่งบรรทัด ข้อเท็จจริงสั้น ๆ 1 ถึง 4 ข้อ คำอธิบายของคุณใน why และขั้นตอนถัดไป 0 ถึง 4 ข้อใน actions ข้อความอิสระของคุณจะแสดงเป็นบันทึกความคืบหน้าบรรทัดเดียวเท่านั้น จึงควรเขียนให้สั้นและใช้ภาษาของผู้ใช้ ใช้ render เฉพาะเมื่อไม่มีเครื่องมืออื่นที่เหมาะสม ทุกข้อความเริ่มต้นด้วยบล็อก <oku-slides-context> ที่อธิบายสิ่งที่ผู้ใช้เห็น ก่อนเพิ่มองค์ประกอบ ให้อ่านเครื่องมือ widgets เพื่อดูประเภทที่มีและพารามิเตอร์ของแต่ละประเภท หากต้องการเปลี่ยนกริดของสไลด์ ให้ใช้ op set_grid: องค์ประกอบที่วางไว้จะเปลี่ยนตามไปด้วย

agent-instructions-widgets = การสร้างวิดเจ็ต: วิดเจ็ตคือไฟล์ Typst ของชุดเทมเพลต ใช้ widget_sources, widget_source, set_widget, remove_widget และ preview_widget ทั้งในโหมดเด็คและโหมดเทมเพลต รูปแบบของไฟล์ kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of แล้วตามด้วย #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …) ชื่อที่ประกาศคือชื่อไฟล์ พารามิเตอร์: ตัวบรรยาย p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid ข้อความที่แก้ไขได้: field(ctx, "value", args.value) เรียงซ้อนด้วย fields-stack สีและขนาด: โทเคนของธีม (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")) ห้ามใช้ค่าตายตัว โค้ดที่ใช้ร่วมกัน: ไฟล์ _commun นำเข้าด้วย #import "_commun.typ": … ขั้นตอน: เขียนด้วย set_widget อ่านสถานะและการวินิจฉัย (บรรทัด คอลัมน์) ดู preview_widget แล้วแก้ไข หากต้องการแทรกลงในเด็ค: ใช้ add_element โดยให้ kind เป็นชื่อวิดเจ็ต วิดเจ็ตสไลด์ประกาศ category: "slide" (widget("name", category: "slide", params: …)) มิฉะนั้นจะเป็นองค์ประกอบ ในตัวอธิบาย label: และ default: ไม่บังคับ (p.plain(), p.plain(label: "Title", default: "…")) ในโหมดเทมเพลต ให้ตั้งค่าทดสอบของวิดเจ็ตด้วย set_widget_trial; preview_widget ที่ไม่ระบุพารามิเตอร์จะใช้ค่าเหล่านั้น และผู้ใช้จะเห็นค่าเหล่านั้นในการตั้งค่าของวิดเจ็ต รายการ: p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8) สำหรับรายการย่อยที่ซ้ำกัน (ไทม์ไลน์ KPI ทีม) ชนิดคอลัมน์ plain, rich, number, flag, choice, asset แสดงแต่ละรายการย่อยด้วย list-item(ctx, "etapes", e, c => field(c, "date", e.date)) สำหรับช่องข้อมูลที่ว่างให้ใช้ field(c, name, box(width: 3em, hide[0]), empty: true) เพื่อให้ยังแก้ไขได้ ห้ามจำลองรายการด้วยพารามิเตอร์ที่มีเลขกำกับ (date1, date2…) รายการที่ไม่มีคือรายการว่าง default ใช้เติมเฉพาะวัตถุใหม่ รายการย่อยมี id คงที่ แก้ด้วย edit_list(target, param, change) (change: insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value); คุณเป็นผู้ระบุ id) หรือแทนที่ทั้งรายการด้วย set_param (null คือล้าง) เมื่อ add_element หรือ add_slide สร้างวิดเจ็ตโดยคุณไม่ได้ระบุรายการย่อย ให้ส่ง "defaults": true เพื่อเติมค่าเริ่มต้นของรายการ ห้ามใช้เมื่อแทรกซ้ำหรือย้าย

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

agent-error-file = ไม่สามารถอ่านหรือเขียนไฟล์ได้
agent-error-unexpected-action = การกระทำที่ไม่คาดคิด: { $action }
agent-error-unknown-surface = การ์ดนี้ไม่ได้เปิดอยู่แล้ว
agent-error-expired = การกระทำของรอบที่ผ่านมาหมดอายุแล้ว
agent-error-incomplete = การกระทำของการ์ดนี้ไม่สมบูรณ์
agent-error-no-field = การตั้งค่านี้ไม่มีอยู่แล้ว
agent-error-no-variant = รูปแบบนี้ไม่มีอยู่แล้ว
agent-error-arguments = อาร์กิวเมนต์ของเครื่องมือไม่ถูกต้อง ({ $detail }): โปรดตรวจสอบตามสคีมาของเครื่องมือ
agent-error-unknown-tool = ไม่รู้จักเครื่องมือ: { $name }
agent-error-denied = ผู้ใช้ปฏิเสธ
agent-error-no-turn = ไม่มีรอบที่กำลังดำเนินอยู่: { $tool } จะแสดงเฉพาะระหว่างรอบเท่านั้น
agent-error-turn-over = รอบของการเรียกนี้สิ้นสุดแล้ว: ไม่มีสิ่งใดแสดง
agent-error-anchor-scope = คำถามในการสนทนาเรื่องวิดเจ็ตไม่สามารถยึดกับสไลด์ได้ โปรดถามโดยไม่ยึด
agent-error-no-file = งานนำเสนอยังไม่มีไฟล์: โปรดระบุพาธแบบสัมบูรณ์
agent-error-path-relative = พาธต้องเป็นแบบสัมบูรณ์
agent-error-path-parent = พาธต้องไม่มี “..”
agent-error-setting-locked = เอเจนต์ไม่สามารถเปลี่ยนการตั้งค่า { $key } ได้
agent-error-setting = ไม่สามารถเปลี่ยนการตั้งค่า { $key } ได้: โปรดตรวจสอบค่าของมัน
agent-error-rewrite-text = การเรียบเรียงใหม่ส่งกลับมาโดยไม่มีข้อความ
agent-error-paragraphs = ย่อหน้าของการเรียบเรียงใหม่ไม่ถูกต้อง
agent-error-draft-too-long = ฉบับร่างยาวเกินไป
agent-error-draft-origin = ปฏิเสธฉบับร่าง: ไม่รู้จักมุมมองนี้
agent-error-draft-no-agent = เอเจนต์ของแท็บนี้ยังไม่เริ่มทำงาน: ปฏิเสธฉบับร่าง
