// Audited source contracts, not a claim of executing 128 generated programs.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';

const check = process.argv.includes('--check');
const read = path => readFileSync(path, 'utf8');
const json = path => JSON.parse(read(path));
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const catalogPath = 'docs/research/uai-plus-runtime-2026-10-26/test-prompts-th.json';
const evidencePath = 'docs/research/uai-plus-runtime-2026-10-26/api-contracts-2026-10-27.json';
const registryPath = 'knowledge_base/compatibility/uai_plus_test_contracts.json';
const reportPath = 'docs/KIDBRIGHT_MAI_PLUS_PROMPT_AUDIT_2026-10-27.md';
const auditCsvPath = 'docs/KIDBRIGHT_MAI_PLUS_PROMPT_AUDIT_2026-10-27.csv';
const guide = 'kidbright_uai_plus_test_contracts.md';
const catalog = json(catalogPath), evidence = json(evidencePath);
if(catalog.total!==128 || catalog.tests.length!==128)throw Error('The audit must cover exactly 128 cases.');
if (evidence.image.sha256 !== '287883ec84279574ecb70eb3856d956b23e1a9840a6ad4e946e74054deb7b422') throw Error('Wrong image reference');
const contracts = {
  discovery: { symbols: {'sys':['device_name'], 'peripheral.pinmap':['get_pins','get_pin_functions']}, note:'Read-only capabilities template; device_id optional/UNKNOWN. No peripheral constructors except display. Node/symbol discovery is not ownership or wiring.' },
  display: { symbols: {'display':['Display.__init__','Display.width','Display.height','Display.show','Display.close','Display.get_backlight','Display.set_backlight'],image:['Image.__init__','Image.draw_rect','Image.draw_line','Image.draw_circle','Image.draw_string','Image.save','load']}, note:'Integer geometry and actual orientation. Backlight 0–100 with saved value; no direct PWM10. Thai font requires a real font file.' },
  touch: { symbols: {touchscreen:['TouchScreen.__init__','TouchScreen.read','TouchScreen.available','TouchScreen.close']}, note:'One [x,y,pressed] point; pressed-gated hit tests, edge detection and fit offsets. No multi-touch guarantee.' },
  adc: { symbols: {'peripheral.adc':['ADC.__init__','ADC.read','RES_BIT_12']}, note:'Plus source maps ADC0 to S1/S2. Use measured MP-021 bands, not old constants; no guaranteed close/deinit.' },
  led: { symbols:{}, note:'Verify led-user node, ownership and original active trigger/brightness before writes. Restore bracketed active trigger; on/off is not PWM dimming.' },
  gpio: { symbols:{'peripheral.gpio':['GPIO.__init__','GPIO.value','GPIO.reset','Mode.IN','Mode.OUT','Pull.PULL_NONE'], 'peripheral.pinmap':['get_pins','get_pin_functions','set_pin_function']}, note:'Source aliases are not physical PCB proof. P24/IO7 unresolved. No get_pin_function or GPIO.close declaration. Reserve A16/A17; fixture and safe LOW/START gate required.' },
  pwm: { symbols:{'peripheral.pwm':['PWM.__init__','PWM.freq','PWM.duty','PWM.disable']}, note:'ID/function/pin are separate; duty is percent. C906Servo helper not found at selected image paths. No IO7 actuation until channel/backend proven.' },
  i2c: { symbols:{'peripheral.i2c':['list_devices','I2C.__init__','I2C.scan','I2C.writeto','I2C.readfrom']}, note:'Confirm external backend/pins/ownership and module protocol first. Software bus 5 is not automatically present. No invented readfrom timeout or implicit insmod.' },
  imu: { symbols:{'ext_dev.imu':['IMU.__init__','IMU.read','Mode.DUAL']}, note:'DUAL contract is six axes; seventh temperature not guaranteed. No unconditional [6], x10 conversion, sign remap or unsupported hardware tap.' },
  rtc: { symbols:{'ext_dev.bm8563':['BM8563.__init__','BM8563.now','BM8563.datetime','BM8563.deinit']}, note:'Module bm8563, not rtc. Actual chip/bus and RTC/NTP ownership need confirmation. Read-only kernel node preferred; no time writes or automatic service changes.' },
  uart: { symbols:{'peripheral.uart':['list_devices','UART.__init__','UART.read','UART.write','UART.close']}, note:'Read timeout is milliseconds, 0 immediate and -1 blocking. Confirm user connector and free port; do not open UART0/A16/A17 owned by the protocol.' },
  camera: { symbols:{camera:['Camera.__init__','Camera.read','Camera.close','Camera.width','Camera.height','Camera.hmirror','Camera.vflip','Camera.set_resolution'],image:['Image.find_blobs','Image.find_qrcodes','Image.find_barcodes','Image.save','QRCode.payload','BarCode.payload']}, note:'Real sensor/frame geometry, RGB888 and FPS from actual successful frames. Each overlay before show; close camera. QR data never executes commands.' },
  audio: { symbols:{audio:['Player.__init__','Player.play','Recorder.__init__','Recorder.record','Recorder.finish','Format.FMT_S16_LE']}, note:'Player.stop/close absent and Player.volume documented unsupported. Use low-amplitude raw PCM and short chunks; recorder.record() can be empty, finish recorder, no guessed audio.read/close.' },
  network: { symbols:{'network.wifi':['Wifi.__init__','Wifi.get_ip','Wifi.get_ssid']}, note:'Use Python socket/urllib/http.server with finite timeout. maix.http is not HTTP GET/POST. MQTT/WebSocket packages and endpoints must be supplied/verified; no pip or RNDIS changes.' },
  files: { symbols:{}, note:'Python 3.11 pathlib/os/json/csv/hashlib/wave as applicable. Writable unique test directory, bounded bytes, atomic replace on same filesystem. No system files, full-card or forced-power-loss tests.' },
  models: { symbols:{nn:['Classifier.__init__','Classifier.classify','YOLOv5.__init__','YOLOv5.detect','YOLOv8.detect','YOLO11.detect']}, note:'Requires actual compatible model/labels/input and audio preprocessing. API presence is not model or output compatibility; no fake inference.' },
  lifecycle: { symbols:{app:['need_exit','set_exit_flag']}, note:'User-operated Run/Stop; adapter owns Launcher/process group. Intentionally failing tests expect nonzero exits. No physical PASS from compile/upload; synthetic fixtures are not newer installed versions.' },
  extensions: { symbols:{}, note:'Discovery/manual procedure only until actual exposed bus/driver/ownership and power configuration are proven. No USB role switch, kernel module loading or motor actuation by default.' }
};
for (const [name, contract] of Object.entries(contracts)) {
  for (const [module, symbols] of Object.entries(contract.symbols)) for (const symbol of symbols)
    if (!evidence.modules[module]?.symbols[symbol]) throw Error(`Unverified API: ${name} ${module}.${symbol}`);
  contract.declarations = Object.fromEntries(Object.entries(contract.symbols).map(([module,symbols]) =>
    [module,Object.fromEntries(symbols.map(symbol=>[symbol,evidence.modules[module].symbols[symbol]]))]));
  contract.guide = guide;
  contract.image_evidence = evidencePath;
}
const profiles = new Map();
function assign(start,end,keys,state='DRAFT_REFERENCE',requirements=[],dependencies=[]) {
  for(let id=start;id<=end;id++) profiles.set(id,{contracts:keys.split(','),state,requirements:[...requirements],dependencies:[...dependencies]});
}
assign(1,5,'discovery,display','TEMPLATE_CHECKED');
assign(6,6,'discovery,display');
assign(7,14,'display');
assign(15,15,'touch,display','TEMPLATE_CHECKED');
assign(16,19,'touch,display');
assign(20,20,'touch','HOLD',['This single-point API cannot establish multi-touch; restrict this case to capability discovery.']);
assign(21,21,'adc,display','TEMPLATE_CHECKED');
assign(22,24,'adc,display','NEEDS_SETUP',['Measured non-overlapping released/S1/S2 bands and tester confirmation.'],['MP-021']);
assign(25,25,'led,display','NEEDS_SETUP',['Actual led-user path/ownership and saved active trigger/brightness.']);
assign(26,26,'adc,led,display','NEEDS_SETUP',['MP-021 bands and verified user LED node.'],['MP-021','MP-025']);
assign(27,40,'gpio,display','NEEDS_SETUP',['Confirmed runtime pin/function and connector wiring; correct voltage/pull/LED or instrument fixture.'],['MP-003']);
for(const id of [37,38]) profiles.set(id,{...profiles.get(id),state:'HOLD',requirements:['IO7=P24 alias/function/routing not proven; read-only discovery first.']});
assign(41,43,'gpio,display','HOLD',['All seven connector mappings, including IO7/P24, and all-channel safe fixture must be established.'],['MP-003']);
assign(44,48,'gpio,display','NEEDS_SETUP',['Verified pins, one output per loopback pair, series resistor/safe LED fixture and START gate.'],['MP-003']);
assign(49,49,'gpio,pwm');
assign(50,54,'gpio,pwm,touch','HOLD',['Confirmed IO7 PWM pin/function/channel/backend; instrument proof before servo; actual servo pulse limits/power.'],['MP-049']);
assign(55,55,'i2c');
assign(56,64,'i2c,files','NEEDS_SETUP',['Actual external bus/backend, correct module wiring/voltage/address and matching device protocol/driver. No implicit module loading.'],['MP-055']);
assign(65,69,'imu,display');
assign(70,70,'rtc,files');
assign(71,71,'rtc,files','NEEDS_SETUP',['Actual hardware RTC source plus separately recorded reference times.'],['MP-070']);
assign(72,72,'rtc,lifecycle','HOLD',['Manual retention protocol excluding automatic RTC/NTP sync; orderly power-off. No automatic power-cycle or service changes.'],['MP-070']);
assign(73,73,'uart');
assign(74,78,'uart,display','NEEDS_SETUP',['Free user UART path, actual connector pinmap, correct logic-level fixture/peer and matching framing/baud.'],['MP-073']);
assign(79,82,'camera,display');
assign(83,83,'camera,adc,files','NEEDS_SETUP',['MP-021 bands and a chosen bounded test output directory.'],['MP-021','MP-079']);
assign(84,85,'camera,display','NEEDS_SETUP',['Actual colored targets or known QR/barcode payloads and the measured thresholds/geometry.'],['MP-079']);
assign(86,86,'camera,display,lifecycle');
assign(87,91,'audio,files,lifecycle');
assign(92,92,'audio,camera,display','NEEDS_SETUP',['Camera and microphone pass independently; concurrent operation is still to be measured.'],['MP-079','MP-088']);
assign(93,93,'network,files');
assign(94,100,'network,lifecycle','NEEDS_SETUP',['Explicit allowed endpoint/broker/LAN address/port; existing network; compatible MQTT/WebSocket package when used.'],['MP-093']);
assign(101,102,'files');
assign(103,104,'files','NEEDS_SETUP',['Actual sensor inputs for logging or a chosen persistent directory for config across new /tmp runs.']);
assign(105,105,'files');
assign(106,106,'files,lifecycle','NEEDS_SETUP',['Chosen persistent test directory; closed files and orderly restart procedure.']);
assign(107,112,'models,camera,audio','NEEDS_SETUP',['Real SG2002-compatible model/labels/input metadata; exact audio preprocessing when applicable; ground-truth fixtures.']);
assign(113,117,'lifecycle,display');
assign(118,118,'uart,network,lifecycle','NEEDS_SETUP',['A chosen documented finite-timeout I/O path and controlled test peer. Native I2C timeout is not assumed.']);
assign(119,119,'display,touch,imu,camera,lifecycle','NEEDS_SETUP',['Separate passing results for every selected subsystem; memory/log bounds.'],['MP-015','MP-065','MP-079']);
assign(120,120,'lifecycle','HOLD',['Desktop synthetic compatibility fixtures only; do not upload as a newer-runtime hardware test.']);
assign(121,122,'extensions','HOLD',['Actual exposed peripheral/adapter, pinmux/driver, ownership and matching public device documentation.']);
assign(123,123,'extensions,discovery');
assign(124,125,'extensions','HOLD',['Separate control transport and verified USB/power arrangement, or an exact expansion protocol/power/driver fixture.']);
assign(126,126,'display,touch,adc,imu,camera,audio','NEEDS_SETUP',['Each selected component tested separately; calibration results for buttons.'],['MP-015','MP-021','MP-065','MP-079','MP-088']);
assign(127,127,'gpio,i2c,uart,pwm','HOLD',['Every connector and subsystem verified separately, including unresolved IO7/P24/PWM; no blanket all-IO actuation.']);
assign(128,128,'files,lifecycle');
// Carry prerequisite results into the exact public test reference as well as prose.
for (const [start,end,dependency] of [[16,19,'MP-015'],[66,69,'MP-065'],[80,86,'MP-079']]) {
  for (let id=start;id<=end;id++) {
    const p=profiles.get(id);
    if(!p.dependencies.includes(dependency))p.dependencies.push(dependency);
  }
}
profiles.get(49).dependencies.push('MP-003');
profiles.get(53).dependencies.push('MP-015');
profiles.get(89).dependencies.push('MP-087','MP-088');
profiles.get(90).dependencies.push('MP-088');
profiles.get(108).dependencies.push('MP-107');
profiles.get(109).dependencies.push('MP-107','MP-079');
profiles.get(110).dependencies.push('MP-107','MP-088');
profiles.get(111).contracts.push('led','gpio');
profiles.get(111).requirements.push('A passing chosen inference test, plus a verified user LED (MP-025) or an individually verified GPIO LED fixture.');
profiles.get(112).requirements.push('A passing chosen inference test (MP-108, MP-109 or MP-110), with its actual cleanup contract.');

