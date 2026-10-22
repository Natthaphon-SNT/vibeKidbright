# KidBright32 V1.5 — ต้นแบบผังขาและการเชื่อมต่อ

เอกสารอ้างอิงฮาร์ดแวร์สำหรับ Knowledge Base, board profile และการสร้าง firmware C/ESP-IDF ของ vibeKidbright IDE

## 1. ขอบเขตและหลักฐาน

| รายการ | ค่า |
|---|---|
| Board profile | `kidbright32-v1.5` |
| Hardware baseline | KidBright32 V1.5 ตามภาพ schematic ที่ผู้ใช้ส่ง |
| ชื่อในกรอบแบบ | `KidBright32 V1.5` |
| วันที่บนแบบ | อ่านได้เป็น `12/08/2021` |
| Sheet / REV | `1/1` / อ่านได้เป็น `GNN1` |
| แหล่ง S1 | ภาพ `1000059031.jpg` |
| SHA-256 ภาพ S1 | `b541844de85824d09f18410528bfc859a7b0eaed59bc614b5d038bc42f35e1d3` |
| วันที่ถอดข้อมูล | 22 ตุลาคม 2026 |
| สถานะ | ถอดจากภาพและตรวจทานเส้นวงจร; ยังไม่ได้วัดบอร์ดจริงหรือทดสอบ firmware |

**นโยบายรุ่นสืบทอดตามที่ผู้ใช้กำหนด:** ใช้เอกสารนี้เป็นต้นแบบกลางของ V1.5 และรุ่นหลังที่ผู้ใช้ระบุว่าใช้ผังขาเดียวกัน ให้รุ่นเหล่านั้นอ้าง `inherits: kidbright32-v1.5` และบันทึกเฉพาะข้อแตกต่าง ไม่คัดลอกกฎของ V1.3 มาทับต้นแบบนี้ การสืบทอดเป็นข้อกำหนดของโครงการจากผู้ใช้ ไม่ใช่หลักฐานว่าตรวจ schematic ของทุกรุ่นหลังแล้ว ไม่ใช้กฎนี้กับบอร์ดคนละตระกูล เช่น minibike, Formula Kid Controller หรือ µAI PLUS โดยอัตโนมัติ

สถานะหลักฐานในไฟล์:

- **S1:** อ่านชื่อขา/เส้นเชื่อมต่อจากภาพต้นฉบับ
- **D1:** คุณสมบัติ GPIO จากเอกสาร Espressif; ไม่ใช่ชื่อเน็ตบนบอร์ด
- **อนุมานวงจร:** พฤติกรรมที่อธิบายจาก topology ยังไม่ได้วัดจริง
- **ต้องยืนยัน:** ภาพไม่ระบุ หรือยังต้องใช้ BOM, datasheet หรือบอร์ดจริง

หมายเลข GPIO, หมายเลขขาโมดูล U2 และหมายเลขขาคอนเนกเตอร์เป็นคนละระบบ ห้ามใช้แทนกัน

## 2. ตาราง GPIO หลัก

