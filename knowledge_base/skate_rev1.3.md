# SKATE Rev 1.3 — Developer Reference
> ESP32-based Expansion Shield · NECTEC / KidBright Ecosystem
> Covers: **SKATE V1.3** — L298N Dual H-Bridge Motor Driver Integration

---

## 1. รู้จักกับ L298N Motor Driver

L298N เป็น IC ขับมอเตอร์แบบ **Dual H-Bridge** สามารถขับมอเตอร์ DC ได้พร้อมกัน 2 ตัว (หรือ Stepper Motor 1 ตัว) โดยรองรับกระแสสูงสุด 2A ต่อ Channel

### คุณสมบัติหลัก

| รายการ | ค่า |
|--------|-----|
| แรงดันขับมอเตอร์ (VCC) | 5V – 35V |
| กระแสสูงสุดต่อ Channel | 2A |
| Logic Voltage | 5V (TTL Compatible) |
| จำนวน Channel | 2 (มอเตอร์ A และ B) |

---

## 2. ขาใช้งาน (Pinout)

| ขา | หน้าที่ |
|----|---------|
| **VCC** | ไฟเลี้ยงมอเตอร์ (5–35V) |
| **GND** | กราวด์ร่วม |
| **5V** | ไฟออก 5V (ถ้าจั๊ม 5VEN ไว้ จะดึงจาก VCC มาให้อัตโนมัติ) |
| **ENA** | สัญญาณ PWM ควบคุมความเร็วมอเตอร์ A |
| **IN1** | ควบคุมทิศทางมอเตอร์ A (ขา 1) |
| **IN2** | ควบคุมทิศทางมอเตอร์ A (ขา 2) |
| **IN3** | ควบคุมทิศทางมอเตอร์ B (ขา 1) |
| **IN4** | ควบคุมทิศทางมอเตอร์ B (ขา 2) |
| **ENB** | สัญญาณ PWM ควบคุมความเร็วมอเตอร์ B |
| **OUT1/OUT2** | ขั้วไฟออกมอเตอร์ A |
| **OUT3/OUT4** | ขั้วไฟออกมอเตอร์ B |

---

## 3. ตารางควบคุมทิศทาง (Truth Table)

| ENA | IN1 | IN2 | ผลลัพธ์มอเตอร์ A |
|-----|-----|-----|-----------------|
| HIGH | HIGH | LOW | หมุนเดินหน้า |
| HIGH | LOW | HIGH | หมุนถอยหลัง |
| HIGH | HIGH | HIGH | เบรก (หยุดทันที) |
| HIGH | LOW | LOW | เบรก (หยุดทันที) |
| LOW | X | X | หยุด (Coast) |

> มอเตอร์ B ควบคุมด้วย ENB, IN3, IN4 ในลักษณะเดียวกัน

---

## 4. การเชื่อมต่อกับบอร์ด SKATE (ESP32)

บอร์ด SKATE V1.3 มีการต่อ L298N ผ่าน schematic ดังนี้ (อ้างอิงจาก `Skate_rev1_3.kicad_sch`):

| สัญญาณ SKATE | ขา ESP32 | ขา L298N |
|-------------|---------|---------|
| IN1 | GPIO | IN1 |
| IN2 | GPIO | IN2 |
| IN3 | GPIO | IN3 |
| IN4 | GPIO | IN4 |
| PWM (ENA) | GPIO (PWM) | ENA |
| +VM_L / +VM_R | VBAT | VCC |
| GND | GND | GND |

> **หมายเหตุ:** บอร์ด SKATE มี StepDown XL4005 เพื่อแปลง VBAT → 5V สำหรับเลี้ยงวงจร Logic

---

## 5. กฎสำคัญในการใช้งาน L298N

### ⚡ กฎด้านไฟฟ้า

1. **ต้องแยก GND ร่วม** — GND ของแบตเตอรี่, L298N และบอร์ดไมโครคอนโทรลเลอร์ต้องต่อกันเสมอ มิเช่นนั้น Logic จะทำงานผิดพลาด
2. **ห้ามต่อไฟมอเตอร์เกิน 35V** — ไม่เช่นนั้น IC จะเสียหายถาวร
3. **ถอด Jumper ENA/ENB ออกก่อนใช้ PWM** — ถ้าจั๊มไว้ ENA/ENB จะถูกล็อคที่ HIGH ตลอด ปรับความเร็วไม่ได้
4. **ระวังความร้อน** — L298N มีประสิทธิภาพต่ำ (~50%) ถ้าใช้มอเตอร์กินกระแสสูงควรติด Heat Sink
5. **แรงดัน Logic ต้องเป็น 3.3V–5V** — ESP32 ใช้ 3.3V ซึ่ง L298N รองรับได้ แต่ควรตรวจสอบระดับ Logic Threshold

### 🔌 กฎการต่อวงจร

