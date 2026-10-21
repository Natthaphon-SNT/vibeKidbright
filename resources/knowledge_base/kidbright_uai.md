# KidBright uAI และ KidBright uAI Plus

> **สถานะ: เอกสารอ้างอิงสำหรับอนาคต** ผู้ใช้กำหนดให้ทำ KidBright แบบ ESP32 ให้สมบูรณ์ก่อน แล้วจึงพัฒนา μAI/Plus ไม่มีการเปิด Build & Flash หรือเพิ่ม runtime/deployment ในรอบนี้
>
> ปรับข้อมูล 17 ตุลาคม 2026: **μAI รุ่น Allwinner V831** จากข้อมูล/ภาพผู้ใช้ เทียบ [ผลิตภัณฑ์ INEX](https://inex.co.th/home/product/kidbright-%CE%BCai/) และ [คู่มือแนะนำ](https://inex.co.th/store/manual/KBmAI_Leaflet240518-opt.pdf); **Plus รุ่น SG2002** จากไฟล์/ภาพใหม่ของผู้ใช้ เทียบ [INEX Plus](https://inex.co.th/home/product/kidbright-microai-plus/) และ static source ของ [μAI IDE](https://kidbright-mai.web.app/) คำแนะนำ training จากคู่มือเดิมยังต้องเทียบเวอร์ชันก่อนใช้

## กฎสำคัญสำหรับผู้ช่วย AI

- KidBright uAI รุ่นแรกและ KidBright uAI Plus เป็นคนละบอร์ด ห้ามเหมารวมสเปค
- เมื่อผู้ใช้ถามเกี่ยวกับ uAI/μAI โดยยังไม่ระบุรุ่น ให้ถามก่อนว่าใช้ **KidBright uAI รุ่นแรก** หรือ **KidBright uAI Plus**
- ถ้าผู้ใช้ไม่แน่ใจ ให้ดูตัวบอร์ด:
  - ตรวจชื่อบน PCB, revision และรหัสโมดูลประมวลผล แล้วเทียบคู่มือตรงรุ่น
  - **ห้ามใช้ “มีจอ/S1/S2 = Plus” หรือ “μAI รุ่นแรกไม่มีจอ”** เพราะ μAI รุ่น V831 ในหลักฐานนี้มีจอ IPS, S1/S2, กล้อง, ไมโครโฟน และ accelerometer อยู่แล้ว
- เมื่อยืนยันรุ่นแล้ว ให้ใช้รุ่นนั้นต่อเนื่องตลอดบทสนทนา ไม่ต้องถามซ้ำ
- หมายเลข 1–23 ในภาพเป็นหมายเลขอุปกรณ์ ไม่ใช่ GPIO ของ Linux หรือ ESP32 ห้ามแปลงเป็น pin map เอง

## KidBright μAI รุ่น V831: ข้อมูลที่ผู้ใช้ให้

ชื่อ `mai`, `μAI`, `µAI` และ `uAI` อาจใช้เรียกในข้อความได้ แต่ชื่อย่อไม่ได้ยืนยันว่าเป็น Plus ให้ผูกข้อมูลกับโมดูล V831/PCB จริง ข้อมูลต่อไปนี้เป็นสเปกเอกสาร ยังไม่มีผลทดสอบบอร์ดจริงหรือโมเดลในโครงการนี้

| ส่วน | คุณสมบัติตามข้อมูลผู้ใช้/ผู้ผลิต |
|---|---|
| หน่วยประมวลผล | Allwinner V831, ARM Cortex-A7 แกนเดียว 800 MHz |
| หน่วยความจำ | DDR2 RAM 64 MB |
| ระบบปฏิบัติการ/ที่เก็บ | Linux บน microSD; เอกสารระบุการ์ด 32 GB หรือมากกว่า ยังไม่ยืนยัน image, partition หรือ filesystem ในงานนี้ |
| กล้อง | OV2685 หรือเทียบเท่า, 2 ล้านพิกเซล |
| จอ | IPS 1.3 นิ้ว, 240×240 พิกเซล |
| เสียงเข้า | ไมโครโฟน MEMS พร้อมวงจรขยายสัญญาณ |
| เสียงออก | วงจรขยาย 3 W, ลำโพง 8Ω ขนาด 1 นิ้ว และจุดต่อลำโพง |
| การเคลื่อนไหว | accelerometer 3 แกน MSA301 หรือเทียบเท่า; ไม่มีหลักฐานในชุดนี้ว่ามี gyroscope |
| เครือข่าย | RTL8189FTV WiFi พร้อมสายอากาศ PCB |
| เวลา | RTC พร้อมแบตเตอรี่สำรอง; ตารางระบุ CR1220 |
| ปุ่ม | S1, S2 และ RESET |
| USB | USB-C 2 ช่อง; รายการสเปกเรียก USB-OTG/USB-DEBUG ส่วนตารางอุปกรณ์เรียกช่องที่สอง USB-UART ต้องยืนยันหน้าที่จริงจากวงจร/คู่มือเชื่อมต่อ |
| Digital I/O | PH2.0 แบบแนวนอน 3 ขา รวม 7 ช่อง; ยังไม่มี GPIO number/logic-level map ที่ยืนยันในงานนี้ |
| I2C | KB-CHAIN 5 ขา และ GROVE 4 ขา; ชื่อ/จำนวน connector ไม่ยืนยันว่ามี controller อิสระกี่ตัว |
| UART | จุดต่อ UART พร้อมไฟสถานะ; ไม่เหมารวมกับพอร์ต USB สำหรับพัฒนา |
| ไฟเลี้ยง | 5 V ผ่าน USB-OTG หรือแจ็คอะแดปเตอร์ มีสวิตช์เลือก; 200 mA เป็นค่าที่เอกสารระบุ ไม่ใช่ผลวัด peak current รวมอุปกรณ์ภายนอก |

## หมายเลขอุปกรณ์ในภาพ μAI V831

| หมายเลข | ส่วนประกอบและบทบาทตามภาพ/ตารางผู้ใช้ |
|---|---|
| 1 | แจ็คไฟเลี้ยง 5 V |
| 2 | สวิตช์เลือกแหล่งไฟ USB-OTG หรือ DC jack ตามทิศทางในคู่มือ |
| 3 | USB-OTG: ไฟเลี้ยงและการสื่อสารกับคอมพิวเตอร์สำหรับส่งโปรแกรม |
| 4 | USB-UART ตามตาราง / USB-DEBUG ตามสเปก; บทบาทและโปรโตคอลยังต้องยืนยัน |
| 5 | จอ IPS 1.3 นิ้ว 240×240 |
| 6 | ตัวตรวจจับความเร่ง |
| 7 | กลุ่มพอร์ต digital I/O |
| 8 | จุดต่อ UART |
| 9 | กลุ่มพอร์ต digital I/O |
| 10 | ไมโครโฟน MEMS |
| 11 | ปุ่ม S1 |
| 12 | ปุ่ม S2 |
| 13 | วงจรขยายเสียง |
| 14 | จุดต่อลำโพง |
| 15 | RESET |
| 16 | I2C GROVE 4 ขา |
| 17 | I2C KB-CHAIN 5 ขา |
| 18 | กล้อง 2 ล้านพิกเซล |
| 19 | RTC |
| 20 | ซ็อกเก็ตแบตเตอรี่ CR1220 สำหรับ RTC |
| 21 | ซ็อกเก็ต microSD สำหรับ Linux image |
| 22 | โมดูล WiFi |
| 23 | โมดูลประมวลผล Allwinner V831 |

## แนวทางการเรียนรู้และขอบเขตการพัฒนาในอนาคต

ข้อมูลผู้ใช้อธิบายการเรียนรู้แบบเก็บข้อมูล → ติดป้ายกำกับ → สร้างโมเดล → ทดสอบ/ประยุกต์ใช้ โดย inference ทำบนอุปกรณ์แบบ Edge AI และใช้ WiFi เชื่อมบริการภายนอกได้ การใช้ LLM ผ่าน prompt ใน vibeKidbright กับการฝึก classifier บนแพลตฟอร์มนี้เป็นคนละขั้นตอน ไม่ถือว่ามี LLM แล้วเท่ากับมี training/deployment workflow ของ μAI

ภาพ IDE ที่ส่งมี Basic, Display/Image, Loops, Logic, Math, GPIO I/O, Text, List, Variables, Functions และปุ่ม AI/Plugin เป็นหลักฐานหน้าจออ้างอิง ไม่ได้ตรวจ generator ทุกบล็อกหรือพิสูจน์ว่าแอปนี้รองรับหมวดเหล่านั้นแล้ว ไม่รับรอง latency/security/ความแม่นยำจากคำอธิบาย Edge AI เพียงอย่างเดียว

ก่อนเริ่ม implementation ในระยะถัดไป ต้องยืนยัน PCB/BOM, Linux image และเวอร์ชัน, toolchain/ABI/runtime, USB transport, Linux device nodes/GPIO และสิทธิ์เข้าถึง, camera/audio/display/sensor API, model format/preprocessing/label order ตลอดจนเส้นทาง upload/recovery แยกจาก ESP32 ไม่ส่ง V831 เข้า `idf.py`/esptool หรือคัดลอก GPIO ของ KidBright32 มาใช้

## IDE และการเลือกบอร์ด

- URL ที่ผู้ใช้และเอกสาร μAI V831 ระบุ: https://mai.kidbright.app.meca.in.th/
- URL ที่ผู้ใช้ระบุสำหรับ Plus: https://kidbright-mai.web.app อ่าน JavaScript รอบนี้พบ config ของทั้ง `kidbright-mai` และ `kidbright-mai-plus` แต่ยังไม่ทดสอบ upload กับบอร์ดจริง
- https://microaithai.com/ide ในไฟล์ใหม่และหน้า INEX ส่ง HTML ที่ redirect ด้วย JavaScript ไปเว็บข้างต้น ส่วน direct GET ของ URL meca รอบล่าสุดได้404 ไม่สรุปว่าบอร์ดเดิมเลิกใช้งาน
- เมื่อเริ่มงานจริงและ IDE มี **Select a Board** ให้เลือกชื่อรุ่นตรงฮาร์ดแวร์ ไม่ใช้การมี/ไม่มีจอเป็นเกณฑ์ตัดสิน
- ตรวจ HTML/JavaScript เป็นข้อมูลเท่านั้น ไม่รัน scripts หรือเชื่อมต่อ IP บอร์ด หลักฐาน snapshot/hash อยู่ใน `docs/research/mai-integration-2026-10-17/source-evidence.json` ของ repository

## μAI Plus รุ่น SG2002: ไฟล์และภาพใหม่ของผู้ใช้

| ส่วน | คุณสมบัติที่ระบุ |
|---|---|
| Processor | SG2002; ผู้ผลิตระบุ NPU1TOPS พร้อม ARM A53/C906/8051 ต้องตรวจ Linux image/ABI จริง ไม่เลือก compiler จากจำนวนแกนในข้อความเพียงอย่างเดียว |
| RAM | DDR3 256MB |
| จอ | capacitive touch2.4นิ้ว; ภาพผู้ใช้ระบุ640×480 ยังต้องตรวจ panel/driver/framebuffer จริง |
| กล้อง | OV2685 หรือเทียบเท่า, 2ล้านพิกเซล |
| ไมโครโฟน | MEMS LMA2718T421 พร้อมวงจรขยาย |
| เสียงออก | AW8010AFCR, กำลัง3W, ลำโพง8Ω1นิ้ว |
| IMU | QMI8658A accelerometer+gyroscope6แกน ต่างจาก μAI V831 ที่ระบุ MSA301-equivalent accelerometer |
| RTC | BM8653 พร้อมแบตเตอรี่สำรอง |
| WiFi | WiFi6 ย่าน2.4/5GHz พร้อม PCB antenna ตามเอกสาร |
| ระบบ/ที่เก็บ | Linux บน microSD32GBหรือมากกว่าตามเอกสาร; image/runtime/version ยังไม่ทดสอบในงานนี้ |
| USB | USB-C OTG และ USB-UART คนละช่อง |
| I/O | PH2.0แบบ3ขา7ช่อง; ภาพระบุ IO2–IO8 ต้องยืนยัน logical-port map ไม่ใช้เป็น SoC GPIO โดยตรง |
| I2C/UART | KB-CHAIN5ขา, GROVE4ขา และจุดต่อ UART; จำนวน connector ไม่ใช่หลักฐานจำนวน controller |
| ปุ่ม | S1/S2, BOOT และ RESET ตามภาพ |
| ไฟเลี้ยง | 5V, เอกสารระบุ200mA มีสวิตช์เลือก OTG/DC; ยังไม่มี peak-load measurement |

ข้อความ hardware ระบุ MobileNetV2/YOLOv5/YOLOv8 แต่ model options ใน IDE snapshot ที่ตรวจสำหรับ Plus มี yolo11n/yolo11s ต้องยึดรายการที่ converter/runtime รุ่นนั้นผ่านจริง ไม่รับรองโมเดลทั้งหมดจากคำโฆษณาหรือชื่อคล้ายกัน

## การเชื่อมต่อและโมเดลที่พบใน IDE snapshot

| รุ่นใน client | Protocol | โมเดลที่ client เลือก |
|---|---|---|
| `kidbright-mai` / V831 reference | `web-adb`, ใช้ WebUSB ADB transport ในเว็บ | ภาพ/วัตถุ `ncnn-int8` `.bin/.param`; เสียง `numpy-fp32` `.npz` |
| `kidbright-mai-plus` / SG2002 reference | `websocket-shell`, เปิด `wss://10.155.55.1:5050` จาก `wsShell` | `cvimodel` `.cvimodel/.mud` |

นี่เป็นผลอ่าน client ไม่ใช่ protocol ที่ทดสอบผ่านบน hardware แล้ว ไม่เชื่อมต่อ IP นี้หรือเรียก shell เพียงเพราะเป็นค่าตัวอย่าง VID/PID ของทั้งสอง config เป็น6353/2เหมือนกัน จึงห้ามใช้ USB numbersอย่างเดียวตัดสินรุ่น

Plus มี envelope `__SYSTEM__:` + JSON, probe `version`/features, chunked file upload และคำสั่ง directory/stat/download ที่เริ่มทำ adapter ได้ แต่ต้องทดสอบ timeout, response matching และ firmware compatibility ก่อนใช้จริง การ connect ของ reference client อาจ sync scripts และบางรายการต้อง reboot จึงแยกการตรวจบอร์ดกับการเตรียม/อัปเดต runtime ให้ผู้ใช้เห็นผลข้างเคียง

เส้นทาง code generation เป็น Python พร้อม libraries ของบอร์ด ไม่ใช่ ESP-IDF C; model metadata/preprocessing/labels/runtimeต้องตรง target มีชื่อ libraryเก่าในชุด Plus ไม่ได้พิสูจน์ว่าชิปบนบอร์ดเป็นรุ่นเดียวกับชื่อไฟล์

ใน vibeKidbright ปัจจุบันมีเพียงตัวเลือก `miuaiplus` ที่ใช้ ADB/logcat monitor และ Build/Flashยังถูกปิด ไม่ถือว่ามี upload/run/model deployment แล้ว แผนรวมเป็น UIเดียวกับ native adaptersแยก3family อยู่ใน `docs/VIBEKIDBRIGHT_UNIFIED_BOARD_ARCHITECTURE_2026-10-17.md` เป็นแบบพัฒนาระยะถัดไป ไม่ใช่ implementationที่เปิดใช้แล้ว

## ข้อมูลการใช้งาน Plus จากคู่มือเดิม

- จอแสดงผลในตัว
- ปุ่ม S1 และ S2
- accelerometer/gyroscope 3 แกน และบล็อกอ่านค่า X/Y/Z
- กล้องและบล็อกแสดงภาพจากกล้อง
- พอร์ต IO2–IO8
- I2C จำนวน 2 ชุด
- เชื่อมต่อคอมพิวเตอร์ด้วย USB-C และสื่อสารแบบ RNDIS

รายการส่วนนี้เป็นข้อมูล Plus ที่บันทึกไว้ ไม่ใช่หลักฐานว่าความสามารถเหล่านี้มีเฉพาะ Plus เพราะ μAI V831 ก็มีจอ S1/S2 กล้อง และ accelerometer เช่นกัน ส่วน gyroscope, IO2–IO8, controller count และ API ต้องตรวจตามรุ่น/คู่มือจริง ไม่ถ่ายค่าข้ามรุ่น

## แก้ปัญหาเชื่อมต่อ uAI Plus ไม่ได้

ทำตามลำดับต่อไปนี้เฉพาะ uAI Plus ที่เชื่อมต่อแบบ RNDIS ผ่าน USB-C:

1. เปิด **Device Manager > Network adapters** แล้วตรวจว่ามี **Remote NDIS based Internet Sharing Device**
2. เปิด https://10.155.55.1:5050 โดยตรง ยอมรับ security exception ของเบราว์เซอร์ แล้วกลับไปที่ IDE และกด **Connect** ใหม่
3. หากยังไม่สำเร็จ ให้กด **Reset** บนบอร์ด รอให้ reboot เสร็จ แล้วเริ่มขั้นตอนเชื่อมต่อใหม่ตั้งแต่ต้น

อย่านำขั้นตอน RNDIS นี้ไปใช้กับ uAI รุ่นแรก เว้นแต่มีเอกสารยืนยันเพิ่มเติม

## อิมเมจ OS และ Runtime จากไฟล์ที่ผู้ใช้ให้เพิ่ม

คู่มือ PDF ที่ผู้ใช้ให้เพิ่มวันที่17ตุลาคมมี89หน้า ยืนยัน RNDIS/HTTPS5050, การติดตั้งแอปให้เริ่มเมื่อboot, บทเรียน GPIO/Sensors และขั้นตอน Colab คู่มือมีตัวอย่าง20/100ภาพ แต่ไม่ได้พิสูจน์ความแม่นยำหรือขั้นต่ำสากล และระบุ Voice Classification โดยมีรายละเอียดน้อยกว่าภาพ/วัตถุ

- ZIP ชื่อ JaxBot, NanoPi, RSP และ Plus แต่ละไฟล์มี `.img` เพียง1ไฟล์ ไม่ใช่โครงสร้าง source IDE โดยตรง ภายใน filesystem ยังไม่ได้ตรวจครบ จึงไม่บอกว่าไม่มี source หรือมี runtime พร้อมแล้ว
- `kidbright-mAi-plus-2026-06-09-v4.12.5.zip` เป็น candidate จากชื่อ/บริบท ยังไม่ boot-test ส่วน JaxBot/NanoPi/RSP ห้ามนำไปใช้แทน μAI หรือ Plus อัตโนมัติ ต้องแยกผลตรวจ filesystem ออกจากผลบนบอร์ดจริง
- ตรวจ RSP ภายในแล้วพบ Raspbian GNU/Linux10 (buster), executable ARM ELF32 และ PM2 startup ที่ระบุ Node.js v10.21.0 เรียก `kbclientNew/server/index.js` กับ Python `server/kb_2/app3.py` มี source Node/Express และ Python/Flask สำหรับ project/run/stop/training/inference แต่ยังไม่ทดสอบ service หรือฮาร์ดแวร์ ไม่ใช้ API นี้แทน V831 ADB หรือ Plus WSS โดยอัตโนมัติ รายละเอียดระดับไฟล์อยู่ใน `docs/research/mai-image-filesystems-2026-10-17.json`
- NanoPi ZIP ใช้ Deflate64 ซึ่งเพิ่มตัวอ่านได้แล้ว มี Linux2partitions; filesystem แรกเป็น Ubuntu18.04.2 LTS/Bionic และ Python3/bash/sh แบบ ARM ELF64 ต่างจาก RSP ARM32 พาร์ทิชันข้อมูลมี `kbclientNew` และ PM2 dump ระบุ Node `server/index.js` กับ Python3 `server/kb_2/app3.py` ข้อจำกัดแคช10GiBเดิมแก้แล้วโดยขยายเป็น32GiB ยังไม่ได้อ่านเนื้อหา entrypoints/package manifest ของ NanoPi หรือยืนยัน active boot root/ฮาร์ดแวร์จริง จึงไม่ยืนยัน API/model/wheel compatibility ข้ามรุ่น
- PDF หน้า75–89แยกลงOSบนmicroSDกับ Install Runtime ภาพหน้า87–88แสดง MaixPy4.12.5 และ Runtime1.24.0 ต้องเก็บ version คนละช่อง ไม่ใช้เลขในชื่อZIPแทน runtime ที่ตรวจจากบอร์ดจริง
- ขั้นตอน format/เขียนการ์ดในคู่มือเป็น reference ไม่ใช่คำสั่งผู้ใช้ให้ทำทันที การเตรียมOS/กู้คืนระบบเป็นงานครูแยกจากการส่งและรัน main.py
- ไม่คัดลอก ZIP/image ลง project main หรือ knowledge bundle แผน Image Library/Prepare Board อยู่ในเอกสารสถาปัตยกรรมและ `docs/KIDBRIGHT_MAI_SUPPLIED_FILES_AUDIT_2026-10-17.md` พร้อม hashes ใน JSON audit

## ซอร์ส GitHub ที่ตรวจเพิ่มวันที่17ตุลาคม2026

ตรวจ `KidBrightAI/kidbright_MAI_ide` commit `7b549397adf90f75d975091a787d5960001edf01` และ `kidbright_MAI_server` commit `e4fc663d89a361ff2448ab1d99b2cd1ede12b3db` แบบอ่าน/parseเท่านั้น หลักฐานใน `docs/research/mai-github-reference-2026-10-17.json` และรายงาน `docs/KIDBRIGHT_MAI_GITHUB_RESEARCH_2026-10-17.md` เป็นข้อมูลเพิ่มจาก snapshotเว็บ/อิมเมจ ไม่ได้ทดสอบอุปกรณ์หรือเปิด native deployment

- V831 default image=`resnet18`, detection=`slim_yolo_v2`, voice=`voice-cnn`; image/detectionใช้ NCNN `.bin/.param`, voiceใช้ NumPy FP32 `.npz` ส่วน Plus default image=`mobilenet-100`, detection=`yolo11n`/`yolo11s`, voice=`voice-cnn` ใช้ `.cvimodel/.mud` ต้องเลือกตาม control flow ของ format registry ไม่ใช้ commentเก่าที่กล่าวว่าเสียง Plusใช้ NumPy
- Training service แยกจาก board transport: `GET /ping`, multipart `POST /upload` fields=`project`,`project_id`, JSON `POST /train` field=`project`, SSE `/listen`, `/convert?project_id=...`; clientดาวน์โหลดไฟล์ผ่าน `/projects/<id>/output/<filename>` ไม่ใช้ `/upload` นี้ส่ง Pythonตรงเข้าบอร์ด
- มี contract gap ที่ต้องแก้ก่อนพอร์ต: clientหยุดเทรนด้วย `GET /terminate` แต่ serverมี `POST /terminate_training` ที่ใช้ thread.join()รอจบ ไม่ใช่ cancellation signal; stage/reporter/taskเป็นglobal ไม่ถือว่าพร้อมหลายงานในห้องเรียน
- Model upload เดิมข้ามเมื่อsizeเท่ากัน และ unknown-format fallbackเป็นNCNN; ใน adapterใหม่ต้องตรวจครบทุกไฟล์ด้วย hash และปฏิเสธ formatผิด/ไม่รู้จัก ไม่รายงานสำเร็จหลังจับ error แล้วเดินต่อ
- สแกนASTไฟล์ที่ board indexโหลดพบ V831 definitions51/generators50 และ Plus59/60 มี placeholder generator บางตัว แต่บางตัวถูกcommentออกจากtoolboxแล้ว ตัวเลขไม่รวมBlocklyมาตรฐาน/ทุกpluginและไม่ใช่ coverage ของ vibeKidbright
- UI locale ของต้นฉบับมี `th`,`en` เท่านั้น แยกจากภาษาคำตอบล่าสุดของ Vibe Coder; ไม่รับรองว่า UIหรือจอฮาร์ดแวร์รองรับทุกภาษาจากการมีLLM
- IDE packageระบุ1.2.5แต่READMEยังเรียก1.2.4; serviceVersion/OS/runtimeต้องตรวจแยก serverมีGPL-2.0 ส่วนIDEไม่พบLICENSE/package licenseในcommitนี้ ต้องตรวจสิทธิ์ก่อนนำซอร์สมาแจก
- คู่มือ Drive `KidBrightAIServerV2.pdf` metadataปี2023อธิบายColab/tunnelและไฟล์EdgeTPU/TFLite/TensorFlow.js เป็นworkflowเก่า ไม่ใช้เป็นmodel contractของV831/Plusปัจจุบัน และคำว่าServerver2ไม่ใช่หลักฐานว่าเป็นบอร์ดPlus

## Pipeline การเทรนโมเดล AI

ส่วนนี้เป็นขั้นตอนจากคู่มือ/ข้อมูลการใช้เดิม ไม่ได้ทดสอบ training service ปัจจุบัน จำนวนตัวอย่างและขั้นตอน Colab ต้องตรวจให้ตรง IDE/model adapter ที่เลือก ไม่ยกไปใช้เป็น universal runtime contract ของทั้งสองบอร์ด

ประเภทที่คู่มือระบุ: **Image Classification**, **Object Detection** และ **Voice Classification**

ลำดับงานหลัก:

1. **Capture** — เก็บตัวอย่าง
   - Image Classification: อย่างน้อย 20 ภาพต่อคลาส/วัตถุ
   - Object Detection: อย่างน้อย 100 ตัวอย่างต่อวัตถุ และควรหลากหลายด้านสภาพแวดล้อม แสง สี และเงาสะท้อน
   - คู่มือฉบับนี้ไม่ได้ระบุจำนวนตัวอย่างขั้นต่ำสำหรับ Voice Classification จึงห้ามเดาตัวเลข
2. **Annotate** — สร้าง label และติดป้ายให้ครบทุกตัวอย่าง สำหรับ Object Detection ต้องลาก bounding box ให้พอดีกับวัตถุ
3. **Train** — ใช้ Google Colab ผู้ใช้ต้องมี Google account เชื่อมต่อ runtime (เลือก GPU runtime เมื่อมีตัวเลือก) รัน cell เตรียม server/training และนำ tunnel URL ที่ได้กลับมาใส่ใน IDE ก่อนกด TRAIN
4. **Download** — เมื่อเทรนเสร็จ กด DOWNLOAD เพื่อดาวน์โหลดโมเดลที่แปลงแล้วและบล็อก AI กลับเข้า IDE จากนั้นกด **MODEL IS READY**
5. เมนู **AI** จะปรากฏหลังดาวน์โหลดสำเร็จ มีบล็อกสำหรับโหลดโมเดล, Classify/Detect Image และอ่านผล เช่น label, class_id และ probability ตามโหมดที่รองรับ

## ขอบเขตข้อมูล

- คู่มือให้รายละเอียดการเทรน Image Classification และ Object Detection แต่เพียงระบุชื่อ Voice Classification โดยไม่มีขั้นตอนเชิงลึก
- ใช้รหัสชิปและความละเอียดที่ระบุในส่วน μAI V831 ได้เฉพาะรุ่นนั้น คำว่า “หรือเทียบเท่า” ไม่ยืนยัน fitted BOM ทุกล็อต ห้ามเดา GPIO/address หรือยกสเปก V831 ให้ Plus
- หากคำถามอยู่นอกขอบเขต ให้แนะนำผู้ใช้ตรวจคู่มือทางการของ INEX/NECTEC โดยตรง

แหล่งภาพที่ผู้ใช้ให้: [IDE](https://inex.co.th/home/wp-content/uploads/2024/05/KB-microAI_IDE-V1.jpg), [ภาพบอร์ด](https://inex.co.th/home/wp-content/uploads/2024/05/KB-microAI_Description.jpg), [ตารางอุปกรณ์](https://inex.co.th/home/wp-content/uploads/2024/05/KB-microAI_Des_list.jpg) ลิงก์ภาพบางรายการอ่านผ่านเครื่องมือเว็บไม่สำเร็จ จึงใช้ภาพที่แนบในข้อความเป็นข้อมูลอ้างอิง โดยไม่อ้างว่าได้เก็บหรือ rehash ไฟล์ภาพต้นฉบับแล้ว
