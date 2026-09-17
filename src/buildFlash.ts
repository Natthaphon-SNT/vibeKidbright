export type BuildResult = "idle" | "building" | "success" | "failed";
export type BoardType = "kidbright32" | "miuaiplus";

export const MIUAI_DEPLOYMENT_UNAVAILABLE =
  "MiuAiPlus deployment is not yet supported — coming soon";

export function isBuildFlashSupported(board: BoardType): boolean {
  return board === "kidbright32";
}

/** Enforce the board routing policy separately from the button's disabled state. */
export async function runBuildForBoard(
  board: BoardType,
  runEspIdf: () => Promise<unknown>,
): Promise<boolean> {
  if (!isBuildFlashSupported(board)) return false;
  await runEspIdf();
  return true;
}

interface BuildLifecycle {
  run: () => Promise<unknown>;
  setResult: (result: BuildResult) => void;
  setBuilding: (building: boolean) => void;
  onFailure: (error: unknown) => void;
}

/** Keep the UI in the building state until the backend process actually exits. */
export async function executeBuildLifecycle({
  run,
  setResult,
  setBuilding,
  onFailure,
}: BuildLifecycle): Promise<void> {
  setBuilding(true);
  setResult("building");

  try {
    await run();
    setResult("success");
  } catch (error) {
    setResult("failed");
    onFailure(error);
  } finally {
    setBuilding(false);
  }
}
