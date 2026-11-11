# ผลตรวจพรอมท์ mAi Plus ทั้ง 128 ข้อ — 27 ตุลาคม 2026

ตรวจทุก ID เทียบ API ของ image, เอกสารที่บันเดิล, ตัวสร้างโปรเจกต์และข้อกำหนด fixture แล้ว ไม่ได้เรียกโมเดลสดสร้าง/รันครบ 128 โปรแกรม ผล audit แยกจากผลฮาร์ดแวร์ซึ่งยังเป็น NOT_RUN

| สถานะ | จำนวน | ความหมาย |
|---|---:|---|
| TEMPLATE_CHECKED | 7 | มีตัวอย่างตรงงานและทดสอบ fixture แล้ว |
| DRAFT_REFERENCE | 43 | มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน |
| NEEDS_SETUP | 60 | รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ |
| HOLD | 18 | รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้ |

## ข้อผิดพลาด/ช่องว่างที่แก้

- MP-015/021 มี API และเทมเพลตจริง; MP-001–005 เพิ่ม read-only preflight พร้อม metadata helper
- MP-020 ไม่มี multi-touch API ใน reference นี้; ไม่ใช้การแตะสองจุดเป็นเกณฑ์ผ่าน
- MP-065 แก้สมมติฐาน IMU เจ็ดค่าเป็นหกแกนและ optional data; ไม่มีการ index อุณหภูมิที่ไม่ได้ยืนยัน
- ใช้ bm8563 แทนโมดูล rtc ที่ไม่มี; battery retention ยังแยก NTP ไม่ได้จึง HOLD
- Player.stop/close ไม่มี และ volume ระบุ unsupported; แก้เป็น low-amplitude PCM/short chunks และ Recorder.finish
- get_pin_function ไม่มีใน 4.12.5; ไม่อ้างว่ารู้/คืน pinmux เดิมจาก getter ที่ไม่มี
- IO7/P24, servo helper/backend, all-seven-pin output, SPI/Bluetooth/USB-host/expansion รอหลักฐานก่อนขับ
- แยก HTTP stdlib ออกจาก maix.http.JpegStreamer และ package prerequisites ของ MQTT/WebSocket
- ระบุ negative tests ที่ตั้งใจล้ม, calibration prerequisites และไม่ให้ finally เป็นข้อพิสูจน์ safety เมื่อถูก kill

## รายข้อ