// Each correction is an explicit review decision, not inferred hardware support.
const fixes = {
1:'ใช้ capabilities; sys.device_id ไม่มีใน stub รุ่นนี้จึงเป็น optional/UNKNOWN ห้ามเรียก device_key',
2:'ตรวจ symbol แบบไม่สร้างอุปกรณ์ รายงาน ext_dev.bm8563 ไม่ใช่ ext_dev.rtc และไม่ import package เสริม',
3:'มี get_pins/get_pin_functions แต่ไม่มี get_pin_function ใน reference 4.12.5; IO7/P24 ยังไม่ใช่ routing ที่ยืนยัน',
4:'อ่านเฉพาะ node/name ที่ระบุ ไม่ dump process command line, credential หรือเปิด serial แย่งเจ้าของ',
5:'แยก sensor_cfg กับชนิด IMU/RTC ที่ยัง UNKNOWN; ชื่อโมดูลไม่ใช่ chip ID',
6:'เป็น planner ไม่ขับพอร์ตหรืออ้างผล self-test จน fixture ของชุดนั้นพร้อม',
7:'ทดสอบสีเป็นแกนหลัก; input เสริมใช้ได้หลัง MP-015 หรือ MP-021 ผ่านเท่านั้น',
8:'ปรับขนาดข้อความให้เห็นขอบและ orientation จริง ไม่อิง 640x480 ตายตัว',
9:'ASCII เป็น baseline; หน้าไทยแยก conditional ต้องมีไฟล์ font ที่มีจริง',
10:'ใช้ draw_line/draw_rect/draw_circle และพิกัด integer จาก reference ที่บันเดิล',
11:'FPS วัดจากเฟรมที่ show สำเร็จ/monotonic elapsed ไม่ตั้งเกณฑ์ FPS ที่ยังไม่มี benchmark',
12:'fit และอัตราส่วนจากขนาดจริง; อย่าคาดว่าภาพ 240x240 ต้องเต็มจอโดยไม่เกิด letterbox',
13:'get_backlight/set_backlight ใช้ 0–100; ต้องอ่านค่าเดิมได้ก่อนจึงเปลี่ยน และคืนใน finally',
14:'ไม่สมมติ partial update API; ใช้ buffer ที่มีขอบเขตและเก็บสถิติจริง',
15:'ใช้ python_example=touchscreen ซึ่งมี marker และ press/release จริง ไม่ใช่ display-only skeleton',
16:'แตะเป้าทีละจุด เก็บ raw และระยะคลาดเคลื่อน; ไม่เขียน calibration ถาวร',
17:'CLEAR/EXIT ต้องตรวจ pressed และขอบกด; ไม่ลากเส้นข้ามช่วงปล่อยนิ้ว',
18:'ใช้ monotonic edge/hold timer แยกจาก rendering loop ไม่เพิ่มตัวนับทุกเฟรม',
19:'รวม scale และ letterbox offset ทั้งสองทิศทาง; ทดสอบขอบดำและขอบปุ่ม',
20:'ลดขอบเขตเป็น SINGLE_TOUCH_ONLY capability report เพราะ API นี้ส่งได้หนึ่งจุด ไม่ใช้สองนิ้วเป็นเกณฑ์ผ่าน',
21:'ใช้ python_example=adc_buttons; วัด 3 วินาที/สถานะก่อนเสนอ threshold และไม่ใช้ threshold เก่า',
22:'ใช้ bands จาก MP-021; กด 20 ครั้งเป็น ground truth ที่ผู้ทดสอบยืนยัน',
23:'threshold จาก calibration; long press ใช้ monotonic และเกิดครั้งเดียวต่อรอบ',
24:'ไม่กดพร้อมกันจนบันทึก bands แยกแล้ว; รายงาน AMBIGUOUS เมื่อแยกไม่ได้',
25:'restore เฉพาะ active trigger ที่อยู่ในวงเล็บกับ brightness เดิม ไม่เขียนรายการ trigger ทั้งบรรทัดกลับ',
26:'init ADC ครั้งเดียว; ใช้ calibration จริงและ LED node ที่ยืนยันแล้ว',
37:'HOLD การขับ IO7: P24 เป็นค่าจาก dropdown ไม่ใช่ข้อพิสูจน์ว่า native runtime รับ alias นี้',
38:'HOLD การตั้ง IO7 เป็น input เช่นเดียวกัน ห้ามเปลี่ยนเป็น A24 แทน',
41:'HOLD การอ่านครบเจ็ดช่องจน IO7 ยืนยัน; floating input ไม่เป็น FAIL ของพอร์ต',
42:'HOLD all-seven output; ห้ามขับ alias ที่ยังไม่ยืนยันเพื่อให้ครบตาราง',
43:'walking-zero ต้องประเมินกระแสรวม fixture ไม่เท่ากับการขับ GPIO ทุกช่องพร้อมกันโดยไร้เงื่อนไข',
44:'IO2 output → IO3 input ผ่าน series resistor; อีกด้านไม่เป็น output',
45:'IO4 output → IO5 input มี timeout ใน loop ที่ควบคุมเองและวัด input จริง',
46:'latency เป็น userspace-observed เท่านั้น ไม่ใช่สเปก hardware GPIO',
47:'กรอก active-high/low และ pull ตามสวิตช์จริง อย่าเดาว่าปล่อยต้องเป็น HIGH',
48:'ใช้ GPIO ที่ยืนยันแล้วกับ LED เท่านั้น; forced kill อาจไม่รัน finally ต้องตรวจ LOW จริงหรือ UNKNOWN',
49:'discovery เท่านั้น ไม่มีการสร้าง PWM; pin/function/channel เป็นคนละข้อมูล',
50:'HOLD จน mapping/backend แน่นอน; ต้องใช้เครื่องมือวัดก่อนต่อ servo ไม่เดา PWM2 จาก helper ที่ไม่พบ',
51:'duty เป็นเปอร์เซ็นต์; HOLD จน IO7 PWM ยืนยัน ห้ามใช้ช่อง backlight แทน',
52:'servo.C906Servo ยังไม่ใช่ dependency ที่ยืนยันใน image; ห้าม import แล้วอ้างว่าพร้อม',
53:'ต้องผ่าน MP-015 และ mapping/pulse limits ของ servo ก่อนรับค่า slider',
54:'ต้องวัดสัญญาณหลัง Stop จริง; disable() มีใน API แต่ไม่รับประกัน cleanup เมื่อ kill',
55:'list_devices/node เป็น discovery ไม่ scan หรือเปิดบัสภายใน และไม่ insmod อัตโนมัติ',
56:'scan เป็น probe ที่อาจมีผลต่ออุปกรณ์ ไม่ใช่ read-only discovery; จำกัด bus/address/operation ตามโมดูล',
57:'ต้องมี external bus และ SHT31 protocol ที่ตรงรุ่น; ตรวจ None/length/CRC ก่อนแปลงค่าจริง',
58:'ระบุรุ่น/measurement command/conversion ของโมดูล ไม่ใช้ค่า raw ของบัสเป็น lux',
59:'ระบุ controller SSD1306/SH1106 และ resolution จากโมดูลจริง ไม่เดาด้วย address',
60:'ตรวจ address conflict และแชร์ bus handle; ไม่สร้างหลายเจ้าของอุปกรณ์เดียวกัน',
61:'ใช้ external ADC ตามชนิด/gain/reference จริง; ADC0 onboard เป็นปุ่ม ไม่ใช่พอร์ต POT ทั่วไป',
62:'เลือกชนิด single-ended/differential และช่วงแรงดันจริง ก่อนทดสอบทุกช่อง ADC ภายนอก',
63:'API readfrom รุ่นนี้ไม่มี timeout ที่ยืนยัน; แยก NACK/None จาก native call ค้าง และใช้ Stop supervisor ตามจริง',
64:'iKB-1Z ต้องมี protocol/driver และการจ่ายไฟตามคู่มือรุ่นจริง ไม่ถือว่ามี Python library ติดตั้งแล้ว',
65:'read() รับประกันหกแกนใน dual mode ไม่ใช่เจ็ดค่า; ตรวจ len และอย่า index [6] เพื่ออุณหภูมิโดยไม่มีหลักฐาน',
66:'ใช้หน่วย/ทิศแกนที่ยืนยัน; raw ผ่านหกท่าไม่ใช่ calibration ที่ใช้กับทุกบอร์ด',
67:'bias เป็นชั่วคราวใน RAM; ระบุหน่วย gyro ที่ตรวจแล้ว ไม่เขียนถาวร',
68:'ไม่คัด x10/sign inversion ของ generator เป็น universal mapping; จำกัดตำแหน่งบนจอจริง',
69:'software shake ไม่ใช่ hardware tap; ห้ามใช้ msa311 แทน QMI8658',
70:'ใช้ kernel RTC ที่มีจริงหรือ bm8563 API หลังยืนยัน chip/bus; ext_dev.rtc ไม่มีใน stub นี้',
71:'ต้องมีเวลาอ้างอิงจริงสองจุดและแยก wall clock jump/NTP ก่อนอ้าง drift',
72:'HOLD battery pass จนแยก NTP/auto RTC sync ได้; ไม่สั่งปิดไฟหรือแก้ service อัตโนมัติ',
73:'UART0/A16/A17 มีเจ้าของจาก log เดิม; list_devices ไม่ได้แปลว่าพอร์ตว่าง',
74:'ใช้ user UART ที่ยืนยันและ read(timeout หน่วย ms) เท่านั้น ไม่ใช้พอร์ตระบบเป็น loopback',
75:'USB-UART console กับ user UART แยกหน้าที่; ต้องยืนยัน connector/path ก่อนเปิด',
76:'ใช้ byte framing และ bounded partial buffer ไม่ decode arbitrary bytes เป็นข้อความ',
77:'read timeout=0 คือ immediate, -1 คือ block; ห้ามใช้ -1 ใน Stop-responsiveness test',
78:'ตั้ง baud อีกฝั่งตรงทุกครั้ง เก็บ checksum/sequence และรายงานค่าที่ driver ปฏิเสธแยกกัน',
79:'ตัวอย่าง camera seed เดิมยังไม่มี FPS; ต้องเพิ่ม measured FPS และ close จริงก่อนเรียกว่า MP-079 ครบ',
80:'camera_overlay เป็น seed ของ overlay ต้องเพิ่มรายละเอียดตามพรอมท์ก่อนถือว่าครบ MP-080',
81:'hmirror/vflip มี API แต่ต้องรู้ค่าเดิมก่อนเปลี่ยน/restore; ไม่ยืนยัน orientation จากรูปนิ่ง',
82:'ปิด device เดิมก่อนเปิด resolution ใหม่ อ่านขนาดจริงและไม่สร้างกล้องทุกเฟรม',
83:'capture หนึ่งครั้งต่อ press edge ใช้ bands จาก MP-021 และบันทึกเฉพาะ directory ทดสอบ',
84:'threshold ตามแสง/วัตถุจริง; centroid/area ไม่ใช่ AI confidence',
85:'payload API ยืนยัน แต่ห้ามเปิด URL หรือ execute QR payload อัตโนมัติ',
86:'FPS/memory/Stop เป็นสิ่งที่ต้องวัด ไม่รับรอง 10 นาทีจาก fixture สามเฟรม',
87:'Player.volume ใน stub ระบุ unsupported; ใช้ PCM amplitude ต่ำแทนการพึ่ง setter และไม่ใช้ stop()/close() ที่ไม่มี',
88:'Recorder.record() คืน buffered PCM หรือ empty; ไม่มี Recorder.read(); RMS/peak ไม่ใช่ calibrated dB SPL',
89:'ปิด recorder ด้วย finish(), ใช้ WAV metadata จริงและลด PCM playback amplitude; ไม่ผูก Stop กับ record_ms=5000 ที่ block',
90:'signedness/endian/width ตรง Format.FMT_S16_LE; clipping ratio ต้องคำนวณจาก PCM จริง',
91:'Player ไม่มี stop/close ใน baseline; ใช้ short PCM chunks/exit loop และปล่อย references; ไม่อ้างว่า finally รันหลัง kill',
92:'full-duplex/resource sharing ยังต้องวัด ไม่เปิด loopback ไมค์→ลำโพงเอง',
93:'อ่าน interface/route แบบ bounded ไม่พิมพ์รหัสผ่าน token หรือ device_key',
94:'ใช้ network เดิมกับ endpoint ที่อนุญาต ไม่เปลี่ยน SSID/route/RNDIS เพื่อให้เทสผ่าน',
95:'ใช้ urllib.request ไม่ใช่ maix.http.get ซึ่งไม่มีใน API นี้; timeout/status/schema แยกกัน',
96:'ส่งได้เฉพาะ endpoint/payload ที่ผู้ทดสอบระบุ ไม่สร้างปลายทางสาธารณะหรือส่งภาพ/เสียงเอง',
97:'ต้องมี compatible MQTT package/broker/topic/credential จริง ไม่ assume maix.network.mqtt หรือ pip เอง',
98:'จำกัด queue/backoff และทดสอบปลายทางหลุดโดยไม่ตัด USB ที่ใช้ IDE',
99:'ต้องมี WebSocket package/API จริง แยก port 5050 ของ IDE และปิดตาม library ที่เลือก',
100:'ใช้ HTTPServer.timeout + handle_request ใน exit-aware loop แล้ว server_close; หลีกเลี่ยง same-thread shutdown deadlock',
101:'read-only storage inventory ไม่มี format/fsck หรือเขียนทดสอบจนเต็ม',
102:'unique test file ภายใต้พื้นที่ของโปรแกรม ขนาด 64KiB; ปิดแล้วอ่านกลับเทียบ hash',
103:'sensor ต้องผ่านแยกก่อน; log error/status ไม่เติมเลข 0 เป็นข้อมูล sensor ปกติ',
104:'แต่ละ Run ใช้ /tmp ใหม่ จึงต้องเลือก persistent directory ก่อนอ้าง config อยู่ข้าม Run',
105:'ใช้ Python path/file API ไม่ต่อชื่อไทยหรือชื่อมีช่องว่างเข้าคำสั่ง shell',
106:'ยืนยันตำแหน่งถาวรและ orderly restart; ห้ามแนะนำถอดไฟระหว่างเขียน SD',
107:'ตรวจ compatible model metadata จากไฟล์จริง ไม่ถือว่า NN API มีแล้วโมเดลใดก็โหลดได้',
108:'Classifier.classify ยืนยัน API แต่ต้องมี model/labels/preprocessing/ground truth ที่ตรงกัน',
109:'YOLO API รุ่นนี้มี v5/v8/11; เลือกจาก model metadata ไม่เดาจากนามสกุลอย่างเดียว',
110:'audio preprocessing เป็นของแต่ละโมเดล ต้องมี specification ก่อน inference; ไม่ใช้ Speech แทน generic classifier',
111:'ต้องผ่าน inference และ GPIO แยกก่อน; first fixture เป็น LED ไม่เป็น relay/motor',
112:'โหลดโมเดลครั้งเดียวต่อรอบ ไม่สร้าง NN/Camera ทุกเฟรม; leak ต้องวัดจริงหลายรอบ',
113:'ผู้ทดสอบกด Run/Stop จาก IDE 20 รอบ ไม่ให้ main.py คุม Launcher หรือสร้างผลผ่านเอง',
114:'return/exit 0 ปกติและ finally cleanup; ไม่ใช้ os._exit ที่ข้าม cleanup',
115:'NEGATIVE TEST: RuntimeError/nonzero เป็นผลที่ตั้งใจและต้องตรวจ traceback/menu ไม่ใช่ hardware PASS',
116:'NEGATIVE TEST: SyntaxError ตั้งใจไม่ผ่าน compile ต้องใช้โปรเจกต์แยกและไม่แก้โปรแกรมใช้งานจริง',
117:'แยก missing-package SKIP กับ intentional ImportError/nonzero ไม่ติดตั้ง dependency เอง',
118:'ใช้ I/O ที่มี finite-timeout จริง; ห้ามสมมติว่า I2C ทุก native call หยุดได้จาก Python loop',
119:'เลือกเฉพาะ subsystem ที่ผ่านก่อน จำกัด memory/log และไม่จงใจ OOM',
120:'DESKTOP SYNTHETIC ONLY ไม่ส่งขึ้นบอร์ดเพื่ออ้างว่าได้ทดสอบเวอร์ชันใหม่ที่ติดตั้ง',
121:'HOLD SPI จนมี connector/pinmux/driver/ownership ของ Plus จริง ไม่ยืม pinout MaixCAM2',
122:'HOLD Bluetooth จนพบ adapter และ driver จริง WiFi ไม่พิสูจน์ Bluetooth',
123:'เป็นขั้นตอน manual/discovery แยก OTG/RNDIS กับ console ไม่แก้ boot config หรือ USB role',
124:'HOLD USB host จนมี control transport สำรองและ power arrangement ที่ยืนยัน',
125:'HOLD motor/expansion จน protocol/power/driver/enable fixture ระบุครบ ห้ามขับโหลดตรงจาก GPIO',
126:'dashboard รวมต้องผ่าน constituent tests ก่อน ไม่ใช้ภาพ mock เป็น sensor จริง',
127:'HOLD wizard ขับครบทุก IO จนกรณีแยกผ่าน รวม IO7; ทำ subset แล้วห้ามสรุปครบทั้งบอร์ด',
128:'วิเคราะห์ผลที่ผู้ใช้ส่งหรืออนุญาตให้อ่านเท่านั้น ไม่สร้าง PASS ให้แถวที่ยัง NOT_RUN และไม่ต้องสร้างโครง display project'
};
const statusNames = {TEMPLATE_CHECKED:'มีตัวอย่างตรงงานและทดสอบ fixture แล้ว',DRAFT_REFERENCE:'มี API อ้างอิง; ต้องสร้างและตรวจโค้ดตามงาน',NEEDS_SETUP:'รออุปกรณ์/ค่าที่วัด/การตั้งค่าที่ระบุ',HOLD:'รอหลักฐานก่อนทดสอบฮาร์ดแวร์ตามขอบเขตนี้'};
const rows = [];
for(let index=0; index<128; index++) {
  const t=catalog.tests[index], n=index+1, id=`MP-${String(n).padStart(3,'0')}`, p=profiles.get(n);
  if(!t||t.id!==id||!p)throw Error(`Missing or shifted case ${id}`);
  t.authored_prompt ??= t.prompt; t.authored_title ??= t.title; t.authored_expected ??= t.expected;
  t.title = n===65 ? 'IMU raw หกแกนและตรวจข้อมูลเพิ่มเติม' : t.authored_title;
  let body=t.authored_prompt;
  if(n===15)body=body.replace('พร้อมกากบาทตามนิ้ว','พร้อม marker ตามนิ้ว');
  if(n===54)body='ให้ผู้ทดสอบ Run → Stop ระหว่าง PWM → ตรวจสัญญาณหยุด → Run ซ้ำ เมื่อ mapping/backend ยืนยันแล้ว ใช้ disable() ตาม API บันทึก channel busy และตรวจสัญญาณจริงหลัง Stop ไม่อ้างว่าคืน pinmux เดิมเมื่อไม่รู้ค่าเดิม หรือ finally ทำงานหลัง forced kill';
  if(n===63)body='ทดสอบรอบไม่มีอุปกรณ์และรอบต่ออุปกรณ์บนบัสภายนอกที่ยืนยัน ให้ผู้ทดสอบปิดไฟก่อนเปลี่ยนสาย แยก NACK/None/exception จาก native call ค้าง ไม่แต่ง timeout argument ให้ I2C.readfrom หากค้างให้ผู้ทดสอบ Stop ผ่าน supervisor และบันทึกเวลาจนออก ไม่รีบูตเอง';
  if(n===65)body='เปิด IMU ที่ยืนยันว่าเป็น QMI8658 อ่าน acceleration x/y/z และ gyro x/y/z หลังตรวจความยาวข้อมูล API dual mode รับประกันหกแกนเท่านั้น อุณหภูมิเป็น optional เมื่อ runtime มีข้อมูลและหน่วยที่ยืนยัน ห้าม index [6] โดยเดา';
  if(n===87)body='ตรวจ audio.Player และสร้าง raw PCM สั้นแบบ FMT_S16_LE ที่ amplitude ต่ำ ระบุ sample rate/channels เล่นรวมหนึ่งวินาทีเป็น chunk สั้นพร้อมเช็ก exit และปล่อย references เมื่อจบ ไม่พึ่ง volume() ซึ่ง reference ระบุ unsupported และไม่เรียก stop()/close() ที่ไม่มี';
  if(n===91)body='ทำโหมดบันทึกหรือเล่นเสียงเป็น chunk สั้นที่หยุดได้ ให้ผู้ทดสอบกด Stop กลางงาน ใช้ Recorder.finish() และปิดไฟล์ใน finally ส่วน Player ปล่อย references ตาม API ไม่แต่ง stop()/close() ตรวจรอบใหม่และไม่ทำ feedback ไมค์ออกลำโพง';
  if(n===102)body+=' การอ่านกลับเทียบ hash ตรวจความถูกต้องของ file I/O เท่านั้น ไม่พิสูจน์การเขียนลงสื่อจริงหรือการคงข้อมูลหลังไฟดับ และไม่สั่งล้าง kernel cache';
  t.prompt=body;
  t.expected=n===15?t.authored_expected.replace('กากบาท','marker'):t.authored_expected;
  const correction=fixes[n]??'ยืนยัน pin/function ของ connector นี้จาก MP-003 และ fixture จริง; ไม่มี get_pin_function หรือ GPIO.close ใน reference baseline';
  const template=n<=5?'capabilities':n===15?'touchscreen':n===21?'adc_buttons':null;
  const r={id,title:t.title,category:t.category,state:p.state,state_label:statusNames[p.state],contracts:p.contracts,
    prerequisites:p.requirements,depends_on:p.dependencies,template,correction,programming_guide:guide,
    task:t.prompt,expected_when_ready:t.expected,
    expected_failure:[115,116,117].includes(n),desktop_only:n===120,action:n===128?'analyze_results':p.state==='HOLD'||n===123?'discovery_or_manual_only':'create_reviewed_project',
    generated_code_verified:template!==null,physical_result:'NOT_RUN'};
  t.audit=r;
  rows.push(r);
}
const summary=Object.fromEntries(Object.keys(statusNames).map(s=>[s,rows.filter(r=>r.state===s).length]));
const registry={schema_version:1,date:'2026-10-27',board:'kidbright_uai_plus',target:'sg2002',maixpy_reference:'4.12.5',runtime_user_reported:'1.24.0',
  method:'Manual case-by-case contract review plus image AST symbol verification and local tests of the named templates. No live-model generation or device execution for this audit.',
  physical_results_unchanged:true,live_model_generation_tested:false,all_cases_have_generated_programs:false,
  guide,api_evidence:evidencePath,api_evidence_sha256:hash(evidencePath),contracts,summary,tests:rows};
