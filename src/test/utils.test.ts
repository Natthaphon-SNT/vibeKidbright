/**
 * utils.test.ts — Vitest unit tests for src/utils.ts
 * Run: npm test  or  npm run test:watch
 */

import { describe, it, expect } from "vitest";
import {
  formatBytes,
  sanitizeProjectName,
  getFileExtension,
  getBasename,
  truncate,
  isEspIdfSourceFile,
  isMarkdownFile,
  formatDuration,
  parseIdfVersion,
  isIdfVersionAtLeast,
  getMonacoLanguage,
} from "../utils";

// ── formatBytes ───────────────────────────────────────────────────────────────

describe("formatBytes", () => {
  it("returns '0 B' for 0", () => {
    expect(formatBytes(0)).toBe("0 B");
  });

  it("formats bytes correctly", () => {
    expect(formatBytes(512)).toBe("512 B");
  });

  it("formats kilobytes correctly", () => {
    expect(formatBytes(1024)).toBe("1 KB");
    expect(formatBytes(1536)).toBe("1.5 KB");
  });

  it("formats megabytes correctly", () => {
    expect(formatBytes(1048576)).toBe("1 MB");
  });

  it("respects decimal places", () => {
    expect(formatBytes(1500, 2)).toBe("1.46 KB");
  });
});

// ── sanitizeProjectName ───────────────────────────────────────────────────────

describe("sanitizeProjectName", () => {
  it("lowercases the name", () => {
    expect(sanitizeProjectName("MyProject")).toBe("myproject");
  });

  it("replaces spaces with underscores", () => {
    expect(sanitizeProjectName("my project")).toBe("my_project");
  });

  it("strips special characters", () => {
    // !@# → each becomes _ → then _ collapse + trim → "my-project"
    expect(sanitizeProjectName("my-project!@#")).toBe("my-project");
    // Mixed: spaces + special chars
    expect(sanitizeProjectName("hello world!")).toBe("hello_world");
  });

  it("collapses multiple underscores", () => {
    expect(sanitizeProjectName("my   project")).toBe("my_project");
  });

  it("trims leading/trailing underscores", () => {
    expect(sanitizeProjectName("  my_project  ")).toBe("my_project");
  });

  it("handles empty string", () => {
    expect(sanitizeProjectName("")).toBe("");
  });

  it("allows hyphens and underscores", () => {
    expect(sanitizeProjectName("my-project_v2")).toBe("my-project_v2");
  });
});

// ── getFileExtension ──────────────────────────────────────────────────────────

describe("getFileExtension", () => {
  it("returns extension without dot", () => {
    expect(getFileExtension("main.c")).toBe("c");
  });

  it("lowercases extension", () => {
    expect(getFileExtension("README.MD")).toBe("md");
  });

  it("handles multiple dots", () => {
    expect(getFileExtension("CMakeLists.txt.bak")).toBe("bak");
  });

  it("returns empty for no extension", () => {
    expect(getFileExtension("Makefile")).toBe("");
    expect(getFileExtension("sdkconfig")).toBe("");
  });

  it("handles paths with directories", () => {
    expect(getFileExtension("/home/user/main.c")).toBe("c");
    expect(getFileExtension("src/main/CMakeLists.txt")).toBe("txt");
  });
});

// ── getBasename ───────────────────────────────────────────────────────────────

describe("getBasename", () => {
  it("returns filename from Unix path", () => {
    expect(getBasename("/home/user/project/main.c")).toBe("main.c");
  });

  it("returns filename from Windows path", () => {
    expect(getBasename("C:\\Users\\user\\project\\main.c")).toBe("main.c");
  });

  it("handles filename with no directory", () => {
    expect(getBasename("main.c")).toBe("main.c");
  });

  it("strips trailing slashes", () => {
    expect(getBasename("/home/user/project/")).toBe("project");
  });
});

// ── truncate ──────────────────────────────────────────────────────────────────

describe("truncate", () => {
  it("does not truncate short strings", () => {
    expect(truncate("hello", 10)).toBe("hello");
  });

  it("truncates and appends ellipsis", () => {
    const result = truncate("hello world", 8);
    expect(result).toHaveLength(8);
    expect(result.endsWith("…")).toBe(true);
  });

  it("handles exact length boundary", () => {
    expect(truncate("hello", 5)).toBe("hello");
  });
});

// ── isEspIdfSourceFile ────────────────────────────────────────────────────────

