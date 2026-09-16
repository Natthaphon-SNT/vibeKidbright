# รายงานผลการทดสอบระบบ vibeKidbright — รอบที่ 2

วันที่ทดสอบ: 15 กันยายน 2026  
เวอร์ชันที่ทดสอบ: 3.6.12  
ระบบปฏิบัติการ: Windows x64  
ผู้ทดสอบ: Codex  
ผลรวม: **Conditional Fail — regression หลักยังผ่าน แต่ defect R1-01 ถึง R1-10 ยังไม่ถูกแก้ และยังไม่พร้อมรับรองการใช้งานกับฮาร์ดแวร์จริง**

## 1. ขอบเขตและข้อจำกัด

รอบนี้ทดสอบซ้ำตามรายงานรอบที่ 1 โดยไม่แก้ source code ของโครงการ ใช้การทดสอบอัตโนมัติ, static analysis, release build, installer build และ ESP-IDF compile-only ผ่าน test project ชั่วคราวนอก repository จากนั้นลบ test project และ coverage output แล้ว

สิ่งที่ยังไม่ได้ทดสอบเพราะไม่มีอุปกรณ์หรือสภาพแวดล้อมที่เหมาะสม:

- Flash, Boot, Serial Monitor, input และ peripheral บน KidBright32 จริง
- ADB discovery, deployment, log stream และ input บน MiuAiPlus จริง; เครื่องทดสอบไม่พบ `adb`
- MSI install/uninstall และ clean-install บน disposable Windows VM
- NSIS clean-install/uninstall เพื่อป้องกันผลกระทบต่อเครื่องทำงานหลัก
- AI live request เพราะไม่มี test credential/quota ที่ได้รับอนุญาต; ตรวจเฉพาะ routing/configuration และ public model catalog

ผล compile-only ไม่ถือเป็นหลักฐานว่า firmware boot หรือ peripheral ทำงานจริง

## 2. สรุปการตรวจ defect จากรอบที่ 1

นิยามสถานะ: **Fixed** = แก้ครบและมีหลักฐานทดสอบ, **Partially Fixed** = ดีขึ้นแต่ยังไม่ครบ acceptance criteria, **Not Fixed** = อาการ/สาเหตุหลักยังอยู่, **Regressed** = แย่ลงจากรอบก่อน

| ID | Severity | สถานะรอบ 2 | หลักฐานและผลเทียบรอบ 1 |
|---|---|---|---|
| R1-01 | High | **Not Fixed** | Backend ยัง `spawn()` แล้วให้ detached thread เรียก `child.wait()` แต่คืน `Ok(())` ทันที (`src-tauri/src/esp_idf.rs:1028,1052-1056`) ขณะที่ UI เปลี่ยน `building` เป็น `success` หลังคำสั่ง Tauri คืนค่า (`src/App.tsx:1278-1312`) การคอมไพล์ fixture ที่ตั้งใจให้ผิดคืน exit code 2 แต่ flow ปัจจุบันไม่มีทางส่ง exit code นี้กลับ UI |
| R1-02 | High | **Not Fixed** | Monitor/input แยก KidBright32 กับ MiuAiPlus แล้ว แต่ Build/Flash ไม่มี branch ตาม board; handler ยังเรียก ESP-IDF `build flash` สำหรับทั้งสองชนิด (`src/App.tsx:1278-1314`) |
| R1-03 | High | **Not Fixed** | UI มีตัวเลือกเพียง `kidbright32` และ `miuaiplus` (`src/App.tsx:410,1823-1824`) และ backend กำหนด `IDF_TARGET=esp32` (`src-tauri/src/esp_idf.rs:662,1013`) แต่ README ยังระบุ ESP32-S2/S3/C3/C6 ด้วย Toolchain คอมไพล์ทั้ง 5 target ได้ แต่ UI เข้าถึง target เหล่านี้ไม่ได้ |
| R1-04 | Medium | **Not Fixed** | preset ใน `src/AiChat.tsx` ยังมี `gpt-5.6-astra`, `gpt-4.1-nano`, `o4-mini`, `o3-mini`, `gemini-1.5-pro`, `gemini-1.5-flash`, `gemini-2.0-flash` เหมือนเดิม และยังไม่ใช้ dynamic catalog สำหรับ OpenRouter |
| R1-05 | Medium | **Not Fixed** | `knowledge_base/sensor_examples/all_sensors_demo.c:258-259` ยังไม่เปลี่ยน และ ESP-IDF ปฏิเสธด้วย `-Werror=misleading-indentation`; ตัวอย่าง ESP-IDF ผ่าน 11/12 เท่ารอบแรก พร้อม warning เดิมอีก 2 จุด |
| R1-06 | Medium | **Not Fixed** | ยังมี 5 test files / 85 tests ไม่มี test case ใหม่ Coverage รวมยัง 12.44% statements และ component หลัก `App`, `AiChat`, `ToolchainSetup`, `WikiView` ยัง 0% statements |
| R1-07 | Medium | **Not Fixed** | Source KB 29 ไฟล์ เทียบ bundle KB 18 ไฟล์ ต่างกัน 13 รายการเท่ารอบแรก Workflow มีขั้น merge KB แต่ `check:bundle` ไม่บังคับ source/resource parity จึงยังปล่อย mismatch ผ่านได้ |
| R1-08 | Low | **Not Fixed** | `cargo fmt --check` ยังไม่ผ่าน; Clippy `-D warnings` ยังล้มด้วย 16 issues ใน library และ 18 issues ใน test target เท่ารอบแรก |
| R1-09 | Low | **Not Fixed** | main JS 4,354.81 kB (gzip 1,136.81 kB) และ TypeScript worker 7,031.83 kB เท่ารอบแรก Vite ยังเตือน chunk ใหญ่และ static/dynamic import ของ Tauri event |
| R1-10 | Low | **Not Fixed** | `package.json`, `Cargo.toml`, `tauri.conf.json` ตรงกันที่ 3.6.12 แต่ Winget manifest ล่าสุดใน repo ยังเป็น 3.6.0 |

