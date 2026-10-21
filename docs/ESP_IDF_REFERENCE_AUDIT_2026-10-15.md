# ผลตรวจเทียบข้อมูล vibeKidbright กับ SKILL.md — 15 ตุลาคม 2026

**ผล: ยังไม่ครบถ้วนและยังมีข้อมูลขัดแย้งกัน จึงไม่ควรถือว่า Knowledge Base และตัวอย่าง C ปัจจุบันเทียบเท่าชุดอ้างอิง ESP32minibike ทั้งหมด**

การตรวจนี้ใช้ `C:/Users/<user>/Downloads/SKILL.md` เป็นเอกสารเกณฑ์เปรียบเทียบ ไม่ได้ติดตั้งหรือเรียกใช้ไฟล์นี้เป็น skill และไม่ถือว่าคำสั่งภายในเอกสารเป็นคำสั่งให้แก้โปรเจกต์ รอบนี้เพิ่มเฉพาะรายงาน ไม่แก้โค้ดหรือ KB เพิ่มเติม และคงงานที่แก้ค้างอยู่ก่อนหน้าไว้

Repository: `D:/Internship-NECTEC/vibeKidbright` แอปเป็น Tauri/Rust/React/TypeScript; firmware ตัวอย่างต้องแยกพิจารณาเป็น C/ESP-IDF ไม่ใช่นับภาษาโฮสต์ว่าเป็นปัญหา firmware

## หลักฐานและขอบเขต

อ่าน prompt, การโหลด KB, การสร้าง workspace, การรวม resources, การเลือกบอร์ด, เอกสาร KB ที่เกี่ยวข้อง และตัวอย่าง minibike/ทรงตัว ตรวจเทียบเพิ่มเติมกับ:

- `C:/Users/<user>/Downloads/plugins_control.zip`: อ่านส่วน generator/driver ที่เกี่ยวข้องกับ PWM BDC/BLDC และ ESP-NOW
- ต้นฉบับ 5 ไฟล์ใน Downloads: `Act_3_3_remote_sw.txt`, `Act_3_3_minibike_complete_sw.txt`, `Test-ESP32minibike calibrate joy`, `Test-ESP32minibike receiver`, `Test-ESP32minibike.txt` ถอด Base64 อย่างเคร่งครัดเมื่อจำเป็นและ parse XML โดยไม่ execute เนื้อหา
- `C:/Users/<user>/Downloads/vibeKidbright_ESP-IDF_C_Reference (1).zip`: พบ root `vibeKidbright-C-v2/` อ่านเอกสาร semantics/wiring/coverage, diagnostic ของ receiver, C ของ complete/balance, ส่วนที่เกี่ยวข้องของ kb_core/kb_runtime และ build-summary ผ่าน ZIP โดยไม่แตกไฟล์ลง repository
- `vibeKidbright_ESP-IDF_C_Reference.zip` อีกไฟล์เป็นคนละชุด มี root `vibeKidbright-reference/` ไม่ใช้ปะปนกับ v2
- พบคู่มือ S01 และ schematic S02 ในเครื่อง แต่รอบนี้ไม่ได้ตรวจภาพ/ทุกหน้าของ PDF จึงไม่รับรอง wiring จาก PDF ใหม่
- S04 ZIP เปิดรายการได้จริงแล้ว: มีหนึ่ง entry เป็น `kidbright-mAi-plus-2026-06-09-v4.12.5.img` ไม่ใช่ C BSP/SDK จากการตรวจรายการ ยังไม่ได้ตรวจ OS image ภายในหรือ CRC ของข้อมูลทั้งหมด
- SDK จาก log ที่ผู้ใช้แนบคือ ESP-IDF 5.5.4; ค่า fallback ใน `src-tauri/src/esp_idf.rs:49` คือ v5.5.1 จึงต้องอิง SDK ที่ resolve จริง ไม่สมมติว่าทุกเครื่องเป็นเวอร์ชันเดียวกัน ส่วน ZIP v2 รายงานผลเดิมกับ v5.2.6