| ID | เทส | สถานะ | ตัวอย่าง | สิ่งที่แก้/ต้องยืนยัน |
|---|---|---|---|---|
| MP-001 | สำรวจเวอร์ชันจริง | TEMPLATE_CHECKED | capabilities | ใช้ capabilities; sys.device_id ไม่มีใน stub รุ่นนี้จึงเป็น optional/UNKNOWN ห้ามเรียก device_key |
| MP-002 | สำรวจ API ทีละโมดูล | TEMPLATE_CHECKED | capabilities | ตรวจ symbol แบบไม่สร้างอุปกรณ์ รายงาน ext_dev.bm8563 ไม่ใช่ ext_dev.rtc และไม่ import package เสริม |
| MP-003 | สำรวจพอร์ตและ pinmux | TEMPLATE_CHECKED | capabilities | มี get_pins/get_pin_functions แต่ไม่มี get_pin_function ใน reference 4.12.5; IO7/P24 ยังไม่ใช่ routing ที่ยืนยัน |
| MP-004 | สำรวจอุปกรณ์ Linux | TEMPLATE_CHECKED | capabilities | อ่านเฉพาะ node/name ที่ระบุ ไม่ dump process command line, credential หรือเปิด serial แย่งเจ้าของ |
| MP-005 | ระบุอุปกรณ์ที่อาจต่างจากแคตตาล็อก | TEMPLATE_CHECKED | capabilities | แยก sensor_cfg กับชนิด IMU/RTC ที่ยัง UNKNOWN; ชื่อโมดูลไม่ใช่ chip ID |
| MP-006 | จัดแผนตามอุปกรณ์ที่มี | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | เป็น planner ไม่ขับพอร์ตหรืออ้างผล self-test จน fixture ของชุดนั้นพร้อม |
| MP-007 | สีพื้นเต็มจอ | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | ทดสอบสีเป็นแกนหลัก; input เสริมใช้ได้หลัง MP-015 หรือ MP-021 ผ่านเท่านั้น |
| MP-008 | ขอบและทิศทางจอ | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | ปรับขนาดข้อความให้เห็นขอบและ orientation จริง ไม่อิง 640x480 ตายตัว |
| MP-009 | ข้อความและตัวเลข | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | ASCII เป็น baseline; หน้าไทยแยก conditional ต้องมีไฟล์ font ที่มีจริง |
| MP-010 | รูปทรงและการซ้อนภาพ | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | ใช้ draw_line/draw_rect/draw_circle และพิกัด integer จาก reference ที่บันเดิล |
| MP-011 | ภาพเคลื่อนไหวและ FPS | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | FPS วัดจากเฟรมที่ show สำเร็จ/monotonic elapsed ไม่ตั้งเกณฑ์ FPS ที่ยังไม่มี benchmark |
| MP-012 | ภาพขนาดเล็กกับการปรับสเกล | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | fit และอัตราส่วนจากขนาดจริง; อย่าคาดว่าภาพ 240x240 ต้องเต็มจอโดยไม่เกิด letterbox |
| MP-013 | ความสว่างจอ | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | get_backlight/set_backlight ใช้ 0–100; ต้องอ่านค่าเดิมได้ก่อนจึงเปลี่ยน และคืนใน finally |
| MP-014 | จอเปิดทิ้งต่อเนื่อง | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | ไม่สมมติ partial update API; ใช้ buffer ที่มีขอบเขตและเก็บสถิติจริง |
| MP-015 | ทัชพิกัดจริง | TEMPLATE_CHECKED | touchscreen | ใช้ python_example=touchscreen ซึ่งมี marker และ press/release จริง ไม่ใช่ display-only skeleton |
| MP-016 | ทัชเก้าจุด | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | แตะเป้าทีละจุด เก็บ raw และระยะคลาดเคลื่อน; ไม่เขียน calibration ถาวร |
| MP-017 | ลากวาดเส้น | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | CLEAR/EXIT ต้องตรวจ pressed และขอบกด; ไม่ลากเส้นข้ามช่วงปล่อยนิ้ว |
| MP-018 | ปุ่มทัชกดค้างและกดซ้ำ | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | ใช้ monotonic edge/hold timer แยกจาก rendering loop ไม่เพิ่มตัวนับทุกเฟรม |
| MP-019 | ทัชกับภาพที่ถูกย่อขยาย | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | รวม scale และ letterbox offset ทั้งสองทิศทาง; ทดสอบขอบดำและขอบปุ่ม |
| MP-020 | ความสามารถหลายจุดสัมผัส | HOLD | ต้องสร้างเฉพาะงาน | ลดขอบเขตเป็น SINGLE_TOUCH_ONLY capability report เพราะ API นี้ส่งได้หนึ่งจุด ไม่ใช้สองนิ้วเป็นเกณฑ์ผ่าน |
| MP-021 | สำรวจ ADC ของปุ่ม | TEMPLATE_CHECKED | adc_buttons | ใช้ python_example=adc_buttons; วัด 3 วินาที/สถานะก่อนเสนอ threshold และไม่ใช้ threshold เก่า |
| MP-022 | ปุ่ม S1 หนึ่งครั้งต่อการกด | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ใช้ bands จาก MP-021; กด 20 ครั้งเป็น ground truth ที่ผู้ทดสอบยืนยัน |
| MP-023 | ปุ่ม S2 และกดค้าง | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | threshold จาก calibration; long press ใช้ monotonic และเกิดครั้งเดียวต่อรอบ |
| MP-024 | ปุ่มสองตัวและสถานะกำกวม | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ไม่กดพร้อมกันจนบันทึก bands แยกแล้ว; รายงาน AMBIGUOUS เมื่อแยกไม่ได้ |
| MP-025 | LED ผู้ใช้บนบอร์ด | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | restore เฉพาะ active trigger ที่อยู่ในวงเล็บกับ brightness เดิม ไม่เขียนรายการ trigger ทั้งบรรทัดกลับ |
| MP-026 | ปุ่มควบคุมจอและ LED | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | init ADC ครั้งเดียว; ใช้ calibration จริงและ LED node ที่ยืนยันแล้ว |
| MP-027 | IO2 digital output | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline |
| MP-028 | IO2 digital input | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline |
| MP-029 | IO3 digital output | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline |
| MP-030 | IO3 digital input | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline |
| MP-031 | IO4 digital output | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline |
| MP-032 | IO4 digital input | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline |
| MP-033 | IO5 digital output | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline |
| MP-034 | IO5 digital input | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline |
| MP-035 | IO6 digital output | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline |
| MP-036 | IO6 digital input | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline |
| MP-037 | IO7 digital output | HOLD | ต้องสร้างเฉพาะงาน | HOLD การขับ IO7: P24 เป็นค่าจาก dropdown ไม่ใช่ข้อพิสูจน์ว่า native runtime รับ alias นี้ |
| MP-038 | IO7 digital input | HOLD | ต้องสร้างเฉพาะงาน | HOLD การตั้ง IO7 เป็น input เช่นเดียวกัน ห้ามเปลี่ยนเป็น A24 แทน |
| MP-039 | IO8 digital output | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline |
| MP-040 | IO8 digital input | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline |
| MP-041 | สำรวจ input เจ็ดช่องพร้อมกัน | HOLD | ต้องสร้างเฉพาะงาน | HOLD การอ่านครบเจ็ดช่องจน IO7 ยืนยัน; floating input ไม่เป็น FAIL ของพอร์ต |
| MP-042 | walking-one output | HOLD | ต้องสร้างเฉพาะงาน | HOLD all-seven output; ห้ามขับ alias ที่ยังไม่ยืนยันเพื่อให้ครบตาราง |
| MP-043 | walking-zero output | HOLD | ต้องสร้างเฉพาะงาน | walking-zero ต้องประเมินกระแสรวม fixture ไม่เท่ากับการขับ GPIO ทุกช่องพร้อมกันโดยไร้เงื่อนไข |
| MP-044 | loopback IO2 → IO3 | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | IO2 output → IO3 input ผ่าน series resistor; อีกด้านไม่เป็น output |
| MP-045 | loopback IO4 → IO5 | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | IO4 output → IO5 input มี timeout ใน loop ที่ควบคุมเองและวัด input จริง |
| MP-046 | loopback IO6 → IO8 | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | latency เป็น userspace-observed เท่านั้น ไม่ใช่สเปก hardware GPIO |
| MP-047 | ปุ่มภายนอกสั่ง LED คนละช่อง | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | กรอก active-high/low และ pull ตามสวิตช์จริง อย่าเดาว่าปล่อยต้องเป็น HIGH |
| MP-048 | Stop ระหว่างขับ GPIO | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ใช้ GPIO ที่ยืนยันแล้วกับ LED เท่านั้น; forced kill อาจไม่รัน finally ต้องตรวจ LOW จริงหรือ UNKNOWN |
| MP-049 | สำรวจ PWM ที่ออกมาที่ connector | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | discovery เท่านั้น ไม่มีการสร้าง PWM; pin/function/channel เป็นคนละข้อมูล |
| MP-050 | วัด PWM บน IO7 | HOLD | ต้องสร้างเฉพาะงาน | HOLD จน mapping/backend แน่นอน; ต้องใช้เครื่องมือวัดก่อนต่อ servo ไม่เดา PWM2 จาก helper ที่ไม่พบ |
| MP-051 | LED ปรับ duty บน IO7 | HOLD | ต้องสร้างเฉพาะงาน | duty เป็นเปอร์เซ็นต์; HOLD จน IO7 PWM ยืนยัน ห้ามใช้ช่อง backlight แทน |
| MP-052 | servo เริ่มกลางช่วง | HOLD | ต้องสร้างเฉพาะงาน | servo.C906Servo ยังไม่ใช่ dependency ที่ยืนยันใน image; ห้าม import แล้วอ้างว่าพร้อม |
| MP-053 | servo ควบคุมด้วย touch | HOLD | ต้องสร้างเฉพาะงาน | ต้องผ่าน MP-015 และ mapping/pulse limits ของ servo ก่อนรับค่า slider |
| MP-054 | PWM กับ Stop และการรันใหม่ | HOLD | ต้องสร้างเฉพาะงาน | ต้องวัดสัญญาณหลัง Stop จริง; disable() มีใน API แต่ไม่รับประกัน cleanup เมื่อ kill |
| MP-055 | สำรวจ I2C connector จริง | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | list_devices/node เป็น discovery ไม่ scan หรือเปิดบัสภายใน และไม่ insmod อัตโนมัติ |
| MP-056 | scan I2C ภายนอกแบบควบคุม | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | scan เป็น probe ที่อาจมีผลต่ออุปกรณ์ ไม่ใช่ read-only discovery; จำกัด bus/address/operation ตามโมดูล |
| MP-057 | อ่านเซนเซอร์ SHT31 | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ต้องมี external bus และ SHT31 protocol ที่ตรงรุ่น; ตรวจ None/length/CRC ก่อนแปลงค่าจริง |
| MP-058 | อ่านเซนเซอร์แสง I2C | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ระบุรุ่น/measurement command/conversion ของโมดูล ไม่ใช้ค่า raw ของบัสเป็น lux |
| MP-059 | จอ OLED ภายนอก | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ระบุ controller SSD1306/SH1106 และ resolution จากโมดูลจริง ไม่เดาด้วย address |
| MP-060 | หลายอุปกรณ์บน I2C | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ตรวจ address conflict และแชร์ bus handle; ไม่สร้างหลายเจ้าของอุปกรณ์เดียวกัน |
| MP-061 | ADC ภายนอกกับ potentiometer | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ใช้ external ADC ตามชนิด/gain/reference จริง; ADC0 onboard เป็นปุ่ม ไม่ใช่พอร์ต POT ทั่วไป |
| MP-062 | ADC ภายนอกทุกช่อง | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | เลือกชนิด single-ended/differential และช่วงแรงดันจริง ก่อนทดสอบทุกช่อง ADC ภายนอก |
| MP-063 | I2C ไม่มีอุปกรณ์และ NACK | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | API readfrom รุ่นนี้ไม่มี timeout ที่ยืนยัน; แยก NACK/None จาก native call ค้าง และใช้ Stop supervisor ตามจริง |
| MP-064 | อ่าน ADC ผ่าน iKB-1Z | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | iKB-1Z ต้องมี protocol/driver และการจ่ายไฟตามคู่มือรุ่นจริง ไม่ถือว่ามี Python library ติดตั้งแล้ว |
| MP-065 | IMU raw หกแกนและตรวจข้อมูลเพิ่มเติม | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | read() รับประกันหกแกนใน dual mode ไม่ใช่เจ็ดค่า; ตรวจ len และอย่า index [6] เพื่ออุณหภูมิโดยไม่มีหลักฐาน |
| MP-066 | IMU หกทิศทาง | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | ใช้หน่วย/ทิศแกนที่ยืนยัน; raw ผ่านหกท่าไม่ใช่ calibration ที่ใช้กับทุกบอร์ด |
| MP-067 | gyro ขณะนิ่งและหมุน | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | bias เป็นชั่วคราวใน RAM; ระบุหน่วย gyro ที่ตรวจแล้ว ไม่เขียนถาวร |
| MP-068 | ลูกบอลตามความเอียง | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | ไม่คัด x10/sign inversion ของ generator เป็น universal mapping; จำกัดตำแหน่งบนจอจริง |
| MP-069 | เขย่าและ tap แบบระบุวิธี | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | software shake ไม่ใช่ hardware tap; ห้ามใช้ msa311 แทน QMI8658 |
| MP-070 | RTC อ่านเวลาและแหล่งข้อมูล | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | ใช้ kernel RTC ที่มีจริงหรือ bm8563 API หลังยืนยัน chip/bus; ext_dev.rtc ไม่มีใน stub นี้ |
| MP-071 | RTC เทียบเวลาจริง | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ต้องมีเวลาอ้างอิงจริงสองจุดและแยก wall clock jump/NTP ก่อนอ้าง drift |
| MP-072 | RTC สำรองไฟ | HOLD | ต้องสร้างเฉพาะงาน | HOLD battery pass จนแยก NTP/auto RTC sync ได้; ไม่สั่งปิดไฟหรือแก้ service อัตโนมัติ |
| MP-073 | สำรวจ UART และเจ้าของพอร์ต | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | UART0/A16/A17 มีเจ้าของจาก log เดิม; list_devices ไม่ได้แปลว่าพอร์ตว่าง |
| MP-074 | UART loopback | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ใช้ user UART ที่ยืนยันและ read(timeout หน่วย ms) เท่านั้น ไม่ใช้พอร์ตระบบเป็น loopback |
| MP-075 | UART ต่อคอมพิวเตอร์ | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | USB-UART console กับ user UART แยกหน้าที่; ต้องยืนยัน connector/path ก่อนเปิด |
| MP-076 | UART ข้อมูลไบนารีและ partial read | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ใช้ byte framing และ bounded partial buffer ไม่ decode arbitrary bytes เป็นข้อความ |
| MP-077 | UART timeout และสายไม่ต่อ | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | read timeout=0 คือ immediate, -1 คือ block; ห้ามใช้ -1 ใน Stop-responsiveness test |
| MP-078 | UART เปรียบเทียบ baud rate | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ตั้ง baud อีกฝั่งตรงทุกครั้ง เก็บ checksum/sequence และรายงานค่าที่ driver ปฏิเสธแยกกัน |
| MP-079 | ภาพสดจากกล้อง | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | ตัวอย่าง camera seed เดิมยังไม่มี FPS; ต้องเพิ่ม measured FPS และ close จริงก่อนเรียกว่า MP-079 ครบ |
| MP-080 | กล้องพร้อมข้อความและวงกลม | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | camera_overlay เป็น seed ของ overlay ต้องเพิ่มรายละเอียดตามพรอมท์ก่อนถือว่าครบ MP-080 |
| MP-081 | ทิศภาพและพิกัด | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | hmirror/vflip มี API แต่ต้องรู้ค่าเดิมก่อนเปลี่ยน/restore; ไม่ยืนยัน orientation จากรูปนิ่ง |
| MP-082 | ความละเอียดหลายค่า | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | ปิด device เดิมก่อนเปิด resolution ใหม่ อ่านขนาดจริงและไม่สร้างกล้องทุกเฟรม |
| MP-083 | ถ่ายภาพเมื่อกดปุ่ม | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | capture หนึ่งครั้งต่อ press edge ใช้ bands จาก MP-021 และบันทึกเฉพาะ directory ทดสอบ |
| MP-084 | ตรวจสีและติดตามวัตถุ | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | threshold ตามแสง/วัตถุจริง; centroid/area ไม่ใช่ AI confidence |
| MP-085 | อ่าน QR และบาร์โค้ด | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | payload API ยืนยัน แต่ห้ามเปิด URL หรือ execute QR payload อัตโนมัติ |
| MP-086 | กล้องทำงานต่อเนื่อง | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | FPS/memory/Stop เป็นสิ่งที่ต้องวัด ไม่รับรอง 10 นาทีจาก fixture สามเฟรม |
| MP-087 | เล่นเสียงทดสอบ | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | Player.volume ใน stub ระบุ unsupported; ใช้ PCM amplitude ต่ำแทนการพึ่ง setter และไม่ใช้ stop()/close() ที่ไม่มี |
| MP-088 | ไมโครโฟนแสดงระดับเสียง | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | Recorder.record() คืน buffered PCM หรือ empty; ไม่มี Recorder.read(); RMS/peak ไม่ใช่ calibrated dB SPL |
| MP-089 | อัดเสียงแล้วเล่นกลับ | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | ปิด recorder ด้วย finish(), ใช้ WAV metadata จริงและลด PCM playback amplitude; ไม่ผูก Stop กับ record_ms=5000 ที่ block |
| MP-090 | เสียง clipping และความเงียบ | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | signedness/endian/width ตรง Format.FMT_S16_LE; clipping ratio ต้องคำนวณจาก PCM จริง |
| MP-091 | audio กับ Stop | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | Player ไม่มี stop/close ใน baseline; ใช้ short PCM chunks/exit loop และปล่อย references; ไม่อ้างว่า finally รันหลัง kill |
| MP-092 | เสียงพร้อมกล้องและจอ | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | full-duplex/resource sharing ยังต้องวัด ไม่เปิด loopback ไมค์→ลำโพงเอง |
| MP-093 | สำรวจ WiFi/RNDIS | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | อ่าน interface/route แบบ bounded ไม่พิมพ์รหัสผ่าน token หรือ device_key |
| MP-094 | ทดสอบ WiFi ที่เชื่อมอยู่ | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ใช้ network เดิมกับ endpoint ที่อนุญาต ไม่เปลี่ยน SSID/route/RNDIS เพื่อให้เทสผ่าน |
| MP-095 | HTTP GET JSON | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ใช้ urllib.request ไม่ใช่ maix.http.get ซึ่งไม่มีใน API นี้; timeout/status/schema แยกกัน |
| MP-096 | HTTP POST ผลทดสอบ | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ส่งได้เฉพาะ endpoint/payload ที่ผู้ทดสอบระบุ ไม่สร้างปลายทางสาธารณะหรือส่งภาพ/เสียงเอง |
| MP-097 | MQTT publish/subscribe | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ต้องมี compatible MQTT package/broker/topic/credential จริง ไม่ assume maix.network.mqtt หรือ pip เอง |
| MP-098 | เครือข่ายหลุดแล้วกลับมา | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | จำกัด queue/backoff และทดสอบปลายทางหลุดโดยไม่ตัด USB ที่ใช้ IDE |
| MP-099 | WebSocket แอปแยกจาก IDE | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ต้องมี WebSocket package/API จริง แยก port 5050 ของ IDE และปิดตาม library ที่เลือก |
| MP-100 | HTTP server ใน LAN | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ใช้ HTTPServer.timeout + handle_request ใน exit-aware loop แล้ว server_close; หลีกเลี่ยง same-thread shutdown deadlock |
| MP-101 | พื้นที่จัดเก็บและ permission | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | read-only storage inventory ไม่มี format/fsck หรือเขียนทดสอบจนเต็ม |
| MP-102 | เขียนอ่านไฟล์เล็ก | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | unique test file ภายใต้พื้นที่ของโปรแกรม ขนาด 64KiB; ปิดแล้วอ่านกลับเทียบ hash |
| MP-103 | CSV บันทึกเซนเซอร์ | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | sensor ต้องผ่านแยกก่อน; log error/status ไม่เติมเลข 0 เป็นข้อมูล sensor ปกติ |
| MP-104 | บันทึก config แบบ atomic | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | แต่ละ Run ใช้ /tmp ใหม่ จึงต้องเลือก persistent directory ก่อนอ้าง config อยู่ข้าม Run |
| MP-105 | ไฟล์และ path ที่มีช่องว่าง/ไทย | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | ใช้ Python path/file API ไม่ต่อชื่อไทยหรือชื่อมีช่องว่างเข้าคำสั่ง shell |
| MP-106 | Run ชั่วคราวกับข้อมูลถาวร | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ยืนยันตำแหน่งถาวรและ orderly restart; ห้ามแนะนำถอดไฟระหว่างเขียน SD |
| MP-107 | ตรวจไฟล์โมเดลก่อน inference | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ตรวจ compatible model metadata จากไฟล์จริง ไม่ถือว่า NN API มีแล้วโมเดลใดก็โหลดได้ |
| MP-108 | image classification | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | Classifier.classify ยืนยัน API แต่ต้องมี model/labels/preprocessing/ground truth ที่ตรงกัน |
| MP-109 | object detection | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | YOLO API รุ่นนี้มี v5/v8/11; เลือกจาก model metadata ไม่เดาจากนามสกุลอย่างเดียว |
| MP-110 | เสียง classification | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | audio preprocessing เป็นของแต่ละโมเดล ต้องมี specification ก่อน inference; ไม่ใช้ Speech แทน generic classifier |
| MP-111 | AI สั่ง LED ด้วย threshold | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ต้องผ่าน inference และ GPIO แยกก่อน; first fixture เป็น LED ไม่เป็น relay/motor |
| MP-112 | AI หลายรอบและคืนหน่วยความจำ | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | โหลดโมเดลครั้งเดียวต่อรอบ ไม่สร้าง NN/Camera ทุกเฟรม; leak ต้องวัดจริงหลายรอบ |
| MP-113 | Run/Stop จอซ้ำ 20 รอบ | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | ผู้ทดสอบกด Run/Stop จาก IDE 20 รอบ ไม่ให้ main.py คุม Launcher หรือสร้างผลผ่านเอง |
| MP-114 | จบโปรแกรมตามปกติ | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | return/exit 0 ปกติและ finally cleanup; ไม่ใช้ os._exit ที่ข้าม cleanup |
| MP-115 | Python exception ที่ตั้งใจให้เกิด | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | NEGATIVE TEST: RuntimeError/nonzero เป็นผลที่ตั้งใจและต้องตรวจ traceback/menu ไม่ใช่ hardware PASS |
| MP-116 | SyntaxError ก่อนเปิดอุปกรณ์ | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | NEGATIVE TEST: SyntaxError ตั้งใจไม่ผ่าน compile ต้องใช้โปรเจกต์แยกและไม่แก้โปรแกรมใช้งานจริง |
| MP-117 | ImportError ของโมดูลที่ไม่มี | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | แยก missing-package SKIP กับ intentional ImportError/nonzero ไม่ติดตั้ง dependency เอง |
| MP-118 | Stop ระหว่างรอ I/O | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | ใช้ I/O ที่มี finite-timeout จริง; ห้ามสมมติว่า I2C ทุก native call หยุดได้จาก Python loop |
| MP-119 | ทรัพยากรหลายส่วนทำงานนาน | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | เลือกเฉพาะ subsystem ที่ผ่านก่อน จำกัด memory/log และไม่จงใจ OOM |
| MP-120 | API รุ่นใหม่หรือบางตัวหาย | HOLD | ต้องสร้างเฉพาะงาน | DESKTOP SYNTHETIC ONLY ไม่ส่งขึ้นบอร์ดเพื่ออ้างว่าได้ทดสอบเวอร์ชันใหม่ที่ติดตั้ง |
| MP-121 | SPI เฉพาะเมื่อมี connector/pinmux ที่ยืนยัน | HOLD | ต้องสร้างเฉพาะงาน | HOLD SPI จนมี connector/pinmux/driver/ownership ของ Plus จริง ไม่ยืม pinout MaixCAM2 |
| MP-122 | Bluetooth แบบตรวจความพร้อม | HOLD | ต้องสร้างเฉพาะงาน | HOLD Bluetooth จนพบ adapter และ driver จริง WiFi ไม่พิสูจน์ Bluetooth |
| MP-123 | USB-OTG และ USB-UART | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | เป็นขั้นตอน manual/discovery แยก OTG/RNDIS กับ console ไม่แก้ boot config หรือ USB role |
| MP-124 | อุปกรณ์ USB host แบบมีเงื่อนไข | HOLD | ต้องสร้างเฉพาะงาน | HOLD USB host จนมี control transport สำรองและ power arrangement ที่ยืนยัน |
| MP-125 | servo/motor ผ่านบอร์ดขยาย | HOLD | ต้องสร้างเฉพาะงาน | HOLD motor/expansion จน protocol/power/driver/enable fixture ระบุครบ ห้ามขับโหลดตรงจาก GPIO |
| MP-126 | dashboard รวม onboard | NEEDS_SETUP | ต้องสร้างเฉพาะงาน | dashboard รวมต้องผ่าน constituent tests ก่อน ไม่ใช้ภาพ mock เป็น sensor จริง |
| MP-127 | ชุดไล่ตรวจ IO ทั้งบอร์ด | HOLD | ต้องสร้างเฉพาะงาน | HOLD wizard ขับครบทุก IO จนกรณีแยกผ่าน รวม IO7; ทำ subset แล้วห้ามสรุปครบทั้งบอร์ด |
| MP-128 | สรุปผลและสร้าง regression suite | DRAFT_REFERENCE | ต้องสร้างเฉพาะงาน | วิเคราะห์ผลที่ผู้ใช้ส่งหรืออนุญาตให้อ่านเท่านั้น ไม่สร้าง PASS ให้แถวที่ยัง NOT_RUN และไม่ต้องสร้างโครง display project |

## ตรวจซ้ำ

ผลตรวจซอฟต์แวร์และขอบเขตที่ยังไม่ได้ทดสอบอยู่ใน [validation record](research/uai-plus-runtime-2026-10-26/prompt-audit-validation-2026-10-27.json) ใช้ [โปรเจกต์ preflight](../src-tauri/target/recovery-projects/mai_plus_preflight) เริ่มสำรวจแบบไม่ขับพอร์ตก่อนเลือกชุดต่อไป

```powershell
node scripts/audit-uai-plus-prompts.mjs --check
python -m unittest discover -s scripts/tests -p "test_uai_plus*.py"
cargo test --manifest-path src-tauri/Cargo.toml --lib --locked --offline
```

สถานะ TEMPLATE_CHECKED อิงชุดทดสอบ fixture ของเทมเพลต การสร้างโค้ดใหม่/แก้รายละเอียดนอกเทมเพลตต้องตรวจแยก ไม่รับรองการสร้างจากทุกโมเดลหรือ firmware ในอนาคต
