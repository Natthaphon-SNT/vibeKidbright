# Toolchain download integrity — 4 พฤศจิกายน 2026

เพิ่มการตรวจ SHA-256 จาก manifest ที่ฝังในโปรแกรมก่อน extract และตรวจซ้ำก่อน activate พร้อม fixtures ของเส้นทางติดตั้งจริง **production assets ทั้งสองยัง unverified จึงปิด automatic installation** และเก็บ toolchain ที่มีอยู่ไว้ ผลนี้ไม่เปลี่ยนการประเมิน release, publisher signature, clean VM หรือ hardware จากรายงานก่อน

## Baseline และการรักษางานเดิม

- อ่าน `src-tauri/target/source-repair-2026-11-04T01-51-14-735Z/REPAIR_REPORT_2026-11-04.md`, `CHANGE_INVENTORY.md`, `change-inventory.json` และ source ที่เกี่ยวข้อง ไม่พบ `AGENTS.md` ใน workspace/source หรือ ancestor directories ที่ตรวจ
- HEAD `7d0157a71e380655e8d610dae74d32ef2c97d562`; ก่อนเริ่มมี tracked dirty 73 และ untracked 135 ไฟล์ งานเดิมไม่ทราบผู้เขียน
- Snapshot/hash ของ public source 689 ไฟล์, Git status/diff ก่อนเริ่ม, สำเนาไฟล์ก่อนแก้ และ logs อยู่ที่ `src-tauri/target/toolchain-integrity-20261104-153137/` ไม่รวม ignored credentials/settings
- รอบนี้แก้ existing files 7 ไฟล์ซึ่ง dirty อยู่ก่อนทั้งหมด เพิ่ม source/manifest/tests 3 ไฟล์ และรายงาน/evidence 2 ไฟล์ ดู `change-inventory.json` และ `round-existing.diff` ใน run directory เพื่อแยกจาก Git diff ที่รวมงานเดิม
- baseline ที่อยู่นอก 7 ไฟล์นี้คง bytes เดิมทั้งหมด ไม่มี baseline file ถูกลบ; untracked เดิมทุกไฟล์คงเดิม ไม่มี stage/commit/push/publish/deploy, ไม่สร้าง installer, ไม่เรียก AI provider, ไม่อ่าน credentials และไม่ต่อ/Flash บอร์ด

## ไฟล์และพฤติกรรมที่เปลี่ยน

| ไฟล์ | ผลของรอบนี้ |
|---|---|
| `src-tauri/toolchain-manifest.json` | แยก logical ID, exact GitHub release/asset ID, intended platform, requested version และ expected SHA-256 ออกจาก download URL; ทั้งสอง entry เป็น unverified โดย expected hash/provenance เป็น null |
| `src-tauri/src/toolchain_integrity.rs` | Parse manifest แบบ strict, ปฏิเสธ missing/ambiguous entry, platform/version mismatch, unverified/incomplete trust record และ custom URL; เป็นเจ้าของ temporary archive snapshot และตรวจ SHA-256 ของ bytes จริง |
| `src-tauri/src/toolchain.rs` | ทุก installation ผ่าน manifest plan ก่อน network/AppData; acquire ทุกส่วนและตรวจให้ครบก่อนเริ่ม extract; extract ใน staging เฉพาะรอบ; rehash ก่อน sentinel/activate; เก็บ snapshot จนจบ; download ทีละส่วน; ใช้ HTTPS; ไม่มี sentinel fast path ที่ทำให้คำสั่งติดตั้งข้ามการตรวจ |
| ไฟล์เดียวกัน | เปลี่ยนจาก fixed `.installing`/`.backup` เป็น temporary session เฉพาะรอบ ไม่แตะ ZIP/staging/backup เก่า; rollback เมื่อ rename ไม่สำเร็จ และคง recovery directory หาก OS rollback ไม่ได้; install lock อยู่กับ blocking task จนจบ แม้ผู้เรียก async หลุด และใช้ lock เดียวกันกับ remove/path repair |
| `src-tauri/src/archive.rs` | รับ `Read + Seek` ของ snapshot ที่ตรวจแล้วแทนเปิด archive pathname ซ้ำ; path-based wrapper เหลือเฉพาะ tests; คง traversal/link/special-file/expansion/cancellation/Windows long-path protections เดิม |
| `src-tauri/src/esp_idf.rs` | ปิด Git/Python setup ก่อนสร้าง directory, อ่าน cache, clone หรือรัน installer scripts; เก็บชื่อ IPC arguments เดิม; readiness ของ SDK เดิมระบุ `existing_installation_not_reverified` |
| `src-tauri/src/lib.rs` | รวม integrity module ทุก target โดยไม่มี platform/feature gate |
| `src/ToolchainSetup.tsx`, `src/App.tsx` | แสดงขั้น verifying, ชี้แจงว่า assets ยัง unverified และ custom URL ต้องมี trusted manifest; ข้อผิดพลาดจาก backend แสดงตามเส้นทาง UI เดิม |
| `src-tauri/src/toolchain_install_tests.rs`, `src/test/toolchainFirstLaunch.test.tsx` | ZIP fixture transactions และ UI error/progress tests; test hashes และ test mutation helper compile เฉพาะ tests |
| `docs/research/toolchain-integrity-2026-11-04.json` | public metadata, URL, dates, observed digest/size, attestation lookup และ trust decision แยกจาก expected hashes |

