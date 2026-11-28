op-slide-not-found = ไม่พบสไลด์: { $subject }
op-element-not-found = ไม่พบองค์ประกอบ: { $subject }
op-duplicate-id = ตัวระบุนี้ถูกใช้แล้ว: { $subject }
op-index-out-of-range = ตำแหน่งอยู่นอกช่วง
op-invalid-value = ค่าไม่ถูกต้องสำหรับ { $subject }
op-invalid-value-because = ค่าไม่ถูกต้องสำหรับ { $subject }: { $reason }
op-value-no-value = พารามิเตอร์ไม่มีค่า
op-value-grid-tracks = กริดมี 1 ถึง 24 คอลัมน์และแถว
op-value-bad-item-id = ตัวระบุรายการย่อยคือข้อความที่ไม่ว่างและไม่มี “/”
op-value-missing-item-id = ทุกรายการย่อยต้องมีตัวระบุ
op-value-too-deep = องค์ประกอบซ้อนกันลึกเกินไป
op-value-empty-path = พาธว่างเปล่า
op-value-not-text = ต้องเป็นข้อความหรือ null
op-value-empty-frame = ความกว้างและความสูงต้องมากกว่า 0
op-value-blocks-off-grid = สไลด์จัดวางช่องแบบโฟลว์เอง และกริดจะซ่อนช่องเหล่านั้น โปรดย้ายแต่ละองค์ประกอบไปยังช่องว่างก่อน
op-value-outside-grid = เซลล์อยู่นอกกริด
op-value-not-tokens = ต้องเป็นออบเจ็กต์หรือ null
op-value-not-a-slot = พารามิเตอร์นี้ไม่ใช่ช่อง
op-value-out-of-bounds = ตัวเลขอยู่นอกช่วงที่อนุญาต
op-value-unreadable = ค่าไม่มีรูปแบบตามที่คาดไว้
op-value-valign = ต้องเป็น top, middle หรือ bottom
op-value-path = เส้นทางประกอบด้วยคำสั่ง M, L, C, Q และ Z ตามด้วยตัวเลข โดยเริ่มด้วย M
knowledge-path-empty = เส้นทางว่างเปล่า
knowledge-path-not-absolute = เส้นทางต้องขึ้นต้นด้วย /
knowledge-path-bad-segment = ส่วนหนึ่งว่างเปล่า ลงท้ายด้วยจุด หรือใช้อักขระอื่นนอกจาก a-z, 0-9, จุด, ขีดล่าง และยัติภังค์
knowledge-path-dot-segment = ส่วนของเส้นทางต้องไม่ขึ้นต้นด้วยจุด
knowledge-path-reserved = ส่วนหนึ่งเป็นชื่อที่ระบบสงวนไว้
knowledge-path-too-deep = เส้นทางมีส่วนมากเกินไป
knowledge-path-too-long = เส้นทางยาวเกินไป
knowledge-path-not-markdown = ชื่อไฟล์ต้องลงท้ายด้วย .md
knowledge-frontmatter-missing = ไฟล์ต้องขึ้นต้นด้วยบล็อกส่วนหัวที่คั่นด้วย ---
knowledge-frontmatter-unterminated = บล็อกส่วนหัวไม่มี --- ปิดท้าย
knowledge-frontmatter-too-large = บล็อกส่วนหัวใหญ่เกินไป
knowledge-frontmatter-not-yaml = ส่วนหัวไม่ใช่ YAML ที่ถูกต้อง
knowledge-frontmatter-not-mapping = ส่วนหัวต้องเป็นการจับคู่ที่มีคีย์เป็นข้อความ
knowledge-frontmatter-duplicate-key = คีย์ "{ $key }" ปรากฏสองครั้งในส่วนหัว
knowledge-frontmatter-alias = ส่วนหัวต้องไม่ใช้แองเคอร์หรือนามแฝงของ YAML
knowledge-frontmatter-tag = ส่วนหัวต้องไม่ใช้แท็กของ YAML
knowledge-frontmatter-too-deep = ส่วนหัวซ้อนลึกเกินไป
knowledge-frontmatter-after-document-end = ส่วนหัวมีข้อความหลังจุดสิ้นสุดเอกสาร YAML ...
knowledge-frontmatter-flow-mapping = ส่วนหัวต้องเป็นการแมปแบบบล็อกเดียว คีย์หนึ่งตัวต่อบรรทัด โดยไม่มีจุดสิ้นสุด `...`
knowledge-frontmatter-type-missing = ส่วนหัวต้องมีประเภทที่ไม่ว่างเปล่า
knowledge-frontmatter-bad-timestamp = ค่า "{ $key }" ในส่วนหัวต้องเป็นวันที่และเวลาแบบ RFC 3339 พร้อมส่วนต่างเขตเวลา
knowledge-frontmatter-not-allowed = ไม่อนุญาตให้ใช้คีย์ "{ $key }" ในส่วนหัวของไฟล์นี้
knowledge-reason-not-found = ไม่มีไฟล์ที่พาธนี้
knowledge-reason-exists = มีไฟล์อยู่ที่พาธนี้แล้ว
knowledge-reason-id-missing = ไฟล์ใหม่ต้องมีรหัส
knowledge-reason-id-invalid = รหัสไฟล์คือเลขฐานสิบหกตัวพิมพ์เล็ก 32 หลัก
knowledge-reason-id-taken = ไฟล์อื่นใช้รหัสนี้แล้ว
knowledge-reason-id-mismatch = ไฟล์ที่พาธนี้มีรหัสอื่น
knowledge-reason-edit-empty-old = ข้อความที่จะแทนที่ต้องไม่ว่าง
knowledge-reason-edit-no-match = ไม่พบข้อความที่จะแทนที่ในไฟล์
knowledge-reason-edit-ambiguous = ข้อความที่จะแทนที่ปรากฏในไฟล์มากกว่าหนึ่งครั้ง
knowledge-reason-too-many-files = ชุดนี้มีไฟล์มากเกินไป
knowledge-reason-file-too-large = ไฟล์ใหญ่เกินไป
knowledge-reason-total-too-large = ชุดนี้ใหญ่เกินไป
knowledge-reason-over-limit = ฐานความรู้เกินขีดจำกัดแล้ว: มีเพียงการลบไฟล์เท่านั้นที่ทำให้กลับมาอยู่ในขีดจำกัด
knowledge-no-text-layer = PDF นี้ไม่มีชั้นข้อความ (เป็นภาพสแกน): ส่งออกใหม่พร้อมข้อความ หรือวางข้อความของมัน
knowledge-legacy-format = อ่านรูปแบบ Office รุ่นเก่านี้ไม่ได้: บันทึกเป็น .pptx, .docx หรือ .xlsx
knowledge-document-encrypted = เอกสารนี้ป้องกันด้วยรหัสผ่าน: บันทึกสำเนาที่ไม่มีการป้องกัน
knowledge-unsupported-document = นำเข้าเอกสารประเภทนี้ไม่ได้: PDF, DOCX, PPTX, XLSX, ODS, Markdown หรือข้อความ
knowledge-document-damaged = เอกสารนี้เสียหายและอ่านไม่ได้
knowledge-document-type-mismatch = เนื้อหาของไฟล์นี้ไม่ตรงกับชื่อไฟล์
knowledge-document-too-large = เอกสารนี้ใหญ่เกินกว่าจะนำเข้าได้
op-knowledge = ไฟล์ความรู้ { $path }: { $reason }
op-unknown-widget = ไม่รู้จักวิดเจ็ต: { $subject }
op-unknown-namespace = เด็คนี้ไม่มีแพ็ก { $pack }
op-pack-required = แพ็ก { $pack } จำเป็นสำหรับ { $by } โปรดนำแพ็กนั้นออกก่อน
op-pack-in-use = แพ็ก { $pack } ยังถูกใช้งานโดยรายการ { $count } รายการ
op-pack-requirement-missing = แพ็ก { $pack } ต้องใช้ { $required } ซึ่งเด็คนี้ไม่มี
op-base-required = ทุกเด็คจะเก็บแพ็กวิดเจ็ตพื้นฐานไว้
op-pack-kind-conflict = เด็คนี้มีแพ็ก { $pack } ของอีกประเภทหนึ่งอยู่แล้ว (วิดเจ็ตหรือเลย์เอาต์)
op-theme-cycle = ธีมขยายกันและกันเป็นวง: { $chain }
op-theme-closure = ธีมที่ปักหมุดในเด็คนี้ไม่ใช่ธีมของเด็คและธีมแม่ของมันพอดี ({ $namespace })
op-unknown-param = ไม่รู้จักพารามิเตอร์: { $subject }
op-param-type-because = ค่าไม่ถูกต้องสำหรับ { $subject }: { $reason }
op-param-shape-text = { $nullable ->
    [yes] ต้องเป็นข้อความ หรือ null
   *[no] ต้องเป็นข้อความ
}
op-param-shape-choice = { $nullable ->
    [yes] ต้องเป็นหนึ่งในค่าของมัน หรือ null
   *[no] ต้องเป็นหนึ่งในค่าของมัน
}
op-param-shape-paragraphs = { $nullable ->
    [yes] ต้องเป็นรายการย่อหน้า หรือ null
   *[no] ต้องเป็นรายการย่อหน้า
}
op-param-shape-elements = { $nullable ->
    [yes] ต้องเป็นรายการองค์ประกอบ หรือ null
   *[no] ต้องเป็นรายการองค์ประกอบ
}
op-param-shape-flag = { $nullable ->
    [yes] ต้องเป็นค่าบูลีน หรือ null
   *[no] ต้องเป็นค่าบูลีน
}
op-param-shape-number = { $nullable ->
    [yes] ต้องเป็นตัวเลข หรือ null
   *[no] ต้องเป็นตัวเลข
}
op-param-shape-bounded-number = { $nullable ->
    [yes] ต้องเป็นตัวเลขภายในขอบเขต หรือ null
   *[no] ต้องเป็นตัวเลขภายในขอบเขต
}
op-param-shape-grid = { $nullable ->
    [yes] ต้องเป็นกริด หรือ null
   *[no] ต้องเป็นกริด
}
op-param-shape-chart-option = { $nullable ->
    [yes] ต้องเป็นตัวเลือกของแผนภูมิ หรือ null
   *[no] ต้องเป็นตัวเลือกของแผนภูมิ
}
op-param-shape-table-option = { $nullable ->
    [yes] ต้องเป็นตัวเลือกของตาราง หรือ null
   *[no] ต้องเป็นตัวเลือกของตาราง
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] ต้องเป็นตัวเลือกของแผนภูมิหรือตาราง หรือ null
   *[no] ต้องเป็นตัวเลือกของแผนภูมิหรือตาราง
}
op-param-shape-items = { $nullable ->
    [yes] ต้องเป็นรายการของรายการย่อย หรือ null
   *[no] ต้องเป็นรายการของรายการย่อย
}
op-param-shape-list = { $nullable ->
    [yes] ต้องเป็นรายการ หรือ null
   *[no] ต้องเป็นรายการ
}
op-param-shape-add-element-only = { $nullable ->
    [yes] เพิ่มองค์ประกอบด้วย add_element, หรือ null
   *[no] เพิ่มองค์ประกอบด้วย add_element
}
op-param-shape-set-param-only = { $nullable ->
    [yes] กำหนดค่าด้วย set_param, หรือ null
   *[no] กำหนดค่าด้วย set_param
}
op-param-shape-path-option = { $nullable ->
    [yes] ต้องเป็นเส้นทางรูปร่าง (ออบเจ็กต์ที่มีคีย์ d) หรือ null
   *[no] ต้องเป็นเส้นทางรูปร่าง (ออบเจ็กต์ที่มีคีย์ d)
}
op-item-not-found = ไม่พบรายการย่อย { $subject }
op-list-full = รายการ { $subject } เต็มแล้ว (มีรายการย่อยได้ไม่เกิน { $max })
op-list-minimum = รายการ { $subject } ต้องมีรายการย่อยอย่างน้อย { $min }
op-list-cell-too-long = เซลล์ของรายการ { $subject } ยาวเกินไป (คอลัมน์ { $column })
op-list-id-too-long = ตัวระบุรายการย่อยของรายการ { $subject } ยาวเกินไป
op-list-too-many-items = รายการ { $subject } มีรายการย่อยมากเกินไป (ไม่เกิน { $max })
op-slot-rejects = ไม่รับ { $subject } ที่นี่
op-wrong-category = ใช้ { $subject } ที่นี่ไม่ได้
op-choice-out-of-range = ค่าไม่ได้รับอนุญาตสำหรับ { $subject }
transaction-refused = การดำเนินการ { $index } ถูกปฏิเสธ: { $reason }

