import { useEffect, useId, useRef, useState } from "react";
import BoardPreparationGuide, { type BoardFamily } from "./BoardPreparationGuide";
import "./LearningWorkspace.css";

type Family = BoardFamily;
type Task = "image" | "object" | "voice";
type ClassicLesson = "sound" | "temperature" | "display";
type Sample = { id: string; name: string; url: string; group: string };
type Collection = { groups: { id: string; name: string }[]; samples: Sample[] };

const TH = {
  workspace: "พื้นที่เรียนรู้", subtitle: "จากไอเดียเล็ก ๆ สู่สิ่งประดิษฐ์ของคุณ", language: "ภาษาพื้นที่เรียนรู้",
  boardChoice: "บอร์ดสำหรับบทเรียน", design: "ตัวอย่างพื้นที่ทำงาน", hero: "ให้ไอเดียของคุณ เรียนรู้ได้ด้วย AI",
  intro: "เริ่มจากสิ่งที่อยากให้บอร์ดรู้จัก แล้วค่อย ๆ เปลี่ยนข้อมูลให้เป็นสิ่งประดิษฐ์", guide: "คู่มือเตรียมบอร์ด",
  workflow: "ขั้นตอนการเรียนรู้", collect: "เก็บข้อมูล", collectHint: "ถ่ายภาพหรือบันทึกเสียง", label: "ติดป้ายกำกับ", labelHint: "จัดข้อมูลให้เป็นกลุ่ม",
  train: "ฝึกโมเดล", trainHint: "ให้ AI เรียนรู้จากตัวอย่าง", code: "เขียนและทดสอบ", codeHint: "นำไอเดียไปใช้บนบอร์ด",
  image: "จำแนกภาพ", object: "ตรวจจับวัตถุ", voice: "จำแนกเสียง", task: "อยากให้ AI เรียนรู้อะไร?",
  library: "คลังข้อมูลของฉัน", libraryHint: "ภาพที่หลากหลายช่วยให้โมเดลได้เรียนรู้จากตัวอย่างที่ต่างกัน", voiceHint: "ลองเก็บเสียงหลายคน หลายระยะ และมีเสียงพื้นหลังที่ต่างกัน",
  groups: "กลุ่มข้อมูล", group: "กลุ่ม", addGroup: "เพิ่มกลุ่ม", groupName: "ชื่อกลุ่ม", samples: "ตัวอย่าง", files: "ไฟล์",
  empty: "เริ่มจากตัวอย่างแรกของคุณ", emptyHint: "ลากไฟล์ภาพมาวางที่นี่ หรือเลือกภาพจากคอมพิวเตอร์", emptyVoice: "เพิ่มเสียงตัวอย่างแรกของคุณ", emptyVoiceHint: "เลือกไฟล์ WAV, MP3, OGG หรือ M4A จากคอมพิวเตอร์",
  import: "เลือกไฟล์จากคอมพิวเตอร์", camera: "ใช้กล้องจากบอร์ด", microphone: "บันทึกเสียงจากบอร์ด", notConnected: "ยังไม่เชื่อมต่อบอร์ด",
  local: "ข้อมูลทดลองอยู่ในหน้านี้ ยังไม่บันทึกลงโปรเจกต์ และจะหายเมื่อปิดหรือโหลดแอปใหม่", remove: "นำตัวอย่างออก", assignment: "กลุ่มของตัวอย่าง",
  annotation: "จัดกลุ่มให้ตรงกับสิ่งที่อยู่ในตัวอย่าง", annotationHint: "ตรวจชื่อกลุ่มและย้ายตัวอย่างที่อยู่ผิดกลุ่มได้ก่อนเริ่มฝึก",
  objectNote: "สำหรับการตรวจจับวัตถุ ต้องมีกรอบระบุตำแหน่งด้วย หน้านี้จัดกลุ่มไฟล์ได้ ส่วนเครื่องมือวาดกรอบยังไม่เปิดใช้งาน",
  backToData: "กลับไปเพิ่มข้อมูล", training: "เตรียมข้อมูลก่อนให้ AI เรียนรู้", trainingHint: "ตรวจว่าทุกกลุ่มมีข้อมูล และตัวอย่างครอบคลุมสิ่งที่จะเจอในการใช้งานจริง",
  format: "โมเดลสำหรับบอร์ดนี้", readiness: "ภาพรวมชุดข้อมูล", missing: "ยังไม่มีตัวอย่าง", startTraining: "เริ่มฝึกโมเดล", later: "การฝึกโมเดลและการส่งงานไป μAI/μAI Plus ยังไม่เปิดใช้งานในรุ่นนี้",
  codeTitle: "จากตัวอย่าง สู่โปรแกรมของคุณ", codeIntro: "เริ่มด้วยตัวอย่างเปิดกล้องนี้ แล้วค่อยเพิ่มโมเดลที่ฝึกไว้", codeNote: "โค้ดอ้างอิงสำหรับบอร์ดที่เลือก ยังไม่ได้ส่งไปรันหรือผูกกับโปรเจกต์ของคุณ",
  copyCode: "คัดลอกตัวอย่างโค้ด", openEditor: "เปิดพื้นที่เขียนโปรแกรม", run: "ส่งและรันบนบอร์ด", copy: "คัดลอกโจทย์", copied: "คัดลอกแล้ว", copyFailed: "คัดลอกไม่สำเร็จ โปรดเลือกข้อความแล้วคัดลอกด้วยตัวเอง",
  next: "ขั้นถัดไป", previous: "ย้อนกลับ", device: "บอร์ดในบทเรียน", illustration: "ภาพประกอบบอร์ด", prepare: "ก่อนเริ่มใช้งานจริง", prepareHint: "ตรวจตามคู่มือด้วยตัวเอง การเลือกบอร์ดในบทเรียนยังไม่ใช่การเชื่อมต่ออุปกรณ์",
  usb: "ใช้สาย USB ที่ส่งข้อมูลได้ และเสียบช่อง USB-OTG", power: "ตรวจแหล่งจ่ายไฟและรอให้บอร์ดเริ่มทำงาน", runtime: "ตรวจ OS และ Runtime ตามคู่มือของรุ่นที่ใช้", network: "ตรวจ USB network และบริการเชื่อมต่อของบอร์ด", cameraCheck: "ทดสอบกล้องและเสียงก่อนเริ่มบทเรียน",
  classicPrepare: "เปิดพื้นที่เขียนโปรแกรม เลือกพอร์ต และตรวจ ESP-IDF ก่อน Build & Flash", promptTitle: "เล่าไอเดียให้ Vibe Coder ช่วยคิด", promptHint: "เริ่มด้วยสิ่งที่อยากทำ ระบุสิ่งที่บอร์ดรับเข้าและผลลัพธ์ที่ต้องการ", prompt: "โจทย์ของฉัน", coder: "เปิด Vibe Coder",
  promptImage: "อยากทำถังขยะอัจฉริยะที่ช่วยแยกขวดพลาสติกกับกระป๋อง ควรเก็บข้อมูลและออกแบบโปรแกรมอย่างไร?", promptObject: "อยากให้กล้องช่วยนับจำนวนวัตถุในภาพ ควรเก็บภาพและติดป้ายตำแหน่งอย่างไร?", promptVoice: "อยากให้บอร์ดรู้จักคำสั่งเสียงของฉัน ควรเก็บเสียงตัวอย่างและทดสอบอย่างไร?",
  classicTitle: "เริ่มประดิษฐ์ด้วย KidBright32", classicIntro: "เขียนโปรแกรมด้วยภาษาธรรมชาติ แล้วตรวจโค้ดก่อน Build & Flash", classicLesson1: "ปุ่มกดและเสียง", classicLesson2: "อ่านค่าอุณหภูมิ", classicLesson3: "แสดงผลบน LED", classicHint: "ไฟล์ที่ผู้เรียนแก้อยู่ใน main ส่วนไฟล์สนับสนุนให้แอปจัดการ",
  chooseExample: "เลือกตัวอย่างพรอมท์", editPrompt: "ปรับโจทย์ของคุณ", usePrompt: "นำไปใช้ใน Vibe Coder", resetPrompt: "คืนค่าตัวอย่าง", promptReview: "ปรับชื่อโปรเจกต์และระบุรุ่นบอร์ดก่อนส่ง คุณจะได้ตรวจโค้ดก่อนนำไปใช้",
  rejected: "บางไฟล์ไม่ตรงกับชนิดข้อมูล หรือมีขนาดเกิน 20 MB", fileHint: "ภาพ PNG / JPG / WebP หรือเสียง WAV / MP3 / OGG / M4A · ไม่เกิน 20 MB ต่อไฟล์",
};
const EN: Record<keyof typeof TH, string> = {
  workspace: "Learning workspace", subtitle: "Small ideas. Your next invention.", language: "Learning workspace language",
  boardChoice: "Lesson board", design: "Workspace preview", hero: "Give your ideas the power to learn",
  intro: "Start with something you want your board to recognize. Turn examples into an invention, one step at a time.", guide: "Board preparation guide",
  workflow: "Learning steps", collect: "Collect", collectHint: "Gather images or sounds", label: "Label", labelHint: "Organize your examples",
  train: "Train", trainHint: "Learn from your dataset", code: "Code & test", codeHint: "Bring your idea to the board",
  image: "Image classification", object: "Object detection", voice: "Voice classification", task: "What should your AI learn?",
  library: "My dataset", libraryHint: "Use varied images so the model can learn from different examples.", voiceHint: "Try different speakers, distances, and background sounds.",
  groups: "Classes", group: "Class", addGroup: "Add class", groupName: "Class name", samples: "examples", files: "files",
  empty: "Every idea starts with an example", emptyHint: "Drop images here, or choose files from your computer.", emptyVoice: "Add your first sound example", emptyVoiceHint: "Choose WAV, MP3, OGG, or M4A files from your computer.",
  import: "Choose local files", camera: "Use board camera", microphone: "Record from board", notConnected: "Board not connected",
  local: "Practice data stays on this page. It is not saved to a project and is cleared when the app closes or reloads.", remove: "Remove example", assignment: "Example class",
  annotation: "Give every example the right class", annotationHint: "Check class names and move examples into the correct group before training.",
  objectNote: "Object detection also needs bounding boxes. This page can organize files; the bounding-box tool is not available yet.",
  backToData: "Back to dataset", training: "Get your dataset ready to learn", trainingHint: "Add examples to every class and cover the situations your invention will encounter.",
  format: "Model for this board", readiness: "Dataset overview", missing: "No examples yet", startTraining: "Start training", later: "Training and deployment to μAI/μAI Plus are not available in this release.",
  codeTitle: "From examples to your program", codeIntro: "Start with this camera example, then add a trained model.", codeNote: "Reference code for the lesson board. It has not been deployed or added to your project.",
  copyCode: "Copy example code", openEditor: "Open code workspace", run: "Upload & run", copy: "Copy idea", copied: "Copied", copyFailed: "Could not copy. Select the text and copy it manually.",
  next: "Next step", previous: "Back", device: "Your lesson board", illustration: "Board illustration", prepare: "Before using your board", prepareHint: "Follow the manual to check your board. Selecting a lesson board does not connect a device.",
  usb: "Use a data-capable USB cable and the USB-OTG port", power: "Check the power source and wait for the board to boot", runtime: "Check the OS and Runtime for your board revision", network: "Check USB networking and the board connection service", cameraCheck: "Test the camera and audio before the lesson",
  classicPrepare: "Open the code workspace, select a port, and check ESP-IDF before Build & Flash.", promptTitle: "Think it through with Vibe Coder", promptHint: "Describe your idea, its inputs, and the result you want.", prompt: "My idea", coder: "Open Vibe Coder",
  promptImage: "I want to build a smart recycling bin that distinguishes plastic bottles from cans. How should I collect examples and design the program?", promptObject: "I want a camera to count objects in a picture. How should I collect images and label their positions?", promptVoice: "I want my board to recognize my voice commands. How should I collect examples and test it?",
  classicTitle: "Start making with KidBright32", classicIntro: "Describe your program, review the code, then Build & Flash.", classicLesson1: "Buttons & sound", classicLesson2: "Read temperature", classicLesson3: "LED display", classicHint: "Learner code stays in main. The app manages supporting files.",
  chooseExample: "Choose a prompt example", editPrompt: "Make the idea yours", usePrompt: "Use in Vibe Coder", resetPrompt: "Reset example", promptReview: "Adjust the project name and specify your board revision before sending. Review the proposed code before applying it.",
  rejected: "Some files have an unsupported format or exceed 20 MB.", fileHint: "PNG / JPG / WebP images or WAV / MP3 / OGG / M4A audio · up to 20 MB per file",
};

