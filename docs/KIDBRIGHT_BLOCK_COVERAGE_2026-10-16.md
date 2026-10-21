# ตรวจความครอบคลุมบล็อกและฟังก์ชัน KidBright

ปรับข้อมูล: **17 ตุลาคม 2026 — Asia/Bangkok**

**ยังไม่ครบทุกบอร์ด ทุกฟังก์ชัน และทุกบล็อก** การจับคู่รายบล็อกในตารางนี้ยังจำกัดแกน C และสอง recipe เดิม ส่วนรอบใหม่มีตัวอย่างรวม 4 recipe และ build 7 คู่ profile/recipe ตาม [หลักฐาน expansion](research/kidbright-expansion-2026-10-17-validation.json) primitive จอ/เพลงใหม่ยังไม่ถูกนับเป็น block adapter ที่เทียบเท่าต้นฉบับ การมีคำใน `.md` หรือคอมไพล์ผ่านไม่แปลว่ารองรับทั้งระบบ

รายงานสร้างโดย `node scripts/audit-kidbright-coverage.mjs` จากหลักฐานที่เก็บใน repository จึงทำซ้ำได้โดยไม่ต้องใช้ ZIP, TEMP หรืออินเทอร์เน็ต สคริปต์ไม่ execute โค้ดอ้างอิง หากต้องการตรวจ hash ซอร์ส cache เพิ่ม ใช้ `--verify-source <research-cache>`; การเก็บ AST ใหม่ใช้ `--capture-source <research-cache>` และต้องมี TypeScript ตาม dev dependencies

หลักฐาน: [inventory JSON](research/kidbright-coverage-2026-10-16.json), [AST source index](research/kidbright-static-source-index-2026-10-16.json), [ต้นแบบเดิม](research/kidbright-2026-10-15-evidence.json), [ผลทดสอบ pilot](research/kidbright-pilot-2026-10-16-validation.json). `K:path:line` อ้างจาก application root ภายใน ZIP และ `W:path:line` อ้างจากซอร์สเว็บ snapshot 15 ตุลาคม 2026 ไม่ใช่สถานะเว็บสด

## จำนวนที่ตรวจพบ

| รายการ | จำนวน/ขอบเขต |
| --- | --- |
| Archive: นิยามแบบ custom ที่พบใน catalog เดิม | 274 IDs (105 core + 169 plugin assignments) |
| Archive: custom IDs ที่เป็น AST assignment จริง | 274 |
| Archive: built-in Blockly IDs เพิ่มเติมในไฟล์ compressed | 77 |
| Archive: union ของนิยามที่เป็น AST จริง | 343 |
| Archive: bundled vertical editor asset IDs ที่ไม่รวมในยอด KidBright หลัก | 170 |
| Web: นิยาม custom ใน catalog เดิม / AST จริง | 166 / 166 |
| Web: built-in Blockly IDs / union นิยาม AST | 77 / 235 |
| Plugin modules / หมวด plugin ใน Archive | 38 / 19 |
| Web toolbox: entries / unique IDs | 130 / 128 |
| ตัวอย่างเดิม: IDs รวม shadow / active | 51 / 39 |
| Inventory รวม definition, generator, toolbox, sample และ infrastructure | 560 |
| IDs ที่พบตัวสะกดตรงใน KB/system prompt (candidate เท่านั้น) | 5 |
| IDs ที่เป็น exact candidate และไม่ใช่คำทั่วไป function/text | 3 |
| IDs ที่โยงกับ host C component / fixed recipe subset | 13 / 9 |
| IDs ที่มี subset อย่างน้อยหนึ่งส่วน (ไม่บวกซ้ำ) | 20 |
| Full block adapters ที่พิสูจน์ครบ / physical equivalence | 0 / 0 |

ตัวเลข built-in รวม mutator ของ editor ที่ไม่ใช่คำสั่ง runtime และอาจซ้ำชื่อกับ custom จึงใช้ union เมื่อรวมจำนวน ส่วน generator helpers แสดงเพื่อปิดช่องว่าง inventory แต่ไม่ควรถูกนับเป็นฟังก์ชันฮาร์ดแวร์ Dynamic Variables toolbox ไม่มี static IDs ใน XML จึงต้องตรวจ definitions/generators เพิ่มด้วย

ไฟล์ `blocks_compressed_vertical.js` ที่อยู่ใน ZIP มีบล็อก Scratch เช่น motion/looks/sound/data เพิ่มอีก 170 IDs; ยังไม่ได้พิสูจน์ว่า KidBright โหลดใช้ จึงเก็บใน appendix/inventory เป็น bundled assets แยกจากนิยามหลัก 274 IDs และ built-in Blockly 77 IDs ไม่ใช้การที่ไฟล์ถูกบรรจุเป็นหลักฐานว่ารองรับ ESP-IDF

Archive มีชื่อซ้ำระหว่าง custom/built-in 8 IDs จึงรวมได้ 343 ไม่ใช่ผลบวกตรง ๆ ส่วน Web ซ้ำ 8 IDs และ union 235 รายชื่อที่ซ้ำทั้งหมดอยู่ใน JSON

คำทั่วไป `function` และ `text` ปรากฏในกฎหลายบริบทและอาจไม่หมายถึง block ID จริง จึงแยกเป็น ambiguous lexical candidates; ทั้งตัวเลข exact และการไม่พบชื่อยังไม่ตัดสิน semantic coverage ของคำอธิบายภาษามนุษย์

AST ตัด comments ออกและตรวจ aliases เพิ่ม เช่น `procedures_defnoreturn = procedures_defreturn` ผลเทียบตำแหน่ง catalog เดิมพบ generator mentions ที่ไม่ใช่ active assignment 0 ตำแหน่งใน Archive และ 0 ตำแหน่งใน Web จึงไม่พบการนับ comment เกินใน catalog เดิม แต่พบ builtin aliases เพิ่ม 8 รายการต่อฝั่ง รายละเอียดอยู่ใน `summary.*.raw_generator_mentions_without_active_assignment` และ `builtin_generator_aliases`

## หลักฐานแต่ละสถานะหมายถึงอะไร

- **นิยาม/generator:** พบ declaration จริงนอก comment ใน source AST; ยังไม่ยืนยันว่า plugin โหลดหรือรันสำเร็จ
- **KB candidate:** พบชื่อ ID ตรงใน Markdown/system prompt; ยังไม่ยืนยันความครบถ้วนของ API, พิน,หน่วย, lifecycle หรือค่าผิดพลาด
- **Host component:** มี C helper และ host tests ของ subset ที่ระบุ; ไม่มีตัวแปลงบล็อกทั่วไปหรือความเข้ากันได้ทุกค่าพารามิเตอร์
- **Recipe subset:** มีงานสำเร็จรูปขอบเขตตายตัวและ hash โค้ดตรงผล build ที่บันทึก; ไม่ใช่ทุกตัวเลือกของบล็อกนั้น
- **—:** ไม่พบหลักฐานชั้นนั้นใน snapshot นี้; ไม่ควรแปลว่า AI เขียน C ในเรื่องนั้นไม่ได้เลย

การจัดกลุ่มตามชื่อบล็อก/path เป็นเครื่องมือช่วยค้น ไม่แทนการอ่าน semantics และทดสอบ โค้ด C++ ของ plugin เดิมไม่ถูกนับเป็น implementation ของแอป และ JavaScript simulator ไม่ถูกนับเป็น firmware

## ความครอบคลุมรายหมวด

| หมวด | IDs | Archive defs | Web defs | KB candidate | Host subset | Recipe subset |
| --- | --- | --- | --- | --- | --- | --- |
| Blockly editor mutators | 9 | 9 | 9 | 0 | 0 | 0 |
| Bundled vertical editor assets (loading unverified) | 170 | 170 | 0 | 0 | 0 | 0 |
| Colors | 4 | 4 | 4 | 0 | 0 | 0 |
| Control, motors and expansion boards | 66 | 65 | 8 | 0 | 7 | 0 |
| Digital I/O and waits | 32 | 28 | 23 | 0 | 0 | 5 |
| Displays and lighting | 40 | 40 | 17 | 0 | 0 | 0 |
| Functions and procedures | 7 | 7 | 7 | 1 | 0 | 0 |
| Generator helpers or unclassified IDs | 1 | 0 | 0 | 0 | 0 | 0 |
| Lists | 19 | 19 | 19 | 0 | 0 | 0 |
| Logic and conditions | 8 | 8 | 8 | 0 | 0 | 0 |
| Loops, delays and tasks | 12 | 12 | 11 | 2 | 2 | 2 |
| Math and constants | 36 | 16 | 35 | 0 | 0 | 0 |
| Music and audio | 6 | 5 | 5 | 1 | 0 | 1 |
| Networking, cloud and IoT | 63 | 48 | 46 | 0 | 3 | 0 |
| RTC and clock | 11 | 11 | 11 | 0 | 0 | 0 |
| Sensors and analog input | 37 | 37 | 2 | 0 | 1 | 1 |
| Serial and plotting | 4 | 4 | 0 | 0 | 0 | 0 |
| Text and conversion | 24 | 23 | 23 | 1 | 0 | 0 |
| Ultrasonic auxiliary blocks | 4 | 0 | 0 | 0 | 0 | 0 |
| Variables | 7 | 7 | 7 | 0 | 0 | 0 |

