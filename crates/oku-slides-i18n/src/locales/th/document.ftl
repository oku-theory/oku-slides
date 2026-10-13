pdf-unreadable = อ่าน PDF ไม่ได้
pdf-unsupported-version = PDF นี้สร้างด้วย Oku Slides เวอร์ชันที่ใหม่กว่า
pdf-corrupted = ซอร์สใน PDF นี้เสียหาย
pdf-write = เขียน PDF ไม่ได้

render-compile = เรนเดอร์งานนำเสนอไม่ได้
render-pdf = ส่งออก PDF ไม่สำเร็จ
render-no-page = ไม่มีหน้านี้
render-image = ไม่สามารถสร้างภาพของสไลด์ได้

render-setup = เริ่มตัวเรนเดอร์ไม่ได้
render-missing-image = ไม่มีรูปภาพ
diagnostic-error = ข้อผิดพลาด
diagnostic-warning = คำเตือน
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = คำแนะนำ: { $hint }
render-chart-failed = วาดแผนภูมิไม่ได้

registry-bad-pack-id = รหัสชุดเทมเพลตไม่ถูกต้อง: { $id }
registry-compile = คอมไพล์วิดเจ็ตของชุดเทมเพลตไม่ได้
registry-missing = ชุดเทมเพลตไม่ได้ประกาศวิดเจ็ตของตน
render-timeout = การเรนเดอร์ใช้เวลานานเกิน { $seconds } วินาทีและถูกหยุด
render-out-of-memory = การเรนเดอร์ต้องใช้หน่วยความจำเกิน { $mib } MiB และถูกหยุด
render-worker-lost = กระบวนการเรนเดอร์หยุดทำงาน
render-frame = ข้อความจากกระบวนการเรนเดอร์อ่านไม่ได้
render-page-too-large = สไลด์นี้ใหญ่เกินกว่าจะแสดงตัวอย่างได้ (เกิน { $mib } MiB มักเป็นเพราะรูปภาพขนาดใหญ่มาก) แต่ยังส่งออกเป็น PDF หรือ PNG ได้
render-stale = การเรนเดอร์ถูกแทนที่ โปรดลองอีกครั้ง
render-pack-disabled = วิดเจ็ตของแพ็ก “{ $pack }” ถูกปิดใช้งานในเซสชันนี้
registry-schema-remote = ทะเบียนวิดเจ็ตไม่ถูกต้อง: { $message }
probe-not-probed = ยังไม่ได้ตรวจสอบ: เวลาที่กำหนดให้วิดเจ็ตของแพ็กหมดแล้ว จะตรวจสอบในการเรนเดอร์ครั้งถัดไป
probe-too-many-files = แพ็กมีไฟล์วิดเจ็ตเกิน { $max } ไฟล์: ไม่ได้โหลดไฟล์นี้
probe-too-large = เกิน { $max } KiB: ไม่ได้โหลด

