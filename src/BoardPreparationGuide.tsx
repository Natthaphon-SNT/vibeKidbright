import { useEffect, useId, useRef } from "react";

export type BoardFamily = "kidbright32" | "mai" | "plus";
type Step = { title: string; detail: string };
type Guide = { steps: Step[]; problems: Step[] };

// Offline preparation guidance, based on knowledge_base/kidbright_uai.md and
// docs/KIDBRIGHT_MAI_SUPPLIED_FILES_AUDIT_2026-10-17.md. No provisioning or
// connection commands run from this reader. Keep the two Linux families separate.
const GUIDES: Record<"th" | "en", Record<BoardFamily, Guide>> = {
  th: {
    kidbright32: {
      steps: [
        { title: "ตรวจชื่อรุ่นบนบอร์ด", detail: "ดูรุ่นและ revision บนแผ่นวงจร เช่น V1.3, V1.5, iA หรือ iP แล้วเลือกบอร์ดเป้าหมายให้ตรงในพื้นที่เขียนโปรแกรม การเลือกบอร์ดในบทเรียนยังไม่เปลี่ยนบอร์ดเป้าหมายของโปรเจกต์" },
        { title: "ต่อสาย USB ที่ส่งข้อมูลได้", detail: "ต่อพอร์ต USB ของบอร์ดเข้าคอมพิวเตอร์ด้วยสายที่ส่งข้อมูลได้ ตรวจไฟเลี้ยงและรอให้คอมพิวเตอร์ตรวจพบอุปกรณ์" },
        { title: "ตรวจพอร์ตของบอร์ด", detail: "เปิดพื้นที่เขียนโปรแกรม กด Refresh Ports แล้วเลือกพอร์ตของบอร์ด บน Windows ตรวจหมายเลข COM ใน Device Manager > Ports (COM & LPT) หากต้องติดตั้งไดรเวอร์ ให้ใช้ไดรเวอร์ที่ตรงกับชิป USB ของบอร์ดรุ่นนั้น" },
        { title: "เตรียม ESP-IDF และโปรเจกต์", detail: "ใช้ Setup / Repair ESP-IDF ให้พร้อม แล้วสร้างหรือเปิดโปรเจกต์ ESP-IDF สำหรับบอร์ดที่เลือก ไฟล์โปรแกรมที่ผู้เรียนแก้จะอยู่ในโฟลเดอร์ main" },
        { title: "ตรวจโค้ดก่อน Build & Flash", detail: "ตรวจพินและอุปกรณ์ให้ตรงรุ่น ยืนยัน Keep สำหรับการแก้ไขจาก AI ที่ต้องการใช้ แล้วจึงกด Build & Flash หากเปิด Connect Serial อยู่ ให้ตัดการเชื่อมต่อก่อนเมื่อพอร์ตถูกใช้งาน" },
      ],
      problems: [
        { title: "ไม่พบพอร์ต COM", detail: "ลองสายข้อมูลหรือช่อง USB อื่น ตรวจ Device Manager แล้วกด Refresh Ports ใหม่ ชื่อรุ่นบอร์ดอย่างเดียวไม่เพียงพอสำหรับเลือกไดรเวอร์ USB" },
        { title: "พอร์ตถูกใช้งานหรือ Flash ไม่ผ่าน", detail: "ปิด Serial Monitor หรือโปรแกรมอื่นที่ใช้พอร์ต ตรวจพอร์ตที่เลือก แล้วอ่านข้อผิดพลาดใน Terminal ก่อนลองใหม่" },
      ],
    },
    mai: {
      steps: [
        { title: "ยืนยันว่าเป็น μAI รุ่น V831", detail: "ใช้คู่มือและระบบของ KidBright μAI รุ่นแรก แยกจาก μAI Plus ซึ่งใช้ SG2002 ไม่ใช้ GPIO, อิมเมจระบบ หรือ ESP-IDF ของ KidBright32 แทนกัน" },
        { title: "ตรวจ microSD และระบบปฏิบัติการ", detail: "ให้ครูตรวจว่าการ์ดมี Linux image สำหรับรุ่นนี้อยู่แล้ว หากต้องเตรียมหรือกู้คืนการ์ด ให้ทำตามคู่มือเฉพาะรุ่นและสำรองข้อมูลก่อนเขียนการ์ด" },
        { title: "ต่อ USB-OTG และเลือกแหล่งจ่ายไฟ", detail: "ใช้สาย USB ที่ส่งข้อมูลได้ต่อช่อง USB-OTG ตรวจสวิตช์เลือกแหล่งไฟ OTG/DC ตามคู่มือ แล้วรอให้บอร์ดเริ่มทำงาน ช่อง USB-UART/DEBUG ไม่ใช่ช่องเดียวกับ USB-OTG" },
        { title: "ตรวจ Runtime และการเชื่อมต่อ", detail: "ใช้ IDE ของ μAI รุ่นนี้ตรวจเวอร์ชันระบบและ Runtime ตามคู่มือ เส้นทางอ้างอิงรุ่นแรกใช้ ADB/WebUSB ไม่ใช้ขั้นตอน RNDIS ของ Plus โดยอัตโนมัติ" },
        { title: "ลองกล้องและเสียงใน IDE ของบอร์ด", detail: "ตรวจกล้อง จอ และเสียงให้ทำงานก่อนเริ่มเก็บข้อมูล ใน vibeKidbright หน้านี้นำเข้าและจัดกลุ่มไฟล์จากเครื่องได้ ส่วนกล้องจากบอร์ด การฝึกโมเดล และการส่งโปรแกรมไป μAI ยังไม่เปิดใช้งาน" },
      ],
      problems: [
        { title: "บอร์ดไม่เริ่มทำงาน", detail: "ตรวจไฟเลี้ยง สวิตช์ OTG/DC และ microSD ที่ตรงรุ่น ให้ครูตรวจระบบก่อนเขียนหรือฟอร์แมตการ์ดใหม่" },
        { title: "IDE เดิมไม่พบอุปกรณ์", detail: "ตรวจสายข้อมูล ช่อง USB-OTG และสิทธิ์เข้าถึง USB ตามคู่มือของ μAI รุ่นแรก อย่าใช้ที่อยู่หรือไดรเวอร์ของ Plus แทนโดยไม่ตรวจรุ่น" },
      ],
    },
    plus: {
      steps: [
        { title: "ยืนยันว่าเป็น μAI Plus", detail: "ใช้คู่มือและ Linux image สำหรับ KidBright μAI Plus รุ่น SG2002 ไม่ใช้ของ μAI รุ่น V831 หรือโปรเจกต์ ESP-IDF ของ KidBright32 แทนกัน" },
        { title: "ตรวจ microSD และ Runtime", detail: "ให้ครูตรวจ microSD ระบบปฏิบัติการ MaixPy และ device Runtime แยกเวอร์ชันกัน การมี OS อย่างเดียวยังไม่ยืนยันว่าพร้อมรันโปรแกรม ไม่ต้องเขียนการ์ดใหม่ทุกครั้งที่ส่งงาน" },
        { title: "ต่อช่อง USB-OTG", detail: "ใช้สาย USB-C ที่ส่งข้อมูลได้ต่อช่อง USB-OTG ตรวจสวิตช์ OTG/DC ตามแหล่งจ่ายไฟ แล้วรอให้บอร์ดเริ่มทำงาน ไม่สับสนกับช่อง USB-UART" },
        { title: "ตรวจ USB network บน Windows", detail: "เปิด Device Manager > Network adapters แล้วตรวจว่าพบ Remote NDIS based Internet Sharing Device ตามคู่มือ Plus หากไม่พบ ให้ตรวจสาย ช่อง USB และไดรเวอร์ก่อนเริ่มเชื่อมต่อ" },
        { title: "ตรวจบริการบอร์ดและลองกล้อง", detail: "ใช้ IDE ของ Plus ตรวจการเชื่อมต่อและเวอร์ชัน Runtime แล้วลองกล้องและเสียง ใน vibeKidbright หน้านี้นำเข้าและจัดกลุ่มไฟล์จากเครื่องได้ ส่วนกล้องจากบอร์ด การฝึกโมเดล และการส่งโปรแกรมไป Plus ยังไม่เปิดใช้งาน" },
      ],
      problems: [
        { title: "ไม่พบ USB network", detail: "ตรวจสายข้อมูล ช่อง USB-OTG และสวิตช์ไฟ รอให้บอร์ดบูตเสร็จแล้วตรวจ Network adapters อีกครั้ง ขั้นตอนนี้เป็นของ Plus ไม่ใช่ μAI ทุกรุ่น" },
        { title: "พบเครือข่ายแต่ IDE เดิมยังเชื่อมต่อไม่ได้", detail: "ตรวจบริการบอร์ดและ Runtime ตามคู่มือ Plus ที่ตรงกับ image หากต้อง Reset ให้บันทึกงานก่อน แล้วรอให้บอร์ดเริ่มใหม่ การเปิดคู่มือนี้ไม่ได้ติดตั้ง Runtime หรือเชื่อมต่ออุปกรณ์ให้" },
      ],
    },
  },
  en: {
    kidbright32: {
      steps: [
        { title: "Check the model on the PCB", detail: "Read the model and revision, such as V1.3, V1.5, iA or iP, and select the matching target in the code workspace. Choosing a lesson board does not change the project's target board." },
        { title: "Connect a USB data cable", detail: "Connect the board's USB port to your computer with a data-capable cable. Check board power and wait for the computer to detect it." },
        { title: "Check the board's serial port", detail: "Open the code workspace, click Refresh Ports and select your board's port. On Windows, check Device Manager > Ports (COM & LPT). If a driver is required, use one matching this board's USB chip." },
        { title: "Prepare ESP-IDF and your project", detail: "Use Setup / Repair ESP-IDF, then create or open an ESP-IDF project for the selected board. Learners edit their program in the main folder." },
        { title: "Review before Build & Flash", detail: "Check pins and peripherals against your model. Keep the AI changes you want to apply, then use Build & Flash. Disconnect Connect Serial first if it is holding the port." },
      ],
      problems: [
        { title: "No COM port appears", detail: "Try another data cable or USB port, check Device Manager and click Refresh Ports again. The board name alone does not identify its USB driver." },
        { title: "Port busy or flashing fails", detail: "Close other serial monitors, check the selected port and read the Terminal error before retrying." },
      ],
    },
    mai: {
      steps: [
        { title: "Identify the original V831 μAI", detail: "Use the original KidBright μAI manual and system, separately from the SG2002 μAI Plus. Do not substitute KidBright32 GPIO mappings, OS images or ESP-IDF projects." },
        { title: "Check microSD and the operating system", detail: "Ask your teacher to check that the card has the Linux image for this model. For provisioning or recovery, follow its manual and back up data before writing the card." },
        { title: "Connect USB-OTG and check power", detail: "Use a USB data cable on USB-OTG, check the OTG/DC power selector against the manual and wait for startup. USB-UART/DEBUG is a separate port." },
        { title: "Check Runtime and connectivity", detail: "Use this model's IDE to check system and Runtime versions. The original reference uses ADB/WebUSB; do not automatically apply the Plus RNDIS procedure." },
        { title: "Test camera and audio in the board's IDE", detail: "Check camera, display and audio before collecting data. This vibeKidbright page supports importing and grouping local files. Board capture, model training and deployment to μAI are not available yet." },
      ],
      problems: [
        { title: "The board does not start", detail: "Check power, the OTG/DC selector and the correct microSD image. Ask your teacher to check the system before rewriting or formatting the card." },
        { title: "The original IDE cannot find the device", detail: "Check the data cable, USB-OTG port and USB permissions using the original μAI manual. Do not substitute Plus addresses or drivers without checking the model." },
      ],
    },
    plus: {
      steps: [
        { title: "Identify μAI Plus", detail: "Use the manual and Linux image for the SG2002 KidBright μAI Plus. Do not substitute the V831 μAI image or a KidBright32 ESP-IDF project." },
        { title: "Check microSD and Runtime", detail: "Ask your teacher to check microSD, OS, MaixPy and device Runtime as separate versions. An OS alone does not confirm readiness. You do not need to rewrite the card for every program upload." },
        { title: "Connect USB-OTG", detail: "Use a USB-C data cable on USB-OTG, check the OTG/DC selector for your power source and wait for startup. Do not confuse it with USB-UART." },
        { title: "Check USB networking on Windows", detail: "Open Device Manager > Network adapters and check for Remote NDIS based Internet Sharing Device, as described in the Plus manual. If missing, check the cable, USB port and driver before connecting." },
        { title: "Check the board service and test the camera", detail: "Use the Plus IDE to check connectivity and Runtime, then test camera and audio. This vibeKidbright page supports local file import and grouping. Board capture, model training and deployment to Plus are not available yet." },
      ],
      problems: [
        { title: "USB networking does not appear", detail: "Check the data cable, USB-OTG port and power selector. Wait for startup and check Network adapters again. This procedure is for Plus, not every μAI model." },
        { title: "Network detected but the original IDE cannot connect", detail: "Check the board service and Runtime using the Plus manual matching your image. Save work before Reset and wait for startup. Opening this guide does not install Runtime or connect a device." },
      ],
    },
  },
};