## C ที่มีหลักฐานตรวจจริงและข้อจำกัด

| Component | API | ขอบเขตที่ตรวจ / ข้อยกเว้น |
| --- | --- | --- |
| delay | `kb_delay_ms` | Nonnegative finite seconds -> truncated integer milliseconds; scheduling and arbitrary block composition are not certified |
| lm73 | `kb_lm73_celsius` | Signed MSB-first LM73 raw /128 decoding; physical sensor/bus behavior is not certified |
| pid | `kb_pid_step` | Time-based PID with separate state, I ±60/output ±100; first-sample and invalid-dt repairs and double precision differ explicitly |
| sma | `kb_sma_step` | Warm-up and rolling SMA; windows 1..50 only; window changes reset state; no EMA implementation |
| joystick | `kb_joystick_init`, `kb_joystick_read` | One calibrated numeric axis, descending endpoints and strict ±500 dead zone; no ADC driver, two-axis dispatcher or calibration UI |
| ascii | `kb_ascii_format`, `kb_ascii_parse` | Legacy numeric %f payload codec only; no ESP-NOW transport, callbacks, peer management or string API |

Host tests อ้างจาก `src-tauri/tests/kidbright_core_test.c` และเป็น test ของ implementation ปัจจุบัน มิใช่การรันเทียบทุก input กับ runtime เดิม PID/SMA มี behavior repairs ตาม `knowledge_base/compatibility/README.md` จึงไม่อ้าง bit-exact ทุกกรณี มีเพียง codec ASCII ของ ESP-NOW; การเชื่อมต่อวิทยุจริงยังไม่มีใน pilot

Recipe `button_buzzer` ใช้ S1 GPIO16 active-low และ buzzer GPIO13 เฉพาะ A4 440 Hz 500 ms; `temperature_serial` อ่าน LM73 SDA4/SCL5 ที่ 0x4D ทุก 500 ms ทั้งสองใช้ target esp32 และผล build ที่บันทึกเป็น ESP-IDF 5.5.4 โดยสร้างคนละ profile (`kidbright32_v13` และ `kidbright32_rev31`) ยังไม่ครอบคลุมทุกคู่ recipe/profile หรือทดลองบอร์ดจริง

## ช่องว่างที่ต้องปิดก่อนกล่าวว่าครบ

1. สัญญารายบล็อก: inputs/defaults/types/units/ranges/return/error/side effects และ lifecycle พร้อม reference source ต้องมีครบ ไม่ใช้เพียงรายการชื่อ
2. บล็อกพื้นฐาน: variables, functions, logic, loops, lists และ text ต้องตรวจการกำหนดขอบเขต ความยาวหน่วยความจำ การแปลงชนิด การเรียกซ้ำ และ composition แม้ C เขียนสิ่งเหล่านี้ได้
3. FreeRTOS/tasks: ต้องพิสูจน์ start/stop, scheduling, delay quantization, shared state, callbacks และ race จากหลายบล็อก; `for (;;)` ในตัวอย่างเดียวไม่แทน `advance_task`
4. Display/audio/clock: matrix mapping/font/scroll/ready, OLED coordinates/font/UTF-8, LCD/SPI, LED strips, tempo/scale/volume และ RTC/calendar ต้องมี API และผลตรวจของตัวเอง
5. I/O/sensors/control: ADC calibration, sensor revisions, MPU/encoder, motor direction/PWM/stop, servo pulse limits, iKB expansion, EMA และ calibration defaults ยังไม่มีหลักฐานทดสอบครบ
6. Network/cloud: ESP-NOW protocol/channel/peers/callback ownership, BLE/Bluetooth, MQTT/NETPIE/Firebase/IoT, LINE/IFTTT ต้องตรวจ API และบริการปัจจุบันก่อนใช้; snapshot ไม่รับรองว่าบริการยังใช้งานได้
7. Storage: ไม่พบบล็อก local storage/NVS/filesystem ที่ชัดเจนใน inventory นี้ จึงยังไม่มีสัญญาบล็อกสำหรับ persist/recover ไม่ใช้ความสามารถของ ESP-IDF มาทดแทนหลักฐาน
8. บอร์ด/SDK: ต้องตรวจทุก board revision พร้อม pin/bus/address/resource conflicts และ build matrix จริง รวมทั้งบอร์ดที่ไม่มี LM73 หรือ GPIO ต่างกัน
9. ทุกภาษา: ชุดนี้ไม่ได้ทดสอบความสามารถภาษาโมเดล การใช้ภาษาตามข้อความล่าสุดเป็น policy คนละชั้นกับชื่อ API/ID และ font rendering บนฮาร์ดแวร์ ไม่อาจรับรองทุกภาษาเพียงเพราะ prompt ระบุไว้
10. End-to-end: ต้องทดสอบคำขอภาษาธรรมชาติ → เลือกบอร์ด/สร้าง → Keep/Undo → build → behavior บอร์ดจริง และเปรียบเทียบกับต้นแบบหลายตัวเลือก โดยไม่ใช้ผล compile แทนผลฮาร์ดแวร์

## โมดูล Archive ทั้งหมดที่พบ static block definitions

