# KidBright32 — Developer Reference
> ESP32-WROOM-32 · NECTEC / Gravitech · KidBright IDE / Arduino compatible · 3.3 V logic

---

## 1. Core MCU — ESP32-WROOM-32

| Parameter | Value |
|---|---|
| CPU | Dual-core Xtensa LX6, up to 240 MHz |
| Flash | 4 MB |
| RAM | 520 KB SRAM |
| Wi-Fi | 802.11 b/g/n 2.4 GHz |
| Bluetooth | Classic BT 4.2 + BLE |
| Logic voltage | **3.3 V** (GPIO are NOT 5 V tolerant) |
| ADC | 18 × 12-bit SAR ADC |
| DAC | 2 × 8-bit DAC |
| PWM | 16 channels (LEDC) |
| Touch | 10 × capacitive touch GPIO |

---

## 2. On-Board Peripherals

### LED Dot Matrix
| Property | Detail |
|---|---|
| Size | 16 × 8 (128 LEDs, red) |
| Driver IC | IS31FL3730 via I2C (addr `0x60` / `0x61`) |
| Arduino | `KidBright_LED` or Adafruit GFX with custom driver |

### Built-in Indicator LEDs
| LED | GPIO | Behavior | Notes |
|---|---|---|---|
| Wi-Fi LED | GPIO2 | Active HIGH — `digitalWrite(2, HIGH)` = ON | Also used as status indicator by KidBright firmware |
| Bluetooth LED | GPIO4 | Active HIGH — `digitalWrite(4, HIGH)` = ON | Also used as status indicator by KidBright firmware |
| Power LED | — | Always ON when board is powered | Not GPIO-controlled |

> ⚠️ GPIO2 and GPIO4 are shared with the Wi-Fi/BT indicator LEDs. Writing to them will light the LEDs; avoid using them for other purposes unless those LEDs are acceptable side-effects.

```cpp
// Blink Wi-Fi LED
pinMode(2, OUTPUT);
digitalWrite(2, HIGH);  // LED ON
delay(500);
digitalWrite(2, LOW);   // LED OFF
```

### Sensors
| Sensor | Interface | Detail |
|---|---|---|
| Temperature | I2C | TMP75, address `0x48`, 12-bit °C |
| Light (LDR) | ADC | GPIO34 / ADC1_CH6 — shared with I/O pad |

### Push Buttons
| Button | GPIO | Notes |
|---|---|---|
| SW1 | GPIO0 | Active LOW · doubles as BOOT button |
| SW2 | GPIO35 | Active LOW · input-only, needs external 10 kΩ pull-up |

### Buzzer
| Property | Detail |
|---|---|
| GPIO | GPIO25 |
| Type | Passive piezo — drive with PWM |

### RTC
| Property | Detail |
|---|---|
| IC | DS1307 or PCF8523 (check board revision) |
| Interface | I2C |
| Arduino | `RTClib` — `RTC.now()` returns `DateTime` |
| Backup | CR1220 coin cell socket |

---

## 3. GPIO & Connectors

### Large-Hole I/O Pads
| Label | GPIO | Capabilities |
|---|---|---|
| 5V | Power | 5 V from USB — ~500 mA shared |
| GND | Power | Ground |
| IO26 | GPIO26 | Digital I/O · DAC2 · ADC2_CH9 |
| IO27 | GPIO27 | Digital I/O · ADC2_CH7 · touch7 |
| IO32 | GPIO32 | Digital I/O · ADC1_CH4 · touch9 |
| IO33 | GPIO33 | Digital I/O · ADC1_CH5 · touch8 |
| IO34 | GPIO34 | **Input only** · ADC1_CH6 · shared with LDR |
| 3.3V | Power | 3.3 V regulated — ~300 mA max |

> ⚠️ **GPIO34** is input-only (no pull-up/down). Shared with the LDR — disconnect LDR path when using for external analog input.

### Servo Connectors
| Connector | GPIO | Notes |
|---|---|---|
| SERVO1 | GPIO16 | 50 Hz PWM, 1–2 ms pulse = 0–180° |
| SERVO2 | GPIO17 | Same spec as SERVO1 |

> Servo power comes from the dedicated **servo power terminal** (screw terminal), not the 3.3 V rail.

### I²C Header
| Pin | GPIO | Notes |
|---|---|---|
| SDA | GPIO21 | 4.7 kΩ pull-up on board |
| SCL | GPIO22 | 4.7 kΩ pull-up on board |
| 3.3V | Power | For external I2C devices |
| GND | Power | Ground |

> Default speed 100 kHz. Fast mode: `Wire.begin(21, 22, 400000)`

### USB Ports
| Port | Detail |
|---|---|
| USB Type-C | Programming & 5 V power (CP2102 USB-UART bridge) |
| USB Type-A | USB host for peripherals (HID, storage) |
| UART0 TX | GPIO1 — shared with USB bridge |
| UART0 RX | GPIO3 — shared with USB bridge |

