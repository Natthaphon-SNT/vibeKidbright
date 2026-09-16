# รายงานผลการทดสอบระบบ vibeKidbright — รอบที่ 3

ช่วงเวลาทดสอบ: 15–16 กันยายน 2026  
เวอร์ชันที่ทดสอบ: 3.6.12  
ระบบปฏิบัติการ: Windows 10.0.26200 x64  
ผู้ทดสอบ: Codex  
ผลรวม: **Conditional Fail — ไม่มี source change ที่เกี่ยวข้องหลังรอบ 2, defect เดิมยังเปิดครบ 10 ข้อ และยังไม่พร้อมเข้าสู่ full hardware acceptance**

## 1. Executive summary

**รอบนี้ไม่มี commit ใหม่หรือการเปลี่ยน source ที่เกี่ยวข้องกับ R1-01 ถึง R1-10 หลังรายงานรอบ 2**

- HEAD ล่าสุดยังเป็น `1669a5da10360bc6d623f5093422d59b50d7f4f3` วันที่ 9 กันยายน 2026
- รายงานรอบ 2 ถูกสร้างเวลา 15 กันยายน 2026 21:18:11 +07:00
- `esp_idf.rs`, `App.tsx`, `AiChat.tsx`, `all_sensors_demo.c`, `Cargo.toml`, `Readme.md` และ Winget manifests ไม่มีไฟล์ใดมีเวลาแก้ไขหลังรายงานรอบ 2
- working tree มีการแก้ไขค้างอยู่ตั้งแต่ก่อนรอบ 2 แต่ไม่มี commit/delta ใหม่ระหว่างรอบ 2 กับรอบ 3

ตามเงื่อนไขของแผนทดสอบรอบ 3 จึงรัน regression แบบย่อเพื่อยืนยันว่าไม่มี drift และเปลี่ยนน้ำหนักหลักไปที่ root-cause analysis กับ MSI diagnostics ไม่รัน full test matrix และ firmware compile 5 targets ซ้ำโดยไม่มี source delta

ผลสำคัญเพิ่มเติมคือ MSI failure จากรอบก่อนถูกแยกสาเหตุได้แล้ว: WiX ICE validation ล้มเมื่อรันภายใต้บัญชี sandbox `CodexSandboxOffline` แต่ผ่านเมื่อรันคำสั่งเดิมภายใต้บัญชีผู้ใช้ปกติ `Acer` แม้ทั้งสองบริบทจะไม่ใช่ Administrator ดังนั้น failure เดิมเป็นข้อจำกัดด้าน identity/access ของ sandbox ไม่ใช่ข้อกำหนดว่าต้อง build แบบ Administrator และไม่ใช่ linker/config failure โดยตรง

## 2. วัตถุประสงค์และขอบเขต

1. ตรวจว่ามีการแก้ source หลังรอบ 2 หรือไม่ก่อนรันชุดทดสอบ
2. ยืนยัน regression แบบย่อเมื่อไม่มี source delta
3. วิเคราะห์ root cause และเสนอแนว patch ที่เฉพาะเจาะจงโดยไม่แก้ source จริง
4. วินิจฉัย MSI/ICE validation ให้ลึกกว่ารอบก่อน
5. ระบุ defect ใหม่และขั้นตอนที่ต้องเกิดขึ้นก่อนส่งเข้าทดสอบรอบถัดไป

รอบนี้ไม่มีการแก้ source code การสร้าง MSI diagnostic output เป็น build artifact ชั่วคราวและถูกลบหลังเก็บผลแล้ว

## 3. สภาพแวดล้อม

| รายการ | เวอร์ชัน/สถานะ |
|---|---|
| Node.js | 24.12.0 |
| npm | 11.7.0 |
| Rust/Cargo | 1.94.1 |
| Python | 3.14.0 |
| Tauri | major version 2 (`^2`) |
| WiX Toolset | 3.14.1.8722 |
| Windows Installer | 5.0.26100.1 |
| `msiserver` | Running, Manual |
| Process sandbox | `LAPTOP-UD8ED6TS\CodexSandboxOffline`, non-admin, Medium integrity |
| Process host test | `LAPTOP-UD8ED6TS\Acer`, non-admin |
| ADB | ไม่พบใน PATH |
| Serial/board | ไม่พบอุปกรณ์ |