function Icon({ name, size = 20 }: { name: string; size?: number }) {
  const paths: Record<string, string> = {
    camera: "M4 7h4l2-3h4l2 3h4v13H4z M16 13a4 4 0 1 1-8 0 4 4 0 0 1 8 0",
    label: "M3 3h8l10 10-8 8L3 11z M7 7h.01", train: "m12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5z",
    code: "m8 7-5 5 5 5 M16 7l5 5-5 5 M14 4l-4 16", chip: "M6 6h12v12H6z M9 9h6v6H9z M9 2v4 M15 2v4 M9 18v4 M15 18v4 M2 9h4 M2 15h4 M18 9h4 M18 15h4",
    plus: "M12 5v14 M5 12h14", arrow: "M4 12h16 M14 6l6 6-6 6", upload: "M12 16V3 M7 8l5-5 5 5 M4 16v5h16v-5",
    mic: "M9 5a3 3 0 0 1 6 0v7a3 3 0 0 1-6 0z M5 10v2a7 7 0 0 0 14 0v-2 M12 19v3 M8 22h8",
    object: "M8 3H3v5 M16 3h5v5 M3 16v5h5 M21 16v5h-5 M8 8h8v8H8z", book: "M12 5c-4-3-8-2-9-1v15c3-2 6-2 9 0 3-2 6-2 9 0V4c-3-2-6-2-9 1z M12 5v14",
    check: "m5 12 4 4L19 6", close: "m6 6 12 12 M6 18 18 6", copy: "M9 9h12v12H9z M15 9V3H3v12h6",
  };
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name] || paths.chip} /></svg>;
}

