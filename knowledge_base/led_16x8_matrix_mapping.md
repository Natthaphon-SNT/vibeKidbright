# KidBright32 16x8 LED Matrix (HT16K33) Mapping Reference

## The Optical Illusion Problem
When writing C arrays for the KidBright 32iA LED Matrix, it is extremely easy to accidentally generate halves of images that look like two distinct arrows pointing outwards (e.g. `<--` and `-->`), leading to the "two arrows on two screens" visual bug.

This happens because the **16x8 HT16K33 Matrix on the KidBright32 is physically mapped in an interleaved, counter-clockwise 90-degree rotated format**.

## Hardware Memory Mapping (Reverse Engineered)
The I2C `led_matrix_show` command sends exactly 16 bytes. The HT16K33 reads these 16 bytes in pairs (Low Byte, High Byte).
On the KidBright 32iA, it translates to the visual 16x8 Canvas (Cols 0-15, Rows 0-7) as follows:

1. **Left Screen (Cols 0-7):** Mapped to the **Even Indexes** (Low Bytes).
   - `Array[0, 2, 4, 6, 8, 10, 12, 14]` = Canvas Columns `0, 1, 2, 3, 4, 5, 6, 7` (Left to Right)
2. **Right Screen (Cols 8-15):** Mapped to the **Odd Indexes** (High Bytes).
   - `Array[1, 3, 5, 7, 9, 11, 13, 15]` = Canvas Columns `8, 9, 10, 11, 12, 13, 14, 15` (Left to Right)
3. **Y-Axis (Rows 0-7):** Mapped to **Bits 0 to 7**.
   - `Bit 0` (0x01) = Top Row (Row 0)
   - `Bit 7` (0x80) = Bottom Row (Row 7)

## How to Translate a Visual Canvas to C Array
If you design an image on a 16x8 Grid where `C[x]` represents the hex value for Column `x` (Bits 0-7):
```c
Array[0]  = C[0];  // Far Left Edge
Array[1]  = C[8];  // Left edge of Right screen
Array[2]  = C[1];
Array[3]  = C[9];
Array[4]  = C[2];
Array[5]  = C[10];
Array[6]  = C[3];
Array[7]  = C[11];
Array[8]  = C[4];
Array[9]  = C[12];
Array[10] = C[5];
Array[11] = C[13];
Array[12] = C[6];
Array[13] = C[14];
Array[14] = C[7];  // Right edge of Left screen
Array[15] = C[15]; // Far Right Edge
```

## Perfect Centered Arrow Examples
To draw shapes that beautifully span the physical gap in the center of the board, use these pre-calculated matrices:

```c
// จุดกึ่งกลาง (สี่เหลี่ยม 4x4 อยู่กึ่งกลางรอยต่อระหว่างจอ)
const uint8_t img_center[16] = {0x00, 0x18, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0x18, 0x00};

// รูปลูกศรชี้ขึ้น (วาดพาดตรงกลางหน้าจอเป็นทรงจรวดชัดเจน)
const uint8_t img_up[16]    = {0x00, 0xFF, 0x00, 0xFE, 0x00, 0x0C, 0x00, 0x08, 0x08, 0x00, 0x0C, 0x00, 0xFE, 0x00, 0xFF, 0x00}; 

// รูปลูกศรชี้ลง
const uint8_t img_down[16]  = {0x00, 0xFF, 0x00, 0x7F, 0x00, 0x30, 0x00, 0x10, 0x10, 0x00, 0x30, 0x00, 0x7F, 0x00, 0xFF, 0x00};

// ลูกศรชี้ซ้าย (หัวลูกศรอยู่ฝั่งซ้าย ก้านยาวพาดเข้าจอขวา)
const uint8_t img_left[16]  = {0x00, 0x18, 0x00, 0x18, 0x18, 0x18, 0x3C, 0x18, 0x7E, 0x18, 0xFF, 0x18, 0x18, 0x00, 0x18, 0x00};

// ลูกศรชี้ขวา
const uint8_t img_right[16] = {0x00, 0x18, 0x00, 0x18, 0x18, 0xFF, 0x18, 0x7E, 0x18, 0x3C, 0x18, 0x18, 0x18, 0x00, 0x18, 0x00};
```

> [!IMPORTANT]
> Always use this memory mapping logic when the user requests custom 16x8 drawings. The visual output will completely shatter if standard linear left-to-right arrays are used.
