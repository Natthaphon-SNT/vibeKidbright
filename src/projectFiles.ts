export interface ProjectFileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  children?: ProjectFileEntry[];
}

interface ProjectPath {
  value: string;
  windows: boolean;
}

function cleanSegments(path: string): string[] | null {
  if (path.includes("\0")) return null;
  const segments = path.replace(/\\/g, "/").split("/").filter(Boolean);
  if (segments.includes("..")) return null;
  return segments.filter(segment => segment !== ".");
}

function absolutePath(path: string): ProjectPath | null {
  const normalized = path.replace(/\\/g, "/");
  const segments = cleanSegments(normalized);
  if (!segments) return null;
  if (/^[a-z]:\//i.test(normalized)) {
    return { value: segments.join("/").toLowerCase(), windows: true };
  }
  if (normalized.startsWith("//") && segments.length >= 2) {
    return { value: `//${segments.join("/").toLowerCase()}`, windows: true };
  }
  if (normalized.startsWith("/") && !normalized.startsWith("//")) {
    return { value: `/${segments.join("/")}`, windows: false };
  }
  return null;
}

function resolvePath(project: ProjectPath, path: string): ProjectPath | null {
  const segments = cleanSegments(path);
  if (!segments || segments.length === 0) return null;
  const normalized = path.replace(/\\/g, "/");
  const absolute = absolutePath(normalized);
  if (absolute) return absolute.windows === project.windows ? absolute : null;
  // A malformed absolute or drive-relative path must not become a relative file.
  if (normalized.startsWith("/") || /^[a-z]:/i.test(normalized)) return null;
  const relative = segments.join("/");
  return { value: `${project.value.replace(/\/$/, "")}/${project.windows ? relative.toLowerCase() : relative}`, windows: project.windows };
}

function visibleSegments(segments: string[], windows: boolean): boolean {
  return segments.every(segment => {
    const name = windows ? segment.toLowerCase() : segment;
    return !name.startsWith(".")
      && name !== "build"
      && !name.startsWith("cmake-build-")
      && name !== (windows ? "cmakelists.txt" : "CMakeLists.txt")
      && name !== (windows ? "kconfig" : "Kconfig")
      && name !== (windows ? "kconfig.projbuild" : "Kconfig.projbuild")
      && name !== "idf_component.yml"
      && !/^sdkconfig(?:\.|$)/.test(name);
  });
}

function projectRelativePath(project: ProjectPath, path: string): string[] | null {
  const resolved = resolvePath(project, path);
  if (!resolved) return null;
  const boundary = `${project.value.replace(/\/$/, "")}/`;
  if (!resolved.value.startsWith(boundary)) return null;
  return resolved.value.slice(boundary.length).split("/");
}

/** A lexical UI policy; the backend remains responsible for filesystem access. */
export function isLearnerEditablePath(projectDir: string, path: string): boolean {
  const project = absolutePath(projectDir);
  if (!project) return false;
  const relative = projectRelativePath(project, path);
  if (!relative || relative.length < 2 || relative[0] !== "main") return false;
  // Win32 aliases/alternate streams must not bypass a protected filename.
  if (project.windows && relative.some(segment => /[:<>"|?*]|[. ]$/.test(segment))) return false;
  if (relative.length === 2 && relative[1] === "kb_board.h") return false;
  return visibleSegments(relative, project.windows);
}

/** Return a new tree containing only the actual root main directory and user files. */
export function learnerProjectTree(entries: ProjectFileEntry[], projectDir: string): ProjectFileEntry[] {
  const project = absolutePath(projectDir);
  if (!project) return [];
  const mainPath = `${project.value.replace(/\/$/, "")}/main`;
  const copyChildren = (children: ProjectFileEntry[], parentPath: string): ProjectFileEntry[] => children.flatMap(entry => {
    const resolved = resolvePath(project, entry.path);
    const prefix = `${parentPath}/`;
    if (!resolved || !resolved.value.startsWith(prefix) || resolved.value.slice(prefix.length).includes("/")) return [];
    if (!isLearnerEditablePath(projectDir, entry.path)) return [];
    if (entry.is_dir) return [{ ...entry, children: copyChildren(entry.children ?? [], resolved.value) }];
    return [{ ...entry }];
  });
  return entries.flatMap(entry => {
    const resolved = resolvePath(project, entry.path);
    const name = project.windows ? entry.name.toLowerCase() : entry.name;
    if (!entry.is_dir || name !== "main" || resolved?.value !== mainPath) return [];
    return [{ ...entry, children: copyChildren(entry.children ?? [], mainPath) }];
  });
}
