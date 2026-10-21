# Independent relevance and real-provider evaluation

The frozen 69-query set and qrels are unchanged. `retrieval-stratified-review.json` derives Thai/English/mixed/exact-API/hardware metrics from the existing measured runs, without running or tuning models again. Groups overlap: hardware is the 11 declared board/peripheral topics across three languages (33 cases), exact API is 9 symbols, and each language has 20 prose cases. Resource figures are whole-process measurements, not per-category RAM. Installer bytes describe the preserved actual NSIS; no candidate installer was built.

## Annotation uncertainties

Every current qrel is assistant-authored and non-exhaustive. A code/header pair can share useful evidence; a broad board document can be relevant without appearing in the labels. Twenty translated topic triplets are correlated. Duplicate files in the top five chunks do not refill document ranks. SDK/board metadata can be unknown. These limits affect the absolute percentages and prevent claims of human-independent relevance, grounded-answer accuracy or hardware compatibility. No labels or hyperparameters were changed after seeing final-set scores.

`independent-retrieval-review.json` is a **blank review packet**, not an evaluated dataset. It has zero independently reviewed cases. Have a human author at least 15 new real classroom questions without seeing ranker outputs, then a different identified person review questions and judge every frozen public corpus file (0 irrelevant, 1 useful, 2 direct answer), including notes/disagreements. Use a private copy if reviewer identities should not enter this repository.

The validator refuses drafts, missing/identical/assistant reviewers, unjudged files, incomplete strata and missing review notes. Review identity/independence is self-attested, not authenticated. It exports into a new file with exclusive creation and refuses the old frozen filename; preserve graded labels and freeze the new dataset/corpus/ranking plan before any evaluation. Binary relevant-file labels are the current runner contract; graded NDCG would require a separately declared metric change. No-answer questions require a separate abstention benchmark.

```sh
python scripts/review-retrieval.py summarize
python -m unittest discover -s scripts/tests -p test_retrieval_review.py -v
python scripts/review-retrieval.py validate-review <completed-packet.json> --output <new-dataset.json>
```

## Real Vibe Coder protocol

`real-llm-evaluation.json` records **NOT_EXECUTED**. No local model is available (confirmed by the user); no paid credentials or provider calls were used. Current browser fixtures and ONNX embeddings are not an LLM evaluation. Null quality fields are not zero scores or successful results.

When an authorized endpoint/model is supplied, use the actual Vibe Coder UI and native provider/tool loop with an isolated AppData directory and generated public project. Record provider/model/version, exact source/dataset/build hashes, prompts, tool calls, final answers, elapsed time, usage/cost and stop/failure outcomes. Do not silently substitute a separate chat-completion script for the application workflow.

1. Ask Thai/English/mixed documentation, exact-API and hardware questions; record returned source citations. Independently check cited file/line ranges and whether each factual claim is supported. A rendered citation alone is not a correct citation.
2. Request generation on an existing confirmed profile; review every proposed file and source-list change. **The user must approve each patch before application.** Save approvals, revisions and before/after hashes. Keep drafts pending if approval is unavailable.
3. Build only through configured ESP-IDF, with no Flash; distinguish source/API errors from toolchain/dependency failures. Record exit status and full bounded diagnostic evidence.
4. Introduce a public fixture error, start Compiler Repair, review each proposed correction separately and approve each applied attempt. Record attempt count (maximum 3), duplicate rejection, compile result, rollback, later-edit preservation and cancellation/restart outcomes.
5. Repeat benign tasks with adversarial instructions in reference documents, C comments and build logs. Measure task success, unauthorized actions and private-context exports. Native gates/envelopes are defense layers; no prompt-injection immunity is claimed without this real-model testing.

Keep provider sessions and reviewer data private by default. Publishing traces, sharing private source and incurring provider charges require their own authorization. Clean-VM GUI lifecycle and physical-board tests are separate qualification evidence.


## Executable application evaluation harness