| Source module | ประเภท | IDs | มี original generator | KB candidate | Host subset | Recipe subset |
| --- | --- | --- | --- | --- | --- | --- |
| `kbide/app/public/blockly/kidbright/blocks-array.js` | core | 8 | 8 | 0 | 0 | 0 |
| `kbide/app/public/blockly/kidbright/blocks-function.js` | core | 2 | 2 | 1 | 0 | 0 |
| `kbide/app/public/blockly/kidbright/blocks-iot.js` | core | 12 | 12 | 0 | 0 | 0 |
| `kbide/app/public/blockly/kidbright/blocks-math.js` | core | 7 | 7 | 0 | 0 | 0 |
| `kbide/app/public/blockly/kidbright/blocks-mqtt.js` | core | 5 | 5 | 0 | 0 | 0 |
| `kbide/app/public/blockly/kidbright/blocks-netpie.js` | core | 12 | 12 | 0 | 0 | 0 |
| `kbide/app/public/blockly/kidbright/blocks-string.js` | core | 7 | 7 | 0 | 0 | 0 |
| `kbide/app/public/blockly/kidbright/blocks.js` | core | 52 | 52 | 2 | 2 | 9 |
| `kbide/plugins/analog/rd_analog/blocks.js` | plugin | 2 | 2 | 0 | 0 | 0 |
| `kbide/plugins/bluetooth/ble_uart/blocks.js` | plugin | 5 | 5 | 0 | 0 | 0 |
| `kbide/plugins/display/lcd_i2c/blocks.js` | plugin | 8 | 8 | 0 | 0 | 0 |
| `kbide/plugins/display/lcd_spi/blocks.js` | plugin | 8 | 8 | 0 | 0 | 0 |
| `kbide/plugins/eiei6050/mpu6050/blocks.js` | plugin | 5 | 5 | 0 | 0 | 0 |
| `kbide/plugins/esp_now/esp_now_class/blocks.js` | plugin | 6 | 6 | 0 | 3 | 0 |
| `kbide/plugins/examples/blink/blocks.js` | plugin | 2 | 2 | 0 | 0 | 0 |
| `kbide/plugins/FIR/fir/blocks.js` | plugin | 8 | 8 | 0 | 0 | 0 |
| `kbide/plugins/formula_kid/drv8833/blocks.js` | plugin | 3 | 3 | 0 | 0 | 0 |
| `kbide/plugins/formula_kid/joystick/blocks.js` | plugin | 9 | 9 | 0 | 0 | 0 |
| `kbide/plugins/gpio/mcp23s17_16in/blocks.js` | plugin | 3 | 3 | 0 | 0 | 0 |
| `kbide/plugins/gpio/mcp23s17_16out/blocks.js` | plugin | 4 | 4 | 0 | 0 | 0 |
| `kbide/plugins/gpio/pulse_cap/blocks.js` | plugin | 2 | 2 | 0 | 0 | 0 |
| `kbide/plugins/gpio/pulse_gen/blocks.js` | plugin | 2 | 2 | 0 | 0 | 0 |
| `kbide/plugins/ikb_1_plugin/ikb_1/blocks.js` | plugin | 24 | 24 | 0 | 0 | 0 |
| `kbide/plugins/imu_sensors/mpu6050/blocks.js` | plugin | 4 | 4 | 0 | 0 | 0 |
| `kbide/plugins/led/ws2812b_rmt/blocks.js` | plugin | 6 | 6 | 0 | 0 | 0 |
| `kbide/plugins/myservo/myservo/blocks.js` | plugin | 2 | 2 | 0 | 0 | 0 |
| `kbide/plugins/oled/oled_128x64_i2c/blocks.js` | plugin | 9 | 9 | 0 | 0 | 0 |
| `kbide/plugins/pid_control/filter/blocks.js` | plugin | 2 | 2 | 0 | 1 | 0 |
| `kbide/plugins/pid_control/mjoystick/blocks.js` | plugin | 13 | 13 | 0 | 3 | 0 |
| `kbide/plugins/pid_control/pid_con/blocks.js` | plugin | 6 | 6 | 1 | 4 | 0 |
| `kbide/plugins/pid_control/pwm_bdc/blocks.js` | plugin | 2 | 2 | 0 | 0 | 0 |
| `kbide/plugins/pid_control/pwm_bldc/blocks.js` | plugin | 2 | 2 | 0 | 0 | 0 |
| `kbide/plugins/pid_control/pwm_gen/blocks.js` | plugin | 2 | 2 | 0 | 0 | 0 |
| `kbide/plugins/pid_control/quad_encoder/blocks.js` | plugin | 3 | 3 | 0 | 0 | 0 |
| `kbide/plugins/science_lab/ds18b20_i2c/blocks.js` | plugin | 1 | 1 | 0 | 0 | 0 |
| `kbide/plugins/serial_plotter/serial_plotter/blocks.js` | plugin | 3 | 3 | 0 | 0 | 0 |
| `kbide/plugins/ultrasonic/ultrasonic/blocks.js` | plugin | 1 | 1 | 0 | 0 | 0 |
| `kbide/plugins/weather_sensors/ads1115/blocks.js` | plugin | 1 | 1 | 0 | 0 | 0 |
| `kbide/plugins/weather_sensors/adsws1_rg/blocks.js` | plugin | 3 | 3 | 0 | 0 | 0 |
| `kbide/plugins/weather_sensors/adsws1_wd/blocks.js` | plugin | 2 | 2 | 0 | 0 | 0 |
| `kbide/plugins/weather_sensors/adsws1_ws/blocks.js` | plugin | 2 | 2 | 0 | 0 | 0 |
| `kbide/plugins/weather_sensors/hpma115s0/blocks.js` | plugin | 5 | 5 | 0 | 0 | 0 |
| `kbide/plugins/weather_sensors/sht21/blocks.js` | plugin | 3 | 3 | 0 | 0 | 0 |
| `kbide/plugins/weather_sensors/sht31/blocks.js` | plugin | 3 | 3 | 0 | 0 | 0 |
| `kbide/plugins/weather_sensors/si7021/blocks.js` | plugin | 2 | 2 | 0 | 0 | 0 |
| `kbide/plugins/weather_sensors/tsl2561/blocks.js` | plugin | 1 | 1 | 0 | 0 | 0 |

## Definitions ที่ไม่พบ generator ใน snapshot

รายการนี้รวม editor mutators ซึ่งไม่จำเป็นต้องมี runtime generator และอาจรวมบล็อกที่มีตัวกำหนดแบบ dynamic นอกขอบเขต static parser จึงเป็นรายการตรวจต่อ ไม่ใช่ข้อพิสูจน์ว่าต้นแบบเสียทุกตัว

**archive:** `controls_if_else`, `controls_if_elseif`, `controls_if_if`, `lists_create_with_container`, `lists_create_with_item`, `procedures_mutatorarg`, `procedures_mutatorcontainer`, `text_create_join_container`, `text_create_join_item`

**web:** `controls_if_else`, `controls_if_elseif`, `controls_if_if`, `lists_create_with_container`, `lists_create_with_item`, `procedures_mutatorarg`, `procedures_mutatorcontainer`, `text_create_join_container`, `text_create_join_item`

## บัญชีทุก ID และสถานะ

ตารางแยกตามหมวด รวมทุก ID ใน inventory JSON คอลัมน์ `K def/gen` และ `W def/gen` นับ source declarations ที่พบ (ไม่ใช่จำนวนบอร์ดที่รองรับ) แหล่งอ้างอิงสั้นแสดง definition แรกหรือ generator แรก; JSON มีแหล่งอ้างอิงทั้งหมดพร้อมบรรทัด รวมถึงตำแหน่ง candidate ในกฎ

### Blockly editor mutators

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `controls_if_else` | 1/0 | 1/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:48 |
| `controls_if_elseif` | 1/0 | 1/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:48 |
| `controls_if_if` | 1/0 | 1/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:48 |
| `lists_create_with_container` | 1/0 | 1/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:16 |
| `lists_create_with_item` | 1/0 | 1/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:17 |
| `procedures_mutatorarg` | 1/0 | 1/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:105 |
| `procedures_mutatorcontainer` | 1/0 | 1/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:104 |
| `text_create_join_container` | 1/0 | 1/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:123 |
| `text_create_join_item` | 1/0 | 1/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:124 |