function BoardIllustration({ family, label }: { family: Family; label: string }) {
  const id = useId();
  return <svg viewBox="0 0 320 210" className="lw-board-art" role="img" aria-label={label}>
    <defs><linearGradient id={id} x2="1" y2="1"><stop stopColor="#273e42"/><stop offset="1" stopColor="#0d1a23"/></linearGradient></defs>
    <ellipse cx="160" cy="193" rx="115" ry="10" fill="currentColor" opacity=".08"/>
    <g transform="translate(26 22) rotate(-5 134 79)">
      <rect x="0" y="0" width="268" height="158" rx="19" fill={family === "kidbright32" ? "#20835e" : family === "mai" ? "#ab3c4f" : "#242e36"}/>
      {[[13, 13], [255, 13], [13, 145], [255, 145]].map(([cx, cy], i) => <circle key={i} cx={cx} cy={cy} r="5" fill="#c9ac75" stroke="#edd8b2" strokeWidth="2"/>)}
      <rect x="230" y="51" width="43" height="54" rx="4" fill="#eee7db"/>
      {[58, 71, 84, 97].map(y => <rect key={y} x="243" y={y} width="17" height="4" rx="1" fill="#838779"/>)}
      {[31, 59, 87, 115].map(y => <g key={y}><rect x="-7" y={y} width="27" height="18" rx="3" fill="#eee7db"/><path d={`M-2 ${y + 6}h15 M-2 ${y + 12}h15`} stroke="#919986" strokeWidth="2"/></g>)}
      <rect x="65" y="-4" width="29" height="15" rx="3" fill="#ccd2cf"/><rect x="105" y="-4" width="29" height="15" rx="3" fill="#ccd2cf"/>
      <rect x="39" y="23" width="184" height="105" rx="9" fill="#10161d" stroke="#49545b" strokeWidth="2"/>
      <rect x="47" y="31" width="168" height="89" rx="4" fill={`url(#${id})`}/>
      {family === "kidbright32" ? Array.from({ length: 40 }, (_, i) => <circle key={i} cx={69 + (i % 10) * 13} cy={46 + Math.floor(i / 10) * 18} r="3.5" fill={i % 3 ? "#ec8e78" : "#465f58"}/>) : <>
        <circle cx="130" cy="60" r="14" fill="#b6e5d9" opacity=".15"/><path d="m124 60 5 5 9-11" stroke="#a5e3cd" strokeWidth="3" fill="none"/>
        <text x="131" y="91" fill="#dbf2ea" fontSize="14" fontFamily="sans-serif" textAnchor="middle" letterSpacing="2">HELLO, IDEA.</text>
        <path d="M111 103h40" stroke="#91b9aa" strokeWidth="2" strokeLinecap="round"/>
      </>}
      <circle cx="53" cy="142" r="7" fill="#e9be49"/><circle cx="207" cy="142" r="7" fill="#e9be49"/>
      <text x="131" y="146" textAnchor="middle" fontSize="11" fill="#dce4e3" fontFamily="sans-serif">{family === "plus" ? "KidBright μAI Plus" : family === "mai" ? "KidBright μAI" : "KidBright32"}</text>
    </g>
  </svg>;
}