## 4. สถานะ defect เทียบ 3 รอบ

| ID | Severity | รอบ 1 | รอบ 2 | รอบ 3 | Trend / หลักฐานรอบ 3 |
|---|---:|---|---|---|---|
| R1-01 | High | Open | Not Fixed | **Not Fixed** | ไม่มี source delta; backend ยังคืน `Ok(())` หลัง spawn และ UI ยัง mark success ใน `finally` |
| R1-02 | High | Open | Not Fixed | **Not Fixed** | ไม่มี source delta; Build/Flash ยังไม่มี branch ตาม `selectedBoard` |
| R1-03 | High | Open | Not Fixed | **Not Fixed** | ไม่มี source delta; UI มี 2 board types และ backend hard-code `IDF_TARGET=esp32` 2 จุด |
| R1-04 | Medium | Open | Not Fixed | **Not Fixed** | ไม่มี source deltaใน `AiChat.tsx`; static preset เดิมยังอยู่ |
| R1-05 | Medium | Open | Not Fixed | **Not Fixed** | `all_sensors_demo.c` ไม่มีการแก้หลังรอบ 2 |
| R1-06 | Medium | Open | Not Fixed | **Not Fixed** | ไม่มี test source ใหม่; ชุดเดิมยัง 5 files / 85 tests |
| R1-07 | Medium | Open | Not Fixed | **Not Fixed** | workflow/script ไม่มี delta; `check:bundle` ยังตรวจชนิดไฟล์/secret แต่ไม่เทียบ file list และ hash |
| R1-08 | Low | Open | Not Fixed | **Not Fixed** | Cargo/Rust source ไม่มี delta; ไม่รัน fmt/Clippy ซ้ำตาม abbreviated-test rule |
| R1-09 | Low | Open | Not Fixed | **Not Fixed** | build ใหม่ยังได้ main 4,354.81 kB และ TS worker 7,031.83 kB |
| R1-10 | Low | Open | Not Fixed | **Not Fixed** | app 3.6.12 แต่ Winget ล่าสุดยัง 3.6.0 |

สรุปหลัง 3 รอบ: **Fixed 0 / Partially Fixed 0 / Not Fixed 10 / Regressed 0**

สถานะรอบ 3 ไม่ได้มาจากการรันทุกกรณีซ้ำ แต่ยืนยันจากการไม่มี source delta, static spot-check และ regression ย่อ ผลนี้เพียงพอต่อการสรุปว่า defect ที่ต้องแก้ด้วย source change ไม่อาจเปลี่ยนเป็น Fixed ได้

## 5. Regression แบบย่อ

| หมวด | คำสั่ง | ผลรอบ 3 | เทียบรอบ 2 |
|---|---|---:|---|
| Frontend tests | `npm test -- --reporter=verbose` | **ผ่าน 85/85**, 5/5 files | ไม่ drift |
| Rust tests | `cargo test --manifest-path src-tauri/Cargo.toml --all-targets -- --nocapture` | **ผ่าน 50/50**, warnings เดิม 2 รายการ | ไม่ drift |
| Release frontend | `npm run build:release` | **ผ่าน**, 1,287 modules | ไม่ drift |
| Main JS | Vite production output | 4,354.81 kB; gzip 1,136.82 kB | ไม่ลดลงอย่างมีนัยสำคัญ |
| TypeScript worker | Vite production output | 7,031.83 kB | เท่าเดิม |
| MSI diagnostic build | `npm run tauri -- build --bundles msi --verbose` | executable ผ่าน; ICE validation ล้มใน sandbox | วินิจฉัยต่อในหัวข้อ 8 |

คำสั่งที่ตั้งใจไม่รันซ้ำเพราะไม่มี source delta: coverage, npm audit, fmt, Clippy, Tauri no-bundle/full/NSIS และ ESP-IDF 5-target/sample matrix ผลอ้างอิงล่าสุดยังเป็นรายงานรอบ 2 และไม่มีไฟล์ที่เกี่ยวข้องเปลี่ยนหลังจากนั้น

