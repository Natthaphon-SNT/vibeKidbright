// ── Friendly Build Error Parser & Hints ────────────────────────────────────
// Parses raw compiler / linker / esptool output lines into structured,
// kid-friendly errors with plain-language Thai + English explanations.

export interface ParsedBuildError {
    /** Source file mentioned by the toolchain (if any) */
    file?: string;
    /** Line number in that file (if any) */
    line?: number;
    /** Column number (if any) */
    column?: number;
    /** Raw message from the toolchain */
    message: string;
    /** Short English title for the list UI */
    title: string;
    /** Plain-language Thai explanation */
    thaiHint: string;
    /** Plain-language English explanation */
    englishHint: string;
}

// ── Hint table ──────────────────────────────────────────────────────────────
interface ErrorHint {
    match: RegExp;
    title: string;
    thai: string;
    en: string;
}

const GCC_ERROR_RE = /^(.+?\.(?:c|cpp|cc|cxx|h|hpp|s|S)):(\d+):(?:(\d+):)?\s*(?:fatal\s+)?error:\s*(.*)$/i;
const CMAKE_ERROR_RE = /^CMake Error at (.+?):(\d+)\s*(?:\(([^)]*)\))?:\s*(.*)$/i;

const HINT_TABLE: ErrorHint[] = [
    {
        match: /expected\s+';'/i,
        title: "Missing semicolon",
        thai: "ลืมใส่เครื่องหมาย ; ท้ายคำสั่ง (ดูบรรทัดนี้และบรรทัดก่อนหน้า)",
        en: "A semicolon ; is missing at the end of a statement. Check this line and the one above it.",
    },
    {
        match: /expected declaration|expected '\}'|at end of input/i,
        title: "Missing closing brace }",
        thai: "มีวงเล็บปีกกา { ที่ยังไม่ปิด } — นับ { กับ } ให้ครบทุกฟังก์ชัน",
        en: "A closing brace } is missing somewhere. Count your { and } pairs.",
    },
    {
        match: /was not declared in this scope|undeclared \(first use/i,
        title: "Unknown variable or function",
        thai: "เรียกใช้ชื่อตัวแปร/ฟังก์ชันที่ยังไม่มี — เช็กการสะกดชื่อว่าถูกต้อง และประกาศก่อนใช้จริง",
        en: "You used a variable or function that was never declared. Check spelling and declare it before use.",
    },
    {
        match: /implicit declaration of function/i,
        title: "Function used without its header",
        thai: "เรียกใช้ฟังก์ชันโดยยังไม่ได้ #include header ของมัน",
        en: "A function was called before its header was included. Add the right #include at the top.",
    },
    {
        match: /No such file or directory/i,
        title: "File or header not found",
        thai: "หาไฟล์ที่ #include ไม่เจอ — ชื่ออาจสะกดผิด หรือยังไม่ได้เพิ่ม component ใน CMakeLists.txt (REQUIRES)",
        en: "An #include'd file was not found. Check the spelling, or add the component to CMakeLists.txt (REQUIRES).",
    },
    {
        match: /undefined reference to/i,
        title: "Missing source file or library",
        thai: "โค้ดถูกแต่ลิงก์ไม่เจอ — มักลืมใส่ไฟล์ .c ใน main/CMakeLists.txt หรือลืม REQUIRES ไลบรารีที่ต้องใช้",
        en: "The code compiled but the linker can't find it. Usually a missing .c file in main/CMakeLists.txt or a missing REQUIRES library.",
    },
    {
        match: /multiple definition of/i,
        title: "Name defined twice",
        thai: "มีการประกาศชื่อเดียวกันซ้ำ 2 ที่ — เปลี่ยนชื่อ หรือย้ายไปไว้ใน header เป็น extern",
        en: "The same name is defined in two places. Rename it, or declare it extern in a header.",
    },
    {
        match: /too few arguments|too many arguments/i,
        title: "Wrong number of arguments",
        thai: "ส่งค่าให้ฟังก์ชันไม่ครบ/เกิน — เทียบกับการประกาศฟังก์ชันดูว่าต้องส่งกี่ค่า",
        en: "You passed the wrong number of values to a function. Compare with how it is declared.",
    },
    {
        match: /format '%[a-z]' expects argument/i,
        title: "printf format mismatch",
        thai: "รูปแบบ %d %s %f ใน printf ไม่ตรงกับค่าที่ส่งเข้าไป",
        en: "The printf placeholders (%d, %s, %f...) don't match the values you pass in.",
    },
    {
        match: /incompatible types|makes (pointer|integer)/i,
        title: "Wrong data type",
        thai: "ชนิดข้อมูลไม่ตรงกัน เช่น ส่งข้อความให้ตัวแปร int — ตรวจชนิดตัวแปรให้ตรง",
        en: "Data type mismatch, e.g. assigning text to an int variable. Check the variable types.",
    },
    {
        match: /redefinition of/i,
        title: "Duplicate definition",
        thai: "ประกาศตัวแปร/ฟังก์ชันชื่อนี้ซ้ำในไฟล์เดียวกัน — ใช้ชื่ออื่นแทน",
        en: "This variable/function is declared twice in the same file. Use a different name.",
    },
    {
        match: /stray '\\\d+'/i,
        title: "Non-English character in code",
        thai: "มีอักขระที่ไม่ใช่ภาษาอังกฤษปนในโค้ด เช่น อัญประกาศไทย \" \" — ต้องเป็น \" ธรรมดาเท่านั้น",
        en: "There's a non-ASCII character in the code, e.g. smart quotes. Replace them with normal ASCII quotes and letters.",
    },
    {
        match: /unterminated comment/i,
        title: "Unclosed comment",
        thai: "มี /* ที่ยังไม่ปิดด้วย */",
        en: "A /* comment block was never closed with */.",
    },
    {
        match: /control reaches end of non-void function/i,
        title: "Missing return value",
        thai: "ฟังก์ชันนี้ระบุว่าต้อง return ค่า แต่บางเส้นทางไม่ได้ return — เพิ่ม return ให้ครบ",
        en: "This function promises to return a value but some path doesn't return anything. Add a return statement.",
    },
    {
        match: /(storage size|invalid application of 'sizeof') .*unknown|incomplete type/i,
        title: "Incomplete type — missing include",
        thai: "ยังไม่รู้จักชนิดข้อมูลนี้ — ต้อง #include header ที่ประกาศ struct/type นี้ก่อน",
        en: "The compiler doesn't know this type yet. Include the header that declares it.",
    },
    {
        match: /request for member .* not a structure|dereferencing pointer to incomplete type/i,
        title: "Wrong access on type",
        thai: "เข้าถึง field ของ struct ไม่ถูกต้อง — เช็กว่าตัวแปรเป็น struct จริงและ include header แล้ว",
        en: "Invalid access to a struct member. Check the variable really is that struct and its header is included.",
    },
    {
        match: /subscripted value is (neither array|not array)/i,
        title: "Indexing a non-array",
        thai: "ใช้ [ ] กับตัวแปรที่ไม่ใช่ array",
        en: "You used square brackets [ ] on something that isn't an array.",
    },
    {
        match: /division by zero/i,
        title: "Division by zero",
        thai: "หารด้วยศูนย์ไม่ได้ — เช็กว่าตัวหารไม่ใช่ 0",
        en: "Dividing by zero is not allowed. Make sure the divisor can't be 0.",
    },
    {
        match: /comparison between signed and unsigned/i,
        title: "Signed/unsigned comparison",
        thai: "(คำเตือน) เปรียบเทียบเลขติดลบได้กับเลขไม่ติดลบ — อาจได้ผลไม่ตรงตามคิด",
        en: "(Warning) Comparing signed and unsigned numbers can give surprising results.",
    },
    {
        match: /GPIO|pin.*not (set|configured)|invalid (gpio|peripheral)/i,
        title: "GPIO/peripheral problem",
        thai: "ระบุ GPIO/pin ไม่ถูกต้อง — เช็กหมายเลข pin กับคู่มือบอร์ด KidBright และว่าได้เรียก gpio_config ก่อนใช้",
        en: "Invalid GPIO/pin usage. Check the pin number against the KidBright board docs and configure it first.",
    },
];

const GENERIC_HINT: Pick<ParsedBuildError, "title" | "thaiHint" | "englishHint"> = {
    title: "Build error",
    thaiHint: "คอมไพเลอร์รายงานปัญหาที่บรรทัดนี้ — อ่านข้อความดิบด้านล่าง หรือกดปุ่ม 🤖 ให้ AI ช่วยแก้",
    englishHint: "The compiler reported a problem here. Read the raw message below, or press 🤖 to let the AI fix it.",
};

function applyHint(message: string, base: Partial<ParsedBuildError>): ParsedBuildError {
    for (const hint of HINT_TABLE) {
        if (hint.match.test(message)) {
            return {
                message,
                title: hint.title,
                thaiHint: hint.thai,
                englishHint: hint.en,
                ...base,
            } as ParsedBuildError;
        }
    }
    return { message, ...GENERIC_HINT, ...base } as ParsedBuildError;
}

// ── Flash / hardware error hints ─────────────────────────────────────────────

const FLASH_CONNECT_RE = /failed to connect to (esp32|esp32-s2|esp32-s3|esp32-c3)?|no serial data received|wrong boot mode/i;
const FLASH_PORT_BUSY_RE = /permission denied|access is denied|could not open port|resource busy/i;
const FATAL_ERROR_RE = /a fatal error occurred:\s*(.*)/i;

function parseFlashError(raw: string): ParsedBuildError | null {
    if (FLASH_CONNECT_RE.test(raw)) {
        return {
            message: raw,
            title: "Can't talk to the board",
            thaiHint: "เชื่อมต่อบอร์ดไม่ได้ — 1) กดปุ่ม BOOT ค้างไว้แล้วกด RST 1 ครั้ง 2) ปิด Serial Monitor ก่อน flash 3) เช็กว่าเลือก COM Port ของบอร์ดถูกต้อง 4) ลองถอด-เสียบสาย USB",
            englishHint: "Couldn't reach the board — 1) Hold BOOT then tap RST once. 2) Close the Serial Monitor before flashing. 3) Check the correct COM port is selected. 4) Try unplugging/replugging USB.",
        };
    }
    if (FLASH_PORT_BUSY_RE.test(raw)) {
        return {
            message: raw,
            title: "COM port is busy",
            thaiHint: "COM Port ถูกใช้งานอยู่ — กด Disconnect Serial Monitor ก่อน แล้วลอง flash ใหม่",
            englishHint: "The COM port is being used by another program. Disconnect the Serial Monitor first, then try again.",
        };
    }
    const fatal = raw.match(FATAL_ERROR_RE);
    if (fatal) {
        return applyHint(fatal[1], { message: raw });
    }
    return null;
}

// ── Public API ───────────────────────────────────────────────────────────────

/** Parse a single output line into 0 or 1 friendly errors. */
export function parseErrorLine(rawLine: string): ParsedBuildError | null {
    const raw = rawLine.trim();
    if (!raw) return null;

    // esptool / flashing problems first (they don't carry file:line)
    const flashErr = parseFlashError(raw);
    if (flashErr) return flashErr;

    // GCC-style: path/file.c:12:34: error: message
    const gcc = raw.match(GCC_ERROR_RE);
    if (gcc) {
        return applyHint(gcc[4], {
            file: gcc[1],
            line: parseInt(gcc[2], 10),
            column: gcc[3] ? parseInt(gcc[3], 10) : undefined,
            message: gcc[4] || raw,
        });
    }

    // CMake-style: CMake Error at path/CMakeLists.txt:5 (target_link_libraries): msg
    const cmake = raw.match(CMAKE_ERROR_RE);
    if (cmake) {
        return applyHint(cmake[4], {
            file: cmake[1],
            line: parseInt(cmake[2], 10),
            message: cmake[4] || raw,
        });
    }

    // Bare linker error lines without file:line (e.g. "/usr/bin/ld: ...: undefined reference to `foo'")
    if (/undefined reference to|ld:\s*error|relocation truncated/i.test(raw)) {
        return applyHint(raw, { message: raw });
    }

    // Linker summary line (the real cause is usually the preceding line, already captured)
    if (/collect2\.exe?:?\s*error|ld returned \d+ exit status/i.test(raw)) {
        return null; // skip noise — the undefined-reference lines above carry the details
    }

    return null;
}

/** Parse a full log (multi-line) into a deduplicated list of friendly errors. */
export function parseBuildErrors(log: string): ParsedBuildError[] {
    const seen = new Set<string>();
    const out: ParsedBuildError[] = [];
    for (const line of log.split(/\r?\n/)) {
        const err = parseErrorLine(line);
        if (!err) continue;
        const key = `${err.file ?? ""}:${err.line ?? 0}:${err.message}`;
        if (seen.has(key)) continue;
        seen.add(key);
        out.push(err);
    }
    return out.slice(0, 20);
}