sources-unreadable = อ่านซอร์สไม่ได้
deck-inline-data = องค์ประกอบ { $element } เก็บข้อมูลไว้ในตัวเลือก: ไฟล์เด็คนี้ไม่ใช่รูปแบบปัจจุบัน
sources-not-sources = นี่ไม่ใช่ไฟล์เก็บซอร์สของ Oku Slides
sources-unsupported-version = ซอร์สนี้สร้างด้วย Oku Slides เวอร์ชันที่ใหม่กว่า
sources-bad-name = ชื่อไฟล์ในซอร์สถูกปฏิเสธ
sources-missing-deck = ไม่มีงานนำเสนอในซอร์ส
sources-too-large = ซอร์สในไฟล์นี้มีขนาดใหญ่เกินกว่าจะเปิดได้
sources-too-deep = เด็คนี้ซ้อนองค์ประกอบลึกเกินไป
sources-base-refused = ไฟล์ข้อมูลของเด็คนี้เสียหายหรือไม่สามารถจัดเก็บได้

issue-empty-slide-id = ตัวระบุสไลด์ว่างเปล่า
issue-duplicate-slide-id = ตัวระบุสไลด์ซ้ำกัน
issue-duplicate-element-id = ตัวระบุองค์ประกอบซ้ำกัน
issue-empty-element-id = ตัวระบุว่างเปล่า
issue-cell-outside-grid = เซลล์อยู่นอกตาราง
issue-empty-frame = กรอบมีขนาดเป็นศูนย์
issue-free-without-frame = องค์ประกอบอิสระไม่มีกรอบ
issue-unknown-widget = วิดเจ็ตที่ไม่รู้จัก: { $kind }
issue-wrong-category = ใช้ { $kind } ที่นี่ไม่ได้
issue-unknown-param = พารามิเตอร์ที่ไม่รู้จัก: { $param }
issue-param-type = ค่าไม่ถูกต้องสำหรับ { $param }
issue-choice-out-of-range = ค่าที่ไม่อนุญาตสำหรับ { $param }
issue-slot-rejects = ไม่รับ { $kind } ที่นี่
issue-placed-in-flow = องค์ประกอบที่วางด้วยกรอบในรายการที่จัดวางแล้ว
issue-unknown-bind = { $param } เชื่อมกับการตั้งค่าเด็คที่ไม่รู้จัก
issue-slot-too-deep = องค์ประกอบซ้อนกันลึกเกินไป
issue-invalid-style = ค่าสไตล์ไม่ถูกต้อง: { $field } ({ $reason ->
        [valign] { op-value-valign }
        [out-of-bounds] { op-value-out-of-bounds }
       *[other] { op-value-unreadable }
    })
