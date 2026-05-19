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

บอร์ด SKATE V1.3 — ขา GPIO จริงที่ใช้ในโค้ด Self-Balancing (อ้างอิงจาก `Skate_rev1_3.kicad_sch` และโค้ดจริง):

| สัญญาณ | ขา ESP32 | หน้าที่ |
|--------|---------|--------|
| **LT** (Left Top) | GPIO 18 | มอเตอร์ซ้าย — เดินหน้า |
| **LB** (Left Bottom) | GPIO 19 | มอเตอร์ซ้าย — ถอยหลัง |
| **RT** (Right Top) | GPIO 26 | มอเตอร์ขวา — เดินหน้า |
| **RB** (Right Bottom) | GPIO 27 | มอเตอร์ขวา — ถอยหลัง |
| **ENCA** (Encoder A) | GPIO 32 | Interrupt สำหรับนับ Pulse |
| **ENCB** (Encoder B) | GPIO 33 | ตรวจทิศทางหมุน |
| **SDA (MPU6050)** | GPIO 4 | I2C Data |
| **SCL (MPU6050)** | GPIO 5 | I2C Clock |
| **+VM_L / +VM_R** | VBAT | ไฟเลี้ยงมอเตอร์จากแบตเตอรี่ |
| **GND** | GND | กราวด์ร่วม |

> **หมายเหตุ:** โค้ดนี้ใช้ `analogWrite()` โดยตรง (ไม่ใช้ `ledcWrite`) ซึ่งทำงานได้บน Arduino ESP32 Core รุ่นเก่า — ถ้าใช้ Core ≥ 3.x ให้เปลี่ยนเป็น `ledcWrite` แทน

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

### 🤖 กฎเฉพาะสำหรับ Self-Balancing Robot (จากโค้ดจริง)

13. **ต้องหยุดมอเตอร์เมื่อหุ่นล้ม** — ใช้เงื่อนไข `if (Angle > -25 && Angle < 25)` เสมอ ป้องกันมอเตอร์หมุนฟรีขณะหุ่นล้มซึ่งทำให้ L298N ร้อนเกิน
14. **Clamp PWM ก่อนส่งให้มอเตอร์เสมอ** — ต้อง limit ค่า PWM ให้อยู่ในช่วง `-255` ถึง `255` ก่อนทุกครั้ง เพราะ PID + Kc×rpm อาจให้ค่าเกินได้
15. **ตั้งทิศทาง (digitalWrite LOW) ด้านตรงข้ามก่อน ค่อย analogWrite** — ในโค้ด Self-Balancing ต้อง `LOW` ขาตรงข้ามก่อนเสมอ ก่อนจะ `analogWrite` ขาที่ต้องการ มิเช่นนั้น H-Bridge จะ short ชั่วขณะ
16. **ระวัง GPIO 18, 19 ของ ESP32** — GPIO 18/19 เป็นขา VSPI (SPI) ถ้าใช้ SPI peripherals อื่นร่วมด้วยต้องระวัง conflict
17. **Encoder ISR ต้องใช้ `IRAM_ATTR` เสมอ** — ฟังก์ชัน `readEncoder()` ต้องมี attribute นี้ ป้องกัน crash เมื่อ ISR ถูกเรียกขณะ Flash กำลัง busy
18. **PID Angle ต้องมี Sample Time สั้นกว่า PID Position** — `pidAngle.SetSampleTime(5ms)` เร็วกว่า `pidpo.SetSampleTime(30ms)` เสมอ เพราะการทรงตัวต้องตอบสนองเร็วกว่าการควบคุมตำแหน่ง
19. **Setpoint มอเตอร์ต้องชดเชย Mechanical Offset** — ค่า `Setpoint = -5.68 - Delta_ang` ตัวเลข `-5.68` คือ offset เชิงกลที่วัดได้จากหุ่นจริง ต้องปรับค่านี้ใหม่ทุกครั้งที่ประกอบหุ่นใหม่หรือเปลี่ยนน้ำหนัก
20. **`Kc` (Back-EMF Compensation) ต้องปรับตาม RPM จริง** — ค่า `Kc = 0.7` ที่ใช้อยู่คือค่าเริ่มต้น ถ้ามอเตอร์ใหม่หรือแรงดันแบตต่างกัน ค่านี้ต้องปรับใหม่ มิเช่นนั้นระบบจะ oscillate

### 🌐 กฎสำหรับ WiFi / ESP-NOW / MQTT

