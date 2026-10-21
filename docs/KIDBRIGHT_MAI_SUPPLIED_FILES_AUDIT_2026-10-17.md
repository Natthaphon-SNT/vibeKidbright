# ตรวจคู่มือและอิมเมจ KidBright AI ที่ผู้ใช้ให้เพิ่ม

วันที่ 17 ตุลาคม 2026 — Asia/Bangkok

ตรวจซอร์ส GitHub เพิ่มหลังอิมเมจแล้ว ดู [ผลตรวจ MAI IDE/server](KIDBRIGHT_MAI_GITHUB_RESEARCH_2026-10-17.md) ซึ่งเติม transport/model/training contracts โดยไม่เปลี่ยนผลตรวจ OS/ABI ของไฟล์แนบ ข้อมูล API ปัจจุบันควรเทียบจาก commit ที่ระบุ ไม่สมมติว่า backend ในอิมเมจเก่าตรงกับ server ปัจจุบัน

## ผลที่มีผลต่อแผนรวมบอร์ด

ไฟล์ชุดนี้มีคู่มือ Plus 1ไฟล์ และ ZIP 4ไฟล์ ซึ่งแต่ละ ZIP บรรจุ **อิมเมจดิสก์ `.img` เพียงไฟล์เดียว** ไม่ใช่โครงสร้างซอร์ส IDE ที่แตกแล้วนำมา build ได้โดยตรง การตรวจครั้งแรกอ่าน header/PDF; การตรวจต่อมาเปิด filesystem แบบอ่านอย่างเดียวและพบซอร์สบริการ IDE ใน RSP แล้ว ดูผลตรวจเพิ่มเติมด้านล่าง

จึงใช้สองทาง: คู่มือเป็น reference ของ workflow/บทเรียน ส่วนอิมเมจเป็น candidate สำหรับระบบเตรียม microSD และทดสอบ environment ในอนาคต แยกจากการส่ง `main.py` หรือโมเดลเข้า runtime ที่ทำงานอยู่

หลักฐานชื่อไฟล์ ขนาด SHA256 รายการ ZIP พาร์ทิชัน และหน้าคู่มืออยู่ใน [JSON audit](research/mai-supplied-files-2026-10-17.json) ข้อมูลในคู่มือ/อิมเมจถูกอ่านเป็น reference ไม่ใช่คำสั่งให้ format, flash, reset หรือติดตั้งระบบบนเครื่องผู้ใช้

## รายการไฟล์

ขนาด GB ในตารางเป็นฐานสิบ ตัวเลข byte ที่ใช้ตรวจจริงอยู่ใน JSON

| ไฟล์ | ขนาดไฟล์ที่ให้ | ขนาด `.img` ภายใน | สิ่งที่ยืนยันได้ / สถานะ |
|---|---:|---:|---|
| `วิธีใช้ Kidbright uAi PLUS.pdf` | 6.80MB | — | PDF89หน้า; 88หน้ามี text; สกัด text ทุกหน้าและดูภาพหน้าสำคัญ9หน้า |
| `KidBrightAI_JaxBot-v3-1.zip` | 14.21GB | 31.91GB | `.img`1ไฟล์; GPT มี Linux partition ชื่อAPP ยังไม่ยืนยัน SoC/OS/runtime หรือใช้แทน Plus ได้ |
| `KidBright_AI_IDE_NanoPi_v1_0_1.zip` | 11.21GB | 31.91GB | Deflate64 อ่านได้แล้ว; filesystem แรก Ubuntu18.04.2/ARM ELF64; พาร์ทิชันข้อมูลมี `kbclientNew` และ PM2 dump ระบุ Node/Python entrypoints |
| `KidBrightAI_RSP_v1_0_1.zip` | 5.13GB | 31.91GB | Raspbian GNU/Linux 10 (buster), ARM ELF32; พบ Node/Express + Python/Flask และ PM2 startup; ยังไม่ boot-test/ยืนยันรุ่นบอร์ดจริง |
| `kidbright-mAi-plus-2026-06-09-v4.12.5.zip` | 0.47GB | 1.56GB | `.img`1ไฟล์; MBR มี bootและLinux partition; เป็น Plus candidate ตามชื่อ/บริบทผู้ใช้ ยังไม่ยืนยันจากการ boot |