issue-missing-asset = ไม่มีแอสเซ็ต: { $asset }
issue-pack-version-mismatch = ทิ้งเวอร์ชันแพ็กที่ถูกแก้ไขแล้ว: { $version }
issue-pack-version-missing = ไม่มีเวอร์ชันแพ็กในแหล่งข้อมูล: { $version }
issue-pack-pin-mismatch = แพ็กที่ฝังไว้ { $pack } ไม่ตรงกับที่เด็คคาดหวัง
issue-pack-tokens-invalid = แพ็กที่ฝังไว้ { $pack } มีโทเค็นคอมโพเนนต์ที่ไม่ถูกต้อง: { $reason }
issue-pack-kind-mismatch = แพ็กที่ฝังไว้ { $pack } ไม่ใช่ประเภทที่เด็คคาดหวัง (วิดเจ็ตหรือเลย์เอาต์)
issue-pack-kind-conflict = เนมสเปซ { $pack } ถูกปักหมุดเป็นทั้งแพ็กวิดเจ็ตและแพ็กเลย์เอาต์: ใช้ได้เพียงอย่างเดียว
issue-pack-requirement-missing = แพ็ก { $pack } ต้องใช้ { $required } ซึ่งเด็คนี้ไม่มี
issue-theme-version-mismatch = ทิ้งเวอร์ชันธีมที่ถูกแก้ไขแล้ว: { $version }
issue-theme-version-missing = ไม่มีเวอร์ชันธีมในแหล่งข้อมูล: { $version }
issue-theme-pin-mismatch = ธีมที่ฝังไว้ { $namespace } ไม่ตรงกับที่เด็คคาดหวัง
issue-theme-closure = ธีมที่ปักหมุดในเด็คนี้ไม่ใช่ธีมของเด็คและธีมแม่ของมันพอดี ({ $namespace })
issue-usage-line-invalid = บรรทัดที่ { $line } ของบันทึกการใช้งานอ่านไม่ได้ จึงทิ้งไป
issue-usage-line-forged = รอบการใช้งาน { $turn } ถูกบันทึกสองครั้งด้วยเนื้อหาต่างกัน จึงทิ้งไปหนึ่งรายการ
issue-usage-too-large = บันทึกการใช้งานใหญ่เกินไป จึงทิ้งไป
issue-usage-unreadable = บันทึกการใช้งานอ่านไม่ได้ จึงทิ้งไป
issue-usage-lines-dropped = { $count ->
   *[other] ทิ้งบันทึกการใช้งานอีก { $count } บรรทัด
}
usage-range-bad-date = “{ $date }” ไม่ใช่วันที่: ใช้รูปแบบ YYYY-MM-DD
usage-range-reversed = วันที่เริ่มต้น { $since } อยู่หลังวันที่สิ้นสุด { $until }
pack-token-entry-invalid = โทเค็นคอมโพเนนต์ “{ $path }” ในไฟล์กำกับแพ็กไม่ถูกต้อง
pack-manifest-json = อ่านไฟล์กำกับแพ็กไม่ได้: { $reason }
pack-manifest-order = ลำดับรายการของแพ็กไม่ถูกต้องที่ “{ $name }”
pack-manifest-data-count = แพ็กมีข้อมูล { $count } รายการ สูงสุด { $max } รายการ
pack-manifest-data-base = ข้อมูล “{ $data }” ของแพ็กอ้างอิงไฟล์ภายนอก ข้อมูลของแพ็กต้องมีเซลล์ของตัวเอง
pack-manifest-data-cells = ข้อมูลของแพ็กมี { $count } เซลล์ สูงสุด { $max } เซลล์
issue-unsupported-link = ลิงก์ที่ไม่รองรับ: { $link }
issue-highlight-zero = สีไฮไลต์เริ่มนับจาก 1
issue-unreadable-element = องค์ประกอบที่อ่านไม่ได้ ถูกแยกไว้
issue-slot-repaired = องค์ประกอบที่ไม่มีช่องที่รู้จัก ถูกวางไว้ใน { $slot }
issue-unreadable-item = รายการย่อยที่อ่านไม่ได้ ถูกแยกไว้
issue-unreadable-row = แถวของตารางที่อ่านไม่ได้ ถูกแยกไว้
issue-data-truncated = ข้อมูล “{ $data }” ใหญ่เกินไป ถูกตัดทอนขณะอ่านและเก็บไว้แบบอ่านอย่างเดียว
issue-data-name-taken = ชื่อข้อมูล “{ $data }” ถูกใช้แล้ว
issue-data-cell-type = ข้อมูล “{ $data }”: คอลัมน์ “{ $column }” มีค่าที่ไม่ใช่ชนิด { $type }
issue-data-too-many = ข้อมูลในเด็คมากเกินไป ({ $count }) ที่ไม่ได้อ่าน: { $dropped }
issue-data-base-missing = ข้อมูล “{ $data }”: ไม่พบไฟล์ฐาน เก็บไว้แบบอ่านอย่างเดียว
issue-data-edits-lost = ข้อมูล “{ $data }”: การแก้ไขตารางที่ถูกแทนที่สูญหายระหว่างการผสาน
issue-data-inline-dropped = องค์ประกอบ { $element }: เวอร์ชันเก่าเก็บข้อมูลของแผนภูมิหรือตารางไว้ในตัวเลือก ข้อมูลนั้นถูกนำออกแล้ว
issue-data-kind-unknown = ข้อมูล “{ $data }”: ไม่รู้จักชนิด “{ $kind }” อาจมาจากเวอร์ชันที่ใหม่กว่า จึงไม่ได้อ่าน
issue-data-query-cycle = ข้อมูล “{ $data }”: คิวรีอ่านตัวเองกลับ ทั้งโดยตรงหรือผ่านคิวรีอื่น จึงไม่ได้คำนวณ
issue-data-query-missing-input = ข้อมูล “{ $data }”: คิวรีอ่าน “{ $input }” ซึ่งไม่มีในเด็ค จึงไม่ได้คำนวณ
issue-data-query-invalid = ข้อมูล “{ $data }”: คิวรีไม่ถูกต้อง ({ $reason }) จึงไม่ได้คำนวณ
issue-data-query-failed = ข้อมูล “{ $data }”: คิวรีล้มเหลว ({ $reason }) จึงไม่มีผลลัพธ์
issue-more = ปัญหาอื่นที่ไม่ได้แสดง: { $count }
op-data-deck-full = ข้อมูลของเด็คจะเกินขีดจำกัด: { $what ->
    [row-ids] รหัสแถว
    [overlay] เซลล์ที่แก้ไข
    [text] ไบต์ของข้อความ
   *[data] ข้อมูล
} ได้ไม่เกิน { $max }
op-data-missing = ไม่พบข้อมูล “{ $data }”
op-data-name-taken = ชื่อข้อมูล “{ $name }” ถูกใช้แล้ว
op-data-column-in-use = คอลัมน์ “{ $column }” แสดงโดย { $views } ให้ผูกกับคอลัมน์อื่นก่อน (bind_data พร้อม columns)
op-data-in-use = ข้อมูลนี้แสดงโดย { $views }{ $more ->
    [0] {""}
   *[other] {" "}และอีก { $more } มุมมอง
}{" "}ให้ลบหรือผูกกับข้อมูลอื่นก่อน
op-data-column-missing = ข้อมูล “{ $data }” ไม่มีคอลัมน์ “{ $column }”
op-data-not-view = พารามิเตอร์ “{ $param }” แสดงข้อมูลไม่ได้ ทำได้เฉพาะตัวเลือกของแผนภูมิหรือตาราง
op-data-truncated = ข้อมูล “{ $data }” ถูกตัดทอนตอนอ่านและอ่านได้อย่างเดียว ให้แทนที่ตารางทั้งหมดเพื่อแก้ไข
op-data-rebind = องค์ประกอบ { $subject } แสดงข้อมูลอื่นอยู่แล้ว ใช้ bind_data เพื่อเปลี่ยน
op-data-internal = ชุดคำสั่งไม่สามารถตั้งค่าฐานหรือชั้นซ้อนของตารางได้
op-data-batch-too-large = ชุดคำสั่งใหญ่เกินไป: จำนวนเซลล์ { $count } สูงสุด { $max }
op-data-inline = องค์ประกอบ { $subject } เก็บข้อมูลไว้ในตัวเลือก ให้เพิ่มเป็นข้อมูลของเด็คด้วย add_data และผูกด้วย bind_data
op-data-derived = ข้อมูล “{ $data }” ได้มาจากคิวรี: ให้แก้ไขคิวรี ไม่ใช่แถว
op-data-source = ข้อมูลหนึ่งรับตารางหรือคิวรี (sql) อย่างใดอย่างหนึ่งเท่านั้น
op-data-cycle = ข้อมูล “{ $data }” จะอ่านตัวเองกลับผ่านคิวรีของตน: ให้เอาวงวนออก
op-data-unknown-input = คิวรีอ่าน “{ $name }” ซึ่งไม่ใช่ข้อมูลในเด็ค
op-data-not-query = ข้อมูล “{ $data }” ไม่ใช่คิวรี: ให้แก้ไขตารางด้วย edit_data
op-data-read-by-queries = ข้อมูลนี้ถูกอ่านโดยคิวรี { $queries }{ $shown ->
    [0] {""}
   *[other] {" "}และแสดงโดย { $views }
}{ $more ->
    [0] {""}
   *[other] {" "}และอีก { $more } รายการ
}{" "}ให้แก้ไขหรือเอาออกก่อน
op-data-rename-captured = คิวรีของ “{ $reader }” ตั้งชื่อตารางหนึ่งของตัวเองว่า “{ $name }” อยู่แล้ว: ให้เปลี่ยนชื่อตารางนั้นในคิวรี (set_query) หรือเลือกชื่ออื่น
op-data-name-ambiguous = มีข้อมูลหลายรายการชื่อ “{ $name }” และมีคิวรีอ่านชื่อนี้ จึงไม่ชัดว่าอ่านรายการใด: ให้แก้ไขคิวรีเหล่านั้นก่อน (set_query)
issue-unreadable-slide = สไลด์ที่อ่านไม่ได้ ถูกแยกไว้
issue-unreadable-field = ช่อง { $field } ของงานนำเสนอที่อ่านไม่ได้ ถูกรีเซ็ต
issue-knowledge-unreadable = รายการความรู้ { $id } ที่อ่านไม่ได้ ถูกแยกไว้
issue-knowledge-entries-dropped = รายการความรู้ที่ถูกตัดทิ้งโดยไม่อ่าน: { $count } รายการ ({ $cause ->
        [files] ไฟล์มากเกินกว่าจะอ่าน
        [file-bytes] ไฟล์ใหญ่เกินกว่าจะอ่าน
        [total-bytes] ชุดข้อมูลใหญ่เกินกว่าจะอ่าน
       *[unreadable] รายการที่อ่านไม่ได้เพิ่มเติม
    })