Manifest ที่ฝังใน executable เป็น trust root ของการตรวจ bytes ไม่มี runtime manifest override, expected-hash CLI input หรือ environment flag สำหรับข้าม gate การเปลี่ยน entry เป็น verified ต้องผ่านการ review หลักฐาน publisher และ binding ก่อน ไม่ใช่เพียงใส่ข้อความ provenance ใน JSON

## แหล่งอ้างอิงและ trust decision

ตรวจ public metadata วันที่ **2026-11-04** โดยไม่ใช้ credentials:

| Asset / identity | Intended tuple จากแอปเดิม | Observed bytes / SHA-256 จาก release metadata | ผล |
|---|---|---|---|
| frameworks.zip; release 349411794, asset 467878450 | windows-x86_64 / bundle 1.0.1 | 300555002 / `57eac23280db220c1b4947b016f97c56659a36aa360fc522cf6b65adb92da5d6` | unverified; ไม่ใช้เป็น expected hash |
| tools.zip; release 349410633, asset 467876766 | windows-x86_64 / bundle 1.0.1 | 1981584946 / `93f570df1acf8cc38e99c6b7b6a9b3513a82ff834798d3fc504ced2030dc9fec` | unverified; ไม่ใช้เป็น expected hash |

แหล่งหลักคือ [framework release metadata](https://api.github.com/repos/Natthaphon-SNT/vibeKidbright/releases/tags/framework) และ [toolchain release metadata](https://api.github.com/repos/Natthaphon-SNT/vibeKidbright/releases/tags/toolchain) ทั้งสองยัง `immutable=false`, `target_commitish=main` และรายการ assets มีเพียง ZIP ไม่มี checksum/signature asset แยกที่ query พบ

ตรวจ public repository attestation API ตาม observed SHA-256 ทั้งสอง ได้ HTTP 404 ทั้งคู่ เก็บ URL/response ใน research JSON; การตอบ 404 แบบ unauthenticated ไม่ได้พิสูจน์ว่าไม่มี attestation ที่อื่น และไม่ได้ตรวจลายเซ็นใดสำเร็จ

ยังไม่มีแหล่งที่ยืนยัน publisher identity และ binding ของ bytes กับ platform/version ได้ครบ Digest จากบัญชี/release เดียวกันจึงเป็นเพียง observation ไม่ใช่ independent trusted provenance; `1.0.1` คือ requested bundle version จาก source เดิม ส่วน Windows tuple เป็น intended target ยังไม่ได้รับการยืนยันจาก archive/publisher

ไม่มีการดาวน์โหลด public artifact รอบนี้ จึงไม่มีข้ออ้างว่าได้เทียบ bytes ของ production ZIP แล้ว ไม่มี hash สมมติ ไม่มี expected hash ที่สร้างจาก ZIP ดาวน์โหลดเดียวกัน Fixtures สร้าง ZIP/hash ของข้อมูลสังเคราะห์สำหรับ tests เท่านั้น

## เส้นทาง install/repair/cache ที่ตรวจ

| เส้นทาง | Gate / ขอบเขต |
|---|---|
| First-launch, full/mini widget, Install/Retry | `ToolchainSetup` → `download_toolchain` → bundled manifest plan → private snapshot → guarded extraction → rehash → activation |
| Setup / Repair ESP-IDF ใน App | `handleAutoInstallGithub` → คำสั่งเดียวกัน ไม่มีการ remove toolchain ก่อน retry และไม่มี fallback ไป legacy installer |
| Direct IPC `download_toolchain`, optional custom URL | gate เดียวกัน; custom URL ที่ไม่ว่างถูกปฏิเสธก่อน network โดยไม่ echo URL/credentials; ready sentinel ไม่ข้าม gate |
| Direct IPC `setup_esp_idf(version, targets)` | หยุดที่ policy error ก่อน filesystem/network/process จึงไม่ใช้ Git checkout, idf_tools cache, tool archives หรือ Python dependency cache เป็นทางลัด |
| ZIP เก่า เช่น `.installing/_download_0.zip` | ไม่ถูกนำกลับมาใช้หรือรับรองจาก sentinel และไม่ถูก cleanup โดย install รอบใหม่ ไม่มี production persistent ZIP cache/import endpoint; fixture cache loader ผ่าน acquisition/snapshot/verification เดียวกันและอ่าน hash ใหม่ทุกครั้ง |
| `repair_toolchain_paths`, `auto_repair_on_startup` | แก้ pyvenv paths ของ SDK ที่มีอยู่ ไม่มี download, archive extraction หรือสร้าง installation sentinel; ไม่รับรองย้อนหลังว่า SDK มี provenance |
| `remove_toolchain` | explicit removal แยกจาก install และใช้ install lock; ไม่ได้เรียกคำสั่งนี้จริงในรอบนี้ |
| Environment/custom/runtime/bundled SDK resolution | เลือก SDK เดิมสำหรับ readiness/build ไม่ได้ดาวน์โหลดหรือติดตั้ง ไม่ใช้เป็นหลักฐาน verified installation |

ตรวจ scripts อื่นด้วย: `build-kidbright-examples.py`/`build-ikb1z-idf.py` เรียก `idf_tools.py export` เท่านั้น ส่วน `scripts/evaluate-clangd.py` เป็น development evaluator นอก application install path ใช้ ZIP/digest pin เดิมและ extracted cache ใต้ ignored target; ไม่ bundled หรือเป็น fallback ให้ ESP-IDF installer ไม่ได้เรียกหรือเปลี่ยนในรอบนี้ และไม่ยกระดับผลเก่าเป็น publisher verification การ audit lifecycle/trust ของ evaluator แยกนี้ยังอยู่นอกผลรับรองของ managed toolchain gate

## ป้องกันการสลับไฟล์และข้อจำกัด

- Acquisition เขียนลง `tempfile::tempfile()` ที่ integrity type เป็นเจ้าของ Hash และ `ZipArchive` ใช้ file handle เดิม; ไม่มีการเปิด cached/download pathname ซ้ำหลังตรวจ SHA-256 อ่าน hash หลัง acquire, ก่อน extract และก่อน activate ของทุกส่วน
- ตรวจ locked source ของ tempfile 3.26.0: Windows สร้างด้วย `share_mode(0)` และ `FILE_FLAG_DELETE_ON_CLOSE`; ไม่อนุญาตเปิด handle อื่นผ่าน filename ตาม sharing semantics ขณะ handle นี้อยู่ Unix ใช้ unlinked file หรือ Linux `O_TMPFILE` เมื่อรองรับ ทั้งนี้ Unix unlink fallback มีช่วง create/unlink สั้น ๆ ตาม implementation ของ dependency
- เปลี่ยน path ของ cached ZIP หลัง snapshot ไม่เปลี่ยน bytes ที่ extract; cache ถูกแก้ก่อนใช้ต้อง hash ใหม่และ fail snapshot ไม่ออกจาก integrity type หลัง acquire และไม่มี external pathname ให้ extractor reopen
- ใช้ AppData parent เดียวกับ final สำหรับ staging/backup เพื่อ rename โดยไม่ข้าม volume คง toolchain เดิมจนทุก gate ผ่าน; snapshot อยู่ใน OS temp และต้องมีพื้นที่สำหรับ archives, staging และ SDK เดิมพร้อมกัน การอ่าน hash หลายรอบเพิ่ม disk I/O
- ป้องกัน pathname substitution ในเส้นทางแอป ไม่รับรองการป้องกัน administrator/kernel/process compromise หรือผู้โจมตีที่แก้หน่วยความจำ/handle/staging ด้วยสิทธิ์บัญชีเดียวกัน และไม่ตรวจ local tampering ของ SDK หลัง activate
- Rename/rollback ไม่ใช่ธุรกรรมที่ crash-atomic หาก OS rollback ล้มเหลวจะคง backup ของ session ไว้และแจ้งเหตุผล ห้ามตีความเป็น old path กู้คืนสำเร็จ ยังไม่ได้จำลอง power loss, antivirus locks, disk-full จริง หรือหลาย application processes ติดตั้งพร้อมกัน; lock ปัจจุบันครอบคลุมภายใน process
- Error แสดงชนิดเหตุผิดพลาดและ asset logical ID แทน raw network URL, private temp path หรือ archive member ที่อาจมีข้อมูลส่วนตัว

## ผล tests/checks ที่รันจริง

รันบน Windows x64 / MSVC, Rust 1.99.0 มี native target ที่ติดตั้งเพียง `x86_64-pc-windows-msvc`:

| คำสั่ง | ผล |
|---|---|
| `cargo test --lib --locked --offline toolchain -- --nocapture` | fixture ชุดแรก 22 tests ผ่าน; ก่อนเพิ่ม final snapshot activation case |
| `cargo test --all-targets --locked --offline` | 290 library tests + 1 example knowledge_eval test ผ่าน; 0 failed/ignored; targets ที่ไม่มี tests compile สำเร็จ ไม่ได้นับเป็น test cases เพิ่ม |
| `cargo clippy --all-targets --locked --offline --message-format=short` | exit 0; มี warnings เดิม ไม่ใช้ `-D warnings` |
| `cargo check --release --lib --locked --offline --message-format=short` | exit 0 พร้อม warnings; compile/check library เท่านั้น ไม่สร้าง installer |
| `npm test -- --reporter=dot` | ผลสุดท้าย 234 tests / 23 files ผ่าน; รอบแรก test ใหม่จำลอง verifying event ก่อนเริ่ม install ทำให้ progress ไม่แสดง จึงแก้ fixture ให้ click Install ก่อนส่ง event และรันทั้งชุดใหม่ผ่าน |
| `npm run build` | TypeScript + production Vite build ผ่าน; มี large chunk warning เดิม |
| `npm run check:public`, `npm run check:bundle` | ผ่าน; bundle check ตรวจ resource เดิม 59 ไฟล์ |
| `rustfmt --edition 2021 --check` สำหรับสอง Rust files ใหม่ | ผ่าน; ไม่ format source dirty เดิมทั้งโครงการ |
| scoped `git diff --check -- <7 changed existing files>` | ผ่าน |
| global `git diff --check` | exit 2: whitespace ใน `Readme.md` และ `knowledge_base/compatibility/board_catalog.json` ซึ่ง hash เท่ากับ baseline จึงรักษา bytes เดิม |

12 native regression tests ใหม่ครอบคลุม matching multi-part install, wrong/modified/truncated bytes, second-part failure ก่อน extract ทั้งชุด, missing/unverified/incomplete/ambiguous manifest/schema, platform/version mismatch, cache recheck แม้มี sentinel, cached-path swap, traversal แม้ hash ตรง, matching-hash invalid ZIP, unreadable source/acquisition error, cancellation, snapshot ถูกแก้หลัง extract ก่อน activate, old SDK/stale-cache preservation และ install/repair wiring ส่วน archive regression tests เดิมยังผ่าน รวม Windows long paths และ guarded extraction

UI tests ใหม่สองรายการตรวจการแสดง integrity rejection โดยไม่มี onReady/fallback install และ verifying progress ที่ไม่ประกาศ success Native fixtures ใช้ transaction/acquisition/activation code เดียวกับ production แต่แทน network acquisition ด้วย ZIP fixture stream ไม่ได้ทดสอบ live HTTP download หรือ Tauri desktop startup/IPC บนบัญชีผู้ใช้จริง

## Packaging, installer และ release

`mod toolchain_integrity` รวมโดยไม่มี platform gate และ manifest ใช้ `include_str!("../toolchain-manifest.json")` จึง compile เข้า Rust executable ทุก target; ถ้า source manifest หาย build จะ fail ไม่ต้องพึ่ง resource copy path หรือ installer download hook Windows NSIS/MSI และ macOS universal workflow ใช้ Tauri project/library เดียวกัน และ all-target tests/Clippy ตาม source repair รอบก่อน ไม่ได้แก้ workflows/dependencies/config รอบนี้

ตรวจ source inclusion และ Windows release library compile สำเร็จเท่านั้น **ยังไม่ได้พิสูจน์ว่า installer ที่มีอยู่บรรจุ implementation ใหม่** เพราะไม่ได้สร้าง/แทนที่ installer และไม่ได้รัน macOS Intel/ARM หรือ Linux native builds; ไม่มี trusted entries สำหรับแพลตฟอร์มเหล่านั้น จึงถูกปฏิเสธตาม platform/version policy

ตรวจ Windows installer เดิมแบบอ่านอย่างเดียว:

| รายการ | bytes | SHA-256 |
|---|---:|---|
| `3.7.4-validation.json` | 316901145 | `91d5bfa1d28195eec04d7a335eb3024ea2f7466e66062c432afc2ce5daa3e61f` |
| `src-tauri/target/release/bundle/nsis/VibeKidbright IDE_3.7.4_x64-setup.exe` ปัจจุบัน | 318087472 | `1ab2857f1ed542d70001c24ea35eb6858d3024e3d937a0c1360b19a8b9b4a686` |

ยัง mismatch และ identity UNVERIFIED ตามหลักฐานเดิม ไม่แก้ expected hash/validation/icon evidence ไม่มี installer ใหม่และไม่มีการตรวจ publisher signature สำเร็จจาก hash comparison นี้

พบ MSI 3.7.4 เดิมด้วย: 325861376 bytes, SHA-256 `6d6fce12ac9019846bb297edfb0fd8b8c7144beadf165d22d1fdb243c3b58378`; validation record ที่อ้างข้างต้นระบุเฉพาะ NSIS จึงไม่มี expected identity สำหรับ MSI นี้ ไม่พบ macOS bundle directories ใน target paths ที่ตรวจ ไม่อนุมานว่า artifacts เหล่านี้มี source ใหม่

## รอบถัดไปและ clean VM

1. ขอ publisher checksum/signature/attestation ที่ตรวจ key หรือ repository/workflow identity ได้อย่างอิสระ ผูก exact asset/version/platform แล้ว review manifest ก่อนเปิด verified entry; ตรวจ bytes ของ asset ที่ระบุใน temporary directory และบันทึก URL/size/hash/comparison แยกจาก identity verification
2. เมื่อได้รับอนุญาตสร้าง installer รุ่นถัดไป ให้ผูก source revision + source/manifest hashes + dependency locks + build target + artifact size/SHA-256 และ signer/notarization evidence ใน validation record ใหม่ เก็บ artifacts เดิมไว้และตรวจ executable ที่ extract จากแต่ละ installer ว่าเป็น build ที่อนุมัติจริง
3. ทดสอบ Windows NSIS/MSI และ macOS Intel/ARM บน clean VM/เครื่องที่เกี่ยวข้อง: install → first launch → blocked unverified install → verified download/extract/activate → retry/cancel/cache tamper → upgrade พร้อม SDK เดิม → uninstall/reinstall พร้อม data-retention policy; ตรวจ Unicode/long paths, proxy/offline, permissions, disk-full, file locks, rollback และ crash recovery
4. รัน native tests และ signature/Gatekeeper/SmartScreen/notarization ตาม platform จริงก่อนประเมิน release ใหม่; แยกสถานะ Linux หากจะรองรับโดยยังต้องจัดการ dependency advisories ตามรายงานเดิม
5. Physical board/flash/runtime/electrical/timing compatibility ยังต้องตรวจด้วย hardware และขอบเขตที่อนุญาต ผล fixtures หรือ hash match ไม่แทน clean-VM lifecycle, publisher signature หรือ physical compatibility