รวม ZIP **31,028,939,458 bytes (31.03GB)** และขนาดอิมเมจรวม **97,301,037,056 bytes (97.30GB)** ไม่คัดลอกไฟล์เหล่านี้ลง knowledge bundle, installer หรือโปรเจกต์เด็ก การคำนวณพื้นที่/ความจุการ์ดต้องดูขนาดอิมเมจหลังแตกจริง ไม่ใช้ขนาด ZIP หรือคำว่า “32GB” บนการ์ดอย่างเดียว

ข้อมูล prefix ที่อ่านไม่ได้ยืนยันว่า filesystem สมบูรณ์หรือ boot ได้ Full ZIP SHA256 ใช้ระบุไฟล์ที่ตรวจ ไม่ใช่การยืนยันว่าเป็น release ทางการหรือผ่าน checksum จากผู้ผลิตแล้ว

## คู่มือยืนยันอะไรเพิ่มเติม

เลขหน้าต่อไปนี้เป็น **PDF pageจริง1–89** รวมหน้าปกที่ไม่มีข้อความสกัดได้ จำนวน88ที่พบระหว่างตรวจเป็นจำนวนหน้าที่มีข้อความ ไม่ใช่จำนวนหน้าทั้งเอกสาร

| หน้า PDF | สิ่งที่คู่มือแสดง | สิ่งที่นำไปออกแบบในแอป |
|---|---|---|
| 4,6,10,13 | USB-OTG, RNDIS, Network adapter และ board-local HTTPS5050 | รอ USB network พร้อมก่อนเปิด transport; แยกอาการสาย/driver/network/service |
| 17–18 | Uploadเพื่อรัน กับติดตั้ง applicationให้เริ่มเมื่อเปิดเครื่องเป็นคนละ action | มี “ส่งและรัน” แยกจาก “ติดตั้งเป็นแอป”; ไม่เปลี่ยน startup โดยไม่บอก |
| 23–29 | S1/S2, beep, read/write logical I/O, acceleration/gyro, servo และจอร่วมกล้อง | เป็นรายการบทเรียน/acceptance test ไม่ถือว่ามี driver ของ vibeKidbright แล้ว |
| 33 | Image Classification, Object Detection และ Voice Classification | แยก model task/output schema ตามชนิด |
| 38,64 | ตัวอย่างการเก็บภาพ20ภาพต่อคลาสและ100ภาพต่อวัตถุตามบทเรียน | ใช้เป็น tutorial guidance ไม่รับรองความแม่นยำหรือเป็นขั้นต่ำสากลทุกโมเดล |
| 42,44–56,69–73 | TrainในColab, รับ URL, download และ MODEL IS READY | ต้องมี training adapterและผลลัพธ์โมเดลพร้อม labels/preprocessing ไม่ใช่แค่เรียก LLM |
| 75–81 | ขั้นตอนเตรียม/เขียน microSD ด้วย imageและโปรแกรมภายนอก | เป็น provisioning/recoveryของครู แยกจากงานประจำวันของนักเรียน |
| 82–89 | ต่อWiFi, Install Runtime/Upgrade, ตรวจversion และลองCamera | หลังลงOSยังต้องตรวจ Runtime ก่อนถือว่าบอร์ดพร้อม |

ส่วน Voice Classification ในคู่มือระบุชื่อหมวด แต่ไม่ได้ให้ขั้นตอนละเอียดเท่าภาพ/วัตถุ ไม่เติมจำนวนตัวอย่างหรือ API เสียงจากการคาดเดา

## ต้องแยกเวอร์ชันให้ถูกชั้น

ภาพหน้าตั้งค่าที่ PDF87–88 แสดง:

- System Version: `maixcam-pro-2026-01-13-maixpy-v4.12.5`
- MaixPy Version: `4.12.5`
- Runtime Version หลังติดตั้ง: `1.24.0`

