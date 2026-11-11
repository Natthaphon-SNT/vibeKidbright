# คลังพรอมท์ mAi Plus 128 ข้อ — ฉบับตรวจ 27 ตุลาคม 2026

**ใช้ฉบับนี้แทนชุดก่อนตรวจ** ตรวจครบ 128 รหัสแล้ว แต่ไม่ได้รันโปรแกรมที่โมเดลสร้างครบ 128 โปรแกรมหรือทดสอบอุปกรณ์จริงครบทุกข้อ ดู [ผลตรวจรายข้อ](KIDBRIGHT_MAI_PLUS_PROMPT_AUDIT_2026-10-27.md) ก่อนเลือกเทส

- **TEMPLATE_CHECKED 7 ข้อ:** มีตัวอย่างตรงงานและทดสอบ fixture แล้ว
- **DRAFT_REFERENCE 43 ข้อ:** มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
- **NEEDS_SETUP 60 ข้อ:** รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
- **HOLD 18 ข้อ:** รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้

มีตัวอย่างตรงงานสำหรับ MP-001–005, MP-015 และ MP-021 เท่านั้น ข้อ DRAFT_REFERENCE ยังต้องสร้างโค้ดเฉพาะงานและตรวจทาน ไม่ใช่ไฟล์พร้อมรัน ข้อ NEEDS_SETUP ต้องระบุ fixture/config/bands ก่อน ข้อ HOLD ทำ discovery/แผน manual ได้ แต่ยังไม่ขับฮาร์ดแวร์

## วิธีใช้และพรอมท์ตั้งต้น

เลือก Plus ส่งกติกาด้านล่างหนึ่งครั้ง แล้วคัดลอกเทสทีละข้อ ตรวจ Keep/Undo ก่อน Run ผู้ใช้เป็นผู้กด Connect/Run/Stop เอง ห้ามรันทั้ง 128 ข้ออัตโนมัติ ตาราง [ผลฮาร์ดแวร์เดิม](KIDBRIGHT_MAI_PLUS_TEST_RESULTS_2026-10-26.csv) ไม่ถูกรีเซ็ตหรือใช้ผล audit แทนผลจริง

```text
ใช้ KidBright mAi Plus SG2002, MaixPy 4.12.5 / Runtime 1.24.0 ตามที่บอร์ดรายงาน
ก่อนสร้างเทส MP-xxx ให้เรียก uai_plus_test_reference ด้วยรหัสนั้น และอ่าน kidbright_uai_plus_test_contracts.md สำหรับ API ที่เกี่ยวข้อง ถ้าแอปรุ่นนี้ไม่มีเครื่องมืออ้างอิง ให้รายงานว่าตัวแอปยังไม่รวมชุด audit และใช้โปรเจกต์ตัวอย่างที่ตรวจไว้ ห้าม fallback เป็นวงกลมแล้วเรียกว่าเทสครบ
TEMPLATE_CHECKED ให้เลือก python_example ตรงตาม reference; ข้ออื่นที่สร้างโค้ดเฉพาะงานให้ส่ง main_python เป็นโปรแกรมที่ขอจริงในเครื่องมือสร้างโปรเจกต์ ตรวจ syntax/API และเสนอ write_file เพิ่มให้ครบก่อนถาม Keep โดยเสนอไฟล์ได้ระหว่าง review ห้ามสร้าง seed MP-021 แล้วให้ Keep ก่อนเริ่มทำ MP-022; NEEDS_SETUP ถามเฉพาะ fixture/config/ผลวัดที่ยังขาด; HOLD ให้ทำเฉพาะ discovery หรือแผน manual ห้ามเดา wiring/driver หรือเริ่มขับโหลด
ใช้ main/main.py, project_type=kidbright_uai_plus และโฟลเดอร์ใหม่ที่ไม่เขียนทับของเดิม ให้ IDE ดูแล Launcher ไม่ใช้ killall/pkill หรือแก้ boot script
เช็ก API ตามภาพอ้างอิงและ runtime จริง ไม่แต่งชื่อเมธอด ไม่มี API เป็น SKIP; driver/read ล้มเป็น ERROR พร้อมหลักฐาน ใช้ exit-aware loop, finite timeout ที่มีจริง และ cleanup ตาม API ของอุปกรณ์ ไม่สมมติว่าทุก object มี close()
ก่อน hardware Run ต้องตรวจโค้ดที่สร้างจริง อุปกรณ์ การต่อสาย แรงดัน พอร์ตที่มีเจ้าของ และ dependency ของเทส ไม่ถือว่า syntax ผ่าน อัปโหลดสำเร็จ หรือ API มีอยู่เป็น hardware PASS
ไม่ติดตั้งแพ็กเกจ โหลด kernel module เปลี่ยน USB role/เครือข่าย ตั้ง RTC หรือขับ motor เอง เก็บ buffer/log/file ให้มีขอบเขต รอผู้ทดสอบยืนยันภาพ เสียง สัญญาณ และเมนูกลับ
```

## การต่อ IO

ซอร์สระบุ IO2=A23, IO3=A27, IO4=A25, IO5=A22, IO6=A24, IO7=P24, IO8=A15 แต่ค่าเหล่านี้ยังไม่ใช่ PCB/routing proof ปิดไฟก่อนเปลี่ยนสาย ตรวจ S/V/G และระดับไฟ ใช้ LED พร้อมตัวต้านทานหรือ loopback ที่มี output เพียงด้านเดียวและ series resistor ห้ามต่อ 5V เข้าขาที่รองรับ 3.3V ห้ามขับ motor โดยตรง

IO7/P24 และ PWM backend ยังเป็น HOLD; ห้ามเปลี่ยน P24 เป็น A24 เพื่อให้รันผ่าน UART0/A16/A17 มีเจ้าของจาก log เดิม ไม่ใช้เทส UART ของผู้ใช้ การทดสอบ external I2C ต้องยืนยัน bus/backend และ protocol ของโมดูลก่อน ไม่มีสิทธิ์ scan บัสกล้อง/IMU เพื่อเดาอุปกรณ์

## พรอมท์รายข้อ

### สำรวจบอร์ดและ runtime

#### MP-001 — สำรวจเวอร์ชันจริง

สถานะก่อนเทส: **TEMPLATE_CHECKED** — มีตัวอย่างตรงงานและทดสอบ fixture แล้ว
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_001 โดยส่ง test_id="MP-001" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-001") ก่อน ทำเทส MP-001: แสดง Python, MaixPy, Runtime, OS และ device ID ที่ตรวจพบจริงบนจอและ Terminal โดยอ่าน metadata ก่อน native import เมื่อทำได้ ถ้าข้อมูลใดไม่มีให้แสดง UNKNOWN และอย่าเปลี่ยนเวอร์ชันที่ติดตั้ง
ข้อแก้ไขที่ต้องใช้: ใช้ capabilities; sys.device_id ไม่มีใน stub รุ่นนี้จึงเป็น optional/UNKNOWN ห้ามเรียก device_key
สถานะ TEMPLATE_CHECKED: มีตัวอย่างตรงงานและทดสอบ fixture แล้ว เลือก python_example="capabilities"
```

เกณฑ์ตรวจ: ค่าที่ตรวจได้มีหลักฐานจากบอร์ด ไม่ใส่ค่า 4.12.5/1.24.0 ทับ UNKNOWN

#### MP-002 — สำรวจ API ทีละโมดูล

สถานะก่อนเทส: **TEMPLATE_CHECKED** — มีตัวอย่างตรงงานและทดสอบ fixture แล้ว
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_002 โดยส่ง test_id="MP-002" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-002") ก่อน ทำเทส MP-002: ตรวจโมดูล display, image, touchscreen, camera, audio, gpio, pinmap, pwm, adc, i2c, uart, IMU และ RTC ที่ติดตั้ง โดยยังไม่เปิดใช้อุปกรณ์ทุกตัวพร้อมกัน แสดงชื่อ API ที่พบและข้อผิดพลาดรายโมดูล ไม่ให้โมดูลเดียวล้มทั้งรายงาน
ข้อแก้ไขที่ต้องใช้: ตรวจ symbol แบบไม่สร้างอุปกรณ์ รายงาน ext_dev.bm8563 ไม่ใช่ ext_dev.rtc และไม่ import package เสริม
สถานะ TEMPLATE_CHECKED: มีตัวอย่างตรงงานและทดสอบ fixture แล้ว เลือก python_example="capabilities"
```

เกณฑ์ตรวจ: รายงาน AVAILABLE/MISSING/ERROR แยกกันและยัง Stop ได้

#### MP-003 — สำรวจพอร์ตและ pinmux

สถานะก่อนเทส: **TEMPLATE_CHECKED** — มีตัวอย่างตรงงานและทดสอบ fixture แล้ว
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_003 โดยส่ง test_id="MP-003" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-003") ก่อน ทำเทส MP-003: สร้างรายงาน IO2–IO8 โดยเทียบตารางอ้างอิงกับรายการ pins/functions จาก runtime แบบอ่านอย่างเดียว ไม่สลับ pinmux และไม่อ้างว่ารายการ function พิสูจน์การเดินสายจริง บันทึก mapping ที่ยังยืนยันไม่ได้
ข้อแก้ไขที่ต้องใช้: มี get_pins/get_pin_functions แต่ไม่มี get_pin_function ใน reference 4.12.5; IO7/P24 ยังไม่ใช่ routing ที่ยืนยัน
สถานะ TEMPLATE_CHECKED: มีตัวอย่างตรงงานและทดสอบ fixture แล้ว เลือก python_example="capabilities"
```

เกณฑ์ตรวจ: ครบเจ็ดช่อง ไม่มีการขับเอาต์พุตหรือแตะ A16/A17

#### MP-004 — สำรวจอุปกรณ์ Linux

สถานะก่อนเทส: **TEMPLATE_CHECKED** — มีตัวอย่างตรงงานและทดสอบ fixture แล้ว
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_004 โดยส่ง test_id="MP-004" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-004") ก่อน ทำเทส MP-004: รายงาน /dev/input, /dev/i2c-*, /dev/ttyS*, /sys/class/leds และพื้นที่เก็บข้อมูลที่อ่านได้อย่างมีขอบเขต ไม่เขียน sysfs ไม่เปิด serial ที่กำลังมีเจ้าของ และไม่พิมพ์รหัสผ่านหรือ credential
ข้อแก้ไขที่ต้องใช้: อ่านเฉพาะ node/name ที่ระบุ ไม่ dump process command line, credential หรือเปิด serial แย่งเจ้าของ
สถานะ TEMPLATE_CHECKED: มีตัวอย่างตรงงานและทดสอบ fixture แล้ว เลือก python_example="capabilities"
```

เกณฑ์ตรวจ: เห็นรายการอุปกรณ์หรือเหตุผลที่อ่านไม่ได้โดยไม่เปลี่ยนค่าระบบ

#### MP-005 — ระบุอุปกรณ์ที่อาจต่างจากแคตตาล็อก

สถานะก่อนเทส: **TEMPLATE_CHECKED** — มีตัวอย่างตรงงานและทดสอบ fixture แล้ว
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_005 โดยส่ง test_id="MP-005" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-005") ก่อน ทำเทส MP-005: อ่านข้อมูลกล้องจาก sensor_cfg ที่มีและข้อมูลไดรเวอร์ IMU/RTC แบบอ่านอย่างเดียว เปรียบเทียบสิ่งที่ตรวจพบกับเอกสาร แสดง UNKNOWN เมื่อยืนยันไม่ได้ อย่าเลือก OV2685 หรือรุ่น RTC จากชื่อโฆษณาเพียงอย่างเดียว
ข้อแก้ไขที่ต้องใช้: แยก sensor_cfg กับชนิด IMU/RTC ที่ยัง UNKNOWN; ชื่อโมดูลไม่ใช่ chip ID
สถานะ TEMPLATE_CHECKED: มีตัวอย่างตรงงานและทดสอบ fixture แล้ว เลือก python_example="capabilities"
```

เกณฑ์ตรวจ: รายงานแยก detected, documented และ unknown โดยไม่แก้ไฟล์ระบบ

#### MP-006 — จัดแผนตามอุปกรณ์ที่มี

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: รายการอุปกรณ์ของผู้ทดสอบ
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_006 โดยส่ง test_id="MP-006" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-006") ก่อน ทำเทส MP-006: สร้างหน้าเลือกชุดทดสอบ onboard, GPIO, I2C, UART, servo และ AI แต่ยังไม่เริ่มขับพอร์ต ให้กรอกรุ่นโมดูล การต่อสาย และแรงดันก่อนเปิดชุดที่ต้องใช้ fixture ที่ยังไม่มีให้เป็น SKIP
ข้อแก้ไขที่ต้องใช้: เป็น planner ไม่ขับพอร์ตหรืออ้างผล self-test จน fixture ของชุดนั้นพร้อม
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ไม่มีผล PASS ปลอมจากอุปกรณ์ที่ไม่ได้ต่อ

### จอและกราฟิก

#### MP-007 — สีพื้นเต็มจอ

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_007 โดยส่ง test_id="MP-007" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-007") ก่อน ทำเทส MP-007: สลับแดง เขียว น้ำเงิน ขาว ดำทุกหนึ่งวินาที ใช้ขนาดจาก display จริง และให้ S1 หรือปุ่มทัชเปลี่ยนสีได้เมื่อ input พร้อมใช้ จบด้วยหน้ารอการยืนยันสีจากผู้ทดสอบ
ข้อแก้ไขที่ต้องใช้: ทดสอบสีเป็นแกนหลัก; input เสริมใช้ได้หลัง MP-015 หรือ MP-021 ผ่านเท่านั้น
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: สีเต็มพื้นที่ ไม่มีช่องตกขอบ และผู้ทดสอบยืนยันสีตรง

#### MP-008 — ขอบและทิศทางจอ

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_008 โดยส่ง test_id="MP-008" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-008") ก่อน ทำเทส MP-008: วาดกรอบห่างขอบจอ 2 พิกเซล พร้อม TOP LEFT RIGHT BOTTOM และพิกัดสี่มุม ตรวจ width/height จริง ไม่บังคับแนวนอน 640x480 ถ้า runtime ให้คนละทิศ
ข้อแก้ไขที่ต้องใช้: ปรับขนาดข้อความให้เห็นขอบและ orientation จริง ไม่อิง 640x480 ตายตัว
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: เห็นกรอบและข้อความครบ ไม่กลับด้านหรือถูกตัด

#### MP-009 — ข้อความและตัวเลข

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_009 โดยส่ง test_id="MP-009" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-009") ก่อน ทำเทส MP-009: แสดง ASCII ตัวพิมพ์เล็กใหญ่ ตัวเลข เครื่องหมาย ค่าติดลบ และทศนิยมหลายขนาด ถ้ามี font ไทยที่ตรวจพบจริงให้เพิ่มหน้าทดสอบไทยโดยไม่ดาวน์โหลด font เอง
ข้อแก้ไขที่ต้องใช้: ASCII เป็น baseline; หน้าไทยแยก conditional ต้องมีไฟล์ font ที่มีจริง
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ASCII อ่านได้ ค่าทศนิยมไม่ทำให้ API พิกัดล้ม ส่วนไทย PASS ได้ต่อเมื่อมี font และตรวจด้วยตา

#### MP-010 — รูปทรงและการซ้อนภาพ

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_010 โดยส่ง test_id="MP-010" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-010") ก่อน ทำเทส MP-010: วาดเส้น สี่เหลี่ยม วงกลม แบบโปร่งและทึบ พร้อมทดสอบลำดับการซ้อน ใช้พิกัดจำนวนเต็มและ image buffer เดียวก่อน show
ข้อแก้ไขที่ต้องใช้: ใช้ draw_line/draw_rect/draw_circle และพิกัด integer จาก reference ที่บันเดิล
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ตำแหน่งและสีตรงตามหน้าตัวอย่าง ไม่มี exception

#### MP-011 — ภาพเคลื่อนไหวและ FPS

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_011 โดยส่ง test_id="MP-011" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-011") ก่อน ทำเทส MP-011: ทำลูกบอลชนขอบพร้อมเลขเฟรมและ FPS ที่วัดจริง ไม่เปิดกล้อง รัน 60 วินาทีแล้วแสดงสถิติ frame time และคงหน้าผลให้ดูจน Stop
ข้อแก้ไขที่ต้องใช้: FPS วัดจากเฟรมที่ show สำเร็จ/monotonic elapsed ไม่ตั้งเกณฑ์ FPS ที่ยังไม่มี benchmark
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ลูกบอลเคลื่อนต่อเนื่อง ตัวเลขเฟรมเพิ่ม และสรุป FPS จากเวลาจริง

#### MP-012 — ภาพขนาดเล็กกับการปรับสเกล

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_012 โดยส่ง test_id="MP-012" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-012") ก่อน ทำเทส MP-012: เปรียบเทียบ canvas ขนาด 240x240 กับ canvas เท่าจอ แสดงวงกลมและเส้นกริด ตรวจโหมด fit ที่ API มีจริง และอธิบายขอบดำตามอัตราส่วน
ข้อแก้ไขที่ต้องใช้: fit และอัตราส่วนจากขนาดจริง; อย่าคาดว่าภาพ 240x240 ต้องเต็มจอโดยไม่เกิด letterbox
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: วงกลมไม่ผิดรูป และบอกพื้นที่แสดงผลที่คำนวณจริง

#### MP-013 — ความสว่างจอ

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_013 โดยส่ง test_id="MP-013" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-013") ก่อน ทำเทส MP-013: ตรวจ API backlight ที่มีจริง ถ้าปรับได้ให้เริ่มระดับกลาง ทดสอบช่วง 20–80 เปอร์เซ็นต์ และคืนค่าเดิมใน finally ถ้าไม่มี API ให้ SKIP ไม่เขียน PWM10 หรือ sysfs แบบเดา
ข้อแก้ไขที่ต้องใช้: get_backlight/set_backlight ใช้ 0–100; ต้องอ่านค่าเดิมได้ก่อนจึงเปลี่ยน และคืนใน finally
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ผู้ทดสอบเห็นความสว่างเปลี่ยนและกลับค่าเดิม หรือได้ SKIP พร้อมเหตุผล

#### MP-014 — จอเปิดทิ้งต่อเนื่อง

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_014 โดยส่ง test_id="MP-014" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-014") ก่อน ทำเทส MP-014: แสดงกริดนาฬิกาจับเวลา 10 นาที อัปเดตเพียงพื้นที่จำเป็นเมื่อ API รองรับ เก็บหน่วยความจำและ frame time เป็นช่วง ไม่สร้าง display ใหม่ทุกเฟรม
ข้อแก้ไขที่ต้องใช้: ไม่สมมติ partial update API; ใช้ buffer ที่มีขอบเขตและเก็บสถิติจริง
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: จออัปเดตตลอด ไม่มีหน่วยความจำเพิ่มต่อเนื่อง และ Stop คืนเมนูได้

### หน้าจอสัมผัส

#### MP-015 — ทัชพิกัดจริง

สถานะก่อนเทส: **TEMPLATE_CHECKED** — มีตัวอย่างตรงงานและทดสอบ fixture แล้ว
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_015 โดยส่ง test_id="MP-015" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-015") ก่อน ทำเทส MP-015: ใช้ touchscreen API ที่ตรวจพบ แสดง x,y,pressed พร้อม marker ตามนิ้วและนับ press/release ถ้าไม่มีไดรเวอร์ให้รายงานตรง ๆ
ข้อแก้ไขที่ต้องใช้: ใช้ python_example=touchscreen ซึ่งมี marker และ press/release จริง ไม่ใช่ display-only skeleton
สถานะ TEMPLATE_CHECKED: มีตัวอย่างตรงงานและทดสอบ fixture แล้ว เลือก python_example="touchscreen"
```

