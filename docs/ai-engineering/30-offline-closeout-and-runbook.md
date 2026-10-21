# ปิดงานเตรียม offline และรายการรันทดสอบภายหลัง — 2026-10-19

ผู้ใช้ยืนยันว่ายังไม่มี endpoint/model และ credentials สำหรับรอบนี้ ให้ทำส่วน
offline และไม่ถาม API ซ้ำ จึงไม่มีการ probe, inference, เปิด credential store,
เปลี่ยน production configuration, Apply/Undo, Build firmware หรือ Flash
สิทธิ์เดิมยังจำกัดเฉพาะ public fixture/KB และไม่เกิน 10 HTTP requests รวม retries
กับ 1 USD แต่ยังไม่มี target ที่ใช้สิทธิ์นั้นได้

ตรวจ manifest ก่อนเริ่มตรง **322 ไฟล์** รักษา dirty changes เดิม รอบนี้ไม่เปลี่ยน
implementation ไม่เพิ่ม framework, tests, dependencies, model, ranking หรือ corpus
ใช้เครื่องมือ/เกณฑ์จาก protocol 23 และ cases เดิม ผลจริงและ hashes อยู่ใน
`offline-closeout-verification-20261019.json` และ `offline-closeout-changes-20261019.json`

## สิ่งที่ทำเสร็จในส่วนเตรียม offline

- รัน preparation, standalone matrix และ review-template ด้วย CLI เดิมไปยัง path
  ใหม่ใน `src-tauri/target/offline-closeout-20261019/` ไม่มีการเขียนทับรายงานเก่า
- matrix ทั้ง 16 เกณฑ์และ 7 required workflow stages คง NOT_EXECUTED; review draft
  22 แถวคง null ไม่มี model response, retrieved chunks หรือ actual-model diff/log
  ที่สร้างขึ้นมาทดแทนข้อมูลจริง
- เตรียม manual-stage draft ตาม schema เดิมสำหรับ compile/diagnostics/
  bounded-repair/rollback ผูกกับ prepared evaluation bytes แต่ยังใช้เป็นหลักฐาน
  actual-model ไม่ได้ ไม่ได้เติม CAPTURED หรือ approval
- ตรวจ negative gates ด้วย CLI จริง: draft manual/review ต้องถูกปฏิเสธและไม่สร้าง
  matrix สำเร็จ ไม่มีการลดเกณฑ์หรือปลอมคำอนุมัติ
- ยืนยัน blind-review packet 20 ข้อและ frozen source snapshot 37 ไฟล์ที่เตรียมไว้
  ยังคง hash เดิมและ judgments เป็น null ไม่คัดลอก rankings/answers/frozen qrels
  เข้าซอง blind review ไม่ใช้ packet นี้เป็น current-corpus ground truth
- รัน Python **24 tests** และ Node **12 tests** ผ่าน จำนวนไม่เพิ่มจากรอบก่อน
  เป็นผลเชิงกลสำหรับ evidence/CLI เท่านั้น ไม่ใช่คุณภาพ AI
- ตรวจ policy/hash ที่เกี่ยวข้อง คง NSIS 3.7.3, MiniLM/ranking, frozen 69 queries/
  qrels, current bundled corpus, nine Block IDs/schema v1, SDK config และ lockfiles

## คำสั่ง offline ที่รันได้และตรวจแล้ว

ใช้ PowerShell ที่ root ของ repository คำสั่งนี้ไม่เรียก endpoint ไม่อ่าน keys และ
ไม่สร้าง installer `$evalRunDir` ต้องเป็น path ใหม่ทุกครั้ง ห้ามลบของเก่าเพื่อให้ผ่าน
การตรวจ EEXIST