ที่ PDF78 ตัวอย่างไฟล์ OS เป็น `kidbright-mAi-plus-2026-05-02-v4.12.5B1.img` แต่ไฟล์ที่ผู้ใช้ให้เป็น `2026-06-09-v4.12.5.zip` จึงไม่เท่ากับ imageเดียวกันจากชื่อคล้ายกัน ตัวเลขเหล่านี้เป็นตัวอย่างในคู่มือ ไม่ใช่การตรวจเวอร์ชันปัจจุบันบนบอร์ดของผู้ใช้

นอกจากนี้ clientเว็บที่ตรวจรอบก่อนมีเวอร์ชันของ managed scripts เช่น `ws_shell.py` แยกออกมาอีกชั้น จึงควรเก็บอย่างน้อย **image release, OS/ABI, MaixPy, device Runtime, transport/service version และ board-scripts manifest** ใน capability report ไม่ใช้ `4.12.5` ตัวเดียวแทนทั้งหมด

## ข้อเสนอที่เพิ่มใน vibeKidbright

1. **Image Library สำหรับครู:** importไฟล์จากเครื่อง/ดาวน์โหลดเฉพาะรุ่นเมื่อต้องใช้ แสดง targetที่ยืนยัน, release, SHA256, compressed/expanded bytesและที่มา ไม่เพิ่มอิมเมจขนาดใหญ่ใน installerหลัก
2. **Prepare Board Wizard:** แยกเตรียมOS/การ์ด → รอboot/USB network → ตรวจRuntime → ตรวจบริการเชื่อมต่อ → Hello/Camera smoke test สถานะ “มีOS” ยังไม่ใช่ “พร้อมรันงาน”
3. **Runtime Compatibility Matrix:** ผูกprofileกับ image/runtime/serviceที่ทดสอบจริง ถ้าversionไม่ตรงให้แจ้งขั้นที่ขาด ไม่พาไปติดตั้ง ESP-IDF สำหรับ Plus
4. **Daily Workflow:** นักเรียนแก้main ส่งโปรแกรม/โมเดลและStopได้ โดยไม่ต้องเขียนmicroSDใหม่ ทุกอย่างที่เกี่ยวกับOSอยู่ในหน้าครู
5. **Recovery แยกจาก Run:** ก่อนเขียนการ์ดต้องมีหน้าทบทวนอุปกรณ์ปลายทาง ความจุ และรายการที่จะถูกแทนที่; ไม่ใช้ project Keep/Undo อ้างว่าคืนข้อมูลการ์ดทั้งก้อนได้
6. **JaxBot/NanoPi/RSPเป็น reference ก่อน:** ชื่อและimageยังไม่ยืนยันว่าเป็น μAI/V831 หรือ Plus/SG2002 ไม่เพิ่ม profile, GPIO, runtimeหรืออุปกรณ์เหล่านี้ลงเส้นทาง deployโดยอัตโนมัติ

ยังไม่มี implementationของImage Libraryหรือการเขียนการ์ดในรอบนี้ รายละเอียดเพิ่มเข้า [แผนรวมบอร์ด](VIBEKIDBRIGHT_UNIFIED_BOARD_ARCHITECTURE_2026-10-17.md) แล้ว โดยรักษาลำดับ KidBright32ก่อน μAI/Plus

## วิธีตรวจและข้อจำกัด

อ่านต้นฉบับ5ไฟล์โดยไม่แก้ไข Hash ZIP/PDFครบทั้งไฟล์ อ่าน ZIP central directory และ sample1MiBจาก `.img` ที่compressionรองรับ สกัดข้อความ PDFทุกหน้า แล้ว render/viewหน้าที่4,14,23,33,78,79,87,88,89โดยใช้เครื่องมือใน cacheที่ถูกignoreของworkspace

ไม่ได้แตกอิมเมจรวม97GB ไม่ mount filesystem ไม่ format/flash ไม่ติดต่อ private IP ไม่รันโปรแกรมจากZIP และไม่ทดสอบบอร์ดจริง ผล header/PDF เดิมเป็น snapshot ของการตรวจครั้งแรก ส่วนผล filesystem ที่ตรวจต่อมาอยู่ด้านล่าง การพบไฟล์บริการยังไม่ยืนยันว่า dependencies ครบหรือบริการเริ่มทำงานได้

## ผลตรวจ filesystem เพิ่มเติม: RSP

