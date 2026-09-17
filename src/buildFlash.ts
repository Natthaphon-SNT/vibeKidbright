export type BuildResult = "idle" | "building" | "success" | "failed";

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
