import { describe, expect, it, vi } from "vitest";
import {
  executeBuildLifecycle,
  isBuildFlashSupported,
  runBuildForBoard,
  type BuildResult,
} from "../buildFlash";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("executeBuildLifecycle", () => {
  it("stays building until the command resolves, then succeeds", async () => {
    const command = deferred<void>();
    let result: BuildResult = "idle";
    let building = false;

    const pending = executeBuildLifecycle({
      run: () => command.promise,
      setResult: (next) => { result = next; },
      setBuilding: (next) => { building = next; },
      onFailure: vi.fn(),
    });

    expect(result).toBe("building");
    expect(building).toBe(true);

    command.resolve();
    await pending;

    expect(result).toBe("success");
    expect(building).toBe(false);
  });

  it("marks a rejected command as failed and never as success", async () => {
    const command = deferred<void>();
    const results: BuildResult[] = [];
    const onFailure = vi.fn();

    const pending = executeBuildLifecycle({
      run: () => command.promise,
      setResult: (next) => { results.push(next); },
      setBuilding: vi.fn(),
      onFailure,
    });

    command.reject(new Error("exit code 2"));
    await pending;

    expect(results).toEqual(["building", "failed"]);
    expect(onFailure).toHaveBeenCalledWith(expect.objectContaining({ message: "exit code 2" }));
  });
});

describe("board deployment routing", () => {
  it("runs ESP-IDF build/flash for KidBright32", async () => {
    const runEspIdf = vi.fn().mockResolvedValue(undefined);

    await expect(runBuildForBoard("kidbright32", runEspIdf)).resolves.toBe(true);
    expect(runEspIdf).toHaveBeenCalledOnce();
    expect(isBuildFlashSupported("kidbright32")).toBe(true);
  });

  it("never runs ESP-IDF build/flash for MiuAiPlus", async () => {
    const runEspIdf = vi.fn().mockResolvedValue(undefined);

    await expect(runBuildForBoard("miuaiplus", runEspIdf)).resolves.toBe(false);
    expect(runEspIdf).not.toHaveBeenCalled();
    expect(isBuildFlashSupported("miuaiplus")).toBe(false);
  });
});
