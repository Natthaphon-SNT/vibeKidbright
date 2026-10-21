# KidBright compatibility pilot

วันที่ตรวจสอบ: 16 ตุลาคม 2026

> รายงานนี้เป็นหลักฐาน pilot เดิมสองงาน ผลขยายตัวอย่างจอ/เพลงและการเทียบรุ่นวันที่17ตุลาคมอยู่ใน [สถานะการพัฒนาปัจจุบัน](KIDBRIGHT_IMPLEMENTATION_STATUS_2026-10-17.md)

เพิ่มทางเดินจากคำสั่งภาษาธรรมชาติไปสู่โปรเจกต์ ESP-IDF C ที่มีโค้ดอ้างอิงแน่นอนแล้ว
รุ่นทดลองนี้ครอบคลุม 2 งาน: กด S1 แล้วส่งเสียง และอ่านอุณหภูมิ LM73 ผ่าน Serial
ทั้งสองงาน build เป็น firmware สำหรับ ESP32 ผ่านด้วย ESP-IDF 5.5.4
ยังไม่มีผลทดสอบกับบอร์ดจริงหรือการเรียกโมเดล AI จริงครบกระบวนการ

## ทดลองใช้ในแอป

เปิดแอปรุ่นที่มีการแก้ไขนี้ โดยรันจากโฟลเดอร์ repository:

```powershell
npm run tauri dev
```

ตัวอย่างคำสั่งสำหรับแชตในแอป (ระบุรุ่นให้ตรงกับบอร์ดที่ใช้จริง):

> สร้างโปรเจกต์ s1_sound สำหรับ KidBright32 v1.3 กด S1 แล้วเล่นเสียง A4
> 440 Hz นาน 500 ms จากนั้นรอปล่อยปุ่มก่อนกดครั้งถัดไป ใช้ตัวอย่าง
> button_buzzer จาก kidbright_reference

หรือ:

> สร้างโปรเจกต์ lm73_serial สำหรับ KidBright32 Rev3.1 อ่าน LM73
> แล้วแสดงอุณหภูมิผ่าน Serial ทุก 500 ms ใช้ตัวอย่าง temperature_serial
> จาก kidbright_reference ถ้าอ่านเซนเซอร์ไม่ได้ให้รายงานข้อผิดพลาด

AI ต้องอ่านข้อมูลอ้างอิง ตรวจ SDK และใช้ `create_project_workspace` สร้างโฟลเดอร์ใหม่
แม้มีโปรเจกต์เปิดอยู่แล้ว ผู้ใช้เลือกโฟลเดอร์แม่ในหน้าต่างเลือกที่จัดเก็บ
ระบบปฏิเสธชื่อโฟลเดอร์ที่มีอยู่แล้ว และสลับ workspace หลังเขียนไฟล์สำเร็จเท่านั้น
หากยกเลิกการสร้าง AI ได้รับคำสั่งให้หยุดงานสร้างครั้งนั้น

หลังสร้างจะมีแถบ **Review new project** พร้อมจำนวนไฟล์และปุ่ม **Keep / Undo**
แสดงในแผงแชตโดยไม่ต้องเอาเมาส์ไปชี้ และไม่ขึ้นกับว่า AI เรียก `write_file` หรือไม่
Keep ยืนยันโปรเจกต์และใช้การแก้ไขที่ค้างอยู่ในโปรเจกต์นั้น; Undo ลบไฟล์ที่ระบบสร้าง
แล้วกลับ workspace ก่อนหน้า หากไม่มีโปรเจกต์ก่อนหน้าจะกลับสู่สถานะยังไม่เปิดโปรเจกต์
หากพบไฟล์ที่ผู้ใช้แก้หรือเพิ่มเอง Undo จะหยุดก่อนลบ ต้องบันทึกหรือจัดเก็บงานเหล่านั้นก่อน
ปุ่มไม่ทำงานระหว่าง AI กำลังทำงานหรือมีการแก้ไขใน editor ที่ยังไม่บันทึก
ต้องเลือก Keep ก่อนสั่ง build, flash หรือคำสั่ง terminal สำหรับโปรเจกต์นี้
สถานะ review เก็บในหน่วยความจำเช่นเดียวกับ diff เดิม จึงใช้กับโปรเจกต์ที่สร้างหลังเปิดแอปรุ่นแก้ไข
ไม่ได้เพิ่ม review ย้อนหลังให้โฟลเดอร์ที่สร้างไปแล้ว และไม่คง review หลังปิดแอป