issue-knowledge-over-limit = ชุดความรู้เกินขีดจำกัด: { $limit ->
        [knowledge-reason-too-many-files] { knowledge-reason-too-many-files }
        [knowledge-reason-file-too-large] { knowledge-reason-file-too-large }
       *[knowledge-reason-total-too-large] { knowledge-reason-total-too-large }
    } (ไฟล์ที่เกี่ยวข้อง: { $count } ไฟล์ ไฟล์แรกคือ { $id }) เก็บไฟล์ทั้งหมดไว้ มีเพียงการลบไฟล์เท่านั้นที่ทำให้กลับมาอยู่ในขีดจำกัด (ระหว่างนี้การเขียนและการแก้ไขจะถูกปฏิเสธ)
issue-knowledge-path-invalid = ย้ายไฟล์ความรู้ { $id } ไปที่ /recovered: { $reason ->
        [knowledge-path-empty] { knowledge-path-empty }
        [knowledge-path-not-absolute] { knowledge-path-not-absolute }
        [knowledge-path-bad-segment] { knowledge-path-bad-segment }
        [knowledge-path-dot-segment] { knowledge-path-dot-segment }
        [knowledge-path-reserved] { knowledge-path-reserved }
        [knowledge-path-too-deep] { knowledge-path-too-deep }
        [knowledge-path-too-long] { knowledge-path-too-long }
       *[knowledge-path-not-markdown] { knowledge-path-not-markdown }
    }
