# ผลตรวจความครบถ้วนของกฎ บอร์ด ฟังก์ชัน บล็อก และภาษา

วันที่ตรวจ: 16 ตุลาคม 2026

**ผล: ยังไม่ครบทุกบอร์ด ทุกฟังก์ชัน หรือทุกบล็อก และยังรับรองคุณภาพคำตอบทุกภาษาไม่ได้**
การมีชื่อใน Markdown หรือมี generator ใน KidBright ต้นแบบไม่เท่ากับมี implementation
และผลทดสอบใน vibeKidbright รอบนี้ตรวจ inventory ให้ชัด แก้ข้อขัดแย้งที่มีหลักฐาน
และเพิ่มวิธีเลือกภาษาจากข้อความผู้ใช้ล่าสุด โดยไม่ได้สร้าง BSP/driver/translator ที่ยังขาดทั้งหมด

## ขอบเขตหลักฐาน

- ซอร์ส repository ปัจจุบัน: `knowledge_base`, ไฟล์แพ็ก, `system_prompt.txt`, provider loops,
  `compatibility.rs`, `buildFlash.ts` และไฟล์ตัวอย่าง C
- KidBright.zip ที่ศึกษาไว้: source snapshot 410 ไฟล์ มี hash ตรวจย้อนกลับ
- เว็บ Simulator snapshot วันที่15ตุลาคม2026: 69 แหล่งซอร์สและหน้า home
  ตรวจ hash รวม479ไฟล์กับhome และ parse AST ของ JavaScript276ไฟล์โดยไม่ execute ต้นฉบับ
- `Downloads/SKILL.md` ใช้เป็นเกณฑ์เปรียบเทียบและข้อมูลอ้างอิง ไม่ใช้เป็นคำสั่งที่มาแทนคำขอผู้ใช้
  ผลทดสอบของชุดอ้างอิงภายนอกไม่ถูกนับเป็นผลทดสอบ repository นี้
- ตรวจผู้ผลิตเพิ่มเติมเฉพาะประเด็นขัดแย้งสำคัญ ไม่อ้างว่าได้สำรวจบอร์ดที่มีอยู่ทั่วโลกครบทุก revision