| หน้าที่ | Net บนแบบ | ESP32 GPIO | ขา U2 | เส้นทาง / หมายเหตุ |
|---|---|---:|---:|---|
| ไฟสถานะ Wi-Fi | WIFI | 2 | 24 | LD3 สีแดง ต่อจาก 3V3 ผ่าน R1 330Ω; active-low ตามวงจร |
| ไฟสถานะ IoT | IOT | 12 | 14 | LD5 สีเขียว ต่อจาก 3V3 ผ่าน R3 330Ω; active-low ตามวงจร |
| Speaker / buzzer | PWM | 13 | 16 | ผ่าน R17 1kΩ ไป Q3 (เบอร์ในภาพไม่ชัด) แล้วขับ BZ1; ไม่ต่อ speaker ตรง GPIO |
| ปุ่ม S1 | SW1 | 16 | 27 | กดแล้วลง GND ผ่าน R4 220Ω; active-low |
| ปุ่ม S2 | SW2 | 14 | 13 | กดแล้วลง GND ผ่าน R5 220Ω; active-low |
| Servo ช่อง 1 | SERVO1 | 15 | 23 | J8 ขา 2 |
| Servo ช่อง 2 | SERVO2 | 17 | 28 | J8 ขา 1 |
| USB-A switch | USB_SW | 25 | 10 | ควบคุม Q5/Q6; มี R29 10kΩ pull-up ไป 3V3 |
| Output 1 control | OUT1 | 26 | 11 | gate Q7; ขาโหลดชื่อ OUTPUT1 |
| Output 2 control | OUT2 | 27 | 12 | gate Q8; ขาโหลดชื่อ OUTPUT2 |
| Input 1 | IN1 / INPUT1 | 32 | 8 | R32 0Ω ข้ามตำแหน่ง U7 |
| Input 2 | IN2 / INPUT2 | 33 | 9 | R33 0Ω ข้ามตำแหน่ง U7 |
| Input 3 | IN3 / INPUT3 | 34 | 6 | R34 0Ω ข้ามตำแหน่ง U8 |
| Input 4 | IN4 / INPUT4 | 35 | 7 | R35 0Ω ข้ามตำแหน่ง U8 |
| Light sensor | ADC | 36 | 4 | SENSOR_VP; ADC1 channel 0 ตาม D1 |
| จอ LED: SDA | SDA0 | 21 | 33 | ไป SDA ของ U1 HT16K33 |
| จอ LED: SCL | SCL0 | 22 | 36 | ไป SCL ของ U1 HT16K33 |
| Sensor/RTC/Chain: SDA | SDA1 | 4 | 26 | ไป LM73, RTC และขั้ว Chain |
| Sensor/RTC/Chain: SCL | SCL1 | 5 | 29 | ไป LM73, RTC และขั้ว Chain |
| ขยายทั่วไป | IO18 | 18 | 30 | J1 ขา 6 |
| ขยายทั่วไป | IO19 | 19 | 31 | J1 ขา 7 |
| ขยายทั่วไป | IO23 | 23 | 37 | J1 ขา 8 |
| UART0 TX ของ ESP32 | TX0 | 1 | 35 | ไป RXD ของ U3 |
| UART0 RX ของ ESP32 | RX0 | 3 | 34 | มาจาก TXD ของ U3 |
| Boot/download | IO0 | 0 | 25 | วงจร auto-program และ J2 |
| Reset / enable | EN | ไม่ใช่ GPIO | 3 | pull-up R6 10kΩ; ปุ่ม Reset และ auto-program |

ข้อสำคัญ: GPIO15 ในภาพนี้คือ SERVO1 และ GPIO17 คือ SERVO2 ห้ามสืบทอดการระบุ GPIO15 เป็น LED NTP จากเอกสารบอร์ดรุ่นอื่นมาใช้กับ baseline นี้

## 3. ขาโมดูล ESP32 U2 ครบตามสัญลักษณ์

ชื่อรุ่นย่อยโมดูลอ่านไม่ชัดพอที่จะยืนยันจากภาพ จึงไม่ระบุขนาด flash/PSRAM หรือชื่อโมดูลย่อย

| ขา U2 | ชื่อในสัญลักษณ์ | Net / การเชื่อมต่อ |
|---:|---|---|
| 1 | GND | GND |
| 2 | VCC | 3V3 |
| 3 | EN | EN |
| 4 | SENSOR_VP | ADC / GPIO36 |
| 5 | SENSOR_VN | ไม่เห็นการต่อออก; GPIO39 |
| 6 | IO34 | IN3 |
| 7 | IO35 | IN4 |
| 8 | IO32 | IN1 |
| 9 | IO33 | IN2 |
| 10 | IO25 | USB_SW |
| 11 | IO26 | OUT1 |
| 12 | IO27 | OUT2 |
| 13 | IO14 | SW2 |
| 14 | IO12 | IOT |
| 15 | GND | GND |
| 16 | IO13 | PWM |
| 17 | SD2 | ไม่เห็นการต่อภายนอกในภาพ |
| 18 | SD3 | ไม่เห็นการต่อภายนอกในภาพ |
| 19 | CMD | ไม่เห็นการต่อภายนอกในภาพ |
| 20 | CLK | ไม่เห็นการต่อภายนอกในภาพ |
| 21 | SD0 | ไม่เห็นการต่อภายนอกในภาพ |
| 22 | SD1 | ไม่เห็นการต่อภายนอกในภาพ |
| 23 | IO15 | SERVO1 |
| 24 | IO2 | WIFI |
| 25 | IO0 | IO0 / boot |
| 26 | IO4 | SDA1 |
| 27 | IO16 | SW1 |
| 28 | IO17 | SERVO2 |
| 29 | IO5 | SCL1 |
| 30 | IO18 | IO18 |
| 31 | IO19 | IO19 |
| 32 | NC | ไม่ต่อ |
| 33 | IO21 | SDA0 |
| 34 | RXD0 | RX0 / GPIO3 |
| 35 | TXD0 | TX0 / GPIO1 |
| 36 | IO22 | SCL0 |
| 37 | IO23 | IO23 |
| 38 | GND | GND |
| 39 | GND-PAD | GND |