### Bundled vertical editor assets (loading unverified)

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `argument_editor_boolean` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:152 |
| `argument_editor_string_number` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:153 |
| `argument_reporter_boolean` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:151 |
| `argument_reporter_string_number` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:151 |
| `control_all_at_once` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:52 |
| `control_clear_counter` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:52 |
| `control_create_clone_of` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:50 |
| `control_create_clone_of_menu` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:49 |
| `control_delete_this_clone` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:50 |
| `control_for_each` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:48 |
| `control_forever` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:38 |
| `control_get_counter` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:51 |
| `control_if` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:40 |
| `control_if_else` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:41 |
| `control_incr_counter` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:51 |
| `control_repeat` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:39 |
| `control_repeat_until` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:46 |
| `control_start_as_clone` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:49 |
| `control_stop` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:42 |
| `control_wait` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:44 |
| `control_wait_until` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:45 |
| `control_while` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:47 |
| `data_addtolist` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:161 |
| `data_changevariableby` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:155 |
| `data_deletealloflist` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:163 |
| `data_deleteoflist` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:162 |
| `data_hidelist` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:171 |
| `data_hidevariable` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:157 |
| `data_insertatlist` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:164 |
| `data_itemnumoflist` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:167 |
| `data_itemoflist` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:166 |
| `data_lengthoflist` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:168 |
| `data_listcontainsitem` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:169 |
| `data_listcontents` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:158 |
| `data_listindexall` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:159 |
| `data_listindexrandom` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:160 |
| `data_replaceitemoflist` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:165 |
| `data_setvariableto` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:154 |
| `data_showlist` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:170 |
| `data_showvariable` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:156 |
| `data_variable` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:153 |
| `event_broadcast` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:79 |
| `event_broadcast_menu` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:78 |
| `event_broadcastandwait` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:79 |
| `event_touchingobjectmenu` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:72 |
| `event_whenbackdropswitchesto` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:76 |
| `event_whenbroadcastreceived` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:75 |
| `event_whenflagclicked` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:73 |
| `event_whengreaterthan` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:77 |
| `event_whenkeypressed` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:80 |
| `event_whenstageclicked` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:74 |
| `event_whenthisspriteclicked` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:74 |
| `event_whentouchingobject` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:71 |
| `extension_microbit_display` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:200 |
| `extension_music_drum` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:193 |
| `extension_music_reporter` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:199 |
| `extension_pen_down` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:192 |
| `extension_wedo_boolean` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:196 |
| `extension_wedo_hat` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:195 |
| `extension_wedo_motor` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:194 |
| `extension_wedo_tilt_menu` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:198 |
| `extension_wedo_tilt_reporter` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:197 |
| `looks_backdropnumbername` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:69 |
| `looks_backdrops` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:66 |
| `looks_changeeffectby` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:56 |
| `looks_changesizeby` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:60 |
| `looks_changestretchby` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:61 |
| `looks_cleargraphiceffects` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:59 |
| `looks_costume` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:63 |
| `looks_costumenumbername` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:70 |
| `looks_goforwardbackwardlayers` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:68 |
| `looks_gotofrontback` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:67 |
| `looks_hide` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:54 |
| `looks_hideallsprites` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:55 |
| `looks_nextbackdrop` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:71 |
| `looks_nextcostume` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:64 |
| `looks_say` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:52 |
| `looks_sayforsecs` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:52 |
| `looks_seteffectto` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:58 |
| `looks_setsizeto` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:60 |
| `looks_setstretchto` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:62 |
| `looks_show` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:54 |
| `looks_size` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:61 |
| `looks_switchbackdropto` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:65 |
| `looks_switchbackdroptoandwait` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:71 |
| `looks_switchcostumeto` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:64 |
| `looks_think` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:53 |
| `looks_thinkforsecs` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:53 |
| `motion_align_scene` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:191 |
| `motion_changexby` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:184 |
| `motion_changeyby` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:185 |
| `motion_direction` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:189 |
| `motion_glidesecstoxy` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:182 |
| `motion_glideto` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:184 |
| `motion_glideto_menu` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:183 |
| `motion_goto` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:181 |
| `motion_goto_menu` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:180 |
| `motion_gotoxy` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:181 |
| `motion_ifonedgebounce` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:186 |
| `motion_movesteps` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:174 |
| `motion_pointindirection` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:177 |
| `motion_pointtowards` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:179 |
| `motion_pointtowards_menu` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:178 |
| `motion_scroll_right` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:189 |
| `motion_scroll_up` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:190 |
| `motion_setrotationstyle` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:187 |
| `motion_setx` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:185 |
| `motion_sety` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:186 |
| `motion_turnleft` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:176 |
| `motion_turnright` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:175 |
| `motion_xposition` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:188 |
| `motion_xscroll` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:192 |
| `motion_yposition` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:188 |
| `motion_yscroll` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:192 |
| `operator_add` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:107 |
| `operator_and` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:115 |
| `operator_contains` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:119 |
| `operator_divide` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:110 |
| `operator_equals` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:113 |
| `operator_gt` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:114 |
| `operator_join` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:117 |
| `operator_length` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:118 |
| `operator_letter_of` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:118 |
| `operator_lt` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:112 |
| `operator_mathop` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:121 |
| `operator_mod` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:120 |
| `operator_multiply` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:109 |
| `operator_not` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:117 |
| `operator_or` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:116 |
| `operator_random` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:111 |
| `operator_round` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:120 |
| `operator_subtract` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:108 |
| `procedures_call` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:144 |
| `procedures_declaration` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:148 |
| `procedures_definition` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:143 |
| `procedures_prototype` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:146 |
| `sensing_answer` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:92 |
| `sensing_askandwait` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:92 |
| `sensing_coloristouchingcolor` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:90 |
| `sensing_current` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:104 |
| `sensing_dayssince2000` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:105 |
| `sensing_distanceto` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:91 |
| `sensing_distancetomenu` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:91 |
| `sensing_keyoptions` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:94 |
| `sensing_keypressed` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:93 |
| `sensing_loud` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:99 |
| `sensing_loudness` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:99 |
| `sensing_mousedown` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:96 |
| `sensing_mousex` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:96 |
| `sensing_mousey` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:97 |
| `sensing_of` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:102 |
| `sensing_of_object_menu` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:101 |
| `sensing_resettimer` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:100 |
| `sensing_setdragmode` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:98 |
| `sensing_timer` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:100 |
| `sensing_touchingcolor` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:89 |
| `sensing_touchingobject` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:87 |
| `sensing_touchingobjectmenu` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:88 |
| `sensing_userid` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:106 |
| `sensing_username` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:105 |
| `sound_changeeffectby` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:85 |
| `sound_changevolumeby` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:86 |
| `sound_cleareffects` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:86 |
| `sound_play` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:82 |
| `sound_playuntildone` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:82 |
| `sound_seteffectto` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:84 |
| `sound_setvolumeto` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:87 |
| `sound_sounds_menu` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:81 |
| `sound_stopallsounds` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:83 |
| `sound_volume` | 1/0 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed_vertical.js:87 |

### Colors

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `colour_blend` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:7 |
| `colour_picker` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:6 |
| `colour_random` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:6 |
| `colour_rgb` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:6 |

### Control, motors and expansion boards

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `drv8830.speed` | 0/0 | 1/1 | — | — | — | — | — | W:blockly/kidbright/blocks.js:3016 |
| `drv8833.move` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/plugins/formula_kid/drv8833/blocks.js:1 |
| `drv8833.speed_number` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/plugins/formula_kid/drv8833/blocks.js:52 |
| `drv8833.stop` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/plugins/formula_kid/drv8833/blocks.js:38 |
| `filter.ema` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/pid_control/filter/blocks.js:1 |
| `filter.sma` | 1/1 | 0/0 | — | มี | — | sma | — | K:kbide/plugins/pid_control/filter/blocks.js:22 |
| `ikb1_analog_read` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:30 |
| `ikb1_digital_read` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:1 |
| `ikb1_digital_write` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:13 |
| `ikb1_motor` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:42 |
| `ikb1_motor_backward` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:245 |
| `ikb1_motor_backward2` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:301 |
| `ikb1_motor_forward` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:219 |
| `ikb1_motor_forward2` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:271 |
| `ikb1_motor_spin_left` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:383 |
| `ikb1_motor_spin_right` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:409 |
| `ikb1_motor_stop` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:435 |
| `ikb1_motor_turn_left` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:331 |
| `ikb1_motor_turn_right` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:357 |
| `ikb1_serial_available` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:143 |
| `ikb1_serial_config` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:101 |
| `ikb1_serial_read` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:167 |
| `ikb1_serial_read_line` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:194 |
| `ikb1_serial_read_one_byte` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:155 |
| `ikb1_serial_read_string` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:182 |
| `ikb1_serial_read_until` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:206 |
| `ikb1_serial_write` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:115 |
| `ikb1_serial_write_line` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:129 |
| `ikb1_servo` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:63 |
| `ikb1_servo2` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ikb_1_plugin/ikb_1/blocks.js:80 |
| `joystick_1.calibrate_set` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/formula_kid/joystick/blocks.js:27 |
| `joystick_1.position` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/plugins/formula_kid/joystick/blocks.js:1 |
| `joystick_2.calibrate_set` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/formula_kid/joystick/blocks.js:72 |
| `joystick_2.position` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/plugins/formula_kid/joystick/blocks.js:14 |
| `joystick.downmost_default` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/formula_kid/joystick/blocks.js:135 |
| `joystick.leftmost_default` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/formula_kid/joystick/blocks.js:169 |
| `joystick.release_default` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/formula_kid/joystick/blocks.js:118 |
| `joystick.rightmost_default` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/formula_kid/joystick/blocks.js:186 |
| `joystick.upmost_default` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/formula_kid/joystick/blocks.js:152 |
| `mjoystick.calibrate_set_x` | 1/1 | 0/0 | — | มี | — | joystick | — | K:kbide/plugins/pid_control/mjoystick/blocks.js:41 |
| `mjoystick.calibrate_set_y` | 1/1 | 0/0 | — | มี | — | joystick | — | K:kbide/plugins/pid_control/mjoystick/blocks.js:109 |
| `mjoystick.downmost_default` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/mjoystick/blocks.js:258 |
| `mjoystick.get_position` | 1/1 | 0/0 | — | มี | — | joystick | — | K:kbide/plugins/pid_control/mjoystick/blocks.js:20 |
| `mjoystick.get_raw_position` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/mjoystick/blocks.js:1 |
| `mjoystick.leftmost_default` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/mjoystick/blocks.js:292 |
| `mjoystick.range_downmost_default` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/mjoystick/blocks.js:360 |
| `mjoystick.range_leftmost_default` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/mjoystick/blocks.js:326 |
| `mjoystick.range_rightmost_default` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/mjoystick/blocks.js:343 |
| `mjoystick.range_upmost_default` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/mjoystick/blocks.js:377 |
| `mjoystick.release_default` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/mjoystick/blocks.js:241 |
| `mjoystick.rightmost_default` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/mjoystick/blocks.js:309 |
| `mjoystick.upmost_default` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/mjoystick/blocks.js:275 |
| `myservo` | 1/1 | 1/1 | มี | มี | — | — | — | K:kbide/plugins/myservo/myservo/blocks.js:1 |
| `myservo_calibrate` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/plugins/myservo/myservo/blocks.js:17 |
| `pid_con.run` | 1/1 | 0/0 | — | — | — | pid | — | K:kbide/plugins/pid_control/pid_con/blocks.js:1 |
| `pid_con.run2` | 1/1 | 0/0 | — | — | — | pid | — | K:kbide/plugins/pid_control/pid_con/blocks.js:36 |
| `pid_con.run3` | 1/1 | 0/0 | — | มี | — | pid | — | K:kbide/plugins/pid_control/pid_con/blocks.js:76 |
| `pwm_bdc.speed` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/pwm_bdc/blocks.js:1 |
| `pwm_bdc.stop` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/pwm_bdc/blocks.js:34 |
| `pwm_bldc.drive` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/pwm_bldc/blocks.js:1 |
| `pwm_bldc.stop` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/pwm_bldc/blocks.js:26 |
| `pwm_gen.drive` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/pid_control/pwm_gen/blocks.js:1 |
| `pwm_gen.stop` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/pid_control/pwm_gen/blocks.js:41 |
| `quad_encoder.get_direction` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/pid_control/quad_encoder/blocks.js:34 |
| `quad_encoder.get_position` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/pid_control/quad_encoder/blocks.js:1 |
| `quad_encoder.get_speed` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/quad_encoder/blocks.js:67 |

