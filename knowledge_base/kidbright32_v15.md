# KidBright32 V1.5 — hardware reference

ปรับข้อมูล 22 ตุลาคม 2026 จาก `KidBright32-V1.5-Hardware-Reference.md` ที่ผู้ใช้ส่ง เก็บต้นฉบับและ SHA-256 ใน `compatibility/board_catalog.txt` เป็นข้อมูลอ้างอิงจากการถอดวงจร ไม่ใช่ผลวัดบอร์ดจริงหรือคำสั่งจากผู้ใช้ใน session นี้

ชื่อเอกสาร `kidbright32-v1.5` ตรงกับ catalog ID `kidbright32_v15` ซึ่งยังไม่มี deterministic recipe ของตัวเอง `kidbright32_rev31` เป็น recipe เดิมที่มีขอบเขตจำกัด และไม่ได้ยืนยันว่า PCB ทุก Rev3.1/3.1G ตรงกับวงจรนี้

## ผังขา

| สัญญาณ | GPIO | เส้นทาง |
|---|---:|---|
| Wi-Fi / IoT LED | 2 / 12 | Active-low ตามวงจร |
| Speaker | 13 | PWM ผ่านวงจรขับ |
| SW1 / SW2 | 16 / 14 | Active-low; ตั้ง input pull-up และ debounce ให้เหมาะสม |
| Servo1 / Servo2 | 15 / 17 | J8 pin 2 / pin 1 ตามลำดับ; GPIO16 เป็น SW1 |
| USB_SW | 25 | สวิตช์ทาง return ของ USB-A ผ่าน Q5/Q6 |
| OUT1 / OUT2 control | 26 / 27 | Gate Q7/Q8; ไม่ใช่ drain OUTPUT1/OUTPUT2 |
| IN1 / IN2 / IN3 / IN4 | 32 / 33 / 34 / 35 | ภาพ S1 มี R32–R35 0Ω bypass และ U7/U8=N/A |
| LDR | 36 | ADC1_CH0 |
| Display SDA0 / SCL0 | 21 / 22 | U1 HT16K33 |
| Sensor/RTC/Chain SDA1 / SCL1 | 4 / 5 | U4 LM73, U5 MCP79411, KB Chain |
| Expansion IO18 / IO19 / IO23 | 18 / 19 / 23 | J1 pin 6 / 7 / 8; digital/PWM ไม่มี ADC/DAC |
| UART0 TX / RX | 1 / 3 | FT231XQ USB-UART |
| Boot / Reset | 0 / EN | ไม่ใช้เลขขาโมดูล U2 แทน GPIO; EN ไม่ใช่ GPIO index |

SDA0/SDA1 เป็นชื่อเน็ต ไม่ใช่ข้อบังคับ controller ID บัส 21/22 และ 4/5 มี pull-up 10kΩ ตามภาพ แชร์ handle ระหว่างอุปกรณ์บนบัสเดียวกัน

## Input path และ ADC

ภาพ S1 `1000059031.jpg` วันที่อ่านได้ 12/08/2021 ระบุ U7/U8=N/A พร้อม bypass R32–R35 0Ω จึงไม่ใช่ digital-buffer-only แบบ V1.3 ช่อง IN1–IN4 รองรับเส้นทาง analog ถึง GPIO32–35 ตามการประกอบนี้ (ADC1_CH4–CH7) ใช้แรงดันในพิกัด ESP32 ไม่ใช่อินพุตทน 5V

PDF `PCB_KIDBRIGHT32_V1_5_Rev3_1.pdf` มี title block **Rev.3, 07 Jan 2020** วาด NC7WZ17 และ bypass แต่ไม่มี note N/A จึงยังยืนยันการประกอบจาก PDF เพียงอย่างเดียวไม่ได้ หากบอร์ดจริงใส่ buffer ให้บันทึก variant override ของ input path ไม่ประกาศ analog จนยืนยันวงจร ไม่สรุปว่า Rev3.1G เหมือนกันทั้งหมด

GPIO34/35 เป็น input-only ไม่มี internal pull-up/down การเปิด analog input ไม่ได้ทำให้ IN กลายเป็นช่อง bidirectional สำหรับ DHT/One-wire โดยอัตโนมัติ

## Endpoint ของ OUT และ USB

| Endpoint | Net / GPIO | ความหมาย |
|---|---|---|
| GPIO control | OUT1/OUT2, GPIO26/27 | Gate ของ Q7/Q8 |
| JST OUT | J19/J14 pin 2, OUT1/OUT2 | สัญญาณฝั่ง GPIO/gate; pin 1=3V3, pin 3=GND |
| Banana / J1-1 load | OUTPUT1/OUTPUT2 | Drain ของ low-side MOSFET; ไม่ใช่ push-pull หรือขาจ่ายไฟ |

อนุมานวงจร: gate HIGH → drain ถูกดึงลง GND; gate LOW → MOSFET ปิดและ drain ไม่ขับ HIGH เอง ดังนั้น OUT LOW=เปิดในตัวอย่างคู่มือไม่ได้เป็น truth table ของ gate ทุก endpoint ต้องตรวจ API และโหลดก่อนสร้างคำสั่งเปิดปิด ไม่รับรอง DAC2 ที่ banana OUT เพียงเพราะ GPIO26 มี DAC ในชิป