import-unreadable = ไม่สามารถอ่านงานนำเสนอนี้ได้
import-not-a-presentation = ไฟล์นี้ไม่ใช่งานนำเสนอ PowerPoint
import-protected = งานนำเสนอนี้ถูกเข้ารหัสหรือเป็นรูปแบบ .ppt แบบเก่า: โปรดบันทึกเป็น .pptx ก่อน
import-too-large = งานนำเสนอนี้ใหญ่เกินกว่าจะนำเข้าได้
import-loss-list-levels = ลดระดับรายการเหลือระดับเดียว
import-loss-merged-cells = แยกเซลล์ที่ผสานไว้
import-loss-percent-stacked = แสดงแผนภูมิแบบเรียงซ้อน 100% เป็นแบบเรียงซ้อน
import-loss-run-style = { $effect ->
    [underline] ขีดเส้นใต้หายไป
    [strike] ขีดทับหายไป
    [baseline] ตัวยกหรือตัวห้อยหายไป
    [caps] ตัวพิมพ์ใหญ่หายไป
    [font] แบบอักษรของข้อความบางส่วนถูกแทนด้วยแบบอักษรของกล่อง
    [tracking] ระยะห่างตัวอักษรของข้อความบางส่วนถูกแทนด้วยระยะของกล่อง
    [weight] ข้อความปกติแสดงเป็นตัวหนาตามกล่อง
    [slant] ข้อความตัวตรงแสดงเป็นตัวเอียงตามกล่อง
   *[highlight] การไฮไลต์หายไป
}
import-loss-transform = การหมุน การพลิก หรือการครอบตัดหายไป
import-loss-color-dropped = สี { $color } ไม่มีสีใกล้เคียงในธีม: ใช้สีของธีมแทน
import-loss-text-truncated = ย่อข้อความให้อยู่ในขีดจำกัด
import-loss-table-truncated = ย่อตารางให้อยู่ในขีดจำกัด
import-loss-opaque = { $reason ->
    [media] ไม่ได้นำเข้าวิดีโอหรือเสียง (ไม่มีภาพสำรอง)
    [ole] ไม่ได้นำเข้าวัตถุที่ฝังไว้ (ไม่มีภาพสำรอง)
    [math] ไม่ได้นำเข้าสมการ (ไม่มีภาพสำรอง)
    [smartart] ไม่ได้นำเข้า SmartArt (ไม่มีภาพสำรอง)
   *[unknown] ไม่ได้นำเข้าวัตถุที่ไม่รองรับ (ไม่มีภาพสำรอง)
}
import-loss-unsupported-image = ไม่ได้นำเข้าภาพรูปแบบ { $format }
import-loss-hidden-slide = นำเข้าสไลด์ที่ซ่อนไว้เป็นสไลด์ที่มองเห็น
import-loss-effects = เงา การสะท้อน และเอฟเฟกต์ 3 มิติหายไป
import-loss-series-dropped = แผนภูมิวงกลมแสดงได้เพียงชุดข้อมูลเดียว: เก็บไว้ { $kept } ตัดออก { $dropped }
import-loss-repeated-decor = ละส่วนหัวหรือท้ายกระดาษ “{ $text }” ที่ซ้ำใน { $slides } สไลด์: ธีมวาดของตัวเอง
import-loss-no-slot = ละเนื้อหา: เทมเพลตที่เลือกไม่มีที่สำหรับเนื้อหานี้
import-loss-page-fitted = หน้าต้นฉบับ { $width } × { $height } pt ไม่ใช่ 16:9: ปรับขนาดเนื้อหาและจัดกึ่งกลางแล้ว
import-loss-paragraph-style = การจัดแนว ระยะบรรทัด หรือระยะย่อหน้าของย่อหน้าถูกปรับตามกล่อง
import-loss-shape-style = { $effect ->
    [gradient] การไล่ระดับสีแสดงเป็นสีทึบ
    [picture] รูปภาพเติมถูกยืดตามกรอบของรูปร่าง
    [dash] เส้นประแสดงเป็นเส้นทึบ
   *[geometry] รูปร่างที่ไม่รู้จักแสดงเป็นสี่เหลี่ยมผืนผ้า
}
import-loss-layout-dropped = เลย์เอาต์มากเกินไปสำหรับแพ็กเดียว: “{ $layout }” ไม่มีเทมเพลต สไลด์ของมันใช้วิดเจ็ตว่างและได้รับการตกแต่งเป็นองค์ประกอบ
import-loss-decor-truncated = ตัดการตกแต่งให้พอดีกับขนาดไฟล์แพ็ก: ละองค์ประกอบ { $omitted } รายการ
import-loss-table-style = รูปแบบตารางเป็นค่าประมาณ: พื้นหลังเซลล์ เส้นขอบ ความกว้างคอลัมน์ หรือขนาดข้อความเฉพาะของตารางหายไป
import-font-substituted = แทนที่ { $family } ด้วย { $by } ที่มีเมตริกเข้ากันได้
import-font-missing = ไม่พบ { $family }: ข้อความใช้ฟอนต์ { $by }
import-font-restricted = { $family }: สัญญาอนุญาตห้ามฝังบางสไตล์ลงในงานนำเสนอ
import-font-restricted-substituted = ฝัง { $family } ลงในงานนำเสนอไม่ได้ (สัญญาอนุญาต): แทนที่ด้วย { $by } ที่มีเมตริกเข้ากันได้
import-font-restricted-missing = ฝัง { $family } ลงในงานนำเสนอไม่ได้ (สัญญาอนุญาต): ข้อความใช้ฟอนต์ { $by }
import-font-too-large = { $family }: ไฟล์ใหญ่เกินกว่าจะฝังในเด็ค ข้อความใช้ฟอนต์ { $by }
import-font-missing-no-fallback = ไม่พบ { $family } และฟอนต์เนื้อหาของธีม: ข้อความแสดงด้วยฟอนต์เริ่มต้นของเอนจิน
import-warning-effect-ignored = ไม่ได้นำเข้าเอฟเฟกต์: { $detail }
import-warning-approximated = แสดงผลแบบประมาณ: { $detail }
import-warning-unreadable-item = ข้ามรายการที่อ่านไม่ได้: { $detail }
import-warning-unreadable-part = ส่วนที่อ่านไม่ได้: { $detail }
import-warning-missing-part = ส่วนที่ขาดหายไป: { $detail }
import-warning-unsupported-chart = แผนภูมิที่ไม่รองรับ: { $detail }
import-warning-unsupported-image = รูปแบบรูปภาพที่ไม่รองรับ: { $detail }
import-warning-unknown-table-style = สไตล์ตารางที่ไม่รู้จัก: { $detail }
import-warning-unsafe-link = ปฏิเสธลิงก์: { $detail }
import-warning-nesting-too-deep = การซ้อนลึกเกินไป ถูกตัด: { $detail }
import-warning-font-embedded-unreadable = อ่านฟอนต์ที่ฝังไว้ไม่ได้: { $detail }
import-warning-truncated = คำเตือนมากเกินไป: ละคำเตือนที่เหลือ