อ่านไฟล์ภายใน ext filesystem ของ Linux partition โดยตรงผ่าน ZIP ไม่บูตอิมเมจ หลักฐานระดับไฟล์และ SHA256 อยู่ใน [filesystem evidence](research/mai-image-filesystems-2026-10-17.json) ข้อมูลนี้เพิ่มจาก [header/PDF audit เดิม](research/mai-supplied-files-2026-10-17.json) ซึ่งเก็บไว้ตามผล ณ เวลานั้น

| ประเด็น | หลักฐานที่อ่านได้ | ขอบเขตที่สรุปได้ |
|---|---|---|
| OS | `etc/os-release`: Raspbian GNU/Linux 10, codename buster | ยืนยัน OS ในอิมเมจ ไม่ระบุ Raspberry Pi รุ่นจากชื่อ RSP |
| ABI | ELF header ของ Python3, bash และ sh: class32, e_machine40 | ARM 32 บิตของ executable ที่ตรวจ ไม่ใช่ V831/SG2002 compatibility test |
| Startup | `etc/systemd/system/pm2-pi.service`: PM2 resurrect ผ่าน Node.js path v10.21.0; PM2 dump ระบุ `kbclientNew/server/index.js` และ `server/kb_2/app3.py` | เป็น startup configuration ที่บันทึกอยู่ ยังไม่ได้เริ่ม service จริง |
| Node service | `server/package.json`: vuenodeproject1.0.0, Express, node-blockly, rosnodejs; `index.js` มี route เช่น `/createProject`, `/runPython3`, `/run`, `/stop`, `/downloadClassifier` | มีซอร์ส backend และแนวทางสัญญา API ให้ศึกษา ไม่ใช่หลักฐานว่า endpoint ใช้ได้หรือปลอดภัยสำหรับ production |
| Python service | `app3.py` import Flask, TensorFlow/Keras, rospy และ speech features; มี route train/log/stop/inference | มีซอร์สส่วน training/inference; ไม่ยืนยันโมเดลที่ใช้ได้จริงหรือผลความแม่นยำ |
| Artifact ที่ขึ้นกับแพลตฟอร์ม | พบชื่อ wheel `tensorflow-2.1.0-cp37-cp37m-linux_armv7l.whl` | เป็นข้อจำกัดของ artifact นี้ ไม่คัดลอกไปเป็น runtime ของ Plus โดยอัตโนมัติ |

### ผลต่อแผนรวมโปรแกรม

RSP เป็น reference เพิ่มของระบบ KidBright AI บน Linux ที่มี Node/Python/ROS ไม่ใช่หลักฐานว่า μAI V831 หรือ μAI Plus SG2002 ใช้ API เดียวกัน หากต้องรองรับ RSP ภายหลัง ให้เพิ่ม adapter แยกหลังยืนยันฮาร์ดแวร์และทดสอบ transport, request/response, log, stop และ model format จริง ไม่เปลี่ยน native adapter ของ μAI/Plus ตามชื่อ ZIP

ใช้ซอร์สนี้เปรียบเทียบ workflow สร้างโปรเจกต์ เก็บข้อมูล ฝึกโมเดล รัน และหยุดงานได้ แต่ก่อนนำซอร์สมาแจกหรือ bundle ต้องตรวจ license และ dependency tree เพิ่ม รอบนี้ยังไม่ตรวจ license ครบ และไม่ได้คัดลอกซอร์สจากอิมเมจเข้าแอป

ตัวอ่านรอบต่อมาใช้ sequential decompression กับ temporary prefix cache จำกัด17GiB แทนการข้าม ZIP ซ้ำไปมา การตรวจ RSP ใช้ prefix ประมาณ11.70GB แล้วลบ cache ชั่วคราวอัตโนมัติ ไม่ใช่การแตกหรือยืนยัน CRC ทั้งอิมเมจ31.91GB

## ผลตรวจ filesystem เพิ่มเติม: NanoPi

ข้อจำกัด Deflate64 method9 ในการตรวจครั้งแรกแก้ด้วย `zipfile-deflate64` แล้ว จึงอ่าน payload และ filesystem ได้ ไม่ต้องแปลงหรือแก้ ZIP ต้นฉบับ