21. **WiFi และ ESP-NOW ใช้ร่วมกันได้ แต่ต้อง `WIFI_STA` mode** — ต้องตั้ง `WiFi.mode(WIFI_STA)` ก่อน `esp_now_init()` เสมอ ถ้าสลับลำดับ ESP-NOW จะ init ล้มเหลว
22. **ห้าม `esp_now_init()` ซ้ำสองครั้ง** — ในโค้ดปัจจุบันมีการเรียก `esp_now_init()` ซ้ำ 2 ครั้งใน `setup()` ซึ่งจะ return error ครั้งที่ 2 ควรเรียกเพียงครั้งเดียว
23. **ห้าม `delay()` ใน loop หลัก** — การใช้ `delay()` จะทำให้ PID loop ขาดความต่อเนื่อง และหุ่นจะล้ม ใช้ `micros()` / `millis()` แทนเสมอ
24. **MQTT `client.loop()` ต้องถูกเรียกทุก iteration ของ loop** — ถ้าไม่เรียกจะทำให้การเชื่อมต่อ broker หลุดโดยไม่รู้ตัว
25. **ควร reconnect MQTT แบบ Non-blocking** — อย่าใส่ `while` loop ใน reconnect function เพราะจะ block PID loop และหุ่นจะล้ม

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

### 6.3 Self-Balancing Robot — โค้ดจริงบน SKATE Board

```cpp
// ============================================================
// SKATE Board - Self-Balancing Robot (Dual PID + ESP-NOW + MQTT)
// Angle PID  : Kp=16, Ki=0,    Kd=0.5   | SampleTime 5ms
// Position PID: Kp=2.12, Ki=0, Kd=1.4  | SampleTime 30ms
// Motor: LT=18, LB=19 (Left) | RT=26, RB=27 (Right)
// Encoder: ENCA=32 (Interrupt), ENCB=33 | PPR=330
// MPU6050: SDA=4, SCL=5
// ============================================================

// การขับมอเตอร์ในโค้ดนี้ใช้ขา "Top/Bottom" แทน IN1–IN4:
//   LT (HIGH) + LB (LOW)  → มอเตอร์ซ้ายเดินหน้า
//   LT (LOW)  + LB (HIGH) → มอเตอร์ซ้ายถอยหลัง
// เช่นเดียวกันสำหรับ RT/RB ฝั่งขวา

// ตัวอย่างการขับมอเตอร์จาก loop():
if (PWM > 0) {
  // เดินหน้า: ปิดขาถอยก่อน แล้วค่อย PWM ขาเดินหน้า
  digitalWrite(LB, LOW);
  digitalWrite(RB, LOW);
  analogWrite(LT, pwmVal - myData.x);   // ชดเชยด้านซ้ายด้วย offset joystick
  analogWrite(RT, pwmVal + myData.x);   // ชดเชยด้านขวา
} else {
  // ถอยหลัง: ปิดขาเดินหน้าก่อน แล้วค่อย PWM ขาถอยหลัง
  digitalWrite(LT, LOW);
  digitalWrite(RT, LOW);
  analogWrite(LB, pwmVal + myData.x);
  analogWrite(RB, pwmVal - myData.x);
}
```

**อธิบาย Logic สำคัญในโค้ด:**

| ส่วน | รายละเอียด |
|------|-----------|
| **Complementary Filter** | `angle = 0.98*(angle + Gy*dt) + 0.02*pitch_acc` — ผสม Gyro (98%) กับ Accelerometer (2%) เพื่อลด Drift |
| **Dual PID** | PID ชั้นนอก (Position) ส่ง `Delta_ang` ไปเป็น Setpoint ให้ PID ชั้นใน (Angle) |
| **Kc × RPM** | `PWM = PWM + (Kc * rpm)` — ชดเชย Back-EMF ของมอเตอร์ ช่วยให้หุ่นตอบสนองเร็วขึ้น |
| **EN_lock mode** | `EN_lock=1` → ล็อคตำแหน่ง (Position Hold), `EN_lock=0` → รับคำสั่งเดินจาก Joystick |
| **Safety cutoff** | `if (Angle > -25 && Angle < 25)` → ถ้าหุ่นเอียงเกิน 25° จะหยุดมอเตอร์ทันที |

---

## 7. ใช้ L298N กับ KidBright ได้ไหม?

### คำตอบ: ได้ และทำได้ครบทั้ง Self-Balancing Robot

บอร์ด **KidBright v1.7.0** (ESP32 based, NECTEC) มี Block พร้อมใช้งานครบสำหรับ Self-Balancing Robot ผ่าน SKATE Board โดย:

