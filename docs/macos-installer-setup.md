# macOS Installer — Setup Guide

## ภาพรวม

Workflow `build-macos.yml` จะ build แอปเป็น **Universal Binary** (.dmg)
ที่รองรับทั้ง **Apple Silicon (M1/M2/M3)** และ **Intel Mac** ในไฟล์เดียวกัน

## ขั้นตอนการทำงาน

```
Checkout repos
    ↓
Merge knowledge_base
    ↓
Build Tauri (universal-apple-darwin)
    ↓
[ถ้ามี cert] Code Sign ด้วย codesign
    ↓
[ถ้ามี Apple ID] Notarize กับ Apple
    ↓
Upload .dmg เป็น GitHub Release (draft)
```

---

## GitHub Secrets ที่ต้องตั้งค่า

ไปที่ **GitHub repo → Settings → Secrets and variables → Actions**

### ✅ ต้องมี (บังคับ)

| Secret | คำอธิบาย |
|--------|-----------|
| `GITHUB_TOKEN` | อัตโนมัติ ไม่ต้องตั้งเอง |

### 🔐 สำหรับ Code Signing (ถ้ามี Apple Developer Account)

| Secret | คำอธิบาย | วิธีหา |
|--------|-----------|--------|
| `APPLE_CERTIFICATE` | ใบรับรอง `.p12` แบบ Base64 | Export จาก Keychain Access → encode: `base64 -i cert.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | รหัสผ่านของไฟล์ `.p12` | รหัสที่ตั้งตอน export |
| `APPLE_SIGNING_IDENTITY` | ชื่อ identity เช่น `Developer ID Application: Your Name (TEAMID)` | รันคำสั่ง: `security find-identity -v -p codesigning` |
| `KEYCHAIN_PASSWORD` | รหัสผ่านสำหรับ keychain ชั่วคราว | ตั้งเองได้ เช่น `random_secure_password` |

### 🍎 สำหรับ Notarization (Apple ID)

| Secret | คำอธิบาย | วิธีหา |
|--------|-----------|--------|
| `APPLE_ID` | Apple ID email | เช่น `dev@example.com` |
| `APPLE_ID_PASSWORD` | App-specific password | สร้างที่ [appleid.apple.com](https://appleid.apple.com) → Sign-In and Security → App-Specific Passwords |
| `APPLE_TEAM_ID` | Team ID | [developer.apple.com](https://developer.apple.com/account) → Membership |

---

## ถ้ายังไม่มี Apple Developer Account

Workflow ยังทำงานได้ปกติ — แค่ข้าม step signing และ notarization
ผู้ใช้ Mac จะเห็น popup "unidentified developer" แต่ยังติดตั้งได้โดย:
> คลิกขวา → Open → Open

---

## ผลลัพธ์

| ไฟล์ | รายละเอียด |
|------|-----------|
| `VibeKidbright IDE_x.x.x_universal.dmg` | Universal Binary รองรับ Apple Silicon + Intel |

ไฟล์จะถูก upload เป็น GitHub Release (draft) พร้อมกับ Windows installer