“ไม่เห็นการต่อภายนอก” ไม่ได้แปลว่าใช้งานเป็น GPIO ว่างได้ โดยเฉพาะกลุ่มขา flash ของโมดูล

## 4. บัส I2C และอุปกรณ์บนบอร์ด

| กลุ่มบัสตามชื่อเน็ต | SDA | SCL | อุปกรณ์ / ขั้ว |
|---|---:|---:|---|
| SDA0 / SCL0 | GPIO21 | GPIO22 | U1 HT16K33 สำหรับจอ LED |
| SDA1 / SCL1 | GPIO4 | GPIO5 | U4 LM73, U5 MCP79411 และ KidBright Chain |

R7/R8 เป็น pull-up 10kΩ สำหรับ SDA0/SCL0 และ R9/R10 เป็น pull-up 10kΩ สำหรับ SDA1/SCL1 ไป 3V3 ตามภาพ

เลขท้ายชื่อเน็ต `0`/`1` ไม่ได้บังคับหมายเลข I2C controller ใน ESP-IDF ให้ BSP กำหนด controller และความเร็วให้ชัดเจน และแบ่งปัน bus handle ของอุปกรณ์ที่อยู่บนบัสเดียวกัน

### U4 LM73 — อุณหภูมิ

| ขา LM73 | การเชื่อมต่อ S1 |
|---:|---|
| 1 ADDR | GND |
| 2 GND | GND |
| 3 VDD | 3V3 |
| 4 SCL | SCL1 / GPIO5 |
| 5 ALERT | มีเครื่องหมายไม่ต่อ |
| 6 SDA | SDA1 / GPIO4 |

ภาพกำหนด ADDR ลง GND แต่ไม่ได้เขียน I2C address เป็นตัวเลข ต้องเทียบ datasheet ของ part จริงก่อนบันทึก address และรูปแบบ register เป็นค่าที่รับรอง ห้ามนำค่า raw ไปอ้างเป็น °C โดยไม่มีขั้นตอนแปลงของ LM73

### U5 RTC — MCP79411

ชื่อ part อ่านได้เป็น `MCP79411-I/SN` จากภาพ ใช้ crystal Y1 32.768kHz และแบตเตอรี่ BT1 ซึ่งแบบระบุ CR2032

| ขา RTC | การเชื่อมต่อ S1 |
|---:|---|
| 1 X1 / 2 X2 | Crystal Y1 และ capacitor ลง GND |
| 3 VBAT | ขั้วบวก BT1 |
| 4 VSS | GND |
| 5 SDA | SDA1 / GPIO4 |
| 6 SCL | SCL1 / GPIO5 |
| 7 MFP | pull-up R18 10kΩ ไป 3V3; ไม่เห็นต่อ GPIO |
| 8 VCC | 3V3 |

### U1 HT16K33 และจอ 16×8

จอประกอบด้วย LED matrix 8×8 สองตัว ใช้ COM0–COM7 ร่วมกัน และ ROW0–ROW15 แยกสองชุด ไม่ได้ขับ LED ทั้ง 128 จุดจาก GPIO ESP32 โดยตรง