registry.templates=Object.fromEntries(['capabilities','touchscreen','adc_buttons'].map(name=>{
 const main=`samples/kidbright-uai-plus/${name}.py`;
 return [name,{main,sha256:hash(main),...(name==='capabilities'?{helper_source:'src-tauri/src/uai_plus_probe.py',helper_sha256:hash('src-tauri/src/uai_plus_probe.py')}:{}),validation:'local fixture tests only; not a device pass'}];
}));
catalog.audit_date='2026-10-27';catalog.audit_registry=registryPath;catalog.audit_summary=summary;
const csvCell=v=>`"${String(v).replaceAll('"','""')}"`;
const auditCsv=[['test_id','title','audit_state','meaning','template','depends_on','prerequisites','correction','physical_result'],
 ...rows.map(r=>[r.id,r.title,r.state,r.state_label,r.template??'',r.depends_on.join('; '),r.prerequisites.join('; '),r.correction,'NOT_RUN'])]
 .map(r=>r.map(csvCell).join(',')).join('\r\n')+'\r\n';
const header=['# คลังพรอมท์ mAi Plus 128 ข้อ — ฉบับตรวจ 27 ตุลาคม 2026','',
 '**ใช้ฉบับนี้แทนชุดก่อนตรวจ** ตรวจครบ 128 รหัสแล้ว แต่ไม่ได้รันโปรแกรมที่โมเดลสร้างครบ 128 โปรแกรมหรือทดสอบอุปกรณ์จริงครบทุกข้อ ดู [ผลตรวจรายข้อ](KIDBRIGHT_MAI_PLUS_PROMPT_AUDIT_2026-10-27.md) ก่อนเลือกเทส', '',
 ...Object.entries(summary).map(([s,count])=>`- **${s} ${count} ข้อ:** ${statusNames[s]}`),'',
 'มีตัวอย่างตรงงานสำหรับ MP-001–005, MP-015 และ MP-021 เท่านั้น ข้อ DRAFT_REFERENCE ยังต้องสร้างโค้ดเฉพาะงานและตรวจทาน ไม่ใช่ไฟล์พร้อมรัน ข้อ NEEDS_SETUP ต้องระบุ fixture/config/bands ก่อน ข้อ HOLD ทำ discovery/แผน manual ได้ แต่ยังไม่ขับฮาร์ดแวร์', '',
 '## วิธีใช้และพรอมท์ตั้งต้น','',
 'เลือก Plus ส่งกติกาด้านล่างหนึ่งครั้ง แล้วคัดลอกเทสทีละข้อ ตรวจ Keep/Undo ก่อน Run ผู้ใช้เป็นผู้กด Connect/Run/Stop เอง ห้ามรันทั้ง 128 ข้ออัตโนมัติ ตาราง [ผลฮาร์ดแวร์เดิม](KIDBRIGHT_MAI_PLUS_TEST_RESULTS_2026-10-26.csv) ไม่ถูกรีเซ็ตหรือใช้ผล audit แทนผลจริง','',
 '```text',
 'ใช้ KidBright mAi Plus SG2002, MaixPy 4.12.5 / Runtime 1.24.0 ตามที่บอร์ดรายงาน',
 'ก่อนสร้างเทส MP-xxx ให้เรียก uai_plus_test_reference ด้วยรหัสนั้น และอ่าน kidbright_uai_plus_test_contracts.md สำหรับ API ที่เกี่ยวข้อง ถ้าแอปรุ่นนี้ไม่มีเครื่องมืออ้างอิง ให้รายงานว่าตัวแอปยังไม่รวมชุด audit และใช้โปรเจกต์ตัวอย่างที่ตรวจไว้ ห้าม fallback เป็นวงกลมแล้วเรียกว่าเทสครบ',
 'TEMPLATE_CHECKED ให้เลือก python_example ตรงตาม reference; ข้ออื่นที่สร้างโค้ดเฉพาะงานให้ส่ง main_python เป็นโปรแกรมที่ขอจริงในเครื่องมือสร้างโปรเจกต์ ตรวจ syntax/API และเสนอ write_file เพิ่มให้ครบก่อนถาม Keep โดยเสนอไฟล์ได้ระหว่าง review ห้ามสร้าง seed MP-021 แล้วให้ Keep ก่อนเริ่มทำ MP-022; NEEDS_SETUP ถามเฉพาะ fixture/config/ผลวัดที่ยังขาด; HOLD ให้ทำเฉพาะ discovery หรือแผน manual ห้ามเดา wiring/driver หรือเริ่มขับโหลด',
 'ใช้ main/main.py, project_type=kidbright_uai_plus และโฟลเดอร์ใหม่ที่ไม่เขียนทับของเดิม ให้ IDE ดูแล Launcher ไม่ใช้ killall/pkill หรือแก้ boot script',
 'เช็ก API ตามภาพอ้างอิงและ runtime จริง ไม่แต่งชื่อเมธอด ไม่มี API เป็น SKIP; driver/read ล้มเป็น ERROR พร้อมหลักฐาน ใช้ exit-aware loop, finite timeout ที่มีจริง และ cleanup ตาม API ของอุปกรณ์ ไม่สมมติว่าทุก object มี close()',
 'ก่อน hardware Run ต้องตรวจโค้ดที่สร้างจริง อุปกรณ์ การต่อสาย แรงดัน พอร์ตที่มีเจ้าของ และ dependency ของเทส ไม่ถือว่า syntax ผ่าน อัปโหลดสำเร็จ หรือ API มีอยู่เป็น hardware PASS',
 'ไม่ติดตั้งแพ็กเกจ โหลด kernel module เปลี่ยน USB role/เครือข่าย ตั้ง RTC หรือขับ motor เอง เก็บ buffer/log/file ให้มีขอบเขต รอผู้ทดสอบยืนยันภาพ เสียง สัญญาณ และเมนูกลับ',
 '```','',
 '## การต่อ IO','',
 'ซอร์สระบุ IO2=A23, IO3=A27, IO4=A25, IO5=A22, IO6=A24, IO7=P24, IO8=A15 แต่ค่าเหล่านี้ยังไม่ใช่ PCB/routing proof ปิดไฟก่อนเปลี่ยนสาย ตรวจ S/V/G และระดับไฟ ใช้ LED พร้อมตัวต้านทานหรือ loopback ที่มี output เพียงด้านเดียวและ series resistor ห้ามต่อ 5V เข้าขาที่รองรับ 3.3V ห้ามขับ motor โดยตรง', '',
 'IO7/P24 และ PWM backend ยังเป็น HOLD; ห้ามเปลี่ยน P24 เป็น A24 เพื่อให้รันผ่าน UART0/A16/A17 มีเจ้าของจาก log เดิม ไม่ใช้เทส UART ของผู้ใช้ การทดสอบ external I2C ต้องยืนยัน bus/backend และ protocol ของโมดูลก่อน ไม่มีสิทธิ์ scan บัสกล้อง/IMU เพื่อเดาอุปกรณ์', '',
 '## พรอมท์รายข้อ',''];