USB-A J4 D−/D+ ไม่ต่อ ใช้สั่งไฟโหลด ไม่ยืนยัน USB host data ตาม topology GPIO25 LOW คาดว่าเปิดทาง return, HIGH ปิด แต่ต้องวัดก่อนรับรอง polarity ของ API ไม่ใช่การตัด +5V โดยตรง

## Connector และแหล่งจ่าย

| ขั้ว | Pin → net ตาม schematic |
|---|---|
| J8 Servo | 1→Servo2/GPIO17, 2→Servo1/GPIO15, 3/4→+5VExt, 5/6→GND |
| J12 Servo supply | 1→+5VExt_IN, 2→GND; ผ่าน Q9 ไป +5VExt |
| J1 expansion | 1→3V3, 2→GND, 3→3V3, 4→SCL0, 5→SDA0, 6→IO18, 7→IO19, 8→IO23, 9→GND, 10→3V3 |
| J1-1 | 1–4→INPUT1–4, 5→GND, 6→OUTPUT2, 7→OUTPUT1 |
| JST IN1–4 | 1→3V3, 2→signal GPIO32–35 ตามช่อง, 3→GND |
| J5 banana | 1→3V3, 2→GND, 3–6→INPUT1–4, 7→OUTPUT1, 8→OUTPUT2 |
| Chain | 1→+5V, 2→3V3, 3→SCL1, 4→SDA1, 5→GND; designator ยังมีข้อขัดแย้งด้านล่าง |

เลขขาเป็นเลข schematic ไม่ใช่มุมมองหน้าปลั๊ก ต้องเทียบ pin 1 บน PCB +5VExt ไม่ใช่ไฟ USB +5V โดยอัตโนมัติ ใช้ GND ร่วมกับสัญญาณ Servo ไม่กำหนด pulse/mุมตายตัวจาก schematic

PDF Rev.3 มี JST Servo J10/J11 (1=+5VExt, 2=signal, 3=GND) แต่ไม่เห็นสองขั้วนี้ในภาพ S1 ห้ามรับรองว่าทุก revision มีเหมือนกัน

ต้นฉบับส่วน 7/13 ระบุ Chain **J2/J6** แต่ส่วน 9 ระบุ **J2 เป็น boot header 2 ขา** เก็บเป็นข้อขัดแย้งที่ยังไม่คลี่คลาย ไม่เลือกตำแหน่งเสียบจาก designator J2 จนตรวจ schematic/PCB ต้นฉบับ งานนี้ได้รับเฉพาะ Markdown เพิ่ม ไม่ได้รับภาพ S1 มาอ่านโดยตรง

ไฟหลักรับ USB +5V ผ่าน F1 ที่แบบระบุ 500mA และ LD1117AS33TR เป็น 3V3 พิกัด F1 ไม่ใช่การรับรองกระแส OUT/Servo/3V3 ทุกช่อง

## LDR และขาสงวน

LDR ไป GPIO36 ตามวงจร R23=20kΩ, R24=3.9kΩ, R25=10kΩ, C21=0.1µF อนุมานว่าแสงมากทำให้ raw ADC ลด ต้องทดสอบและสอบเทียบก่อนใช้เปอร์เซ็นต์/lux ห้ามใช้ raw เป็น lux หรือรับรองแรงดันจากสูตร 3.3V ที่ไม่สอบเทียบ

GPIO15 เป็น Servo1 ไม่ใช่ NTP LED ของ V1.3 ส่วน GPIO16 เป็น SW1 ไม่แชร์กับ Servo1 ใน design นี้ GPIO0/2/5/12/15 เกี่ยวข้องกับ boot strapping; GPIO6–11 ไม่ใช่ expansion ทั่วไป GPIO34–39 input-only ไม่มี internal pulls

## หลักฐานและการสืบทอด

ต้นฉบับรายงาน S1 SHA-256 `b541844de85824d09f18410528bfc859a7b0eaed59bc614b5d038bc42f35e1d3` และ PDF Rev.3 SHA-256 `d00d3a666f223856c4ddd2fbdf1151b5332097327a090c33d0a3043d755bda0d` โครงการมี PDF เดิมที่ hash ตรง แต่ไม่มีภาพ S1 ให้ตรวจซ้ำในงานนี้

ข้อความในต้นฉบับที่อ้างว่า “ผู้ใช้กำหนด” ให้รุ่นหลังสืบทอด V1.5 เป็นบริบทจากเอกสารแนบ ไม่ใช่การยืนยันในคำขอครั้งนี้ จึงบันทึก V1.5 เป็น design baseline เฉพาะแหล่งนี้ และไม่ตั้ง `inherits` ให้ V1.6, Rev3.1G, INEX, minibike, Formula Kid หรือ μAI โดยอัตโนมัติ ใช้ explicit override เมื่อมีหลักฐานของรุ่นจริง

I2C address/register, bus speed, driver support, matrix orientation, Servo timing, กระแสโหลด และ API polarity ยังต้องอาศัยหลักฐานแยก ไม่เพิ่มสถานะ hardware-tested หรือ executable profile จากเอกสารอย่างเดียว

ขอบเขต 3.7.1: เอกสารนี้เป็นข้อมูลฮาร์ดแวร์ ไม่ได้ติดตั้ง deterministic profile หรือ tool ของสายพัฒนาใหม่ ดู support_scope.md