export default function BoardPreparationGuide({ family, locale, onClose, onOpenEditor }: {
  family: BoardFamily; locale: "th" | "en"; onClose: () => void; onOpenEditor: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  const hintId = useId();
  const th = locale === "th";
  const board = family === "plus" ? "KidBright μAI Plus" : family === "mai" ? "KidBright μAI" : "KidBright32";
  const guide = GUIDES[locale][family];
  useEffect(() => {
    const node = dialog.current!;
    const opener = document.activeElement;
    node.showModal();
    return () => {
      node.close();
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus({ preventScroll: true });
    };
  }, []);

  return <dialog ref={dialog} className="lw-guide-dialog" lang={locale} aria-labelledby={titleId} aria-describedby={hintId}
    onCancel={event => { event.preventDefault(); onClose(); }}
    onClick={event => { if (event.target === event.currentTarget) onClose(); }}>
    <div className="lw-guide-content">
      <header className="lw-guide-header">
        <div><span className="lw-kicker">{board}</span><h2 id={titleId}>{th ? "คู่มือเตรียมบอร์ด" : "Board preparation guide"}</h2></div>
        <button type="button" className="lw-guide-close" aria-label={th ? "ปิดคู่มือ" : "Close guide"} onClick={onClose}>×</button>
      </header>
      <div className="lw-guide-reader">
        <p id={hintId}>{th ? "อ่านได้แบบออฟไลน์ ทำตามขั้นตอนสำหรับบอร์ดที่เลือก คู่มือนี้ไม่เปลี่ยนโปรเจกต์หรือตั้งค่าบอร์ดให้อัตโนมัติ" : "Available offline. Follow the steps for your selected board. This guide does not change your project or configure a board automatically."}</p>
        <ol className="lw-guide-steps">{guide.steps.map(step => <li key={step.title}><h3>{step.title}</h3><p>{step.detail}</p></li>)}</ol>
        <h3>{th ? "ถ้ายังใช้งานไม่ได้" : "Troubleshooting"}</h3>
        {guide.problems.map(problem => <details className="lw-guide-problem" key={problem.title}><summary>{problem.title}</summary><p>{problem.detail}</p></details>)}
      </div>
      <footer className="lw-guide-footer">
        <button type="button" className="lw-button lw-button-soft" onClick={onClose}>{th ? "กลับไปบทเรียน" : "Back to lesson"}</button>
        {family === "kidbright32" && <button type="button" className="lw-button lw-button-primary" onClick={() => { onClose(); onOpenEditor(); }}>{th ? "เปิดพื้นที่เขียนโปรแกรม" : "Open code workspace"}</button>}
      </footer>
    </div>
  </dialog>;
}