| พาร์ทิชัน | Offset (bytes) | ขนาด (bytes) | สิ่งที่ตรวจพบ |
|---|---:|---:|---|
| 1, type0x83 | 100,663,296 | 6,719,275,008 | ext filesystem, block4096; `etc/os-release` เป็น Ubuntu18.04.2 LTS/Bionic; Python3/bash/sh เป็น ARM ELF64 (e_machine183) |
| 2, type0x83 | 6,819,938,304 | 25,095,028,224 | ext filesystem, block4096; มี `root`/`work`; พบ `root/home/<user>/kbclientNew` และ PM2 startup declarations |

`rc.local` ของ filesystem แรกอ้างถึง FriendlyELEC release generator และมีเงื่อนไขสำหรับชื่อ NanoPi-NEO4 การพบชื่อในเงื่อนไขไม่ได้ยืนยันฮาร์ดแวร์เป้าหมายจริง เพราะ script อาจรองรับหลายรุ่น โฟลเดอร์ `/opt` ที่ตรวจมีตัวอย่างกล้อง/OpenCV/Qt ยังไม่พบ startup ของ KidBright IDE ในรายการ systemd/init.d ที่อ่านจาก filesystem แรก จึงไม่อ้างว่าไม่มี IDE ทั้งอิมเมจ

การตรวจพาร์ทิชันข้อมูลรอบแรกติดขีดจำกัด prefix cache10GiB ต่อมาขยายเป็น32GiBให้ครอบคลุมขนาดอิมเมจ และอ่านโฟลเดอร์ที่ขาดได้แล้ว การตรวจรอบนี้คลาย prefix23,330,422,784bytes (23.33GB) ลงแคชชั่วคราวแล้วลบทิ้ง ไม่ mount หรือรันโปรแกรมในอิมเมจ

สิ่งที่ยืนยันเพิ่มเติม:

- `root/home/<user>/kbclientNew` มี `server`, `nectec-client`, `kidbright_api` และโฟลเดอร์โครงการอื่น เป็นหลักฐานของโครงสร้างแอปในพาร์ทิชันข้อมูล
- `root/home/<user>/.pm2/dump.pm2` ระบุสองงาน: Node `kbclientNew/server/index.js` และ Python3 `kbclientNew/server/kb_2/app3.py`; พบชื่อ `pm2-pi.service` ใน systemd ของพาร์ทิชันข้อมูลด้วย
- พบชื่อ artifact `tensorflow-2.3.0-cp36-cp36m-linux_aarch64.whl` ต่างจาก RSP ที่พบชื่อ2.1.0/cp37/armv7l ชื่อ wheel ไม่ยืนยันเวอร์ชันที่ติดตั้งหรือกำลังรันจริง

**ขอบเขต NanoPi:** อ่าน PM2 dump และ directory entries แล้ว แต่ยังไม่ได้อ่านเนื้อหา `server/package.json`, `index.js` หรือ `app3.py` ของ NanoPi การไล่โฟลเดอร์จำกัด240directories/4ระดับและข้าม dependencies/datasets จึงไม่ใช่การตรวจซอร์สทุกไฟล์ ยังไม่ยืนยันว่าซอร์สหรือ API เท่ากับ RSP และยังไม่ยืนยันวิธีประกอบ active boot root จากสองพาร์ทิชัน

ผลนี้ทำให้การตรวจ ABI จำเป็นชัดเจน: RSP ที่ตรวจเป็น ARM32 ส่วน NanoPi filesystem แรกเป็น ARM64 แม้ทั้งสองใช้ Linux ก็ไม่ควรถ่าย wheel, runtime หรือ driver ข้ามกันโดยไม่ตรวจ และยังไม่ใช่หลักฐานว่าใช้แทน μAI V831 หรือ μAI Plus SG2002 ได้

SHA256 ของ ZIPทั้งสองตรวจซ้ำหลังอ่าน filesystem แล้วตรงกับ full-input audit เดิม หลักฐานเก็บผลตรวจใหม่และข้อจำกัดเดิมที่ถูกแก้แล้วแยกกัน การอ่าน prefix ยังไม่ใช่การตรวจ CRC ครบทั้งอิมเมจหรือการทดสอบ boot