let lastCategory='';
for(const t of catalog.tests){const r=t.audit;if(t.category!==lastCategory){header.push(`### ${t.category}`,'');lastCategory=t.category;}
 const request=r.action==='analyze_results'?'วิเคราะห์ผล':r.desktop_only?'จัดทำชุดทดสอบ SYNTHETIC บนคอมพิวเตอร์ ห้าม upload ขึ้นบอร์ด':r.action==='discovery_or_manual_only'?`จัดทำรายงาน discovery/แผน manual สำหรับ ${t.project} ยังไม่ขับฮาร์ดแวร์ หากต้องสร้างโปรเจกต์ discovery ให้ส่ง test_id="${t.id}" และ python_example="capabilities"`:`สร้างโปรเจกต์ ${t.project} โดยส่ง test_id="${t.id}" ในเครื่องมือสร้างโปรเจกต์`;
 header.push(`#### ${t.id} — ${t.title}`,'',`สถานะก่อนเทส: **${r.state}** — ${r.state_label}`,`อุปกรณ์: ${t.equipment}`,
 `ต้องมีก่อน: ${r.prerequisites.join(' / ')||'ตรวจ runtime และโค้ดเฉพาะงานก่อน hardware Run'}${r.depends_on.length?'; ผลจาก '+r.depends_on.join(', '):''}`,'','```text',
 `${request} สำหรับ mAi Plus ตามกติกาฉบับ audit เรียก uai_plus_test_reference(test_id="${t.id}") ก่อน ${r.state==='HOLD'?'ขอบเขตเป้าหมายหลังแก้ blocker (ยังไม่ทำตอนนี้)':'ทำเทส'} ${t.id}: ${t.prompt}`,
 `ข้อแก้ไขที่ต้องใช้: ${r.correction}`,
 `สถานะ ${r.state}: ${r.state_label}${r.template?' เลือก python_example="'+r.template+'"':''}`,'```','',
 ...(r.state==='HOLD'?['เกณฑ์ตรวจขณะ HOLD: ได้รายงานสิ่งที่ขาด/แผน manual โดยยังไม่ขับโหลด ไม่ถือเป็นผลผ่านฮาร์ดแวร์',`เกณฑ์เป้าหมายหลังแก้ blocker: ${t.expected}`]:[`เกณฑ์ตรวจ: ${t.expected}`]),'');}