const initialCollection = (task: Task): Collection => ({ groups: [{ id: `${task}-1`, name: "" }, { id: `${task}-2`, name: "" }], samples: [] });
const PLUS_EXAMPLE = "from maix import camera, display\n\ndisp = display.Display()\ncam = camera.Camera(disp.width(), disp.height())\n\ntry:\n    while True:\n        disp.show(cam.read())\nfinally:\n    cam.close()\n";
const MAI_EXAMPLE = "from maix import camera, display\n\nwhile True:\n    frame = camera.capture()\n    display.show(frame)\n";

export default function LearningWorkspace({ onOpenEditor, onOpenCoder }: { onOpenEditor: () => void; onOpenCoder: (prompt?: string) => void }) {
  const [locale, setLocale] = useState<"th" | "en">(() => localStorage.getItem("vibe-learning-language") === "en" ? "en" : "th");
  const t = locale === "th" ? TH : EN;
  const [family, setFamily] = useState<Family>("plus");
  const [task, setTask] = useState<Task>("image");
  const [stage, setStage] = useState(0);
  const [showGuide, setShowGuide] = useState(false);
  const [classicLesson, setClassicLesson] = useState<ClassicLesson>("sound");
  const [classicDrafts, setClassicDrafts] = useState<Partial<Record<ClassicLesson, string>>>({});
  const [collections, setCollections] = useState<Record<Task, Collection>>(() => ({ image: initialCollection("image"), object: initialCollection("object"), voice: initialCollection("voice") }));
  const [selectedGroup, setSelectedGroup] = useState("");
  const [ideas, setIdeas] = useState<Partial<Record<Task, string>>>({});
  const [notice, setNotice] = useState("");
  const fileInput = useRef<HTMLInputElement>(null);
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);
  const sequence = useRef(1);
  const urls = useRef(new Set<string>());
  const prefix = useId();
  useEffect(() => { const owned = urls.current; return () => { owned.forEach(url => URL.revokeObjectURL(url)); }; }, []);
  const collection = collections[task];
  const activeGroup = collection.groups.some(group => group.id === selectedGroup) ? selectedGroup : collection.groups[0].id;
  const groupName = (id: string) => { const i = collection.groups.findIndex(group => group.id === id); return collection.groups[i]?.name.trim() || `${t.group} ${i + 1}`; };
  const boardName = family === "plus" ? "KidBright μAI Plus" : family === "mai" ? "KidBright μAI" : "KidBright32";
  const modelFormat = family === "plus" ? ".cvimodel + .mud" : task === "voice" ? ".npz · NumPy FP32" : ".bin + .param · NCNN";
  const steps = [{ name: t.collect, hint: t.collectHint, icon: "camera" }, { name: t.label, hint: t.labelHint, icon: "label" }, { name: t.train, hint: t.trainHint, icon: "train" }, { name: t.code, hint: t.codeHint, icon: "code" }];
  const example = family === "mai" ? MAI_EXAMPLE : PLUS_EXAMPLE;
  const idea = ideas[task] ?? (task === "voice" ? t.promptVoice : task === "object" ? t.promptObject : t.promptImage);
  const classicExamples = locale === "th" ? [
    { id: "sound", title: t.classicLesson1, hint: "กด S1 หนึ่งครั้ง เล่นเสียงหนึ่งครั้ง", prompt: "สร้างโปรเจกต์ใหม่ชื่อ s1_sound สำหรับ KidBright32 [ระบุรุ่นบอร์ด] ให้กด S1 แล้วเล่นเสียง A4 ที่ 440 Hz นาน 500 ms และรอปล่อยปุ่มก่อนรับครั้งถัดไป ตรวจรุ่นบอร์ด ขา และวิธีขับเสียงจากข้อมูลอ้างอิงก่อนเขียนโค้ด ให้ฉันตรวจและเลือก Keep หรือ Undo ก่อนนำโค้ดไปใช้ ยังไม่ต้อง Build หรือ Flash" },
    { id: "temperature", title: t.classicLesson2, hint: "อ่านค่าเป็นระยะ แล้วดูผลผ่าน Serial", prompt: "สร้างโปรเจกต์ใหม่ชื่อ temperature สำหรับ KidBright32 [ระบุรุ่นบอร์ด] อ่านอุณหภูมิทุก 1 วินาทีและแสดงค่าองศาเซลเซียสผ่าน Serial ตรวจสอบไอซี เซนเซอร์ และบัส I2C ของรุ่นนี้จากข้อมูลอ้างอิงก่อน ห้ามเดาขาหรือ API ให้ฉันตรวจและเลือก Keep หรือ Undo ก่อนนำโค้ดไปใช้ ยังไม่ต้อง Build หรือ Flash" },
    { id: "display", title: t.classicLesson3, hint: "ให้ข้อความ HELLO เลื่อนบนจอ LED", prompt: "สร้างโปรเจกต์ใหม่ชื่อ led_message สำหรับ KidBright32 [ระบุรุ่นบอร์ด] ให้ข้อความ HELLO เลื่อนจากขวาไปซ้ายบนจอ LED 16×8 ตรวจสอบไดรเวอร์ การต่อขา และตัวอย่างของรุ่นนี้จากข้อมูลอ้างอิงก่อนเขียนโค้ด ให้ฉันตรวจและเลือก Keep หรือ Undo ก่อนนำโค้ดไปใช้ ยังไม่ต้อง Build หรือ Flash" },
  ] : [
    { id: "sound", title: t.classicLesson1, hint: "One S1 press, one short tone", prompt: "Create a new project named s1_sound for KidBright32 [specify board revision]. On S1 press, play A4 at 440 Hz for 500 ms, then wait for release before accepting another press. Verify the board, pins and buzzer driver against references before writing code. Propose changes for Keep or Undo review. Do not build or flash yet." },
    { id: "temperature", title: t.classicLesson2, hint: "Read periodically and view the result over Serial", prompt: "Create a new project named temperature for KidBright32 [specify board revision]. Read temperature every second and print Celsius over Serial. Verify the sensor IC and I2C bus against board references; do not guess pins or APIs. Propose changes for Keep or Undo review. Do not build or flash yet." },
    { id: "display", title: t.classicLesson3, hint: "Scroll HELLO across the LED matrix", prompt: "Create a new project named led_message for KidBright32 [specify board revision]. Scroll HELLO from right to left across the 16×8 LED matrix. Check this board's driver, wiring and reference examples before writing code. Propose changes for Keep or Undo review. Do not build or flash yet." },
  ];
  const classicExample = classicExamples.find(item => item.id === classicLesson)!;
  const classicPrompt = classicDrafts[classicLesson] ?? classicExample.prompt;
  const updateCollection = (update: (value: Collection) => Collection) => setCollections(current => ({ ...current, [task]: update(current[task]) }));

  const importFiles = (files: File[]) => {
    const allowed = task === "voice" ? /\.(wav|mp3|ogg|m4a)$/i : /\.(png|jpe?g|webp)$/i;
    const accepted = files.filter(file => allowed.test(file.name) && file.size <= 20 * 1024 * 1024);
    const samples = accepted.map(file => { const url = URL.createObjectURL(file); urls.current.add(url); return { id: `sample-${sequence.current++}`, name: file.name, url, group: activeGroup }; });
    updateCollection(current => ({ ...current, samples: [...current.samples, ...samples] }));
    setNotice(accepted.length < files.length ? t.rejected : "");
  };
  const removeSample = (sample: Sample) => {
    URL.revokeObjectURL(sample.url); urls.current.delete(sample.url);
    updateCollection(current => ({ ...current, samples: current.samples.filter(item => item.id !== sample.id) }));
  };
  const copyText = async (value: string) => { try { await navigator.clipboard.writeText(value); setNotice(t.copied); } catch { setNotice(t.copyFailed); } };
  const chooseStage = (next: number) => { setStage(next); setNotice(""); };
  const visibleSamples = stage === 1 ? collection.samples : collection.samples.filter(sample => sample.group === activeGroup);

  return <section className="learning-workspace" lang={locale} aria-label={t.workspace}>
    <header className="lw-header">
      <div><span className="lw-eyebrow">vibeKidbright / {t.workspace}</span><p>{t.subtitle}</p></div>
      <div className="lw-header-actions"><span className="lw-preview-tag"><span />{t.design}</span><div className="lw-locale" role="group" aria-label={t.language}>{(["th", "en"] as const).map(code => <button key={code} aria-pressed={locale === code} onClick={() => { setLocale(code); localStorage.setItem("vibe-learning-language", code); setNotice(""); }}>{code.toUpperCase()}</button>)}</div></div>
    </header>
    <div className="lw-body">
      <div className="lw-board-picker" role="group" aria-label={t.boardChoice}>{(["kidbright32", "mai", "plus"] as const).map(value => <button key={value} aria-pressed={family === value} onClick={() => { setFamily(value); setNotice(""); }}><Icon name="chip" size={17}/>{value === "plus" ? "μAI Plus" : value === "mai" ? "μAI" : "KidBright32"}</button>)}</div>
      <div className="lw-hero"><div><span className="lw-kicker">LEARN. MAKE. REPEAT.</span><h1>{family === "kidbright32" ? t.classicTitle : t.hero}</h1><p>{family === "kidbright32" ? t.classicIntro : t.intro}</p></div><button className="lw-button lw-button-soft" aria-haspopup="dialog" aria-expanded={showGuide} onClick={() => setShowGuide(true)}><Icon name="book"/>{t.guide}</button></div>
      {family !== "kidbright32" && <div className="lw-steps" role="tablist" aria-label={t.workflow}>{steps.map((step, index) => <button key={step.icon} role="tab" id={`${prefix}-tab-${index}`} aria-controls={`${prefix}-panel`} aria-selected={stage === index} tabIndex={stage === index ? 0 : -1} ref={node => { tabs.current[index] = node; }} onClick={() => chooseStage(index)} onKeyDown={event => {
        const next = event.key === "ArrowRight" ? (stage + 1) % 4 : event.key === "ArrowLeft" ? (stage + 3) % 4 : event.key === "Home" ? 0 : event.key === "End" ? 3 : -1;
        if (next >= 0) { event.preventDefault(); chooseStage(next); tabs.current[next]?.focus(); }
      }}><span className="lw-step-number">0{index + 1}</span><span><strong>{step.name}</strong><small>{step.hint}</small></span><Icon name={step.icon}/></button>)}</div>}
      <div className="lw-grid">
        <div className="lw-main-column">
          {family === "kidbright32" ? <article className="lw-card lw-classic">
            <div className="lw-card-heading"><span className="lw-symbol"><Icon name="code"/></span><div><h2>{t.classicTitle}</h2><p>{t.classicHint}</p></div></div>
            <div className="lw-prompt-picker"><label htmlFor={`${prefix}-example`}>{t.chooseExample}</label><select id={`${prefix}-example`} value={classicLesson} onChange={event => setClassicLesson(event.target.value as ClassicLesson)}>{classicExamples.map((item, i) => <option key={item.id} value={item.id}>0{i + 1} · {item.title}</option>)}</select><p>{classicExample.hint}</p></div>
            <div className="lw-prompt-preview"><div className="lw-prompt-label"><label htmlFor={`${prefix}-classic-prompt`}>{t.editPrompt}</label><button disabled={classicDrafts[classicLesson] === undefined} onClick={() => setClassicDrafts(current => { const next = { ...current }; delete next[classicLesson]; return next; })}>{t.resetPrompt}</button></div>
              <textarea id={`${prefix}-classic-prompt`} rows={6} value={classicPrompt} onChange={event => setClassicDrafts(current => ({ ...current, [classicLesson]: event.target.value }))}/><p>{t.promptReview}</p>
            </div>
            <div className="lw-prompt-actions"><button className="lw-button lw-button-primary" disabled={!classicPrompt.trim()} onClick={() => onOpenCoder(classicPrompt)}>{t.usePrompt}<Icon name="arrow" size={18}/></button><button className="lw-button lw-button-soft" onClick={() => void copyText(classicPrompt)} disabled={!classicPrompt.trim()}><Icon name="copy" size={16}/>{t.copy}</button><button className="lw-prompt-editor" onClick={onOpenEditor}>{t.openEditor}<Icon name="code" size={16}/></button></div>
          </article> : <>
            <div className="lw-task-row"><span>{t.task}</span><div role="group" aria-label={t.task}>{(["image", "object", "voice"] as const).map(value => <button key={value} aria-pressed={task === value} onClick={() => { setTask(value); setSelectedGroup(""); setNotice(""); }}><Icon name={value === "voice" ? "mic" : value === "object" ? "object" : "camera"} size={16}/>{t[value]}</button>)}</div></div>
            <article className="lw-card lw-workbench" role="tabpanel" id={`${prefix}-panel`} aria-labelledby={`${prefix}-tab-${stage}`}>
              <div className="lw-card-heading"><span className="lw-symbol"><Icon name={steps[stage].icon}/></span><div><h2>{stage === 0 ? t.library : stage === 1 ? t.annotation : stage === 2 ? t.training : t.codeTitle}</h2><p>{stage === 0 ? (task === "voice" ? t.voiceHint : t.libraryHint) : stage === 1 ? t.annotationHint : stage === 2 ? t.trainingHint : t.codeIntro}</p></div>{stage < 2 && <span className="lw-count">{collection.samples.length} {t.samples}</span>}</div>
              {task === "object" && stage === 1 && <p className="lw-inline-note">{t.objectNote}</p>}
              {stage < 2 && <div className="lw-dataset">
                <aside className="lw-groups"><div className="lw-section-label">{t.groups}<span>{collection.groups.length}</span></div>{collection.groups.map((group, index) => <div className={`lw-group ${activeGroup === group.id ? "is-active" : ""}`} key={group.id}>
                  <button aria-label={`${t.group} ${index + 1}`} aria-pressed={activeGroup === group.id} className="lw-group-dot" onClick={() => setSelectedGroup(group.id)}><span style={{ background: ["#d97b5e", "#569b87", "#719ac4", "#b28bb5"][index % 4] }}/></button>
                  <input aria-label={`${t.groupName} ${index + 1}`} value={group.name} placeholder={`${t.group} ${index + 1}`} maxLength={48} onFocus={() => setSelectedGroup(group.id)} onChange={event => updateCollection(current => ({ ...current, groups: current.groups.map(item => item.id === group.id ? { ...item, name: event.target.value } : item) }))}/><span>{collection.samples.filter(sample => sample.group === group.id).length}</span>
                </div>)}<button className="lw-add-group" onClick={() => updateCollection(current => ({ ...current, groups: [...current.groups, { id: `group-${sequence.current++}`, name: "" }] }))}><Icon name="plus" size={15}/>{t.addGroup}</button></aside>
                <div className="lw-sample-area" onDragOver={event => event.preventDefault()} onDrop={event => { event.preventDefault(); importFiles(Array.from(event.dataTransfer.files)); }}>
                  {visibleSamples.length === 0 ? <div className="lw-empty"><div className="lw-empty-graphic"><Icon name={task === "voice" ? "mic" : "camera"} size={35}/><span>+</span></div><h3>{task === "voice" ? t.emptyVoice : t.empty}</h3><p>{task === "voice" ? t.emptyVoiceHint : t.emptyHint}</p><button className="lw-button lw-button-primary" onClick={() => { if (stage === 1) chooseStage(0); fileInput.current?.click(); }}><Icon name="upload" size={17}/>{t.import}</button></div> : <div className="lw-sample-grid">{visibleSamples.map(sample => <div className="lw-sample" key={sample.id}>{task === "voice" ? <div className="lw-audio"><Icon name="mic"/><audio controls preload="none" src={sample.url} aria-label={sample.name}/></div> : <img src={sample.url} alt={sample.name}/>}<button className="lw-remove" aria-label={`${t.remove}: ${sample.name}`} onClick={() => removeSample(sample)}><Icon name="close" size={14}/></button><span className="lw-sample-name" title={sample.name}>{sample.name}</span>{stage === 1 && <select aria-label={`${t.assignment}: ${sample.name}`} value={sample.group} onChange={event => updateCollection(current => ({ ...current, samples: current.samples.map(item => item.id === sample.id ? { ...item, group: event.target.value } : item) }))}>{collection.groups.map(group => <option key={group.id} value={group.id}>{groupName(group.id)}</option>)}</select>}</div>)}</div>}
                  <input ref={fileInput} type="file" aria-label={t.import} hidden multiple accept={task === "voice" ? ".wav,.mp3,.ogg,.m4a" : ".png,.jpg,.jpeg,.webp"} onChange={event => { importFiles(Array.from(event.target.files || [])); event.target.value = ""; }}/>
                  {visibleSamples.length > 0 && <button className="lw-add-group" onClick={() => fileInput.current?.click()}><Icon name="plus" size={15}/>{t.import}</button>}
                </div>
              </div>}
              {stage === 2 && <div className="lw-training"><div className="lw-section-label">{t.readiness}<span>{collection.samples.length} {t.samples}</span></div>{collection.groups.map(group => { const count = collection.samples.filter(sample => sample.group === group.id).length; return <div className="lw-balance" key={group.id}><span>{groupName(group.id)}</span><div><i style={{ width: `${count / Math.max(1, collection.samples.length) * 100}%` }}/></div><small>{count || t.missing}</small></div>; })}<div className="lw-format"><Icon name="chip"/><span>{t.format}<strong>{boardName} <code>{modelFormat}</code></strong></span></div><button className="lw-button lw-button-primary" disabled>{t.startTraining}</button><p className="lw-capability-note">{t.later}</p></div>}
              {stage === 3 && <div className="lw-code-panel"><div className="lw-code-title"><span>main.py · {boardName}</span><button onClick={() => void copyText(example)}><Icon name="copy" size={15}/>{t.copyCode}</button></div><pre><code>{example}</code></pre><p>{t.codeNote}</p><div className="lw-code-actions"><button className="lw-button lw-button-primary" onClick={onOpenEditor}>{t.openEditor}<Icon name="arrow" size={17}/></button><button className="lw-button lw-button-soft" disabled>{t.run}</button></div><p className="lw-capability-note">{t.later}</p></div>}
              {stage < 2 && <footer className="lw-dataset-footer"><span>{t.fileHint}</span><button disabled className="lw-button lw-button-soft"><Icon name={task === "voice" ? "mic" : "camera"} size={16}/>{task === "voice" ? t.microphone : t.camera}</button></footer>}
            </article>
            <div className="lw-under-panel"><p>{stage < 2 ? t.local : ""}</p><div>{stage > 0 && <button className="lw-back" onClick={() => chooseStage(stage - 1)}>{t.previous}</button>}{stage < 3 && <button className="lw-button lw-button-soft" onClick={() => chooseStage(stage + 1)}>{t.next}<Icon name="arrow" size={16}/></button>}</div></div>
          </>}
          {family !== "kidbright32" && <article className="lw-card lw-idea"><div className="lw-card-heading"><img src="/app-icon.png" width="30" height="30" alt=""/><div><h2>{t.promptTitle}</h2><p>{t.promptHint}</p></div><span className="lw-idea-spark">✦</span></div><textarea aria-label={t.prompt} value={idea} onChange={event => setIdeas(current => ({ ...current, [task]: event.target.value }))}/><div className="lw-idea-actions"><button onClick={() => void copyText(idea)}><Icon name="copy" size={15}/>{t.copy}</button><button onClick={() => onOpenCoder(`${boardName}\n${idea}`)}>{t.coder}<Icon name="arrow" size={16}/></button></div></article>}
        </div>
        <aside className="lw-side-column"><article className="lw-card lw-device-card"><div className="lw-section-label">{t.device}<Icon name="chip" size={16}/></div><BoardIllustration family={family} label={t.illustration}/><h2>{boardName}</h2><span className="lw-offline"><span />{t.notConnected}</span><p>{t.prepareHint}</p>{family !== "kidbright32" && <div className="lw-future"><Icon name="train" size={16}/><span>{t.later}</span></div>}</article>
          <article className="lw-guide-card"><span className="lw-symbol"><Icon name="book"/></span><h2>{t.prepare}</h2><p>{family === "kidbright32" ? t.classicPrepare : t.prepareHint}</p><button className="lw-guide-toggle" aria-haspopup="dialog" aria-expanded={showGuide} onClick={() => setShowGuide(true)}>{t.guide}<Icon name="arrow" size={16}/></button></article>
        </aside>
      </div>
      <div className="lw-notice" role="status" aria-live="polite">{notice}</div>
    </div>
    {showGuide && <BoardPreparationGuide family={family} locale={locale} onClose={() => setShowGuide(false)} onOpenEditor={onOpenEditor}/>}
  </section>;
}