โครงปัจจุบันมีไฟล์โปรแกรมที่แสดง8ไฟล์ และmetadataภายใน1ไฟล์ที่ `.vibekidbright/project.json`
โฟลเดอร์ภายในไม่แสดงในIDE และตั้งHidden attributeเมื่อสร้างบนWindows รวมเมื่อSave As
ไม่มีการสร้าง `KIDBRIGHT_COMPATIBILITY.md` ที่rootอีก กฎอ้างอิงอยู่ในKBของแอป
โปรเจกต์เก่าที่มีJSON/Markdownสองชื่อเดิมจะซ่อนจากรายการIDE และยังอ่านprofileเดิมได้
ไม่มีการลบหรือย้ายไฟล์เก่าอัตโนมัติ โครงเดิม10ไฟล์จึงเป็นข้อมูลก่อนปรับการแสดงผล
`CMakeLists.txt` สามไฟล์ตั้งค่าที่ root, `main` และ component คนละระดับ
`kb_board.h` กำหนดขาบอร์ด; `kb_core.c/.h` เป็นไลบรารีที่แบ่งใช้ระหว่างตัวอย่าง
JSONภายในบันทึกโปรไฟล์ ส่วนMarkdownอ้างอิงอ่านจากKB ไม่ปะปนกับไฟล์ที่เด็ก/ครูแก้
ไลบรารีกลางมีฟังก์ชัน PID/SMA/joystick/codec รวมอยู่ แม้ตัวอย่างส่งเสียงใช้เพียง delay
จำนวนไฟล์นี้จึงเป็นโครงที่ตั้งใจสร้าง และสามารถลดส่วนที่ไม่ใช้ในงานปรับโครงสร้างภายหลังได้

คำสั่งแก้ไข เช่น “เปลี่ยนเสียงในโปรเจกต์นี้เป็น 880 Hz” ยังคงแก้โปรเจกต์ที่เปิดอยู่
ควรตรวจ diff ก่อนรับการเปลี่ยนแปลงของ AI และ build อีกครั้งหลังปรับโค้ด

## สิ่งที่เพิ่ม

| ส่วน | พฤติกรรม |
|---|---|
| `kidbright_reference` | คืนกฎ รุ่นบอร์ด ตัวอย่าง และไฟล์ C ที่ระบบใช้จริง รองรับทั้ง schema ฝั่ง OpenAI และ Google |
| การสร้างโปรเจกต์ | ส่ง `board_profile` พร้อม `reference_example` เพื่อวางไฟล์อ้างอิงโดยตรง ไม่ต้องให้ AI แต่งไดรเวอร์เริ่มต้นทั้งหมด |
| `.vibekidbright/project.json` | metadataภายในที่ไม่แสดงในรายการไฟล์ เก็บรุ่นบอร์ด/ตัวอย่าง/SDK และอ่านmetadataชื่อเดิมได้เมื่อเปิดโปรเจกต์เก่า |
| `main/kb_board.h` | รวมขาที่เลือกอย่างชัดเจน ไม่เลือกบอร์ดจากชื่อโปรเจกต์เก่า |
| บริบทโปรเจกต์ของ AI | อ่าน profile ปัจจุบันใหม่ทุกครั้งที่เรียก provider รวมหลังเครื่องมือสลับ workspace |
| `components/kb_reference` | แกน C สำหรับ delay, LM73, PID, SMA, joystick และตัวแปลงแพ็กเก็ต ASCII |
| Knowledge base | ทำให้ต้นฉบับกับไฟล์ที่แพ็กในแอปตรงกัน และตรวจความตรงกันใน `build:release` |

โปรไฟล์ที่รองรับใน pilot คือ `kidbright32_v13` และ `kidbright32_rev31`
ทั้งคู่กำหนด S1 เป็น GPIO16 active low, buzzer GPIO13 และ LM73 ที่ SDA4/SCL5 address 0x4D
ขอบเขตการรับรองของโปรไฟล์มีเฉพาะอุปกรณ์เหล่านี้ ต้องตรวจรุ่นบอร์ดและอุปกรณ์จริงก่อนใช้
ไม่มีการเลือกโปรไฟล์ทดแทนให้รุ่นที่ไม่รองรับ