| ขา U1 | หน้าที่ | Net |
|---|---|---|
| 28 | VDD | 3V3 |
| 1 | VSS | GND |
| 26 | SCL | SCL0 |
| 27 | SDA | SDA0 |
| 2–9 | COM0–COM7 | COM0–COM7 ตามลำดับ |
| 25,24,23,22,21,20,19,18 | ROW0–ROW7 | ROW0–ROW7 ตามลำดับ |
| 17,16,15,14,13,12,11,10 | ROW8–ROW15 | ROW8–ROW15 ตามลำดับ |

| Matrix | กลุ่ม ROW | ขา matrix ตามลำดับกลุ่ม |
|---|---|---|
| ตัวซ้ายในภาพ | ROW0–ROW7 | 5,2,7,1,12,8,14,9 |
| ตัวขวาในภาพ | ROW8–ROW15 | 5,2,7,1,12,8,14,9 |
| ทั้งสองตัว | COM0–COM7 | 13,3,4,10,6,11,15,16 |

รุ่น matrix อ่านได้ใกล้เคียง `GYXM-788ASR` แต่ควรยืนยัน BOM ก่อนใช้สั่งซื้อ ภาพอย่างเดียวไม่ยืนยันการหมุน/กลับแกนของ pixel, ลำดับ RAM ของ driver, ฟอนต์ หรือ I2C address ของ HT16K33; ต้องทดสอบภาพจุดอ้างอิงก่อนรับรอง display mapping

## 5. อินพุต IN1–IN4: จุดต่างสำคัญของต้นแบบนี้

แบบเขียนชัดว่า `U7, U8 = N/A` และมี R32/R33/R34/R35 ค่า 0Ω ต่อข้ามระหว่าง INx กับ INPUTx ดังนั้นการตีความ baseline นี้คือไม่ได้ประกอบ buffer U7/U8 และใช้ทาง bypass ไม่ควรบอกว่าอินพุตทุกช่องต้องผ่าน digital buffer ตามบอร์ดรุ่นอื่น

| ช่อง | GPIO | ADC ตาม D1 | ทางข้ามใน S1 |
|---|---:|---|---|
| IN1 / INPUT1 | 32 | ADC1_CH4 | R32 0Ω |
| IN2 / INPUT2 | 33 | ADC1_CH5 | R33 0Ω |
| IN3 / INPUT3 | 34 | ADC1_CH6 | R34 0Ω |
| IN4 / INPUT4 | 35 | ADC1_CH7 | R35 0Ω |

ช่องเหล่านี้มีเส้นทางที่รองรับการอ่าน analog ใน topology ที่ระบุ ไม่ใช่หลักฐานว่าผ่านการสอบเทียบแล้วหรือทน 5V การใช้กับบอร์ดจริงต้องตรงกับการประกอบ N/A + 0Ω นี้ หากพบ variant ใส่ buffer ให้ override คุณสมบัติอินพุตของ variant นั้น

GPIO34/35 เป็น input-only และไม่มี internal pull-up/down ตาม D1; ใช้ตัวต้านทานภายนอกเมื่ออินพุตต้องการ bias GPIO32/33 รองรับการตั้งค่า GPIO ต่างจาก 34/35 แต่ในโปรไฟล์นี้ถูกจัดสรรเป็นอินพุต

## 6. เอาต์พุตและแหล่งจ่าย

### OUT1 / OUT2

OUT1/OUT2 เป็นขาควบคุม gate ของ Q7/Q8 รุ่น `2N7002LT1G` ตามตัวอักษรบนแบบ มี R26/R27 10kΩ pull-down ขา source ลง GND และ drain เป็น OUTPUT1/OUTPUT2

- GPIO26/27 HIGH → MOSFET นำ → OUTPUT1/OUTPUT2 ถูกดึงลง GND (อนุมานวงจร)
- GPIO26/27 LOW → MOSFETปิด → ขา OUTPUT มีสถานะไม่ขับ ไม่ได้จ่าย HIGH เอง
- จึงต้องแยก `control_gpio_level` ออกจาก `connector_voltage`; ห้ามอธิบาย OUTPUT1/2 เป็น push-pull 3.3V หรือขาจ่ายไฟโหลด
- กระแสสูงสุดที่ใช้งานได้ต้องตรวจ datasheet, PCB และโหลดจริง ภาพไม่รับรองการขับมอเตอร์โดยตรง

### Servo