เกณฑ์ตรวจ: markerตามจุดสัมผัสและปล่อยแล้วสถานะกลับ ไม่ค้าง pressed

#### MP-016 — ทัชเก้าจุด

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-015

```text
สร้างโปรเจกต์ mai_test_016 โดยส่ง test_id="MP-016" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-016") ก่อน ทำเทส MP-016: ให้แตะเป้าแบบ 3x3 ครบเก้าจุด แสดงค่าคลาดเคลื่อนจากศูนย์กลางเป้าและเก็บค่าดิบ ไม่เขียน calibration ถาวร
ข้อแก้ไขที่ต้องใช้: แตะเป้าทีละจุด เก็บ raw และระยะคลาดเคลื่อน; ไม่เขียน calibration ถาวร
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: แตะมุมและกลางจอได้ครบ พร้อมค่าคลาดเคลื่อนให้ผู้ทดสอบประเมิน

#### MP-017 — ลากวาดเส้น

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-015

```text
สร้างโปรเจกต์ mai_test_017 โดยส่ง test_id="MP-017" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-017") ก่อน ทำเทส MP-017: ทำกระดานวาดเส้นด้วยนิ้ว มี CLEAR และ EXIT ไม่เชื่อมเส้นข้ามช่วงที่ปล่อยนิ้ว และจำกัดจำนวนจุดที่เก็บ
ข้อแก้ไขที่ต้องใช้: CLEAR/EXIT ต้องตรวจ pressed และขอบกด; ไม่ลากเส้นข้ามช่วงปล่อยนิ้ว
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: เส้นตามนิ้ว ยกนิ้วแล้วเริ่มเส้นใหม่ CLEAR ทำงาน

#### MP-018 — ปุ่มทัชกดค้างและกดซ้ำ

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-015

```text
สร้างโปรเจกต์ mai_test_018 โดยส่ง test_id="MP-018" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-018") ก่อน ทำเทส MP-018: ทำปุ่มนับจำนวนสองแบบ หนึ่งครั้งต่อการแตะ และกดค้างเพื่อเพิ่มซ้ำหลังครบ 500ms แสดงสถานะ debounce กับเวลาจริง
ข้อแก้ไขที่ต้องใช้: ใช้ monotonic edge/hold timer แยกจาก rendering loop ไม่เพิ่มตัวนับทุกเฟรม
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: แตะครั้งเดียวเพิ่มครั้งเดียว ส่วนการกดค้างเกิดซ้ำตามเงื่อนไข

#### MP-019 — ทัชกับภาพที่ถูกย่อขยาย

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-015

```text
สร้างโปรเจกต์ mai_test_019 โดยส่ง test_id="MP-019" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-019") ก่อน ทำเทส MP-019: วาดปุ่มบน canvas 240x240 แล้ว fit ลงจอจริง แปลงพิกัด touch กลับสู่ canvas โดยรวม offset ขอบดำ ใช้ API mapping ถ้ามีหรือคำนวณเอง
ข้อแก้ไขที่ต้องใช้: รวม scale และ letterbox offset ทั้งสองทิศทาง; ทดสอบขอบดำและขอบปุ่ม
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: แตะตรงปุ่มที่เห็นจึงทำงาน แตะขอบดำไม่กดปุ่ม

#### MP-020 — ความสามารถหลายจุดสัมผัส

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: This single-point API cannot establish multi-touch; restrict this case to capability discovery.

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_020 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-020" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-020") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-020: ตรวจว่าระดับไดรเวอร์และ API ส่งได้กี่จุด ถ้ารองรับหลายจุดให้แสดงแต่ละจุด ถ้า API ส่งได้เพียงจุดเดียวให้แสดง SINGLE_TOUCH_ONLY ไม่สร้างข้อมูลจุดที่สอง
ข้อแก้ไขที่ต้องใช้: ลดขอบเขตเป็น SINGLE_TOUCH_ONLY capability report เพราะ API นี้ส่งได้หนึ่งจุด ไม่ใช้สองนิ้วเป็นเกณฑ์ผ่าน
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: รายงานความสามารถจริง ไม่ตัดสินว่าแผงเสียเพราะ API จำกัดหนึ่งจุด

### ปุ่ม S1/S2 และ LED บนบอร์ด

#### MP-021 — สำรวจ ADC ของปุ่ม

สถานะก่อนเทส: **TEMPLATE_CHECKED** — มีตัวอย่างตรงงานและทดสอบ fixture แล้ว
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_021 โดยส่ง test_id="MP-021" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-021") ก่อน ทำเทส MP-021: อ่าน ADC0 แบบที่ runtime รองรับ แล้วให้ผู้ทดสอบเก็บค่าไม่กด กด S1 และกด S2 อย่างละ 3 วินาที แสดง min/max/median ก่อนกำหนด threshold ไม่ใช้เลข threshold เดิมโดยไม่ตรวจ
ข้อแก้ไขที่ต้องใช้: ใช้ python_example=adc_buttons; วัด 3 วินาที/สถานะก่อนเสนอ threshold และไม่ใช้ threshold เก่า
สถานะ TEMPLATE_CHECKED: มีตัวอย่างตรงงานและทดสอบ fixture แล้ว เลือก python_example="adc_buttons"
```

เกณฑ์ตรวจ: ข้อมูลสามสถานะมีการบันทึกจริง และแจ้งถ้าช่วงซ้อนกัน

#### MP-022 — ปุ่ม S1 หนึ่งครั้งต่อการกด

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: Measured non-overlapping released/S1/S2 bands and tester confirmation.; ผลจาก MP-021

```text
สร้างโปรเจกต์ mai_test_022 โดยส่ง test_id="MP-022" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-022") ก่อน ทำเทส MP-022: อ่าน S1 ด้วยเส้นทางปุ่มของ Plus ที่ตรวจแล้ว นับขอบกดพร้อม debounce ทดสอบกด 20 ครั้ง กดค้าง และปล่อย แสดง raw กับสถานะที่ตีความ
ข้อแก้ไขที่ต้องใช้: ใช้ bands จาก MP-021; กด 20 ครั้งเป็น ground truth ที่ผู้ทดสอบยืนยัน
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: กดค้างไม่ถูกนับเพิ่มหลายครั้ง และจำนวนตรงกับผู้ทดสอบ

#### MP-023 — ปุ่ม S2 และกดค้าง

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: Measured non-overlapping released/S1/S2 bands and tester confirmation.; ผลจาก MP-021

```text
สร้างโปรเจกต์ mai_test_023 โดยส่ง test_id="MP-023" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-023") ก่อน ทำเทส MP-023: ทดสอบ S2 ด้วยการกดสั้นและกดค้าง 2 วินาที แสดง event DOWN UP LONG_PRESS กับเวลาที่วัด และคงประวัติเพียง 20 event ล่าสุด
ข้อแก้ไขที่ต้องใช้: threshold จาก calibration; long press ใช้ monotonic และเกิดครั้งเดียวต่อรอบ
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: แต่ละ event เกิดครบและ LONG_PRESS เกิดครั้งเดียวต่อรอบ

#### MP-024 — ปุ่มสองตัวและสถานะกำกวม

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: Measured non-overlapping released/S1/S2 bands and tester confirmation.; ผลจาก MP-021

```text
สร้างโปรเจกต์ mai_test_024 โดยส่ง test_id="MP-024" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-024") ก่อน ทำเทส MP-024: ทดสอบ S1, S2, ไม่กด และกดพร้อมกัน ถ้าเป็นวงจร ADC ladder ที่แยกพร้อมกันไม่ได้ให้รายงาน AMBIGUOUS ไม่อ้างว่าตรวจสองปุ่มอิสระได้
ข้อแก้ไขที่ต้องใช้: ไม่กดพร้อมกันจนบันทึก bands แยกแล้ว; รายงาน AMBIGUOUS เมื่อแยกไม่ได้
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: สถานะกำกวมไม่ถูกนับเป็นการกดที่มั่นใจ

#### MP-025 — LED ผู้ใช้บนบอร์ด

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: Actual led-user path/ownership and saved active trigger/brightness.

```text
สร้างโปรเจกต์ mai_test_025 โดยส่ง test_id="MP-025" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-025") ก่อน ทำเทส MP-025: ค้นหา led-user ที่มีจริง เก็บ trigger/brightness เดิม กระพริบช้า 10 รอบแล้วคืนค่าเดิม อย่าเปลี่ยนไฟ power หรือ WiFi ถ้าแยกเจ้าของไม่ได้
ข้อแก้ไขที่ต้องใช้: restore เฉพาะ active trigger ที่อยู่ในวงเล็บกับ brightness เดิม ไม่เขียนรายการ trigger ทั้งบรรทัดกลับ
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: ผู้ทดสอบเห็น LED ที่ระบุติดดับและค่าเดิมถูกคืน

#### MP-026 — ปุ่มควบคุมจอและ LED

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: MP-021 bands and verified user LED node.; ผลจาก MP-021, MP-025

```text
สร้างโปรเจกต์ mai_test_026 โดยส่ง test_id="MP-026" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-026") ก่อน ทำเทส MP-026: ทำ S1 เพิ่มตัวนับ S2 ล้างตัวนับ และให้ LED ผู้ใช้ติดตามสถานะที่ตรวจได้จริง ใช้ loop แบบไม่บล็อกจอและไม่แย่ง ADC object ซ้ำหลายตัว
ข้อแก้ไขที่ต้องใช้: init ADC ครั้งเดียว; ใช้ calibration จริงและ LED node ที่ยืนยันแล้ว
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: จอ ปุ่ม และ LED ทำงานร่วมกันโดยตัวนับไม่กระโดด

### IO2–IO8 แยก input/output ทุกช่อง

#### MP-027 — IO2 digital output

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: LED พร้อมตัวต้านทานหรือ logic analyzer
ต้องมีก่อน: Confirmed runtime pin/function and connector wiring; correct voltage/pull/LED or instrument fixture.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_027 โดยส่ง test_id="MP-027" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-027") ก่อน ทำเทส MP-027: ทดสอบ IO2 โดย mapping อ้างอิงคือ A23 ยืนยันชื่อ pin และ GPIO function กับ runtime ก่อน ต่อ LED หรือเครื่องมือวัดตามระดับสัญญาณที่บอร์ดรองรับ เริ่ม LOW รอ START จากผู้ทดสอบ แล้วสลับ HIGH/LOW อย่างละหนึ่งวินาที 10 รอบ ตั้ง LOW และคืนทรัพยากรใน finally
ข้อแก้ไขที่ต้องใช้: ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: วัดหรือเห็นสัญญาณที่ IO2 ตรงกับคำสั่งครบ 10 รอบ ไม่ใช้การอ่านค่าที่สั่งเองเป็นหลักฐานทางกายภาพ

#### MP-028 — IO2 digital input

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: สวิตช์พร้อมตัวต้านทาน pull และสายที่ยืนยันแล้ว
ต้องมีก่อน: Confirmed runtime pin/function and connector wiring; correct voltage/pull/LED or instrument fixture.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_028 โดยส่ง test_id="MP-028" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-028") ก่อน ทำเทส MP-028: ทดสอบ IO2 โดย mapping อ้างอิงคือ A23 ตั้งเป็น input ตาม API ที่มี ใช้สวิตช์หรือแหล่งลอจิกที่ระดับถูกต้องพร้อม pull ที่เหมาะสม แสดง raw HIGH/LOW นับขอบและ debounce อย่าอ้างว่า floating input ต้องคงค่า
ข้อแก้ไขที่ต้องใช้: ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: สลับ input ที่ต่อจริงแล้วค่าบนจอและ Terminal เปลี่ยนตรงทุกครั้ง

#### MP-029 — IO3 digital output

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: LED พร้อมตัวต้านทานหรือ logic analyzer
ต้องมีก่อน: Confirmed runtime pin/function and connector wiring; correct voltage/pull/LED or instrument fixture.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_029 โดยส่ง test_id="MP-029" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-029") ก่อน ทำเทส MP-029: ทดสอบ IO3 โดย mapping อ้างอิงคือ A27 ยืนยันชื่อ pin และ GPIO function กับ runtime ก่อน ต่อ LED หรือเครื่องมือวัดตามระดับสัญญาณที่บอร์ดรองรับ เริ่ม LOW รอ START จากผู้ทดสอบ แล้วสลับ HIGH/LOW อย่างละหนึ่งวินาที 10 รอบ ตั้ง LOW และคืนทรัพยากรใน finally
ข้อแก้ไขที่ต้องใช้: ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: วัดหรือเห็นสัญญาณที่ IO3 ตรงกับคำสั่งครบ 10 รอบ ไม่ใช้การอ่านค่าที่สั่งเองเป็นหลักฐานทางกายภาพ