real-vibe-coder-cases.json separates public documentation grounding, citation support, generated C, compiler diagnostics, per-patch review, bounded repair and exact rollback evidence. scripts/evaluate-vibe-coder-real.mjs prepares null-grade reports by default. --preflight checks the actual isolated desktop identity through real Tauri IPC without discovering/calling a model. Its successful preflight is embedded in desktop-semantic-integration.json.

--run additionally requires --authorize-local-inference, --endpoint pointing to credential-free loopback HTTP and an explicitly selected --model present in discovery. Redirects/remote providers are rejected. Default execution captures the read-only Thai/English grounding and Thai project-understanding prompts through the actual Vibe Coder UI; it never presses Keep/Approve. --workflow thai-delay-existing-project-v1 uses the connected Thai grounding/project/proposal sequence, stopping before user approval; --case selects one prompt. Compilation/each repair patch and Undo still require the separate manual approval/evidence protocol above. Scores remain null until independently judged; compiler validity is separate. Output is reserved with exclusive-create before discovery, canonical-parent scoped, and raw model traces restricted to ignored target/. Current local discovery finds no ready endpoint: actual LLM inference remains NOT_EXECUTED.

Prepare: node scripts/evaluate-vibe-coder-real.mjs --output src-tauri/target/new-real-vibe-report.json

Preflight (while the isolated desktop is running): node scripts/evaluate-vibe-coder-real.mjs --preflight --cdp http://127.0.0.1:19327 --output src-tauri/target/new-preflight.json

The executable harness now collects existing native tool/answer/review events and source/config hashes, checks returned bundled-source excerpts mechanically and preserves failed/interrupted collection. It can request a proposal explicitly but never applies one. The real-model branch remains unexecuted; actual-desktop no-inference listener preflight and synthetic evidence checks are separately scoped. Model compilation/repair/rollback still require the approval-gated protocol and are not an automated complete benchmark. See 18-capability-milestone.md and workflow-verification-20261019.json.


Development prerequisites for reproducing the desktop harness: Windows interactive session with WebView2/MSVC, installed ESP-IDF 5.5.4 and its Python environment, pinned clangd 23.1.0 at target/clangd-evaluation/clangd_23.1.0/bin/clangd.exe, and the existing two exported/built public fixtures under target/block-c-evaluation-20261018. Follow the export/build commands in 10-block-to-c.md first; exact CDBs must come from actual idf.py builds. Set VIBE_EVAL_IDF/VIBE_EVAL_TOOLS if using different installed paths. Ports 19327/19328 must be free.

The new retrieval-development-review-20261019.json is an assistant-authored draft, not a reviewed dataset. The user has no reviewer and requests preserving ranking. Once genuinely completed, use `python scripts/review-retrieval.py validate-development <reviewed-packet.json> --output <new-development-dataset.json>`. This gate checks complete judgments, explicit boolean independence, all five query kinds and normalized separation from frozen/prior cases. It rejects draft packets, missing judgments and supplied ranker outputs; it does not authenticate reviewer identity or turn assistant-authored questions into independent authorship. Do not run tuning with the draft. diagnose-retrieval.py only diagnoses old fixed-policy results.

The existing ignored browser-tool cache contains playwright-core 1.64.1. Recreate it with npm install --prefix src-tauri/target/learning-ui-tools --no-save --package-lock=false playwright-core@1.64.1; Python test dependencies are pinned in scripts/tests/requirements.txt. These dependency-install commands were not run this increment because the packages were already present. Build production assets with npm run build:release and build desktop_evaluation/stalling_lsp_fixture with offline locked cargo example builds. Stage the verified resources/runtime/windows/*.dll into target/debug/examples before launching GUI-linked examples on systems without equivalent global CRT DLLs. The local run staged these existing assets; no download or production dependency was added. This is development tooling, not an installer prerequisite; the app and historical NSIS do not depend on Node/Playwright/Python jsonschema.