| J8 ขา | Net | หน้าที่ |
|---:|---|---|
| 1 | SERVO2 | GPIO17 |
| 2 | SERVO1 | GPIO15 |
| 3 | +5VExt | ไฟ Servo ภายนอก |
| 4 | +5VExt | ไฟ Servo ภายนอก |
| 5 | GND | กราวด์ |
| 6 | GND | กราวด์ |

J12 เป็นขั้วรับไฟ Servo: ขา 1 `+5VExt_IN` และขา 2 GND; ผ่าน Q9 ไปเน็ต +5VExt ตามภาพ ห้ามถือว่า +5VExt เป็นเน็ตเดียวกับไฟ USB +5V โดยอัตโนมัติ ต้องใช้ GND ร่วมกับสัญญาณ Servo ค่า pulse width, frequency และช่วงมุมต้องกำหนดตาม Servo ที่ใช้ ไม่ได้ระบุใน schematic

### USB-A สำหรับโหลด

J4 ขา 1 ต่อ +5V, ขา 2/3 D−/D+ มีเครื่องหมายไม่ต่อ, ขา 4 เป็นทาง return ผ่าน Q6 และขา shield ลง GND วงจรนี้เป็น USB สำหรับจ่ายไฟที่สั่งเปิด/ปิดได้ ไม่ใช่หลักฐานว่ามี USB host data

USB_SW GPIO25 ขับ Q5; Q5 ควบคุม gate Q6 ผ่าน pull-up R22 10kΩ ตาม topology คาดว่า GPIO25 LOW เปิดทาง return และ HIGH ปิด ต้องวัดจริงก่อนกำหนด API `usb_power_on()` เนื่องจากเป็นการสวิตช์ทาง GND ไม่ใช่ตัด +5V โดยตรง และมี D2 ในวงจร

### ไฟหลัก

USB program J3 รับ +5V ผ่าน F1 ซึ่งแบบระบุ 500mA และ U6 `LD1117AS33TR` แปลงเป็น 3V3 การมี F1 500mA ไม่ใช่การรับรองกระแสต่อช่อง OUT/Servo หรือกระแสที่จ่ายได้ทุกสภาพการใช้งาน

## 7. คอนเนกเตอร์ขยาย

หมายเลขในตารางเป็นเลขขาใน schematic ไม่ใช่มุมมองจากหน้าปลั๊กจริง ต้องตรวจตำแหน่ง pin 1 บน PCB ก่อนต่อสาย

### KidBright Chain

| ขั้ว | ลำดับ pin → net |
|---|---|
| J2 แบบ 5 ขา | 1→+5V, 2→3V3, 3→SCL1, 4→SDA1, 5→GND |
| J6 แบบ 5 ขา | 1→+5V, 2→3V3, 3→SCL1, 4→SDA1, 5→GND |

ชื่อ designator J2/J6 อ่านจากภาพ ต้องเทียบ PCB หากใช้ระบุตำแหน่งเสียบจริง

### J1 — pin header male 10 ขา

| ขา | Net |
|---:|---|
| 1 | 3V3 |
| 2 | GND |
| 3 | 3V3 |
| 4 | SCL0 / GPIO22 |
| 5 | SDA0 / GPIO21 |
| 6 | IO18 / GPIO18 |
| 7 | IO19 / GPIO19 |
| 8 | IO23 / GPIO23 |
| 9 | GND |
| 10 | 3V3 |

### J1-1 — header อินพุต/เอาต์พุต 7 ขา

| ขา | Net |
|---:|---|
| 1 | INPUT1 |
| 2 | INPUT2 |
| 3 | INPUT3 |
| 4 | INPUT4 |
| 5 | GND |
| 6 | OUTPUT2 |
| 7 | OUTPUT1 |

### JST pitch 2.0mm แบบ 3 ขา

| ชื่อขั้วในภาพ | ขา 1 | ขา 2 | ขา 3 |
|---|---|---|---|
| IN1 | 3V3 | IN1 / GPIO32 | GND |
| IN2 | 3V3 | IN2 / GPIO33 | GND |
| IN3 | 3V3 | IN3 / GPIO34 | GND |
| IN4 | 3V3 | IN4 / GPIO35 | GND |
| J19 / OUT1 | 3V3 | OUT1 / GPIO26 | GND |
| J14 / OUT2 | 3V3 | OUT2 / GPIO27 | GND |