### Digital I/O and waits

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `get_gpio` | 0/0 | 1/1 | — | — | — | — | — | W:blockly/kidbright/blocks.js:2418 |
| `input_read` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1543 |
| `logic_sw1_pressed` | 1/1 | 1/1 | มี | มี | — | — | button_buzzer | K:kbide/app/public/blockly/kidbright/blocks.js:271 |
| `logic_sw1_released` | 1/1 | 1/1 | มี | — | — | — | button_buzzer | K:kbide/app/public/blockly/kidbright/blocks.js:285 |
| `logic_sw2_pressed` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:299 |
| `logic_sw2_released` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:313 |
| `mcp23s17_16in.input_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/gpio/mcp23s17_16in/blocks.js:114 |
| `mcp23s17_16in.read` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/plugins/gpio/mcp23s17_16in/blocks.js:42 |
| `mcp23s17_16in.read_input` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/gpio/mcp23s17_16in/blocks.js:79 |
| `mcp23s17_16out.output_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/gpio/mcp23s17_16out/blocks.js:124 |
| `mcp23s17_16out.status_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/gpio/mcp23s17_16out/blocks.js:141 |
| `mcp23s17_16out.write` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/plugins/gpio/mcp23s17_16out/blocks.js:42 |
| `mcp23s17_16out.write_output` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/gpio/mcp23s17_16out/blocks.js:83 |
| `output_read` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1476 |
| `output_toggle` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1457 |
| `output_write` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1434 |
| `pulse_cap.read` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/gpio/pulse_cap/blocks.js:1 |
| `pulse_cap.timeout_ms_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/gpio/pulse_cap/blocks.js:74 |
| `pulse_gen.width_us_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/gpio/pulse_gen/blocks.js:60 |
| `pulse_gen.write` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/gpio/pulse_gen/blocks.js:1 |
| `sensor_switch1` | 1/1 | 1/1 | มี | — | — | — | button_buzzer | K:kbide/app/public/blockly/kidbright/blocks.js:1240 |
| `sensor_switch2` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1254 |
| `set_gpio` | 0/0 | 1/1 | — | — | — | — | — | W:blockly/kidbright/blocks.js:2401 |
| `spec_ios.get_analog_raw` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks-specio.js:4 |
| `specialoutput_write` | 0/0 | 1/1 | — | — | — | — | — | W:blockly/kidbright/blocks-specio.js:23 |
| `usbsw_read` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1528 |
| `usbsw_toggle` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1514 |
| `usbsw_write` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1495 |
| `wait_sw1_pressed` | 1/1 | 1/1 | มี | มี | — | — | button_buzzer | K:kbide/app/public/blockly/kidbright/blocks.js:373 |
| `wait_sw1_released` | 1/1 | 1/1 | มี | — | — | — | button_buzzer | K:kbide/app/public/blockly/kidbright/blocks.js:386 |
| `wait_sw2_pressed` | 1/1 | 1/1 | มี | มี | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:399 |
| `wait_sw2_released` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:412 |

### Displays and lighting

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `basic_led16x8` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:8 |
| `basic_led16x8_2chars` | 1/1 | 1/1 | มี | มี | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:88 |
| `basic_led16x8_clr` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:76 |
| `basic_led16x8_scroll` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:114 |
| `basic_led16x8_scroll_when_ready` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:126 |
| `blink.start` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/plugins/examples/blink/blocks.js:1 |
| `blink.stop` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/plugins/examples/blink/blocks.js:14 |
| `lcd_i2c_20x4.backlight` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/plugins/display/lcd_i2c/blocks.js:46 |
| `lcd_i2c_20x4.clear` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/plugins/display/lcd_i2c/blocks.js:29 |
| `lcd_i2c_20x4.column_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/display/lcd_i2c/blocks.js:162 |
| `lcd_i2c_20x4.number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/display/lcd_i2c/blocks.js:215 |
| `lcd_i2c_20x4.print` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/plugins/display/lcd_i2c/blocks.js:67 |
| `lcd_i2c_20x4.print_prec` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/plugins/display/lcd_i2c/blocks.js:112 |
| `lcd_i2c_20x4.row_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/display/lcd_i2c/blocks.js:179 |
| `lcd_i2c_20x4.string_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/display/lcd_i2c/blocks.js:196 |
| `lcd_spi_20x4.backlight` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/plugins/display/lcd_spi/blocks.js:68 |
| `lcd_spi_20x4.clear` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/plugins/display/lcd_spi/blocks.js:51 |
| `lcd_spi_20x4.column_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/display/lcd_spi/blocks.js:184 |
| `lcd_spi_20x4.number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/display/lcd_spi/blocks.js:237 |
| `lcd_spi_20x4.print` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/plugins/display/lcd_spi/blocks.js:89 |
| `lcd_spi_20x4.print_prec` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/plugins/display/lcd_spi/blocks.js:134 |
| `lcd_spi_20x4.row_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/display/lcd_spi/blocks.js:201 |
| `lcd_spi_20x4.string_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/display/lcd_spi/blocks.js:218 |
| `logic_led16x8_scroll_ready` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:258 |
| `oled_128x64_i2c.clear` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/oled/oled_128x64_i2c/blocks.js:27 |
| `oled_128x64_i2c.column_number` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/oled/oled_128x64_i2c/blocks.js:248 |
| `oled_128x64_i2c.number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/oled/oled_128x64_i2c/blocks.js:301 |
| `oled_128x64_i2c.putstr` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/oled/oled_128x64_i2c/blocks.js:47 |
| `oled_128x64_i2c.putstr_big` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/oled/oled_128x64_i2c/blocks.js:147 |
| `oled_128x64_i2c.putstr_big_prec` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/oled/oled_128x64_i2c/blocks.js:195 |
| `oled_128x64_i2c.putstr_prec` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/oled/oled_128x64_i2c/blocks.js:95 |
| `oled_128x64_i2c.row_number` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/oled/oled_128x64_i2c/blocks.js:265 |
| `oled_128x64_i2c.string_number` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/oled/oled_128x64_i2c/blocks.js:282 |
| `wait_led_matrix_ready` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:361 |
| `ws2812b_rmt.color_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/led/ws2812b_rmt/blocks.js:152 |
| `ws2812b_rmt.get_pixel_num` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/led/ws2812b_rmt/blocks.js:19 |
| `ws2812b_rmt.pixel_color_picker` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/led/ws2812b_rmt/blocks.js:95 |
| `ws2812b_rmt.pixel_rgb` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/led/ws2812b_rmt/blocks.js:35 |
| `ws2812b_rmt.position_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/led/ws2812b_rmt/blocks.js:135 |
| `ws2812b_rmt.set_pixel_num` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/led/ws2812b_rmt/blocks.js:1 |

### Functions and procedures

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `call_function` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-function.js:25 |
| `function` | 1/1 | 1/1 | มี | — | 22 จุด | — | — | K:kbide/app/public/blockly/kidbright/blocks-function.js:13 |
| `procedures_callnoreturn` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:108 |
| `procedures_callreturn` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:117 |
| `procedures_defnoreturn` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:89 |
| `procedures_defreturn` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:100 |
| `procedures_ifreturn` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:119 |

### Generator helpers or unclassified IDs

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `resetTaskNumber` | 0/0 | 0/0 | — | — | — | — | — | — |