## 6. Root-cause analysis

> **ข้อเสนอแนะเชิงเทคนิคในหัวข้อนี้ไม่ใช่การแก้ไขจริง และยังต้องผ่าน code review กับ automated/hardware tests หลังนำไปพัฒนา**

### 6.1 R1-01 — Build/Flash lifecycle คืน success ก่อน process จบ

#### Root cause

`run_shell_command` ใน `src-tauri/src/esp_idf.rs:946-1056` ใช้ `std::process::Command`, เปิด thread อ่าน stdout/stderr และเปิด thread ที่สามเรียก `child.wait()` แต่ไม่ส่งผลของ `ExitStatus` กลับ caller จากนั้น Tauri command คืน `Ok(())` ทันที

ผลกระทบไม่ได้มีเพียงข้อความ success ผิด:

- `App.tsx:1304` resolve เกือบทันทีหลัง spawn
- `finally` ที่ `App.tsx:1309-1313` ปลด `isBuilding` และเปลี่ยน `building` เป็น `success`
- ผู้ใช้สามารถเริ่ม build ซ้อนในขณะที่ process แรกยังทำงาน
- error ที่มาจาก terminal event แข่งกับ state update ใน `finally` ทำให้สถานะขึ้นกับ timing
- command result ไม่มี exit code, signal, duration หรือ job identity ให้ทดสอบได้

#### แนว patch ที่แนะนำ

ทางเลือกขั้นต่ำคือเปลี่ยน return type จาก `Result<(), String>` เป็น structured outcome และรอ child จริง:

```rust
#[derive(Clone, Serialize)]
struct CommandOutcome {
    success: bool,
    code: Option<i32>,
}

// std::process::Child::wait เป็น blocking จึงย้ายไป spawn_blocking
let status = tauri::async_runtime::spawn_blocking(move || child.wait())
    .await
    .map_err(|e| format!("wait task failed: {e}"))?
    .map_err(|e| format!("process wait failed: {e}"))?;

let outcome = CommandOutcome {
    success: status.success(),
    code: status.code(),
};

if outcome.success { Ok(outcome) }
else { Err(format!("command exited with {:?}", outcome.code)) }
```

ต้อง clone `AppHandle` สำหรับ stdout, stderr และ completion ก่อนย้าย ownership เข้าแต่ละ thread อีกทางที่เหมาะกับงานยาวและการ cancel คือคืน `job_id` ทันที แล้ว background task emit `command-finished` payload `{ jobId, success, code }`; UI ต้อง correlate event ด้วย `jobId`

ฝั่ง `App.tsx` ต้อง:

- ตั้ง `success` เฉพาะเมื่อ outcome success หรือได้รับ completion event ของ job ปัจจุบัน
- ตั้ง `failed` จาก nonzero exit ไม่ใช่อาศัยการ parse ข้อความ log
- ให้ `finally` ทำเพียง cleanup เช่น `setIsBuilding(false)` ห้ามตัดสิน success
- unsubscribe completion listener และกัน stale event จาก job เก่า

Acceptance tests ขั้นต่ำ: exit 0, exit 2, spawn error, toolchain missing, stdout/stderr interleave, double-click build และ completion event จาก job เก่า

### 6.2 R1-02 — MiuAiPlus ใช้ ESP-IDF Build/Flash flow

#### Root cause

board selector ถูกเพิ่มในขอบเขต terminal/monitor แต่ไม่ได้ถูกออกแบบเป็น capability model ทั้งระบบ `selectedBoard` ถูกใช้ที่ `App.tsx:815`, `1040`, `1054`, `1093` สำหรับ ADB/serial แต่ไม่ถูกใช้ใน `handleBuildFlash` ที่ `1278-1314`

นอกจากนี้ `run_shell_command` resolve ESP-IDF และ Python ที่บรรทัด 953-955 ก่อน branch เลือก `adb` ที่ 992 ทำให้แม้เรียก ADB ผ่าน generic command ก็ยังพึ่ง ESP-IDF โดยไม่จำเป็น