#### MP-030 — IO3 digital input

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: สวิตช์พร้อมตัวต้านทาน pull และสายที่ยืนยันแล้ว
ต้องมีก่อน: Confirmed runtime pin/function and connector wiring; correct voltage/pull/LED or instrument fixture.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_030 โดยส่ง test_id="MP-030" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-030") ก่อน ทำเทส MP-030: ทดสอบ IO3 โดย mapping อ้างอิงคือ A27 ตั้งเป็น input ตาม API ที่มี ใช้สวิตช์หรือแหล่งลอจิกที่ระดับถูกต้องพร้อม pull ที่เหมาะสม แสดง raw HIGH/LOW นับขอบและ debounce อย่าอ้างว่า floating input ต้องคงค่า
ข้อแก้ไขที่ต้องใช้: ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: สลับ input ที่ต่อจริงแล้วค่าบนจอและ Terminal เปลี่ยนตรงทุกครั้ง

#### MP-031 — IO4 digital output

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: LED พร้อมตัวต้านทานหรือ logic analyzer
ต้องมีก่อน: Confirmed runtime pin/function and connector wiring; correct voltage/pull/LED or instrument fixture.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_031 โดยส่ง test_id="MP-031" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-031") ก่อน ทำเทส MP-031: ทดสอบ IO4 โดย mapping อ้างอิงคือ A25 ยืนยันชื่อ pin และ GPIO function กับ runtime ก่อน ต่อ LED หรือเครื่องมือวัดตามระดับสัญญาณที่บอร์ดรองรับ เริ่ม LOW รอ START จากผู้ทดสอบ แล้วสลับ HIGH/LOW อย่างละหนึ่งวินาที 10 รอบ ตั้ง LOW และคืนทรัพยากรใน finally
ข้อแก้ไขที่ต้องใช้: ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: วัดหรือเห็นสัญญาณที่ IO4 ตรงกับคำสั่งครบ 10 รอบ ไม่ใช้การอ่านค่าที่สั่งเองเป็นหลักฐานทางกายภาพ

#### MP-032 — IO4 digital input

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: สวิตช์พร้อมตัวต้านทาน pull และสายที่ยืนยันแล้ว
ต้องมีก่อน: Confirmed runtime pin/function and connector wiring; correct voltage/pull/LED or instrument fixture.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_032 โดยส่ง test_id="MP-032" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-032") ก่อน ทำเทส MP-032: ทดสอบ IO4 โดย mapping อ้างอิงคือ A25 ตั้งเป็น input ตาม API ที่มี ใช้สวิตช์หรือแหล่งลอจิกที่ระดับถูกต้องพร้อม pull ที่เหมาะสม แสดง raw HIGH/LOW นับขอบและ debounce อย่าอ้างว่า floating input ต้องคงค่า
ข้อแก้ไขที่ต้องใช้: ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: สลับ input ที่ต่อจริงแล้วค่าบนจอและ Terminal เปลี่ยนตรงทุกครั้ง

#### MP-033 — IO5 digital output

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: LED พร้อมตัวต้านทานหรือ logic analyzer
ต้องมีก่อน: Confirmed runtime pin/function and connector wiring; correct voltage/pull/LED or instrument fixture.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_033 โดยส่ง test_id="MP-033" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-033") ก่อน ทำเทส MP-033: ทดสอบ IO5 โดย mapping อ้างอิงคือ A22 ยืนยันชื่อ pin และ GPIO function กับ runtime ก่อน ต่อ LED หรือเครื่องมือวัดตามระดับสัญญาณที่บอร์ดรองรับ เริ่ม LOW รอ START จากผู้ทดสอบ แล้วสลับ HIGH/LOW อย่างละหนึ่งวินาที 10 รอบ ตั้ง LOW และคืนทรัพยากรใน finally
ข้อแก้ไขที่ต้องใช้: ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: วัดหรือเห็นสัญญาณที่ IO5 ตรงกับคำสั่งครบ 10 รอบ ไม่ใช้การอ่านค่าที่สั่งเองเป็นหลักฐานทางกายภาพ

#### MP-034 — IO5 digital input

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: สวิตช์พร้อมตัวต้านทาน pull และสายที่ยืนยันแล้ว
ต้องมีก่อน: Confirmed runtime pin/function and connector wiring; correct voltage/pull/LED or instrument fixture.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_034 โดยส่ง test_id="MP-034" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-034") ก่อน ทำเทส MP-034: ทดสอบ IO5 โดย mapping อ้างอิงคือ A22 ตั้งเป็น input ตาม API ที่มี ใช้สวิตช์หรือแหล่งลอจิกที่ระดับถูกต้องพร้อม pull ที่เหมาะสม แสดง raw HIGH/LOW นับขอบและ debounce อย่าอ้างว่า floating input ต้องคงค่า
ข้อแก้ไขที่ต้องใช้: ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: สลับ input ที่ต่อจริงแล้วค่าบนจอและ Terminal เปลี่ยนตรงทุกครั้ง

#### MP-035 — IO6 digital output

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: LED พร้อมตัวต้านทานหรือ logic analyzer
ต้องมีก่อน: Confirmed runtime pin/function and connector wiring; correct voltage/pull/LED or instrument fixture.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_035 โดยส่ง test_id="MP-035" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-035") ก่อน ทำเทส MP-035: ทดสอบ IO6 โดย mapping อ้างอิงคือ A24 ยืนยันชื่อ pin และ GPIO function กับ runtime ก่อน ต่อ LED หรือเครื่องมือวัดตามระดับสัญญาณที่บอร์ดรองรับ เริ่ม LOW รอ START จากผู้ทดสอบ แล้วสลับ HIGH/LOW อย่างละหนึ่งวินาที 10 รอบ ตั้ง LOW และคืนทรัพยากรใน finally
ข้อแก้ไขที่ต้องใช้: ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: วัดหรือเห็นสัญญาณที่ IO6 ตรงกับคำสั่งครบ 10 รอบ ไม่ใช้การอ่านค่าที่สั่งเองเป็นหลักฐานทางกายภาพ

#### MP-036 — IO6 digital input

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: สวิตช์พร้อมตัวต้านทาน pull และสายที่ยืนยันแล้ว
ต้องมีก่อน: Confirmed runtime pin/function and connector wiring; correct voltage/pull/LED or instrument fixture.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_036 โดยส่ง test_id="MP-036" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-036") ก่อน ทำเทส MP-036: ทดสอบ IO6 โดย mapping อ้างอิงคือ A24 ตั้งเป็น input ตาม API ที่มี ใช้สวิตช์หรือแหล่งลอจิกที่ระดับถูกต้องพร้อม pull ที่เหมาะสม แสดง raw HIGH/LOW นับขอบและ debounce อย่าอ้างว่า floating input ต้องคงค่า
ข้อแก้ไขที่ต้องใช้: ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: สลับ input ที่ต่อจริงแล้วค่าบนจอและ Terminal เปลี่ยนตรงทุกครั้ง

#### MP-037 — IO7 digital output

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: LED พร้อมตัวต้านทานหรือ logic analyzer
ต้องมีก่อน: IO7=P24 alias/function/routing not proven; read-only discovery first.; ผลจาก MP-003

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_037 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-037" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-037") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-037: ทดสอบ IO7 โดย mapping อ้างอิงคือ P24 ยืนยันชื่อ pin และ GPIO function กับ runtime ก่อน ต่อ LED หรือเครื่องมือวัดตามระดับสัญญาณที่บอร์ดรองรับ เริ่ม LOW รอ START จากผู้ทดสอบ แล้วสลับ HIGH/LOW อย่างละหนึ่งวินาที 10 รอบ ตั้ง LOW และคืนทรัพยากรใน finally
ข้อแก้ไขที่ต้องใช้: HOLD การขับ IO7: P24 เป็นค่าจาก dropdown ไม่ใช่ข้อพิสูจน์ว่า native runtime รับ alias นี้
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: วัดหรือเห็นสัญญาณที่ IO7 ตรงกับคำสั่งครบ 10 รอบ ไม่ใช้การอ่านค่าที่สั่งเองเป็นหลักฐานทางกายภาพ

#### MP-038 — IO7 digital input

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: สวิตช์พร้อมตัวต้านทาน pull และสายที่ยืนยันแล้ว
ต้องมีก่อน: IO7=P24 alias/function/routing not proven; read-only discovery first.; ผลจาก MP-003

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_038 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-038" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-038") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-038: ทดสอบ IO7 โดย mapping อ้างอิงคือ P24 ตั้งเป็น input ตาม API ที่มี ใช้สวิตช์หรือแหล่งลอจิกที่ระดับถูกต้องพร้อม pull ที่เหมาะสม แสดง raw HIGH/LOW นับขอบและ debounce อย่าอ้างว่า floating input ต้องคงค่า
ข้อแก้ไขที่ต้องใช้: HOLD การตั้ง IO7 เป็น input เช่นเดียวกัน ห้ามเปลี่ยนเป็น A24 แทน
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: สลับ input ที่ต่อจริงแล้วค่าบนจอและ Terminal เปลี่ยนตรงทุกครั้ง

#### MP-039 — IO8 digital output

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: LED พร้อมตัวต้านทานหรือ logic analyzer
ต้องมีก่อน: Confirmed runtime pin/function and connector wiring; correct voltage/pull/LED or instrument fixture.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_039 โดยส่ง test_id="MP-039" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-039") ก่อน ทำเทส MP-039: ทดสอบ IO8 โดย mapping อ้างอิงคือ A15 ยืนยันชื่อ pin และ GPIO function กับ runtime ก่อน ต่อ LED หรือเครื่องมือวัดตามระดับสัญญาณที่บอร์ดรองรับ เริ่ม LOW รอ START จากผู้ทดสอบ แล้วสลับ HIGH/LOW อย่างละหนึ่งวินาที 10 รอบ ตั้ง LOW และคืนทรัพยากรใน finally
ข้อแก้ไขที่ต้องใช้: ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: วัดหรือเห็นสัญญาณที่ IO8 ตรงกับคำสั่งครบ 10 รอบ ไม่ใช้การอ่านค่าที่สั่งเองเป็นหลักฐานทางกายภาพ

#### MP-040 — IO8 digital input

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: สวิตช์พร้อมตัวต้านทาน pull และสายที่ยืนยันแล้ว
ต้องมีก่อน: Confirmed runtime pin/function and connector wiring; correct voltage/pull/LED or instrument fixture.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_040 โดยส่ง test_id="MP-040" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-040") ก่อน ทำเทส MP-040: ทดสอบ IO8 โดย mapping อ้างอิงคือ A15 ตั้งเป็น input ตาม API ที่มี ใช้สวิตช์หรือแหล่งลอจิกที่ระดับถูกต้องพร้อม pull ที่เหมาะสม แสดง raw HIGH/LOW นับขอบและ debounce อย่าอ้างว่า floating input ต้องคงค่า
ข้อแก้ไขที่ต้องใช้: ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: สลับ input ที่ต่อจริงแล้วค่าบนจอและ Terminal เปลี่ยนตรงทุกครั้ง

### GPIO หลายช่องและ loopback

#### MP-041 — สำรวจ input เจ็ดช่องพร้อมกัน

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: fixture input ที่กำหนดระดับครบเจ็ดช่อง
ต้องมีก่อน: All seven connector mappings, including IO7/P24, and all-channel safe fixture must be established.; ผลจาก MP-003

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_041 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-041" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-041") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-041: อ่าน IO2–IO8 เป็น input ทั้งหมด แสดงตาราง bit raw และเวลาที่เปลี่ยนล่าสุด ถ้ายังไม่ได้ต่อ fixture ให้แสดง FLOATING/UNVERIFIED อย่าให้ PASS จากการอ่านได้เฉย ๆ
ข้อแก้ไขที่ต้องใช้: HOLD การอ่านครบเจ็ดช่องจน IO7 ยืนยัน; floating input ไม่เป็น FAIL ของพอร์ต
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: เปลี่ยน fixture ทีละช่องแล้วเปลี่ยนเฉพาะช่องที่เกี่ยวข้อง

#### MP-042 — walking-one output

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: LED พร้อมตัวต้านทานครบเจ็ดช่องหรือ logic analyzer
ต้องมีก่อน: All seven connector mappings, including IO7/P24, and all-channel safe fixture must be established.; ผลจาก MP-003

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_042 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-042" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-042") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-042: ตั้งเอาต์พุต IO2–IO8 ที่ยืนยันแล้ว เริ่มทุกช่อง LOW รอ START แล้วให้ HIGH เพียงหนึ่งช่องไล่ทีละช่อง ทุกรอบต้องมีป้ายชื่อช่องที่กำลังขับ
ข้อแก้ไขที่ต้องใช้: HOLD all-seven output; ห้ามขับ alias ที่ยังไม่ยืนยันเพื่อให้ครบตาราง
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: เห็น HIGH เพียงหนึ่งช่องและลำดับ IO2 ถึง IO8 ตรง

#### MP-043 — walking-zero output

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: fixture output ครบเจ็ดช่อง
ต้องมีก่อน: All seven connector mappings, including IO7/P24, and all-channel safe fixture must be established.; ผลจาก MP-003

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_043 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-043" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-043") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-043: ทำแพตเทิร์นทุกช่อง HIGH ยกเว้นหนึ่งช่อง LOW ไล่เจ็ดช่อง แล้วกลับ LOW ทั้งหมด จำกัดการทดสอบเฉพาะ LED โหลดต่ำที่ต่อถูกต้อง
ข้อแก้ไขที่ต้องใช้: walking-zero ต้องประเมินกระแสรวม fixture ไม่เท่ากับการขับ GPIO ทุกช่องพร้อมกันโดยไร้เงื่อนไข
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: แต่ละรอบมี LOW เพียงช่องเดียว ไม่มีกระแสโหลดเกินพอร์ต

#### MP-044 — loopback IO2 → IO3

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: สาย loopback ผ่านตัวต้านทานอนุกรม 1–4.7kΩ
ต้องมีก่อน: Verified pins, one output per loopback pair, series resistor/safe LED fixture and START gate.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_044 โดยส่ง test_id="MP-044" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-044") ก่อน ทำเทส MP-044: ยืนยันว่าต่อ IO2 output ไป IO3 input ผ่านตัวต้านทานและไม่มีโหลดอื่น จากนั้นส่งแพตเทิร์นบิตช้า 100 บิต อ่าน input เทียบทีละบิต ใช้ timeout และหยุดขับเมื่อผิดพลาด
ข้อแก้ไขที่ต้องใช้: IO2 output → IO3 input ผ่าน series resistor; อีกด้านไม่เป็น output
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: จำนวนบิตตรงและรายงาน mismatch จากการอ่าน IO3 จริง

#### MP-045 — loopback IO4 → IO5

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: สาย loopback ผ่านตัวต้านทานอนุกรม 1–4.7kΩ
ต้องมีก่อน: Verified pins, one output per loopback pair, series resistor/safe LED fixture and START gate.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_045 โดยส่ง test_id="MP-045" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-045") ก่อน ทำเทส MP-045: ทดสอบ IO4 output ไป IO5 input ด้วยแพตเทิร์น 00110101 ซ้ำที่ 5Hz เปรียบเทียบค่าที่อ่านจริงและสรุปจำนวนผิด ห้ามตั้งขาทั้งสองเป็น output พร้อมกัน
ข้อแก้ไขที่ต้องใช้: IO4 output → IO5 input มี timeout ใน loop ที่ควบคุมเองและวัด input จริง
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: input ตามแพตเทิร์นครบและจบโดย output เป็น LOW