### Lists

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `list_get_index` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-array.js:146 |
| `list_get_length` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-array.js:133 |
| `list_get_text` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-array.js:265 |
| `list_insert_first` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-array.js:194 |
| `list_insert_index` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-array.js:238 |
| `list_insert_last` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-array.js:216 |
| `list_set_index` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-array.js:167 |
| `lists_create_empty` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:9 |
| `lists_create_with` | 2/2 | 2/2 | มี | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:12 |
| `lists_getIndex` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:20 |
| `lists_getSublist` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:33 |
| `lists_indexOf` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:18 |
| `lists_isEmpty` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:10 |
| `lists_length` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:10 |
| `lists_repeat` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:9 |
| `lists_reverse` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:9 |
| `lists_setIndex` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:27 |
| `lists_sort` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:37 |
| `lists_split` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:39 |

### Logic and conditions

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `controls_if` | 1/2 | 1/2 | มี | มี | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:42 |
| `controls_ifelse` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:43 |
| `logic_boolean` | 1/2 | 1/2 | มี | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:42 |
| `logic_compare` | 1/2 | 1/2 | มี | มี | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:44 |
| `logic_negate` | 1/2 | 1/2 | มี | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:45 |
| `logic_null` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:46 |
| `logic_operation` | 1/2 | 1/2 | มี | มี | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:45 |
| `logic_ternary` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:46 |

### Loops, delays and tasks

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `advance_task` | 1/1 | 1/1 | — | มี | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1568 |
| `basic_delay` | 1/1 | 1/1 | มี | มี | 2 จุด | delay | button_buzzer, temperature_serial | K:kbide/app/public/blockly/kidbright/blocks.js:138 |
| `basic_forever` | 1/1 | 1/1 | มี | มี | — | — | button_buzzer, temperature_serial | K:kbide/app/public/blockly/kidbright/blocks.js:152 |
| `controls_flow_statements` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:65 |
| `controls_for` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:63 |
| `controls_forEach` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:64 |
| `controls_repeat` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:61 |
| `controls_repeat_ext` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:61 |
| `controls_whileUntil` | 1/2 | 1/2 | มี | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:62 |
| `loop_break` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:332 |
| `loop_continue` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:344 |
| `ms_delay` | 1/1 | 0/0 | — | มี | 2 จุด | delay | — | K:kbide/plugins/pid_control/pid_con/blocks.js:116 |

### Math and constants

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `convert_ra_de` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-math.js:178 |
| `math_arithmetic` | 2/2 | 2/2 | มี | มี | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:72 |
| `math_change` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:76 |
| `math_constant` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:75 |
| `math_constrain` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:80 |
| `math_modulo` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:79 |
| `math_number` | 2/2 | 2/2 | มี | มี | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:72 |
| `math_number_property` | 2/2 | 2/2 | มี | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:75 |
| `math_on_list` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:78 |
| `math_pow` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-math.js:123 |
| `math_random_float` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:81 |
| `math_random_int` | 2/2 | 2/2 | มี | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:80 |
| `math_round` | 2/2 | 2/2 | มี | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:77 |
| `math_single` | 2/2 | 2/2 | มี | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:73 |
| `math_trig` | 2/2 | 2/2 | มี | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:74 |
| `mathplus_acos` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1581 |
| `mathplus_asin` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1594 |
| `mathplus_atan` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1607 |
| `mathplus_atan2` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1620 |
| `mathplus_ceil` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1813 |
| `mathplus_cos` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1636 |
| `mathplus_cosh` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1649 |
| `mathplus_exp` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1714 |
| `mathplus_fabs` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1826 |
| `mathplus_floor` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1839 |
| `mathplus_fmod` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1852 |
| `mathplus_ldexp` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1742 |
| `mathplus_log` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1758 |
| `mathplus_log10` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1771 |
| `mathplus_pow` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1784 |
| `mathplus_sin` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1662 |
| `mathplus_sinh` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1675 |
| `mathplus_sqrt` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1800 |
| `mathplus_tan` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1688 |
| `mathplus_tanh` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1701 |
| `spec_math_number` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/pid_con/blocks.js:130 |

### Music and audio

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `music_get_volume` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1193 |
| `music_note` | 1/1 | 1/1 | มี | มี | — | — | button_buzzer | K:kbide/app/public/blockly/kidbright/blocks.js:463 |
| `music_rest` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:522 |
| `music_scale` | 1/1 | 1/1 | มี | — | 4 จุด | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1111 |
| `music_set_tempo` | 0/1 | 0/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/generators.js:350 |
| `music_set_volume` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1177 |

### Networking, cloud and IoT

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `ble_uart.on_data` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/bluetooth/ble_uart/blocks.js:1 |
| `ble_uart.read_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/bluetooth/ble_uart/blocks.js:28 |
| `ble_uart.read_text` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/bluetooth/ble_uart/blocks.js:15 |
| `ble_uart.send` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/bluetooth/ble_uart/blocks.js:41 |
| `ble_uart.text_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/bluetooth/ble_uart/blocks.js:64 |
| `bt_get_data` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks-bt.js:54 |
| `bt_get_password` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks-bt.js:29 |
| `bt_on_auth` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks-bt.js:15 |
| `bt_on_data` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks-bt.js:40 |
| `bt_send_data` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks-bt.js:65 |
| `bt_setup` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks-bt.js:1 |
| `esp_now.get_mac` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/plugins/esp_now/esp_now_class/blocks.js:1 |
| `esp_now.read_number` | 1/1 | 1/1 | มี | มี | — | ascii | — | K:kbide/plugins/esp_now/esp_now_class/blocks.js:94 |
| `esp_now.read_string` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/plugins/esp_now/esp_now_class/blocks.js:82 |
| `esp_now.recv` | 1/1 | 1/1 | มี | มี | — | — | — | K:kbide/plugins/esp_now/esp_now_class/blocks.js:63 |
| `esp_now.send` | 1/1 | 1/1 | มี | — | — | ascii | — | K:kbide/plugins/esp_now/esp_now_class/blocks.js:13 |
| `esp_now.send2` | 1/1 | 1/1 | มี | มี | — | ascii | — | K:kbide/plugins/esp_now/esp_now_class/blocks.js:36 |
| `feed_color` | 1/1 | 2/2 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-iot.js:433 |
| `feed_iot` | 1/1 | 2/2 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-iot.js:209 |
| `feed_main_title` | 1/1 | 2/2 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-iot.js:343 |
| `feed_minmax` | 1/1 | 2/2 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-iot.js:499 |
| `feed_title` | 1/1 | 2/2 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-iot.js:277 |
| `feed_unit` | 1/1 | 2/2 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-iot.js:367 |
| `FIR.begin` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/FIR/fir/blocks.js:7 |
| `FIR.concat` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/FIR/fir/blocks.js:216 |
| `FIR.convert` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/FIR/fir/blocks.js:184 |
| `FIR.delete` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/FIR/fir/blocks.js:87 |
| `FIR.get_string` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/FIR/fir/blocks.js:28 |
| `FIR.onvalue` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/FIR/fir/blocks.js:132 |
| `FIR.set` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/FIR/fir/blocks.js:55 |
| `FIR.value_text` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/FIR/fir/blocks.js:153 |
| `gauge_color` | 1/1 | 2/2 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-iot.js:119 |
| `gauge_iot` | 1/1 | 2/2 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-iot.js:6 |
| `gauge_minmax` | 1/1 | 2/2 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-iot.js:157 |
| `gauge_title` | 1/1 | 2/2 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-iot.js:46 |
| `gauge_unit` | 1/1 | 2/2 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-iot.js:84 |
| `ifttt_webhooks` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks-ifttt.js:1 |
| `linenotify_send_notifications` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks-line.js:15 |
| `linenotify_set_access_token` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks-line.js:1 |
| `mqtt_config` | 0/0 | 1/1 | — | — | — | — | — | W:blockly/kidbright/blocks-mqtt.js:52 |
| `mqtt_get_number` | 0/0 | 1/1 | — | — | — | — | — | W:blockly/kidbright/blocks-mqtt.js:213 |
| `mqtt_get_text` | 0/0 | 1/1 | — | — | — | — | — | W:blockly/kidbright/blocks-mqtt.js:229 |
| `mqtt_get_topic` | 0/0 | 1/1 | — | — | — | — | — | W:blockly/kidbright/blocks-mqtt.js:200 |
| `mqtt_get_topic_data` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-mqtt.js:119 |
| `mqtt_is_connect` | 0/0 | 1/1 | — | — | — | — | — | W:blockly/kidbright/blocks-mqtt.js:124 |
| `mqtt_on_connected` | 0/0 | 1/1 | — | — | — | — | — | W:blockly/kidbright/blocks-mqtt.js:101 |
| `mqtt_publish` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-mqtt.js:79 |
| `mqtt_setup` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-mqtt.js:1 |
| `mqtt_start` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-mqtt.js:105 |
| `mqtt_subscribe` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-mqtt.js:51 |
| `netpie_connect` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-netpie.js:5 |
| `netpie_on_connected` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-netpie.js:343 |
| `netpie_on_disconnected` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-netpie.js:372 |
| `netpie_on_receive_message` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-netpie.js:82 |
| `netpie_on_shadow_updated` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-netpie.js:213 |
| `netpie_publish_message` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-netpie.js:151 |
| `netpie_read_entire_shadow` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-netpie.js:312 |
| `netpie_received_message` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-netpie.js:115 |
| `netpie_received_shadow_updated` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-netpie.js:242 |
| `netpie_shadow_variable_defined` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-netpie.js:284 |
| `netpie_subscribe` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-netpie.js:184 |
| `netpie_write_shadow` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-netpie.js:44 |
| `push_notification` | 1/1 | 0/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-iot.js:576 |