header.push('## ขอบเขตหลักฐาน','',
 'API signatures มาจาก image 4.12.5 และเอกสาร primary source ที่อ้างใน guide; การพบ signature ไม่ใช่ผลทดลอง driver หรือสายจริง ค่าจาก simulator/fixture ไม่ถูกย้ายเป็น PASS ของบอร์ด และรุ่นจำลองไม่ใช่การทดสอบรุ่นใหม่ที่ติดตั้งจริง', '',
 'ผลผู้ใช้ยืนยันก่อนหน้านี้มีเพียงวงจร Connect/Run/Stop กลับเมนูตาม [device-session.json](research/uai-plus-runtime-2026-10-26/device-session.json) ไม่ใช่ผลผ่าน 128 ข้อ', '');
const report=['# ผลตรวจพรอมท์ mAi Plus ทั้ง 128 ข้อ — 27 ตุลาคม 2026','',
 'ตรวจทุก ID เทียบ API ของ image, เอกสารที่บันเดิล, ตัวสร้างโปรเจกต์และข้อกำหนด fixture แล้ว ไม่ได้เรียกโมเดลสดสร้าง/รันครบ 128 โปรแกรม ผล audit แยกจากผลฮาร์ดแวร์ซึ่งยังเป็น NOT_RUN','',
 '| สถานะ | จำนวน | ความหมาย |','|---|---:|---|',...Object.entries(summary).map(([s,n])=>`| ${s} | ${n} | ${statusNames[s]} |`),'',
 '## ข้อผิดพลาด/ช่องว่างที่แก้','',
 '- MP-015/021 มี API และเทมเพลตจริง; MP-001–005 เพิ่ม read-only preflight พร้อม metadata helper',
 '- MP-020 ไม่มี multi-touch API ใน reference นี้; ไม่ใช้การแตะสองจุดเป็นเกณฑ์ผ่าน',
 '- MP-065 แก้สมมติฐาน IMU เจ็ดค่าเป็นหกแกนและ optional data; ไม่มีการ index อุณหภูมิที่ไม่ได้ยืนยัน',
 '- ใช้ bm8563 แทนโมดูล rtc ที่ไม่มี; battery retention ยังแยก NTP ไม่ได้จึง HOLD',
 '- Player.stop/close ไม่มี และ volume ระบุ unsupported; แก้เป็น low-amplitude PCM/short chunks และ Recorder.finish',
 '- get_pin_function ไม่มีใน 4.12.5; ไม่อ้างว่ารู้/คืน pinmux เดิมจาก getter ที่ไม่มี',
 '- IO7/P24, servo helper/backend, all-seven-pin output, SPI/Bluetooth/USB-host/expansion รอหลักฐานก่อนขับ',
 '- แยก HTTP stdlib ออกจาก maix.http.JpegStreamer และ package prerequisites ของ MQTT/WebSocket',
 '- ระบุ negative tests ที่ตั้งใจล้ม, calibration prerequisites และไม่ให้ finally เป็นข้อพิสูจน์ safety เมื่อถูก kill',
 '', '## รายข้อ','', '| ID | เทส | สถานะ | ตัวอย่าง | สิ่งที่แก้/ต้องยืนยัน |','|---|---|---|---|---|',
 ...rows.map(r=>`| ${r.id} | ${r.title} | ${r.state} | ${r.template??'ต้องสร้างเฉพาะงาน'} | ${r.correction.replaceAll('|','/')} |`),'',
 '## ตรวจซ้ำ','',
 'ผลตรวจซอฟต์แวร์และขอบเขตที่ยังไม่ได้ทดสอบอยู่ใน [validation record](research/uai-plus-runtime-2026-10-26/prompt-audit-validation-2026-10-27.json) ใช้ [โปรเจกต์ preflight](../src-tauri/target/recovery-projects/mai_plus_preflight) เริ่มสำรวจแบบไม่ขับพอร์ตก่อนเลือกชุดต่อไป','',
 '```powershell','node scripts/audit-uai-plus-prompts.mjs --check','python -m unittest discover -s scripts/tests -p "test_uai_plus*.py"','cargo test --manifest-path src-tauri/Cargo.toml --lib --locked --offline','```','',
 'สถานะ TEMPLATE_CHECKED อิงชุดทดสอบ fixture ของเทมเพลต การสร้างโค้ดใหม่/แก้รายละเอียดนอกเทมเพลตต้องตรวจแยก ไม่รับรองการสร้างจากทุกโมเดลหรือ firmware ในอนาคต', ''];
const outputs=new Map([[registryPath,JSON.stringify(registry,null,2)+'\n'],[catalogPath,JSON.stringify(catalog,null,2)+'\n'],
 [catalog.prompt_document,header.join('\n')],[reportPath,report.join('\n')],[auditCsvPath,'\ufeff'+auditCsv]]);
for(const [path,contents] of outputs){if(check){if(!existsSync(path)||read(path)!==contents)throw Error('Stale audit artifact: '+path);}else writeFileSync(path,contents);}
console.log(JSON.stringify({total:rows.length,summary,api_modules:Object.keys(evidence.modules).length,live_model_generation_tested:false,hardware_results_reset:false}));