#### MP-046 — loopback IO6 → IO8

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: สาย loopback ผ่านตัวต้านทานอนุกรม 1–4.7kΩ
ต้องมีก่อน: Verified pins, one output per loopback pair, series resistor/safe LED fixture and START gate.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_046 โดยส่ง test_id="MP-046" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-046") ก่อน ทำเทส MP-046: ทดสอบ IO6 output ไป IO8 input ด้วยการสลับช้า วัดเวลาที่พบการเปลี่ยนแบบ userspace และรายงานว่าเป็น software-observed latency ไม่ใช่สเปกเวลาขา GPIO
ข้อแก้ไขที่ต้องใช้: latency เป็น userspace-observed เท่านั้น ไม่ใช่สเปก hardware GPIO
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: ข้อมูลรับตรงและไม่กล่าวอ้างความแม่นยำระดับไมโครวินาที

#### MP-047 — ปุ่มภายนอกสั่ง LED คนละช่อง

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: สวิตช์ที่ IO2 และ LED ที่ IO3 พร้อมวงจรที่ถูกต้อง
ต้องมีก่อน: Verified pins, one output per loopback pair, series resistor/safe LED fixture and START gate.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_047 โดยส่ง test_id="MP-047" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-047") ก่อน ทำเทส MP-047: ทำ IO2 input ควบคุม IO3 output พร้อม debounce และแสดง input/output บนจอ กดค้างและกดเร็วแล้วตรวจว่าตามเงื่อนไข ตั้ง LED ดับเมื่อ Stop
ข้อแก้ไขที่ต้องใช้: กรอก active-high/low และ pull ตามสวิตช์จริง อย่าเดาว่าปล่อยต้องเป็น HIGH
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: สวิตช์ที่ต่อจริงควบคุม LED ได้ ไม่แกว่งเมื่อปล่อย

#### MP-048 — Stop ระหว่างขับ GPIO

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: fixture LED โหลดต่ำ
ต้องมีก่อน: Verified pins, one output per loopback pair, series resistor/safe LED fixture and START gate.; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_048 โดยส่ง test_id="MP-048" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-048") ก่อน ทำเทส MP-048: ทำแพตเทิร์น IO ที่ตรวจ mapping แล้วและรองรับ finally ให้ผู้ทดสอบกด Stop ระหว่าง HIGH ตรวจระดับจริงหลัง Stop และบันทึกว่า cleanup สำเร็จหรือเป็น UNKNOWN หากถูก kill ก่อน finally
ข้อแก้ไขที่ต้องใช้: ใช้ GPIO ที่ยืนยันแล้วกับ LED เท่านั้น; forced kill อาจไม่รัน finally ต้องตรวจ LOW จริงหรือ UNKNOWN
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: LED/สัญญาณกลับสภาวะที่กำหนดและเมนูกลับมา โดยไม่อ้าง cleanup หากวัดไม่ได้

### PWM และ servo

#### MP-049 — สำรวจ PWM ที่ออกมาที่ connector

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว ยังไม่ต่อโหลด
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-003

```text
สร้างโปรเจกต์ mai_test_049 โดยส่ง test_id="MP-049" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-049") ก่อน ทำเทส MP-049: ตรวจ IO7 และทางเลือก PWM จาก runtime กับเอกสาร Plus สรุป pin, function, channel และข้อขัดแย้ง โดยยังไม่สร้าง PWM อย่าเหมารวมว่า IO ทุกช่องมี hardware PWM
ข้อแก้ไขที่ต้องใช้: discovery เท่านั้น ไม่มีการสร้าง PWM; pin/function/channel เป็นคนละข้อมูล
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ได้ mapping ที่ตรวจได้หรือ UNKNOWN ไม่มีการเปลี่ยน backlight channel

#### MP-050 — วัด PWM บน IO7

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: oscilloscope หรือ logic analyzer
ต้องมีก่อน: Confirmed IO7 PWM pin/function/channel/backend; instrument proof before servo; actual servo pulse limits/power.; ผลจาก MP-049

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_050 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-050" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-050") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-050: เมื่อยืนยัน mapping IO7/PWM แล้ว สร้างสัญญาณ 50Hz duty ต่ำที่กำหนดได้ วัด period และ pulse width จากเครื่องมือจริง ไม่ต่อ servo ระหว่างวัด และ disable PWM ใน finally
ข้อแก้ไขที่ต้องใช้: HOLD จน mapping/backend แน่นอน; ต้องใช้เครื่องมือวัดก่อนต่อ servo ไม่เดา PWM2 จาก helper ที่ไม่พบ
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: ค่าจากเครื่องมือตรงกับค่าที่ตั้งใน tolerance ที่ผู้ทดสอบกำหนด

#### MP-051 — LED ปรับ duty บน IO7

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: LED พร้อมตัวต้านทานที่ใช้กับระดับลอจิกได้
ต้องมีก่อน: Confirmed IO7 PWM pin/function/channel/backend; instrument proof before servo; actual servo pulse limits/power.; ผลจาก MP-049

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_051 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-051" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-051") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-051: ใช้ hardware PWM ของ IO7 ที่ยืนยันแล้วที่ 1kHz ไล่ duty 0,25,50,75,100 เปอร์เซ็นต์ แสดงแต่ละขั้น คงแต่ละขั้นหนึ่งวินาทีและปิดท้ายด้วย duty 0
ข้อแก้ไขที่ต้องใช้: duty เป็นเปอร์เซ็นต์; HOLD จน IO7 PWM ยืนยัน ห้ามใช้ช่อง backlight แทน
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: เครื่องมือวัด duty ตรง หรือผู้ทดสอบยืนยันความสว่างเป็นผลเชิงสายตา

#### MP-052 — servo เริ่มกลางช่วง

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: servo ที่ทราบรุ่น แหล่งจ่ายและการต่อ GND ถูกต้อง
ต้องมีก่อน: Confirmed IO7 PWM pin/function/channel/backend; instrument proof before servo; actual servo pulse limits/power.; ผลจาก MP-049

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_052 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-052" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-052") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-052: ใช้ servo บน IO7 ตามไดรเวอร์ Plus ที่มีจริง กำหนดช่วง pulse จาก datasheet ของ servo เริ่มตำแหน่งกลางก่อน ขยับเฉพาะช่วงปลอดการติดขัดที่ผู้ทดสอบกำหนด ไม่ลองสุดมุม 0/180 โดยอัตโนมัติ
ข้อแก้ไขที่ต้องใช้: servo.C906Servo ยังไม่ใช่ dependency ที่ยืนยันใน image; ห้าม import แล้วอ้างว่าพร้อม
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: servo เคลื่อนตามคำสั่งโดยไม่ติดกลไกและไฟเลี้ยงบอร์ดไม่ตก

#### MP-053 — servo ควบคุมด้วย touch

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: fixture servo ที่ผ่านการทดสอบกลางช่วงแล้ว
ต้องมีก่อน: Confirmed IO7 PWM pin/function/channel/backend; instrument proof before servo; actual servo pulse limits/power.; ผลจาก MP-049, MP-015

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_053 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-053" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-053") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-053: ทำ slider บนจอควบคุม servo IO7 จำกัดมุมและอัตราการเคลื่อน แสดง pulse width จริงที่สั่ง ปล่อยนิ้วแล้วหยุดเปลี่ยนเป้าหมาย และให้ปุ่มยุติการส่ง PWM
ข้อแก้ไขที่ต้องใช้: ต้องผ่าน MP-015 และ mapping/pulse limits ของ servo ก่อนรับค่า slider
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: servo ตาม slider ภายในขอบเขต ไม่มีการกระชากจากพิกัดผิด

#### MP-054 — PWM กับ Stop และการรันใหม่

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: fixture PWM ที่ผ่านการยืนยันแล้ว
ต้องมีก่อน: Confirmed IO7 PWM pin/function/channel/backend; instrument proof before servo; actual servo pulse limits/power.; ผลจาก MP-049

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_054 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-054" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-054") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-054: ให้ผู้ทดสอบ Run → Stop ระหว่าง PWM → ตรวจสัญญาณหยุด → Run ซ้ำ เมื่อ mapping/backend ยืนยันแล้ว ใช้ disable() ตาม API บันทึก channel busy และตรวจสัญญาณจริงหลัง Stop ไม่อ้างว่าคืน pinmux เดิมเมื่อไม่รู้ค่าเดิม หรือ finally ทำงานหลัง forced kill
ข้อแก้ไขที่ต้องใช้: ต้องวัดสัญญาณหลัง Stop จริง; disable() มีใน API แต่ไม่รับประกัน cleanup เมื่อ kill
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: ไม่เกิด channel busy จากรอบก่อน และมีหลักฐานว่าสัญญาณหยุดหลัง Stop

### I2C เซนเซอร์ภายนอก และ ADC ภายนอก

#### MP-055 — สำรวจ I2C connector จริง

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_055 โดยส่ง test_id="MP-055" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-055") ก่อน ทำเทส MP-055: ตรวจ external I2C ของ Plus ทั้ง KB-CHAIN/GROVE จากเอกสารและ device node ที่มี แยกจากบัสกล้อง/IMU ตรวจว่า image ใช้ hardware bus หรือ i2c-gpio ที่เตรียมไว้ ห้ามเดา bus 1/5 หรือ insmod แก้ระบบเอง
ข้อแก้ไขที่ต้องใช้: list_devices/node เป็น discovery ไม่ scan หรือเปิดบัสภายใน และไม่ insmod อัตโนมัติ
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: มี bus/pin/backend ที่มีหลักฐานหรือแสดงสิ่งที่ขาดก่อนทดสอบ

#### MP-056 — scan I2C ภายนอกแบบควบคุม

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: โมดูล I2C ที่ทราบรุ่นและต่อถูกต้อง
ต้องมีก่อน: Actual external bus/backend, correct module wiring/voltage/address and matching device protocol/driver. No implicit module loading.; ผลจาก MP-055

```text
สร้างโปรเจกต์ mai_test_056 โดยส่ง test_id="MP-056" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-056") ก่อน ทำเทส MP-056: บนบัสภายนอกที่ยืนยันแล้ว ตรวจ ACK เฉพาะช่วง address ที่คู่มืออุปกรณ์อนุญาต หรือใช้ probe ที่ไดรเวอร์รองรับ แสดง address 7-bit ไม่ scan บัสภายในที่ไม่เกี่ยวข้องและไม่เขียน register แบบสุ่ม
ข้อแก้ไขที่ต้องใช้: scan เป็น probe ที่อาจมีผลต่ออุปกรณ์ ไม่ใช่ read-only discovery; จำกัด bus/address/operation ตามโมดูล
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: พบ address ของอุปกรณ์ที่ต่อจริงโดยอธิบายชนิด probe

#### MP-057 — อ่านเซนเซอร์ SHT31

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: SHT31 และสาย I2C ที่แรงดันถูกต้อง
ต้องมีก่อน: Actual external bus/backend, correct module wiring/voltage/address and matching device protocol/driver. No implicit module loading.; ผลจาก MP-055

```text
สร้างโปรเจกต์ mai_test_057 โดยส่ง test_id="MP-057" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-057") ก่อน ทำเทส MP-057: อ่านอุณหภูมิและความชื้นจาก SHT31 ที่ address ที่ยืนยันแล้ว ตรวจ CRC ของข้อมูลถ้ารองรับ แสดงค่าพร้อมหน่วยและเก็บ min/max 60 วินาที แยก NACK/CRC failure จากค่าปกติ
ข้อแก้ไขที่ต้องใช้: ต้องมี external bus และ SHT31 protocol ที่ตรงรุ่น; ตรวจ None/length/CRC ก่อนแปลงค่าจริง
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: ได้ค่าจริงและมีข้อมูลคุณภาพ ไม่คืน 0 เป็นค่าปกติเมื่ออ่านล้ม

#### MP-058 — อ่านเซนเซอร์แสง I2C

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: BH1750 หรือเซนเซอร์แสง I2C ที่ระบุรุ่นจริง
ต้องมีก่อน: Actual external bus/backend, correct module wiring/voltage/address and matching device protocol/driver. No implicit module loading.; ผลจาก MP-055

```text
สร้างโปรเจกต์ mai_test_058 โดยส่ง test_id="MP-058" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-058") ก่อน ทำเทส MP-058: ใช้ไดรเวอร์ตรงรุ่น อ่านแสงเป็น lux และกราฟตามเวลา ให้บัง/เปิดแสงตามขั้นตอน ถ้าไม่ใช่ BH1750 อย่าใช้ register ของมันแทน
ข้อแก้ไขที่ต้องใช้: ระบุรุ่น/measurement command/conversion ของโมดูล ไม่ใช้ค่า raw ของบัสเป็น lux
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: ค่าตอบสนองการบังแสงและแสดงหน่วยที่ไดรเวอร์ยืนยัน

#### MP-059 — จอ OLED ภายนอก

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: OLED I2C ที่ทราบ controller และ resolution
ต้องมีก่อน: Actual external bus/backend, correct module wiring/voltage/address and matching device protocol/driver. No implicit module loading.; ผลจาก MP-055

```text
สร้างโปรเจกต์ mai_test_059 โดยส่ง test_id="MP-059" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-059") ก่อน ทำเทส MP-059: ตรวจว่าเป็น SSD1306 หรือ SH1106 และขนาดจริงก่อนใช้ไดรเวอร์ จากนั้นแสดงลายกริดกับตัวนับพร้อมจอหลัก ไม่เขียน address อื่นเพื่อเดารุ่น
ข้อแก้ไขที่ต้องใช้: ระบุ controller SSD1306/SH1106 และ resolution จากโมดูลจริง ไม่เดาด้วย address
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: OLED แสดงตรงขอบเขตที่กำหนด จอหลักและ Stop ยังทำงาน

#### MP-060 — หลายอุปกรณ์บน I2C

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: โมดูล I2C สองตัวที่ address ไม่ชนกัน
ต้องมีก่อน: Actual external bus/backend, correct module wiring/voltage/address and matching device protocol/driver. No implicit module loading.; ผลจาก MP-055

```text
สร้างโปรเจกต์ mai_test_060 โดยส่ง test_id="MP-060" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-060") ก่อน ทำเทส MP-060: อ่านเซนเซอร์สองตัวบน external bus เดียวกัน ใช้ bus object ร่วมและเวลาแยกกัน แสดงจำนวน transaction กับ error ต่ออุปกรณ์ แยกสาเหตุ address ชนก่อนเริ่ม
ข้อแก้ไขที่ต้องใช้: ตรวจ address conflict และแชร์ bus handle; ไม่สร้างหลายเจ้าของอุปกรณ์เดียวกัน
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: อุปกรณ์ทั้งสองอัปเดตโดยไม่รบกวนกัน

#### MP-061 — ADC ภายนอกกับ potentiometer

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: ADC I2C ที่ระบุรุ่น เช่น ADS1115 และ potentiometer
ต้องมีก่อน: Actual external bus/backend, correct module wiring/voltage/address and matching device protocol/driver. No implicit module loading.; ผลจาก MP-055

```text
สร้างโปรเจกต์ mai_test_061 โดยส่ง test_id="MP-061" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-061") ก่อน ทำเทส MP-061: ใช้ ADC I2C ตรงรุ่นอ่าน potentiometer ในช่วงแรงดันที่ ADC/โมดูลอนุญาต แสดง raw, gain และแรงดันตาม reference จริง ไม่เรียก ADC ของ ESP32 และไม่เสียบ analog เข้าขา IO2–IO8 โดยสมมติว่าอ่านได้
ข้อแก้ไขที่ต้องใช้: ใช้ external ADC ตามชนิด/gain/reference จริง; ADC0 onboard เป็นปุ่ม ไม่ใช่พอร์ต POT ทั่วไป
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: ค่าต่ำ กลาง สูงเปลี่ยนต่อเนื่องและสูตรตรงกับ ADC ที่ใช้

#### MP-062 — ADC ภายนอกทุกช่อง

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: ADC I2C พร้อม fixture หลายช่อง
ต้องมีก่อน: Actual external bus/backend, correct module wiring/voltage/address and matching device protocol/driver. No implicit module loading.; ผลจาก MP-055

```text
สร้างโปรเจกต์ mai_test_062 โดยส่ง test_id="MP-062" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-062") ก่อน ทำเทส MP-062: ไล่ทุกช่องของ ADC ที่ต่อจริง ระบุ single-ended หรือ differential ชัดเจน บันทึก sample rate, gain, raw และค่าจากมัลติมิเตอร์อ้างอิง ไม่อ้างว่าเทียบแล้วถ้าไม่ได้กรอกค่ามิเตอร์
ข้อแก้ไขที่ต้องใช้: เลือกชนิด single-ended/differential และช่วงแรงดันจริง ก่อนทดสอบทุกช่อง ADC ภายนอก
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: มีผลแต่ละช่องพร้อมค่าคลาดเคลื่อนที่คำนวณได้จริง