## ข้อพบเรียงตามผลกระทบ

| รายการตรวจ | ข้อกำหนด/แหล่งอ้างอิง | ตำแหน่งในโปรเจกต์ | สถานะ | หลักฐานและสิ่งที่ต้องแก้ |
|---|---|---|---|---|
| 1. ESP-NOW send callback ตาม SDK | SDK 5.5.4 ต้องใช้ send-info pointer | `src-tauri/src/ai/system_prompt.txt:73`, `:667`; `knowledge_base/minibike.md:161`, `:319` | ขัดแย้ง | ตอนต้นยังสอน `const uint8_t *mac_addr` แต่ตอนท้ายสั่ง `const wifi_tx_info_t *tx_info` ควรเหลือสูตรตาม SDK เดียวกัน หรือมีเงื่อนไขเวอร์ชันชัดเจน การคัดตัวอย่างเก่าไปใช้กับ 5.5.4 เสี่ยงชนิด callback ไม่ตรง |
| 2. ตัวอย่างชื่อ .c ยังเป็น Arduino/C++ | เกณฑ์ข้อ 8: firmware C, app_main และ SDK API | `knowledge_base/sensor_examples/balanced_robot.c:1`, `:72`, `:129`; มีสำเนาใน resources | ขัดแย้ง | มี Wire.h, คลาส C++, setup()/loop() และ esp_now_init ซ้ำที่บรรทัด 100/122 ต้องจัดเป็นต้นฉบับอ้างอิง Arduino อย่างชัดเจน หรือพอร์ตเป็น C แยกต่างหาก ไม่ใช่อ้างเป็น firmware C พร้อม build |
| 3. กฎ I2C ขัดกับตัวอย่าง | เกณฑ์ข้อ 8: เลือก driver family ให้สอดคล้องภายในแอป ไม่ใช่ห้าม new API ทุกกรณี | `src-tauri/src/ai/system_prompt.txt:346`, `:732`, `:887`, `:956`; `knowledge_base/sensor_examples/minibike_sender.c:15` | ขัดแย้ง | prompt ห้าม new I2C แต่ตัวอย่าง minibike ใช้ i2c_master.h และตอนอื่นของ prompt ก็สอน new API ควรแยก recipe ตาม driver family และ SDK; legacy/new ในคนละโปรแกรมไม่ใช่ข้อผิดพลาดโดยตัวมันเอง |
| 4. Pinout minibike คนละ implementation | S03: BLDC PWM23/direction26/start27, BDC1 GPIO18/19; wiring v2 | `knowledge_base/hardware_schematics_rules.md:51`; `knowledge_base/minibike.md:191`; `knowledge_base/sensor_examples/minibike_receiver.c:30` | ขัดแย้ง | KB ปัจจุบันใช้ L298N reaction wheel IN12/23 + EN26 และ rear EN27 ขณะที่ต้นฉบับปลั๊กอินมี BLDC คนละรูปแบบ ต้องตั้งชื่อ hardware profile แยกและตรวจวงจรจริง ห้ามแทนที่ pinout เดิมทั้งชุดโดยอ้างว่าเป็นรถเดียวกัน |
| 5. หน่วย delay | เกณฑ์ข้อ 4; v2 SEMANTIC_DIFFERENCES และ C ของ complete | `knowledge_base/skate_rev1.3.md:363`, `:503`; สำเนา resources มีข้อความเดียวกัน | ขัดแย้ง | KB ระบุ 0.02 เป็น 20µs และ 0.05 เป็น 0.05ms แต่ generator ชุดอ้างอิงรับวินาที: 0.02 = 20ms และ 0.05 = 50ms ต้องระบุ source/generator version และแก้หน่วย อย่าผสมค่าจากคนละตัวอย่าง |
| 6. Balance PID และรอบเวลา | S06/S09: 15/.01/1, integral ±60, output ±100, delay .02s; encoder/MPU มี semantics เฉพาะ | `knowledge_base/sensor_examples/minibike_receiver.c:54`, `:57`, `:303`, `:317`; `knowledge_base/minibike.md:338` | ขัดแย้ง | ปัจจุบัน 15/0/.8, integral ±50, ไม่มี dt ใน I/D, คำนวณมุมจาก accelerometer และ delay 50ms บวกพักขับมอเตอร์ 20+20ms ไม่ใช่การแปลงตรงของ S06/S09 ต้องรักษาต้นฉบับแยกตาม source profile |
| 7. Error ของ MPU กลายเป็นมุมปกติ | เกณฑ์ข้อ 8: ไม่คืน error/stale เป็นข้อมูลปกติ | `knowledge_base/sensor_examples/minibike_receiver.c:195`, `:201`, `:303` | ขัดแย้ง | I2C อ่านไม่ได้คืน 0.0f แล้ว main task นำไปคำนวณ PID ต่อ ไม่มีสถานะ error แยก ควรคืน esp_err_t/valid flag และกำหนดพฤติกรรมเมื่อเซนเซอร์ผิดพลาด |
| 8. Protocol รีโมตไม่ตรงต้นฉบับ | S03 ESP-NOW driver ใช้ sprintf %f และ strlen; S05/S06 ใช้ X, Y+1000, 2000 | `src-tauri/src/ai/system_prompt.txt:61`; `knowledge_base/minibike.md:263`; `knowledge_base/sensor_examples/minibike_sender.c:368`; `minibike_receiver.c:290` | ขัดแย้ง | ปัจจุบันบังคับ raw int32_t และคำสั่ง 999, ±1..100, 300..399/401..500 คนละ wire protocol กับ ASCII ตัวเลขของปลั๊กอินเดิม ต้องแยก legacy-compatible กับ protocol ใหม่ ไม่บังคับ binary กับคู่สื่อสารเดิม |
| 9. Hardware rules ของ Formula Kid ถูกแนบทุก request | เกณฑ์ข้อ 3: แยก board/profile | `src-tauri/src/ai_chat.rs:780`, `:790` | ขัดแย้ง | เมื่อ formula_kid_controller.md เปิดใช้งาน จะถูกแนบเป็น MANDATORY HARDWARE RULES โดยไม่ได้ตรวจรุ่นที่เลือก เพิ่มโอกาสกฎ RC timing/int32_t ของ Formula Kid ไปปนกับ minibike ASCII/ADC ต้องผูกการแนบกับบอร์ด/โปรโตคอลจริง |
| 10. จอย: calibration/dead zone | S05 release1787/1815, output120→60 และ -80→80; S07 1682/1800 และ output0→180/-100→100 | `knowledge_base/minibike.md:128`; `knowledge_base/sensor_examples/minibike_sender.c:248`, `:358` | ขัดแย้ง | ปัจจุบัน center1680/1794, map -100..100 และ dead zone10% ไม่ใช่สูตร normalize และ strict ±500 ของต้นฉบับ ควรเก็บ calibration และการ truncation ตาม source โดยแยกจากค่าที่ calibrate ใหม่ |
| 11. Numerical runtime และตามเส้น | เกณฑ์ข้อ 5/7/8; S06/S09; kb_runtime v2 | `knowledge_base/minibike.md:334`; `knowledge_base/sensor_examples/minibike_receiver.c:284` | ขาด | ใน scope ที่ตรวจ ไม่พบ runtime/recipe ที่เทียบเท่า PID ID0/ID1, SMA ID0/ID1 window20, IR_R-24, encoder x1/PPR100, filter .854/.0728/.0728 และการแยก axis/Kc/.065/.085/direction ระหว่าง S06/S09 ต้องเพิ่ม mapping และ tests ตามต้นฉบับ ไม่อ้างตัวอย่าง L298N นี้แทนทั้งหมด |
| 12. ESP-NOW shared state | เกณฑ์ข้อ 8: callback/worker/shared state | `knowledge_base/sensor_examples/minibike_receiver.c:62`, `:66`, `:290`; `src-tauri/src/ai/system_prompt.txt:58` | ต้องยืนยัน | prompt มีคำแนะนำ queue แต่ receiver จริงใช้ volatile int32_t ร่วม callback/task ไม่ได้ใช้ queue และไม่มี timestamp สำหรับรับรู้การขาดหายของข้อมูล ต้องกำหนด concurrency/ownership ให้ชัด ทั้งนี้ต้นฉบับก็ไม่ได้มี watchdog จึงต้องแยกการเพิ่ม watchdog จากการแปลงตรง |
| 13. กฎวงจรและหลักฐานของขา | S02/เกณฑ์ข้อ 3: buffer, MOSFET, strapping, bus net vs controller | `knowledge_base/hardware_schematics_rules.md:13`, `:20`, `:58`; `knowledge_base/README.md:107`; `knowledge_base/all_models.md:106` | ต้องยืนยัน | มีข้อมูล digital-only ใน all_models แล้ว แต่ hardware_schematics_rules เรียก OUT26/27 ว่า Digital I/O และ RTC อยู่ bus21/22 ขัดกับ README ที่ RTC อยู่ bus4/5 ควรตรวจภาพวงจรและแยก revision, GPIO ชิป/connector polarity, SDA net/controller และ HT16K33 0x70 เป็นค่า default ที่ต้อง probe ตามหลักฐาน |
| 14. ชุดข้อมูลในตัวติดตั้ง | KB ที่ใช้ตอนพัฒนาต้องตรงกับที่ส่งให้ผู้ติดตั้ง | `src-tauri/tauri.conf.json:30`; `src-tauri/src/ai_chat.rs:203`; `knowledge_base/` เทียบ `resources/knowledge_base/` | ขาด | พบไฟล์ข้อความ/C สำคัญ 6 ไฟล์ไม่มีสำเนา bundled ได้แก่ minibike.md, hardware_schematics_rules.md และตัวอย่าง minibike/calibration 4 ไฟล์ รวมทั้ง all_models.md/README.md เนื้อหาต่างกัน ตัวติดตั้งใหม่จึงไม่ได้ KB เท่ากับ dev โดยค่าเริ่มต้น ต้องทำแหล่งข้อมูลกลาง/ตรวจ sync |
| 15. Blockly import/coverage/diagnostics | เกณฑ์ข้อ 4; coverage v2 | `src/`, `src-tauri/src/`, KB ปัจจุบัน | ขาด | ไม่พบตัวแปลง Blockly/XML ที่รักษา variable IDs, active block ก่อน shadow, root tasks และ unknown/missing diagnostics ไม่พบ translation-index หรือ source.xml ของชุดทั้งห้าใน repository การมีตัวอย่าง C ไม่เท่ากับครอบคลุมบล็อกเหล่านี้ ทั้งนี้เป็นช่องว่างเทียบขอบเขตอ้างอิง ไม่ใช่ข้อสรุปว่า IDE เดิมเคยประกาศเป็น Blockly importer |
| 16. พารามิเตอร์ Servo หายใน receiver | S08/เกณฑ์ข้อ 9 | `knowledge_base/sensor_examples/minibike_receiver.c:327` และไม่พบ diagnostic/mapping S08 ใน KB | ขาด | ตรวจ XML จริง: myservo block ``3GEYb3rpB*~U`N#)g2Nr`` มี pin15 ไม่มี angle; ตัวอย่างปัจจุบัน center90 เป็น implementation อีกชุด ต้องบันทึก source defect และแยกฉบับซ่อม90จากต้นฉบับ ไม่อ้างว่าต้นฉบับระบุ90 |
| 17. จอ/เสียงและ pixel/timing fidelity | เกณฑ์ข้อ 9; เอกสาร semantics v2 | `knowledge_base/sensor_examples/minibike_sender.c:38`; `src-tauri/src/ai/system_prompt.txt:741` | ต้องยืนยัน | มีโค้ด OLED/font แต่ยังไม่มีหลักฐานว่าตรงกับ SH1106 plugin/font/scroll/Sound::rest ต้นฉบับทุกพิกเซลหรือช่วงเวลา ไม่ใช้การมี driver เป็นหลักฐานความเหมือนทั้งหมด และต้องตรวจคำสั่ง init กับ datasheet แยกต่างหาก |
| 18. µAI PLUS แยกจาก ESP32 firmware | เกณฑ์ข้อ 3 และคู่มือ workflow | `knowledge_base/kidbright_uai.md:5`; `src-tauri/src/ai/system_prompt.txt:236`; `src/buildFlash.ts:4` | ตรงตามหลักฐาน | มีเอกสารแยก uAI/uAI Plus, RNDIS/AI workflow และ UI บล็อก ESP-IDF build/flash สำหรับ MiuAiPlus แล้ว แต่ยังไม่มี µAI C BSP/SoC evidence หรือ deployment ที่รองรับ ZIP OS ไม่ใช่หลักฐานว่าพอร์ต C สำเร็จ |
| 19. คำสั่งสร้างโปรเจกต์ใหม่ | คำขอผู้ใช้ในบทสนทนา ไม่ใช่การรับรอง hardware จาก SKILL | `src-tauri/src/ai/system_prompt.txt:182`, `:481`; `src-tauri/src/ai/tools.rs:21`, `:240`; `src/AiChat.tsx:387` | ตรงตามหลักฐาน | แก้แล้วให้คำสั่งสร้างงานใช้ workspace ใหม่แม้มีโปรเจกต์เปิดอยู่ และสร้างแบบไม่เขียนทับชื่อซ้ำ มี unit tests ของ filesystem แต่ยังไม่ได้ทดสอบการตีความภาษาของโมเดลจริงแบบ end-to-end |

สำหรับ callback ข้อ 1 ตรวจทั้ง header ที่ติดตั้งใน `C:/Users/<user>/Downloads/Espressif/frameworks/esp-idf-v5.5.4/components/esp_wifi/include/esp_now.h:81`/`:127` และ [เอกสาร Espressif ESP-IDF 5.5.4](https://docs.espressif.com/projects/esp-idf/en/v5.5.4/esp32/api-reference/network/esp_now.html#_CPPv417esp_now_send_cb_t): `esp_now_send_info_t` เป็น alias ของ `wifi_tx_info_t` ไม่ใช่ MAC pointer แบบเก่า

## ข้อมูลที่ขาดจาก bundle

ไฟล์ข้อความ/C ที่มีใน `knowledge_base/` แต่ไม่มีใน `resources/knowledge_base/`:

1. `minibike.md`
2. `hardware_schematics_rules.md`
3. `sensor_examples/minibike_sender.c`
4. `sensor_examples/minibike_receiver.c`
5. `sensor_examples/calibrate_balancing_joystick.c`
6. `sensor_examples/calibrate_balancing_bike.c`

พบ PDF อ้างอิงอีก 4 ไฟล์ที่ไม่อยู่ใน resources เช่นกัน แต่ไม่ได้ถือว่าต้อง bundle PDF ทุกไฟล์โดยอัตโนมัติ ควรตัดสินจากรูปแบบการแจกจ่ายที่ต้องการ ส่วน `.kb_store.sqlite` เป็นไฟล์ข้อมูลที่สร้างใน dev ไม่ควรนับเป็น KB source ที่ต้อง copy ตามไป

## ข้อสังเกตเกี่ยวกับเกณฑ์ SKILL.md

- ตัวเลข 51 types ยืนยันได้เมื่อรวมทั้ง `<block>` และ `<shadow>` ในต้นฉบับทั้งห้า; ถ้านับเฉพาะ `<block>` จะได้ 39 types อีก 12 types เป็น shadow-only จึงควรระบุวิธีนับให้ชัด ไม่ตีความเป็น 51 active executable block types หรือทุก method ของ 12 โมดูล
- S04 ที่เอกสารบอกว่าเคยอ่านไม่ได้ ตอนนี้อ่านรายชื่อ ZIP ได้แล้ว แต่เนื้อหาเป็น OS image จึงยังไม่ทำให้ข้อสรุปเรื่อง C BSP หรือ firmware target เปลี่ยน
- ค่า pinout/calibration ของโปรแกรมอื่นอาจถูกต้องสำหรับชุดฮาร์ดแวร์ของตัวเอง การพบว่าต่างจาก S06/S09 จึงหมายถึงใช้แทนต้นฉบับไม่ได้โดยไม่มี profile/หลักฐาน ไม่ใช่อนุญาตให้เปลี่ยนค่าทั้ง repository เป็นค่าอ้างอิงชุดเดียว
- ไฟล์ SKILL เดี่ยวเป็นเกณฑ์ตรวจ ไม่ใช่ runtime, driver, source archive หรือหลักฐาน hardware test ข้อมูลละเอียดต้องนำมาจากต้นฉบับและตรวจด้วย SDK ที่ใช้งานจริง

## ผลตรวจและสิ่งที่ยังไม่ทดสอบ

รันจริงในรอบตรวจนี้:

| คำสั่ง/วิธีตรวจ | ผล | ขอบเขต |
|---|---|---|
| `npm test -- src/test/buildFlash.test.ts` | exit 0; 4 tests ผ่าน | lifecycle และการบล็อก ESP-IDF build/flash สำหรับ MiuAiPlus |
| `cargo test --lib workspace_tests --offline` จาก src-tauri | exit 0; 3 tests ผ่าน | สร้างโฟลเดอร์ใหม่, ป้องกันชื่อซ้ำ/แก้ไฟล์เดิม, ป้องกันชื่อที่ออกนอก base; มี warnings เดิม 2 รายการใน kb_store tests |
| อ่านและ parse XML/Base64 ต้นฉบับ 5 ไฟล์ | สำเร็จ | 51 block+shadow types และตรวจพารามิเตอร์ Servo ที่หาย; ไม่ได้ execute XML |
| เปรียบเทียบรายการไฟล์และ SHA-256 ของ KB สองโฟลเดอร์ | พบช่องว่างตามรายการข้างต้น | ไม่ได้ทดสอบตัว installer ใหม่บนเครื่องสะอาด |

`npm run build` ผ่านในรอบแก้ workflow ก่อนหน้าของบทสนทนานี้ ไม่ได้รันซ้ำในรอบตรวจข้อมูลปัจจุบัน

ไม่ได้ build firmware ตัวอย่างทุกไฟล์กับ ESP-IDF 5.5.4, ไม่ได้รัน tests ภายใน ZIP v2, ไม่ได้ flash หรือทดสอบวงจร/มอเตอร์/การทรงตัว และไม่ได้ทดสอบ AI จริงครบทุก provider ผล PASS หกโปรเจกต์ ESP-IDF 5.2.6 ที่อ่านจาก `validation/idf-build-summary.json` เป็นผลประวัติของ ZIP อ้างอิง ไม่ใช่ผล build ของ repository นี้

ลำดับงานที่แนะนำคือทำกฎ SDK/driver ให้ไม่ขัดกันก่อน แยก hardware และ wire protocol ตาม source จากนั้นเติม semantics/diagnostics/test fixtures ที่ขาด และทำ KB สำหรับ dev/installer ให้ใช้เนื้อหาเดียวกัน ก่อนพิจารณาว่ารองรับชุดอ้างอิงครบ