```powershell
$ErrorActionPreference = 'Stop'
python -X utf8 -c "from pathlib import Path; import hashlib,json; v=json.loads(Path('docs/ai-engineering/verification.json').read_text(encoding='utf-8')); bad=[p for p,h in v['source_sha256'].items() if hashlib.sha256(Path(p).read_bytes()).hexdigest()!=h]; assert not bad,bad; print(len(v['source_sha256']), 'hashes verified')"
if ($LASTEXITCODE -ne 0) { throw 'Current source/evidence manifest mismatch' }

$evalRunTag = Get-Date -Format 'yyyyMMdd-HHmmss-fff'
$evalRunDir = "src-tauri/target/offline-$evalRunTag"
New-Item -ItemType Directory -Path $evalRunDir -ErrorAction Stop | Out-Null

node scripts/evaluate-vibe-coder-real.mjs --workflow thai-delay-existing-project-v1 --output "$evalRunDir/prepared.json"
if ($LASTEXITCODE -ne 0) { throw 'Preparation failed' }

node scripts/evaluate-capabilities.mjs --evidence "$evalRunDir/prepared.json" --output "$evalRunDir/matrix.json"
if ($LASTEXITCODE -ne 0) { throw 'Matrix preparation failed' }

node scripts/evaluate-capabilities.mjs --evidence "$evalRunDir/prepared.json" --template review --output "$evalRunDir/review-draft.json"
if ($LASTEXITCODE -ne 0) { throw 'Review-template preparation failed' }

python -m unittest discover -s scripts/tests -v
if ($LASTEXITCODE -ne 0) { throw 'Offline Python checks failed' }

node --test scripts/tests/*.test.mjs
if ($LASTEXITCODE -ne 0) { throw 'Offline Node checks failed' }
```

รอบนี้ใช้ path คงที่ใหม่ `src-tauri/target/offline-closeout-20261019` แทน timestamp
ส่วนคำสั่ง CLI และชุดทดสอบเป็นชุดเดียวกัน ดู executed command ledger สำหรับคำสั่ง/
exit code/log hash ที่รันจริง ไม่ใช่ถือ code fence นี้เป็น execution evidence

## เงื่อนไขก่อนเปิด actual-model ในอนาคต

1. มี provider/endpoint/model-version และ test credential channel ที่ระบุจริง
   ห้ามค้นหา keys หรือถือ default ของแอพเป็นสิทธิ์ของผู้ใช้ ไม่ถามเรื่องนี้ซ้ำในรอบนี้
2. บังคับเพดานก่อนทุก native HTTP attempt รวม metadata, retries และ redirects
   พิสูจน์ offline ว่า request ที่ 11 ไม่ถูกส่ง เพดานครอบคลุมทั้ง pilot ไม่รีเซ็ต
   เมื่อเปลี่ยน case/turn และหยุดเมื่อถึงเพดานใดก่อน
3. มีขอบเขตค่าใช้จ่ายที่รับประกันไม่เกิน 1 USD ตาม provider/model จริง การอ่าน usage
   หลังคิดเงินไม่ใช่ hard cap และไม่อนุมานว่า localhost เป็นบริการฟรี
4. จำกัด outbound context ให้เป็น public fixture/KB เท่านั้น ตรวจ serialized request
   จริงก่อนส่ง ไม่ให้ credential headers, URL keys, private source หรือ SDK/user paths
   หลุดไปเพราะ permission policy สำหรับ loopback กว้างกว่าสิทธิ์ของ pilot
5. เก็บ request/response/counts แบบ redacted จาก native route จริง พร้อม model/
   provider/version เท่าที่ยืนยันได้ ไม่แทนที่ด้วย prompt ที่สร้างจากการคาดเดา
6. คง per-attempt project/revision-bound user approval, no automatic Keep/Apply/
   Undo และ maximum three repair attempts ไม่ Flash

ยังไม่ผ่านข้อเหล่านี้ในซอร์สปัจจุบัน `--run` จึงถูกปิดก่อน CDP และ `/models`
ไม่มี CLI/environment bypass การเตรียม offline เสร็จไม่ได้หมายความว่า implementation
ส่วน native metering/cost/request capture เสร็จแล้ว

## เส้นทางแอพและรายการ pilot เดิม

เส้นทางที่ต้องทดสอบคือ **React Vibe Coder → Tauri → native provider/tool loop →
authorized model → real review/build controls** ผ่าน credential-isolated debug
desktop ที่มี identifier `th.nectec.vibekidbright.desktop-evaluation` ไม่ใช้แอพ
ติดตั้งของผู้ใช้หรือโปรเจกต์ส่วนตัว ต้องบันทึก source revision และ fixture hash ใหม่
ก่อนเริ่มทุก run

case IDs และ prompt ต้นฉบับอยู่ใน `real-vibe-coder-cases.json` ห้ามเปลี่ยนโจทย์หรือ
required evidence ให้ผ่านง่ายขึ้น harness ปัจจุบันรับเฉพาะ public generated
`repeat_delay` / `forever_delay` fixtures ที่ระบุไว้ในซอร์ส ไม่เลือก directory อื่น
เพื่อเลี่ยง whitelist