#### MP-063 — I2C ไม่มีอุปกรณ์และ NACK

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: fixture I2C; เปลี่ยนสายขณะปิดไฟ
ต้องมีก่อน: Actual external bus/backend, correct module wiring/voltage/address and matching device protocol/driver. No implicit module loading.; ผลจาก MP-055

```text
สร้างโปรเจกต์ mai_test_063 โดยส่ง test_id="MP-063" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-063") ก่อน ทำเทส MP-063: ทดสอบรอบไม่มีอุปกรณ์และรอบต่ออุปกรณ์บนบัสภายนอกที่ยืนยัน ให้ผู้ทดสอบปิดไฟก่อนเปลี่ยนสาย แยก NACK/None/exception จาก native call ค้าง ไม่แต่ง timeout argument ให้ I2C.readfrom หากค้างให้ผู้ทดสอบ Stop ผ่าน supervisor และบันทึกเวลาจนออก ไม่รีบูตเอง
ข้อแก้ไขที่ต้องใช้: API readfrom รุ่นนี้ไม่มี timeout ที่ยืนยัน; แยก NACK/None จาก native call ค้าง และใช้ Stop supervisor ตามจริง
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: รายงานสาเหตุถูกและกลับ Run ได้เมื่อแก้สาย

#### MP-064 — อ่าน ADC ผ่าน iKB-1Z

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: iKB-1Z คู่มือการจ่ายไฟ และเซนเซอร์ที่รองรับ
ต้องมีก่อน: Actual external bus/backend, correct module wiring/voltage/address and matching device protocol/driver. No implicit module loading.; ผลจาก MP-055

```text
สร้างโปรเจกต์ mai_test_064 โดยส่ง test_id="MP-064" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-064") ก่อน ทำเทส MP-064: ตรวจ protocol/ไลบรารีของ iKB-1Z ที่มีจริง แล้วอ่านทุกช่อง ADC ที่คู่มือรุ่นนั้นระบุ พร้อมให้กรอกการจ่ายไฟและการต่อ I2C ตามผู้ผลิตก่อนเริ่ม ไม่ต่อไฟย้อนจากสองแหล่งโดยเดา
ข้อแก้ไขที่ต้องใช้: iKB-1Z ต้องมี protocol/driver และการจ่ายไฟตามคู่มือรุ่นจริง ไม่ถือว่ามี Python library ติดตั้งแล้ว
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: ระบุบอร์ดขยาย ช่องจริง และค่าที่เปลี่ยนตาม fixture ครบ

### IMU และนาฬิกา RTC

#### MP-065 — IMU raw หกแกนและตรวจข้อมูลเพิ่มเติม

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_065 โดยส่ง test_id="MP-065" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-065") ก่อน ทำเทส MP-065: เปิด IMU ที่ยืนยันว่าเป็น QMI8658 อ่าน acceleration x/y/z และ gyro x/y/z หลังตรวจความยาวข้อมูล API dual mode รับประกันหกแกนเท่านั้น อุณหภูมิเป็น optional เมื่อ runtime มีข้อมูลและหน่วยที่ยืนยัน ห้าม index [6] โดยเดา
ข้อแก้ไขที่ต้องใช้: read() รับประกันหกแกนใน dual mode ไม่ใช่เจ็ดค่า; ตรวจ len และอย่า index [6] เพื่ออุณหภูมิโดยไม่มีหลักฐาน
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ครบค่าที่ API มีจริงและหน่วยถูกระบุชัด

#### MP-066 — IMU หกทิศทาง

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-065

```text
สร้างโปรเจกต์ mai_test_066 โดยส่ง test_id="MP-066" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-066") ก่อน ทำเทส MP-066: แนะนำให้วางบอร์ดหงาย คว่ำ และตั้งแต่ละด้าน เก็บค่าเฉลี่ยทุกท่าและระบุทิศแกนตามข้อมูลจริงก่อนทำ remap
ข้อแก้ไขที่ต้องใช้: ใช้หน่วย/ทิศแกนที่ยืนยัน; raw ผ่านหกท่าไม่ใช่ calibration ที่ใช้กับทุกบอร์ด
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: แกนที่รับแรงโน้มถ่วงเปลี่ยนสอดคล้องกับท่า ไม่อ้าง calibration สำเร็จจากค่าชั่วคราว

#### MP-067 — gyro ขณะนิ่งและหมุน

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-065

```text
สร้างโปรเจกต์ mai_test_067 โดยส่ง test_id="MP-067" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-067") ก่อน ทำเทส MP-067: เก็บ bias ขณะนิ่ง 5 วินาทีแบบไม่เขียนถาวร จากนั้นให้หมุนรอบสามแกน แสดงค่าดิบกับค่าหัก bias พร้อม rate และหน่วยจากไดรเวอร์
ข้อแก้ไขที่ต้องใช้: bias เป็นชั่วคราวใน RAM; ระบุหน่วย gyro ที่ตรวจแล้ว ไม่เขียนถาวร
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ขณะนิ่งค่าคงที่กว่าเดิมและการหมุนสะท้อนแกนที่ถูกต้อง

#### MP-068 — ลูกบอลตามความเอียง

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-065

```text
สร้างโปรเจกต์ mai_test_068 โดยส่ง test_id="MP-068" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-068") ก่อน ทำเทส MP-068: ใช้ค่า accelerometer ขยับลูกบอลบนจอ จำกัดขอบและทำ low-pass filter ที่ปรับได้ วางบอร์ดนิ่งแล้วเปรียบเทียบ raw/filtered ไม่ใช้ค่าอุณหภูมิเป็น ambient temperature
ข้อแก้ไขที่ต้องใช้: ไม่คัด x10/sign inversion ของ generator เป็น universal mapping; จำกัดตำแหน่งบนจอจริง
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ทิศลูกบอลสัมพันธ์กับท่าที่กำหนดและไม่หลุดขอบ

#### MP-069 — เขย่าและ tap แบบระบุวิธี

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-065

```text
สร้างโปรเจกต์ mai_test_069 โดยส่ง test_id="MP-069" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-069") ก่อน ทำเทส MP-069: ทำ shake detector จาก acceleration magnitude พร้อม threshold/refractory period ถ้าไม่มี hardware tap API ให้ระบุว่าเป็น software shake ห้าม import msa311 โดยสมมติว่าเป็นเซนเซอร์บนบอร์ดนี้
ข้อแก้ไขที่ต้องใช้: software shake ไม่ใช่ hardware tap; ห้ามใช้ msa311 แทน QMI8658
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: รายงาน event ตามวิธีที่ใช้และไม่อ้างรองรับ hardware tap ที่ไม่ได้ตรวจ

#### MP-070 — RTC อ่านเวลาและแหล่งข้อมูล

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_070 โดยส่ง test_id="MP-070" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-070") ก่อน ทำเทส MP-070: อ่านเวลาระบบกับ hardware RTC ที่เข้าถึงได้ แสดง UTC/timezone/เวลาท้องถิ่นแยกกัน ตรวจรุ่นจากไดรเวอร์จริง ไม่เลือกระหว่าง BM8653/BM8563 จากการสะกดที่คล้ายกัน และยังไม่ตั้งเวลา
ข้อแก้ไขที่ต้องใช้: ใช้ kernel RTC ที่มีจริงหรือ bm8563 API หลังยืนยัน chip/bus; ext_dev.rtc ไม่มีใน stub นี้
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: เห็นแหล่งเวลาแต่ละชุดและอ่าน RTC ไม่ได้ก็รายงานตรง

#### MP-071 — RTC เทียบเวลาจริง

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: นาฬิกาอ้างอิงของผู้ทดสอบ
ต้องมีก่อน: Actual hardware RTC source plus separately recorded reference times.; ผลจาก MP-070

```text
สร้างโปรเจกต์ mai_test_071 โดยส่ง test_id="MP-071" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-071") ก่อน ทำเทส MP-071: อ่าน RTC ต่อเนื่อง 10 นาที บันทึกจุดเริ่ม/จบและให้กรอกเวลาอ้างอิงเอง คำนวณ offset/drift โดยยังไม่เปลี่ยนเวลาเครื่อง
ข้อแก้ไขที่ต้องใช้: ต้องมีเวลาอ้างอิงจริงสองจุดและแยก wall clock jump/NTP ก่อนอ้าง drift
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: มีผลเปรียบเทียบที่อาศัยเวลาอ้างอิงจริง ไม่ใช่ RTC เทียบตัวเอง

#### MP-072 — RTC สำรองไฟ

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: แบตเตอรี่ RTC ติดตั้งถูกต้อง
ต้องมีก่อน: Manual retention protocol excluding automatic RTC/NTP sync; orderly power-off. No automatic power-cycle or service changes.; ผลจาก MP-070

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_072 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-072" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-072") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-072: สร้างขั้นตอนจดเวลาและปิดโปรแกรมให้เรียบร้อย จากนั้นให้ผู้ทดสอบปิดไฟบอร์ดตามปกติ รอหนึ่งนาทีแล้วเปิดใหม่ อ่าน RTC ก่อนการ sync เครือข่าย ถ้าพิสูจน์ว่า NTP ยังไม่แทรกไม่ได้ให้ผล INCONCLUSIVE
ข้อแก้ไขที่ต้องใช้: HOLD battery pass จนแยก NTP/auto RTC sync ได้; ไม่สั่งปิดไฟหรือแก้ service อัตโนมัติ
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: ยืนยันการเดินต่อของ hardware RTC ได้เฉพาะเมื่อแยกผล NTP ได้

### UART และ USB-UART

#### MP-073 — สำรวจ UART และเจ้าของพอร์ต

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_073 โดยส่ง test_id="MP-073" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-073") ก่อน ทำเทส MP-073: แสดง serial device ที่มีและเจ้าของพอร์ตเมื่ออ่านได้ เทียบ connector กับ pinmux จริง ทำเครื่องหมาย UART0/A16/A17 ว่าอาจถูกใช้โดย Maix Comm Protocol ไม่ปิดบริการหรือเปิดพอร์ตทับ
ข้อแก้ไขที่ต้องใช้: UART0/A16/A17 มีเจ้าของจาก log เดิม; list_devices ไม่ได้แปลว่าพอร์ตว่าง
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ระบุพอร์ตว่างที่ใช้งานได้จริง หรือ SKIP เมื่อแยกพอร์ตไม่ได้

#### MP-074 — UART loopback

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: UART ว่างที่ยืนยันแล้วและสาย TX→RX ที่ระดับถูกต้อง
ต้องมีก่อน: Free user UART path, actual connector pinmap, correct logic-level fixture/peer and matching framing/baud.; ผลจาก MP-073

```text
สร้างโปรเจกต์ mai_test_074 โดยส่ง test_id="MP-074" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-074") ก่อน ทำเทส MP-074: ใช้ UART สำหรับผู้ใช้ที่ว่างจริง ต่อ TX กับ RX ผ่านวงจรที่เหมาะสม ส่ง payload มี sequence/checksum ที่ 115200 ใช้ read timeout และนับ packet loss ห้ามแย่งพอร์ตระบบ
ข้อแก้ไขที่ต้องใช้: ใช้ user UART ที่ยืนยันและ read(timeout หน่วย ms) เท่านั้น ไม่ใช้พอร์ตระบบเป็น loopback
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: ข้อมูลรับกลับตรง sequence และ checksum

#### MP-075 — UART ต่อคอมพิวเตอร์

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: USB-TTL ที่ระดับลอจิกถูกต้องและ Terminal อีกฝั่ง
ต้องมีก่อน: Free user UART path, actual connector pinmap, correct logic-level fixture/peer and matching framing/baud.; ผลจาก MP-073

```text
สร้างโปรเจกต์ mai_test_075 โดยส่ง test_id="MP-075" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-075") ก่อน ทำเทส MP-075: ทำ echo server บน UART ว่างของ connector ที่ยืนยันแล้ว รับข้อความจบบรรทัด ส่งตอบพร้อมหมายเลขและเวลา โดยแยกจาก USB-UART console ของระบบ
ข้อแก้ไขที่ต้องใช้: USB-UART console กับ user UART แยกหน้าที่; ต้องยืนยัน connector/path ก่อนเปิด
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: ข้อความสองทิศทางถูกต้องและมีหลักฐานจากอีกฝั่ง

#### MP-076 — UART ข้อมูลไบนารีและ partial read

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: คู่ทดสอบ serial ที่ควบคุม payload ได้
ต้องมีก่อน: Free user UART path, actual connector pinmap, correct logic-level fixture/peer and matching framing/baud.; ผลจาก MP-073

```text
สร้างโปรเจกต์ mai_test_076 โดยส่ง test_id="MP-076" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-076") ก่อน ทำเทส MP-076: รับข้อมูลไบนารีรวม 00,0A,0D,7F,80,FF ใช้ framing และ length/checksum รองรับการอ่านแตกหลายรอบกับหลาย packet ในรอบเดียว จำกัด buffer
ข้อแก้ไขที่ต้องใช้: ใช้ byte framing และ bounded partial buffer ไม่ decode arbitrary bytes เป็นข้อความ
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: payload ไม่เสียจากการ decode เป็น text และ packet ถูกประกอบครบ

#### MP-077 — UART timeout และสายไม่ต่อ

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: fixture UART ว่าง
ต้องมีก่อน: Free user UART path, actual connector pinmap, correct logic-level fixture/peer and matching framing/baud.; ผลจาก MP-073

```text
สร้างโปรเจกต์ mai_test_077 โดยส่ง test_id="MP-077" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-077") ก่อน ทำเทส MP-077: ทดลองกรณีอีกฝั่งไม่ส่งข้อมูล ใช้ finite timeout และแสดงสถานะรอที่จอ กด Stop ระหว่าง read ต้องจบได้ จากนั้นค่อยต่อสายใหม่เมื่อปิดไฟ
ข้อแก้ไขที่ต้องใช้: read timeout=0 คือ immediate, -1 คือ block; ห้ามใช้ -1 ใน Stop-responsiveness test
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: โปรแกรมไม่แขวนและ UART ถูกคืนหลังจบ

#### MP-078 — UART เปรียบเทียบ baud rate

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: คู่ทดสอบ serial ที่ตั้ง baud ได้ตรงกัน
ต้องมีก่อน: Free user UART path, actual connector pinmap, correct logic-level fixture/peer and matching framing/baud.; ผลจาก MP-073

```text
สร้างโปรเจกต์ mai_test_078 โดยส่ง test_id="MP-078" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-078") ก่อน ทำเทส MP-078: ทดสอบ 9600,38400,115200 เฉพาะค่าที่ไดรเวอร์รองรับ ส่งแพตเทิร์น 100 packet ต่อค่า วัด error จาก checksum และบอกให้ตั้งอีกฝั่งตรงทุกครั้ง
ข้อแก้ไขที่ต้องใช้: ตั้ง baud อีกฝั่งตรงทุกครั้ง เก็บ checksum/sequence และรายงานค่าที่ driver ปฏิเสธแยกกัน
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: แสดงผลแยกแต่ละ baud ไม่ตีความค่าที่ไม่รองรับว่าเป็นพอร์ตเสีย

### กล้องและประมวลผลภาพพื้นฐาน

#### MP-079 — ภาพสดจากกล้อง

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_079 โดยส่ง test_id="MP-079" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-079") ก่อน ทำเทส MP-079: เปิดกล้อง RGB888 ขนาดเริ่มต้นที่ runtime รองรับเพียงครั้งเดียว แสดงภาพสดพร้อม FPS ตรวจข้อมูล sensor ที่พบจริง ไม่บังคับรุ่นจากแคตตาล็อก และ close ใน finally
ข้อแก้ไขที่ต้องใช้: ตัวอย่าง camera seed เดิมยังไม่มี FPS; ต้องเพิ่ม measured FPS และ close จริงก่อนเรียกว่า MP-079 ครบ
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: เห็นภาพเปลี่ยนเมื่อขยับวัตถุและ Stop คืนเมนู