**สรุป defect เดิม:** Fixed 0, Partially Fixed 0, Not Fixed 10, Regressed 0

## 3. Regression test

| หมวด | คำสั่ง/กรณีทดสอบ | ผลรอบ 2 | เทียบรอบ 1 |
|---|---|---:|---|
| Frontend unit | `npm test -- --reporter=verbose` | **ผ่าน 85/85**, 5/5 files | เท่าเดิม |
| Frontend coverage | `npm run test:coverage` | **ผ่านคำสั่ง**; statements 12.44%, branches 83.80%, functions 69.04%, lines 12.44% | เท่าเดิม |
| Release web build | `npm run build:release` | **ผ่าน**; TypeScript ผ่าน, Vite 1,287 modules, public 199 files, bundle resources 18 files | ผ่านต่อเนื่อง; public count +1 เพราะมี report รอบแรก |
| Dependency audit | `npm audit --offline` | **ผ่าน**, 0 cached vulnerabilities | เท่าเดิม; จำกัดเฉพาะฐานข้อมูล offline cache |
| Rust tests | `cargo test --manifest-path src-tauri/Cargo.toml --all-targets -- --nocapture` | **ผ่าน 50/50**; มี test-code warnings 2 รายการ | เท่าเดิม |
| Rust format | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | **ไม่ผ่าน**; formatting diff จำนวนมาก | เท่าเดิม |
| Rust lint | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | **ไม่ผ่าน**; library 16 / test target 18 | เท่าเดิม |
| Tauri executable | `npm run tauri -- build --no-bundle` | **ผ่าน** | เท่าเดิม |
| Full Tauri bundle | `npm run tauri -- build` | **ไม่ผ่านที่ MSI/WiX `light.exe`** หลัง executable build ผ่าน | เท่าเดิมในเชิงผลลัพธ์; ไม่พบ MSI output |
| NSIS bundle | `npm run tauri -- build --bundles nsis` | **ผ่าน** | เท่าเดิม |
| ESP-IDF smoke | esp32, esp32s2, esp32s3, esp32c3, esp32c6 | **ผ่าน compile-only 5/5** | เท่าเดิม |
| KB ESP-IDF examples | 12 C sources | **ผ่าน 11/12** | เท่าเดิม |

ไม่พบ automated regression ใหม่ที่ล้มเมื่อเทียบกับรอบแรก แต่ quality gates และ MSI limitation เดิมยังคงอยู่

## 4. รายละเอียด automated tests และ coverage

