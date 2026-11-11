# mAi Plus: Python projects and USB Run/Stop — 2026-10-22

สถานะปัจจุบัน: การแก้ Run/Stop รอบแรกด้านล่างยังหยุดเฉพาะ GUI และยังไม่พอสำหรับ
image ที่มี launcher_daemon แยกต่างหาก ดู
[การแก้ runtime 4.12.5/1.24.0 และการตรวจเวอร์ชันใหม่](KIDBRIGHT_UAI_PLUS_RUNTIME_COMPATIBILITY_2026-10-26.md)
ซึ่งเป็นรายละเอียดล่าสุด ยังไม่มีผลผ่านด้านจอบนบอร์ดจริง

พัฒนาต่อในสายปัจจุบัน 3.7.4 ตามคำอนุญาตล่าสุดของผู้ใช้ แยกจาก hotfix 3.7.1
สำหรับ classic KidBright ไม่ได้ย้อนเวอร์ชัน ไม่ได้แก้ hotfix และยังไม่เผยแพร่ release

## สาเหตุที่คำสั่งเดิมทำงานไม่ได้

- System prompt และ support contract ยังสั่งให้เลื่อน Plus ออกไป แม้เลือก Plus แล้ว
- คำแนะนำให้หลีกเลี่ยงบล็อกยังบังคับ ESP-IDF C ซึ่งไม่ใช่ runtime ของ SG2002 Plus
- ตัวสร้างโปรเจกต์สร้างได้เฉพาะ CMake/main.c และตัวเปิดโปรเจกต์ตรวจ CMake เท่านั้น
- บอร์ดที่เลือกใน UI ไม่ได้ถูกส่งเป็นบริบทแยกให้ Vibe Coder
- UI แสดง ADB/logcat สำหรับ Plus แต่ source ของ IDE ต้นทางใช้ WebSocket shell
  ผ่าน USB RNDIS จึงไม่ควรถือว่าเห็นหมายเลข ADB แล้วแปลว่ารัน Python บน Plus ได้

## เทียบเอกสาร

