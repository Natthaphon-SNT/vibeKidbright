# macOS 3.7.1: path และ Clean install SDK

`Application Support` กับ `Application\ Support` อ้างถึงโฟลเดอร์เดียวกันเมื่อเขียนใน shell:
เครื่องหมาย backslash ใช้ escape ช่องว่าง ไม่ใช่ส่วนหนึ่งของชื่อโฟลเดอร์จริง
ค่าที่แอปเก็บใน JSON/PathBuf ต้องใช้ชื่อจริง ไม่เติม shell escaping

`toolchain` คือโฟลเดอร์หลักของชุดเครื่องมือเดิม ไม่ใช่ Python executable
ตัว build ต้องค้นหาต่อไปที่ `tools/python_env/idf<version>_py<version>_env/bin/python`
ข้อความ `No ESP-IDF Python venv found` บอกว่าค้นหา venv ที่ใช้ได้ไม่พบ
ไม่ได้ยืนยันจาก log เพียงอย่างเดียวว่าชื่อผู้ใช้ถูก hardcode หรือเป็นปัญหา escaping

ESP-IDF มีข้อจำกัดเรื่องช่องว่างใน SDK/project paths ตาม
[เอกสาร Espressif](https://docs.espressif.com/projects/esp-idf/en/v5.5.1/esp32/get-started/linux-macos-setup.html)
จึงแยก SDK ออกจาก AppData ใน `Library/Application Support`

## พฤติกรรม Clean install

ในแอป เลือก Setup / Repair ESP-IDF → Copy macOS Setup แล้วรันใน Terminal
คำสั่งสร้างโฟลเดอร์ใหม่ทุกครั้ง:

```text
$HOME/.vibekidbright/macos-sdk/install-<รหัสใหม่>/
├── esp-idf/
└── .espressif/python_env/idf5.5_py<version>_env/bin/python
```

- อ่าน `$HOME` บน Mac ที่รันจริง ไม่ฝังชื่อผู้ใช้จากเครื่องผู้พัฒนา
- ดาวน์โหลด SDK/compiler และสร้าง Python environment ใหม่ ไม่ reuse Python ของชุดเก่า
- สร้าง venv ในตำแหน่งสุดท้ายตั้งแต่แรก ไม่ย้าย venv ข้ามโฟลเดอร์หรือข้ามเครื่อง
- หลัง Python เรียก `idf.py --version` ผ่าน จึงเปลี่ยน `current.json` แบบ atomic
  โดยเก็บเพียงชื่อโฟลเดอร์ย่อย และคำนวณ home ของผู้ใช้บนเครื่องนั้นตอนอ่าน
- หาก setup ล้มเหลว ค่าอ้างอิงเดิมยังอยู่ ชุดเก่าเก็บไว้สำรอง ไม่ลบโปรเจกต์หรือการตั้งค่าอื่น
- กด Check Native SDK เพื่อเลือกชุดใหม่ ตรวจเครื่องมือ แล้วบันทึกทับค่า SDK/tools path เก่า
- ติดตั้งแอปจาก DMG อย่างเดียวไม่ล้าง AppData/SDK; Clean install SDK เป็นขั้นตอนที่ผู้ใช้เริ่มข้างต้น
- ต้องมีอินเทอร์เน็ต, Git, Python native 3.9–3.13 และ SDK/project paths ที่ไม่มีช่องว่าง

เวอร์ชันในแอปคงเป็น 3.7.1 การทดสอบ regression ครอบคลุมสอง home directory,
การปฏิเสธ pointer ที่เป็น absolute path ของเครื่องอื่น และการคงค่าเดิมเมื่อ Python ชุดใหม่เสีย
CI จำลอง SDK เก่าที่ไม่สมบูรณ์ก่อน setup แล้วตรวจ build firmware/เปิดแอปจาก DMG บน Intel และ Apple Silicon