#### MP-080 — กล้องพร้อมข้อความและวงกลม

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-079

```text
สร้างโปรเจกต์ mai_test_080 โดยส่ง test_id="MP-080" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-080") ก่อน ทำเทส MP-080: อ่านเฟรมใหม่แต่ละรอบ วาดเลขเฟรม เวลาที่ผ่าน และวงกลมลงเฟรมก่อน show ไม่วาด overlay ครั้งเดียวทิ้งไว้นอก loop
ข้อแก้ไขที่ต้องใช้: camera_overlay เป็น seed ของ overlay ต้องเพิ่มรายละเอียดตามพรอมท์ก่อนถือว่าครบ MP-080
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ภาพสดและ overlay อัปเดตพร้อมกัน ไม่สะสมรอยจากเฟรมก่อน

#### MP-081 — ทิศภาพและพิกัด

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-079

```text
สร้างโปรเจกต์ mai_test_081 โดยส่ง test_id="MP-081" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-081") ก่อน ทำเทส MP-081: แสดง crosshair กลางภาพและป้ายซ้ายขวา ให้ผู้ทดสอบขยับมือเพื่อยืนยันทิศ ทดลอง flip/mirror เฉพาะที่ API รองรับโดยคืนค่าหลังจบ
ข้อแก้ไขที่ต้องใช้: hmirror/vflip มี API แต่ต้องรู้ค่าเดิมก่อนเปลี่ยน/restore; ไม่ยืนยัน orientation จากรูปนิ่ง
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ทิศภาพและการวาดพิกัดสอดคล้องและตั้งค่ากลับได้

#### MP-082 — ความละเอียดหลายค่า

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-079

```text
สร้างโปรเจกต์ mai_test_082 โดยส่ง test_id="MP-082" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-082") ก่อน ทำเทส MP-082: ทดสอบ 320x240 กับ 640x480 เมื่อไดรเวอร์รองรับ ปิดกล้องก่อนเปิดค่าถัดไป บันทึกขนาดเฟรมจริง FPS และหน่วยความจำ ไม่อ้างว่าสูงกว่าที่ API ส่งมา
ข้อแก้ไขที่ต้องใช้: ปิด device เดิมก่อนเปิด resolution ใหม่ อ่านขนาดจริงและไม่สร้างกล้องทุกเฟรม
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ได้ผลแยกแต่ละความละเอียดพร้อมเหตุผลของค่าที่ใช้ไม่ได้

#### MP-083 — ถ่ายภาพเมื่อกดปุ่ม

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: บอร์ดอย่างเดียวและพื้นที่ทดสอบบนการ์ด
ต้องมีก่อน: MP-021 bands and a chosen bounded test output directory.; ผลจาก MP-021, MP-079

```text
สร้างโปรเจกต์ mai_test_083 โดยส่ง test_id="MP-083" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-083") ก่อน ทำเทส MP-083: แสดง live preview แล้วให้ S1 บันทึก JPEG หนึ่งไฟล์ต่อการกดลงไดเรกทอรีทดสอบ ชื่อไม่ชน ตรวจอ่านกลับและแสดง path/ขนาด ห้ามบันทึกทุกเฟรมเมื่อกดค้าง
ข้อแก้ไขที่ต้องใช้: capture หนึ่งครั้งต่อ press edge ใช้ bands จาก MP-021 และบันทึกเฉพาะ directory ทดสอบ
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: จำนวนไฟล์ตรงกับจำนวนกดและไฟล์เปิดกลับได้

#### MP-084 — ตรวจสีและติดตามวัตถุ

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: วัตถุสีแดง เขียว น้ำเงิน
ต้องมีก่อน: Actual colored targets or known QR/barcode payloads and the measured thresholds/geometry.; ผลจาก MP-079

```text
สร้างโปรเจกต์ mai_test_084 โดยส่ง test_id="MP-084" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-084") ก่อน ทำเทส MP-084: ตรวจความสามารถ color blob ก่อนใช้ ให้ผู้ทดสอบเลือกสีและปรับ threshold แสดงกรอบ centroid และพื้นที่ blob พร้อมค่าที่ใช้ ไม่มีโมเดล AI ก็ทำได้ถ้า API ภาพมี
ข้อแก้ไขที่ต้องใช้: threshold ตามแสง/วัตถุจริง; centroid/area ไม่ใช่ AI confidence
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: กรอบตามวัตถุและแจ้งว่า threshold มีผลตามสภาพแสง

#### MP-085 — อ่าน QR และบาร์โค้ด

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: ภาพ QR/บาร์โค้ดสำหรับทดสอบ
ต้องมีก่อน: Actual colored targets or known QR/barcode payloads and the measured thresholds/geometry.; ผลจาก MP-079

```text
สร้างโปรเจกต์ mai_test_085 โดยส่ง test_id="MP-085" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-085") ก่อน ทำเทส MP-085: ตรวจ API ที่มี แล้วอ่าน QR หรือ barcode แสดงชนิดกับข้อความทั้ง Terminal/จอ จำกัดความถี่ log และทำ deduplicate ถ้ารูปแบบใดไม่มี API ให้ SKIP
ข้อแก้ไขที่ต้องใช้: payload API ยืนยัน แต่ห้ามเปิด URL หรือ execute QR payload อัตโนมัติ
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: ข้อความที่อ่านตรงต้นฉบับ ไม่เปิด URL หรือสั่งงานตาม payload อัตโนมัติ

#### MP-086 — กล้องทำงานต่อเนื่อง

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-079

```text
สร้างโปรเจกต์ mai_test_086 โดยส่ง test_id="MP-086" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-086") ก่อน ทำเทส MP-086: เปิด preview 10 นาที เก็บ FPS/frame time/memory เป็นช่วงแล้วคงหน้าสรุป จัดการ None/error จาก read และให้ Stop กลางการจับภาพได้ ไม่เปิด Camera ใหม่ทุกเฟรม
ข้อแก้ไขที่ต้องใช้: FPS/memory/Stop เป็นสิ่งที่ต้องวัด ไม่รับรอง 10 นาทีจาก fixture สามเฟรม
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ไม่มี frame buffer เติบโตไม่จำกัดและ Stop/Run กล้องซ้ำทำได้

### ไมโครโฟนและลำโพง

#### MP-087 — เล่นเสียงทดสอบ

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_087 โดยส่ง test_id="MP-087" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-087") ก่อน ทำเทส MP-087: ตรวจ audio.Player และสร้าง raw PCM สั้นแบบ FMT_S16_LE ที่ amplitude ต่ำ ระบุ sample rate/channels เล่นรวมหนึ่งวินาทีเป็น chunk สั้นพร้อมเช็ก exit และปล่อย references เมื่อจบ ไม่พึ่ง volume() ซึ่ง reference ระบุ unsupported และไม่เรียก stop()/close() ที่ไม่มี
ข้อแก้ไขที่ต้องใช้: Player.volume ใน stub ระบุ unsupported; ใช้ PCM amplitude ต่ำแทนการพึ่ง setter และไม่ใช้ stop()/close() ที่ไม่มี
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ผู้ทดสอบได้ยินเสียงชัดและหยุดตามเวลา

#### MP-088 — ไมโครโฟนแสดงระดับเสียง

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_088 โดยส่ง test_id="MP-088" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-088") ก่อน ทำเทส MP-088: อ่าน PCM จากไมโครโฟนด้วย sample format ที่ตรวจแล้ว คำนวณ RMS/peak แสดง VU meter ขณะเงียบและพูด อย่าเรียกค่าดังกล่าวว่า dB SPL ถ้าไม่มีการสอบเทียบ
ข้อแก้ไขที่ต้องใช้: Recorder.record() คืน buffered PCM หรือ empty; ไม่มี Recorder.read(); RMS/peak ไม่ใช่ calibrated dB SPL
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ค่าเปลี่ยนตามเสียงจริงและแยก silent input จาก read failure

#### MP-089 — อัดเสียงแล้วเล่นกลับ

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียวและพื้นที่ทดสอบบนการ์ด
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-087, MP-088

```text
สร้างโปรเจกต์ mai_test_089 โดยส่ง test_id="MP-089" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-089") ก่อน ทำเทส MP-089: อัดคำพูด 5 วินาทีลง WAV ในโฟลเดอร์ทดสอบ ปิด recorder ก่อนเปิด player แล้วเล่นกลับหนึ่งครั้ง แสดง sample rate/channels/bit depth ที่ใช้จริง
ข้อแก้ไขที่ต้องใช้: ปิด recorder ด้วย finish(), ใช้ WAV metadata จริงและลด PCM playback amplitude; ไม่ผูก Stop กับ record_ms=5000 ที่ block
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ผู้ทดสอบได้ยินคำพูดเดิมและ metadata ของ WAV ถูกต้อง

#### MP-090 — เสียง clipping และความเงียบ

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run; ผลจาก MP-088

```text
สร้างโปรเจกต์ mai_test_090 โดยส่ง test_id="MP-090" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-090") ก่อน ทำเทส MP-090: เก็บช่วงเงียบ พูดปกติ และเสียงดังในระยะเหมาะสม วัด RMS/peak กับสัดส่วนตัวอย่างชนขอบช่วง PCM ใช้กราฟเปรียบเทียบโดยไม่อ้างว่าเป็นเครื่องวัดเสียงสอบเทียบ
ข้อแก้ไขที่ต้องใช้: signedness/endian/width ตรง Format.FMT_S16_LE; clipping ratio ต้องคำนวณจาก PCM จริง
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: เห็นสัญญาณจริงและระบุ clipping จากข้อมูล PCM

#### MP-091 — audio กับ Stop

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_091 โดยส่ง test_id="MP-091" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-091") ก่อน ทำเทส MP-091: ทำโหมดบันทึกหรือเล่นเสียงเป็น chunk สั้นที่หยุดได้ ให้ผู้ทดสอบกด Stop กลางงาน ใช้ Recorder.finish() และปิดไฟล์ใน finally ส่วน Player ปล่อย references ตาม API ไม่แต่ง stop()/close() ตรวจรอบใหม่และไม่ทำ feedback ไมค์ออกลำโพง
ข้อแก้ไขที่ต้องใช้: Player ไม่มี stop/close ใน baseline; ใช้ short PCM chunks/exit loop และปล่อย references; ไม่อ้างว่า finally รันหลัง kill
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: เสียงหยุด ไฟล์ที่ปิดครบอ่านได้ และเปิด audio รอบถัดไปได้

#### MP-092 — เสียงพร้อมกล้องและจอ

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: Camera and microphone pass independently; concurrent operation is still to be measured.; ผลจาก MP-079, MP-088

```text
สร้างโปรเจกต์ mai_test_092 โดยส่ง test_id="MP-092" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-092") ก่อน ทำเทส MP-092: แสดงภาพกล้องพร้อม VU meter จากไมค์โดยไม่เล่นออกลำโพง จำกัดอัตราอัปเดตแต่ละส่วน วัด dropped frames และ latency ที่สังเกตจริง ถ้ารuntime แชร์ทรัพยากรไม่ได้ให้รายงานข้อจำกัด
ข้อแก้ไขที่ต้องใช้: full-duplex/resource sharing ยังต้องวัด ไม่เปิด loopback ไมค์→ลำโพงเอง
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: ทั้งสองข้อมูลอัปเดต หรือได้ข้อผิดพลาดที่ระบุ subsystem ชัด

### เครือข่ายและ IoT

#### MP-093 — สำรวจ WiFi/RNDIS

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_093 โดยส่ง test_id="MP-093" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-093") ก่อน ทำเทส MP-093: อ่าน interface, IP, route และสถานะ WiFi แบบไม่เปลี่ยน config ไม่เผยรหัสผ่านหรือ token แยก USB RNDIS ที่ใช้ IDE กับ WiFi ที่ใช้เข้าบริการ
ข้อแก้ไขที่ต้องใช้: อ่าน interface/route แบบ bounded ไม่พิมพ์รหัสผ่าน token หรือ device_key
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: รายงานสองเส้นทางชัด และ Connect Plus ยังอยู่

#### MP-094 — ทดสอบ WiFi ที่เชื่อมอยู่

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: WiFi ที่ตั้งไว้แล้วและปลายทางทดสอบที่อนุญาต
ต้องมีก่อน: Explicit allowed endpoint/broker/LAN address/port; existing network; compatible MQTT/WebSocket package when used.; ผลจาก MP-093

```text
สร้างโปรเจกต์ mai_test_094 โดยส่ง test_id="MP-094" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-094") ก่อน ทำเทส MP-094: ใช้การเชื่อม WiFi เดิมทดสอบถึง gateway/ปลายทางที่ผู้ทดสอบระบุ ใช้ timeout วัด round trip แยก DNS/TCP failure ไม่สั่งเปลี่ยน SSID หรือปิด RNDIS
ข้อแก้ไขที่ต้องใช้: ใช้ network เดิมกับ endpoint ที่อนุญาต ไม่เปลี่ยน SSID/route/RNDIS เพื่อให้เทสผ่าน
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: บอกได้ว่าล้มที่ชั้นใดโดยไม่ตัดการเชื่อม IDE

#### MP-095 — HTTP GET JSON

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: endpoint สำหรับทดสอบที่ผู้ใช้ระบุ
ต้องมีก่อน: Explicit allowed endpoint/broker/LAN address/port; existing network; compatible MQTT/WebSocket package when used.; ผลจาก MP-093

```text
สร้างโปรเจกต์ mai_test_095 โดยส่ง test_id="MP-095" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-095") ก่อน ทำเทส MP-095: ดึง JSON ด้วย timeout ตรวจ HTTP status กับ schema แสดงค่าบนจอ รองรับ JSON เสีย/field ขาด ไม่ใส่ข้อมูลส่วนตัวใน URL
ข้อแก้ไขที่ต้องใช้: ใช้ urllib.request ไม่ใช่ maix.http.get ซึ่งไม่มีใน API นี้; timeout/status/schema แยกกัน
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: แสดงข้อมูลจริงและ error ที่ถูกต้อง ไม่ใช้ข้อมูลจำลองแทนเมื่อเน็ตล้ม

#### MP-096 — HTTP POST ผลทดสอบ

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: endpoint ส่วนตัวที่ผู้ใช้อนุญาต
ต้องมีก่อน: Explicit allowed endpoint/broker/LAN address/port; existing network; compatible MQTT/WebSocket package when used.; ผลจาก MP-093

```text
สร้างโปรเจกต์ mai_test_096 โดยส่ง test_id="MP-096" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-096") ก่อน ทำเทส MP-096: ส่งเฉพาะ test ID, ค่าที่วัด และสถานะไป endpoint ที่กำหนด ตรวจ response หลีกเลี่ยง token ใน log และไม่ส่งเสียง ภาพ หรือ credential ออกไป
ข้อแก้ไขที่ต้องใช้: ส่งได้เฉพาะ endpoint/payload ที่ผู้ทดสอบระบุ ไม่สร้างปลายทางสาธารณะหรือส่งภาพ/เสียงเอง
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: ฝั่งรับยืนยันข้อมูลตรงและไม่มี secret ใน Terminal

#### MP-097 — MQTT publish/subscribe

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: broker และ topic ทดสอบที่ผู้ใช้ระบุ
ต้องมีก่อน: Explicit allowed endpoint/broker/LAN address/port; existing network; compatible MQTT/WebSocket package when used.; ผลจาก MP-093

```text
สร้างโปรเจกต์ mai_test_097 โดยส่ง test_id="MP-097" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-097") ก่อน ทำเทส MP-097: ตรวจ MQTT library ที่มีจริง ใช้ namespace topic เฉพาะทดสอบ ส่ง IMU หรือตัวนับและรับคำสั่งเปลี่ยนสีจอ กำหนด reconnect/backoff ไม่ hard-code credential
ข้อแก้ไขที่ต้องใช้: ต้องมี compatible MQTT package/broker/topic/credential จริง ไม่ assume maix.network.mqtt หรือ pip เอง
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: เห็น publish และ subscribe จริงทั้งสองฝั่งและไม่ใช้ retained command เก่าขับโหลด