| วิธีการ | รายละเอียด |
|---------|-----------|
| **PWM BDC Motor Drive** | Block สั่งมอเตอร์ DC พร้อม PWM และทิศทาง (Clockwise / Counterclockwise) |
| **PID Controller** | Block PID พร้อมปรับ Kp, Ki, Kd ได้ใน IDE |
| **Quadrature Encoder** | Block อ่าน Encoder 2 Phase (Phase A I/O, Phase B I/O) และ Pulses per Round |
| **MPU6050** | Block อ่านมุม Angle และ calibrate ได้โดยตรง |
| **ESP-NOW** | Block รับ-ส่งข้อมูลไร้สายระหว่างบอร์ด |
| **I2C OLED 128x64** | Block แสดงผลค่า Debug บนจอ OLED |

### ข้อควรระวังสำหรับ KidBright Block

- KidBright ใช้แรงดัน **3.3V** ซึ่ง L298N รองรับ แต่ควรตรวจสอบ Logic Threshold ของโมดูล L298N ที่ใช้
- Block `PWM BDC Motor Drive` ใช้ขา IN/PWM ของ SKATE Board ได้โดยตรงผ่าน KB CHAIN
- **MS Delay ที่ใช้ใน Main Loop ต้องสั้นมาก** — ในโค้ดจริงใช้ `MS Delay 0.02` (20µs) เพื่อให้ PID loop เร็วพอ

---

## 8. ตัวอย่าง KidBright Block — Self-Balancing Robot (v1.7.0)

อ้างอิงจากโค้ด Block จริงบน KidBright IDE

### Task หลัก — Balancing Loop

```
Task
  set Kc       to 0.7
  set Setpoint to 0
  Wait Switch 1 pressed          ← รอกดปุ่มก่อน Start
  Note C7 Duration ♩             ← เสียงแจ้งเตือนพร้อม
  I2C OLED 128x64 SH1106 Ch0 Address 0x3C  Print(1, 2) → OK!
  MPU6050 calibrate Channel 0
  set setpoint_pos to [Quadrature Encoder Read Position  PhaseA=IN2  PhaseB=IN1  PPR=330]
  set drv_pos   to 0
  set LockPos_EN to 1
  set Mot_offset to 0

  Forever
    set Angle to [MPU6050 angle measurement  Channel 0  Axis Y]

    if  Angle >= -45  and  Angle <= 45
    do
      set PWM to [PID Controller #0  Kp=8.8  Ki=0.001  Kd=0.4
                  SetPoint=(Setpoint - drv_pos)  Input=Angle]

      set PWM to  PWM + (Kc × [Quadrature Encoder Read Speed
                                PhaseA=IN2  PhaseB=IN1  PPR=330  Filtered=Yes])

      if PWM >= 0
      do
        PWM BDC Motor Drive 1  Direction=Clockwise      Speed(%) = PWM - Mot_offset
        PWM BDC Motor Drive 2  Direction=Clockwise      Speed(%) = PWM + Mot_offset
      else
        PWM BDC Motor Drive 1  Direction=Counterclockwise  Speed(%) = PWM - Mot_offset
        PWM BDC Motor Drive 2  Direction=Counterclockwise  Speed(%) = PWM + Mot_offset

    else
      PWM BDC Motor Stop 1
      PWM BDC Motor Stop 2

    MS Delay 0.02
```

**ตัวแปรสำคัญ Task หลัก:**

| ตัวแปร | ค่าเริ่มต้น | ความหมาย |
|--------|-----------|---------|
| `Kc` | 0.7 | Back-EMF compensation gain |
| `Setpoint` | 0 | มุมที่ต้องการ (องศา) |
| `Kp / Ki / Kd` | 8.8 / 0.001 / 0.4 | PID gain สำหรับ Angle |
| `Mot_offset` | 0 | ค่าชดเชยความต่างของมอเตอร์ซ้าย-ขวา |
| `LockPos_EN` | 1 | เปิด/ปิด Position Lock mode |
| Safety range | ±45° | ถ้าเอียงเกิน → หยุดมอเตอร์ทันที |

---

### Task ESP-NOW — รับคำสั่งจาก Controller