Frontend tests ผ่านทั้งหมด 85 tests ใน 5 files โดย test files ยังคงเป็นชุดเดิม:

- `codeEditorDiff.test.tsx`
- `utils.test.ts`
- `toast.test.ts`
- `buildErrorList.test.ts`
- `errorHints.test.ts`

| Component/ภาพรวม | Statements |
|---|---:|
| ทั้งโครงการ | 12.44% |
| `src/App.tsx` | 0% |
| `src/AiChat.tsx` | 0% |
| `src/ToolchainSetup.tsx` | 0% |
| `src/WikiView.tsx` | 0% |

เส้นทางเสี่ยงสูง เช่น board switching, Build/Flash lifecycle, process failure, toolchain missing, AI provider switching และ no-device behavior จึงยังไม่มี automated regression protection

Rust tests ผ่าน 50/50 แต่ `fmt` และ Clippy ยังไม่ผ่าน โดยกลุ่ม lint เดิมรวมถึง unused code/import, manual flatten/split, non-minimal boolean, `saturating_sub`, `lines().flatten()`, too many arguments และ identical branches

## 5. Firmware compile-only matrix

ใช้ ESP-IDF 5.5.4 สร้าง smoke project แยก directory ต่อ target เพื่อไม่ให้ `sdkconfig` และ build cache ปะปนกัน

| Target | Compile | App binary | Partition remaining | Flash/Boot |
|---|---:|---:|---:|---:|
| ESP32 | ผ่าน | 0x22460 bytes | 87% | ไม่ได้ทดสอบ |
| ESP32-S2 | ผ่าน | 0x20160 bytes | 87% | ไม่ได้ทดสอบ |
| ESP32-S3 | ผ่าน | 0x27470 bytes | 85% | ไม่ได้ทดสอบ |
| ESP32-C3 | ผ่าน | 0x213a0 bytes | 87% | ไม่ได้ทดสอบ |
| ESP32-C6 | ผ่าน | 0x1d060 bytes | 89% | ไม่ได้ทดสอบ |

หมายเหตุ: การพยายาม build หลาย target โดยใช้ project directory เดียวพร้อมกันทำให้เกิด `sdkconfig` target mismatch ซึ่งเป็นข้อผิดพลาดของ test harness ไม่ใช่ defect ของแอป เมื่อแยก directory แล้วทั้ง 5 target ผ่าน

## 6. Expanded tests และการแยก Hardware/VM

| กรณี | วิธีทดสอบรอบนี้ | ผล | ระดับหลักฐาน |
|---|---|---|---|
| Intentional compile error | สร้าง fixture ที่มี syntax/identifier error แล้วเรียก ESP-IDF โดยตรง | process คืน exit 2; compiler/ninja รายงาน failure ถูกต้อง แต่ app backend ยังคืน success หลัง spawn | Compile + static app analysis |
| Invalid board/target | ตั้ง `IDF_TARGET=esp999` กับ project ที่กำหนด esp32 | ESP-IDF ปฏิเสธและคืน exit 2 เพราะ target mismatch | Compile tool error-path only; ไม่ใช่ UI black-box |
| Missing toolchain | ตรวจเส้นทาง resolve/error ใน source | มี error path ฝั่ง backend แต่ไม่มี UI/component test และไม่ได้ย้าย toolchain จริงออกจากเครื่อง | Static only |
| Board type isolation | ตรวจ state/branch ใน `App.tsx` และ backend | Monitor/input มี KidBright/MiuAiPlus branch แต่ Build/Flash ไม่แยก board และ target ESP32 family ไม่อยู่ใน UI | Static only |
| KidBright32 V1.3/V1.5/V1.6 | ใช้ compile target esp32 ร่วม | Compile ผ่าน แต่ยืนยัน pin map/peripheral/boot ราย revision ไม่ได้ | Compile-only; hardware required |
| MiuAiPlus | ตรวจ ADB path ใน source; เครื่องไม่มี `adb` และไม่มี device | ไม่ได้ทดสอบ deployment/log/input จริง | Static only; hardware required |
| AI providers | ตรวจ preset/routing และเทียบเอกสาร catalog ทางการ | พบ stale/invalid preset เดิม; ไม่ได้ส่ง request จริง | Static/catalog only; credential required |
| MSI package | Full Tauri build | WiX `candle` ทำงาน แต่ `light.exe` ล้ม; ไม่สร้าง MSI | Build environment limitation; ต้องยืนยันบน working Windows VM |
| NSIS install/uninstall | สร้าง installer สำเร็จ | ไม่ได้ติดตั้งบนเครื่องหลัก | Disposable VM required |

