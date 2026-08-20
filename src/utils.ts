/**
 * utils.ts — Pure utility functions shared across the app.
 * These are intentionally free of Tauri/React dependencies so they
 * can be tested with vitest without mocking the desktop runtime.
 */

/**
 * Format bytes into a human-readable string (e.g. 1536 → "1.5 KB").
 */
export function formatBytes(bytes: number, decimals = 1): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const dm = Math.max(0, decimals);
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

/**
 * Sanitize a project name: strips special characters, trims whitespace,
 * replaces spaces with underscores, lower-cases.
 */
export function sanitizeProjectName(name: string): string {
  return name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]/g, "_")
    .replace(/_+/g, "_")
    .replace(/^_|_$/g, "");
}

/**
 * Extract the file extension from a path (without the dot).
 * Returns empty string for paths with no extension.
 */
export function getFileExtension(path: string): string {
  const match = path.match(/\.([^./\\]+)$/);
  return match ? match[1].toLowerCase() : "";
}

/**
 * Get the basename from a file path (cross-platform).
 */
export function getBasename(path: string): string {
  return path.replace(/[/\\]+$/, "").split(/[/\\]/).pop() ?? path;
}

/**
 * Truncate a string to maxLength characters, appending "…" if needed.
 */
export function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength - 1) + "…";
}

/**
 * Returns true if the given filename is a C/C++ or CMake source file
 * that might be part of an ESP-IDF project.
 */
export function isEspIdfSourceFile(filename: string): boolean {
  const ext = getFileExtension(filename);
  return ["c", "h", "cpp", "hpp", "cmake"].includes(ext) ||
    filename === "CMakeLists.txt" ||
    filename === "sdkconfig" ||
    filename === "Kconfig";
}

/**
 * Detect whether a file path points to a Markdown file.
 */
export function isMarkdownFile(path: string): boolean {
  return getFileExtension(path) === "md";
}

/**
 * Format a duration in milliseconds into a human-readable string.
 * e.g. 65000 → "1m 5s", 30000 → "30s", 500 → "0.5s"
 */
export function formatDuration(ms: number): string {
  if (ms < 1000) return `${(ms / 1000).toFixed(1)}s`;
  const totalSeconds = Math.round(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes === 0) return `${seconds}s`;
  return `${minutes}m ${seconds}s`;
}

/**
 * Parse the ESP-IDF version from a `version.txt` file content string.
 * Returns null if the version string is not valid.
 */
export function parseIdfVersion(versionText: string): { major: number; minor: number; patch: number } | null {
  const match = versionText.trim().match(/^v?(\d+)\.(\d+)\.(\d+)/);
  if (!match) return null;
  return {
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: parseInt(match[3], 10),
  };
}

/**
 * Check if an ESP-IDF version meets the minimum required version.
 */
export function isIdfVersionAtLeast(
  version: { major: number; minor: number; patch: number },
  minMajor: number,
  minMinor: number,
  minPatch = 0
): boolean {
  if (version.major !== minMajor) return version.major > minMajor;
  if (version.minor !== minMinor) return version.minor > minMinor;
  return version.patch >= minPatch;
}

/**
 * Determine the appropriate icon/language ID for Monaco editor from a filename.
 */
export function getMonacoLanguage(filename: string): string {
  const ext = getFileExtension(filename);
  const map: Record<string, string> = {
    c: "c",
    h: "c",
    cpp: "cpp",
    hpp: "cpp",
    md: "markdown",
    json: "json",
    yaml: "yaml",
    yml: "yaml",
    toml: "toml",
    py: "python",
    sh: "shell",
    cmake: "cmake",
    txt: "plaintext",
  };
  if (filename === "CMakeLists.txt") return "cmake";
  if (filename === "sdkconfig" || filename === "Kconfig") return "ini";
  return map[ext] ?? "plaintext";
}