```
Task
  set mode to 0
  Forever
    ESP-NOW on receiving
      set Data to [ESP-NOW read number]

      if  mode == 0
      do
        if  Data >= 0  and  Data <= 180
        do
          if  Data < 90
          do  set Mot_offset to -15
          else if  Data > 90
          do  set Mot_offset to 15
          else
            set Mot_offset  to 0
            set LockPos_EN  to 1        ← กลับมา Lock Position

        else if  Data >= 900  and  Data <= 1100
        do
          set Data      to  Data - 1000   ← แปลงเป็น offset ±100
          set Setpoint  to  Data
          if  Setpoint == 0
          do  set LockPos_EN to 1
          else  set LockPos_EN to 0

        else
          PWM BDC Motor Stop 1
          PWM BDC Motor Stop 2

    Delay 0.3
```

**Protocol ESP-NOW ที่ใช้:**

| ช่วงค่า Data | ความหมาย |
|------------|---------|
| `0–180` | Joystick X axis → ควบคุม `Mot_offset` (เลี้ยวซ้าย/ขวา) |
| `< 90` | เลี้ยวซ้าย → `Mot_offset = -15` |
| `> 90` | เลี้ยวขวา → `Mot_offset = +15` |
| `= 90` | กลาง → `Mot_offset = 0`, Lock Position |
| `900–1100` | Joystick Y axis → offset = `Data - 1000` → ควบคุม `Setpoint` (เดิน/หยุด) |
| `Setpoint = 0` | หยุด → `LockPos_EN = 1` |
| `Setpoint ≠ 0` | เดิน → `LockPos_EN = 0` |
| นอกช่วง | หยุดมอเตอร์ทันที (Safety) |

---

### Task Position Lock — ล็อคตำแหน่ง

```
Task
  Forever
    if  LockPos_EN == 1
    do
      set pos      to [Quadrature Encoder Read Position  PhaseA=IN2  PhaseB=IN1  PPR=330]
      set err_pos  to  setpoint_pos - pos

      if  err_pos >= 20  or  err_pos <= -20
      do
        set drv_pos to [PID Controller #1  Kp=0.008  Ki=0.00001  Kd=0.0001
                        SetPoint=setpoint_pos  Input=pos]
      else
        set drv_pos      to 0
        set setpoint_pos to [Quadrature Encoder Read Position  PhaseA=IN2  PhaseB=IN1  PPR=330]

    MS Delay 0.05
```

**ตัวแปรสำคัญ Position Lock:**

| ตัวแปร | ค่า | ความหมาย |
|--------|-----|---------|
| `Kp / Ki / Kd` | 0.008 / 0.00001 / 0.0001 | PID gain สำหรับ Position (ค่าน้อยมาก ป้องกัน overshoot) |
| Dead zone | ±20 pulses | ถ้า error น้อยกว่านี้ไม่สั่ง PID (ประหยัด CPU) |
| `MS Delay` | 0.05ms | รอบ Loop ของ Position task ช้ากว่า Balance task |

---

### กฎเพิ่มเติมสำหรับ KidBright Block

26. **Encoder ขา Phase A/B ต้องสลับกัน** — ในโค้ด KidBright ใช้ `PhaseA=IN2, PhaseB=IN1` ซึ่งสลับจากการต่อปกติ ต้องตรวจสอบให้ตรงกับ Hardware จริง มิเช่นนั้นค่า Position จะนับถอยหลัง
27. **MS Delay ใน Loop ต้องไม่เกิน 1ms** — ถ้าใส่ Delay นานเกินใน Balancing Loop หุ่นจะล้มก่อนที่ PID จะทำงาน
28. **Position PID ต้องมี Dead Zone** — ถ้าไม่มี `if err_pos >= 20` หุ่นจะสั่นอยู่ที่จุดเดิมตลอดเวลา เพราะ Encoder มี noise
29. **Calibrate MPU6050 ขณะหุ่นอยู่นิ่งสมดุลเท่านั้น** — Block `MPU6050 calibrate` จะเซ็ต zero point ณ เวลาที่เรียก ถ้าหุ่นเอียงอยู่จะ calibrate ผิด
30. **รอกด Switch ก่อน Start เสมอ** — Block `Wait Switch 1 pressed` ช่วยให้ผู้ใช้วางหุ่นให้สมดุลก่อนที่ระบบจะเริ่ม Calibrate และ Lock Position

---

## 9. อ้างอิง

- Schematic: `Skate_rev1_3.kicad_sch` (KiCad EDA 9.0.6, Rev 1.2)
- KidBright IDE ver. 1.7.0 (NECTEC)
- [robotsiam.com - การใช้ Arduino UNO R3 กับ L298N](https://www.robotsiam.com/article/7/)
- [analogread.com - L298N Motor Driver](https://www.analogread.com/article/13/)
- Datasheet: L298N Dual Full-Bridge Driver (STMicroelectronics)