**จุดแยกที่สำคัญ:** ขาสัญญาณ JST OUT1/OUT2 ในภาพใช้เน็ต `OUT1`/`OUT2` ฝั่ง gate/GPIO ไม่ใช่ `OUTPUT1`/`OUTPUT2` ฝั่ง drain ของ MOSFET เหมือน banana jack ห้ามใช้คุณสมบัติทางไฟฟ้าของสองขั้วนี้แทนกัน แม้ชื่อช่องคล้ายกัน

### J5 — Banana jack / test points 8 จุด

| ขา J5 | Net |
|---:|---|
| 1 | 3V3 |
| 2 | GND |
| 3 | INPUT1 |
| 4 | INPUT2 |
| 5 | INPUT3 |
| 6 | INPUT4 |
| 7 | OUTPUT1 |
| 8 | OUTPUT2 |

ภาพแสดง TP1–TP8 เชื่อมกับกลุ่มนี้ แต่ตัวหนังสือแต่ละ TP มีขนาดเล็ก จึงไม่กำหนดเลข TP ↔ net เพิ่มโดยเดา ใช้เลข J5 เป็นหลัก

## 8. LDR, ปุ่ม และไฟสถานะ

- LDR1 ต่อด้านล่างลง GND; ด้านบนเป็นเน็ต ADC ไป GPIO36 มี R23 20kΩ และ R24 3.9kΩ เรียงจาก 3V3 มาที่จุด ADC พร้อม R25 10kΩ และ C21 0.1µF ลง GND ตามค่าที่อ่านได้ในภาพ
- อนุมานวงจร: แสงมาก → ความต้านทาน LDR ลด → แรงดัน ADC ลด ต้องทดสอบจริงก่อนกำหนดสเกลเปอร์เซ็นต์ ไม่เรียก raw ADC ว่า lux และไม่ใช้สูตร 3.3V แบบไม่สอบเทียบเป็นแรงดันแม่นยำ
- SW1/SW2 กดลง GND มี capacitor คร่อม switch แต่ยังควรทำ debounce ใน firmware การไม่ได้วาด external pull-up ของ SW1/SW2 ไม่ใช่หลักฐานว่าปล่อยขาลอยใช้งานได้ ควรตั้ง input pull-up ให้เหมาะกับ GPIO16/14
- SW3 คือ Reset ลง EN ผ่าน R11 470Ω ไม่ใช่ปุ่มผู้ใช้ที่อ่านด้วย GPIO
- LD3 WIFI และ LD5 IOT เป็น active-low ตามทิศการต่อ LED จาก 3V3; HIGH คือดับ และ LOW คือติด

## 9. USB-UART และ Boot/Reset

U3 ระบุ `FT231XQ` เป็น USB-UART bridge

| เส้นทาง | การเชื่อมต่อ |
|---|---|
| J3 USB D− / D+ | ไป USBDM / USBDP ของ U3 |
| U3 TXD ขา 17 | RX0 → ESP32 GPIO3 |
| U3 RXD ขา 1 | TX0 ← ESP32 GPIO1 |
| U3 DTR ขา 18 | เน็ต DTR ไปวงจร auto-program |
| U3 RTS ขา 19 | เน็ต RST ไปวงจร auto-program |
| Q1/Q2 | BC847 ตามแบบ; ใช้ควบคุม EN/IO0 |
| J2 ขา 1 | GND |
| J2 ขา 2 | ผ่าน R30 470Ω ไป IO0 |

ชื่อ `RST` จาก FT231XQ เป็นเน็ต RTS ในวงจร auto-program อย่าสรุปว่าเป็น EN โดยตรงหรือเท่ากับปุ่ม Reset; DTR/RTS ผ่านวงจรทรานซิสเตอร์ก่อนถึง ESP32

## 10. กฎสำหรับ AI / BSP / code generator