6. **ไฟเลี้ยงมอเตอร์และ Logic ควรแยกวงจรกัน** — เพื่อป้องกัน Noise จากมอเตอร์รบกวน Microcontroller
7. **ใส่ Capacitor คู่ขนาน** ที่ขั้วมอเตอร์ (เช่น 100nF) เพื่อดูด Spike แรงดัน
8. **ต่อสายมอเตอร์ให้แน่น** ที่ Screw Terminal — หลวมทำให้เกิด Arc และ IC เสียหายได้

### 💻 กฎการเขียนโปรแกรม

9. **ห้ามให้ IN1 และ IN2 เป็น HIGH พร้อมกันนาน** — จะทำให้ H-Bridge Short และ IC ร้อนจัด
10. **ตั้งทิศทางก่อนเปิด ENA** — ควร Set IN1/IN2 ก่อน แล้วค่อยให้ PWM/ENA เพื่อความปลอดภัย
11. **ใช้ PWM ในการปรับความเร็ว** — ไม่ควรปรับแรงดัน VCC โดยตรง เพราะจะทำให้มอเตอร์เสียหายเร็ว
12. **ค่อยๆ เร่งความเร็ว (Ramp Up)** — อย่าสั่ง Full Speed ทันที จะช่วยยืดอายุมอเตอร์และ L298N

---

## 6. ตัวอย่างโค้ด (Arduino / ESP32)

### 6.1 ควบคุมทิศทางพื้นฐาน (ไม่มี PWM)

```cpp
// ============================================
// SKATE Board - L298N Basic Direction Control
// Motor A = Left Motor | Motor B = Right Motor
// ============================================

// กำหนดขาตามที่ต่อบน SKATE Board
#define IN1  25   // มอเตอร์ A - ซ้าย
#define IN2  26
#define IN3  27   // มอเตอร์ B - ขวา
#define IN4  14

void setup() {
  pinMode(IN1, OUTPUT);
  pinMode(IN2, OUTPUT);
  pinMode(IN3, OUTPUT);
  pinMode(IN4, OUTPUT);
  stopMotors();
}

void loop() {
  moveForward();   delay(2000);  // เดินหน้า 2 วินาที
  moveBackward();  delay(2000);  // ถอยหลัง 2 วินาที
  turnLeft();      delay(1000);  // เลี้ยวซ้าย 1 วินาที
  turnRight();     delay(1000);  // เลี้ยวขวา 1 วินาที
  stopMotors();    delay(1000);  // หยุด
}

// ฟังก์ชันเดินหน้า
void moveForward() {
  digitalWrite(IN1, HIGH);
  digitalWrite(IN2, LOW);
  digitalWrite(IN3, HIGH);
  digitalWrite(IN4, LOW);
}

// ฟังก์ชันถอยหลัง
void moveBackward() {
  digitalWrite(IN1, LOW);
  digitalWrite(IN2, HIGH);
  digitalWrite(IN3, LOW);
  digitalWrite(IN4, HIGH);
}

// ฟังก์ชันเลี้ยวซ้าย (มอเตอร์ขวาหมุน มอเตอร์ซ้ายหยุด)
void turnLeft() {
  digitalWrite(IN1, LOW);
  digitalWrite(IN2, LOW);
  digitalWrite(IN3, HIGH);
  digitalWrite(IN4, LOW);
}

// ฟังก์ชันเลี้ยวขวา (มอเตอร์ซ้ายหมุน มอเตอร์ขวาหยุด)
void turnRight() {
  digitalWrite(IN1, HIGH);
  digitalWrite(IN2, LOW);
  digitalWrite(IN3, LOW);
  digitalWrite(IN4, LOW);
}

// ฟังก์ชันหยุด
void stopMotors() {
  digitalWrite(IN1, LOW);
  digitalWrite(IN2, LOW);
  digitalWrite(IN3, LOW);
  digitalWrite(IN4, LOW);
}
```

---

### 6.2 ควบคุมความเร็วด้วย PWM (ESP32 `ledcWrite`)

