"""One mechanical correction of PowerShell codepage damage, without score tuning.
Only replaces the pre-authored Thai/mixed texts; English, IDs and qrels stay fixed.
"""
import hashlib
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]/'docs/ai-engineering'
TEXTS=[
('แปลงเวลาหน่วงจากวินาทีที่มีทศนิยมเป็นมิลลิวินาทีและตรวจค่าที่ไม่ถูกต้อง','แปลง delay วินาทีเป็น milliseconds พร้อมเช็คค่า'),
('คำนวณความยาวตัวโน้ตจากความเร็วของเพลงและจำนวนจังหวะ','คำนวณ duration ของโน้ตจาก BPM และ beats'),
('จัดข้อมูลพิกเซลสิบหกคอลัมน์แปดแถวเป็นแพ็กเก็ตส่งให้ตัวควบคุมจอ','pack เฟรม LED 16x8 ไปยัง HT16K33'),
('กดปุ่มแล้วส่งเสียงหนึ่งครั้งจากนั้นรอให้ปล่อยปุ่มก่อน','กด S1 แล้ว beep ครั้งเดียว ต้องรอ release'),
('อ่านอุณหภูมิผ่านบัสเซนเซอร์โดยแปลงค่าติดลบได้ถูกต้อง','อ่าน temperature ติดลบจาก LM73 ผ่าน I2C'),
('เก็บสถานะตัวควบคุมสัดส่วนสะสมและอนุพันธ์โดยใช้เวลาที่ผ่านไป','ตัวควบคุม PID ต้องเก็บ integral และ previous error'),
('ลดความแกว่งของค่าที่วัดด้วยค่าเฉลี่ยเลื่อนหน้าต่าง','กรองสัญญาณด้วย moving average แบบ window'),
('ปรับเทียบจุดกึ่งกลางคันโยกและแปลงช่วงค่าอินพุต','calibrate joystick midpoint แล้ว map ค่า raw'),
('เข้ารหัสและถอดข้อมูลตัวเลขไร้สายเป็นข้อความแทนการส่งทศนิยมไบนารี','ส่ง ESP-NOW payload แบบ ASCII ตัวเลข ไม่ใช่ binary float'),
('บอร์ดสีชมพูกับบอร์ดที่มีเซนเซอร์ความเร่งต่างกันอย่างไร','เทียบ KidBright32iP กับ iA เรื่องแสงและ accelerometer'),
('พอร์ตอินพุตและเอาต์พุตดิจิทัลภายนอกต่อกับขาใด','IN1 IN4 OUT1 OUT2 ใช้ GPIO ไหน'),
('ขอตัวอย่างอ่านความเร่งสามแกนของบอร์ดรุ่นที่ใช้ชิปเอ็มซี','อ่าน acceleration เซนเซอร์ MC3479 ตัวอย่าง driver'),
('อ่านค่าแกนจากรีจิสเตอร์ของเซนเซอร์ความเร่งรุ่นเก่า','อ่านค่าแกน KXTJ3 ผ่าน register I2C'),
('อ่านตัวต้านทานแปรตามแสงภายนอกด้วยวงจรแปลงแอนะล็อก','อ่าน external LDR ผ่าน ADC แบบแยก'),
('ต่อจอภาพขาวดำแบบโอแอลอีดีด้วยบัสสองสายและเริ่มต้นใช้งาน','ต่อจอ OLED SSD1306 ผ่าน I2C ต้องตั้งค่าอะไร'),
('ขอตัวอย่างเฟิร์มแวร์หุ่นยนต์รักษาสมดุลทรงตัว','ตัวอย่าง self balancing robot โค้ดควบคุม'),
('อธิบายการต่อวงจรรีโมตและรูปแบบสื่อสารของรถฟอร์มูลา','รีโมต Formula Kid ใช้ protocol และขาอะไร'),
('ข้อจำกัดวงจรไฟฟ้าและการสื่อสารของมอเตอร์ไซค์จำลองมีอะไรบ้าง','minibike ข้อจำกัด hardware และการสื่อสาร'),
('บอร์ดปัญญาประดิษฐ์ที่มีกล้องใช้เครื่องมือคอมไพล์เหมือนบอร์ดไมโครคอนโทรลเลอร์หรือไม่','บอร์ด μAI Linux ใช้ ESP-IDF เหมือน KidBright32 หรือเปล่า'),
('ต้องตรวจสอบรุ่นย่อยของบอร์ดจากหลักฐานใดก่อนกำหนดขาวงจร','ยืนยัน board revision จาก schematic ก่อนเลือก pin mapping'),
]
def main():
    path=ROOT/'evaluation-heldout-v1.json'
    cases=json.loads(path.read_text(encoding='utf-8'))
    if not any('???' in q['query'] for q in cases):
        raise RuntimeError('Dataset is not codepage damaged; do not rewrite frozen evaluation data')
    archive=ROOT/'invalid-encoding-results';archive.mkdir(exist_ok=True)
    (archive/path.name).write_bytes(path.read_bytes())
    freeze=ROOT/'evaluation-heldout-v1-freeze.json'
    (archive/freeze.name).write_bytes(freeze.read_bytes())
    original=json.loads(path.read_text(encoding='utf-8'))
    for i,(thai,mixed) in enumerate(TEXTS):
        cases[i*3+1]['query']=thai;cases[i*3+2]['query']=mixed
    # Restore the one Greek character damaged by the same pipe in the English row.
    # English wording and qrels are otherwise byte-for-byte identical.
    for before,after in zip(original,cases):
        assert before['id']==after['id'] and before['relevant_files']==after['relevant_files']
        if after['language'] in {'en','technical'}:assert before==after
    assert all(any('\u0e00'<=c<='\u0e7f' for c in q['query']) for q in cases if q['language'] in {'th','mixed'})
    data=(json.dumps(cases,ensure_ascii=False,indent=2)+'\n').encode('utf-8');path.write_bytes(data)
    manifest=json.loads(freeze.read_text(encoding='utf-8'))
    manifest['invalid_previous_sha256']=manifest['sha256'];manifest['sha256']=hashlib.sha256(data).hexdigest()
    manifest['encoding_correction']='PowerShell native pipe replaced Thai with question marks. Original intended Thai/mixed texts restored from the authored task source; IDs, qrels, English, model selection and ranker parameters unchanged. First run invalid, retained separately. No score-based tuning.'
    freeze.write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print('Restored UTF-8; frozen digest '+manifest['sha256'])
if __name__=='__main__':main()