---

## 4. Communication Buses

| Bus | Pins | Notes |
|---|---|---|
| I2C (Wire) | SDA=GPIO21 · SCL=GPIO22 | On-board: LED matrix, RTC, temp sensor + I2C header |
| SPI | MOSI=GPIO23 · MISO=GPIO19 · CLK=GPIO18 · CS=GPIO5 | KidBright Chain — up to 64 devices |
| UART0 | TX=GPIO1 · RX=GPIO3 | USB bridge / Serial monitor |
| KidBright DEV_I2C0 | Software I2C bus 0 | Internal device manager (FreeRTOS) |
| KidBright DEV_I2C1 | Software I2C bus 1 | Internal device manager (FreeRTOS) |
| KidBright DEV_SPI | Hardware SPI chain | Up to 64 chained modules |
| KidBright DEV_IO | I/O chain | General digital I/O expansion |

---

## 5. GPIO Assignment Map

| GPIO | Function | Notes |
|---|---|---|
| GPIO0 | SW1 / BOOT | Active LOW — LOW at reset = flash mode |
| GPIO2 | Wi-Fi LED | Active HIGH · shared with indicator LED |
| GPIO4 | Bluetooth LED | Active HIGH · shared with indicator LED |
| GPIO1 | UART0 TX | USB bridge — avoid as general I/O |
| GPIO3 | UART0 RX | USB bridge — avoid as general I/O |
| GPIO5 | SPI CS | KidBright Chain |
| GPIO16 | SERVO1 | PWM output |
| GPIO17 | SERVO2 | PWM output |
| GPIO18 | SPI CLK | KidBright Chain |
| GPIO19 | SPI MISO | KidBright Chain |
| GPIO21 | I2C SDA | On-board bus |
| GPIO22 | I2C SCL | On-board bus |
| GPIO23 | SPI MOSI | KidBright Chain |
| GPIO25 | Buzzer | PWM audio |
| GPIO26 | I/O Pad | DAC2 · ADC2_CH9 · digital I/O |
| GPIO27 | I/O Pad | ADC2_CH7 · touch7 · digital I/O |
| GPIO32 | I/O Pad | ADC1_CH4 · touch9 · digital I/O |
| GPIO33 | I/O Pad | ADC1_CH5 · touch8 · digital I/O |
| GPIO34 | I/O Pad + LDR | **Input only** · ADC1_CH6 |
| GPIO35 | SW2 | **Input only** · Active LOW |

---

## 6. Arduino Quick-Start Snippets

### Buttons
```cpp
pinMode(0, INPUT_PULLUP);   // SW1
pinMode(35, INPUT);          // SW2 — input only, add external pull-up

if (digitalRead(0) == LOW)  { /* SW1 pressed */ }
if (digitalRead(35) == LOW) { /* SW2 pressed */ }
```

### Buzzer
```cpp
ledcSetup(0, 1000, 8);   // channel 0, 1 kHz, 8-bit resolution
ledcAttachPin(25, 0);
ledcWrite(0, 128);        // 50% duty — beep ON
delay(500);
ledcWrite(0, 0);          // beep OFF
```

### Light Sensor (ADC)
```cpp
int raw = analogRead(34);          // 0–4095
float voltage = raw * 3.3 / 4095.0;
```

### Servo
```cpp
#include <ESP32Servo.h>
Servo s1;
s1.attach(16);    // SERVO1
s1.write(90);     // 0–180 degrees
```

### Temperature Sensor (TMP75)
```cpp
Wire.begin(21, 22);
Wire.beginTransmission(0x48);
Wire.write(0x00);             // temperature register
Wire.endTransmission();
Wire.requestFrom(0x48, 2);
int raw = (Wire.read() << 4) | (Wire.read() >> 4);
float tempC = raw * 0.0625;
```

### Wi-Fi
```cpp
#include <WiFi.h>
WiFi.begin("SSID", "PASSWORD");
while (WiFi.status() != WL_CONNECTED) delay(500);
Serial.println(WiFi.localIP());
```

---

## 7. Key Gotchas

- **3.3 V only** — never connect 5 V signals to GPIO directly.
- **GPIO34 & GPIO35** are input-only — no internal pull-up/down hardware.
- **GPIO0 LOW at boot** = flash mode — don't drive it LOW externally at startup.
- **GPIO1 & GPIO3** are shared with USB-UART — using them as I/O breaks Serial.
- **ADC2 (GPIO26, GPIO27) cannot be used while Wi-Fi is active** — use ADC1 pins (GPIO32, GPIO33, GPIO34) instead.
- **Servo power** must come from the dedicated servo terminal, not the board's 3.3 V rail.
- **LDR shares GPIO34** — disconnect or account for LDR when reading external analog signals.
- **I2C pull-ups are already on board** — don't add more to GPIO21/22.
- **Arduino board target**: ESP32 Dev Module · Flash: 4 MB · Upload speed: 921600.