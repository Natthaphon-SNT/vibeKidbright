# Actual-model evidence readiness — 2026-10-19

## Result and authority

Actual Vibe Coder model requests: **0**. Actual-model workflow successes: **0**;
failures: **0**; all **7 required workflow stages NOT_EXECUTED**. No inference
endpoint was probed, no credentials/configuration stores were opened and no
provider was invoked. No current authorized endpoint/model, source-export scope
or request/budget ceiling is present in the nonsecret engineering records.
Historical localhost discovery is not current availability or authorization.
Source defaults in config.rs are not authorization; the shared runtime config
and secure credential getters were deliberately not called/read.

The current manifest, not the older 274-file number, matched **307 files** before
changes. Repository/ancestor AGENTS.md paths checked were absent. Git status,
diff summary and the existing AI implementation diff were inspected and captured
under ignored target. Previous dirty work is preserved. The latest preceding
report is 27-ip-matrix-user-evidence.md; its physical result is user-reported iP
display evidence, not actual model execution. Protocol 23 and capability report
24 remain the acceptance contract. No criteria/case requirements were relaxed.

## Reproduced implementation defect and limited fix

The existing evaluation CLI silently accepted an unsupported `--max-requests 0`
argument in an offline preparation run. No inference was made in this repro.
The shared argument parser would also ignore unknown options in run mode.
That could falsely suggest a caller's ceiling was enforced. The parser now
rejects unknown/duplicate options and missing values before report reservation,
desktop connection or model discovery. A regression covers these cases,
including explicit local-run flags combined with the unsupported ceiling.

This **does not implement a provider request budget**. The current harness counts
submitted UI turns, not native HTTP requests/retries/tool turns/token use.
A user-approved hard HTTP/token ceiling must be enforced and tested before a
run that depends on it. No parser success or UI-turn cap substitutes for that.
Only scripts/evaluate-vibe-coder-real.mjs and its existing Python test module
changed implementation. No production provider, app architecture, configuration,
model/ranking, dependency, lockfile, corpus, SDK or Block mapping changed.

## Fresh offline evidence and scope

Existing tools prepared a new report for thai-delay-existing-project-v1 and a
standalone matrix/review template, without --run/--preflight or an endpoint:

- src-tauri/target/actual-model-prepared-20261019.json
- src-tauri/target/actual-model-matrix-20261019.json
- src-tauri/target/actual-model-review-draft-20261019.json

The template has 22 required criterion/case judgment rows, all null, bound to the
exact prepared evaluation bytes. It is **not a review packet from actual model
outputs**, because those outputs do not exist. Retrieved chunks, serialized
requests, model responses, proposed/applied diffs, model-assisted build logs and
rollback observations remain absent. There is no fabricated CAPTURED evidence.

Python offline suite: **23 passed**; targeted harness regressions: **7 passed**;
Node evidence/matrix suite: **12 passed**. These exercise synthetic source/citation,
hash, status, review, approval-boundary and CLI behavior. They are mechanical
checks, not app/model quality. Frontend, Rust, desktop, firmware and installer
tests were not rerun because this change touches the offline harness/parser and
evidence only. No new installer or application binary was built.

## All sixteen criteria

These are the fresh pilot's AI-quality and per-case workflow-mechanical statuses.
The successful offline test suites above are separate observations. Historical
deterministic/compiler/desktop checks are not transferred into these cells.

| Criterion | AI quality | Workflow mechanics |
|---|---|---|
| Retrieval relevance | NOT_EXECUTED | NOT_EXECUTED |
| Citation existence | NOT_EXECUTED | NOT_EXECUTED |
| Citation factual support | NOT_EXECUTED | NOT_EXECUTED |
| ESP-IDF API/version correctness | NOT_EXECUTED | NOT_EXECUTED |
| Hardware metadata consistency | NOT_EXECUTED | NOT_EXECUTED |
| Correct project identification | NOT_EXECUTED | NOT_EXECUTED |
| File/symbol selection | NOT_EXECUTED | NOT_EXECUTED |
| Dependency understanding | NOT_EXECUTED | NOT_EXECUTED |
| Patch scope | NOT_EXECUTED | NOT_EXECUTED |
| Unrelated-code preservation | NOT_EXECUTED | NOT_EXECUTED |
| Compiler-error classification | NOT_EXECUTED | NOT_EXECUTED |
| Focused repair | NOT_EXECUTED | NOT_EXECUTED |
| Per-attempt approval | NOT_EXECUTED | NOT_EXECUTED |
| Compilation success | NOT_EXECUTED | NOT_EXECUTED |
| Rollback correctness | NOT_EXECUTED | NOT_EXECUTED |
| Bounded repair attempts | NOT_EXECUTED | NOT_EXECUTED |

