# Compiler validation and approval boundaries

Updated 2026-10-18. The installed, configured `esp_idf::run_idf_command` route remains the build authority. The former separate toolchain-build implementation delegates to this route, so selected SDK/Python/environment and review gates are shared.

## Controlled SDK processes

SDK and user-terminal execution acquire a workspace lease before build preparation. Conflicting writes/builds, including nested workspace paths, are rejected. Cancellation is scoped to an owned operation, never an arbitrary PID. Configuration `sdk_execution.timeout_seconds` defaults to 1800, accepts 1..7200, and has native get/set commands. The UI offers Stop build. Blocking preparation retains its lease even if the async caller drops. A stale build tree is moved to a same-parent recovery container instead of recursively deleted.

On Windows the process starts suspended and hidden, is assigned to a kill-on-close Job Object, then resumes. Containment/resume failures fail closed. Closing the job terminates contained descendants before pipe readers are joined. Unix uses a process group; that path is not exercised on this Windows host. Stdout/stderr drain concurrently, with bounded 64 KiB tails and UTF-8 boundary handling. Actual exit code, cancellation and timeout never report success. Native tests include a real Python parent/grandchild, inherited pipes, timeout, cancellation, workspace exclusion and bounded output. This does not yet control every toolchain downloader/setup, serial monitor or ADB subprocess.

[Microsoft Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects) documents tree containment and kill-on-close. The stable Rust Windows API does not expose the primary thread handle: contained threads are resumed using the native ToolHelp/Thread API after assignment. This implementation uses `windows-sys = 0.61.2`.

AI `run_command` remains limited to exact `idf.py build`, `idf.py size`, `idf.py --version`, with no extra arguments/shell operators. Flash/Monitor/Clean remain user controls. `install_idf_library` now stages a JSON-compatible YAML manifest for review, never immediately spawns idf.py or changes dependencies. Existing arbitrary YAML must be read and proposed completely through write_file; nothing is silently rewritten.

## Structured diagnostics and bounded repair

The native diagnostics parser classifies compiler, dependency/header, linker, CMake and build-system evidence, including severity, optional file/line/column and bounded messages. Existing friendly frontend hints remain. `compiler_diagnostics` exports this evidence only after the private-context gate, rather than putting private build logs into an automatic user-message prompt.

A user starts Compiler Repair from the build errors. Native session maximum is explicitly 1..3; the UI defaults to 3 and offers 1..3 before the first application. State flow:

`awaiting_proposal -> awaiting_approval -> applying -> building -> succeeded | failed | cancelled`

Interrupted application/build becomes `recovery_required`; reaching the approved attempt limit after rollback becomes `exhausted`. Failed/applied attempts must be reviewed or rolled back before another proposed attempt. Proposal and session revisions include the complete state. The model has no apply tool. The user must inspect the per-file diffs, select the approval checkbox, and press Apply this patch & verify build for each attempt. Old approval cannot authorize a later revision. Generic per-file/message Keep/Undo cannot bypass an active repair's grouped review.

A repair applies only saved project main/components C/H/CMake files via the durable change journal, then runs the configured SDK build, without flashing. Repeated unsuccessful complete patch fingerprints are persisted/rejected. Attempts and failure fingerprints survive restart. Cancellation waits for native process termination; source remains recoverable until explicit Undo. Undo uses exact-content checks and preserves later user work. The current workflow is user-paced: it does not automatically call a paid/local LLM repeatedly. A real-provider multi-attempt repair success rate has not been measured.

See [safety/recovery](11-safety-recovery.md), [Block pilot](10-block-to-c.md), and the final verification record. Compilation, upload and physical behavior are separate evidence classes.

AI Stop/consent revocation also drops the AI-owned SDK future. Its cancellation guard signals only its own workspace job, while the blocking worker retains file exclusion until native cleanup finishes. A regression runs a real child and checks that the native worker terminates; this does not cancel unrelated user-operated jobs.


## Revision-bound frontend consent

CompilerRepairReview now stores approval for an exact project/revision pair and consumes it as Apply starts. A failed application returning the same revision requires another checkbox approval. This replaces an unbound boolean cleared in a later effect. Frontend regressions cover retries and equal revision labels across projects. Native per-attempt authorization and maximum 1..3 remain unchanged. These tests do not establish actual model repair quality. See 22-evidence-integrity-report.md.


## Capability and restart evidence

The real-file repair attempt regression now reopens serialized session/journal representations between failed attempts and verifies duplicate rejection, exact rollback and an unrelated header across the three-attempt limit. This is persisted-store simulation, not power-loss/OS reboot or real-model repair. Capability matrix mechanical observations keep these safety checks separate from unexecuted model-dependent Build-aware judgments.