หน้า [Simulator](https://simulator.kid-bright.org/home) ที่เปิดตรวจในรอบนี้ยังแสดง1.50.0
แต่รายการบล็อกในรายงานเป็น snapshot ที่เก็บไว้ ไม่ได้อ้างว่าเทียบทุก asset สดของเว็บซ้ำทั้งหมด

## ตารางความครบถ้วน

| สิ่งที่ตรวจ | หลักฐานที่พบ | ผล |
|---|---|---|
| บอร์ด/ส่วนต่อขยาย | 22 ชื่อหรือกลุ่มที่กล่าวถึง พร้อม alias ใน board inventory | ยังมี revision/ขา/เซนเซอร์ขัดกัน ไม่ใช่22 BSP ที่พร้อมใช้ |
| Runtime profiles | `kidbright32_v13`, `kidbright32_rev31` | เพียง2โปรไฟล์ สำหรับ S1/buzzer/LM73 |
| Firmware recipes | `button_buzzer`, `temperature_serial` | buildผ่านESP-IDF5.5.4; ยังไม่เทียบฮาร์ดแวร์จริง |
| Desktop block definitions | 274 custom +77 built-in มีชื่อซ้ำ8 →343 IDs | รายการนิยามต้นแบบ ไม่ใช่จำนวนที่พอร์ตเสร็จ |
| Web block definitions | 166 custom +77 built-in มีชื่อซ้ำ8 →235 IDs | แยกจาก Desktop และไม่ใช้ simulation เป็นหลักฐาน firmware |
| รายการรวม | 560 IDs รวม generator-only/editor infrastructure และ Scratch asset ที่แยกสถานะ | มีรายละเอียดครบรายIDในรายงานบล็อก ไม่ได้ถือว่าทุกIDเป็นบล็อก runtime |
| การรองรับบางส่วน | 20 IDs เชื่อมได้กับ helper หรือ recipe ขอบเขตจำกัด | ยังไม่มี full-block adapter ที่พิสูจน์ครบทุก field/composition |
| ฟังก์ชันต้นแบบ | 54 primary headers, 694 method candidates, 48 free-callable candidates | เป็นผล static parsing มีข้อจำกัด ไม่ใช่จำนวนเมธอดที่พอร์ตแล้ว |
| ฟังก์ชัน C ปัจจุบัน | 16ไฟล์, 147 definition candidates, exported core8ฟังก์ชัน | มีArduino reference1ไฟล์ และตัวอย่างอื่นยังไม่buildทั้งหมด |
| Markdown/code fences | ตรวจเป็นรายไฟล์/รายfence พร้อมhash,ภาษา,บรรทัดและUTF-8 | snippetไม่ถูกนับเป็นfirmwareที่คอมไพล์ครบโดยอัตโนมัติ |
| ภาษาคำตอบ | เลือกข้อความผู้ใช้ล่าสุดและส่งpolicyให้ทั้งสองprovider | ทดสอบofflineของrouting; ยังไม่ทดสอบผลโมเดลจริงทุกภาษา |

Scratch vertical asset170 IDs ถูกบรรจุในZIP แต่ยังไม่มีหลักฐานว่าKidBrightโหลดใช้งานจริง
จึงไม่บวกเข้ายอด343นิยามหลัก ส่วน mutator/helper ของeditorก็แยกจากคำสั่งฮาร์ดแวร์

รายละเอียดทั้งหมด:

- [รายบล็อก/หมวด/ID](KIDBRIGHT_BLOCK_COVERAGE_2026-10-16.md)
- [รายฟังก์ชันและเมธอด](KIDBRIGHT_FUNCTION_COVERAGE_2026-10-16.md)
- [board inventory และหลักฐานข้อขัดแย้ง](research/board-rule-audit-2026-10-16.json)
- [inventory ของเอกสารและ fenced code](research/rule-documents-2026-10-16.json)

## ข้อที่แก้ในรอบนี้

| ประเด็น | การแก้/สถานะหลังตรวจ |
|---|---|
| กฎcopyตรงต้นฉบับขัดกับกฎพอร์ตArduinoเป็นESP-IDF | แยกliteral copyออกจากbehavioral port และห้ามซ่อม/เปลี่ยนความหมายเงียบๆ |
| กฎอุปกรณ์เหมารวมทุกบอร์ด และpin tablesซ้ำในprompt | ถอนตารางซ้ำที่บังคับโดยไม่มีโปรไฟล์ แล้วให้ยืนยันPCB/เอกสารต้นทางก่อนใช้ |
| iA/32iA,32iP SW2/servo และRTC bus | ระบุเป็นvariant conflict; ไม่เดาขาทดแทน และไม่รับรองว่าตารางประวัติครบแล้ว |
| สูตรLM73ขัดกันท้ายprompt | ใช้signed register raw/128; แก้คำว่าright-justifiedและแยกregister scalingจากresolution0.25°Cใน11-bit mode |
| ADCControllerถูกบังคับlegacyทั้งที่ตัวอย่างใช้oneshot | ถอนข้อห้ามoneshotและผูกAPIกับSDK; ปรับCMakeตัวอย่างในMDให้มีesp_adc |
| iKB-1/iKB-1Zถูกเหมารวมเป็นMCP23017@0x20 | เขียนอ้างอิงใหม่จากINEXiKB-1Z:0x48/0x49และpacketจริง; originaliKB-1ยังต้องยืนยันrevision |
| SH1106/SSD1306สลับcharge-pumpและaddressing | แก้MD/promptเป็นSH1106AD+8Bและpage addressing; sourceCตัวอย่างเก่ายังต้องตรวจพอร์ตก่อนใช้ |
| ESP-NOWcallback/fieldclaimsซ้ำและเหมารวมเวอร์ชัน | ใช้ชนิดตามheaderSDK; ถอนข้ออ้างครอบจักรวาลเกี่ยวกับฟิลด์ของwifi_tx_info_t |
| บังคับinstallทุกcomponentซ้ำ | ตรวจSDK/projectdependencyก่อน และติดตั้งเฉพาะexternalpackageที่ขาดหลังKeep |
| READMEอ้างทุกไฟล์เป็นESP-IDF | แยกArduinoreferenceและsnippetที่ยังไม่build; ไม่อ้างว่าlegacyADCถูกลบเหมือนกันทุกSDK |
| all_models.mdมีbyteUTF-8เสียและตารางV1.6ปะปน | ซ่อมencodingและถอนส่วนตารางเสีย พร้อมระบุว่าตารางV1.6ที่ตรวจยืนยันครบยังขาด |
| ภาษาตามข้อความล่าสุดมีเพียงกฎstatic | เพิ่มตัวเลือกแหล่งข้อความและpolicyใหม่ทุกproviderrequest รวมหลังGooglepromptcompaction |

เพิ่ม [support_scope.md](../knowledge_base/support_scope.md) เป็นขอบเขตปัจจุบันที่ถูกแนบใน
system contextจริง ไฟล์ประวัติที่มีคำว่า“mandatory”ไม่สามารถขยายการรองรับอัตโนมัติ
โค้ดไม่ได้ใช้ข้อมูลอิสระในattachment/KBเป็นคำสั่งแทนผู้ใช้

เอกสารผู้ผลิตที่ใช้แก้ประเด็นสำคัญ:

- [TI LM73 datasheet §7.3.3](https://www.ti.com/lit/ds/symlink/lm73.pdf)
- [Espressif ESP-NOW5.5.4](https://docs.espressif.com/projects/esp-idf/en/v5.5.4/esp32/api-reference/network/esp_now.html)
- [INEX KidBright32iA](https://inex.co.th/home/product/kidbright32ia/)
- [INEX μAI Plus](https://inex.co.th/home/product/kidbright-microai-plus/)
- [INEX iKB-1Z protocol source](https://github.com/inexglobal/pxt-iKB1z/blob/main/iKB-1z.ts)
- [SH1106 datasheet, printed page22](https://www.pololu.com/file/0J1813/SH1106.pdf)

board audit JSON บันทึกข้อพบตอนอ่านก่อนแก้บางส่วน จึงมีline referenceของสภาพก่อนซ่อม
ให้ใช้ตารางนี้และhashของinventoryล่าสุดประกอบ ไม่ถือว่ารายการเดิมยังผิดอยู่ทุกจุด
ไฟล์ `rule-documents-before-2026-10-16.json` เป็นสภาพก่อนซ่อมencodingระหว่างรอบนี้
ไม่ใช่snapshotก่อนเริ่มงานทั้งหมด

## การเลือกภาษา

ให้ความสำคัญกับคำขอเลือกภาษาที่ระบุในข้อความผู้ใช้ล่าสุด ถ้าไม่ได้ระบุ ให้ใช้ภาษา
ของคำขอธรรมชาติหลักในข้อความล่าสุด หากข้อความมีเฉพาะcode/dataให้ย้อนหาuserproseล่าสุด
และใช้ไทยเมื่อไม่มีข้อมูลภาษาเลย

ตัวช่วยไม่นับภาษาในtool/assistant,ไฟล์อ้างอิง,codefence,log,quoteหรือfrontendwrapper
เป็นภาษาหลัก และไม่คัดลอกเนื้อหาผู้ใช้เข้าsystemprompt
รองรับข้อความUnicodeโดยไม่ทำรายการภาษาที่อนุญาตแบบตายตัว

เพิ่มเติมวันที่17ตุลาคม: เลขลำดับข้อความอย่างเดียวไม่เพียงพอเมื่อมีคำสั่งสั้น เช่น `try again` หลังบทสนทนาภาษาไทย
จึงแนบข้อความธรรมชาติที่คัดเลือกเป็นข้อมูลJSONในuser messageเดิมเฉพาะpayloadที่ส่งprovider
ไม่เพิ่มuser turn ไม่เขียนลงประวัติ และไม่ยกข้อความนั้นเป็นsystem instruction
กฎระบุว่าการทำงานต่อไม่ใช่การคงภาษาคำตอบเดิม และใช้หลักเดียวกันในทั้งสองprovider

ทั้งOpenAI-compatibleและGoogleใช้กฎเดียวกัน รวมคำอธิบาย สรุป และคำถามรุ่นบอร์ด
ชื่อAPI/path/board/packetและปุ่มKeep/Undoคงเดิม ขอบเขตนี้เป็นภาษาคำตอบAI
ไม่ใช่การแปลUIทั้งหมดหรือแปลrawlogจากcompiler

ทดสอบoffline9cases ครอบคลุมThai→English,Japanese,Chinese,Arabic,Burmese,Lao,
French,Russian,mixedlanguage,explicitpreference,Markdownheading,code-onlyfallback,
multimodaltextและtool/referenceexclusion บางcaseทดสอบหลายข้อความ
ผลดังกล่าวยืนยันการเตรียมcontextเท่านั้น ไม่ยืนยันความคล่องภาษาของโมเดลจริงทุกภาษา
การทดสอบเพิ่มเติมของข้อความสั้น การสลับภาษากลับ และpayloadหลังtool call ดู
[learner-sidebar-language-2026-10-17-validation.json](research/learner-sidebar-language-2026-10-17-validation.json)

## งานที่ยังขาดเพื่อรับรองครบ

1. โปรไฟล์มีversion/source/pin/resource constraintsสำหรับบอร์ดและส่วนต่อขยายที่เหลือ
2. สัญญาและCimplementationรายblock/method รวมตัวแปร,array/function/task/callback,
   การจัดตารางเวลา,display/fonts/scroll,sensor/IMU/encoder,motor,radio,IoTและerrorpaths
3. XML/Blocklyimportที่เก็บIDs,mutation,shadowprecedenceและทุกrootstack พร้อมunknown-blockdiagnostics
4. BuildmatrixตามSDK/board/feature combination และdifferentialtestsกับต้นแบบ
5. ทดสอบฮาร์ดแวร์และคู่ส่งรับจริง รวมmotorfeedbackและprotocolinteroperability
6. ทดสอบผ่านUIและผู้ให้บริการAIจริงด้วยชุดคำสั่งหลายภาษา ไม่มีการเรียกAPIหรือflashในauditนี้

## ทำซ้ำ

```powershell
npm run sync:kb
npm run audit:coverage
python scripts/audit-kidbright-functions.py --archive-root (Join-Path $env:TEMP 'vibekidbright-research-20261015/archive')
npm run audit:rules
npm run check:coverage
npm run check:rules
cargo test --manifest-path src-tauri/Cargo.toml --lib --offline
npm run build:release
```

Block inventoryทำซ้ำจากหลักฐานที่เก็บในrepositoryได้ ส่วนfunctioninventoryต้องใช้sourcecacheที่hashตรง
เมื่อเพิ่มหรือแก้MD/promptต้องgenerateรายงานcoverageและdocumentinventoryใหม่ก่อนrelease
การตรวจfreshnessไม่ใช่การตรวจความถูกต้องของทุกวงจรหรือทุกสมการ

ผลทดสอบรอบนี้และขอบเขตการตรวจถูกบันทึกใน
[rule-audit-validation-2026-10-16.json](research/rule-audit-validation-2026-10-16.json)
ผลfirmwareสองตัวอย่างและhostcore122checksเป็นหลักฐานpilotที่บันทึกไว้ ไม่ได้flashซ้ำในรอบauditนี้