issue-knowledge-path-collision = มีไฟล์ความรู้หลายไฟล์อยู่ที่ { $path }: ไฟล์อื่นถูกเปลี่ยนชื่อ
issue-knowledge-invalid = ไฟล์ความรู้ { $path } ไม่ถูกต้อง: { $reason ->
        [knowledge-frontmatter-missing] { knowledge-frontmatter-missing }
        [knowledge-frontmatter-unterminated] { knowledge-frontmatter-unterminated }
        [knowledge-frontmatter-too-large] { knowledge-frontmatter-too-large }
        [knowledge-frontmatter-not-yaml] { knowledge-frontmatter-not-yaml }
        [knowledge-frontmatter-not-mapping] { knowledge-frontmatter-not-mapping }
        [knowledge-frontmatter-duplicate-key] { knowledge-frontmatter-duplicate-key }
        [knowledge-frontmatter-alias] { knowledge-frontmatter-alias }
        [knowledge-frontmatter-tag] { knowledge-frontmatter-tag }
        [knowledge-frontmatter-too-deep] { knowledge-frontmatter-too-deep }
        [knowledge-frontmatter-after-document-end] { knowledge-frontmatter-after-document-end }
        [knowledge-frontmatter-flow-mapping] { knowledge-frontmatter-flow-mapping }
        [knowledge-frontmatter-type-missing] { knowledge-frontmatter-type-missing }
        [knowledge-frontmatter-bad-timestamp] { knowledge-frontmatter-bad-timestamp }
       *[knowledge-frontmatter-not-allowed] { knowledge-frontmatter-not-allowed }
    }