#### แนว patch ที่แนะนำ

เพิ่ม branch ก่อนสร้าง `flashArgs`:

```ts
if (selectedBoard === "miuaiplus") {
  if (!selectedAdbDevice) throw new Error("No MiuAiPlus ADB device selected");
  await invoke("deploy_miuai", { device: selectedAdbDevice, /* artifact */ });
} else {
  await runIdfWrappedCommand("idf.py", ["build", "flash", "-p", selectedSerialPort], projectDir);
}
```

ปัจจุบันไม่มี deployment command สำหรับ MiuAiPlus; `start_adb_monitor` และ `send_adb_input` เป็นเพียง logcat/shell input ไม่ควรถูกใช้แทน deployment ทีมต้องกำหนด artifact และ device-side contract ก่อน แล้วเพิ่ม `deploy_miuai` ใน backend โดย reuse `find_adb`, `list_adb_devices` และ device selection จาก path ปัจจุบัน หาก contract ยังไม่พร้อม quick safety fix คือ disable ปุ่ม Build & Flash เมื่อเลือก MiuAiPlus พร้อมข้อความ “deployment not supported” แทนการ flash ผิดระบบ

ควรแยก generic shell, ESP-IDF และ ADB runners เพื่อไม่ให้ ADB พึ่ง `resolve_idf_paths()` และควรใช้ allowlisted arguments แทนการประกอบ shell string

### 6.3 R1-03 — target support ไม่ตรง README

#### Root cause

ระบบผูกแนวคิด “board type” กับ transport เพียง 2 ค่า แต่ไม่มี state สำหรับ ESP chip target ขณะที่ backend hard-code `IDF_TARGET=esp32` สองจุด (`run_idf_command` บรรทัด 662 และ `run_shell_command` บรรทัด 1013) README จึงอ้าง toolchain capability เป็น application capability

#### ความยากและทางเลือก

- **Quick win ระยะสั้น:** แก้ README ให้ระบุว่า UI รองรับ firmware target เฉพาะ ESP32/KidBright32 และระบุ S2/S3/C3/C6 เป็นเพียง toolchain compile capability ใช้การเปลี่ยนเอกสารจุดเดียวและลด false claim ทันที
- **Feature ที่ถูกต้องระยะกลาง:** เพิ่ม `EspTarget = esp32 | esp32s2 | esp32s3 | esp32c3 | esp32c6`, แยกจาก `BoardType`, เพิ่ม target selector, ส่ง target เข้า backend, validate ด้วย allowlist และแทน hard-code 2 จุด ต้องจัดการ `sdkconfig`/build directory ต่อ target และเรียก `idf.py set-target` เมื่อเปลี่ยน targetเพื่อป้องกัน mismatch

โครงสร้างปัจจุบันทำให้ feature นี้ไม่ใช่แค่เพิ่ม `<option>`; ต้องแก้ data model, Tauri command contract, environment setup, cache isolation และ tests

### 6.4 R1-06 — component coverage 0%

#### Root cause

logic สำคัญถูกรวมอยู่ใน `App.tsx` และผูกกับ Tauri `invoke/listen`, timers, localStorage และ state หลายชุด ทำให้ unit test ยาก Test suite ปัจจุบันจึงเน้น pure utilities และ component ย่อย ไม่ครอบคลุม orchestration

#### ลำดับ test ที่ควรทำ

1. `App.tsx` Build/Flash failure-path และ success-path โดย mock `invoke`/`listen` — ปิด R1-01 ก่อน
2. board routing: KidBright32 เรียก ESP-IDF, MiuAiPlus ไม่เรียก ESP-IDF — ปิด R1-02
3. double-submit lock, stale completion event, no project/port/device และ toolchain missing
4. `ToolchainSetup.tsx`: ready, missing, repair failure, cancel/retry
5. `AiChat.tsx`: provider switch, invalid/deprecated model, missing API key และ request error
6. `WikiView.tsx`: missing KB, navigation/search และ malformed content

