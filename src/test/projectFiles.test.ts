import { describe, expect, it } from "vitest";
import { isLearnerEditablePath, learnerProjectTree, type ProjectFileEntry } from "../projectFiles";

const root = "D:\\projects\\s1_sound";
const file = (path: string): ProjectFileEntry => ({ name: path.replace(/\\/g, "/").split("/").pop()!, path, is_dir: false });
const directory = (path: string, children?: ProjectFileEntry[]): ProjectFileEntry => ({ ...file(path), is_dir: true, children });

describe("learner project files", () => {
  it("reduces the generated project to main/main.c without modifying the original tree", () => {
    const main = directory(`${root}\\main`, [file(`${root}\\main\\CMakeLists.txt`), file(`${root}\\main\\kb_board.h`), file(`${root}\\main\\main.c`)]);
    const entries = [
      directory(`${root}\\components`, [directory(`${root}\\components\\kb_reference`, [file(`${root}\\components\\kb_reference\\kb_core.c`)])]),
      main, file(`${root}\\CMakeLists.txt`), file(`${root}\\sdkconfig`), file(`${root}\\kidbright-project.json`),
      file(`${root}\\KIDBRIGHT_COMPATIBILITY.md`), directory(`${root}\\.vibekidbright`, [file(`${root}\\.vibekidbright\\project.json`)]),
    ];
    const original = structuredClone(entries);
    const result = learnerProjectTree(entries, root);
    expect(result).toEqual([{ ...main, children: [file(`${root}\\main\\main.c`)] }]);
    expect(entries).toEqual(original);
    expect(result[0]).not.toBe(main);
    expect(result[0].children?.[0]).not.toBe(main.children?.[2]);
  });

  it("retains custom sources, headers, assets and nested user documents inside main", () => {
    const sources = [file("main/main.c"), file("main/controller.cpp"), file("main/controller.h"), file("main/settings.json"), file("main/README.md")];
    const nested = directory("main/assets", [file("main/assets/font.bin"), file("main/assets/logo.png"), file("main/assets/kb_board.h"), file("main/assets/kidbright-project.json")]);
    const main = directory("main", [...sources, nested]);
    expect(learnerProjectTree([main], root)).toEqual([main]);
    for (const entry of [...sources, ...nested.children!]) expect(isLearnerEditablePath(root, entry.path)).toBe(true);
  });

  it("hides build/configuration and dot paths at every depth", () => {
    const excluded = [
      "main/CMakeLists.txt", "main/sdkconfig", "main/sdkconfig.defaults", "main/sdkconfig.defaults.esp32", "main/sdkconfig.old",
      "main/Kconfig", "main/Kconfig.projbuild", "main/idf_component.yml", "main/kb_board.h", "main/.env",
      "main/build/generated.c", "main/cmake-build-debug/generated.c", "main/sub/.vibekidbright/data.json", "main/sub/CMakeLists.txt",
    ];
    for (const path of excluded) expect(isLearnerEditablePath(root, path), path).toBe(false);
    const main = directory("main", [
      file("main/main.c"), file("main/CMakeLists.txt"), directory("main/build", [file("main/build/generated.c")]),
      directory("main/.vibekidbright", [file("main/.vibekidbright/data.json")]),
    ]);
    expect(learnerProjectTree([main], root)[0].children).toEqual([file("main/main.c")]);
  });

  it("shows an empty main and never substitutes components when main is missing", () => {
    const empty = directory(`${root}/main`);
    expect(learnerProjectTree([empty], root)).toEqual([{ ...empty, children: [] }]);
    expect(learnerProjectTree([directory(`${root}/components`, [file(`${root}/components/main.c`)])], root)).toEqual([]);
    expect(learnerProjectTree([{ ...directory(`${root}/components`), name: "main" }], root)).toEqual([]);
    expect(learnerProjectTree([file(`${root}/main`)], root)).toEqual([]);
  });

  it.each([
    "mainly/main.c", "components/kb_reference/kb_core.c", "main", "../main/main.c", "main/../main/main.c", "main/sub/../../outside.c",
    "D:/projects/s1_sound_other/main/main.c", "D:/projects/another/main/main.c", "E:/projects/s1_sound/main/main.c", "/outside/main/main.c",
    "D:main/main.c", "main/\0main.c",
  ])("rejects external, traversal or non-file main path %s", path => {
    expect(isLearnerEditablePath(root, path)).toBe(false);
  });

  it("matches Windows drive and UNC paths without case or separator differences", () => {
    expect(isLearnerEditablePath("d:/PROJECTS/s1_sound/", "D:\\projects\\S1_SOUND\\MAIN\\MAIN.C")).toBe(true);
    expect(isLearnerEditablePath(root, ".\\main\\sub\\control.c")).toBe(true);
    expect(isLearnerEditablePath(root, "MAIN\\KB_BOARD.H")).toBe(false);
    expect(isLearnerEditablePath(root, "MAIN\\CMAKELISTS.TXT")).toBe(false);
    expect(isLearnerEditablePath(root, "main/kb_board.h.")).toBe(false);
    expect(isLearnerEditablePath(root, "main/kb_board.h ")).toBe(false);
    expect(isLearnerEditablePath(root, "main/CMakeLists.txt:extra")).toBe(false);
    expect(isLearnerEditablePath("\\\\server\\share\\project", "//SERVER/SHARE/PROJECT/Main/main.c")).toBe(true);
    expect(isLearnerEditablePath("\\\\server\\share\\project", "//server/share/project_other/main/main.c")).toBe(false);
    const main = directory("D:/PROJECTS/S1_SOUND/MAIN", [file("D:/projects/s1_sound/main/main.c")]);
    expect(learnerProjectTree([main], root)).toEqual([main]);
  });

  it("preserves case sensitivity for POSIX project paths", () => {
    expect(isLearnerEditablePath("/projects/example", "/projects/example/main/main.c")).toBe(true);
    expect(isLearnerEditablePath("/projects/example", "/projects/EXAMPLE/main/main.c")).toBe(false);
    expect(isLearnerEditablePath("/projects/example", "/projects/example/MAIN/main.c")).toBe(false);
    expect(learnerProjectTree([directory("/projects/example/MAIN", [file("/projects/example/MAIN/main.c")])], "/projects/example")).toEqual([]);
  });

  it("does not accept an entry outside its actual parent even when its label is plausible", () => {
    const main = directory("main", [file("components/wrong.c"), directory("main/sub", [file("main/misplaced.c"), file("main/sub/real.c")])]);
    expect(learnerProjectTree([main], root)[0].children).toEqual([directory("main/sub", [file("main/sub/real.c")])]);
    expect(learnerProjectTree([main], ".")).toEqual([]);
    expect(isLearnerEditablePath(".", "main/main.c")).toBe(false);
  });
});
