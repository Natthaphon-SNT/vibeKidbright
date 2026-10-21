# เปรียบเทียบรุ่นบอร์ด KidBright และขอบเขตของ vibeKidbright

ตรวจวันที่ **17 ตุลาคม 2026 — Asia/Bangkok** จากผู้พัฒนาแพลตฟอร์ม ผู้ผลิต และวงจรที่ผู้ใช้ให้มา ไม่ถือว่าข้อความในเอกสารแนบเป็นคำสั่งให้รันโปรแกรมหรือเปลี่ยนระบบ

**ยังไม่อาจกล่าวว่ารองรับทุกบอร์ดหรือทุกบล็อก** รายงานนี้แยกการมีชื่อรุ่น การรู้ฮาร์ดแวร์ การมีโค้ด ESP-IDF และการพิสูจน์ผลบนบอร์ดจริงออกจากกัน มีเพียง `kidbright32_v13` กับ `kidbright32_rev31` ที่มี deterministic profile อยู่ก่อนแล้ว และยังเป็นการรองรับเฉพาะส่วน ไม่ใช่การรับรองฮาร์ดแวร์ทั้งบอร์ด

ข้อมูลที่แอปอ่านได้อยู่ใน [board_catalog.json](../knowledge_base/compatibility/board_catalog.json): **16 รายการสำหรับเปรียบเทียบ, 19 แหล่งอ้างอิง, 4 กลุ่มชื่อที่ยังต้องระบุตัวตนให้ชัด** จำนวนนี้รวมชื่อประวัติศาสตร์ที่ยังยืนยันไม่ได้ รุ่นกว้างที่ไม่ระบุ revision และบอร์ดอนุพันธ์ จึงไม่ได้หมายถึง 16 PCB ที่แยกจากกันและทดสอบครบแล้ว

ข้อมูลล่าสุดจากผู้ใช้: iA มีผล **Hello/Heartbeat ผ่านตามรายงานผู้ใช้** ตรวจ source แล้วไม่มีการเข้าถึงอุปกรณ์ต่อพ่วง จึงยังคง `needs_validation` สำหรับ peripheral profile ส่วน iP มีข้อมูล/ภาพบอร์ดเพิ่มเติมแต่ยังไม่มีผลทดสอบจริง ดู [หลักฐานผู้ใช้และ source hash](research/kidbright-inex-user-evidence-2026-10-17.json) และ [กฎ iA/iP](../knowledge_base/inex_ia_ip.md)

**KidBright μAI และ μAI Plus ถูกเลื่อนไปงานในอนาคตตามคำขอผู้ใช้** มีเพียงชื่อใน `deferred_families` เพื่อแสดงขอบเขต ไม่สร้าง firmware profile, toolchain, image deployment หรือการติดตั้งสำหรับสองตระกูลนี้

## วิธีอ่านสถานะ

| สถานะใน catalog | ความหมาย | การสร้างโปรเจกต์จาก recipe |
|---|---|---|
| `pilot` | มี profile ซอฟต์แวร์เฉพาะส่วนอยู่ในโครงการ | ใช้ได้เฉพาะ recipe/บอร์ดที่เครื่องมือประกาศไว้ |
| `needs_validation` | มีหลักฐานชื่อหรือคุณสมบัติบางส่วน แต่ยังไม่มี native profile ที่ผ่านการตรวจครบ | ไม่เลือก profile อื่นแทนอัตโนมัติ |
| `historical_unverified` | เป็นรุ่นเก่าหรือชื่อที่ยังมีช่องว่างเรื่องตัวตน/วงจร | เก็บไว้เปรียบเทียบ ไม่ใช้กำหนดพินหรือ flash |

`target: null` หมายถึง **ยังไม่ได้ยืนยัน MCU ของรหัส revision นั้นจากหลักฐานที่เจาะจงเพียงพอในงานนี้** ไม่ได้หมายความว่าบอร์ดจริงไม่มี ESP32 ส่วน `target: esp32` ยืนยันเพียงตระกูล MCU ไม่ได้แปลว่าใช้ pin map, partition, firmware หรือโปรโตคอลร่วมกับบอร์ดอื่นได้