describe("isEspIdfSourceFile", () => {
  it("recognizes C source files", () => {
    expect(isEspIdfSourceFile("main.c")).toBe(true);
    expect(isEspIdfSourceFile("driver.h")).toBe(true);
  });

  it("recognizes C++ source files", () => {
    expect(isEspIdfSourceFile("app.cpp")).toBe(true);
    expect(isEspIdfSourceFile("app.hpp")).toBe(true);
  });

  it("recognizes CMake files", () => {
    expect(isEspIdfSourceFile("CMakeLists.txt")).toBe(true);
    expect(isEspIdfSourceFile("module.cmake")).toBe(true);
  });

  it("recognizes sdkconfig and Kconfig", () => {
    expect(isEspIdfSourceFile("sdkconfig")).toBe(true);
    expect(isEspIdfSourceFile("Kconfig")).toBe(true);
  });

  it("rejects non-ESP-IDF files", () => {
    expect(isEspIdfSourceFile("README.md")).toBe(false);
    expect(isEspIdfSourceFile("package.json")).toBe(false);
  });
});

// ── isMarkdownFile ────────────────────────────────────────────────────────────

describe("isMarkdownFile", () => {
  it("recognizes .md files", () => {
    expect(isMarkdownFile("README.md")).toBe(true);
    expect(isMarkdownFile("/docs/guide.md")).toBe(true);
  });

  it("rejects non-markdown files", () => {
    expect(isMarkdownFile("main.c")).toBe(false);
    expect(isMarkdownFile("notes.txt")).toBe(false);
  });
});

// ── formatDuration ────────────────────────────────────────────────────────────

describe("formatDuration", () => {
  it("formats sub-second as decimal seconds", () => {
    expect(formatDuration(500)).toBe("0.5s");
  });

  it("formats exactly 1 second", () => {
    expect(formatDuration(1000)).toBe("1s");
  });

  it("formats seconds", () => {
    expect(formatDuration(30000)).toBe("30s");
  });

  it("formats minutes and seconds", () => {
    expect(formatDuration(65000)).toBe("1m 5s");
    expect(formatDuration(120000)).toBe("2m 0s");
  });
});

// ── parseIdfVersion ───────────────────────────────────────────────────────────

describe("parseIdfVersion", () => {
  it("parses a valid version", () => {
    expect(parseIdfVersion("v5.4.1")).toEqual({ major: 5, minor: 4, patch: 1 });
  });

  it("parses version without 'v' prefix", () => {
    expect(parseIdfVersion("5.4.0")).toEqual({ major: 5, minor: 4, patch: 0 });
  });

  it("returns null for invalid input", () => {
    expect(parseIdfVersion("not-a-version")).toBeNull();
    expect(parseIdfVersion("")).toBeNull();
  });

  it("handles version with trailing text", () => {
    expect(parseIdfVersion("v5.3.2-dirty")).toEqual({ major: 5, minor: 3, patch: 2 });
  });
});

// ── isIdfVersionAtLeast ───────────────────────────────────────────────────────

describe("isIdfVersionAtLeast", () => {
  const v540 = { major: 5, minor: 4, patch: 0 };
  const v541 = { major: 5, minor: 4, patch: 1 };
  const v550 = { major: 5, minor: 5, patch: 0 };
  const v500 = { major: 5, minor: 0, patch: 0 };

  it("returns true when version meets minimum exactly", () => {
    expect(isIdfVersionAtLeast(v540, 5, 4, 0)).toBe(true);
  });

  it("returns true when patch is higher", () => {
    expect(isIdfVersionAtLeast(v541, 5, 4, 0)).toBe(true);
  });

  it("returns true when minor is higher", () => {
    expect(isIdfVersionAtLeast(v550, 5, 4, 0)).toBe(true);
  });

  it("returns false when patch is lower", () => {
    expect(isIdfVersionAtLeast(v540, 5, 4, 1)).toBe(false);
  });

  it("returns false when minor is lower", () => {
    expect(isIdfVersionAtLeast(v500, 5, 4, 0)).toBe(false);
  });
});

// ── getMonacoLanguage ─────────────────────────────────────────────────────────

describe("getMonacoLanguage", () => {
  it("returns 'c' for .c and .h files", () => {
    expect(getMonacoLanguage("main.c")).toBe("c");
    expect(getMonacoLanguage("driver.h")).toBe("c");
  });

  it("returns 'cmake' for CMakeLists.txt", () => {
    expect(getMonacoLanguage("CMakeLists.txt")).toBe("cmake");
  });

  it("returns 'markdown' for .md files", () => {
    expect(getMonacoLanguage("README.md")).toBe("markdown");
  });

  it("returns 'json' for .json files", () => {
    expect(getMonacoLanguage("config.json")).toBe("json");
  });

  it("returns 'python' for .py files", () => {
    expect(getMonacoLanguage("build.py")).toBe("python");
  });

  it("returns 'ini' for sdkconfig and Kconfig", () => {
    expect(getMonacoLanguage("sdkconfig")).toBe("ini");
    expect(getMonacoLanguage("Kconfig")).toBe("ini");
  });

  it("returns 'plaintext' for unknown extensions", () => {
    expect(getMonacoLanguage("README")).toBe("plaintext");
    expect(getMonacoLanguage("unknown.xyz")).toBe("plaintext");
  });
});
