# ตรวจระดับฟังก์ชัน KidBright — ปรับข้อมูล 17 ตุลาคม 2026

**ยังไม่ครบทุกฟังก์ชัน และยังไม่มีหลักฐานว่าเมธอดต้นฉบับใดเทียบเท่ากันทั้งเมธอดบนบอร์ดจริง** รายงานนี้เพิ่มรายการเมธอด C++ และฟังก์ชัน C แยกจากรายชื่อ Blockly เพื่อให้เห็นส่วนที่ยังไม่ได้ตรวจ โดยไม่ใช้ชื่อคล้ายกันเป็นหลักฐานว่าทำงานเหมือนกัน

## ขอบเขตและวิธีตรวจ

ตรวจไฟล์ที่คัดออกจาก ZIP ไว้ 410 ไฟล์ เทียบ SHA256 กับหลักฐานเดิม: ไม่ตรง 0 ไฟล์, ขาด 0 ไฟล์ SHA256 ZIP ที่อ้างจากการตรวจครั้งก่อนคือ `8a3f7c8e9b8ba08375853213faac1b42bfac3e767a56fa81edfe9a124125495b` รอบนี้ไม่ได้อ่าน ZIP ทั้งก้อนซ้ำ

รายการเป็น **candidate จาก regex และระดับวงเล็บปีกกา** ไม่ใช่ผล C++ compiler: macro, template, overload, operator, inheritance และเงื่อนไข preprocessor ยังทำให้มีรายการตกหล่นหรือเกินได้ จึงยังเรียกว่า “ครอบคลุมทุกฟังก์ชัน” ไม่ได้

| รายการใน snapshot | จำนวน |
|---|---:|
| Header ของ KidBright/plugin ไม่รวม RapidJSON | 54 |
| Class/struct candidates | 53 |
| Method candidates ทุก access | 694 |
| Public method candidates รวม constructor/lifecycle | 658 |
| Header ของ RapidJSON ที่แยกเป็น dependency | 36 |
| ไฟล์ runtime C/C++ ต้นฉบับ | 38 |
| ไฟล์ C ใน KB ปัจจุบัน | 20 |
| ฟังก์ชันที่ประกาศ export ใน `kb_core.h` | 8 |
| การจับคู่ที่ตรวจขอบเขตด้วยตนเอง รวมสอง recipe | 10 |
| เมธอดต้นฉบับที่พิสูจน์ความเทียบเท่าทั้งเมธอด | 0 |

## โค้ดปัจจุบันมีอะไร

| ไฟล์ | ตระกูลโค้ดจาก include | I2C จาก include | ฟังก์ชันที่พบ |
|---|---|---|---:|
| `accel_kxtj3.c` | esp_idf_c_reference | legacy_driver_i2c_h_deprecated_in_idf_5 | 7 |
| `accel_mc3479.c` | esp_idf_c_reference | legacy_driver_i2c_h_deprecated_in_idf_5 | 3 |
| `adc_ldr_external.c` | esp_idf_c_reference | none_includes_observed | 8 |
| `all_sensors_demo.c` | esp_idf_c_reference | legacy_driver_i2c_h_deprecated_in_idf_5 | 21 |
| `balanced_robot.c` | arduino_cpp_reference_despite_c_extension | none_includes_observed | 3 |
| `calibrate_balancing_bike.c` | esp_idf_c_reference | legacy_driver_i2c_h_deprecated_in_idf_5 | 9 |
| `calibrate_balancing_joystick.c` | esp_idf_c_reference | legacy_driver_i2c_h_deprecated_in_idf_5 | 9 |
| `fomulakid_receiver.c` | esp_idf_c_reference | legacy_driver_i2c_h_deprecated_in_idf_5 | 14 |
| `formulakid_sender.c` | esp_idf_c_reference | legacy_driver_i2c_h_deprecated_in_idf_5 | 10 |
| `kidbright_full_system_demo.c` | esp_idf_c_reference | legacy_driver_i2c_h_deprecated_in_idf_5 | 17 |
| `minibike_receiver.c` | esp_idf_c_reference | legacy_driver_i2c_h_deprecated_in_idf_5 | 14 |
| `minibike_sender.c` | esp_idf_c_reference | new_i2c_driver | 16 |
| `temp_lm73.c` | esp_idf_c_reference | legacy_driver_i2c_h_deprecated_in_idf_5 | 5 |
| `button_buzzer.c` | esp_idf_c_reference | none_includes_observed | 1 |
| `kb_basic.c` | portable_c_core | none_includes_observed | 8 |
| `kb_core.c` | portable_c_core | none_includes_observed | 9 |
| `kb_display_idf.c` | esp_idf_c_reference | none_includes_observed | 4 |
| `matrix_counter.c` | esp_idf_c_reference | new_i2c_driver | 1 |
| `music_scale.c` | esp_idf_c_reference | none_includes_observed | 1 |
| `temperature_serial.c` | esp_idf_c_reference | new_i2c_driver | 1 |