## 7. Knowledge Base parity และ sample compilation

### 7.1 Source เทียบ bundled resources

- `knowledge_base`: 29 files
- `resources/knowledge_base`: 18 files
- รายการที่ไม่ตรงกัน: 13
- ชื่อเหมือนแต่เนื้อหาต่างกัน: `README.md`, `all_models.md`
- source-only 11 รายการ:
  - `.kb_store.sqlite`
  - `hardware_schematics_rules.md`
  - `minibike.md`
  - `sensor_examples/20190923105536_5d883db8de154.pdf`
  - `sensor_examples/Bike_Controller_Schematic_V1.0.pdf`
  - `sensor_examples/calibrate_balancing_bike.c`
  - `sensor_examples/calibrate_balancing_joystick.c`
  - `sensor_examples/KBminibike-schematic-2131758754.pdf`
  - `sensor_examples/minibike_receiver.c`
  - `sensor_examples/minibike_sender.c`
  - `sensor_examples/PCB_SCH_V1.3_2018.pdf`

CI workflows มีขั้น merge knowledge base แต่ไม่มี parity gate ที่ทำให้ build ล้มเมื่อสอง directory ไม่ตรงกัน จึงยังถือว่า R1-07 ไม่ถูกแก้

### 7.2 Sample compile

ตัวอย่าง ESP-IDF 12 ไฟล์ผ่าน 11 และล้ม 1:

```text
all_sensors_demo.c:258:5: error: this 'if' clause does not guard... [-Werror=misleading-indentation]
all_sensors_demo.c:259:5: error: this 'if' clause does not guard... [-Werror=misleading-indentation]
```

Warning ที่ยังพบ:

- `fomulakid_receiver.c:176`: unused variable `d`
- `minibike_receiver.c:203`: unused variable `accX`

เมื่อตัดเฉพาะ `all_sensors_demo.c` ออกจาก fixture อีก 11 ไฟล์ build สำเร็จ `balanced_robot.c` เป็น Arduino example และไม่รวมใน ESP-IDF compile เพราะต้องใช้ Arduino libraries เพิ่มเติม

## 8. AI model catalog validation

รายการ preset ใน UI ยังไม่เปลี่ยนจากรอบแรก การตรวจรอบนี้อิงเอกสารทางการ ณ วันที่ทดสอบ:

- OpenAI ระบุ Astra ปัจจุบันเป็น `gpt-6-astra` และมี `gpt-5.6-sol`; UI ยังใช้ `gpt-5.6-astra`
- `gpt-4.1-nano`, `o4-mini`, `o3-mini` แสดงเป็น deprecated ใน OpenAI model catalog
- Gemini changelog ระบุ Gemini 1.5 Pro/Flash ปิด 29 กันยายน 2025 และ Gemini 2.0 Flash ปิด 1 มิถุนายน 2026
- OpenRouter มี Models API แบบ dynamic แต่ UI ยัง hard-code รายการ จึงไม่สามารถรับรองว่าทุก preset ใช้งานได้ตลอด

แหล่งอ้างอิง:

