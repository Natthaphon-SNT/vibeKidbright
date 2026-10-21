# iKB-1 / iKB-1Z — ขอบเขตอ้างอิงและโปรโตคอล INEX

เอกสารนี้อ้างอิงซอร์สผู้ผลิต `inexglobal/pxt-iKB1z` สาขา `main` ที่อ่านวันที่ 2026-10-16 ไม่ใช่การรับรองฮาร์ดแวร์ทุก revision หรือ implementation ESP-IDF C ที่ผ่าน build แล้ว

- **iKB-1Z:** ยืนยัน revision และสถานะ jumper ก่อนใช้ค่าด้านล่าง
- **iKB-1 รุ่นเดิม:** แม้ README ผู้ผลิตระบุทั้งสองชื่อ ต้องตรวจคู่มือ/firmware ของบอร์ดจริงก่อนนำ address และคำสั่งไปใช้ ห้ามสมมติว่าเหมือนทุก revision
- ไม่มี deterministic recipe, C adapter หรือผลทดสอบกับบอร์ดจริงในโปรเจกต์นี้สำหรับ iKB
- ตาราง MCP23017 และ address `0x20` ในเอกสารรุ่นก่อนถูกถอนแล้ว ไม่ใช่ protocol ของแหล่งอ้างอิงที่ตรวจพบนี้

## แหล่งอ้างอิงผู้ผลิต

- [INEX iKB-1Z product](https://inex.co.th/home/product/ikb-1z/)
- [INEX protocol source: iKB-1z.ts](https://github.com/inexglobal/pxt-iKB1z/blob/main/iKB-1z.ts)
- [Raw source](https://raw.githubusercontent.com/inexglobal/pxt-iKB1z/main/iKB-1z.ts)
- [INEX package README](https://github.com/inexglobal/pxt-iKB1z)
- [INEX MicroPython library](https://github.com/inexglobal/uPython_iKB) เป็นอีก implementation ซึ่งต้องตรวจแยกเมื่อใช้

## Address และ bus

ซอร์ส `iKB-1z.ts` บรรทัด 3–7, 114–124 ระบุ 7-bit address **0x48 เมื่อ jumper ปิด (default)** และ **0x49 เมื่อ jumper เปิด** ไม่ใช่ช่วง `0x20`, `0x21`, `0x22` แบบ MCP23017

ต้องยืนยัน SDA/SCL และ connector จากรุ่นของ host จริงก่อนกำหนด `I2C_NUM_*` ห้ามถือว่า KB-CHAIN ของ KidBright ทุกรุ่นต่อ GPIO21/22 หรือใช้เลข controller เดียวกัน ควร probe address และใช้ bus ที่ initialization ไว้แล้ว เลือก I2C driver family เดียวทั้งแอป

## คำสั่งที่พบใน snapshot นี้

ตารางนี้อธิบายซอร์สอ้างอิง ไม่ใช่ firmware driver พร้อมใช้งาน `p` หมายถึง digital port 0–7 ส่วน word ส่งเป็น unsigned 16-bit big-endian ตาม API ในซอร์ส

| งาน | ค่าที่ source ส่ง/อ่าน | Source lines |
|---|---|---|
| Digital output | `[0x08+p, state]`, state 0 หรือ 1 | 132–136 |
| Digital input | ส่ง `[0x08+p, 2]` แล้วอ่าน unsigned byte | 144–149 |
| Input with pull-up | ส่ง `[0x08+p, 3]` แล้วอ่าน unsigned byte | 150–155 |
| ADC0–ADC7 | ส่งหนึ่ง byte `0x80,0x90,...,0xF0` ตามช่อง แล้วอ่าน unsigned 16-bit big-endian; README ระบุช่วง 0–1023 | 9–26, 158–167 |
| Servo CH10–15 | command `0x41,0x42,0x44,0x48,0x50,0x60`; ALL=`0x7F`; word=`command*256+Angle` | 51–65, 138–143 |
| Motor CH1–4 | command `0x21,0x22,0x24,0x28`; CH1&2=`0x23`; ALL=`0x2F` | 67–84 |
| Motor forward | word=`command*256+Speed`, Speed 0–100 | 187–199 |
| Motor backward | word=`command*256+(256-Speed)` | 187–199 |
| Stop both motors | word `0x2300` | 234–238 |
| Reset | หนึ่ง byte `0x00` | 182–186 |
| Serial available / read | ส่ง word `0x0001` / `0x0002` แล้วอ่าน unsigned byte | 239–255 |

ไม่ใช้ packet มอเตอร์สาม byte `[0x70,speed1,speed2]` หรือ Servo `[0x50,channel,angle]` จากเอกสารรุ่นก่อนกับ snapshot นี้

## จุดที่ต้องตรวจในการพอร์ตเป็น C

- Servo source ประกาศ Angle=-1..200; ค่า -1 ต้องตรวจพฤติกรรม word arithmetic และ firmware จริงก่อนกำหนดความหมายหรือแปลงเป็นสอง byte ด้วย cast
- Motor backward Speed=0 ทำให้สูตรเกิด carry; ต้องรักษาหรือแก้พฤติกรรมอย่างเปิดเผย พร้อมทดสอบ ไม่เทียบเป็น `[command,(uint8_t)-Speed]` โดยอัตโนมัติ
- ตรวจ transaction boundary, read length, error return, timeout และการ lock bus ตาม host SDK
- ระบุรุ่นบอร์ด/firmware, source revision, waveform/packet และผลทดสอบจริงก่อนเพิ่มสถานะ supported
- ช่วงไฟเลี้ยง กระแส motor/servo และระดับสัญญาณต้องตรวจคู่มือของ revision จริงก่อนต่ออุปกรณ์ ข้อมูล protocol อย่างเดียวไม่ยืนยันข้อจำกัดไฟฟ้า
