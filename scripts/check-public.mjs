import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { basename, extname, join, relative } from "node:path";

const bundleMode = process.argv.includes("--bundle");
const bundleRoot = "resources/knowledge_base";

function listFilesRecursively(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    return entry.isDirectory() ? listFilesRecursively(path) : [path];
  });
}

const publishableFiles = bundleMode
  ? listFilesRecursively(bundleRoot)
  : execFileSync(
      "git",
      ["ls-files", "--cached", "--others", "--exclude-standard", "-z"],
      { encoding: "utf8" },
    ).split("\0").filter((file) => file && existsSync(file));

const allowedHomeNames = new Set([
  "user",
  "username",
  "name",
  "runner",
  "<user>",
  "<username>",
]);

const secretExtensions = new Set([".pfx", ".p12", ".pem", ".key", ".sqlite", ".db"]);
const bundleExtensions = new Set([
  ".c", ".h", ".jpeg", ".jpg", ".md", ".pdf", ".png", ".txt", ".webp",
]);
const textExtensions = new Set([
  ".c", ".cc", ".cpp", ".css", ".h", ".html", ".js", ".json", ".jsx",
  ".md", ".mjs", ".ps1", ".rs", ".sh", ".toml", ".ts", ".tsx", ".txt",
  ".yaml", ".yml",
]);

const errors = [];

for (const file of publishableFiles) {
  const lowerName = basename(file).toLowerCase();
  const extension = extname(file).toLowerCase();

  if (
    secretExtensions.has(extension) ||
    lowerName === ".embeddings.json" ||
    lowerName.endsWith(".backup") ||
    lowerName === ".env" ||
    (lowerName.startsWith(".env.") && lowerName !== ".env.example") ||
    lowerName.startsWith("cert_base64")
  ) {
    errors.push(`${file}: sensitive/local file must not be tracked`);
    continue;
  }

  if (bundleMode && !bundleExtensions.has(extension)) {
    errors.push(`${relative(bundleRoot, file)}: file type is not allowed in the app bundle`);
    continue;
  }

  if (!textExtensions.has(extension) || file.endsWith(".embeddings.json")) continue;

  let content;
  try {
    content = readFileSync(file, "utf8");
  } catch {
    continue;
  }

  const pathPatterns = [
    /[A-Za-z]:[\\/]+Users[\\/]+([^\\/\s"']+)/gi,
    /(?<![:A-Za-z0-9.])\/Users\/([^/\s"']+)/g,
    /(?<![:A-Za-z0-9.])\/home\/([^/\s"']+)/g,
  ];

  for (const pattern of pathPatterns) {
    for (const match of content.matchAll(pattern)) {
      if (!allowedHomeNames.has(match[1].toLowerCase())) {
        errors.push(`${file}: machine-specific home path (${match[0]})`);
      }
    }
  }

  const secretPatterns = [
    /-----BEGIN(?: [A-Z]+)? PRIVATE KEY-----/g,
    /\bsk-(?:proj-)?[A-Za-z0-9_-]{20,}\b/g,
    /\bAIza[0-9A-Za-z_-]{30,}\b/g,
    /\bgh[pousr]_[A-Za-z0-9]{20,}\b/g,
  ];

  for (const pattern of secretPatterns) {
    if (pattern.test(content)) {
      errors.push(`${file}: possible credential or private key`);
    }
  }
}

if (errors.length > 0) {
  console.error("Public-readiness check failed:\n" + errors.map((item) => `- ${item}`).join("\n"));
  process.exit(1);
}

const scope = bundleMode ? "bundle resource" : "tracked/unignored";
console.log(`Public-readiness check passed (${publishableFiles.length} ${scope} files scanned).`);