## ความหมายที่รักษาจากต้นแบบ

อ้างอิงการศึกษา [KidBright Desktop และ Simulator](KIDBRIGHT_BLUEPRINT_RESEARCH_2026-10-15.md)
และ [กฎพร้อมรายละเอียดแกน C](../knowledge_base/compatibility/README.md)

- `basic_delay` และ `ms_delay` ใน generator เดิมรับหน่วยวินาที ดังนั้น 0.02 คือ 20 ms
- LM73 แปลงข้อมูล signed MSB-first ด้วย raw /128 รวมอุณหภูมิติดลบ
- PID ใช้เวลาจริง จำกัด integral ±60 และ output ±100 แยก state ของแต่ละตัวควบคุม
- SMA เฉลี่ยช่วงเริ่มต้นก่อนเต็มหน้าต่าง แล้วเลื่อนหน้าต่าง แยก state แต่ละตัวกรอง
- Joystick รองรับการตั้งปลายช่วงกลับทิศ และ dead zone แบบ strict ที่ center ±500
- ปลั๊กอิน ESP-NOW เดิมส่งตัวเลข ASCII `%f` โดยไม่นับ NUL เป็นข้อมูลที่ส่ง
  จึงแยกจากโปรโตคอล binary `int32_t` ของตัวอย่าง custom อย่างชัดเจน

มีส่วนที่ตั้งใจซ่อมพฤติกรรมเดิม: PID ครั้งแรกให้เฉพาะ P, ปฏิเสธ dt ที่ไม่เป็นบวก;
SMA ปฏิเสธหน้าต่างนอก 1..50 และ reset เมื่อเปลี่ยนขนาด; decoder ปฏิเสธแพ็กเก็ตผิดรูป
PID ใช้ double จึงไม่ได้อ้างว่าให้ผล float ตรงกันทุกบิตกับต้นแบบ
ตัวอย่างอุณหภูมิรายงาน I2C error แทนการแสดง 0°C เมื่ออ่านไม่สำเร็จ

ถอดการบังคับกฎ Formula Kid กับทุกโปรเจกต์ออกแล้ว กฎมอเตอร์และโปรโตคอลต้องเลือกให้ตรงงาน
ตัวอย่าง LM73 ใช้ I2C master API ของ SDK 5.5 ต้องไม่ปน legacy I2C ใน firmware เดียวกัน

## ผลตรวจสอบ

| การตรวจ | ผล |
|---|---|
| แกน C บนเครื่อง host, MSVC `/W4 /WX` | ผ่าน 122 checks |
| Rust backend `cargo test --lib --offline` | ดูผลล่าสุดในรายงานการจัดไฟล์โปรเจกต์; รวมการตรวจ Keep/Undo,metadataและfile tree |
| Frontend `npm test` | ดูผลล่าสุดในรายงานการจัดไฟล์โปรเจกต์; รวมการกรองไฟล์ภายในจากแถบ Keep/Undo |
| `npm run build:release` | ผ่านการตรวจ KB, public files, bundle และ build frontend |
| `button_buzzer` / `kidbright32_v13` | build ESP-IDF 5.5.4, target ESP32 ผ่าน |
| `temperature_serial` / `kidbright32_rev31` | build ESP-IDF 5.5.4, target ESP32 ผ่าน |

จำนวน Rust/Frontend เป็นชุดทดสอบของแอปทั้งหมดที่รันครั้งนี้ ไม่ใช่จำนวน tests ที่เพิ่มใหม่
ชุดทดสอบ review ตรวจการสร้างโดยไม่เรียก `write_file`, การเก็บไฟล์โค้ดพร้อมmetadataภายในครบ,
การยกเลิกและคืน workspace, การหยุดเมื่อพบไฟล์ที่แก้หรือเพิ่มเอง, การแยก pending diff
ของแต่ละโปรเจกต์, event ที่มาถึงระหว่างโหลด และการป้องกัน generation/Undo ทำงานซ้อนกัน
แกน C ตรวจค่าที่คาดหวังสำหรับ delay, LM73, PID, SMA, joystick และ ASCII codec
รวม invalid input และ state isolation แต่ไม่ได้รันเทียบกับ generator เดิมแบบ differential testing
firmware สร้างจากตัวส่งออกที่ใช้ฟังก์ชันเดียวกับเครื่องมือ AI และใช้ไฟล์ C ชุดเดียวกัน
ไฟล์ SDK รุ่นที่ทดสอบระบุ 5.5.4 ใน `tools/cmake/version.cmake`