ควร extract process orchestration เป็น service/hook ที่รับ dependency (`invoke`, event source) เพื่อทดสอบ state machine โดยไม่ mount `App` ทั้งหน้า แล้วมี integration tests จำนวนน้อยครอบ DOM จริง

### 6.5 R1-07 — ไม่มี deterministic KB parity gate

#### Root cause

workflow ใช้ copy-overlay จาก external KB แล้วตามด้วย local KB แต่ไม่ล้าง destination และไม่ตรวจ collision ส่วน `scripts/check-public.mjs --bundle` ตรวจเฉพาะ extension/secret/path ของไฟล์ใน `resources/knowledge_base`; ไม่อ่าน `knowledge_base` เพื่อเทียบชื่อหรือ hash จึงรายงานผ่านแม้สองชุดไม่ตรงกัน

#### Mechanism ที่ทำได้จริง

1. ประกาศ source of truth ให้ชัด: local-only หรือ deterministic union ระหว่าง external/local
2. sync ไป staging directory ว่าง ไม่ overlay บนไฟล์เก่า
3. normalize relative path ด้วย `/`, ตัด allowlist เช่น `.embeddings.json`, backup และ local DB
4. สร้าง manifest `{ relativePath, sha256, origin }` โดย sort path ก่อน
5. fail เมื่อ source file หายจาก bundle, hash ต่าง, มี unexpected extra หรือ external/local ชื่อชนกันแต่ hash ต่าง
6. รัน gate หลัง merge และก่อน Tauri build ทั้ง Windows/macOS

รูปแบบคำสั่งที่แนะนำคือ `node scripts/check-kb-parity.mjs --source knowledge_base --bundle resources/knowledge_base` และเพิ่ม test fixture สำหรับ missing, extra, content mismatch, collision และ excluded files

### 6.6 Root cause/next fix สำหรับ defect ที่เหลือ

| ID | Root cause | การแก้ที่เจาะจง |
|---|---|---|
| R1-04 | model lists hard-code ใน JSX ไม่มี lifecycle/catalog validation | แยก catalog เป็น typed config, mark deprecated, เพิ่ม catalog test; OpenRouter ใช้ Models API/cache และ fallback เมื่อ offline |
| R1-05 | สอง `if` อยู่บรรทัดเดียวภายใต้ `-Werror=misleading-indentation` | แยก statement/ใส่ braces และ compile sample matrix ใน CI; แก้ unused variables อีก 2 จุด |
| R1-08 | CI รัน tests แต่ไม่มี fmt/Clippy warning gate | เพิ่ม `cargo fmt --check` และ `cargo clippy --all-targets --all-features -- -D warnings` ก่อน build; cleanup baseline ครั้งเดียว |
| R1-09 | `App.tsx` static-import `CodeEditor`, `CodeEditor.tsx` import Monaco และไม่มี `manualChunks` | ใช้ `React.lazy(() => import("./CodeEditor"))` พร้อม Suspense และกำหนด Monaco worker/vendor chunks; ตั้ง bundle budget ใน CI |
| R1-10 | release workflowอ่าน version จาก Tauri config แต่ไม่ generate/validate Winget manifest | เพิ่ม version-consistency script และ generate Winget path/URL/hash จาก release artifact; fail CI เมื่อไม่ตรง |

## 7. MSI diagnostics เชิงลึก

### 7.1 ผลใน sandbox

`npm run tauri -- build --bundles msi --verbose` แสดงคำสั่งจริงของ WiX และล้มใน `light.exe` ระหว่าง `Validating database`:

```text
LGHT0217: Error executing ICE action 'ICE01' ... Windows Installer Service could not be accessed
LGHT0217: Error executing ICE action 'ICE02' ...
...
LGHT0217: Error executing ICE action 'ICE07' ...
LGHT0216: unexpected Win32 exception 0x643: ICE09 Fatal error during installation
```

การเรียก `light.exe -v -notidy` โดยตรงให้ process exit 216 และยืนยัน sequence ว่า update file information, cabinet creation, database generation และ module merge ผ่าน ก่อนล้มเฉพาะ validation

