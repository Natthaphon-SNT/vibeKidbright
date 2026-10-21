# ผลตรวจซอร์ส GitHub สำหรับรวม μAI และ μAI Plus ใน vibeKidbright

วันที่ 17 ตุลาคม 2026 — ตรวจซอร์สและข้อความคู่มือแบบอ่านอย่างเดียว ไม่เชื่อมต่อบอร์ดหรือเริ่ม training

## ข้อสรุปที่ใช้พัฒนาต่อได้

ข้อมูล GitHub เติมสัญญา transport, project package, training server และ model format ได้มากกว่าอิมเมจเดิม จึงเริ่มออกแบบและทดสอบ adapter โดยอิงซอร์สจริงได้ แต่ยังไม่ใช่หลักฐานว่า vibeKidbright ส่งโปรแกรมหรือโมเดลไปสองบอร์ดได้แล้ว คงลำดับ KidBright32 ก่อน μAI/Plus ตามแผนเดิม

หลักฐานและ hash อยู่ใน [GitHub reference JSON](research/mai-github-reference-2026-10-17.json) เก็บไฟล์ซอร์ส301ไฟล์ รวม2,120,445bytes ใน cache ที่ไม่รวมในแอป ตรวจ Git blob SHA1 ของทุกไฟล์กับ tree ที่ตรึง commit แล้ว มี SHA256 แยกแต่ละไฟล์ การดาวน์โหลดไม่ได้หมายถึงตรวจทุกบรรทัดครบ และไม่ได้ import/eval ซอร์สต้นฉบับ

