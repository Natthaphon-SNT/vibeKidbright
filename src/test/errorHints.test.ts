import { describe, it, expect } from "vitest";
import { parseErrorLine, parseBuildErrors, type ParsedBuildError } from "../errorHints";

describe("parseErrorLine", () => {
    it("parses a GCC error with file, line and column", () => {
        const err = parseErrorLine(
            "D:/proj/main/main.c:12:5: error: expected ';' before 'return'"
        );
        expect(err).not.toBeNull();
        expect(err!.file).toBe("D:/proj/main/main.c");
        expect(err!.line).toBe(12);
        expect(err!.column).toBe(5);
        expect(err!.title).toBe("Missing semicolon");
        expect(err!.thaiHint).toContain(";");
    });

    it("parses a GCC fatal error without column", () => {
        const err = parseErrorLine("main.c:3: fatal error: foo.h: No such file or directory");
        expect(err).not.toBeNull();
        expect(err!.line).toBe(3);
        expect(err!.column).toBeUndefined();
        expect(err!.title).toBe("File or header not found");
    });

    it("maps undeclared identifiers", () => {
        const err = parseErrorLine("main.c:8:14: error: 'led_pin' undeclared (first use in this function)");
        expect(err!.title).toBe("Unknown variable or function");
    });

    it("maps undefined references to the linker hint", () => {
        const err = parseErrorLine("/usr/bin/ld: main.c:(.text+0x20): undefined reference to `sensor_read'");
        expect(err!.title).toBe("Missing source file or library");
    });

    it("flags smart quotes / non-ASCII characters", () => {
        const err = parseErrorLine(`main.c:5:12: error: stray '\\342' in program`);
        expect(err!.title).toBe("Non-English character in code");
    });

    it("parses CMake errors", () => {
        const err = parseErrorLine(
            "CMake Error at main/CMakeLists.txt:7 (target_link_libraries): Cannot find component: driver"
        );
        expect(err).not.toBeNull();
        expect(err!.file).toBe("main/CMakeLists.txt");
        expect(err!.line).toBe(7);
    });

    it("gives a kid-friendly hint for failed board connection", () => {
        const err = parseErrorLine("A fatal error occurred: Failed to connect to ESP32: No serial data received.");
        expect(err).not.toBeNull();
        expect(err!.title).toBe("Can't talk to the board");
        expect(err!.thaiHint).toContain("BOOT");
    });

    it("detects a busy COM port", () => {
        const err = parseErrorLine("serial.serialutil.SerialException: could not open port 'COM3': Permission denied(13)");
        expect(err!.title).toBe("COM port is busy");
    });

    it("returns null for normal progress lines", () => {
        expect(parseErrorLine("[4/12] Building C object CMakeFiles/x.dir/main.c.obj")).toBeNull();
        expect(parseErrorLine("Writing at 0x00010000... (45 %)")).toBeNull();
        expect(parseErrorLine("Hash of data verified.")).toBeNull();
        expect(parseErrorLine("")).toBeNull();
    });

    it("returns null for linker noise lines (details captured elsewhere)", () => {
        expect(parseErrorLine("collect2.exe: error: ld returned 1 exit status")).toBeNull();
    });

    it("falls back to a generic hint for unknown errors", () => {
        const err = parseErrorLine("main.c:99:1: error: something totally exotic happened");
        expect(err).not.toBeNull();
        expect(err!.title).toBe("Build error");
        expect(err!.thaiHint.length).toBeGreaterThan(0);
    });
});

describe("parseBuildErrors", () => {
    it("collects multiple unique errors from a full log", () => {
        const log = [
            "--- Starting Build & Flash ---",
            "[1/10] Building C object main/CMakeFiles/__idf_main.dir/main.c.obj",
            "FAILED: main/CMakeFiles/__idf_main.dir/main.c.obj",
            "main/main.c:10:5: error: expected ';' before '}' token",
            "main/main.c:15:9: error: 'x' undeclared (first use in this function)",
            "ninja: build stopped: subcommand failed.",
        ].join("\n");
        const errs = parseBuildErrors(log);
        expect(errs).toHaveLength(2);
        expect(errs[0].line).toBe(10);
        expect(errs[1].line).toBe(15);
    });

    it("deduplicates repeated identical errors", () => {
        const log = [
            "main.c:1:1: error: expected '=' before '<' token",
            "main.c:1:1: error: expected '=' before '<' token",
        ].join("\n");
        expect(parseBuildErrors(log)).toHaveLength(1);
    });

    it("caps output at 20 errors", () => {
        const lines: string[] = [];
        for (let i = 1; i <= 30; i++) {
            lines.push(`main.c:${i}:1: error: expected ';'`);
        }
        expect(parseBuildErrors(lines.join("\n"))).toHaveLength(20);
    });

    it("returns an empty list for clean logs", () => {
        const log = "[6/6] Linking CXX executable app.elf\nHash of data verified.";
        expect(parseBuildErrors(log)).toHaveLength(0);
    });

    it("always provides hints on every parsed error", () => {
        const errs: ParsedBuildError[] = parseBuildErrors(
            "main.c:1:1: error: expected ';'\nA fatal error occurred: Failed to connect to ESP32."
        );
        for (const e of errs) {
            expect(e.title).toBeTruthy();
            expect(e.thaiHint).toBeTruthy();
            expect(e.englishHint).toBeTruthy();
        }
    });
});
