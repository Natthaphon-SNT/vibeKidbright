# Thai instruction to verifiable ESP-IDF: next capability milestone

Updated 2026-10-19. This milestone follows the user's three capability levels. Passing infrastructure checks, increasing test counts, or adding Block IDs does not satisfy it. Board/Block expansion and an RC are deferred until credible evidence supports all three levels. The production embedding model and historical NSIS 3.7.3 remain unchanged.

| Level | Required observable behavior | Evidence needed before passing | Current status |
|---|---|---|---|
| 1 Knowledge-aware | Answer Thai, English and mixed technical requests using appropriate ESP-IDF references; distinguish SDK contracts from local helper behavior and unknown hardware compatibility | Actual Vibe Coder/native provider trace, original query and retrieval inputs/results, source/version hashes, correct file/line or byte ranges, and independent claim-by-claim relevance/support review | NOT_EVALUATED with a real LLM; retrieval and source-integrity infrastructure exist |
| 2 Project-aware | Explain the actual entry point, C declarations/definitions/includes, CMake source registration and component dependencies; preserve the selected project and board | Real project/file tools and arguments/results, before/after source manifests, correct explanation checked against the fixture; no unsupported semantic or dependency claims | NOT_EVALUATED with a real LLM; syntax/semantic components have separately scoped prior evidence |
| 3 Build-aware | Turn the Thai request into a reviewed patch, build that exact revision, explain real diagnostics, propose a bounded repair and recover safely | Each explicit user approval bound to its revision, unchanged sources before approval, configured SDK/version/target/command/exit status, source/CDB/firmware hashes, actual failure and rebuild, cancellation and Undo evidence | NOT_EVALUATED with a real LLM; deterministic C builds are prior infrastructure evidence |

No single delay example certifies general ESP-IDF competence. Start with the representative workflow below, then repeat independently authored classroom tasks in all three languages, including missing evidence and version/board incompatibility. Publish the task inventory and acceptance rules before collecting model outputs. Keep teacher/reviewer identities and raw traces private unless separately authorized. Do not derive a success percentage from an incomplete or assistant-graded run.

The current bundled corpus has useful examples and local helpers, but not the complete official ESP-IDF/FreeRTOS documentation. A helper implementation is evidence of that helper's behavior, not authoritative coverage of all SDK APIs. Missing reference evidence must remain a failure/abstention or an explicit limitation. This increment adds no documentation corpus and changes no retrieval weights, frozen labels or model artifacts.

## One connected Thai workflow

`real-vibe-coder-cases.json` declares `thai-delay-existing-project-v1`. Use the existing generated public fixture, `kidbright32_v13`, target `esp32`, with the prior ESP-IDF 5.5.4 configuration. These constraints do not certify other boards or SDK versions.

1. Explain `kb_delay_ms` from retrieved source, including input/output units and limits. Check the returned evidence and factual support separately.
2. Read the open project's `main/main.c`, `main/CMakeLists.txt`, and `kb_reference` declaration/implementation. Explain the actual call chain and component dependencies. Do not edit, build, switch projects or manufacture dependency information.
3. In the same conversation, propose a loop with a 100 ms delay per iteration, retaining the current profile. Explain tick conversion and scheduler limitations; do not promise an exact physical 100 ms period. Stop with a reviewable diff. Saved source hashes must remain unchanged.
4. The user reviews all files and explicitly approves that revision in the app. Record approval separately from the model's request or the presence of a review button. Build through the configured ESP-IDF runner without Flash. Tie raw exit status and artifact hashes to the approved source snapshot; a completion banner alone is insufficient.
5. A controlled public-fixture compiler fault also requires review/authorization before application. Record the failing revision and real diagnostic path, line, column and severity. Ask AI to explain and propose a correction. Each applied attempt needs a separate approval, maximum three attempts, and its own actual rebuild. Record repeated-patch rejection and cancellation independently.
6. With the user's Undo/recovery action, compare exact original hashes; a later user edit must survive as a conflict. Interrupted/unknown builds stay unknown. Preserve failed outputs and logs alongside successful ones.