#### MP-098 — เครือข่ายหลุดแล้วกลับมา

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: เครือข่ายทดสอบที่ควบคุมได้
ต้องมีก่อน: Explicit allowed endpoint/broker/LAN address/port; existing network; compatible MQTT/WebSocket package when used.; ผลจาก MP-093

```text
สร้างโปรเจกต์ mai_test_098 โดยส่ง test_id="MP-098" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-098") ก่อน ทำเทส MP-098: ทำหน้าสถานะออนไลน์/offline ใช้ request timeout และ backoff แล้วให้ผู้ทดสอบทำปลายทางใช้งานไม่ได้ชั่วคราวโดยไม่ถอด USB RNDIS ไม่เขียน queue ไม่จำกัด
ข้อแก้ไขที่ต้องใช้: จำกัด queue/backoff และทดสอบปลายทางหลุดโดยไม่ตัด USB ที่ใช้ IDE
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: จอไม่ค้าง queue มีขอบเขตและกลับออนไลน์ได้

#### MP-099 — WebSocket แอปแยกจาก IDE

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: WebSocket endpoint ทดสอบ
ต้องมีก่อน: Explicit allowed endpoint/broker/LAN address/port; existing network; compatible MQTT/WebSocket package when used.; ผลจาก MP-093

```text
สร้างโปรเจกต์ mai_test_099 โดยส่ง test_id="MP-099" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-099") ก่อน ทำเทส MP-099: เปิด WebSocket สำหรับแอปบน URL/port ที่กำหนดต่างจากพอร์ต 5050 ของ IDE รับส่ง sequence และ heartbeat มี timeout ปิด socket ใน finally ไม่แย่ง ws_shell
ข้อแก้ไขที่ต้องใช้: ต้องมี WebSocket package/API จริง แยก port 5050 ของ IDE และปิดตาม library ที่เลือก
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: ข้อมูลสองทิศทางตรงและ Stop/Connect IDE ยังทำงาน

#### MP-100 — HTTP server ใน LAN

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: คอมพิวเตอร์บน LAN เดียวกัน
ต้องมีก่อน: Explicit allowed endpoint/broker/LAN address/port; existing network; compatible MQTT/WebSocket package when used.; ผลจาก MP-093

```text
สร้างโปรเจกต์ mai_test_100 โดยส่ง test_id="MP-100" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-100") ก่อน ทำเทส MP-100: ทำเว็บสถานะ read-only บน port ว่างที่ไม่ใช่ 5050 ให้ client อ่านตัวนับ/IMU เป็น JSON จำกัดจำนวน client และปิด server เมื่อ Stop ไม่เปิดพอร์ต router หรือเผยแพร่สู่ Internet
ข้อแก้ไขที่ต้องใช้: ใช้ HTTPServer.timeout + handle_request ใน exit-aware loop แล้ว server_close; หลีกเลี่ยง same-thread shutdown deadlock
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: อ่านได้จากอีกเครื่องใน LAN และ port ถูกคืนหลัง Stop

### ไฟล์ การ์ด และข้อมูลคงอยู่

#### MP-101 — พื้นที่จัดเก็บและ permission

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_101 โดยส่ง test_id="MP-101" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-101") ก่อน ทำเทส MP-101: รายงาน mount, พื้นที่ว่างและตำแหน่งที่เขียนทดสอบได้อย่างมีขอบเขต อ่านอย่างเดียวก่อน ไม่ format ไม่ fsck ไม่แตะ partition image
ข้อแก้ไขที่ต้องใช้: read-only storage inventory ไม่มี format/fsck หรือเขียนทดสอบจนเต็ม
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: แสดงพื้นที่/สิทธิ์จริงและระบุตำแหน่งทดสอบชัด

#### MP-102 — เขียนอ่านไฟล์เล็ก

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: โฟลเดอร์ทดสอบที่กำหนด
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_102 โดยส่ง test_id="MP-102" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-102") ก่อน ทำเทส MP-102: เขียนไฟล์ทดสอบขนาด 64KiB เฉพาะโฟลเดอร์ที่กำหนด flush/close แล้วอ่านกลับเทียบ SHA-256 ไม่ใช้ RAM cache ของแอปเป็นหลักฐาน และไม่เขียนเต็มการ์ด การอ่านกลับเทียบ hash ตรวจความถูกต้องของ file I/O เท่านั้น ไม่พิสูจน์การเขียนลงสื่อจริงหรือการคงข้อมูลหลังไฟดับ และไม่สั่งล้าง kernel cache
ข้อแก้ไขที่ต้องใช้: unique test file ภายใต้พื้นที่ของโปรแกรม ขนาด 64KiB; ปิดแล้วอ่านกลับเทียบ hash
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: hash ของข้อมูลที่อ่านกลับตรงและไฟล์อยู่นอก system directory

#### MP-103 — CSV บันทึกเซนเซอร์

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: IMU หรือเซนเซอร์ที่อ่านผ่านแล้ว
ต้องมีก่อน: Actual sensor inputs for logging or a chosen persistent directory for config across new /tmp runs.

```text
สร้างโปรเจกต์ mai_test_103 โดยส่ง test_id="MP-103" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-103") ก่อน ทำเทส MP-103: บันทึก timestamp แบบ monotonic ควบคู่เวลาปฏิทิน ค่าเซนเซอร์และ status ลง CSV สองนาที จำกัดขนาดไฟล์และปิดครบใน finally แสดงจำนวนแถวที่อ่านตรวจกลับได้
ข้อแก้ไขที่ต้องใช้: sensor ต้องผ่านแยกก่อน; log error/status ไม่เติมเลข 0 เป็นข้อมูล sensor ปกติ
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: CSV เปิดอ่านได้ จำนวนแถวตรง และไม่ใส่ค่าเซนเซอร์ปลอมเมื่อ read error

#### MP-104 — บันทึก config แบบ atomic

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: โฟลเดอร์ทดสอบที่กำหนด
ต้องมีก่อน: Actual sensor inputs for logging or a chosen persistent directory for config across new /tmp runs.

```text
สร้างโปรเจกต์ mai_test_104 โดยส่ง test_id="MP-104" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-104") ก่อน ทำเทส MP-104: ทำ config JSON สำหรับสีจอและค่าทดสอบในโฟลเดอร์เฉพาะ เขียน temp แล้ว replace รองรับไฟล์ JSON เสียโดยแจ้งเตือนและใช้ค่า default ไม่ทับ config ของ IDE/Launcher
ข้อแก้ไขที่ต้องใช้: แต่ละ Run ใช้ /tmp ใหม่ จึงต้องเลือก persistent directory ก่อนอ้าง config อยู่ข้าม Run
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: เปิดรอบใหม่อ่านค่าเดิมได้และไฟล์เสียไม่ทำแอปล้ม

#### MP-105 — ไฟล์และ path ที่มีช่องว่าง/ไทย

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: โฟลเดอร์ทดสอบที่กำหนด
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_105 โดยส่ง test_id="MP-105" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-105") ก่อน ทำเทส MP-105: สร้างอ่านไฟล์ทดสอบชื่อมีช่องว่างและภาษาไทยผ่าน pathlib/open ไม่ต่อชื่อเข้า shell ตรวจเนื้อหา UTF-8 แล้วลบเฉพาะไฟล์ที่โปรแกรมสร้างเมื่อผู้ทดสอบเลือก cleanup
ข้อแก้ไขที่ต้องใช้: ใช้ Python path/file API ไม่ต่อชื่อไทยหรือชื่อมีช่องว่างเข้าคำสั่ง shell
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: path ไม่ถูกตัดหรือ shell ตีความและเนื้อหาตรง

#### MP-106 — Run ชั่วคราวกับข้อมูลถาวร

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: โฟลเดอร์เก็บข้อมูลถาวรสำหรับทดสอบที่ยืนยันแล้ว
ต้องมีก่อน: Chosen persistent test directory; closed files and orderly restart procedure.

```text
สร้างโปรเจกต์ mai_test_106 โดยส่ง test_id="MP-106" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-106") ก่อน ทำเทส MP-106: แสดงว่าโค้ดรันอยู่ใต้ /tmp และให้เลือกโฟลเดอร์ข้อมูลถาวรเอง สร้าง counter ขนาดเล็กที่นั่น ทดสอบ Stop/Run แล้วค่อยปิดเปิดบอร์ดตามปกติหลังปิดไฟล์ครบ ไม่ติดตั้ง boot app
ข้อแก้ไขที่ต้องใช้: ยืนยันตำแหน่งถาวรและ orderly restart; ห้ามแนะนำถอดไฟระหว่างเขียน SD
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: ข้อมูลคงอยู่ตามพื้นที่จริงโดยแยกจากอายุโฟลเดอร์ /tmp

### AI แบบมีโมเดลที่เข้ากันได้

#### MP-107 — ตรวจไฟล์โมเดลก่อน inference

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: โมเดลสำหรับ MaixPy/SG2002 ที่ผู้ใช้มี
ต้องมีก่อน: Real SG2002-compatible model/labels/input metadata; exact audio preprocessing when applicable; ground-truth fixtures.

```text
สร้างโปรเจกต์ mai_test_107 โดยส่ง test_id="MP-107" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-107") ก่อน ทำเทส MP-107: ตรวจ model file, label, input/output metadata และ API ที่จะใช้ แสดงหน่วยความจำก่อนโหลด ไม่โหลด ONNX/TFLite ตรง ๆ ถ้า runtime ต้องใช้รูปแบบเฉพาะ และไม่ดาวน์โหลดหรือเทรนโมเดลเอง
ข้อแก้ไขที่ต้องใช้: ตรวจ compatible model metadata จากไฟล์จริง ไม่ถือว่า NN API มีแล้วโมเดลใดก็โหลดได้
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: แจ้งได้ว่าโมเดลพร้อมหรือขาดอะไร โดยไม่สร้างผล inference จำลอง

#### MP-108 — image classification

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: โมเดล classification ที่รองรับและภาพทดสอบที่มีคำตอบ
ต้องมีก่อน: Real SG2002-compatible model/labels/input metadata; exact audio preprocessing when applicable; ground-truth fixtures.; ผลจาก MP-107

```text
สร้างโปรเจกต์ mai_test_108 โดยส่ง test_id="MP-108" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-108") ก่อน ทำเทส MP-108: โหลดโมเดลครั้งเดียว รันภาพที่กำหนด แสดง label, score, preprocessing และเวลาที่วัดจริง เทียบกับคำตอบที่ผู้ทดสอบระบุ แยกความแม่นยำออกจากการโหลดสำเร็จ
ข้อแก้ไขที่ต้องใช้: Classifier.classify ยืนยัน API แต่ต้องมี model/labels/preprocessing/ground truth ที่ตรงกัน
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: มีผลจากโมเดลจริงและนับถูก/ผิดตามชุดทดสอบ

#### MP-109 — object detection

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: โมเดล detection ที่เข้ากันได้และวัตถุทดสอบ
ต้องมีก่อน: Real SG2002-compatible model/labels/input metadata; exact audio preprocessing when applicable; ground-truth fixtures.; ผลจาก MP-107, MP-079

```text
สร้างโปรเจกต์ mai_test_109 โดยส่ง test_id="MP-109" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-109") ก่อน ทำเทส MP-109: แสดงกล้องพร้อมกรอบ detection ทำ mapping พิกัดจาก input โมเดลกลับสู่ภาพอย่างถูกต้อง ระบุ confidence/NMS และ inference time ไม่อ้าง FPS จากเฟรมจอแทนโมเดล
ข้อแก้ไขที่ต้องใช้: YOLO API รุ่นนี้มี v5/v8/11; เลือกจาก model metadata ไม่เดาจากนามสกุลอย่างเดียว
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: กรอบตรงวัตถุและผลเป็น inference จริง

#### MP-110 — เสียง classification

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: โมเดลเสียงที่รองรับและตัวอย่างเสียงที่มีคำตอบ
ต้องมีก่อน: Real SG2002-compatible model/labels/input metadata; exact audio preprocessing when applicable; ground-truth fixtures.; ผลจาก MP-107, MP-088

```text
สร้างโปรเจกต์ mai_test_110 โดยส่ง test_id="MP-110" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-110") ก่อน ทำเทส MP-110: ตรวจ sample rate, window และ preprocessing ของโมเดลให้ตรงกับ PCM ไมค์ แสดง top-k และเวลาประมวลผล เทียบอย่างน้อยสามประเภทพร้อมเสียงนอกกลุ่ม
ข้อแก้ไขที่ต้องใช้: audio preprocessing เป็นของแต่ละโมเดล ต้องมี specification ก่อน inference; ไม่ใช้ Speech แทน generic classifier
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: ไม่ป้อนข้อมูลผิดรูปแบบและมีผลเทียบ ground truth ที่ผู้ใช้กำหนด

#### MP-111 — AI สั่ง LED ด้วย threshold

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: โมเดลที่ผ่านแล้วและ LED โหลดต่ำบน IO ที่ยืนยันแล้ว
ต้องมีก่อน: Real SG2002-compatible model/labels/input metadata; exact audio preprocessing when applicable; ground-truth fixtures. / A passing chosen inference test, plus a verified user LED (MP-025) or an individually verified GPIO LED fixture.

```text
สร้างโปรเจกต์ mai_test_111 โดยส่ง test_id="MP-111" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-111") ก่อน ทำเทส MP-111: ให้ผล AI ที่เกิน threshold ต่อเนื่องตามจำนวนครั้งที่กำหนดจึงติด LED ใช้ hysteresis/cooldown และดับเมื่อ inference error หรือ Stop ไม่ต่อ motor/relay ในชุดแรก
ข้อแก้ไขที่ต้องใช้: ต้องผ่าน inference และ GPIO แยกก่อน; first fixture เป็น LED ไม่เป็น relay/motor
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: LED ตอบสนองผลจริงโดยไม่กระพริบจาก label แกว่ง

#### MP-112 — AI หลายรอบและคืนหน่วยความจำ

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: โมเดลที่ผ่านการโหลดแล้ว
ต้องมีก่อน: Real SG2002-compatible model/labels/input metadata; exact audio preprocessing when applicable; ground-truth fixtures. / A passing chosen inference test (MP-108, MP-109 or MP-110), with its actual cleanup contract.

```text
สร้างโปรเจกต์ mai_test_112 โดยส่ง test_id="MP-112" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-112") ก่อน ทำเทส MP-112: ให้ผู้ทดสอบ Run inference → Stop → Run ซ้ำห้ารอบ เก็บ memory ก่อน/หลังและเวลาที่โมเดลโหลดจริง ปิด camera/audio ตามที่ใช้ ไม่สร้าง instance โมเดลทุกเฟรม
ข้อแก้ไขที่ต้องใช้: โหลดโมเดลครั้งเดียวต่อรอบ ไม่สร้าง NN/Camera ทุกเฟรม; leak ต้องวัดจริงหลายรอบ
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: รอบใหม่ไม่เจอ resource busy และรายงานแนวโน้ม memory ตามข้อมูลจริง

### Run/Stop ข้อผิดพลาด และความทนทาน

#### MP-113 — Run/Stop จอซ้ำ 20 รอบ

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว; ผู้ทดสอบกดจาก IDE
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_113 โดยส่ง test_id="MP-113" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-113") ก่อน ทำเทส MP-113: สร้างแอปจอแสดง RUN กับเวลาที่ผ่านและ exit-aware loop พร้อม finally log ให้ผู้ทดสอบ Run/Stop 20 รอบจาก IDE จดเวลาเข้าโปรแกรม ออกโปรแกรม และเมนูกลับ ห้ามให้ main.py ฆ่า Launcher เอง
ข้อแก้ไขที่ต้องใช้: ผู้ทดสอบกด Run/Stop จาก IDE 20 รอบ ไม่ให้ main.py คุม Launcher หรือสร้างผลผ่านเอง
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: แต่ละรอบรับ exit status จริงและผู้ทดสอบยืนยันเมนูกลับครบ