`balanced_robot.c` เป็น Arduino/C++ reference แม้นามสกุลเป็น `.c` ส่วนไฟล์ตัวอย่าง ESP-IDF อื่นใน `sensor_examples` ยังไม่ได้คอมไพล์ในรอบ audit นี้ การพบ include legacy I2C หมายถึงต้องเลือก driver family ให้สอดคล้องกับทั้งโปรแกรม ไม่ได้แปลว่าไฟล์ใช้ไม่ได้โดยอัตโนมัติ

## ส่วนที่ตรวจพฤติกรรมแล้วและข้อจำกัด

แกน C มี 8 ฟังก์ชัน: แปลงวินาทีเป็นมิลลิวินาที, ถอดค่า LM73, PID, SMA, ตั้งค่า/อ่าน joystick และเข้า/ถอดรหัส ASCII `%f` มีหลักฐาน host tests 122 checks แต่อ่านเทียบต้นฉบับเพียงขอบเขตที่ระบุ ไม่ได้รัน differential test กับ runtime ต้นฉบับ

PID ใช้ double และแก้ first-sample/invalid dt; SMA แก้การเปลี่ยนขนาดหน้าต่าง; codec ตรวจแพ็กเก็ตเข้มกว่าต้นฉบับ จึงไม่ใช่ bit-exact implementation เมธอด `FILTER::ema`, framework `Device/Driver`, radio/peer management และเมธอดอื่นไม่ถูกนับว่ารองรับจากการมี helper เหล่านี้

สอง recipe คือกด S1/เสียง 440 Hz 500 ms และ LM73/Serial 500 ms มีหลักฐาน build ESP-IDF 5.5.4 แต่ยังไม่ได้เทียบ waveform, เวลา, ค่าเซนเซอร์ หรือโปรแกรมจริงกับบอร์ด ไม่มีการ flash ใน audit นี้

รอบ 17 ตุลาคมเพิ่ม `kb_basic.c`, HT16K33 adapter และ recipe เพลง/ตัวเลข รวมเป็น 4 recipe และ 7 คู่ profile/recipe ที่ build ผ่านตาม [หลักฐาน expansion](research/kidbright-expansion-2026-10-17-validation.json) มี host checks 122 เดิมและ 368 ใหม่ แต่ไม่เพิ่มจำนวน original method mappings จาก primitive เหล่านี้โดยอัตโนมัติ สถานะ build รายไฟล์ใน JSON ต้องตรงแฮช source/dependencies ของผลทดสอบที่บันทึกไว้

ผู้ใช้รายงานโปรแกรม Hello/Heartbeat ผ่านบน iA เป็นผล basic runtime แยกจากการตรวจนี้ ดู [หลักฐานผู้ใช้](research/kidbright-inex-user-evidence-2026-10-17.json) ยังไม่ได้ทดสอบอุปกรณ์ต่อพ่วงด้วยไฟล์นั้น

ตัวช่วยเลือกภาษาตอบมี 15 identified offline cases ตามรายการทดสอบภาษาเดิม อยู่ใน Rust suite 119 tests ที่บันทึกผลล่าสุด ตรวจเฉพาะการเลือกข้อความและ provider payload ไม่ได้ยืนยันผลตอบจริงครบทุกภาษา

รายละเอียดชื่อ/signature/access/บรรทัด/แฮชของทุก candidate ในขอบเขตอยู่ใน [JSON audit](research/kidbright-function-audit-2026-10-16.json) รายการที่ไม่มี manual mapping มีสถานะ `not_reviewed_no_equivalence_proven` เสมอ ไม่มีการคำนวณเปอร์เซ็นต์จากการจับชื่อ

## ทำซ้ำ

```powershell
python scripts/audit-kidbright-functions.py --archive-root (Join-Path $env:TEMP 'vibekidbright-research-20261015/archive')
```

สคริปต์อ่านไฟล์ต้นฉบับเป็นข้อมูลเท่านั้น ไม่ import, compile หรือ execute โค้ดใน archive แฮช input เป็น snapshot ณ เวลาสร้างรายงาน ควรรันซ้ำเมื่อแก้ source หรือหลักฐานที่อ้าง