```cpp
// ============================================
// SKATE Board - L298N PWM Speed Control
// ESP32 ใช้ LEDC แทน analogWrite
// ============================================

#define IN1  25
#define IN2  26
#define IN3  27
#define IN4  14
#define ENA  32   // PWM Channel มอเตอร์ A
#define ENB  33   // PWM Channel มอเตอร์ B

// LEDC Config
#define PWM_FREQ     5000   // ความถี่ 5kHz
#define PWM_RES      8      // ความละเอียด 8-bit (0-255)
#define CH_A         0      // LEDC Channel 0
#define CH_B         1      // LEDC Channel 1

void setup() {
  pinMode(IN1, OUTPUT);
  pinMode(IN2, OUTPUT);
  pinMode(IN3, OUTPUT);
  pinMode(IN4, OUTPUT);

  // ตั้งค่า LEDC PWM สำหรับ ESP32
  ledcSetup(CH_A, PWM_FREQ, PWM_RES);
  ledcSetup(CH_B, PWM_FREQ, PWM_RES);
  ledcAttachPin(ENA, CH_A);
  ledcAttachPin(ENB, CH_B);

  stopMotors();
}

void loop() {
  // เร่งความเร็วค่อยๆ (Ramp Up)
  for (int speed = 0; speed <= 255; speed += 5) {
    moveForwardSpeed(speed);
    delay(30);
  }
  delay(2000);

  // ลดความเร็วค่อยๆ (Ramp Down)
  for (int speed = 255; speed >= 0; speed -= 5) {
    moveForwardSpeed(speed);
    delay(30);
  }
  stopMotors();
  delay(1000);
}

// เดินหน้าพร้อมปรับความเร็ว (0-255)
void moveForwardSpeed(int speed) {
  digitalWrite(IN1, HIGH);
  digitalWrite(IN2, LOW);
  digitalWrite(IN3, HIGH);
  digitalWrite(IN4, LOW);
  ledcWrite(CH_A, speed);
  ledcWrite(CH_B, speed);
}

// ถอยหลังพร้อมปรับความเร็ว
void moveBackwardSpeed(int speed) {
  digitalWrite(IN1, LOW);
  digitalWrite(IN2, HIGH);
  digitalWrite(IN3, LOW);
  digitalWrite(IN4, HIGH);
  ledcWrite(CH_A, speed);
  ledcWrite(CH_B, speed);
}

// เลี้ยวพร้อมปรับความเร็วแต่ละล้อ (Differential Drive)
void turnSpeed(int leftSpeed, int rightSpeed) {
  // มอเตอร์ซ้าย
  if (leftSpeed >= 0) {
    digitalWrite(IN1, HIGH); digitalWrite(IN2, LOW);
  } else {
    digitalWrite(IN1, LOW);  digitalWrite(IN2, HIGH);
    leftSpeed = -leftSpeed;
  }
  // มอเตอร์ขวา
  if (rightSpeed >= 0) {
    digitalWrite(IN3, HIGH); digitalWrite(IN4, LOW);
  } else {
    digitalWrite(IN3, LOW);  digitalWrite(IN4, HIGH);
    rightSpeed = -rightSpeed;
  }
  ledcWrite(CH_A, constrain(leftSpeed,  0, 255));
  ledcWrite(CH_B, constrain(rightSpeed, 0, 255));
}

void stopMotors() {
  ledcWrite(CH_A, 0);
  ledcWrite(CH_B, 0);
  digitalWrite(IN1, LOW);
  digitalWrite(IN2, LOW);
  digitalWrite(IN3, LOW);
  digitalWrite(IN4, LOW);
}
```

---

## 7. ใช้ L298N กับ KidBright ได้ไหม?

### คำตอบ: ได้ แต่มีข้อจำกัด

บอร์ด **KidBright** (ESP32 based, NECTEC) สามารถต่อ L298N ผ่านขา I/O ได้ โดย:

| วิธีการ | รายละเอียด |
|---------|-----------|
| **ผ่าน KB CHAIN** | เชื่อมต่อผ่าน I2C ไปยัง SKATE Board ซึ่งมี L298N ต่ออยู่แล้ว (กรณีของ SKATE) |
| **ต่อตรง GPIO** | ใช้ขา Digital Output ของ KidBright ต่อกับ IN1–IN4 โดยตรง |
| **ใช้ KidBright IDE** | เขียน Block Code สั่ง Digital Output แทน โดยไม่ต้องใช้ภาษา C |

### ข้อควรระวังสำหรับ KidBright

- KidBright ใช้แรงดัน **3.3V** ซึ่ง L298N รับได้ แต่ควรตรวจสอบว่า Logic Level ของ L298N รุ่นที่ใช้รองรับ 3.3V หรือไม่
- KidBright IDE (Block-based) ไม่มีฟังก์ชัน PWM สำหรับ L298N โดยตรง → ต้องเขียนผ่าน MicroPython หรือ Arduino IDE แทน
- ถ้าใช้ SKATE Board คู่กับ KidBright ผ่าน KB CHAIN, SKATE ทำหน้าที่เป็น Expansion Shield และ ESP32 บน SKATE จะเป็นตัวควบคุม L298N โดยตรง

---

## 8. อ้างอิง

- Schematic: `Skate_rev1_3.kicad_sch` (KiCad EDA 9.0.6, Rev 1.2)
- [robotsiam.com - การใช้ Arduino UNO R3 กับ L298N](https://www.robotsiam.com/article/7/)
- [analogread.com - L298N Motor Driver](https://www.analogread.com/article/13/)
- Datasheet: L298N Dual Full-Bridge Driver (STMicroelectronics)