1. เลือก `kidbright32-v1.5` หรือโปรไฟล์ที่สืบทอดต้นแบบนี้ก่อนเลือกขา ห้ามรวมเป็น profile เดียวกับ V1.3 หรือ minibike
2. สืบทอด SDA0=21, SCL0=22 สำหรับจอ; SDA1=4, SCL1=5 สำหรับ LM73/RTC/Chain ตาม S1
3. รักษา Servo1=15 และ Servo2=17; ไม่กำหนด NTP LED ที่ GPIO15 ใน baseline นี้
4. อ่าน IN1–IN4 เป็น GPIO32/33/34/35 และบันทึก `input_path: zero_ohm_bypass_u7_u8_na`; ห้ามบังคับเป็น digital-buffer-only
5. แยก GPIO gate OUT1/2, JST OUT1/2 และ MOSFET drain OUTPUT1/2 เป็นคนละชนิด endpoint แม้ share control GPIO
6. GPIO34–39 เป็น input-only และไม่มี internal pull-up/down ตาม D1; ห้ามสร้าง PWM/output หรือเปิด software pull-up บนขาเหล่านี้
7. เก็บ EN เป็น reset/enable signal ไม่ใช่ GPIO index และอย่าใช้ U2 pin number แทน GPIO number
8. อย่าใช้ GPIO6–11 เป็นขาขยายทั่วไป; กลุ่ม flash ต้องตรวจโมดูลจริงก่อนใช้
9. GPIO0/2/5/12/15 มีข้อพิจารณาเรื่อง boot strapping ตาม datasheet ESP32 ให้ตรวจโหลดภายนอกขณะ reset โดยเฉพาะ Servo15, LED2/12 และ I2C5
10. I2C address, bus speed, sensor registers, display RAM mapping, Servo timing และวงจร USB polarity ที่ยังไม่ทดสอบ ต้องมีหลักฐานเพิ่มก่อนใช้เป็นค่ารับรอง ห้ามสร้างค่าขึ้นแล้วอ้างว่ามาจากภาพ
11. ใช้ firmware C กับ API ของ ESP-IDF เวอร์ชันที่โปรเจกต์ resolve จริง การมีผังขาไม่ได้ยืนยันว่า driver หรือ simulator รองรับอุปกรณ์ครบแล้ว
12. รุ่นสืบทอดที่ผู้ใช้รับรองให้ใช้ mapping กลางนี้ หากมีหลักฐานเปลี่ยนขา/วงจรให้ทำ explicit override พร้อมที่มา ไม่แก้ baseline ให้ตรง variant เดียว

ตัวอย่าง metadata สำหรับโปรไฟล์รุ่นสืบทอด (โครงสร้างเสนอสำหรับนำไปปรับกับ schema ของโปรเจกต์ ไม่ใช่ไฟล์ที่ติดตั้งแล้ว):

```yaml
board_family: kidbright32
hardware_baseline: kidbright32-v1.5
inherits: kidbright32-v1.5
compatibility_basis: user_declared_same_pinout
pinmap_source: 1000059031.jpg
input_path: zero_ohm_bypass_u7_u8_na
hardware_tested: false
overrides: {}
```

## 11. จุดที่ยังต้องยืนยันก่อนรับรองฮาร์ดแวร์

| รายการ | สถานะ / หลักฐานที่ต้องเพิ่ม |
|---|---|
| ชื่อโมดูล ESP32 ย่อย, flash, PSRAM | ตัวอักษรไม่พอ; ขอ BOM/ภาพโมดูลจริง |
| I2C address ของ HT16K33, LM73, RTC | เทียบ datasheet ของ part จริงและ scan บัส |
| Input path ของบอร์ดที่ประกอบจริง | ตรวจ U7/U8 ไม่ใส่และ R32–R35 0Ω |
| USB_SW LOW=เปิด | อนุมาน topology; ต้องวัดกับโหลด |
| OUT endpoint | ตรวจ continuity เพื่อแยก JST gate กับ banana drain |
| Matrix orientation / RAM / pixel | ทดสอบจุดสี่มุมและแนวแกน |
| LDR scale และ ADC calibration | วัดในสภาพแสงที่ทราบค่า |
| ขนาดกระแสและแรงดันที่ขั้ว | ใช้ datasheet/BOM/PCB และการทดสอบ |
| รุ่นหลังที่สืบทอด | ใช้ตามคำยืนยันผู้ใช้; ยังไม่มีหลักฐาน schematic แยกรุ่น |