- [OpenAI Models](https://developers.openai.com/api/docs/models)
- [OpenAI All models](https://developers.openai.com/api/docs/models/all)
- [Gemini API changelog](https://ai.google.dev/gemini-api/docs/changelog)
- [OpenRouter Models API](https://openrouter.ai/docs/api/api-reference/models/list-all-models-and-their-properties)

## 9. Release และ installer artifacts

| Artifact | ผล | ขนาด | SHA-256 |
|---|---:|---:|---|
| `src-tauri/target/release/vibe-kidbright.exe` | ผ่าน | 44,745,216 bytes | `27CD68B36A7E116783981D2B43E70908AB20B97E7EE35DA59A1DD3BCB3757CB4` |
| `src-tauri/target/release/bundle/nsis/VibeKidbright IDE_3.6.12_x64-setup.exe` | ผ่าน | 13,101,088 bytes | `4ABDF7124A90FB38F9F878C858DFF6FF719AEA783D8BCA726B9B65BBAEA0C576` |
| `src-tauri/target/release/bundle/msi/VibeKidbright IDE_3.6.12_x64_en-US.msi` | ไม่ถูกสร้าง | — | — |

Full bundle ล้มที่:

```text
failed to bundle project `failed to run ...\WixTools314\light.exe`
```

ขณะตรวจหลัง build บริการ `msiserver` แสดงสถานะ Running แต่ `light.exe` ยังคืน failure โดยไม่มี diagnostic เพิ่มใน output รอบนี้ จึงยังสรุป root cause ไม่ได้ และไม่ควรถือว่า MSI ผ่านจนกว่าจะทดสอบบน Windows VM ที่ WiX/Windows Installer ทำงานสมบูรณ์

Bundle size ยังเท่ารอบแรก:

- main JavaScript: 4,354.81 kB; gzip 1,136.81 kB
- TypeScript Monaco worker: 7,031.83 kB
- ยังมี Vite warning สำหรับ chunk มากกว่า 500 kB

## 10. Version consistency

| แหล่ง | เวอร์ชัน |
|---|---:|
| `package.json` | 3.6.12 |
| `src-tauri/Cargo.toml` | 3.6.12 |
| `src-tauri/tauri.conf.json` | 3.6.12 |
| NSIS filename | 3.6.12 |
| Winget manifest ล่าสุดใน repository | 3.6.0 |

Core package metadata ตรงกัน แต่ distribution metadata ของ Winget ตามหลัง 12 patch versions

## 11. Defect ใหม่จากรอบ 2

| ID | Severity | รายการ | สถานะ |
|---|---|---|---|
| — | — | ไม่พบ defect ใหม่ที่แยกจาก R1-01 ถึง R1-10 ได้อย่างมีหลักฐาน | — |

MSI build failure, lack of hardware/ADB/VM และ shared-directory `sdkconfig` mismatch ถูกจัดเป็นข้อจำกัดของสภาพแวดล้อม/test harness ไม่เปิดเป็น R2 defect

## 12. Verdict และคำแนะนำก่อน hardware acceptance

**Verdict: Conditional Fail / Not ready for full hardware acceptance**

เหตุผลหลัก:

1. R1-01 และ R1-02 เป็น High severity และยังอยู่: UI อาจรายงาน Build/Flash สำเร็จก่อน process จบ และ MiuAiPlus ยังวิ่งผ่าน ESP-IDF Build/Flash flow
2. R1-03 ยังทำให้คำกล่าวอ้าง ESP32-S2/S3/C3/C6 ไม่สอดคล้องกับ UI/backend แม้ compiler toolchain รองรับ
3. Component หลักมี coverage 0% และไม่มี test ใหม่มาป้องกัน lifecycle/error paths
4. KB sample ยัง compile ไม่ครบ, KB bundle ยังไม่ parity, fmt/Clippy ยังไม่ผ่าน และ AI preset ยังล้าสมัย
5. ยังไม่มีหลักฐานจาก Flash/Boot/Serial/ADB/peripheral จริง และ MSI/clean install ยังไม่ผ่านการทดสอบบน VM

เกณฑ์ขั้นต่ำก่อนรอบ hardware acceptance:

- ให้ backend await child process และส่ง exit status/error กลับ UI พร้อม automated test สำหรับ success/nonzero exit
- แยก Build/Flash ตาม board type; MiuAiPlus ต้องใช้ deployment flow ที่ถูกต้อง
- ทำ UI target support ให้ตรง README หรือแก้ README ให้ตรงความสามารถจริง
- เพิ่ม component/integration tests สำหรับ `App`, `AiChat`, `ToolchainSetup`, `WikiView`
- แก้ KB sample, เพิ่ม deterministic parity gate ใน CI, ทำ fmt/Clippy ให้ผ่าน และปรับ AI catalog
- สร้างและ clean-install ทั้ง MSI/NSIS บน disposable VM
- ทำ hardware matrix อย่างน้อย KidBright32 V1.3/V1.5/V1.6 และ MiuAiPlus ครอบคลุม Flash, Boot, monitor/input, failure recovery และ peripheral สำคัญ