Independent reviewers/judgments: **0**. No assistant self-certifies as independent.
When actual evidence exists, CAPTURED must remain NEEDS_HUMAN_REVIEW unless the
protocol's mechanical/reviewer gates and every required case are satisfied.

## Retrieval identity and blind review

Frozen 69 queries/qrels remain byte-identical. Their original 37-source snapshot
was verified against the preserved 3.7.3 extraction and copied for blind review
to src-tauri/target/actual-model-blind-review-20261019/corpus. The original 20-query
development packet was copied byte-for-byte with no rankings, model answers or
frozen qrels included in the reviewer folder. All judgments stay null. The strict
development gate rejects this draft, as expected. Source hashes, query IDs and
the packet hash are in the fresh review manifest/readiness report.

The **current corpus is not the frozen corpus**: all_models.md, inex_ia_ip.md,
support_scope.md and compatibility/README.md changed in earlier increments.
The current bundled inventory has 42 files; five are outside the old text
snapshot (four existing PDF resources plus the new iP C reference). The current
as-frozen hash gate correctly rejects this drift. No corpus file was changed in
this increment. Historical matching/metrics are not reassigned to current bytes.
No retrieval ranking run, latency/RAM measurement, new qrel, relevance judgment,
hyperparameter tuning or accuracy gain is reported.

This review folder supports historical frozen-source review only. Current-corpus
evaluation requires a separately identified snapshot/review scope; silently
rebasing the old packet or weakening the existing CLI's frozen-corpus check is
not permitted. The original 20 questions remain assistant-authored, even after
future human review; they are not independent human-authored ground truth.

## Smallest authorized pilot and remaining blockers

The existing public repeat_delay/forever_delay fixtures were source-hashed without
reading user projects or SDK/provider configuration. Their source readiness record
preserves the original case IDs and all required stages.

| Blocker | Smallest next step |
|---|---|
| Endpoint/model authority and data scope absent | User supplies provider/endpoint/model, authorized public-source export scope and budget/request ceiling; no paid authorization is implied. |
| Native HTTP budget and exact request capture incomplete | Before authorized inference, enforce the agreed ceiling in the isolated route and capture redacted actual serialized requests plus request counts; current UI turns/native events alone are insufficient. |
| No actual pilot | Use the credential-isolated debug desktop with React → Tauri → native provider/tool path. Collect grounding-th, project-th, generation-th; stop at review. This initial 3-turn collection alone does not cover Build-aware. |
| Apply/build/diagnosis/repair/rollback absent | User performs real project/revision-bound approval, configured build, controlled public-fault/diagnostic stage and separately approved repair attempts (maximum 3), then user-controlled Undo. Record source/patch/build/rollback hashes; never auto-approve or Flash. |
| Later stages not automatically collected by the current harness | Existing manual-stage packet can reference real user-controlled artifacts, but must include actual model/approval/native/build records and receive independent assessment. A submitted manual CAPTURED claim is not authenticated proof. |
| Independent assessment absent | A separate reviewer evaluates actual outputs and the hash-bound 22-row criterion/case form; leave judgments null until then. |
| Current retrieval relevance unreviewed | Review the exact declared corpus; preserve old/final bytes and distinguish historical frozen review from a future current-corpus evaluation. |
| Clean Windows VM/install lifecycle absent | Run install/upgrade/uninstall and user-data preservation in a disposable clean VM after AI gates are met. |
| Hardware/platform qualification incomplete | Measure requested functions on identified physical boards; separately run supported OS paths. A user-reported iP display pass is scoped evidence only. |
| Dependency advisory disposition incomplete | Review existing platform-specific findings separately; no new dependency/network audit or migration was performed here. |

The isolated desktop/harness path is a debug application route, not installed
release qualification. Full provider requests and per-request counts are not yet
captured by its event collector, and it stops before review-required Apply. A
future direct provider-only test cannot be called end-to-end desktop success.

Release stays **NOT_READY**. All eight original requirements remain **PARTIAL**.
Historical NSIS 3.7.3, production MiniLM/ranking, frozen queries/qrels, current
bundled corpus, nine Block IDs/schema v1, SDK configuration and lockfiles are
preserved. No commit, push, publication, deployment, promotion or installer build
was performed. Commands, exit codes, protected hashes and change attribution are
in actual-model-verification-20261019.json and actual-model-changes-20261019.json.