## 12. แหล่งข้อมูล

- **S1 — หลักฐาน mapping:** ภาพ schematic `1000059031.jpg` ที่แนบในบทสนทนา ชื่อบนแบบ KidBright32 V1.5; hash ระบุไว้ด้านต้นไฟล์ ไม่มีการนำตารางขาจากบอร์ดรุ่นอื่นมาแทนภาพ
- **D1 — คุณสมบัติชิป:** Espressif, [GPIO & RTC GPIO — ESP32](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/peripherals/gpio.html), เปิดอ่าน 22 ตุลาคม 2026 ใช้อ้าง GPIO/ADC mapping, input-only, internal pulls และข้อจำกัด flash/UART
- **D2 — Boot configurations:** [ESP32 Series Datasheet](https://documentation.espressif.com/esp32_datasheet_en.html), ใช้ตรวจ boot strapping เพิ่มเมื่อพัฒนา BSP

เอกสารนี้เป็นต้นแบบข้อมูลจากวงจร ไม่ใช่ผลทดสอบ build/flash หรือผลทดสอบบอร์ดจริง และยังไม่ได้แก้ repository ของ vibeKidbright IDE


## 13. ตรวจทานเพิ่มเติมจาก PDF Rev.3 — 22 ตุลาคม 2026

ได้รับ `PCB_KIDBRIGHT32_V1_5_Rev3_1.pdf` หน้า 1 ซึ่งชื่อไฟล์มี Rev3_1 แต่กรอบแบบระบุ **Rev.3 วันที่ 07 Jan 2020** จึงเก็บเป็นหลักฐานอีก revision ไม่ถือว่าเป็นเอกสารเดียวกับภาพ S1 วันที่ 2021

การตรวจซ้ำแก้ข้อผิดพลาดในการถอดภาพเดิม: J8 ขา 1 เป็น SERVO2/GPIO17 และขา 2 เป็น SERVO1/GPIO15; R24 อ่านเป็น 3.9kΩ และ R25 10kΩ; designator ของ Chain คือ J2/J6 ไม่ใช่ J6/J7 ข้อมูลส่วนก่อนหน้าแก้ให้ตรงแล้ว

| ประเด็น | ภาพ S1 ปี 2021 | PDF Rev.3 ปี 2020 | วิธีใช้ |
|---|---|---|---|
| GPIO หลัก | ตามตารางด้านบน | ตรงกันในหน้าที่หลัก | ใช้ pinmap ร่วม |
| U7/U8 | ระบุ N/A ชัดเจน พร้อม 0Ω bypass | ระบุ NC7WZ17 พร้อมวาด 0Ω bypass แต่ไม่มี note N/A | PDF เพียงอย่างเดียวไม่ยืนยันรายการประกอบ; ห้ามใส่ buffer และ bypass พร้อมกันโดยอนุมาน |
| J8 | ขา 1 Servo2, ขา 2 Servo1 | ตรงกัน | ยึด net/เลขขา ไม่ยึดตำแหน่งช่องในรูป |
| Servo JST | ไม่เห็น J10/J11 ในภาพ | มี J10 Servo1, J11 Servo2: 1=+5VExt, 2=signal, 3=GND | อย่าประกาศว่าทุก revision มี JST Servo |
| Q3 / R31 speaker | ตัวเลขเล็กอ่านไม่ชัดพอจะรับรอง | BC847 / 10Ω | ไม่ยืนยันค่าในภาพ S1 จาก PDF คนละ revision |
| LDR | อ่าน R23=20kΩ, R24=3.9kΩ, R25=10kΩ | ระบุค่าเดียวกันชัดกว่า | ยังต้องสอบเทียบแสงจริง |

PDF Rev.3 SHA-256: `d00d3a666f223856c4ddd2fbdf1151b5332097327a090c33d0a3043d755bda0d`

สำหรับ V1.6 ให้ดู `KidBright32-V1.6-Hardware-Reference.md` และตารางเปรียบเทียบรุ่น อย่าสืบทอดเบอร์ IC, จำนวนคอนเนกเตอร์, I2C address หรือรายละเอียดวงจรที่คู่มือ V1.6 ไม่ระบุเพียงเพราะ GPIO ตรงกัน