op-invalid-chart = แผนภูมิ { $subject }: { $reason }
op-invalid-table = ตาราง { $subject }: { $reason }

replica-corrupted = ประวัติงานนำเสนอในไฟล์นี้เสียหาย
replica-too-large = ประวัติงานนำเสนอในไฟล์นี้มีขนาดใหญ่เกินกว่าจะเปิดได้
replica-write = ไม่สามารถอัปเดตประวัติงานนำเสนอได้
replica-unrelated = งานนำเสนอทั้งสองนี้ไม่มีต้นกำเนิดร่วมกัน
replica-unsupported-version = งานนำเสนอนี้ใช้รูปแบบที่ Oku Slides เวอร์ชันนี้อ่านไม่ได้
replica-too-deep = ไม่สามารถบันทึกงานนำเสนอนี้ได้: องค์ประกอบที่ถูกลบซ้อนกันลึกเกินไปในประวัติ เลิกทำการย้ายครั้งล่าสุด แล้วบันทึกอีกครั้ง

widget-unreadable = ไม่สามารถอ่านการประกาศวิดเจ็ตได้
widget-fault-empty-name = วิดเจ็ต { $widget }: วิดเจ็ตหรือพารามิเตอร์ไม่มีชื่อ
widget-fault-duplicate-widget = วิดเจ็ต { $widget } ถูกประกาศสองครั้ง
widget-fault-duplicate-param = วิดเจ็ต { $widget } ประกาศพารามิเตอร์เดียวกันสองครั้ง
widget-fault-empty-choice = วิดเจ็ต { $widget }: ตัวเลือกไม่มีค่า
widget-fault-default-not-in-choice = วิดเจ็ต { $widget }: ค่าเริ่มต้นของตัวเลือกไม่ได้อยู่ในค่าที่กำหนด
widget-fault-default-wrong-type = วิดเจ็ต { $widget }: ค่าเริ่มต้นไม่ตรงกับชนิดของพารามิเตอร์
widget-fault-bind-not-plain = วิดเจ็ต { $widget }: มีเพียงพารามิเตอร์ข้อความธรรมดาเท่านั้นที่ผูกกับงานนำเสนอได้
widget-fault-unknown-slot-kind = วิดเจ็ต { $widget }: สล็อตยอมรับวิดเจ็ตองค์ประกอบที่ไม่รู้จัก
widget-fault-empty-list = วิดเจ็ต { $widget }: รายการต้องมีอย่างน้อยหนึ่งคอลัมน์
widget-fault-column-type = วิดเจ็ต { $widget }: คอลัมน์ของรายการเป็นได้แค่ plain, rich, number, flag, choice หรือ asset (หรือ p.list ที่ซ้อนกันได้หนึ่งชั้น); หากต้องการเนื้อหาอิสระต่อรายการย่อย ให้ใช้ p.slot กับวิดเจ็ตลูก
widget-fault-reserved-name = วิดเจ็ต { $widget }: ชื่อที่สงวนไว้: id, kind หรือชื่อที่มี /
widget-fault-list-bounds = วิดเจ็ต { $widget }: ค่าสูงสุดของรายการต่ำกว่าค่าต่ำสุด
widget-fault-list-default-too-large = วิดเจ็ต { $widget }: ค่าเริ่มต้นของรายการเกินขีดจำกัดขนาดของรายการ
widget-fault-list-depth = วิดเจ็ต { $widget }: รายการที่ซ้อนอยู่ไม่สามารถมีคอลัมน์รายการได้ รายการซ้อนกันได้เพียงชั้นเดียว
widget-fault-list-default-below-min = วิดเจ็ต { $widget }: ค่าเริ่มต้นของรายการที่ซ้อนอยู่มีรายการย่อยน้อยกว่าค่าต่ำสุด
widget-fault-slide-sizing = วิดเจ็ต { $widget } เป็นสไลด์: มีเพียงวิดเจ็ตองค์ประกอบเท่านั้นที่ประกาศ grow หรือ typography
widget-fault-pack-category = วิดเจ็ต { $widget } ไม่ตรงกับแพ็กของตน: แพ็กเลย์เอาต์มีวิดเจ็ตสไลด์ ส่วนแพ็กวิดเจ็ตมีวิดเจ็ตองค์ประกอบ
widget-fault-unqualified = วิดเจ็ต { $widget } ไม่มีเนมสเปซของแพ็ก
widget-fault-too-long = วิดเจ็ต { $widget }: ชื่อ ป้ายกำกับ ค่า คำอธิบาย หรือรายการเกินขีดจำกัด
issue-empty-item-id = รายการ { $param } มีรายการย่อยที่ไม่มีตัวระบุ
issue-duplicate-item-id = รายการ { $param } มีตัวระบุของรายการย่อยซ้ำกัน
issue-invalid-item-id = รายการ { $param } มีตัวระบุรายการย่อยที่มีเครื่องหมายทับ
issue-list-bounds = รายการ { $param } มีรายการย่อยน้อยเกินไปหรือมากเกินไป
issue-view-truncated = { $element } แสดงข้อมูลเพียงบางส่วน ({ $what ->
    [rows] แถว
   *[columns] คอลัมน์
}: { $shown } จาก { $total })
issue-render-budget = ไม่ได้วาด { $element }: มุมมองของงานนำเสนออ่านข้อมูลมากกว่าที่การเรนเดอร์อนุญาต
issue-data-missing = { $element } อ้างถึงข้อมูล “{ $data }” ซึ่งไม่มีในงานนำเสนอ
issue-view-column-missing = { $element } อ่านคอลัมน์ “{ $column }” ซึ่งไม่มีในข้อมูลของมัน
issue-view-unreadable = ไม่ได้วาด { $element }: อ่านตัวเลือกของมันไม่ได้
op-replica-full = ประวัติของงานนำเสนอใกล้ถึง { $max } MiB ซึ่งเป็นขนาดสูงสุดที่เปิดใหม่ได้: โปรดลบเนื้อหาก่อนเพิ่มอีก