| ลำดับ / case ID | การดำเนินการเมื่อพร้อมจริง | หลักฐานขั้นต่ำ / จุดหยุด |
|---|---|---|
| 1 `grounding-th` | ส่ง prompt เดิมผ่าน Vibe Coder เพื่อค้นและอธิบายเอกสาร | เก็บ request, response, retrieved chunk bytes/hash/range, citations; reviewer ตรวจ relevance/support/API ไม่แก้ไฟล์ |
| 2 `project-th` | อ่าน tree/source/CMake และค้น symbol ของ fixture เดิม | ผูก canonical project/source revision, declaration/definition/caller, dependencies; แยกสิ่งที่ไม่ทราบและห้ามแตะไฟล์ |
| 3 `generation-th` | ให้ AI เสนอ patch ตามโจทย์ภาษาไทยเดิม | เก็บ proposed diff/revision/citations; saved source ต้องไม่เปลี่ยนก่อน Keep และต้องหยุดให้ผู้ใช้ตรวจ |
| 4 `compile` | ผู้ใช้อนุมัติ revision จริง แล้ว Build ผ่าน configured ESP-IDF | คำสั่งจริง, SDK/target ที่ตรวจแล้ว, build log/exit code, source/CDB/firmware hashes; compilation ไม่รับรอง hardware |
| 5 `diagnostics` | ใช้ public fault ที่ผู้ใช้ควบคุมใน fixture และให้โมเดลวิเคราะห์ compiler error จริง | เก็บ fault/source revision, raw diagnostics, classification ของโมเดลและหลักฐานเปรียบเทียบ ห้ามใช้ simulated log เป็นผล actual-model |
| 6 `bounded-repair` | ให้ AI เสนอ focused repair และรอ user approval ทุกครั้ง | เก็บ patch/revision/consent consumption/attempt count/rebuild; failure ต้องขอ consent ใหม่, failed duplicate/stale approval ต้องถูกปฏิเสธ, limit ≤3 |
| 7 `rollback` | ผู้ใช้สั่ง Undo ผ่านกลไกเดิม | เก็บ rollback result และ exact before/after source hashes รวม unrelated files; ทดสอบ conflict/recovery ตาม acceptance ไม่ถือว่า hash เท่ากันคือผ่าน power-loss/OS reboot |

สามข้อความแรกเป็นเพียง initial collection ไม่ครอบคลุม Build-aware การทดลอง repair
หนึ่งครั้งไม่แทนหลักฐาน failed-duplicate, stale consent, เพดานสามครั้งหรือ restart
recovery ต้องเก็บกรณีที่ยังขาดก่อนให้ criterion นั้น PASS หาก 10 HTTP requests หรือ
1 USD หมดก่อน ให้หยุดและคงกรณีที่เหลือ NOT_EXECUTED ไม่เพิ่มเพดานหรือข้ามเกณฑ์เอง

`grounding-en` และ `generation` ใน cases เดิมเป็นกรณีเพิ่มเติมที่ต้องแยกรายงาน
หากรัน ไม่ใช้ pilot ภาษาไทยแทนการตรวจภาษาอังกฤษ/mixed-language หรือการรับรอง
ทุกบอร์ด/ทุกบล็อก

## คำสั่ง future actual run — ยังรันสำเร็จไม่ได้ตอนนี้

เป็นรูปคำสั่งของ harness เดิมสำหรับ **local loopback route เท่านั้น** ใช้หลัง
prerequisites และ native safeguards ได้รับ implementation/verification แล้ว
ไม่รองรับ remote/paid provider เพียงเพราะแทน URL ลงไป ห้ามลบตัวกันหรือ bypass
เพื่อให้ code fence นี้รันได้ และอย่ารัน placeholder ตามตัวอักษร

```powershell
# FUTURE ONLY: fill concrete, explicitly authorized values after prerequisites pass.
node scripts/evaluate-vibe-coder-real.mjs --run --authorize-local-inference --workflow thai-delay-existing-project-v1 --endpoint '<AUTHORIZED_LOOPBACK_ENDPOINT>' --model '<AUTHORIZED_MODEL>' --output '<NEW_IGNORED_RUN_DIRECTORY>/actual.json'
```

