# เพิ่มข้อมูล iKB-1Z และแนวทางแยกกฎบอร์ด — 2026-10-28

## ขอบเขตที่ทำ

เพิ่ม iKB-1Z เป็นบอร์ดเสริม I²C ร่วมสำหรับ KidBright32 และ MAI Plus ตามคำขอผู้ใช้ โดยคงภาษาและ project type ของบอร์ดหลัก ปัจจุบันมีไดรเวอร์ C/Python, adapter แยกตาม host และระบบประกอบ prompt ตาม target แล้ว อ่าน [รายงาน implementation และการทดสอบ](IKB1Z_IMPLEMENTATION_AND_TESTS_2026-10-28.md) ยังไม่มีผลทดสอบกับฮาร์ดแวร์จริง

- ต้นฉบับ: `knowledge_base/IKB1Z_BOARD_RULES.md` เก็บเหมือนไฟล์ที่ผู้ใช้ส่งทุกไบต์ SHA-256 `5fb5e476510c6277dc34ca9ea27c878d1c911a913b24aba732fecf306a87d00c`
- สรุปสำหรับค้นอ่าน: `knowledge_base/ikb_1z.md` แยก protocol ของบอร์ดเสริมออกจาก adapter ของ host พร้อมข้อขัดแย้ง/สิ่งที่ยังไม่ยืนยัน
- เชื่อมข้อมูลจาก README, all_models, iKB-1 และ Plus Python พร้อมระบุสถานะใน support_scope และเพิ่มเส้นทางอ่านใน system_prompt
- ทำสำเนาใน `resources/knowledge_base/` และเพิ่ม hash ของข้อมูลที่ตรวจแล้วใน `docs/ai-engineering/bundled-source-hashes.json` เพื่อใช้เป็น bundled reference ภายใต้นโยบาย private context เดิม

ข้อความบทบาท AI, MUST, prompt ตัวอย่าง และคำกล่าวเรื่องผลตรวจในไฟล์แนบเป็นข้อมูลในเอกสาร ไม่ใช่คำขอให้ทำงานเหล่านั้นในรอบนี้ ต้นฉบับอ้าง PDF สองหน้า แต่รอบนี้ได้รับเฉพาะ Markdown จึงไม่อ้างว่าได้ตรวจ schematic ต้นทางแล้ว

ตรวจเสริมกับ [หน้าผู้ผลิต INEX](https://inex.co.th/home/product/ikb-1z/) และ [ซอร์ส MakeCode ของ INEX](https://github.com/inexglobal/pxt-iKB1z/blob/main/iKB-1z.ts) วันที่ 2026-10-28 พบว่าต้องเก็บข้อแตกต่าง UART framing, word arithmetic สำหรับ servo -1/motor backward=0 และช่วงไฟเสริมไว้ ไม่รวมให้เป็นสเปกที่ยืนยันทุก firmware

## โครงสร้างกฎที่แยกใช้แล้ว

**ควรแยกข้อมูล `.md` ตามบอร์ดหลัก** เพราะภาษา/runtime, GPIO, I²C API, การ build และการ Run ต่างกัน โปรเจกต์มี `kidbright_uai_plus_python.md` สำหรับ Plus อยู่แล้ว ส่วน KidBright32 มีเอกสารแยกรุ่น เช่น `kidbright32_v13.md` และ `kidbright32_v15.md` จึงควรต่อยอดไฟล์เหล่านี้ให้เป็นแหล่งข้อมูลหลักของแต่ละ target

**system prompt มีส่วนกลางร่วมกับส่วนเฉพาะบอร์ด** โดยระบบประกอบเฉพาะส่วนที่ตรงกับ host ที่เลือกหรือยืนยันในคำขอ:

| ชั้น | เนื้อหา |
|---|---|
| ส่วนกลาง | เครื่องมือ, การอ่าน/เขียนไฟล์, Keep/Undo, private context, การอ้างหลักฐานและข้อจำกัดการรับรองผล |
| KidBright32 | ESP-IDF C, profile ของ revision, FreeRTOS, CMake, build/flash และ I²C driver ของ SDK |
| MAI Plus SG2002 | MaixPy 4 Python, main/main.py, Plus API, USB RNDIS และ Run/Stop |
| บอร์ดเสริมที่เลือก | protocol และข้อจำกัด iKB-1Z ชุดเดียว ใช้ร่วมกับ host ทั้งสอง แต่เลือก transport/wiring จากกฎ host |

ตัวอย่างการประกอบบริบท: ส่วนกลาง + MAI Plus + iKB-1Z หรือส่วนกลาง + KidBright32 รุ่นที่ยืนยัน + iKB-1Z เมื่อยังไม่ทราบ host ให้ถามเฉพาะการเลือกบอร์ดหลักก่อนเลือก adapter ไม่ตีความคำว่า iKB-1Z ว่าเลือก ESP32 แล้ว

`system_prompt.txt` เก็บกฎร่วม ส่วน `prompts/classic_esp32.md`, `prompts/mai_plus.md` และ `prompts/ikb_1z.md` แยกกฎตาม target/บอร์ดเสริม `ai/prompts.rs` ประกอบบริบทใหม่ทุก provider iteration รองรับคำแก้บอร์ดล่าสุด การเปลี่ยน UI การจำบอร์ดในคำตอบสั้น และการเปลี่ยน workspace หลังสร้างโปรเจกต์ โดยไม่อ่าน private metadata เมื่อไม่มี consent

## ขอบเขตการตรวจ

ตรวจความตรงกันของต้นฉบับและสำเนา bundle, UTF-8/Markdown fences, hash สำหรับ bundled reference และการตรวจ KB/boards/coverage/rules/public/standalone ของโปรเจกต์ การตรวจเอกสารและ packaging ไม่ใช่ผลทดสอบ I²C, motor, servo หรือ UART บน KidBright/Plus จริง