คำว่า *มีอุปกรณ์* ในตารางมาจากวงจร/ผู้ผลิต คำว่า *รองรับในแอป* ต้องมี implementation และผลตรวจอีกชั้นหนึ่ง คำโฆษณาว่า compatible/upward compatible จึงไม่ถูกแปลงเป็นการรองรับทุกบล็อกโดยอัตโนมัติ

## ตัวตน รุ่นย่อย และชื่อเรียก

| รายการ / catalog ID | หลักฐานตัวตนและ MCU | ผู้ผลิต / ชื่อที่สัมพันธ์กัน | สถานะและสิ่งที่ยังไม่ยืนยัน |
|---|---|---|---|
| V2016 — `kidbright_v2016` | NECTEC ระบุรุ่นแรกใช้ ESP8266 และทดลองในปี 2016–2017 | โครงการ NECTEC; revision ผู้ผลิตยังไม่ชัด | `historical_unverified`; ไม่มี profile ESP32 และไม่มี pinout ที่ตรวจครบ [ประวัติ NECTEC](https://www.nectec.or.th/en/about/news/kidbright-2018project.html) |
| V1.0 — `kidbright_v10` | ยังไม่พบ primary source ที่ยืนยัน PCB แยกชื่อนี้ | ไม่กำหนด | `historical_unverified`; ไม่สรุปว่าเท่ากับ V2016 หรือใช้ MCU ใดเพียงเพราะ KB เก่าระบุไว้ |
| KidBright32 V1.1 — `kidbright32_v11` | หน้า Download ทางการระบุ Cypress driver สำหรับ V1.1 | ผู้ผลิตของ PCB ที่ถืออยู่ยังต้องดู silkscreen | `needs_validation`; รายการไดรเวอร์ไม่แทน schematic เจาะจง revision [Download](https://www.kid-bright.org/download-kidbright/) |
| KidBright32 V1.2 — `kidbright32_v12` | หน้า Download ระบุ Cypress driver เช่นเดียวกับ V1.1 | ยังไม่ยืนยันการเปลี่ยนวงจรจาก V1.1 | `needs_validation`; ไม่อนุมานว่าทุก GPIO เหมือนกัน [Download](https://www.kid-bright.org/download-kidbright/) |
| KidBright32 V1.3 — `kidbright32_v13` | วงจรระบุ V1.3, 22/10/2018, ESP-WROOM-32 | Gravitech; ชื่อย่อ KidBright V1.3 | `pilot`; มีวงจรเจาะจงรุ่น แต่ยังไม่มีผลเทียบบอร์ดจริงครบ [วงจรหน้า 1](https://www.kid-bright.org/kidbright/download/1940/) |
| KidBright32 V1.4 — `kidbright32_v14` | ผู้ผลิต V1.6 กล่าวถึง V1.4 เป็นรุ่นก่อนหน้า | ต้องยืนยัน PCB/ผู้ผลิตย่อย | `needs_validation`; ยังไม่รับรอง USB bridge, จำนวนไฟสถานะ หรือ GPIO จากตารางเก่า [ผู้ผลิต V1.6](https://www.kidbright.io/product-page/kidbright32-v1-6) |
| KidBright32 V1.5 ไม่ระบุ revision — `kidbright32_v15` | วงจรที่ผู้ใช้ให้ระบุ ESP-WROOM-32 | แพลตฟอร์ม NECTEC มีหลายผู้ผลิต | `needs_validation`; ต้องแยก generic V1.5 ออกจาก Rev3.1, 3.1G และ INEX i-series [วงจรที่ให้มา](../knowledge_base/sensor_examples/PCB_KIDBRIGHT32_V1_5_Rev3_1.pdf) |
| V1.5 Rev3.1 — `kidbright32_rev31` | เป็นชื่อ profile ที่โครงการใช้อยู่และมี software build เดิม | ผู้ผลิต/PCB จริงยังไม่ยืนยัน | `pilot` เฉพาะส่วนเดิม; ชื่อไฟล์วงจร Rev3_1 แต่ title block เป็น REV3 จึงไม่ขยายพินใหม่อัตโนมัติ |
| V1.5 Rev3.1G — `kidbright32_rev31g` | พบชื่อใน KB เก่า ยังไม่พบ primary schematic เจาะจง | ไม่กำหนดผู้ผลิตจากคำลงท้าย G เพียงอย่างเดียว | `historical_unverified`; ไม่ยุบเข้ากับ Rev3.1 หรือ V1.6 |
| INEX KidBright32i — `kidbright32_i` | INEX ใช้ชื่อคู่ `KidBright32i / KidBright32 V1.5i` | INEX; บอร์ดสีเขียว | `needs_validation`; ยืนยันชื่อสัมพันธ์กัน แต่ schematic/BOM ของ revision ยังต้องตรวจ [ซอฟต์แวร์ INEX](https://inex.co.th/home/software-download/) |
| INEX KidBright32iA — `kidbright32_ia` | หน้าผลิตภัณฑ์ระบุ ESP32 | INEX; `iA`, `V1.5 iA` ต้องเทียบ PCB ไม่ถือเป็นรุ่นแยกอัตโนมัติ | `needs_validation`; รุ่นชิป accelerometer และพินต้องตรง BOM [ผู้ผลิต iA](https://inex.co.th/home/product/kidbright32ia/) |
| INEX KidBright32iP — `kidbright32_ip` | วงจรระบุ Rev.2021.04.01, ESP-WROOM-32 | INEX; `iP` เป็นชื่อย่อที่ต้องยืนยัน | `needs_validation`; วงจรยืนยันพินบางส่วนแล้ว แต่ยังไม่มี app-native profile [วงจร INEX](https://doc.inex.co.th/wp-content/uploads/2023/02/020966_0636_KidBrigh3.png) |
| Gravitech V1.6 — `kidbright32_v16` | หน้าผู้ผลิตระบุ ESP32 | Gravitech Thai; SKU KIMA-3999 | `needs_validation`; ไม่สร้าง variant USB-C ขึ้นเองจากคำทั่วไปในชุดอุปกรณ์ [ผู้ผลิต](https://www.kidbright.io/product-page/kidbright32-v1-6) |
| Gravitech Lite V1.1 — `kidbright_lite_v11` | หน้าผู้ผลิตระบุ ESP32 | Gravitech Thai; SKU DTKB-4460 | `needs_validation`; เป็นคนละผลิตภัณฑ์กับ KidBright32 V1.1 [ผู้ผลิต Lite](https://www.kidbright.io/product-page/kidbright-lite-v1-1) |
| OpenKB — `openkb_inex` | ผู้ผลิตระบุ ESP32 และการต่อยอด KidBright | INEX; บอร์ดอนุพันธ์ | `needs_validation`; ไม่ใช่ชื่อ alias ของ 32i หรือ V1.5 [ผู้ผลิต OpenKB](https://inex.co.th/home/product/openkb/) |
| KB32-FT — `kb32_ft_makerasia` | ผู้ผลิตระบุ ESP32 และ hardware compatibility layer | Maker Asia; บอร์ดอนุพันธ์ | `needs_validation`; TFT จำลอง matrix ไม่ใช่ HT16K33 จริง [เอกสารผู้ผลิต](https://kb32ft.makerasia.com/) |

V1.1/V1.2/V1.4 และ 32i ไม่ได้ถูกยกระดับจากชื่อครอบครัวหรือจากบอร์ดที่หน้าตาใกล้กัน เมื่อมี schematic เจาะจงและ PCB ตรงกันจึงค่อยเติม MCU/pin map ที่ยังเว้นไว้ใน catalog

## เปรียบเทียบอุปกรณ์และทางพัฒนา

| รุ่น / กลุ่ม | Display / เสียง / ปุ่ม | เซนเซอร์ / RTC / ส่วนเพิ่ม | USB / การพัฒนาที่มีหลักฐาน | สถานะ firmware ในแอปและช่องว่าง |
|---|---|---|---|---|
| V2016 | ไม่ขยายจากรุ่นใหม่ | ฮาร์ดแวร์แต่ละ revision ยังไม่ตรวจครบ | ประวัติระบุ smartphone-app programming; ESP8266 | ไม่มี ESP32 ESP-IDF recipe; ต้องศึกษาสายเครื่องมือเดิมต่างหาก |
| V1.0 | ไม่ยืนยัน | ไม่ยืนยัน | ชื่อ hardware ยังไม่ชัด | เก็บเฉพาะประเด็นประวัติศาสตร์ |
| V1.1 | ไม่คัดลอกจาก V1.3 | ไม่คัดลอกจาก V1.3 | Cypress driver จากหน้า Download | ต้องได้ schematic/PCB และ build ของรุ่นนี้ |
| V1.2 | ไม่สรุปว่าเหมือน V1.1 ทุกจุด | ยังไม่ยืนยันความต่าง revision | Cypress driver จากหน้า Download | ยังไม่มี profile ที่ผ่านตรวจ |
| V1.3 | HT16K33 + matrix แดง 16×8, buzzer, S1/S2 | LM73, LDR, MCP79411 RTC; ไม่มีหลักฐานเพิ่ม IMU/RGB ในวงจรนี้ | FT231XQ; โครงการใช้ ESP-IDF 5.5 บน target esp32 | มี pilot; matrix ใช้ font ตัวเลขที่เพิ่มใหม่ ไม่อ้าง font/scroll เดิมหรือ orientation บนบอร์ดจริง |
| V1.4 | ยังไม่ยืนยันรายละเอียด LED status/bridge จากวงจรเฉพาะรุ่น | ยังไม่ยืนยันรายชิป | ชื่อรุ่นปรากฏในคำอธิบายความเข้ากันได้ของ V1.6 | ยังไม่มี profile; ไม่รับผล build ของ V1.6 แทน |
| V1.5 ไม่ระบุ revision | วงจรที่ให้มี HT16K33, buzzer และสองปุ่ม | LM73, MCP79411, LDR, analog input paths และ servo connectors | แบบวงจรที่ให้ใช้ FT231XQ; MCU ESP32 | รู้ design บางส่วน แต่ต้องเทียบ PCB/revision ก่อนเลือก profile |
| Rev3.1 profile เดิม | S1/buzzer เป็นขอบเขตเดิม | LM73 เป็นขอบเขตเดิม | มี software build ของ recipe เดิม; ไม่ใช่ physical pass | คง profile เดิม; matrix recipe ใหม่จำกัด V1.3 เพราะช่องว่าง revision |
| Rev3.1G | ไม่กำหนดจากชื่อ G | ไม่กำหนดจากตารางที่ขัดกัน | ยังไม่มี primary source เฉพาะ revision | ไม่มี native profile |
| INEX 32i / V1.5i | ยังต้องเทียบวงจรสีเขียว | ผู้ผลิตกล่าวถึง RTC/EEPROM ที่ปรับปรุงในตระกูลนี้ | USB-C; ชื่อปรากฏในรายการเครื่องมือ INEX | alias ที่ยืนยันได้ไม่แทนการยืนยัน BOM/พินทั้งหมด |
| INEX 32iA | matrix 16×8, piezo, 2 ปุ่ม | หน้าปัจจุบันระบุ **LDR** และปลั๊กอิน **iA-MC3479**, I2C temperature, battery-backed RTC | USB-C; ผู้ผลิตระบุ KidBright IDE, KB-IDE, Arduino IDE, microBlock | ต้องยืนยัน accelerometer/address/PCB; ตัวอย่าง MC3479 ใน KB ไม่ใช่ profile ทั้งบอร์ด |
| INEX 32iP Rev.2021.04.01 | HT16K33, speaker, S1/S2 ในวงจร | **Phototransistor**, **LM73-1**, **MCP79411**, servo 2 ช่อง | USB-C, **FT231XS** ในวงจร; ผู้ผลิตมีตัวอย่าง KidBright/microBlock | มี pin evidence เฉพาะ design นี้ แต่ยังไม่มี native profile หรือ physical comparison |
| Gravitech V1.6 | matrix 16×8, buzzer, 2 ปุ่ม | **MPU-6050**, **Gerora RGB 6 ดวง**, temperature/light/RTC | หน้าผลิตภัณฑ์ระบุ **Micro USB** และมี plugins | ต้องตรวจ S2, RGB pin/order, IMU/bus และ native build; ไม่ใช้ MPU/servo/RGB จากคนละ variant |
| Gravitech Lite V1.1 | matrix แดง 8×8 สองชุด, buzzer, 2 ปุ่ม | **LSM6DS3TR-C**, **Gerora RGB 4 ดวง**, light/temperature; รายละเอียด RTC ยังไม่ยืนยัน | **USB-C**, ผู้ผลิตระบุ KidBright IDE และ Arduino IDE | ต้องทำ pin/BOM/address profile ใหม่; ไม่สลับ driver กับ V1.6 |
| INEX OpenKB | matrix 16×8, piezo, 2 ปุ่ม | LDR, I2C temperature, RTC สำรองด้วย **supercapacitor**, JST I/O มากขึ้น | **Micro USB**; ผู้ผลิตระบุ KidBright IDE + plugin และ KB-IDE | ต้องตรวจ complete pin ownership; ไม่คัดลอกการสำรอง RTC แบบถ่านของ 32iP |
| Maker Asia KB32-FT | **ST7735 TFT สี 0.96 นิ้ว 80×160**, จำลอง matrix; buzzer | มี motion/light/temperature/RTC ตามเอกสาร; pin/chip ยังต้องตรวจ | ผู้ผลิตระบุ KidBright IDE, KB-IDE, Arduino, microBlock, MicroPython, **ESP-IDF** | ต้องพอร์ต compatibility layer ของ TFT และทรัพยากรจริง; การรองรับ ESP-IDF ของผู้ผลิตไม่ใช่ผล build จากแอปนี้ |

แหล่งของตารางอุปกรณ์: [วงจร V1.3](https://www.kid-bright.org/kidbright/download/1940/), [แบบ V1.5 ที่ให้มา](../knowledge_base/sensor_examples/PCB_KIDBRIGHT32_V1_5_Rev3_1.pdf), [INEX iA](https://inex.co.th/home/product/kidbright32ia/), [INEX iP](https://inex.co.th/home/product/kidbright32ip/), [วงจร iP](https://doc.inex.co.th/wp-content/uploads/2023/02/020966_0636_KidBrigh3.png), [V1.6](https://www.kidbright.io/product-page/kidbright32-v1-6), [Lite](https://www.kidbright.io/product-page/kidbright-lite-v1-1), [OpenKB](https://inex.co.th/home/product/openkb/), [KB32-FT](https://kb32ft.makerasia.com/). แต่ละค่าที่ catalog เก็บมี `source_ids` ผูกแยก ไม่ใช้การอ้างตารางรวมแทนหลักฐานของพิน

## วงจร V1.3 และ V1.5 ที่ผู้ใช้มี

| เรื่อง | V1.3 ที่ระบุ revision ชัด | แบบ V1.5 ที่ให้มา | ข้อจำกัด |
|---|---|---|---|
| Title block | SCH-KIDBRIGHT32, V1.3, 22/10/2018 | KidBright32 V1.5, REV3, 07 Jan 2020 | ชื่อไฟล์ฝั่ง V1.5 เป็น `Rev3_1` แต่ตัวแบบเขียน `REV3` |
| MCU | ESP-WROOM-32 | ESP-WROOM-32 | ยืนยันตระกูล MCU ไม่รับรอง partition/flash size ทุกเครื่อง |
| Matrix | HT16K33 ที่ SDA0/SCL0 | HT16K33 ที่ SDA0/SCL0 | SDA0=GPIO21, SCL0=GPIO22 ในแบบทั้งสอง |
| Temperature | LM73 ที่ SDA1/SCL1 | LM73 ที่ SDA1/SCL1 | SDA1=GPIO4, SCL1=GPIO5; ตรวจ ordering code/address จริง |
| RTC | MCP79411-I/SN ที่ SDA1/SCL1 | MCP79411-I/SN ที่ SDA1/SCL1 | **RTC ไม่ได้อยู่ display bus** ในสองแบบนี้ |
| USB-UART | FT231XQ | FT231XQ ในแบบที่ให้ | ไม่ควรเขียน FT232RL/CP2102 เป็นค่าตายตัวของชื่อรุ่น |
| Input/servo | วงจร input ของรุ่นนี้และ output drivers ต้องใช้ให้ตรง | แบบ V1.5 เพิ่มเส้นทาง analog input และ servo connectors | ห้ามนำ input/output polarity หรือ servo supply ของแบบหนึ่งแทนอีกแบบ |

ตำแหน่ง I2C ใน schematic เป็น **ชื่อ net ทางฮาร์ดแวร์** ส่วน `I2C_NUM_0`/`I2C_NUM_1` คือ controller instance ที่ firmware เลือก ใน runtime อ้างอิง display ถูกจัดอยู่ bus 0 และ Chain/LM73/RTC อยู่ bus 1 การเขียน ESP-IDF ใหม่ต้องรักษาพิน/อุปกรณ์จริงและใช้ driver family ที่เข้ากัน ไม่เลือก bus จากชื่อเซนเซอร์เพียงอย่างเดียว

Header ใน Desktop source snapshot ระบุ `HT16K33_ONBOARD_ADDR=0x70`, LM73 candidates `0x49`/`0x4D`, และ `MCP7940N_ONBOARD_ADDR=0x6F` เป็น **ค่าที่ซอฟต์แวร์อ้างอิงคาดหวัง** ไม่แทนการยืนยันชิปทุกล็อต รุ่นหลักสูตรหรือการเรียกคลาส MCP7940N จึงไม่ควรใช้แทน BOM ที่วงจรเขียน MCP79411

การที่ผู้ใช้มี schematic V1.3/V1.5 ยังไม่ใช่การระบุว่ามีบอร์ดจริงรุ่นใดเชื่อมต่ออยู่ จึงไม่มี physical validation หรือการ flash เพิ่มจากรายงานนี้

## ข้อขัดแย้งที่แก้ได้จากแหล่งผู้ผลิต และข้อที่ยังเปิดอยู่

1. **iA ไม่ควรถูกเหมารวมว่าใช้ KXTJ3 และ phototransistor:** หน้าผู้ผลิตปัจจุบันเขียน LDR และปลั๊กอิน iA-MC3479 หากพบ PCB เก่าที่ใช้ชิปอื่นต้องเพิ่ม revision พร้อมหลักฐาน ไม่แก้ชื่อไดรเวอร์โดยเดา [INEX iA](https://inex.co.th/home/product/kidbright32ia/)
2. **iP: LM75 ในบทความกับ LM73-1 ในวงจรไม่ตรงกัน:** schematic ระบุ Rev.2021.04.01 และ LM73-1 ชัด จึงให้วงจรที่ตรง PCB เป็นหลักฐานระดับพิน ส่วน address/ordering code ต้องตรวจจริง [บทความ](https://doc.inex.co.th/kidbright32ip-with-microblock-ide-ep1-intro/), [วงจร](https://doc.inex.co.th/wp-content/uploads/2023/02/020966_0636_KidBrigh3.png)
3. **iP: S2 กับ servo 2 เป็นคนละสัญญาณ:** วงจรเดียวกันแสดง S2=GPIO14, S1=GPIO16, SV1=GPIO15, SV2=GPIO17 การเอา GPIO17 ไปใส่ S2 เพราะอ่านชื่อ SV2 ผิดจะเปลี่ยนพฤติกรรมของโปรแกรม [วงจร iP](https://doc.inex.co.th/wp-content/uploads/2023/02/020966_0636_KidBrigh3.png)
4. **V1.6 ไม่ควรถูกกำหนด USB-C ทุกตัว:** หน้าผลิตภัณฑ์หลักระบุ Micro USB แต่คำอธิบายสายของ Beginner Pack เปิดความเป็นไปได้ของ USB-C บางรุ่น ยังไม่มี PCB identifier สำหรับแยกเป็นอีก profile [ตัวบอร์ด](https://www.kidbright.io/product-page/kidbright32-v1-6), [ชุดอุปกรณ์](https://www.kidbright.io/product-page/kidbright32-v1-6-beginner-pack)
5. **V1.5/Rev3.1/Rev3.1G ไม่ใช่ชื่อที่แทนกันได้ทันที:** แบบที่ให้มี revision identity gap และยังไม่พบ primary source เจาะจง 3.1G จึงคงช่องว่างไว้แทนการสร้างตาราง GPIO ที่ดูครบแต่ไม่มีหลักฐาน
6. **ชื่อ i, iA, iP ต้องแยกกัน:** คู่ชื่อ `32i / V1.5i` มีหลักฐานจาก INEX แต่ `V1.5 iA` ไม่ถูกนับเป็นผลิตภัณฑ์ใหม่เพราะ KB เก่าใส่อีกแถว ต้องตรวจชื่อผู้ผลิตและ silkscreen [INEX Software Download](https://inex.co.th/home/software-download/)
7. **Lite V1.1 เป็นรุ่นที่ KB เก่าไม่ครอบคลุม:** IMU คนละชนิดและ RGB คนละจำนวนกับ V1.6 จึงไม่ใช้ driver bundle ร่วมกันอัตโนมัติ [Gravitech Lite](https://www.kidbright.io/product-page/kidbright-lite-v1-1)
8. **KB32-FT ต้องมี TFT compatibility layer:** การส่ง byte ไป address 0x70 ตาม HT16K33 ไม่ทดแทน ST7735 ของอนุพันธ์นี้ [Maker Asia](https://kb32ft.makerasia.com/)

## สิ่งที่นำไปใช้กับแอปได้ในงานนี้

Catalog ใช้บอก AI ว่ามีรุ่นใดที่เปรียบเทียบไว้ หลักฐานอะไรยืนยันแล้ว และประเด็นใดต้องตรวจต่อ **รายชื่อที่เปรียบเทียบไม่ใช่รายชื่อ recipe ที่สร้างได้** ตัวเลือก recipe ต้องมาจาก `kidbright_reference` และตรวจ `reference_profile_id` ของตัวเอง ไม่ย้ายจาก unknown board ไป V1.3 ตามชื่อโครงการเก่า

ในซอร์สระหว่างงานนี้มีตัวอย่าง `button_buzzer`, `temperature_serial`, `music_scale` และ `matrix_counter` โดย matrix จำกัด V1.3 ข้อจำกัดของแต่ละงานและผล build ล่าสุดให้ตรวจจาก [compatibility README](../knowledge_base/compatibility/README.md) กับหลักฐาน validation ของงานนั้น ไม่ใช้การมีไฟล์ `.c` ใน catalog เป็นผลทดสอบ

การเพิ่มรุ่นถัดไปต้องมีข้อมูลอย่างน้อย: PCB/revision ที่ตรงกับเอกสาร, MCU/flash/bootloader, pin/bus/address map รายอุปกรณ์, resource conflicts, dependencies และ SDK ที่ build ได้ แล้วจึงเพิ่ม recipe tests, firmware builds และผลเทียบกับบอร์ดจริง รายการที่ยังเว้น unknown ไม่ถูกแทนด้วย placeholder sensor หรือค่า GPIO ของบอร์ดใกล้เคียง

Native ESP-IDF firmware อาจเขียนทับ firmware ที่เครื่องมืออื่นติดตั้งไว้ การที่ผู้ผลิตระบุว่าใช้ Arduino/MicroPython/KidBright IDE ได้เป็นเส้นทางพัฒนาคนละแบบ ไม่ได้หมายถึง application binary, model file หรือ API ใช้แทนกันได้ รายงานนี้ไม่สั่ง erase/flash/recovery ใด ๆ

## ขอบเขตที่ไม่ถูกนับเป็นบอร์ดเพิ่ม

Beginner/Education/Book sets, JR-BOT, Rover และสถานี UtuNoi เป็นชุดอุปกรณ์หรือระบบที่ต้องดู base board กับวงจรต่อพ่วง ไม่ใช่ PCB revision ใหม่เพียงเพราะชื่อชุดต่างกัน ส่วน iKB-1/iKB-1Z, KB Chain, motor extensions, minibike/Formula Kid wiring และ servo supply ต้องมี resource/protocol contract ของตนเอง ไม่ถูกเหมารวมใน `target: esp32`

งานนี้ไม่ได้สำรวจ community derivatives ทุกตัวในโลก รุ่นที่ผู้ผลิตไม่ได้เผยแพร่รหัส PCB หรือ board ที่ยังไม่มี source ที่ตรวจได้ต้องคงเป็น unknown การเปรียบเทียบจึงครบตาม **รายชื่อใน KB เดิมที่ถูกขอให้ตรวจ พร้อมรุ่นผู้ผลิตเพิ่มเติมที่ระบุใน catalog** ไม่ใช่การรับรอง hardware universe ทั้งหมด

## แหล่งหลักและการตรวจซ้ำ

| Source ID | หลักฐาน / วิธีใช้ |
|---|---|
| `nectec-history-2018` | [ประวัติ NECTEC](https://www.nectec.or.th/en/about/news/kidbright-2018project.html): V2016/ESP8266 |
| `nectec-platform-brochure` | [เอกสารแพลตฟอร์ม](https://www.kid-bright.org/download/9270/): 2016 → V1.3 → V1.5 และแนวทางอุตสาหกรรม |
| `nectec-download` | [หน้า Download](https://www.kid-bright.org/download-kidbright/): ชื่อรุ่น hardware ที่ใช้ Cypress/FTDI ไม่สับสนกับเวอร์ชัน IDE |
| `nectec-v13-schematic` | [วงจร V1.3 หน้า 1](https://www.kid-bright.org/kidbright/download/1940/); local PDF hash ใน catalog |
| `provided-v15-design` | [วงจร V1.5 ที่ให้มา](../knowledge_base/sensor_examples/PCB_KIDBRIGHT32_V1_5_Rev3_1.pdf); ชื่อไฟล์กับ title revision ต่างกัน |
| `desktop-reference-header` | [รายการ source/hash ของ archive](research/kidbright-2026-10-15-evidence.json); `kidbright32.h` เป็นค่าซอฟต์แวร์ ไม่ใช่หลักฐานทดสอบ PCB |
| `existing-pilot` | [ผล software validation เดิม](research/kidbright-pilot-2026-10-16-validation.json); ไม่ใช่ physical certification |
| `historical-kb-labels` | [audit snapshot วันที่ 16 ตุลาคม](research/board-rule-audit-2026-10-16.json) ใช้ระบุข้ออ้างเก่าที่ต้องตรวจ ไม่ให้ authority แก่พินที่ขัดกันหรืออ้าง catalog กลับเข้าหาตัวเอง |
| `inex-software` | [รายการซอฟต์แวร์ INEX](https://inex.co.th/home/software-download/): คู่ชื่อ 32i/V1.5i |
| `inex-32ia-product` | [ผลิตภัณฑ์ 32iA](https://inex.co.th/home/product/kidbright32ia/) |
| `inex-32ip-product` | [ผลิตภัณฑ์ 32iP](https://inex.co.th/home/product/kidbright32ip/) |
| `inex-32ip-intro` | [บทความแนะนำ 32iP](https://doc.inex.co.th/kidbright32ip-with-microblock-ide-ep1-intro/); มี prose/schematic discrepancy |
| `inex-32ip-schematic` | [ภาพวงจร 32iP Rev.2021.04.01](https://doc.inex.co.th/wp-content/uploads/2023/02/020966_0636_KidBrigh3.png); ตรวจภาพโดยตรงและบันทึก SHA-256 |
| `provided-inex-user-evidence` | [ผล Hello/Heartbeat ที่ผู้ใช้รายงานและข้อมูล iA/iP](research/kidbright-inex-user-evidence-2026-10-17.json); มี source snapshot/hash แต่ไม่มี build log/serial trace หรือผลอุปกรณ์ต่อพ่วง |
| `gravitech-v16-product` / `gravitech-v16-kit` | [V1.6](https://www.kidbright.io/product-page/kidbright32-v1-6) / [Beginner Pack](https://www.kidbright.io/product-page/kidbright32-v1-6-beginner-pack) |
| `gravitech-lite11-product` | [Lite V1.1](https://www.kidbright.io/product-page/kidbright-lite-v1-1) |
| `inex-openkb-product` | [OpenKB](https://inex.co.th/home/product/openkb/) |
| `makerasia-kb32ft` | [KB32-FT](https://kb32ft.makerasia.com/) |

SHA-256 ของวงจร V1.3 ที่เก็บในโครงการ: `10c9ae1c637e2ee75aaa1a9335db941f0bbd6b9f7f73fcc7301e045d722beb15`

SHA-256 ของแบบ V1.5 ที่ให้มา: `d00d3a666f223856c4ddd2fbdf1151b5332097327a090c33d0a3043d755bda0d`

SHA-256 ของภาพวงจร INEX 32iP ที่อ่านในงานนี้: `5c2b28ef0f8523d25881ea4dc90a5cf1fb75f90bec98afa3590f6a2937460e8e`

URLs และคุณสมบัติเชิงพาณิชย์เป็น snapshot วันที่ตรวจ ผู้ผลิตอาจเปลี่ยนรุ่น BOM หรือเนื้อหา จึงต้องตรวจ revision ของของจริงอีกครั้งเมื่อนำไปเพิ่ม profile ไม่ใช้วันที่ crawl หรือจำนวนผลค้นหาเป็นหลักฐานพิน