| แหล่ง | Snapshot ที่ตรวจ | บทบาท |
|---|---|---|
| [kidbright_MAI_ide](https://github.com/KidBrightAI/kidbright_MAI_ide/tree/7b549397adf90f75d975091a787d5960001edf01) | `7b549397adf90f75d975091a787d5960001edf01`, 9ตุลาคม2026 | ซอร์ส IDE, board descriptors, generators, transport, project/model IO |
| [kidbright_MAI_server](https://github.com/KidBrightAI/kidbright_MAI_server/tree/e4fc663d89a361ff2448ab1d99b2cd1ede12b3db) | `e4fc663d89a361ff2448ab1d99b2cd1ede12b3db`, 17กันยายน2026 | Flask training/conversion server สำหรับหลาย target |
| [องค์กร KidBrightAI](https://github.com/KidBrightAI?tab=repositories) | API พบ public repositories8รายการ | ตรวจรายการและ tree ของ `server`/`front-end` เพิ่ม ไม่อ้างว่าซอร์สเก่าใช้ API ปัจจุบันทั้งหมด |
| [คู่มือ KidBrightAIServerV2](https://drive.google.com/file/d/1wCcq--Cnh3xakmJUCzt5OSaiM4S3xG16/view) | Drive metadata แก้ไข15กุมภาพันธ์2023, PDF1,513,171bytes | คู่มือ training/tunnel รุ่นเก่า อ่านข้อความที่ connector สกัด ไม่ได้ตรวจภาพหรือ hash binary PDF |

`package.json` ของ IDE ระบุ1.2.5 แต่ README ยังเรียก current release1.2.4 จึงใช้ commit เป็นหลัก ไม่ถือว่าตัวเลขนี้เท่ากับเวอร์ชันเว็บที่ deploy หรือ Runtime บนบอร์ด ส่วน repo `kidbright_MAI-plus_server` มี size0ใน metadata รอบนี้ ไม่ใช้เป็นหลักฐานว่ามี server แยกที่พร้อมใช้งาน

## Board adapter และโมเดลต้องแยกตามรุ่นและงาน

| Target | การเชื่อมต่อในซอร์ส | Classification | Object detection | Voice |
|---|---|---|---|---|
| μAI V831, `kidbright-mai` | `web-adb`: ADB sync + shell | default `resnet18`; NCNN INT8 `.bin/.param` | `slim_yolo_v2`; NCNN INT8 | `voice-cnn`; CPU NumPy FP32 `model_cpu.npz` |
| μAI Plus, `kidbright-mai-plus` | `websocket-shell`: WSS5050 | default `mobilenet-100`; `.cvimodel/.mud` | `yolo11n`/`yolo11s`; `.cvimodel/.mud` | `voice-cnn`; `.cvimodel/.mud` ผ่าน NPU runtime |

ตรวจจาก [V831 descriptor](https://github.com/KidBrightAI/kidbright_MAI_ide/blob/7b549397adf90f75d975091a787d5960001edf01/boards/kidbright-mai/index.js), [Plus descriptor](https://github.com/KidBrightAI/kidbright_MAI_ide/blob/7b549397adf90f75d975091a787d5960001edf01/boards/kidbright-mai-plus/index.js) และ [format dispatch](https://github.com/KidBrightAI/kidbright_MAI_ide/blob/7b549397adf90f75d975091a787d5960001edf01/src/engine/model-formats/registry.js)

server เลือก pipeline ตาม board/model: V831 voice ส่งน้ำหนัก FP32ออก `.npz`; V831ภาพ/วัตถุผ่าน ONNX/NCNN/spnntools; Plus ผ่าน ONNXและ tpu-mlir ที่ระบุ `cv181x` ก่อนสร้าง `.cvimodel/.mud` ต้องเก็บ labels, input shape, preprocessing, normalization, audio feature parameters และ runtime version คู่กับโมเดล ไม่เปลี่ยนนามสกุลไฟล์เพื่อย้ายบอร์ด

`.mud` ต้องอ้างชื่อ `.cvimodel` ที่ส่งจริง เพราะ client ตั้งชื่อไฟล์ตาม hash และแก้ `model = ...` ใน metadata ส่วน Plus classifier/detector/voice runtimes ที่ตรวจใช้ `dual_buff=False` รายงานประสิทธิภาพหรือผลบอร์ดจริงใน upstream README เป็นผลของผู้พัฒนาต้นฉบับ ไม่ใช่ผลทดสอบของงานนี้

มี comment เก่าใน `ncnn-int8.js` ที่กล่าวว่าเสียงทั้งสองรุ่นใช้ NumPy ซึ่งขัดกับ `registry.js` และ Plus runtime ปัจจุบัน รายงานนี้อิง control flow และ implementation ที่อ่าน ไม่ยก comment นั้นมาเป็น compatibility rule

## สัญญาส่งโปรแกรมและไฟล์

ทั้งสองบอร์ดใช้ `/root/app` สำหรับโปรแกรมและ libraries และ `/root/model` สำหรับโมเดล ซอร์ส shared upload สร้าง `run.py` จากโค้ดผู้เรียนพร้อม board/plugin libraries ดังนั้น `main/main.py` ของ vibeKidbright ควรเป็นไฟล์ผู้เรียน และ adapter จัดรูปแบบ deployment ให้เอง

Plus เปิด `wss://10.155.55.1:5050` รับ binary PTY output แยกจาก text system response คำขอมี prefix `__SYSTEM__:` แล้วตามด้วย JSON เช่น `cmd=version/stat/upload/download/listdir` การตอบเป็น text frame ที่ครอบ JSON ด้วย `>>> System:` ส่วน upload ใช้ base64 chunk256KiB โหมด `wb` แล้ว `ab` และรอ acknowledgement ตรง path [handler](https://github.com/KidBrightAI/kidbright_MAI_ide/blob/7b549397adf90f75d975091a787d5960001edf01/src/engine/protocols/websocket-shell.js), [service](https://github.com/KidBrightAI/kidbright_MAI_ide/blob/7b549397adf90f75d975091a787d5960001edf01/boards/kidbright-mai-plus/scripts/ws_shell.py)

service snapshot เป็น `ws_shell.py`1.3.0 มี version/features probe, file IO, WiFi และ TCP relay คำขอไม่มี request ID ทั่วไป จึงต้องจัดคิวตามชนิด/path พร้อม timeout และแยก system frame จาก log ไม่ให้ข้อความ stdout ทำให้สถานะงานผิด ทั้งสอง descriptor ใช้ USB VID/PIDเหมือนกัน จึงต้องตรวจ runtime/เลือกบอร์ดเพิ่มเติม

การติดตั้งเป็น Maix App บน Plus แยกจาก Run: ใช้ `/maixapp/apps/<id>` และอาจแก้ autostart/รีสตาร์ต launcher ส่วน Connect ใน reference อาจ sync managed scripts แม้ผู้ใช้ยังไม่ส่งโปรแกรม ใน vibeKidbright ให้ Connect ตรวจอย่างเดียว และย้ายการเปลี่ยน service/startup ไปหน้าเตรียมบอร์ดของครู

## สัญญา training server ที่ตรวจจากโค้ด

| ขั้น | Method/path | ข้อมูลสำคัญ |
|---|---|---|
| ตรวจบริการ | `GET /ping` | `result=pong`, `device`, `backend`, `stage` |
| ส่งโครงการ | `POST /upload` | multipart `project` เป็น ZIP และ `project_id` |
| เริ่มฝึก | `POST /train` | IDEส่ง JSON `project`, `train_config`; handler อ่าน `project` แล้วเปิด worker thread |
| รับความคืบหน้า | `GET /listen` | SSE ที่มี JSON events เช่น epoch/batch/train_end/error |
| แปลงโมเดล | `GET /convert?project_id=...` | server แปลงและตอบผล ต้องตรวจ artifact สำเร็จจริง |
| ดาวน์โหลดจริงใน client | `GET /projects/<id>/output/<filename>` | format class เลือกแต่ละไฟล์ตาม target/task |
| ดาวน์โหลด ZIP ฝั่ง server | `GET /download_model?project_id=...` | `model.zip`; ไม่ใช่เส้นทางหลักของ format downloader ที่อ่าน |

อ้าง [training client](https://github.com/KidBrightAI/kidbright_MAI_ide/blob/7b549397adf90f75d975091a787d5960001edf01/src/store/server.js) และ [server handlers](https://github.com/KidBrightAI/kidbright_MAI_server/blob/e4fc663d89a361ff2448ab1d99b2cd1ede12b3db/main.py#L130) Server stage0–5เป็น none/prepare/train/trained/convert/converted ไม่ใช้ `stage` เพียงอย่างเดียวแทนสถานะงานรายคนในห้องเรียน

Project ZIP ใหม่มี `project.json`, `dataset.json`, dataset ตามชนิดงาน และ `model/` ตาม format ส่วน `project.json` เก็บ board, projectType, labels, graph/trainConfig และข้อมูล workspace จึงต้องมี import converter ที่ตรวจ schema/target ก่อนแปลงเข้า project ของ vibeKidbright [Project IO](https://github.com/KidBrightAI/kidbright_MAI_ide/blob/7b549397adf90f75d975091a787d5960001edf01/src/services/ProjectIOService.js)

## ช่องว่างที่ต้องแก้ก่อน reuse

1. **ยกเลิก training ไม่ตรงกัน:** client เรียก `GET /terminate` แต่ server มี `POST /terminate_training`; handler ใช้ `join()` รอ thread จบ ไม่มี cancellation signal ใน handler จึงต้องทำ job cancellation/timeout พร้อม contract test ไม่แสดงว่ายกเลิกสำเร็จจากการรอจบ
2. **งานหลายคนยังไม่ยืนยัน:** `STAGE`, reporter และ train_task เป็น global ใน server ที่ตรวจ ให้เริ่มหนึ่ง training job ต่อ service instance หรือเพิ่ม job IDs/isolation ก่อนเปิด classroom concurrency
3. **model download มีทางเก่าค้าง:** helper `downloadModel()` เรียก `/download` ที่ไม่มีใน routes snapshot นี้ แต่ flow `convertModel()` ใช้ format downloader `/projects/...` ต้องเลือกเส้นทางที่รองรับจริง ไม่สรุปว่าดาวน์โหลดทั้งหมดเสีย
4. **การยืนยันไฟล์ยังต้องปรับ:** model naming ใช้ MD5 ของ primary blob และ skip บางไฟล์เมื่อขนาดเท่ากัน ให้ manifest ของ vibeKidbright ตรวจ hash ทุกไฟล์ รวม `.param/.mud` และตรวจส่งครบก่อนรัน
5. **อย่ากลืนข้อผิดพลาด:** shared model upload/save/load บางชั้นจับ error แล้วเดินต่อ ต้องให้ deployment ล้มเหลวเมื่อไฟล์โมเดลขาด และไม่ใช้ unknown-format fallback เป็น NCNN อัตโนมัติ
6. **source license ต่างกัน:** server มี GPL-2.0; IDE snapshot ไม่มี LICENSE file และไม่มี package license declaration ขณะที่ vibeKidbright มี MIT ให้แยกตรวจสิทธิ์ก่อนคัดลอกหรือ bundle ซอร์ส ไม่ตีความการเปิดอ่านสาธารณะเป็นสิทธิ์แจกต่อทุกส่วน

## บล็อกและภาษา: สิ่งที่นับได้กับสิ่งที่ยังไม่รับรอง

สแกน TypeScript AST เฉพาะไฟล์ที่ `boards/*/index.js` โหลด ไม่ execute module ผล parse errors0ทั้งสองบอร์ด:

| บอร์ด | block definition IDs ในไฟล์ที่โหลด | Python generator IDs | generator ที่คืน `...` |
|---|---:|---:|---|
| V831 | 51 | 50 | when-switch, RGB, RGB-hex |
| Plus | 59 | 60 | when-switch |

when-switch ถูก comment ออกจาก toolbox ทั้งสองบอร์ด; RGB V831ก็ถูก comment ออก ขณะที่ Plusมี generator RGB และรายการ toolbox แล้ว จึงไม่อ้างว่า placeholder ทั้งหมดเป็นบล็อกที่นักเรียนกดใช้ได้ `text_print` ใช้ generator มาตรฐานนอกชุด local ได้ ส่วนชื่อที่มี generatorแต่ไม่มี local definition ต้องไล่ registrationเพิ่มเติม ตัวเลขนี้ไม่รวมมาตรฐาน Blockly/ทุก plugin และไม่ใช่ความครอบคลุมของ vibeKidbright หรือผลทดสอบฮาร์ดแวร์

[i18n source](https://github.com/KidBrightAI/kidbright_MAI_ide/blob/7b549397adf90f75d975091a787d5960001edf01/src/plugins/i18n.js#L35) ระบุภาษา UIเพียง `th`/`en` ไม่ใช่ทุกภาษา การตอบภาษาตามข้อความล่าสุดของ Vibe Coder เป็นอีกข้อกำหนดหนึ่ง ต้องทดสอบแยกจาก locale ของหน้าแอป และไม่ถือว่าฮาร์ดแวร์แสดงข้อความ Unicodeทุกภาษาได้โดยอัตโนมัติ

## คู่มือ Drive ปี2023ใช้ส่วนใดได้

คู่มืออธิบาย Colab, session token, tunnel, Train/Download และ Save project ZIP ซึ่งมี `model_edgetpu.tflite`, TensorFlow.js `model.json`/shard และ labels ใช้เป็นบทเรียนเรื่อง workflow/ตรวจไฟล์ครบได้ แต่ไม่ใช้เป็น format contract ของ V831/Plus ปัจจุบัน คำว่า Server ver2 ไม่ได้แปลว่าเป็น μAI Plus ไม่มีการนำ token ไปใช้หรือเปิด tunnel ในการตรวจนี้

## งานต่อไปใน vibeKidbright

คง UI/โปรเจกต์/main-only/Keep–Undoร่วมกัน แยก ESP-IDF, V831 ADB และ Plus WSS เป็น board adapters แล้วเพิ่ม TrainingAdapter แยกจากการคุยบอร์ด เริ่มจากตรวจรุ่น/runtime → Hello upload/run/stop → camera/audio/sensor → นำเข้าโมเดลแต่ละ format → training/conversion/cancel ก่อนทดสอบรอบเต็ม

เกณฑ์ผ่านต้องรวมเลือกผิดรุ่น, formatผิด, ส่งไฟล์ขาด, socketหลุด, process error, ยกเลิก training, restart service และบทเรียนจริงบนบอร์ดทั้งสอง ไม่เพิ่ม active profiles จากการมีซอร์ส GitHub อย่างเดียว รายละเอียดการจัดระยะอยู่ใน [แผนรวมบอร์ด](VIBEKIDBRIGHT_UNIFIED_BOARD_ARCHITECTURE_2026-10-17.md)