หลักฐานแบบอ่านด้วยโปรแกรม:
[kidbright-pilot-2026-10-16-validation.json](research/kidbright-pilot-2026-10-16-validation.json)
เก็บรุ่น SDK, สถานะ build, ขนาดและ SHA-256 ของ firmware และ source ที่ตรวจ
build directory และ log เต็มอยู่ใต้ `src-tauri/target/` ตาม path ในหลักฐาน ซึ่งเป็นไฟล์ local ที่ไม่ติด Git
ไม่มีการแฟลชบอร์ดจากการทดสอบนี้

## ทำซ้ำโดยไม่เรียก AI

จาก root repository:

```powershell
npm run test:compat
npm test
cargo test --manifest-path src-tauri/Cargo.toml --lib --offline
npm run build:release
```

Windows ต้องมี MSVC C Build Tools สำหรับ host test ส่วน Linux/macOS ใช้ `cc` หรือ `CC`
การทดสอบ firmware ต้องมี ESP-IDF 5.5 และ toolchain ที่ติดตั้งไว้แล้ว
ตัวอย่างต่อไปนี้ใช้ PowerShell และสร้างพื้นที่ใหม่ใน `src-tauri/target`:

```powershell
$pilotRoot = Join-Path (Get-Location) ("src-tauri/target/kidbright-pilot-" + [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds())
New-Item -ItemType Directory -Path $pilotRoot | Out-Null

cargo run --manifest-path src-tauri/Cargo.toml --example export_kidbright_reference --offline -- (Join-Path $pilotRoot "button_buzzer") kidbright32_v13 button_buzzer
cargo run --manifest-path src-tauri/Cargo.toml --example export_kidbright_reference --offline -- (Join-Path $pilotRoot "temperature_serial") kidbright32_rev31 temperature_serial

# ตั้งค่าทั้งสามตัวแปรนี้ให้ตรงกับ SDK และ tools ที่ติดตั้งในเครื่อง
$idfRoot = "D:/path/to/esp-idf-v5.5.4"
$toolsRoot = "D:/path/to/idf-tools"
$sdkPython = "D:/path/to/idf-python/Scripts/python.exe"
python scripts/build-kidbright-examples.py --idf-path $idfRoot --tools-path $toolsRoot --python $sdkPython --examples-root $pilotRoot
```

ตัวส่งออกปฏิเสธโฟลเดอร์ที่มีอยู่แล้ว สคริปต์ build ไม่ติดตั้ง dependency ไม่ดาวน์โหลด SDK
และไม่สั่ง flash ผลลัพธ์อยู่ใน `$pilotRoot/build-results.json` พร้อม log ของแต่ละงาน

## งานที่ยังต้องยืนยัน

ยังไม่รับรองว่าทุกคำสั่ง vibe coding จะเลือก tool และรักษาโค้ดได้ถูกต้องทุกครั้ง
ต้องลองผ่าน UI และผู้ให้บริการ AI จริง รวมการสร้าง/ยกเลิก/แก้โปรเจกต์เดิม
พร้อมเทียบบอร์ดจริงกับโปรแกรม KidBright ที่มีตรรกะเดียวกัน:

1. S1: ตรวจการกดค้าง การปล่อยและกดซ้ำ ความถี่ 440 Hz และระยะเสียง 500 ms
2. LM73: เปรียบเทียบค่าที่สภาวะเดียวกัน ตรวจรอบอ่านและรายงานข้อผิดพลาด I2C
3. ตัวช่วย PID/SMA/joystick/codec: ทดสอบเมื่อผูกกับเซนเซอร์ task และคู่ส่งรับจริง

LED matrix/scroll, encoder, MPU, มอเตอร์, line following, ESP-NOW คู่ส่งรับเต็มชุด,
การนำเข้า Blockly/XML ทั้งโปรแกรม และ µAI ยังไม่มี implementation ครบใน pilot นี้
งานเหล่านี้ต้องเพิ่ม profile, recipe และการตรวจผลตามฮาร์ดแวร์เฉพาะต่อไป
