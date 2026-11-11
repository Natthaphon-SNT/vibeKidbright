# ติดตั้ง vibeKidbright บน macOS

ตัวติดตั้งเป็น **Universal DMG** ไฟล์เดียวสำหรับ **Apple Silicon และ Intel**
ผู้ใช้แอปไม่ต้องติดตั้ง Node.js, Rust หรือ Python เพื่อเปิด IDE, แก้โค้ด,
อ่านฐานความรู้ หรือค้นหาด้วยโมเดลออฟไลน์ที่รวมมาในแอป

## ดาวน์โหลดและติดตั้ง

1. เปิด [Build macOS Installer](https://github.com/Natthaphon-SNT/vibeKidbright/actions/workflows/build-macos.yml)
   แล้วเลือก run ของเวอร์ชัน/commit ที่ต้องการ ถ้ายังไม่มี run ของซอร์สล่าสุด
   ผู้ดูแลต้องนำการแก้ไขขึ้น repository และกด **Run workflow** หรือ push เข้า `main` ก่อน
2. รอให้ job **Install and launch (aarch64)** และ **Install and launch (x86_64)** ผ่าน
   แล้วดาวน์โหลด artifact `vibeKidbright-macOS-<version>-universal` จากท้ายหน้า run
   GitHub อาจให้ลงชื่อเข้าใช้เพื่อดาวน์โหลด artifact; artifact เก็บไว้ 30 วัน
3. แตก ZIP จะได้ `VibeKidbright-IDE_<version>_universal.dmg` และ `SHA256SUMS-macos.txt`
   เปิด Terminal ในโฟลเดอร์นั้นเพื่อตรวจไฟล์:

   ```sh
   shasum -a 256 -c SHA256SUMS-macos.txt
   ```

4. เปิด DMG แล้วลาก **VibeKidbright IDE.app** ไปไว้ใน **Applications**
5. Eject ดิสก์ติดตั้ง แล้วเปิดแอปจาก Applications
6. ตั้งค่า AI provider และ API key ของตัวเองเมื่อต้องการใช้ผู้ช่วย AI

Workflow แนบ DMG, checksum และผลตรวจทั้งสองสถาปัตยกรรมเข้ากับ **Draft Release**
หลังทดสอบผ่านทั้งคู่ ผู้ดูแลต้องตรวจแล้ว Publish release เพื่อให้ผู้ใช้ทั่วไปดาวน์โหลด
จาก [Releases](https://github.com/Natthaphon-SNT/vibeKidbright/releases) ได้
การมี artifact จาก build job อย่างเดียวยังไม่แปลว่า installation jobs ผ่านแล้ว

### ถ้า macOS แจ้งว่าไม่สามารถตรวจสอบผู้พัฒนา

เมื่อยังไม่มี Apple Developer certificate แอปจะลงลายเซ็นแบบ **ad-hoc**
ซึ่งใช้กับ Apple Silicon ได้ แต่ยังไม่ได้รับการ notarize จาก Apple
หลังตรวจที่มาและ checksum แล้ว ให้ลองเปิดแอป จากนั้นไปที่
**System Settings → Privacy & Security → Open Anyway** และยืนยันเปิด
ตาม [คู่มือของ Apple](https://support.apple.com/en-us/102445)
เครื่องของโรงเรียน/องค์กรอาจมีนโยบายที่ต้องให้ผู้ดูแลเครื่องอนุญาต

รายละเอียดการลงลายเซ็นและข้อจำกัด ad-hoc อยู่ใน
[เอกสาร Tauri](https://v2.tauri.app/distribute/sign/macos/)

## ตั้งค่า Build & Flash บน Mac

สำหรับกรณีติดตั้งแอปแล้วขึ้น Ready แต่กด Build เจอ `No ESP-IDF Python venv found`
ดู [รายละเอียดการแก้และขอบเขตทดสอบ](MACOS_SDK_BUILD_REPAIR_2026-11-11.md)
แอปรุ่นที่รวมการแก้ไขนี้ตรวจ Python และเครื่องมือจริงก่อนขึ้น Ready

การเปิด IDE ไม่ต้องมี ESP-IDF แต่ Build & Flash สำหรับ KidBright32 ต้องมี SDK
และ compiler ที่ตรงกับ Mac เครื่องนั้น **อย่าใช้ `tools.zip` ของ Windows บน Mac**
manifest ปัจจุบันยังไม่มีแพ็กเกจ macOS ที่ verified; ปุ่ม Auto Install จึงยังติดตั้ง
 toolchain ให้ Mac ไม่ได้ การเพิ่ม DMG ไม่ได้ปลดการตรวจ integrity นี้

1. เปิด **Setup / Repair ESP-IDF** หรือขยาย widget แจ้งเตือน toolchain
   บน Mac จะมี **Copy macOS Setup** คัดลอกคำสั่งไปวางใน Terminal บน Mac
   คำสั่งเลือก Python native ตาม CPU และติดตั้ง ESP-IDF **v5.5.1**, compiler,
   CMake, Ninja และ Python environment ที่ `$HOME/.vibekidbright/.espressif`
   แยกจาก toolchain เก่า หากยังไม่มี Git/Python ให้ติดตั้ง Xcode Command Line Tools
   ตามคำแนะนำในหน้าจอก่อน ดูขั้นตอนต้นทางของ
   [ขั้นตอน macOS ของ Espressif](https://docs.espressif.com/projects/esp-idf/en/v5.5.1/esp32/get-started/linux-macos-setup.html)
2. รอคำสั่ง setup สำเร็จแล้วกด **Check Native SDK** แอปจะตรวจ native SDK และบันทึก
   เส้นทางที่ใช้งานได้ ไม่เก็บ path ใหม่หาก Python หรือเครื่องมือยังรันไม่ได้
3. หากมี ESP-IDF ติดตั้งเองอยู่แล้ว ใช้ **Setup / Repair ESP-IDF → Manual Path**
   เลือก **ESP-IDF Framework Path** เช่น `$HOME/esp/esp-idf`
   ซึ่งต้องมี `tools/idf.py`
   และ **ESP-IDF Tools Path** เช่น `$HOME/.espressif`
   ซึ่งต้องมี `python_env` และ `tools` แล้วกด **Save & Apply**
4. เก็บ SDK/tools และโปรเจกต์ใน path ที่ไม่มีช่องว่าง อย่าใช้ SDK ใน
   `Library/Application Support` หรือย้าย Windows venv มาใช้บน Mac
5. เสียบบอร์ดด้วยสาย USB ที่ส่งข้อมูลได้ เลือกพอร์ต `/dev/cu.*` ของบอร์ด
   แล้วทดลอง Build & Flash ถ้าไม่พบพอร์ตให้ตรวจ USB driver ของชิปบนบอร์ดนั้น

แอปที่เปิดจาก Finder ไม่จำเป็นต้องได้รับ `PATH` จาก Terminal:
เส้นทาง SDK ที่บันทึกผ่าน Manual Path ใช้หลังปิดและเปิดแอปใหม่ได้
ผล installation check ไม่ได้รับรองการ flash หรือ peripheral บนบอร์ดจริง

## Build บนเครื่อง Mac

สำหรับผู้พัฒนา ต้องมี Xcode Command Line Tools, Git, Rust stable,
Node.js ตาม `package.json` และ npm 10 ขึ้นไป:

```sh
xcode-select --install
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm ci
npm run build:macos
```

คำสั่งนี้เตรียมโมเดลออฟไลน์ ตรวจ resource และ build Universal DMG โดยใช้ Cargo lockfile
ไฟล์อยู่ที่ `src-tauri/target/universal-apple-darwin/release/bundle/dmg/`
การ build และลงลายเซ็น macOS ต้องรันบน Mac; จาก Windows/Linux ใช้ workflow ด้านบน

ก่อนใช้ installation probe กับ local build ให้สร้าง checksum ในโฟลเดอร์ DMG:

```sh
cd src-tauri/target/universal-apple-darwin/release/bundle/dmg
shasum -a 256 *.dmg > SHA256SUMS-macos.txt
```

กลับมาที่ root repository แล้วรัน (probe ใช้ Python **3.11+**):

```sh
npm run test:macos-installer -- \
  --installer-directory src-tauri/target/universal-apple-darwin/release/bundle/dmg \
  --report src-tauri/target/macos-installation-check.json \
  --expected-version 3.7.4 \
  --expected-arch aarch64
```

บน Intel ใช้ `--expected-arch x86_64` ต้องทดสอบทั้งสองเครื่อง/runner
เพื่อยืนยัน Universal DMG ทั้งสอง slice โดยไม่อาศัย Rosetta

## สิ่งที่ workflow ตรวจ

- รัน frontend tests, Rust tests ทุก target ของแพ็กเกจ และ Clippy บน build host
- สร้าง Universal DMG พร้อม ad-hoc signature หรือ Developer ID ตาม secrets
- ถ้ามี credentials ครบ จะ notarize และ staple DMG ก่อนสร้าง checksum
- เก็บ DMG เป็น Actions artifact แยกจาก Draft Release
- นำ **DMG ไฟล์เดียวกัน** ไปตรวจบน `macos-15` (Apple Silicon)
  และ `macos-15-intel` ([runner labels ของ GitHub](https://docs.github.com/en/actions/reference/runners/github-hosted-runners))
- ตรวจ SHA-256, disk image, Universal slices, signature และ dependency จากเครื่อง build
- Mount แบบอ่านอย่างเดียว คัดลอก `.app` ไป path ที่มีช่องว่าง แล้ว unmount ก่อนรัน
- เรียก `--self-check` ด้วย data directory ใหม่และ working directory แยก
  โดยลบ developer paths และปิด HTTP proxies: ตรวจฐานความรู้, ONNX inference จริง,
  SQLite search, settings, Keychain และการกู้คืน/Undo
- เปิดโปรแกรมที่คัดลอกไว้และตรวจว่ายังทำงานหลัง 12 วินาที
- ติดตั้ง native SDK ด้วยคำสั่งชุดเดียวกับในแอป แล้วให้ executable ที่คัดลอกจาก DMG
  รัน `--sdk-self-check` สร้างโปรเจกต์ทดสอบใหม่และ build firmware ESP32 จริง
  โดยใช้ตัวสร้าง command/environment เดียวกับปุ่ม Build; ไม่ flash hardware
- แนบรายงาน JSON และ startup log; สร้าง Draft Release หลังตรวจผ่านทั้งสองเครื่อง

การตรวจนี้ยังไม่ใช่ interactive GUI test, การทดสอบ Gatekeeper จาก browser download,
การทดสอบบน macOS รุ่นเก่าทุกรุ่น หรือการทดสอบ USB/บอร์ดจริง
การตั้ง HTTP proxy ไม่ใช่การตัดเครือข่ายด้วย firewall
รายงานจะแยกขอบเขตเหล่านี้ไว้ ไม่อ้างว่าเป็น clean user VM ที่ติดตั้งด้วยมือ

## Apple Developer secrets สำหรับผู้ดูแล

ไปที่ **Repository → Settings → Secrets and variables → Actions**
`GITHUB_TOKEN` สำหรับ workflow มีให้โดย GitHub อัตโนมัติ

| Secret | ใช้สำหรับ |
|---|---|
| `APPLE_CERTIFICATE` | Developer ID Application certificate `.p12` แบบ Base64 (`base64 -i cert.p12`) |
| `APPLE_CERTIFICATE_PASSWORD` | รหัสผ่านไฟล์ `.p12` |
| `APPLE_SIGNING_IDENTITY` | ชื่อจาก `security find-identity -v -p codesigning` |
| `KEYCHAIN_PASSWORD` | รหัสผ่าน temporary build keychain |
| `APPLE_ID` | Apple ID สำหรับ notarization |
| `APPLE_ID_PASSWORD` | App-specific password; workflow ส่งให้ Tauri เป็น `APPLE_PASSWORD` |
| `APPLE_TEAM_ID` | Apple Developer Team ID |

เมื่อไม่มี certificate ใช้ ad-hoc; เมื่อมี certificate ต้องมี signing identity
และควรมี notarization credentials ครบเพื่อแจกจ่ายผ่าน Gatekeeper
workflow ลบ temporary signing keychain หลัง build เสมอ

## สถานะหลักฐาน ณ 11 พฤศจิกายน 2026

รอบก่อนหน้า [39544978384](https://github.com/Natthaphon-SNT/vibeKidbright/actions/runs/39544978384)
ที่ commit `7d0157a` build Universal DMG สำเร็จเมื่อ 21 ตุลาคม 2026
แต่เก็บเฉพาะ Draft Release และไม่ได้รัน installation probe ของงานนี้
Public latest release `v3.7.1` ที่ตรวจในรอบนี้มีเฉพาะตัวติดตั้ง Windows

การแก้ไขรอบนี้ทำบน Windows: portable probe tests และการตรวจซอร์สไม่ใช่หลักฐาน
ว่า DMG ของ working tree ล่าสุด build/เปิดบน Mac ผ่านแล้ว ต้องรัน workflow ที่รวม
การแก้ไขนี้และตรวจรายงานทั้งสองสถาปัตยกรรมก่อนแจกจ่าย