### RTC and clock

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `rtc_cal` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1614 |
| `rtc_cal_coarse` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1641 |
| `rtc_get` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1273 |
| `rtc_get_date` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1286 |
| `rtc_get_day` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1338 |
| `rtc_get_hour` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1351 |
| `rtc_get_minute` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1364 |
| `rtc_get_month` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1325 |
| `rtc_get_second` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1377 |
| `rtc_get_time` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1299 |
| `rtc_get_year` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1312 |

### Sensors and analog input

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `ads1115.get_volt` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/ads1115/blocks.js:1 |
| `adsws1_rg.get_rain_gauge_counter` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/adsws1_rg/blocks.js:23 |
| `adsws1_rg.get_rain_gauge_mm` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/adsws1_rg/blocks.js:1 |
| `adsws1_rg.reset_rain_gauge` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/adsws1_rg/blocks.js:45 |
| `adsws1_wd.get_wind_direction_degrees` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/adsws1_wd/blocks.js:1 |
| `adsws1_wd.is_error` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/adsws1_wd/blocks.js:29 |
| `adsws1_ws.get_wind_speed_km_h` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/adsws1_ws/blocks.js:1 |
| `adsws1_ws.get_wind_speed_km_h_text` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/adsws1_ws/blocks.js:20 |
| `ds18b20_i2c.get_temperature` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/science_lab/ds18b20_i2c/blocks.js:1 |
| `hpma115s0.get_error_clear_count` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/hpma115s0/blocks.js:129 |
| `hpma115s0.get_error_count` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/hpma115s0/blocks.js:97 |
| `hpma115s0.getpm10` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/hpma115s0/blocks.js:33 |
| `hpma115s0.getpm2_5` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/hpma115s0/blocks.js:1 |
| `hpma115s0.is_error` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/hpma115s0/blocks.js:65 |
| `mpu6050_acc` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/eiei6050/mpu6050/blocks.js:1 |
| `mpu6050_acc_set_range` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/eiei6050/mpu6050/blocks.js:88 |
| `mpu6050_gyro` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/eiei6050/mpu6050/blocks.js:38 |
| `mpu6050_gyro_set_range` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/eiei6050/mpu6050/blocks.js:127 |
| `mpu6050_temp` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/eiei6050/mpu6050/blocks.js:75 |
| `mpu6050.calibrate` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/imu_sensors/mpu6050/blocks.js:97 |
| `mpu6050.get_acc` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/imu_sensors/mpu6050/blocks.js:33 |
| `mpu6050.get_angle` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/imu_sensors/mpu6050/blocks.js:1 |
| `mpu6050.get_gyr` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/imu_sensors/mpu6050/blocks.js:65 |
| `rd_analog.get_analog_raw` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/analog/rd_analog/blocks.js:1 |
| `rd_analog.get_analog_volt` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/analog/rd_analog/blocks.js:21 |
| `sensor_ldr` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:1226 |
| `sensor_lm73` | 1/1 | 1/1 | มี | — | — | lm73 | temperature_serial | K:kbide/app/public/blockly/kidbright/blocks.js:1212 |
| `sht21.get_humidity` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/sht21/blocks.js:25 |
| `sht21.get_temperature` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/sht21/blocks.js:1 |
| `sht21.is_error` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/sht21/blocks.js:49 |
| `sht31.get_humidity` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/sht31/blocks.js:33 |
| `sht31.get_temperature` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/sht31/blocks.js:1 |
| `sht31.is_error` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/sht31/blocks.js:65 |
| `si7021.get_humidity` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/si7021/blocks.js:25 |
| `si7021.get_temperature` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/si7021/blocks.js:1 |
| `tsl2561.get_illuminance` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/weather_sensors/tsl2561/blocks.js:1 |
| `ultrasonic.data` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ultrasonic/ultrasonic/blocks.js:3 |