#### MP-114 — จบโปรแกรมตามปกติ

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_114 โดยส่ง test_id="MP-114" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-114") ก่อน ทำเทส MP-114: แสดง countdown ห้าวินาทีแล้วจบ main.py เองด้วย exit code 0 ปิดทรัพยากรใน finally ไม่ใช้ os._exit และไม่เรียก restart Launcher เอง
ข้อแก้ไขที่ต้องใช้: return/exit 0 ปกติและ finally cleanup; ไม่ใช้ os._exit ที่ข้าม cleanup
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: IDE เห็น exit 0 และ adapter คืนเมนูได้

#### MP-115 — Python exception ที่ตั้งใจให้เกิด

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว; ไม่มีโหลดภายนอก
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_115 โดยส่ง test_id="MP-115" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-115") ก่อน ทำเทส MP-115: แสดงภาพสองวินาทีแล้ว raise RuntimeError ที่มี test ID อย่างตั้งใจ ให้ finally พิมพ์ cleanup log โดยไม่กลบ exception ตรวจจาก IDE ว่าได้ traceback และ nonzero exit ไม่ใช่ PASS
ข้อแก้ไขที่ต้องใช้: NEGATIVE TEST: RuntimeError/nonzero เป็นผลที่ตั้งใจและต้องตรวจ traceback/menu ไม่ใช่ hardware PASS
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ข้อผิดพลาดมองเห็น เมนูคืน และ Run โปรแกรมปกติถัดไปได้

#### MP-116 — SyntaxError ก่อนเปิดอุปกรณ์

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว; โปรเจกต์ทดสอบแยก
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_116 โดยส่ง test_id="MP-116" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-116") ก่อน ทำเทส MP-116: เตรียมสำเนาโปรแกรมสำหรับทดสอบ SyntaxError โดยไม่แก้โปรเจกต์ใช้งานจริง อธิบายว่าความล้มเหลวเป็นผลที่ตั้งใจ แล้วให้ทดสอบผ่าน IDE ว่า log/exit/menu ถูกต้อง
ข้อแก้ไขที่ต้องใช้: NEGATIVE TEST: SyntaxError ตั้งใจไม่ผ่าน compile ต้องใช้โปรเจกต์แยกและไม่แก้โปรแกรมใช้งานจริง
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ไม่รายงาน success เมื่อ parser ล้ม และแก้ไฟล์แล้วรันใหม่ได้

#### MP-117 — ImportError ของโมดูลที่ไม่มี

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: บอร์ดอย่างเดียว; ไม่มีโหลดภายนอก
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
สร้างโปรเจกต์ mai_test_117 โดยส่ง test_id="MP-117" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-117") ก่อน ทำเทส MP-117: ทำโหมด preflight ที่แจ้งโมดูลขาดอย่างเป็นระเบียบ กับโหมดทดสอบ ImportError ที่ปล่อย exception ตั้งใจ ทั้งคู่ต้องปิดทรัพยากรที่เปิดแล้ว ไม่ติดตั้งแพ็กเกจอัตโนมัติ
ข้อแก้ไขที่ต้องใช้: แยก missing-package SKIP กับ intentional ImportError/nonzero ไม่ติดตั้ง dependency เอง
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: แยก SKIP จาก runtime failure ชัดเจน และไม่ค้าง Launcher

#### MP-118 — Stop ระหว่างรอ I/O

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: fixture ที่มี read timeout หรือ network endpoint ทดสอบ
ต้องมีก่อน: A chosen documented finite-timeout I/O path and controlled test peer. Native I2C timeout is not assumed.

```text
สร้างโปรเจกต์ mai_test_118 โดยส่ง test_id="MP-118" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-118") ก่อน ทำเทส MP-118: ทำการอ่าน UART/socket แบบ finite timeout ให้ Stop ผ่าน exit-aware loop และ signal cleanup ทดสอบไม่มีข้อมูลเข้าพร้อมจับเวลาจน exit ห้ามใช้ลูปรอข้อมูลไม่สิ้นสุด
ข้อแก้ไขที่ต้องใช้: ใช้ I/O ที่มี finite-timeout จริง; ห้ามสมมติว่า I2C ทุก native call หยุดได้จาก Python loop
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: Stop จบในเวลาที่อธิบายได้และไม่มีเจ้าของพอร์ตตกค้าง

#### MP-119 — ทรัพยากรหลายส่วนทำงานนาน

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: เฉพาะส่วนที่ทดสอบผ่านแล้ว
ต้องมีก่อน: Separate passing results for every selected subsystem; memory/log bounds.; ผลจาก MP-015, MP-065, MP-079

```text
สร้างโปรเจกต์ mai_test_119 โดยส่ง test_id="MP-119" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-119") ก่อน ทำเทส MP-119: รวมจอ IMU touch และกล้องตามที่อุปกรณ์รองรับ ทำงาน 30 นาที เก็บ memory/CPU/FPS/error เป็นช่วง จำกัด buffer/log และหยุดได้ทุกช่วง ไม่จงใจทำ OOM หรือเขียนเต็มการ์ด
ข้อแก้ไขที่ต้องใช้: เลือกเฉพาะ subsystem ที่ผ่านก่อน จำกัด memory/log และไม่จงใจ OOM
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: มีสถิติตลอดรอบ ไม่อ้าง PASS ด้านที่ผู้ทดสอบยังไม่ยืนยัน

#### MP-120 — API รุ่นใหม่หรือบางตัวหาย

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: บอร์ด baseline; fixture ซอฟต์แวร์สำหรับทดสอบ compatibility
ต้องมีก่อน: Desktop synthetic compatibility fixtures only; do not upload as a newer-runtime hardware test.

```text
จัดทำชุดทดสอบ SYNTHETIC บนคอมพิวเตอร์ ห้าม upload ขึ้นบอร์ด สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-120") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-120: แยก preflight ของแต่ละฟีเจอร์ ถ้าขาด API ให้แจ้งชื่อและ SKIP ฟีเจอร์นั้น ทดสอบ logic ด้วย fixture รุ่นสมมติบนคอมพิวเตอร์และติดป้าย SYNTHETIC ห้ามเปลี่ยนเวอร์ชันหรืออัปเดต OS บอร์ดเอง
ข้อแก้ไขที่ต้องใช้: DESKTOP SYNTHETIC ONLY ไม่ส่งขึ้นบอร์ดเพื่ออ้างว่าได้ทดสอบเวอร์ชันใหม่ที่ติดตั้ง
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: baseline ใช้ API ที่มีได้ รุ่นสมมติไม่ถูกนับเป็น hardware pass ของรุ่นใหม่

### ส่วนขยายที่ต้องยืนยันเพิ่ม และชุดรวม

#### MP-121 — SPI เฉพาะเมื่อมี connector/pinmux ที่ยืนยัน

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: โมดูล SPI ที่ทราบรุ่นและเอกสารการต่อสำหรับ Plus
ต้องมีก่อน: Actual exposed peripheral/adapter, pinmux/driver, ownership and matching public device documentation.

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_121 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-121" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-121") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-121: ตรวจว่ามี SPI ที่ออก connector ของ Plus และไม่ได้ชนจอ การ์ด WiFi หรือ system owner จริงหรือไม่ ถ้ายืนยันไม่ได้ให้ SKIP ถ้าได้ให้อ่าน ID register ที่เอกสารอุปกรณ์ระบุและเทียบค่า โดยไม่เขียน flash
ข้อแก้ไขที่ต้องใช้: HOLD SPI จนมี connector/pinmux/driver/ownership ของ Plus จริง ไม่ยืม pinout MaixCAM2
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: มีหลักฐาน pin/bus/ID จริง หรือ SKIP ที่ระบุสิ่งที่ขาด

#### MP-122 — Bluetooth แบบตรวจความพร้อม

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: บอร์ดอย่างเดียวหรือ dongle ที่ทราบรุ่น
ต้องมีก่อน: Actual exposed peripheral/adapter, pinmux/driver, ownership and matching public device documentation.

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_122 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-122" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-122") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-122: ตรวจว่าฮาร์ดแวร์และ driver Bluetooth มีจริงก่อน อย่าอนุมานจากการมี WiFi ถ้ามีให้ scan เฉพาะอุปกรณ์ทดสอบของผู้ใช้และแสดง capability ถ้าไม่มีให้ SKIP ไม่ติดตั้ง driver
ข้อแก้ไขที่ต้องใช้: HOLD Bluetooth จนพบ adapter และ driver จริง WiFi ไม่พิสูจน์ Bluetooth
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: รายงานตาม adapter จริงและไม่มีการจับคู่อุปกรณ์อื่นอัตโนมัติ

#### MP-123 — USB-OTG และ USB-UART

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: สาย USB ที่รองรับข้อมูล; เอกสารพอร์ต
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_123 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-123" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-123") ก่อน ทำเทส MP-123: ทำคู่มือทดสอบแยก USB RNDIS/IDE กับ USB-UART console ตรวจ enumerated device และทิศทางสาย ไม่สลับ OTG เป็น host ระหว่างรันผ่าน RNDIS และไม่เปลี่ยน console boot config
ข้อแก้ไขที่ต้องใช้: เป็นขั้นตอน manual/discovery แยก OTG/RNDIS กับ console ไม่แก้ boot config หรือ USB role
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ระบุได้ว่าขั้วใดใช้หน้าที่ใดโดย IDE ไม่ถูกตัดจากการตั้งค่าของแอป

#### MP-124 — อุปกรณ์ USB host แบบมีเงื่อนไข

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: แหล่งจ่ายและ USB host adapter ที่ตรงคู่มือ; ช่องทางควบคุมสำรอง
ต้องมีก่อน: Separate control transport and verified USB/power arrangement, or an exact expansion protocol/power/driver fixture.

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_124 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-124" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-124") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-124: ตรวจความเป็นไปได้ของ USB host และความขัดแย้งกับ RNDIS ก่อน ถ้าต้องสลับพอร์ตที่ใช้ IDE ให้หยุดที่แผนต่อสายและขั้นตอนทดสอบแยก ไม่สลับ role อัตโนมัติ ไม่จ่ายไฟย้อน
ข้อแก้ไขที่ต้องใช้: HOLD USB host จนมี control transport สำรองและ power arrangement ที่ยืนยัน
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: มีแผนทดสอบที่แยกช่องทางควบคุมชัด หรือ SKIP เมื่อไม่มี fixture

#### MP-125 — servo/motor ผ่านบอร์ดขยาย

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: iKB-1Z หรือ driver ที่ระบุรุ่น; โหลดและไฟเลี้ยงตามคู่มือ
ต้องมีก่อน: Separate control transport and verified USB/power arrangement, or an exact expansion protocol/power/driver fixture.

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_125 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-125" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-125") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-125: อ่าน protocol กับข้อจำกัดบอร์ดขยายก่อน ทำ dry-run ของคำสั่งบนจอก่อนต่อโหลด จากนั้นให้ผู้ทดสอบเลือก enable ใช้ความเร็ว/มุมต่ำและ stop command ใน finally ห้ามขับ motor จาก GPIO ตรง
ข้อแก้ไขที่ต้องใช้: HOLD motor/expansion จน protocol/power/driver/enable fixture ระบุครบ ห้ามขับโหลดตรงจาก GPIO
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: โหลดจริงทำงานหลัง enable และหยุดได้ตามคำสั่งโดยไม่มีการจ่ายไฟผิด

#### MP-126 — dashboard รวม onboard

สถานะก่อนเทส: **NEEDS_SETUP** — รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
อุปกรณ์: บอร์ดอย่างเดียว; แต่ละฟีเจอร์ต้องผ่านแยกแล้ว
ต้องมีก่อน: Each selected component tested separately; calibration results for buttons.; ผลจาก MP-015, MP-021, MP-065, MP-079, MP-088

```text
สร้างโปรเจกต์ mai_test_126 โดยส่ง test_id="MP-126" ในเครื่องมือสร้างโปรเจกต์ สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-126") ก่อน ทำเทส MP-126: สร้าง dashboard มีแท็บจอ touch S1/S2 IMU กล้องและระดับไมค์ เปิดเฉพาะ subsystem ที่หน้าใช้งาน ปิดเมื่อเปลี่ยนหน้า มี self-test checklist ที่รอผู้ทดสอบยืนยันผลภาพ/เสียง
ข้อแก้ไขที่ต้องใช้: dashboard รวมต้องผ่าน constituent tests ก่อน ไม่ใช้ภาพ mock เป็น sensor จริง
สถานะ NEEDS_SETUP: รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ
```

เกณฑ์ตรวจ: สลับหน้าได้ไม่เกิด resource busy และไม่รายงาน PASS แทนสายตาผู้ทดสอบ

#### MP-127 — ชุดไล่ตรวจ IO ทั้งบอร์ด

สถานะก่อนเทส: **HOLD** — รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
อุปกรณ์: fixture GPIO/I2C/UART/PWM ตามรายการที่ต่อจริง
ต้องมีก่อน: Every connector and subsystem verified separately, including unresolved IO7/P24/PWM; no blanket all-IO actuation.

```text
จัดทำรายงาน discovery/แผน manual สำหรับ mai_test_127 ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="MP-127" และ python_example="capabilities" สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-127") ก่อน ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้) MP-127: สร้าง wizard ไล่ IO2–IO8 input/output, external I2C, UART และ IO7 PWM ทีละชุด ให้แสดงผังต่อสายก่อน START บันทึก observed/expected และรอปรับสายเมื่อปิดไฟ ไม่ตั้ง output ชนกันและ SKIP ชุดที่ไม่มี fixture
ข้อแก้ไขที่ต้องใช้: HOLD wizard ขับครบทุก IO จนกรณีแยกผ่าน รวม IO7; ทำ subset แล้วห้ามสรุปครบทั้งบอร์ด
สถานะ HOLD: รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้
```

เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์
เกณฑ์เป้าหมายหลังแก้ blocker: รายงานครอบคลุมทุกช่องแยกผล ไม่เหมารวมทั้งบอร์ดจากพอร์ตเดียว

#### MP-128 — สรุปผลและสร้าง regression suite

สถานะก่อนเทส: **DRAFT_REFERENCE** — มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
อุปกรณ์: ผลจริงจากการทดสอบก่อนหน้า
ต้องมีก่อน: ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run

```text
วิเคราะห์ผล สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="MP-128") ก่อน ทำเทส MP-128: รวบรวม test ID, version, pin mapping, fixture, expected, observed, status และ log ที่ผู้ทดสอบให้ แยก PASS/FAIL/SKIP/NOT_RUN/WAIT_USER/INCONCLUSIVE สร้างรายการทดสอบซ้ำที่สั้นลงจากเคสผ่านแล้ว ห้ามเติมผลจากการคอมไพล์หรือ upload สำเร็จ
ข้อแก้ไขที่ต้องใช้: วิเคราะห์ผลที่ผู้ใช้ส่งหรืออนุญาตให้อ่านเท่านั้น ไม่สร้าง PASS ให้แถวที่ยัง NOT_RUN และไม่ต้องสร้างโครง display project
สถานะ DRAFT_REFERENCE: มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน
```

เกณฑ์ตรวจ: ทุก PASS ย้อนกลับไปดูหลักฐานได้และเคสที่ยังไม่ทำคง NOT_RUN

## ขอบเขตหลักฐาน

API signatures มาจาก image 4.12.5 และเอกสาร primary source ที่อ้างใน guide; การพบ signature ไม่ใช่ผลทดลอง driver หรือสายจริง ค่าจาก simulator/fixture ไม่ถูกย้ายเป็น PASS ของบอร์ด และรุ่นจำลองไม่ใช่การทดสอบรุ่นใหม่ที่ติดตั้งจริง

ผลผู้ใช้ยืนยันก่อนหน้านี้มีเพียงวงจร Connect/Run/Stop กลับเมนูตาม [device-session.json](research/uai-plus-runtime-2026-10-26/device-session.json) ไม่ใช่ผลผ่าน 128 ข้อ