ผู้ใช้ยืนยันเว็บจริงคือ [kidbright-mai.web.app](https://kidbright-mai.web.app/)
ตรวจดาวน์โหลด HTML และ JavaScript ปัจจุบันแบบอ่านอย่างเดียวเมื่อ 2026-10-22 แล้ว
พบว่าตรงกับ snapshot ที่ตรวจไว้ทุก byte; config ของ Plus ใช้ `websocket-shell`
ที่ `wss://10.155.55.1:5050` ตรงกับ transport ที่เพิ่ม หลักฐานอยู่ใน
[live-web.json](research/uai-plus-training-2026-10-22/live-web.json)
การตรวจนี้ไม่ได้รันโค้ดบนบอร์ดหรือเปลี่ยน runtime

ใช้สไลด์อังกฤษล่าสุดตามที่ผู้ใช้ระบุเป็นหลักเมื่อเนื้อหาทับซ้อนกับคู่มือไทยเก่า
เอกสารทั้งหมดเป็นข้อมูลอ้างอิง ไม่ใช่คำสั่งให้ agent เปลี่ยนขอบเขตงานเอง

| แหล่งข้อมูล | สิ่งที่ยืนยันและขอบเขต |
|---|---|
| English training PDF, 151 หน้า, หน้า 3 | SG2002, RAM 256 MB |
| English หน้า 5–13 | USB-OTG, power switch, RNDIS, เลือก Plus และ secure connection |
| English หน้า 16 | จอ 2.4 นิ้ว 640×480; image → draw → display; ตัวอย่างในรูปเป็น Blockly |
| English หน้า 16–17 | canvas 240×240 ในตัวอย่างเป็นขนาด buffer ไม่ใช่ขนาด panel |
| คู่มือไทยเก่า, 89 หน้า | แยก OS, MaixPy, device Runtime และ Upload/run ออกจาก Install at boot |
| Plus generators ที่ commit `7b549397adf90f75d975091a787d5960001edf01` | Python API และ WebSocket upload/stat contract |
| [MaixPy display](https://wiki.sipeed.com/maixpy/doc/en/vision/display.html), [image](https://wiki.sipeed.com/maixpy/doc/en/vision/image_ops.html), [camera](https://wiki.sipeed.com/maixpy/doc/en/vision/camera.html) | ตรวจชื่อ API กับเอกสาร primary source; ไม่ใช่ผลทดสอบบอร์ดจริง |

SHA-256 ของ PDF และตำแหน่งหลักฐานอยู่ใน
[sources.json](research/uai-plus-training-2026-10-22/sources.json)
ไม่ได้แจกจ่าย PDF ทั้งเล่มหรือคัดลอก implementation ของ IDE ต้นทางมาเป็น adapter

## สิ่งที่เพิ่ม

- `create_project_workspace` มี `project_type="kidbright_uai_plus"` และ
  `python_example="display_text_circle"`, `"camera"`, `"camera_overlay"`
- สร้าง `main/main.py` กับ private target metadata ใน workspace ใหม่ ไม่แก้โปรเจกต์เดิม
  ไม่ใส่ CMake/sdkconfig ไม่ใช้ classic board_profile มาทดแทน Plus
- Keep/Undo ครอบคลุมโปรเจกต์ Python และการแก้ไฟล์เหมือนเดิม
- เปิดและคืนโปรเจกต์ Python หลังเปิดแอปใหม่ได้ ไฟล์ helper ใน main สร้าง/เปลี่ยนชื่อ/ลบได้
  เก็บชื่อ entrypoint `main/main.py` ไว้เพื่อให้ Run และการเปิดคืนโปรเจกต์มีเป้าหมายแน่นอน
- ส่ง selected board เป็นข้อมูลแบบจำกัดค่าแยกจากข้อความผู้ใช้ ไม่ส่ง private path เป็นข้อความผู้ใช้
- ปรับ prompt/KB/support scope ให้ Plus เป็นงานปัจจุบัน พร้อมตัวอย่าง Python ที่มีชื่อ API จริง
  Original μAI V831 ยังเป็นคนละ target และไม่ได้เปิดใช้งานผ่านงานนี้
- เปลี่ยน UI ของ Plus เป็น Connect Plus / Run Python / Stop Python ผ่าน native WebSocket
  ไม่ใช้ ADB/logcat หรือ ESP-IDF Build & Flash
- TLS ใช้เฉพาะที่อยู่ USB `10.155.55.1:5050` ยืนยัน SHA-256 certificate ก่อนใช้งานครั้งแรก
  connection ที่ส่งไฟล์ต้องตรงกับ fingerprint ที่ผู้ใช้ยอมรับ; ไม่เปลี่ยนนโยบาย TLS ของ cloud APIs
- Run บันทึกไฟล์ก่อน ตรวจว่าจบ Keep/Undo แล้ว จึงส่ง snapshot ของ main ไปยัง `/tmp` โฟลเดอร์ใหม่
  ตรวจ upload acknowledgement/path/ขนาดไฟล์ก่อนขอรัน `python3`
- แสดง log และ exit code แยกจากสถานะส่งคำสั่ง Stop (ปรับกลไก Stop เมื่อ 2026-10-26 ตามด้านล่าง)
  ถ้าหลุดก่อนรับ exit code ให้รายงานสถานะรันไม่ทราบผล ไม่รายงานว่าสำเร็จ
- ตรวจป้องกัน path links/junctions และจำกัด prototype ที่ 64 ไฟล์ รวม 8 MiB สูงสุดไฟล์ละ 4 MiB
  ส่งเฉพาะ main ไม่ส่ง private metadata หรือไฟล์อื่นที่ root
- ไม่ติดตั้ง boot app ไม่เขียน OS ไม่เปลี่ยน system scripts และไม่มี AI tool สำหรับสั่งรันบอร์ดเอง

## ลองใช้งานใน local

1. เริ่มแอปจาก source ปัจจุบันใหม่ด้วย `npm run tauri dev` เพื่อโหลด Rust backend และ prompt ใหม่
2. เลือก **mAi Plus** แล้วสั่ง Vibe Coder เช่น
   “สร้างโปรเจกต์ใหม่สำหรับ mAi Plus แสดง Hello KidBright กับวงกลมสีแดงบนจอ”
3. เลือกที่เก็บโปรเจกต์ ตรวจ `main/main.py` และกด **Keep**
4. ต่อ USB-OTG เลือกสวิตช์ OTG ตรวจ RNDIS แล้วกด **Connect Plus**
5. ยืนยัน certificate ของบอร์ดที่ต่ออยู่ครั้งแรก แล้วกด **Run Python**
6. ตรวจจอและ terminal ถ้าจะหยุดกด **Stop Python**

ปุ่ม NEW ขณะเลือก Plus สร้างโปรเจกต์ Python ตัวอย่างจอให้ได้เช่นกัน
ตัวอย่างพร้อมอ่านอยู่ใน `samples/kidbright-uai-plus/`
ถ้า image บนบอร์ดไม่รองรับ WebSocket stat/upload contract จะรายงาน error และไม่เปลี่ยน runtime
ทางเลือกสำหรับ image ที่รองรับคือใช้โค้ดเดียวกันใน MaixVision ตามคู่มือของรุ่นนั้น

## ผลตรวจ local และสิ่งที่ยังไม่ยืนยัน

| การตรวจ | ผล |
|---|---|
| Frontend | 191 tests ผ่าน |
| Rust library | 226 tests ผ่าน รวม Plus creation, project operations และ protocol fixtures |
| Python samples | 5 tests ผ่าน: syntax, ขนาด canvas, draw ก่อน show, frame ใหม่, camera cleanup |
| Release frontend build | ผ่าน รวม runtime/bundle/KB/catalog/coverage/rule/public checks |
| Native Windows development build | `cargo build --offline --bin vibe-kidbright` ผ่าน |
| Production UI กับ native IPC fixtures | restore/edit/save/connect/run/stop/new Python project ผ่าน; ไม่มี page error หรือ external request |
| UI ขณะเปิด Vibe Coder | ปุ่ม Plus อยู่ใน toolbar ครบที่ความกว้าง 1280 และ 1536 พิกเซล |

ภาพ [UI ใน fixture](previews/uai-plus-python-workspace.png) และ
[ผล browser probe](research/uai-plus-training-2026-10-22/ui-probe.json)
แสดงการทดสอบส่วน UI เท่านั้น ไม่ใช่ภาพจากบอร์ดจริง
ชุดทดสอบ Python ใช้ fake `maix` ไม่ได้จำลอง driver หรือพิสูจน์ผลของ runtime บนบอร์ด
ไม่ได้เรียก live model/provider ในการตรวจรอบนี้ จึงยังไม่รับรองคุณภาพคำตอบของทุกโมเดล

ยังต้องทดสอบกับ Plus จริง: certificate/WS ของ image ที่ติดตั้ง, MaixPy/runtime version,
ข้อความ/วงกลม/กล้องบนจอ และ Stop/ถอด USB ระหว่างรัน
งานนี้ยังไม่เปิดการฝึกโมเดล touch input หรือการติดตั้งแอปให้เริ่มเมื่อเปิดบอร์ด
warning เดิมเรื่อง native dead code และขนาด Monaco bundle ไม่ทำให้ build ล้มเหลว

## แก้ Run/Stop หลังได้รับ log จากบอร์ด — 2026-10-26

ผู้ใช้ส่ง log ว่า upload และเริ่ม Python ได้ มี Maix multimedia initialization
แต่จอยังแสดงหน้าเดิม และ Stop ด้วย Ctrl+C ตามด้วย WebSocket หลุดก่อนรับ exit code
อาการนี้ยังไม่ใช่ผลผ่านด้านจอ ไม่ควรอธิบายเพียงว่าเชื่อมต่อสำเร็จแล้ว

ตรวจ source ต้นทางที่ commit เดิมพบ `boards/kidbright-mai-plus/main.py`
จัดการปล่อยจอจาก Launcher ก่อน user code และเปิด Launcher กลับเมื่อจบ
adapter เดิมของเราข้ามขั้นนี้ สาเหตุบนบอร์ดจริงยังเป็นข้อสันนิษฐาน เพราะการเปิด
session อ่านสถานะเพิ่มเติมครั้งนี้หมดเวลาระหว่าง WebSocket handshake

- เพิ่ม supervisor ที่เขียนเอง ใช้ Python standard library อยู่เฉพาะ `/tmp`
  หยุดเฉพาะ executable `/maixapp/apps/launcher/launcher` ที่ตรวจ PID/start time แล้ว
  ไม่ใช้ `killall` หรือค้นชื่อแบบ substring เพื่อหยุดโปรแกรมอื่น
- ถ้าพบ app อื่นหรือ run เก่าครองทรัพยากร ให้รายงานและไม่เริ่ม MaixPy ซ้ำ
  ใช้ lock ป้องกันหลาย session เริ่มโปรแกรมพร้อมกัน
- เริ่ม `main.py` ใน process group ของ run นี้ Stop ส่งไฟล์ควบคุมผ่าน system channel
  ไม่พึ่ง Ctrl+C ใน PTY เดิม ส่ง SIGINT → SIGTERM → SIGKILL เฉพาะกลุ่มนี้เมื่อจำเป็น
  คืน Launcher และออก exit marker หลังยืนยันว่า Python จบแล้วเท่านั้น
- ถ้าโปรแกรมติดใน driver แม้ส่ง termination แล้ว ให้รอการจบจริง ไม่รายงานจบสำเร็จ
  การหลุดก่อนรับ exit status ยังเป็น unknown
- แสดง Python/MaixPy package metadata ระหว่าง Run และอ่าน protocol error `message`
  ตาม server contract เพื่อไม่ทำรายละเอียด error หาย
- ไม่แก้ `main.py` ในโปรเจกต์ที่ผู้ใช้กำลังเปิด ไม่ติดตั้ง boot app หรือเปลี่ยน image

ทดสอบ local: Python 16 tests (รวม supervisor 11), Rust ส่วน Plus 8 tests,
UI controls 4 tests ผ่าน การทดสอบใช้ fixtures/mocks ไม่ใช่ผลทดสอบไดรเวอร์จริง
ตรวจ Rust library ทั้งชุด 227 tests ผ่าน และ native Windows development build
(`cargo build --locked --offline --bin vibe-kidbright`) ผ่าน
หลังโหลด backend ใหม่ ควรเริ่มจากบอร์ดที่ไม่มี run เก่าค้าง แล้วทดสอบ
Hello/วงกลม → Stop → หน้า Launcher กลับ → Run ซ้ำ โดยตรวจจอจริงและ exit status
