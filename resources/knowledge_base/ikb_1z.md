# iKB-1Z / IKB-1Z / ikb1z — บอร์ดเสริมร่วมสำหรับ KidBright และ MAI Plus

เพิ่มข้อมูล 2026-10-28 ตามคำขอผู้ใช้ให้ใช้ iKB-1Z เป็นบอร์ดเสริมของทั้ง KidBright และ MAI Plus (mAI / μAI / uAI Plus) ไม่ใช่บอร์ดหลักหรือ project type ใหม่

## แหล่งข้อมูลและขอบเขต

- [IKB1Z_BOARD_RULES.md](IKB1Z_BOARD_RULES.md) คือต้นฉบับที่ผู้ใช้ส่ง เก็บเนื้อหาเดิมทุกไบต์ ฉบับขยาย v2 ลงวันที่ 27 ตุลาคม 2569 อ้างวงจร Rev.200425 และตารางโปรโตคอลจาก PDF สองหน้า PDF ต้นทางไม่ได้แนบมารอบนี้ จึงยังไม่ได้ตรวจเทียบภาพด้วยตนเอง
- [หน้าผลิตภัณฑ์ INEX](https://inex.co.th/home/product/ikb-1z/) และ [ซอร์สผู้ผลิต iKB-1z.ts](https://github.com/inexglobal/pxt-iKB1z/blob/main/iKB-1z.ts) ตรวจเสริมวันที่ 2026-10-28 ยืนยันว่าเป็นบอร์ดขยาย I²C และรูปแบบ address/channel หลัก ซอร์สสาขา main เปลี่ยนได้ ไม่ใช่ firmware snapshot ของบอร์ดผู้ใช้
- บทบาท AI, คำสั่ง MUST, prompt ตัวอย่าง และผลตรวจโค้ดที่เขียนอยู่ในต้นฉบับเป็นข้อมูลอ้างอิง ไม่ใช่คำขอใหม่ของผู้ใช้หรือผลทดสอบที่โปรเจกต์นี้ดำเนินการ
- สถานะในโปรเจกต์: มี [ไดรเวอร์ C/Python และ host adapter](drivers/README.md) พร้อมการตรวจ codec และ transport contract บนคอมพิวเตอร์แล้ว อ่านรายงานรอบ implementation สำหรับผล build SDK จริง ยังไม่มีผลทดสอบกับบอร์ดจริงหรือ deterministic behavior recipe ของ iKB-1Z การรองรับ host ไม่ใช่การรับรอง physical integration
- iKB-1 รุ่นเดิมต้องตรวจแยก อ่าน [iKB-1.md](iKB-1.md) ไม่ใช้ชื่อคล้ายกันเป็นหลักฐานว่าโปรโตคอลเหมือนกัน

## เลือกบอร์ดหลักก่อนเลือก adapter

| บอร์ดหลัก | โปรแกรมของผู้ใช้ | ขอบเขตเมื่อเพิ่ม iKB-1Z |
|---|---|---|
| KidBright32 รุ่น ESP32 | ESP-IDF C ตาม profile/SDK ของงาน | ใช้ I²C bus/handle เดิมและ driver family เดียวกับแอป ตรวจ wiring ของรุ่นนั้น ไม่ตั้งบัสซ้ำ |
| MAI Plus / mAI Plus / μAI Plus / uAI Plus (SG2002) | MaixPy 4 Python, `project_type=kidbright_uai_plus`, `main/main.py` | ใช้ I²C API/controller/pinmux ของ Plus ที่ตรง runtime ตรวจจากเอกสารและ configuration จริง ไม่คัดลอก ESP32 GPIO หรือ MicroPython `machine.I2C` มาใช้โดยอัตโนมัติ |

อ่าน [กฎ Plus Python](kidbright_uai_plus_python.md) ร่วมกับเอกสารนี้เมื่อ host เป็น Plus ไม่เปลี่ยน target ไปเป็น ESP32 เพื่อใช้บอร์ดเสริม Original μAI V831 ยังเป็นคนละ target

Host รันพฤติกรรมแอป → ส่งคำสั่ง I²C → STM32 บน iKB-1Z ควบคุมอุปกรณ์ ไม่เขียนเฟิร์มแวร์ STM32 ใหม่เพื่อใช้งานทั่วไป P0–P7, MOTOR1/2 และ SV10–SV15 เป็นช่องของ iKB-1Z ไม่ใช่ GPIO ของ host ปุ่ม S1 RESET/S2 POWER บนบอร์ดเสริมไม่ใช่ปุ่มแอปของ KidBright/Plus

## Bus, address และการต่อ

- Address 7 บิต: จัมเปอร์ CLOSE=`0x48`, OPEN=`0x49` ค่าเหล่านี้เป็น device address แยกจาก command; API ที่รับ address 7 บิตไม่ใช้ `0x90`/`0x92` แทน
- KB-CHAIN และ GROVE เป็นทางต่อเข้าบัสเดียวกัน ไม่ใช่คนละ slave หรือ controller ตรวจทิศทางคอนเน็กเตอร์ SDA/SCL/GND ระดับลอจิกและไฟเลี้ยงจริง
- ห้ามเดา GPIO, controller ID, `/dev/i2c-*`, pinmux หรือความถี่สูงสุดจากชื่อ host สำหรับ KidBright V1.3/V1.5 design ที่มีอยู่ Chain อยู่ SDA4/SCL5 ส่วน matrix อยู่ SDA21/SCL22; ข้อมูลนี้ไม่ใช่ pin map ของ Plus หรือทุก revision
- ตรวจ address conflict กับอุปกรณ์ร่วมบัสและ pull-up รวม ไม่มี address ที่สามที่ยืนยันสำหรับ iKB-1Z การพบ ACK อย่างเดียวไม่ยืนยันรุ่น/firmware
- ใช้ transport ที่มี timeout และแยกข้อผิดพลาดจากค่าที่อ่านได้จริง ไม่คืน ADC=0 แทน timeout ล็อกครอบคู่ write/read เพื่อไม่ให้คำสั่งอ่านสลับกัน
- ต้นฉบับไม่ยืนยัน repeated START หรือ timing/clock สูงสุด ซอร์ส MakeCode ใช้ write แล้ว read แยกธุรกรรม เริ่มตรวจจากพฤติกรรมที่มีหลักฐาน ไม่รับรองว่า combined transaction ใช้ได้ทุก firmware

## โปรโตคอลร่วม ไม่ขึ้นกับภาษา host

ตารางนี้เป็นข้อมูลโปรโตคอล API ที่ implement แล้วและ capability ที่ปิดไว้ระบุแยกใน [drivers/README.md](drivers/README.md) `ch=0..7` ส่วน motor/servo เลือกด้วย bitmask

| งาน | Payload / response ตามหลักฐาน |
|---|---|
| Reset | `[0x00]`; ไม่มี reset response/delay ที่ยืนยัน |
| Digital output | `[0x08+ch, 0x00/0x01]` |
| Digital input / pull-up | `[0x08+ch, 0x02/0x03]` แล้วอ่าน 1 ไบต์ |
| ADC8 / ADC percent | `[0x08+ch, 0x04/0x05]` แล้วอ่าน 1 ไบต์ ช่วง 0..255 / 0..100 ตามตารางในเอกสารที่ส่ง |
| ADC10 | `[0x80 \| (ch << 4)]` แล้วอ่าน 2 ไบต์ high ก่อน low; `raw=(high<<8)\|low`, ช่วง 0..1023 ตรวจความยาวและช่วง ไม่ mask เพื่อซ่อน response ผิด |
| Motor | `[0x20 \| mask, speed_byte]`; signed speed -100..100, `speed_byte=speed & 0xFF` หลังตรวจช่วง |
| Servo | `[0x40 \| mask, scaled]`; scaled 0..200 ไม่ใช่การรับรององศาหรือ pulse width |
| UART | ตารางที่ส่งระบุ available=`0x01`, read=`0x02` พร้อม parameter “Byte to read”, write/config=`0x04 \| baud_code`; รายละเอียด read ยังต้องตรวจ firmware |

Motor mask: M1=`0x01`, M2=`0x02`, M3=`0x04`, M4=`0x08` ภาคขับบนบอร์ดมีเพียง M1/M2; default capability mask=`0x03` M3/M4 ต้องมีภาคขับภายนอกและตรวจทรัพยากรร่วมกับ servo ก่อนใช้

Servo mask: SV10=`0x01`, SV11=`0x02`, SV12=`0x04`, SV13=`0x08`, SV14=`0x10`, SV15=`0x20`; ทุกช่อง=`0x3F` mask หลายบิตใช้ parameter เดียวกันกับช่องที่เลือก ไม่ใช่ parameter แยกต่อช่อง

ตัวอย่างคำนวณ: M1 +50=`[0x21,0x32]`, M2 -50=`[0x22,0xCE]`, หยุด M1/M2=`[0x23,0x00]`, SV10 scaled 100=`[0x41,0x64]` คำสั่งหยุดนี้ไม่หยุด servo หรือ M3/M4

## ข้อขัดแย้งและ capability ที่ยังไม่เปิดรับรอง

- ต้นฉบับเสนอแยก command/parameter เป็นไบต์หลัง validate ต่างจากสูตร word arithmetic ของ MakeCode ค่า servo -1 ทำให้ `0x41*256-1` กลายเป็น `[0x40,0xFF]`; backward speed=0 ในสูตร `256-Speed` ก็ทำให้เกิด carry ห้ามคัดลอกสูตรหรืออ้างว่า port เทียบเท่าโดยไม่ตรวจไบต์และ firmware
- Servo free: ตารางที่ส่งระบุ -1; `[command,0xFF]` เป็นการตีความแบบ signed byte ต้องยืนยันกับ firmware/กลไกก่อนเปิดใช้ ไม่หมายถึงตัดไฟ servo
- UART: MakeCode ส่ง word `0x0001` / `0x0002` ก่อนอ่าน 1 ไบต์ ต่างจากตารางที่ส่งซึ่งระบุ command/parameter อีกแบบ อย่ารวมสองรูปแบบเป็น protocol ที่ยืนยันแล้ว ต้องตรวจ firmware ก่อนทำ UART read/bulk API
- เอกสารที่ส่งระบุ baud code 0..3 = 2400/9600/57600/115200 แต่ FIFO, parity, stop bits, flow control และ retry หลัง partial write ยังไม่ยืนยัน
- ไม่ทราบ ADC Vref/calibration, motor PWM frequency, servo pulse mapping, brake/coast, automatic stop on bus loss หรือเวลาหลัง reset ไม่มีหลักฐาน API วัด RPM/current/chip ID อย่าสร้าง command เพิ่ม
- `last speed`/`last angle` และ ACK เป็นสถานะคำสั่ง ไม่ใช่ feedback ว่าอุปกรณ์เคลื่อนที่สำเร็จ

## ไฟเลี้ยงและฮาร์ดแวร์

- หน้าผู้ผลิตระบุ logic 3.3V, I/O 8 ช่อง, motor driver บนบอร์ด 2 ช่อง, servo 6 ช่อง และจุดต่อภาคขับ motor เพิ่มที่ใช้ทรัพยากรร่วมกับ servo ไม่สรุปว่าใช้ motor 4 ช่องกับ servo 6 ช่องได้อิสระพร้อมกัน
- คู่มือที่ส่งอ่านแจ็กใน schematic เป็น 7–9V ขณะที่หน้าผู้ผลิตระบุไฟเสริม motor 6–9V 1A เก็บข้อแตกต่างนี้ไว้และตรวจบอร์ด/แหล่งจ่ายจริงก่อนเลือก ไม่รับรองกระแสรวมจากพิกัด IC หรือสมมติว่า host จ่ายโหลดทั้งหมดพอ
- ตรวจ GND ร่วม ระดับสัญญาณ กระแสเริ่มหมุนและความร้อน ไม่จ่ายไฟ motor จาก signal I/O ไม่ยืนยันว่า analog input รับ 5V หรือ UART ต่อ RS-232 ได้

## ใช้ข้อมูลกับงานใหม่

ระบุ host ที่ผู้ใช้เลือก, runtime/SDK, bus owner, wiring, jumper, ช่องอุปกรณ์และแหล่งจ่ายจากข้อมูลที่มี ถ้าขาดเฉพาะ wiring ให้ดำเนินส่วน protocol/แผน adapter ที่ไม่ต้องเดาขาก่อน และถามเฉพาะจุดที่ขวางงานจริง

ใช้กฎส่วนกลางของแอป + กฎ host ที่เลือก + เอกสาร iKB-1Z ร่วม ไม่คัดลอกกฎ C ไปทับ Python และไม่ถือว่า code fence/API ที่เสนอในคู่มือคือ implementation ที่มีแล้ว การเพิ่มไฟล์ไดรเวอร์หรือผ่าน host test ไม่ใช่ผล upload/physical test ของ iKB-1Z