ต้องมี isolated desktop ที่พร้อมใช้อยู่และเลือก fixture ถูกต้องก่อนเรียก CLI นี้
ตัว CLI ไม่ launch/จัด environment ให้ครบเอง ตัว desktop probe ที่มีอยู่ทำการ
ทดสอบเชิงกลและปิด window เมื่อเสร็จ จึงไม่ใช่ launcher สำหรับ interactive model
pilot ที่ต้องรอผู้ใช้อนุมัติ ต้องเตรียมการเปิด isolated app/production frontend
แยกตาม executable entrypoint และขอบเขตที่ตรวจยืนยันแล้ว ไม่ถือการ launch หรือ
preflight เป็น actual-model success

native counter/cost/scope enforcement และ full request capture ยังต้องทำก่อนเปิด
เส้นทางนี้ จึงไม่มีคำสั่ง “เปิดครบแล้ว” ที่ตรวจสำเร็จให้คัดลอกในปัจจุบัน

## การรวบรวมและตรวจหลักฐานหลัง actual run

เก็บไฟล์ actual run ใน ignored target เท่านั้น เพราะ logs อาจมีข้อมูลที่ต้องตรวจ
ก่อนเผยแพร่ ทุก artifact ต้องผูก case ID, source/input revision และ SHA256:
retrieved chunks, redacted actual request, response, citations, proposed/applied
diff, user-bound revision approval, build command/log/exit code, SDK/model metadata
และ rollback/unrelated-file hashes อย่าเติม artifact ที่ไม่ได้เกิดขึ้น

```powershell
# FUTURE ONLY: these paths must contain actual, independently assessed evidence.
node scripts/evaluate-capabilities.mjs --evidence '<ACTUAL_EVALUATION.json>' --manual '<RECORDED_MANUAL_STAGES.json>' --output '<NEW_MATRIX.json>'
node scripts/evaluate-capabilities.mjs --evidence '<ACTUAL_EVALUATION.json>' --template review --output '<NEW_REVIEW_DRAFT.json>'
node scripts/evaluate-capabilities.mjs --evidence '<ACTUAL_EVALUATION.json>' --manual '<RECORDED_MANUAL_STAGES.json>' --review '<INDEPENDENT_REVIEW.json>' --output '<NEW_REVIEWED_MATRIX.json>'
```

template ที่สร้างจาก prepared.json ผูก bytes ของ preparation เท่านั้น ต้องสร้าง
template ใหม่จาก actual evaluation bytes จริง ห้ามเปลี่ยนเพียง hash หรือใส่ผล
มนุษย์ให้ข้อมูลที่ยังไม่ execute manual packet ต้องเป็น schema/status ตาม protocol
เดิมและอ้าง actual artifacts; draft มี null ไม่ผ่าน gate ค่า CAPTURED ไม่ใช่
AI-quality PASS ส่วน reviewer ต้องเป็นบุคคลอื่นที่รับรองความเป็นอิสระตามเกณฑ์
file hashes ยืนยัน identity ของข้อมูล ไม่ยืนยันตัวบุคคลหรือความจริงของคำกล่าว

## สถานะปิดรอบ

การเตรียม offline/คำสั่ง/รายการหลักฐานที่ทำได้ในรอบนี้เสร็จแล้ว ไม่มีคำถาม API
เพิ่มเติม แต่ไม่ปิดงาน actual-model หรือ native safeguards ว่า COMPLETE

| กลุ่ม | สถานะ |
|---|---|
| 16 AI-quality criteria และ actual workflow mechanics | NOT_EXECUTED ทุกข้อ |
| 7 required actual workflow stages | NOT_EXECUTED |
| Model requests / metadata probes / ค่า API pilot | 0 / 0 / 0 USD |
| Independent reviewers/judgments | 0; review judgments ยัง null |
| Retrieval performance ใหม่ | ไม่รัน; ไม่อ้าง accuracy gain |
| Budget/cost/outbound-scope/request capture implementation | ยังไม่พร้อมเปิด actual run |
| Clean VM/hardware/cross-platform qualification | ยังไม่ครบ; ไม่นับ user-reported iP display เป็นการรับรองทั้งหมด |
| Release / ข้อกำหนดเดิม 8 ข้อ | NOT_READY / PARTIAL ทุกข้อ |

ไม่สร้าง installer/application build, ไม่ commit/push/publish/deploy และคง NSIS
3.7.3 เดิม ไม่มีการใช้ deterministic builds หรือ debug fixtures รับรอง installed
release หรือคุณภาพ AI