### 7.2 Control test ด้วย `-sval`

คำสั่งเดียวกันเมื่อเพิ่ม `-sval` เพื่อ suppress MSI validation:

- exit 0
- สร้าง MSI 17,563,648 bytes
- ยืนยันว่า input `.wixobj`, localization, extensions, cabinet generation และ layout ทำงานได้

`-sval` ใช้เพื่อวินิจฉัยเท่านั้น ไม่ควรใช้เป็น production fix เพราะจะข้าม ICE quality checks

### 7.3 Process privilege และ host-context test

| บริบท | Admin | ผล ICE validation |
|---|---:|---:|
| `CodexSandboxOffline` | ไม่ใช่ | ล้ม: service inaccessible |
| `Acer` นอก sandbox | ไม่ใช่ | ผ่านและสร้าง MSI 17,563,648 bytes |

ดังนั้น Administrator privilege ไม่ใช่เงื่อนไขที่ขาด บริบท sandbox มี token/identity และ service-access policy ต่างจากผู้ใช้ host แม้อยู่ Medium integrity เหมือน process ปกติ

ข้อมูลประกอบ:

- `msiserver` แสดง Running
- VBScript engine CLSID ชี้ไป `C:\Windows\System32\vbscript.dll`
- JScript engine CLSID ชี้ไป `C:\Windows\System32\jscript.dll`
- ไม่พบ MsiInstaller event ใน Application log ของช่วงทดสอบจากบริบท sandbox
- ตรวจ Microsoft Defender status ไม่ได้เพราะ Access denied และไม่ได้ปิด antivirus/security software เนื่องจากไม่เหมาะสมต่อเครื่องใช้งานร่วม; host-context control test ให้คำตอบโดยไม่ต้องลดการป้องกันระบบ

**ข้อสรุป MSI failure เดิม:** ปิดประเด็น “ต้องเป็น Administrator” และ “Tauri link config ล้ม” ได้ สาเหตุที่มีหลักฐานสูงสุดคือ service/COM access ถูกจำกัดเฉพาะ sandbox account อย่างไรก็ตาม release CI/VM ยังต้องสร้าง MSI โดยไม่ใช้ `-sval` เพื่อยืนยัน reproducibility

### 7.4 Validation warnings ที่พบเมื่อรันใน host context

เมื่อ ICE ทำงานได้จริง พบ warning สี่รายการ:

1. ICE03: `DownloadAndInvokeBootstrapper` target string ยาวเกินข้อกำหนดของ `CustomAction.Target` (`main.wxs:210`)
2. ICE40: กำหนด `REINSTALLMODE=amus` ใน Property table (`main.wxs:33`)
3. ICE57: `CMP_UninstallShortcut` ผสม per-user HKCU key path กับ per-machine data (`main.wxs:116-132`)
4. ICE61: upgrade policy ไม่มี Maximum และ `AllowDowngrades=yes` (`main.wxs:40`)

ไฟล์ `main.wxs` เป็น generated output แนวแก้ควรเริ่มจากอัปเดต Tauri 2 ให้เป็น patch ล่าสุดและตรวจ bundle config; ถ้ายังเกิดให้ใช้ WiX template ที่ review/ทดสอบแล้ว ไม่ควร patch generated file หลัง build

## 8. Defect ใหม่รอบ 3

รายการต่อไปนี้เป็น build-quality defect/risk ที่ยืนยันจาก ICE warning แต่ผลกระทบ runtime ยังต้องทดสอบด้วย install/upgrade matrix บน VM

| ID | Severity | รายการ | หลักฐาน | สถานะ |
|---|---:|---|---|---|
| R3-01 | Medium | WebView2 bootstrapper custom-action command เกิน MSI schema length อาจทำให้ fallback install ล้มบนเครื่องที่ไม่มี WebView2 | ICE03, generated `main.wxs:210` | Open |
| R3-02 | Medium | MSI อนุญาต downgrade และไม่มี maximum version อาจให้ installer เก่าแทนที่รุ่นใหม่ | ICE61, `MajorUpgrade AllowDowngrades="yes"` | Open |
| R3-03 | Low | per-machine uninstall component ใช้ HKCU key path ทำให้ repair/uninstall แบบหลายผู้ใช้ไม่สม่ำเสมอ | ICE57, `CMP_UninstallShortcut` | Open |
| R3-04 | Low | explicit `REINSTALLMODE=amus` อาจสร้างพฤติกรรม repair/update ที่ไม่คาดหมาย | ICE40 | Open |

