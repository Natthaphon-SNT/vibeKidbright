import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface ProjectReview {
    projectPath: string;
    projectName: string;
    previousProjectPath: string | null;
    files: string[];
    messageId: string;
    revision: string;
}

function normalizePath(path: string) {
    const normalized = path.replace(/\\/g, "/").replace(/\/+$/, "");
    return /^(?:[a-z]:\/|\/\/)/i.test(normalized) ? normalized.toLowerCase() : normalized;
}

function isVisibleProjectFile(path: string) {
    const segments = path.replace(/\\/g, "/").toLowerCase().split("/")
        .filter(segment => segment !== "" && segment !== ".");
    if (segments.includes(".vibekidbright")) return false;
    return segments.length !== 1 || !["kidbright-project.json", "kidbright_compatibility.md"].includes(segments[0]);
}

export default function ProjectCreationReview({ isBusy, projectDir, hasUnsavedChanges = false }: {
    isBusy: boolean;
    projectDir: string;
    hasUnsavedChanges?: boolean;
}) {
    const [reviews, setReviews] = useState<ProjectReview[]>([]);
    const [workingPath, setWorkingPath] = useState<string | null>(null);
    const [errors, setErrors] = useState<Record<string, string>>({});
    const [loadError, setLoadError] = useState("");
    const refreshRef = useRef<() => void>(() => {});

    useEffect(() => {
        let mounted = true;
        let revision = 0;
        const refresh = async () => {
            const requestedAt = revision;
            try {
                const pending = await invoke<ProjectReview[]>("get_pending_project_reviews");
                // An event received during the read already contains newer state.
                if (mounted && requestedAt === revision) {
                    setReviews(pending);
                    setLoadError("");
                }
            } catch (error) {
                if (mounted) setLoadError(`Could not load project review: ${String(error)}`);
            }
        };
        refreshRef.current = () => { void refresh(); };

        const pendingListener = listen<string>("ai-project-review-pending", (event) => {
            if (!mounted) return;
            try {
                const review = JSON.parse(event.payload) as ProjectReview;
                if (!review.projectPath || !Array.isArray(review.files)) return;
                revision++;
                setLoadError("");
                setReviews(current => [
                    ...current.filter(item => item.projectPath !== review.projectPath),
                    review,
                ]);
            } catch { /* Ignore an incomplete event; the saved review remains available. */ }
        });
        const resolvedListener = listen<string>("ai-project-review-resolved", (event) => {
            if (!mounted) return;
            try {
                const result = JSON.parse(event.payload) as { projectPath: string };
                revision++;
                setReviews(current => current.filter(item => item.projectPath !== result.projectPath));
            } catch { /* Leave the review available if the event cannot be read. */ }
        });

        // Listen first so a project created while the initial read runs is retained.
        void Promise.all([pendingListener, resolvedListener]).then(() => {
            if (mounted) void refresh();
        }).catch(error => {
            if (mounted) {
                setLoadError(`Could not listen for project review: ${String(error)}`);
                void refresh();
            }
        });
        return () => {
            mounted = false;
            refreshRef.current = () => {};
            void pendingListener.then(unlisten => unlisten()).catch(() => {});
            void resolvedListener.then(unlisten => unlisten()).catch(() => {});
        };
    }, []);

    const resolve = async (projectPath: string, action: "keep" | "undo") => {
        if (isBusy || hasUnsavedChanges || workingPath) return;
        setWorkingPath(projectPath);
        setErrors(current => ({ ...current, [projectPath]: "" }));
        try {
            const review = reviews.find(item => item.projectPath === projectPath);
            if (!review?.revision) throw new Error("Reload the current project review before approving it.");
            await invoke(action === "keep" ? "keep_ai_project" : "undo_ai_project", { projectPath, expectedRevision: review.revision });
            setReviews(current => current.filter(item => item.projectPath !== projectPath));
        } catch (error) {
            setErrors(current => ({ ...current, [projectPath]: String(error) }));
        } finally {
            setWorkingPath(null);
        }
    };

    const visibleReviews = reviews
        .filter(review => projectDir === "." || normalizePath(review.projectPath) === normalizePath(projectDir))
        .map(review => ({ ...review, files: review.files.filter(isVisibleProjectFile) }));
    if (visibleReviews.length === 0 && !loadError) return null;

    return (
        <div
            className="shrink-0 max-h-[40%] overflow-y-auto px-3 py-2 space-y-2"
            style={{ borderTop: "1px solid var(--border-color)", backgroundColor: "var(--bg-sidebar)" }}
            aria-label="Project creation review"
        >
            {loadError && (
                <div className="text-xs" role="alert">
                    <span>{loadError} </span>
                    <button className="underline" onClick={() => refreshRef.current()}>Retry</button>
                </div>
            )}
            {visibleReviews.map(review => (
                <section
                    key={review.projectPath}
                    className="rounded-lg p-3 text-xs space-y-2"
                    style={{ border: "1px solid var(--accent)", backgroundColor: "var(--bg-panel)", color: "var(--text-primary)" }}
                    aria-label={`Review project ${review.projectName}`}
                >
                    <div className="font-semibold">Review new project: {review.projectName}</div>
                    <p style={{ color: "var(--text-secondary)" }}>
                        Keep confirms this project. Undo moves the unchanged generated project into a recovery folder and {review.previousProjectPath ? "returns to your previous project." : "closes this project."}
                    </p>
                    <div className="break-all" style={{ color: "var(--text-secondary)" }}>{review.projectPath}</div>
                    <details>
                        <summary className="cursor-pointer">{review.files.length} {review.files.length === 1 ? "file" : "files"} created</summary>
                        <ul className="mt-1 space-y-0.5 max-h-28 overflow-y-auto font-mono" style={{ color: "var(--text-secondary)" }}>
                            {review.files.map(file => <li className="break-all" key={file}>{file}</li>)}
                        </ul>
                    </details>
                    {errors[review.projectPath] && <p role="alert" style={{ color: "var(--accent)" }}>{errors[review.projectPath]}</p>}
                    <div className="flex gap-2">
                        <button
                            type="button"
                            className="rounded px-4 py-1.5 font-semibold disabled:opacity-50 disabled:cursor-not-allowed"
                            style={{ backgroundColor: "var(--accent)", color: "var(--text-on-primary)" }}
                            disabled={isBusy || hasUnsavedChanges || workingPath !== null}
                            onClick={() => { void resolve(review.projectPath, "keep"); }}
                        >Keep</button>
                        <button
                            type="button"
                            className="rounded px-4 py-1.5 font-semibold disabled:opacity-50 disabled:cursor-not-allowed"
                            style={{ border: "1px solid var(--border-color)", backgroundColor: "var(--bg-hover)", color: "var(--text-primary)" }}
                            disabled={isBusy || hasUnsavedChanges || workingPath !== null}
                            onClick={() => { void resolve(review.projectPath, "undo"); }}
                        >Undo</button>
                    </div>
                    {isBusy && <p style={{ color: "var(--text-secondary)" }}>Available when AI finishes.</p>}
                    {hasUnsavedChanges && <p style={{ color: "var(--text-secondary)" }}>Save or discard your unsaved changes before choosing Keep or Undo.</p>}
                    {workingPath === review.projectPath && <p role="status">Saving your choice…</p>}
                </section>
            ))}
        </div>
    );
}