Stages 4–6 remain a manual approval/evidence protocol. The executable harness does not press Keep, Approve, Undo or Flash, does not create a compiler fault automatically, and does not certify a build/repair from model prose.

## Existing harness, strengthened evidence

`scripts/evaluate-vibe-coder-real.mjs` continues to use the actual isolated desktop and Vibe Coder UI. The new `--workflow` option submits the first three prompt stages in one fresh conversation and stops at review. `--case` can select an individual prompt. Default execution remains read-only. All runs still require explicit loopback inference authorization and an exact model from discovery; no paid credential access is added.

It now attaches existing native events instead of relying only on body text: tool starts/results, model/answer/done/error, pending reviews, file modifications and terminal output. Tool arguments/history are retained in the native done payload when the provider emits it. It collects source/config hashes before and after each turn, flags unsolicited proposals in read-only cases, and retains failed/interrupted evidence. Event overflow is explicit and fails collection. It does not intercept/replace provider or IPC implementations.

`scripts/lib/vibe-evidence.mjs` checks returned knowledge source paths, ranges, byte offsets, SHA256 and clipped excerpts against bundled files. Project-search hits are checked separately against the selected fixture, never against public KB provenance. It detects literal citations absent from retrieved hits. Relevance, claim support, project understanding and compiler validity stay null for human/real-build evaluation. Other citation formats and sources read through other tools need review. This is not a general factual verifier or prompt-injection immunity test.

Reports are reserved before discovery/submission, so an existing output aborts before inference. Real-model transcripts must remain under ignored `src-tauri/target`; fixture roots, file links/junctions and escaping paths are rejected. Snapshot scope is files beneath `main`/`components` plus listed root build/config files, not every possible filesystem write or transient edit. Concurrent OS races and active user input are not isolated by this test tooling.

UI submissions are counted as `submitted_user_turns`. HTTP inference requests remain null during a run because native retries/tool turns are not instrumented; token usage/cost is likewise not measured. Prepared/preflight reports correctly retain zero inference. A captured result never automatically promotes any capability level to PASS.

```sh
node scripts/evaluate-vibe-coder-real.mjs --workflow thai-delay-existing-project-v1 --output src-tauri/target/new-workflow-plan.json
node scripts/probe-desktop-semantic.mjs --preflight-only --output src-tauri/target/new-recorder-preflight.json
```

After an authorized local model is actually ready, start the existing isolated desktop and use its CDP port. This command is a future execution recipe, not an executed evaluation:

```sh
node scripts/evaluate-vibe-coder-real.mjs --run --authorize-local-inference --endpoint http://127.0.0.1:1234/v1 --model DISCOVERED_MODEL_ID --workflow thai-delay-existing-project-v1 --output src-tauri/target/new-real-workflow.json
```

The existing desktop prerequisites in `13-independent-evaluation.md` apply. The desktop probe now stores default results in its unique run directory; `--output` reserves a fresh optional export. It preserves the historical desktop report.

## Evidence and remaining blockers

See `workflow-verification-20261019.json` for executed commands, exit codes, preserved failures and successful no-inference desktop preflight. It verifies subscriptions, delivery of an explicitly synthetic marker through native IPC/events, and cleanup; it is not a real tool/model trace. Synthetic trace/source tests verify the recorder's checks, not model answers. The broad desktop attempt stopped after fourteen observations when the UI had left the editor before the expected timeout fallback; the cause is undetermined and the failed run is retained. A subsequent startup race exposed CDP's initial opaque-origin page; the probe now waits for its actual app origin/bridge before accessing localStorage. The focused recorder preflight is separately scoped and does not erase those failures.

Read-only discovery at the three common localhost endpoints still found none available on 2026-10-19; this is not exhaustive service discovery. No actual LLM prompt, paid key read, approved patch, model-generated firmware compilation, real-model repair or physical test was executed. All three capability levels remain NOT_EVALUATED and all eight original requirements remain PARTIAL. Production application source is unchanged in this increment. No installer rebuild, new version, RC, commit, push, publication or deployment occurred.
