# macOS: ติดตั้งแอปได้แต่ Build ไม่ได้

ผู้ใช้ส่งภาพ error `No ESP-IDF Python venv found` ใต้โฟลเดอร์
`$HOME/Library/Application Support/th.nectec.vibekidbright/toolchain/tools/python_env`
ทั้งที่หน้าจอแสดง Ready ไม่ทราบ CPU และประวัติการติดตั้ง SDK ของเครื่องนั้น

## สิ่งที่พบในซอร์ส

- `check_toolchain` เชื่อ `.toolchain_ready` และ `check_esp_idf` ตรวจเพียง framework directory
  จึงขึ้น Ready ได้แม้ Python ไม่มีหรือย้ายมาจาก Windows
- resolver เลือก managed SDK ตัวแรกก่อนตรวจ venv ทำให้ SDK ที่ไม่ครบซ่อน SDK native ที่ใช้งานได้
- การหา Python เลือก environment ใดก็ได้ ไม่ผูกกับ major/minor ของ ESP-IDF
- ไม่ค้นทะเบียน `idf-env.json` และ SDK ที่ติดตั้งตามมาตรฐานเมื่อเปิดแอปจาก Finder
- การสแกน tool directories ไม่ครอบคลุม macOS `CMake.app/Contents/bin`
- เลือกพอร์ตแรกในรายการได้ถึง `/dev/cu.debug-console` ซึ่งเป็น system console
- ตำแหน่ง Application Support มีช่องว่าง ขณะที่
  [Espressif ระบุว่า SDK/project paths ไม่รองรับช่องว่าง](https://docs.espressif.com/projects/esp-idf/en/v5.5.1/esp32/get-started/linux-macos-setup.html)

ข้อสรุปว่าเครื่องผู้ใช้โหลด ZIP ของ Windows เป็นข้อสันนิษฐานจาก layout/error
ไม่ได้ตรวจไฟล์บน Mac เครื่องจริง การแก้จึงครอบคลุมทั้งไม่มี Python, venv เสีย,
ผิด CPU, ผิดเวอร์ชัน และเครื่องที่มี native SDK อยู่แล้ว

## พฤติกรรมหลังแก้

- readiness และ Build ใช้ resolver/validation ชุดเดียวกัน marker ไม่ทำให้ Ready
- อ่านเวอร์ชันจาก `version.txt` หรือ `tools/cmake/version.cmake`
- เลือก venv ที่ตรงกับ ESP-IDF major/minor และ native CPU; ตรวจ executable Mach-O
  ทั้ง thin และ Universal รวมถึง execute permission บน macOS
- ตรวจ Python ด้วย `idf.py --version` ใน bounded subprocess;
  ตรวจ CMake/Ninja/compiler ด้วย `--version` ก่อนแจ้ง Ready บน Mac
- implicit SDK ที่ไม่ครบถูกข้าม เพื่อค้น native SDK จากทะเบียน Espressif หรือ `$HOME/esp`
- explicit custom paths ที่ผิดยังแสดง error ให้แก้ ไม่แอบเปลี่ยน SDK ที่ผู้ใช้เลือก
  ปุ่ม Check Native SDK เป็นการเลือก native SDK ใหม่โดยผู้ใช้ และบันทึกหลังตรวจผ่านเท่านั้น
- อ่าน `export_paths` และ `platform_overrides` จาก `tools/tools.json`
  เพื่อรองรับ CMake.app และ layout ของ compiler ตามเวอร์ชัน SDK
- prepend venv ที่เลือกให้ PATH, ตั้ง IDF/VIRTUAL_ENV และลบ PYTHONHOME/PYTHONPATH ที่อาจตกค้าง
- ตัด system debug-console, Bluetooth Incoming Port และ `/dev/tty.*` ที่ซ้ำออกจากพอร์ต macOS
- ไม่แก้ Windows `pyvenv.cfg` บน Mac และไม่รายงาน repair สำเร็จโดยไม่มี SDK พร้อมใช้

## เครื่องที่ยังไม่มี SDK / ต้องซ่อม

หน้าติดตั้งบน Mac มี Copy macOS Setup ให้ผู้ใช้รันใน Terminal:
ใช้ official Espressif repository รุ่น v5.5.1 และ `idf_tools.py` ติดตั้งเครื่องมือ
สำหรับ ESP32, CMake/Ninja และสร้าง Python environment ใหม่ใน
`$HOME/.vibekidbright/.espressif` ซึ่งแยกจาก toolchain เก่าใน AppData
คำสั่งเลือก native Python 3.9–3.13 สำหรับ Intel/Apple Silicon โดยใช้ `arch`
และบอกวิธีติดตั้ง Xcode Command Line Tools เมื่อขาด prerequisites

แอปไม่เรียกคำสั่งนี้เองเป็น fallback และไม่ได้ปลด integrity gate ของ managed ZIP
ผู้ใช้ต้องเริ่ม setup ใน Terminal และอาจต้องดาวน์โหลด dependency จากอินเทอร์เน็ต
เมื่อสำเร็จกด Check Native SDK แล้วเลือก USB port ของบอร์ด
โปรเจกต์ต้องอยู่ในโฟลเดอร์ที่ไม่มีช่องว่างด้วย

## การตรวจสอบ

- regression fixtures รัน logic ของ Mac ทั้ง `aarch64` และ `x86_64` บน development host:
  marker ลวง, Windows venv, environment ผิด CPU/เวอร์ชัน, compiler ผิด CPU,
  Application Support path, SDK registry เสีย/หาย และ CMake.app platform override
- frontend regression ตรวจว่าหน้า Mac ใช้ native setup/recheck ไม่เรียก download ZIP ของ Windows
- packaged probe `--sdk-self-check` สร้างโปรเจกต์ทดสอบใหม่ใน scratch directory,
  ใช้ตัวสร้าง command เดียวกับ Build และยืนยันว่ามี firmware ESP32 ไม่เป็นไฟล์ว่าง
- workflow นำ DMG ไฟล์เดียวไปติดตั้งบน `macos-15` และ `macos-15-intel`
  แล้วใช้คำสั่ง setup จากหน้าจอจริงและ packaged executable ทำ firmware build

การพัฒนานี้ทำบน Windows ผล fixtures ไม่ใช่หลักฐานว่า build/ติดตั้ง SDK บน Mac จริงผ่าน
ต้องรัน workflow ที่รวมซอร์สนี้ก่อนแจก installer ใหม่ และยังไม่มีผลทดสอบ USB flash
หรือบอร์ดจริงของผู้ใช้
