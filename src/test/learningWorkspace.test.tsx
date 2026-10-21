import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import LearningWorkspace from "../LearningWorkspace";

const makeUrl = vi.fn();
const revokeUrl = vi.fn();
const openEditor = vi.fn();
const openCoder = vi.fn();
const upload = (name: string, type = "image/png") => fireEvent.change(screen.getByLabelText("เลือกไฟล์จากคอมพิวเตอร์"), {
  target: { files: [new File(["sample"], name, { type })] },
});

describe("Learning workspace practice dataset", () => {
  const originalShowModal = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, "showModal");
  const originalClose = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, "close");
  // jsdom has no top-layer dialog implementation. Real focus/inert/viewport
  // behavior is exercised separately in the production browser probe.
  beforeAll(() => {
    Object.defineProperty(HTMLDialogElement.prototype, "showModal", { configurable: true, value: function (this: HTMLDialogElement) { this.setAttribute("open", ""); } });
    Object.defineProperty(HTMLDialogElement.prototype, "close", { configurable: true, value: function (this: HTMLDialogElement) { this.removeAttribute("open"); } });
  });
  afterAll(() => {
    for (const [name, descriptor] of [["showModal", originalShowModal], ["close", originalClose]] as const) {
      if (descriptor) Object.defineProperty(HTMLDialogElement.prototype, name, descriptor);
      else Reflect.deleteProperty(HTMLDialogElement.prototype, name);
    }
  });
  beforeEach(() => {
    localStorage.clear(); vi.clearAllMocks();
    let count = 0;
    makeUrl.mockImplementation(() => `blob:practice-${++count}`);
    Object.defineProperty(URL, "createObjectURL", { configurable: true, value: makeUrl });
    Object.defineProperty(URL, "revokeObjectURL", { configurable: true, value: revokeUrl });
  });
  afterEach(cleanup);
  const mount = () => render(<LearningWorkspace onOpenEditor={openEditor} onOpenCoder={openCoder}/>);
  it("selects and edits a KidBright32 prompt before handing it to Vibe Coder", () => {
    mount(); fireEvent.click(screen.getByRole("button", { name: "KidBright32" }));
    expect((screen.getByLabelText("ปรับโจทย์ของคุณ") as HTMLTextAreaElement).value).toContain("s1_sound");
    fireEvent.change(screen.getByLabelText("เลือกตัวอย่างพรอมท์"), { target: { value: "temperature" } });
    expect((screen.getByLabelText("ปรับโจทย์ของคุณ") as HTMLTextAreaElement).value).toContain("Serial");
    fireEvent.change(screen.getByLabelText("ปรับโจทย์ของคุณ"), { target: { value: "โจทย์ที่ฉันแก้เอง" } });
    expect(openCoder).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "นำไปใช้ใน Vibe Coder" }));
    expect(openCoder).toHaveBeenCalledExactlyOnceWith("โจทย์ที่ฉันแก้เอง");
    expect(openEditor).not.toHaveBeenCalled();
  });
  it("preserves each example draft and localizes untouched examples", () => {
    mount(); fireEvent.click(screen.getByRole("button", { name: "KidBright32" }));
    fireEvent.change(screen.getByLabelText("ปรับโจทย์ของคุณ"), { target: { value: "My edited idea" } });
    fireEvent.change(screen.getByLabelText("เลือกตัวอย่างพรอมท์"), { target: { value: "display" } });
    fireEvent.click(screen.getByRole("button", { name: "EN" }));
    expect((screen.getByLabelText("Make the idea yours") as HTMLTextAreaElement).value).toContain("Scroll HELLO");
    fireEvent.change(screen.getByLabelText("Choose a prompt example"), { target: { value: "sound" } });
    expect(screen.getByLabelText("Make the idea yours")).toHaveValue("My edited idea");
    fireEvent.click(screen.getByRole("button", { name: "Reset example" }));
    expect((screen.getByLabelText("Make the idea yours") as HTMLTextAreaElement).value).toContain("On S1 press");
  });
  it("disables empty prompt handoff and forwards the AI lesson idea", () => {
    mount(); fireEvent.click(screen.getByRole("button", { name: "KidBright32" }));
    fireEvent.change(screen.getByLabelText("ปรับโจทย์ของคุณ"), { target: { value: " " } });
    expect(screen.getByRole("button", { name: "นำไปใช้ใน Vibe Coder" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "μAI Plus" }));
    fireEvent.change(screen.getByLabelText("โจทย์ของฉัน"), { target: { value: "ช่วยวางแผนเก็บภาพ" } });
    fireEvent.click(screen.getByRole("button", { name: "เปิด Vibe Coder" }));
    expect(openCoder).toHaveBeenCalledWith("KidBright μAI Plus\nช่วยวางแผนเก็บภาพ");
  });

  it.each([
    ["KidBright32", "Check the board's serial port", "Check USB networking on Windows"],
    ["μAI", "Identify the original V831 μAI", "Check USB networking on Windows"],
    ["μAI Plus", "Check USB networking on Windows", "Check the board's serial port"],
  ])("opens the offline English guide for %s", (board, expected, other) => {
    mount();
    fireEvent.click(screen.getByRole("button", { name: "EN" }));
    fireEvent.click(screen.getByRole("button", { name: board }));
    fireEvent.click(screen.getAllByRole("button", { name: "Board preparation guide" })[0]);
    const guide = screen.getByRole("dialog", { name: "Board preparation guide" });
    expect(guide).toHaveAttribute("open");
    expect(within(guide).getByText(expected)).toBeVisible();
    expect(within(guide).queryByText(other)).not.toBeInTheDocument();
    expect(within(guide).getByText(/Available offline/)).toBeVisible();
    fireEvent.click(within(guide).getByRole("button", { name: "Back to lesson" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("opens the Thai guide from both entry points and handles Escape cancellation", () => {
    mount();
    const triggers = screen.getAllByRole("button", { name: "คู่มือเตรียมบอร์ด" });
    for (const trigger of triggers) {
      trigger.focus();
      fireEvent.click(trigger);
      const guide = screen.getByRole("dialog", { name: "คู่มือเตรียมบอร์ด" });
      expect(within(guide).getByText("ตรวจ USB network บน Windows")).toBeVisible();
      fireEvent(guide, new Event("cancel", { bubbles: false, cancelable: true }));
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expect(trigger).toHaveFocus();
      expect(trigger).toHaveAttribute("aria-expanded", "false");
    }
    expect(openEditor).not.toHaveBeenCalled();
  });

  it("preserves imported examples, class names and the current stage after reading a guide", () => {
    mount(); upload("lesson.png");
    fireEvent.change(screen.getByLabelText("ชื่อกลุ่ม 1"), { target: { value: "Keep this class" } });
    fireEvent.click(screen.getByRole("tab", { name: /ติดป้ายกำกับ/ }));
    fireEvent.click(screen.getAllByRole("button", { name: "คู่มือเตรียมบอร์ด" })[0]);
    fireEvent.click(screen.getByRole("button", { name: "ปิดคู่มือ" }));
    expect(screen.getByAltText("lesson.png")).toBeVisible();
    expect(screen.getByLabelText("ชื่อกลุ่ม 1")).toHaveValue("Keep this class");
    expect(screen.getByRole("tab", { name: /ติดป้ายกำกับ/ })).toHaveAttribute("aria-selected", "true");
    expect(revokeUrl).not.toHaveBeenCalled();
  });

  it("closes the KidBright32 guide before opening the actual code workspace", () => {
    mount();
    fireEvent.click(screen.getByRole("button", { name: "KidBright32" }));
    fireEvent.click(screen.getAllByRole("button", { name: "คู่มือเตรียมบอร์ด" })[1]);
    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "เปิดพื้นที่เขียนโปรแกรม" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(openEditor).toHaveBeenCalledOnce();
  });

  it("imports real local examples, reassigns their class, and releases preview URLs on removal", () => {
    mount();
    fireEvent.change(screen.getByLabelText("ชื่อกลุ่ม 1"), { target: { value: "ขวด" } });
    fireEvent.change(screen.getByLabelText("ชื่อกลุ่ม 2"), { target: { value: "กระป๋อง" } });
    fireEvent.click(screen.getByRole("button", { name: "กลุ่ม 1" }));
    upload("bottle.png");
    expect(screen.getByAltText("bottle.png")).toHaveAttribute("src", "blob:practice-1");
    fireEvent.click(screen.getByRole("tab", { name: /ติดป้ายกำกับ/ }));
    fireEvent.change(screen.getByLabelText("กลุ่มของตัวอย่าง: bottle.png"), { target: { value: "image-2" } });
    fireEvent.click(screen.getByRole("tab", { name: /เก็บข้อมูล/ }));
    expect(screen.queryByAltText("bottle.png")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "กลุ่ม 2" }));
    expect(screen.getByAltText("bottle.png")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "นำตัวอย่างออก: bottle.png" }));
    expect(revokeUrl).toHaveBeenCalledWith("blob:practice-1");
    expect(screen.queryByAltText("bottle.png")).not.toBeInTheDocument();
  });

  it("keeps datasets separate by task and changes model format by lesson board", () => {
    mount(); upload("bottle.png");
    fireEvent.click(screen.getByRole("button", { name: "จำแนกเสียง" }));
    expect(screen.queryByAltText("bottle.png")).not.toBeInTheDocument();
    upload("hello.wav", "audio/wav");
    fireEvent.click(screen.getByRole("button", { name: "μAI" }));
    fireEvent.click(screen.getByRole("tab", { name: /ฝึกโมเดล/ }));
    expect(screen.getByText(".npz · NumPy FP32")).toBeVisible();
    expect(screen.getByRole("button", { name: "เริ่มฝึกโมเดล" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "μAI Plus" }));
    expect(screen.getByText(".cvimodel + .mud")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "จำแนกภาพ" }));
    fireEvent.click(screen.getByRole("tab", { name: /เก็บข้อมูล/ }));
    expect(screen.getByAltText("bottle.png")).toBeVisible();
    expect(screen.queryByLabelText("hello.wav")).not.toBeInTheDocument();
  });

  it("rejects unsupported files and revokes remaining previews when the workspace closes", () => {
    const view = mount(); upload("document.svg", "image/svg+xml");
    expect(makeUrl).not.toHaveBeenCalled();
    expect(screen.getByRole("status")).toHaveTextContent("บางไฟล์ไม่ตรงกับชนิดข้อมูล");
    upload("photo.jpg", "image/jpeg");
    view.unmount();
    expect(revokeUrl).toHaveBeenCalledWith("blob:practice-1");
  });

  it("switches workspace language without discarding renamed classes or local examples", () => {
    mount(); upload("sample.png");
    fireEvent.change(screen.getByLabelText("ชื่อกลุ่ม 1"), { target: { value: "My class" } });
    fireEvent.click(screen.getByRole("button", { name: "EN" }));
    expect(screen.getByLabelText("Class name 1")).toHaveValue("My class");
    expect(screen.getByAltText("sample.png")).toBeVisible();
    expect(screen.getByRole("tab", { name: /Collect/ })).toHaveAttribute("aria-selected", "true");
    expect(localStorage.getItem("vibe-learning-language")).toBe("en");
  });

  it("supports keyboard step navigation and keeps board execution unavailable", () => {
    mount();
    fireEvent.keyDown(screen.getByRole("tab", { name: /เก็บข้อมูล/ }), { key: "End" });
    expect(screen.getByRole("tab", { name: /เขียนและทดสอบ/ })).toHaveFocus();
    expect(screen.getByRole("button", { name: "ส่งและรันบนบอร์ด" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "เปิดพื้นที่เขียนโปรแกรม" }));
    expect(openEditor).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: "เปิด Vibe Coder" }));
    expect(openCoder).toHaveBeenCalledOnce();
    expect(screen.getByText("ยังไม่เชื่อมต่อบอร์ด")).toBeVisible();
  });
});