## 9. สิ่งที่ยังทดสอบไม่ได้

- KidBright32 V1.3/V1.5/V1.6: Flash, Boot, Serial, input, pin/peripheral และ recovery
- MiuAiPlus: ADB discovery, deploy, logcat, input และ recovery
- ESP32-S2/S3/C3/C6 ผ่าน UI จริง
- MSI: clean install, repair, upgrade, downgrade, uninstall, multi-user และเครื่องที่ไม่มี WebView2
- NSIS: clean install/uninstall บน disposable VM
- AI provider live request เพราะไม่มี test credential/quota ที่ได้รับอนุญาต
- Antivirus-off comparison; ไม่ลดการป้องกันบนเครื่องใช้งานหลัก

## 10. คำสั่งหลักที่ใช้

```powershell
git log -n 8 --date=iso-strict
git status --short
Get-FileHash <relevant-files> -Algorithm SHA256

npm test -- --reporter=verbose
cargo test --manifest-path src-tauri/Cargo.toml --all-targets -- --nocapture
npm run build:release
npm run tauri -- build --bundles msi --verbose

light.exe -v -notidy -ext WixUIExtension.dll -ext WixUtilExtension.dll `
  -o output-verbose.msi -cultures:en-us -loc locale.wxl main.wixobj

light.exe -v -sval -ext WixUIExtension.dll -ext WixUtilExtension.dll `
  -o output-sval.msi -cultures:en-us -loc locale.wxl main.wixobj
```

ทำ host-context control ด้วย `light.exe -v` คำสั่งเดียวกันโดยไม่ใช้ `-sval` และลบ diagnostic MSI/WixPDB กับ WiX temp directories หลังเก็บผล

## 11. Verdict และขั้นตอนถัดไป

**Verdict: Conditional Fail / Not ready for full hardware acceptance**

การมีผล **0 fixed หลัง 3 รอบ** ไม่ได้สะท้อนเพียงคุณภาพตัวแอป แต่เป็นสัญญาณว่า defect-resolution process ระหว่างทีมทดสอบกับทีมพัฒนายังไม่เกิด closed loop: ไม่มี commit ที่อ้างอิง defect, ไม่มี owner/target release และไม่มี acceptance test ที่ผูกกับแต่ละข้อ

ไม่ควรเริ่มรอบ 4 ด้วยการรันทดสอบชุดเดิมอีกทันที ขั้นตอนถัดไปที่แนะนำ:

1. ส่งรายงาน R1–R3 ให้ทีมพัฒนาและให้ acknowledge เป็นลายลักษณ์อักษร
2. กำหนด owner, priority, target version และ acceptance criteria ให้ R1-01 ถึง R1-10 และ R3-01 ถึง R3-04
3. ปิด High severity ตามลำดับ R1-01 → R1-02 → R1-03 พร้อม automated tests ใน commit เดียวกัน
4. ให้ PR/commit message อ้าง defect ID และแนบผล `test`, `fmt`, `clippy`, bundle budget และ KB parity gate
5. ส่งเข้ารอบทดสอบถัดไปเมื่อมี release candidate/commit SHA ชัดเจน ไม่ทดสอบ working tree ที่มี uncommitted changes โดยไม่มี baseline
6. เตรียม Windows disposable VM และ hardware matrix ก่อนขอ full acceptance

เงื่อนไขออกจาก Conditional Fail คือ High severity เดิมต้องปิดพร้อม regression tests, MSI ต้องผ่าน ICE validation ใน CI/VM โดยไม่ใช้ `-sval` และต้องมีผล Flash/Boot/ADB จากอุปกรณ์จริง