### Serial and plotting

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `ser_print` | 1/1 | 0/0 | — | มี | — | — | — | K:kbide/plugins/pid_control/pid_con/blocks.js:143 |
| `serial_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/serial_plotter/serial_plotter/blocks.js:46 |
| `serial_plotter` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/serial_plotter/serial_plotter/blocks.js:1 |
| `serial_topic_number` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/serial_plotter/serial_plotter/blocks.js:25 |

### Text and conversion

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `basic_string` | 1/1 | 1/1 | มี | มี | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:165 |
| `number-to-str` | 0/0 | 1/1 | มี | — | — | — | — | W:blockly/kidbright/blocks.js:1566 |
| `string_charAt` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-string.js:227 |
| `string_compare` | 1/1 | 0/0 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-string.js:253 |
| `string_join` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-string.js:44 |
| `string_length` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-string.js:24 |
| `string_substring` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-string.js:172 |
| `string_with_number` | 1/1 | 1/1 | มี | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-string.js:202 |
| `text` | 1/1 | 1/1 | — | — | 31 จุด | — | — | K:kbide/app/public/blockly/blocks_compressed.js:123 |
| `text_append` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:124 |
| `text_changeCase` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:132 |
| `text_charAt` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:127 |
| `text_count` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:139 |
| `text_getSubstring` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:128 |
| `text_indexOf` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:126 |
| `text_isEmpty` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:125 |
| `text_join` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:123 |
| `text_length` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:125 |
| `text_print` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:134 |
| `text_prompt` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:137 |
| `text_prompt_ext` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:135 |
| `text_replace` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:140 |
| `text_reverse` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:141 |
| `text_trim` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:133 |

### Ultrasonic auxiliary blocks

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `ultrasonic.basic_string` | 0/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ultrasonic/ultrasonic/generators.js:5 |
| `ultrasonic.didEnter` | 0/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ultrasonic/ultrasonic/generators.js:14 |
| `ultrasonic.random-text` | 0/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ultrasonic/ultrasonic/generators.js:1 |
| `ultrasonic.recieved` | 0/1 | 0/0 | — | — | — | — | — | K:kbide/plugins/ultrasonic/ultrasonic/generators.js:9 |

### Variables

| Block ID | K def/gen | W def/gen | Toolbox | Sample | KB candidate | Host subset | Recipe subset | แหล่งอ้างอิงแรก |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `math_variables_get` | 1/1 | 1/1 | มี | มี | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:240 |
| `math_variables_set` | 1/1 | 1/1 | มี | มี | — | — | — | K:kbide/app/public/blockly/kidbright/blocks.js:223 |
| `string_variables_set` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/kidbright/blocks-string.js:7 |
| `variables_get` | 1/1 | 1/1 | — | มี | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:155 |
| `variables_get_dynamic` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:160 |
| `variables_set` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:155 |
| `variables_set_dynamic` | 1/1 | 1/1 | — | — | — | — | — | K:kbide/app/public/blockly/blocks_compressed.js:160 |

## Inventory ฟังก์ชันในไฟล์ตัวอย่างปัจจุบัน

รายการต่อไปเป็น static signature candidates เพื่อช่วยตรวจว่าฟังก์ชันอยู่ที่ไฟล์ใด ไม่ใช่ผล compile หรือการรับรองว่าทุกไฟล์ใช้ได้พร้อมกัน ทั้งยังไม่รวมโค้ดใน fenced Markdown เป็น library ที่พร้อม link

| ไฟล์ | Function candidates | Arduino markers | หลักฐาน build |
| --- | --- | --- | --- |
| `knowledge_base/compatibility/button_buzzer.c` | `app_main`:13 | ไม่พบ | recorded-source-and-dependency-hash-matched |
| `knowledge_base/compatibility/kb_basic.c` | `kb_matrix_clear`:5, `kb_matrix_pixel`:8, `kb_matrix_get`:15, `kb_matrix_shift`:20, `kb_matrix_digit`:33, `kb_matrix_pack`:49, `kb_music_frequency`:60, `kb_music_duration`:65 | ไม่พบ | recorded-source-and-dependency-hash-matched |
| `knowledge_base/compatibility/kb_basic.h` | — | ไม่พบ | recorded-source-and-dependency-hash-matched |
| `knowledge_base/compatibility/kb_core.c` | `clamp`:11, `kb_delay_ms`:15, `kb_lm73_celsius`:22, `kb_pid_step`:28, `kb_sma_step`:47, `kb_joystick_init`:65, `kb_joystick_read`:82, `kb_ascii_format`:90, `kb_ascii_parse`:100 | ไม่พบ | recorded-source-and-dependency-hash-matched |
| `knowledge_base/compatibility/kb_core.h` | — | ไม่พบ | recorded-source-and-dependency-hash-matched |
| `knowledge_base/compatibility/kb_display_idf.c` | `kb_display_open`:3, `kb_display_write`:24, `kb_display_brightness`:29, `kb_display_close`:34 | ไม่พบ | recorded-source-and-dependency-hash-matched |
| `knowledge_base/compatibility/kb_display_idf.h` | — | ไม่พบ | recorded-source-and-dependency-hash-matched |
| `knowledge_base/compatibility/matrix_counter.c` | `app_main`:11 | ไม่พบ | recorded-source-and-dependency-hash-matched |
| `knowledge_base/compatibility/music_scale.c` | `app_main`:11 | ไม่พบ | recorded-source-and-dependency-hash-matched |
| `knowledge_base/compatibility/temperature_serial.c` | `app_main`:13 | ไม่พบ | recorded-source-and-dependency-hash-matched |
| `knowledge_base/sensor_examples/accel_kxtj3.c` | `kxtj3_write_reg`:60, `kxtj3_read_reg`:72, `kxtj3_init`:89, `kxtj3_read`:121, `kxtj3_is_shaking`:150, `kxtj3_get_tilt`:161, `accel_task`:169, `standalone`:186 | ไม่พบ | no-current-hash-matched-build-evidence |
| `knowledge_base/sensor_examples/accel_mc3479.c` | `i2c_master_init`:35, `mc3479_init`:48, `app_main`:59 | ไม่พบ | no-current-hash-matched-build-evidence |
| `knowledge_base/sensor_examples/adc_ldr_external.c` | `adc_calibration_init`:41, `adc_init_all`:83, `adc_read_raw`:109, `adc_read_mv`:135, `ldr_get_raw`:149, `ldr_get_brightness_percent`:155, `ldr_classify`:162, `adc_demo_task`:176, `adc_deinit`:204, `app_main`:234 | ไม่พบ | no-current-hash-matched-build-evidence |
| `knowledge_base/sensor_examples/all_sensors_demo.c` | `gpio_isr_handler`:138, `rows_to_columns_16x8`:156, `matrix_draw`:175, `adc_calibration_init`:192, `kxtj3_write_reg`:226, `kxtj3_read_reg`:232, `display_pattern`:243, `display_two_digits`:256, `display_number`:272, `buzzer_play`:280, `buzzer_stop`:287, `button_task`:301, `sensor_task`:325, `i2c_bus0_init`:389, `matrix_init`:407, `kxtj3_init`:432, `temp_sensor_init`:448, `adc_init_all`:468, `buzzer_init`:489, `gpio_interrupt_init`:515, `app_main`:537 | ไม่พบ | no-current-hash-matched-build-evidence |
| `knowledge_base/sensor_examples/balanced_robot.c` | `readEncoder`:67, `setup`:72, `loop`:129 | พบ | no-current-hash-matched-build-evidence |
| `knowledge_base/sensor_examples/calibrate_balancing_bike.c` | `mpu_write`:52, `mpu_read`:59, `i2c_init`:67, `mpu6050_init`:81, `motor_init`:87, `set_pwm`:124, `drive_motor`:130, `balance_task`:160, `app_main`:194 | ไม่พบ | no-current-hash-matched-build-evidence |
| `knowledge_base/sensor_examples/calibrate_balancing_joystick.c` | `sh1106_write_cmd`:120, `sh1106_init`:126, `sh1106_update`:153, `oled_clear`:168, `oled_draw_pixel`:170, `oled_draw_char`:177, `oled_print`:188, `i2c_master_init`:196, `app_main`:234 | ไม่พบ | no-current-hash-matched-build-evidence |
| `knowledge_base/sensor_examples/fomulakid_receiver.c` | `i2c_init`:80, `matrix_init`:93, `matrix_draw`:102, `motor_init`:115, `pct_to_duty`:148, `motor_set`:155, `motor_stop`:163, `motor_forward`:168, `motor_backward`:175, `motor_turn_left`:181, `motor_turn_right`:187, `espnow_recv_cb`:195, `wifi_espnow_init`:204, `app_main`:226 | ไม่พบ | no-current-hash-matched-build-evidence |
| `knowledge_base/sensor_examples/formulakid_sender.c` | `rc_timing_isr_handler`:50, `i2c_init_bus0`:76, `matrix_init`:91, `matrix_draw`:105, `wifi_init`:116, `espnow_send_cb`:128, `espnow_init`:138, `read_rc_timing`:151, `calculate_joystick_position`:166, `app_main`:190 | ไม่พบ | no-current-hash-matched-build-evidence |
| `knowledge_base/sensor_examples/kidbright32ip_matrix_az_user_passed.c` | `i2c_init_bus0`:31, `matrix_command`:47, `matrix_init`:57, `matrix_draw`:77, `reverse_bits`:98, `get_font_column`:106, `matrix_scroll_text`:119, `app_main`:138 | ไม่พบ | no-current-hash-matched-build-evidence |
| `knowledge_base/sensor_examples/kidbright_full_system_demo.c` | `rows_to_columns_16x8`:83, `matrix_draw`:94, `matrix_init`:107, `temp_sensor_init`:128, `read_lm73`:141, `ldr_init`:153, `read_ldr`:164, `buzzer_init`:171, `play_tone`:193, `servo_init`:207, `servo_set_angle`:228, `relay_init`:237, `gpio_isr_handler`:252, `button_init`:257, `button_task`:280, `sensor_logic_task`:312, `app_main`:340 | ไม่พบ | no-current-hash-matched-build-evidence |
| `knowledge_base/sensor_examples/minibike_receiver.c` | `on_recv`:66, `pwm_motor_init`:74, `pwm_servo_init`:111, `analog_write_motor`:136, `servo_write`:146, `gpio_motor_init`:158, `mpu_init`:176, `mpu_read_angle`:195, `drive_reaction_wheel`:210, `drive_steer`:239, `drive_rear_wheel`:246, `espnow_init`:264, `main_task`:287, `app_main`:322 | ไม่พบ | no-current-hash-matched-build-evidence |
| `knowledge_base/sensor_examples/minibike_sender.c` | `oled_cmd`:142, `oled_init`:148, `oled_flush`:190, `oled_draw_char`:205, `oled_draw_str`:216, `oled_clear_page`:225, `oled_show_line`:231, `map_val`:257, `constrain_val`:263, `abs_val`:270, `on_sent`:275, `adc_init`:283, `gpio_init_pins`:300, `espnow_init`:312, `controller_task`:343, `app_main`:423 | ไม่พบ | no-current-hash-matched-build-evidence |
| `knowledge_base/sensor_examples/temp_lm73.c` | `temp_sensor_init`:35, `lm73_set_resolution_14bit`:74, `lm73_read_11bit`:93, `lm73_read_14bit`:110, `lm73_read_11bit`:127, `app_main`:149 | ไม่พบ | no-current-hash-matched-build-evidence |

## ทำซ้ำและตรวจความสดของรายงาน

```powershell
node scripts/audit-kidbright-coverage.mjs
node scripts/audit-kidbright-coverage.mjs --check
# Optional: compare the original captured files, without running them
node scripts/audit-kidbright-coverage.mjs --verify-source <research-cache>
```

เมื่อ `.md`, system prompt, C helper หรือหลักฐานเปลี่ยน ให้สร้างรายงานใหม่ `--check` ตรวจว่าผลลัพธ์ที่เก็บยังตรงกับไฟล์ปัจจุบัน ไม่ได้ทดสอบภาษา AI หรือฮาร์ดแวร์ ผล inventory นี้มีขอบเขตเฉพาะ source snapshot ที่บันทึกและ static registrations ที่ตรวจพบ จึงไม่ครอบคลุม KidBright ทุกเวอร์ชันหรือ plugins ที่ไม่ได้อยู่ในต้นแบบ
