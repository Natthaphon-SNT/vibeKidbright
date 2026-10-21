# Capability validation protocol — 2026-10-19

The starting 253 recorded source/evidence hashes and historical NSIS hash match. This increment preserves production ranking/MiniLM, frozen 69 query/qrel bytes, corpus, nine Block IDs/schema v1, SDK configuration and user data. No local endpoint/model is explicitly authorized. No paid credentials, provider calls, new installer, RC, commit or publication is permitted by this work.

## Status contract

`scripts/lib/capability-matrix.mjs` defines sixteen criteria across Knowledge-aware, Project-aware and Build-aware for the existing thai-delay-existing-project-v1 workflow. Every required case has `status`, `ai_quality_status` and `mechanical_status`. A separate `mechanical_observation` can describe deterministic tests without changing AI status.

- PASS: the specific case/criterion has reported actual local-model execution and an identified, self-attested independent reviewer supplies a substantiated judgment bound to those exact evaluation bytes. This is not reviewer authentication.
- FAIL: a reviewer rejects a case, or evidence fails its mechanical qualification gate. Consult ai_quality_status to distinguish evidence failure from demonstrated poor model output.
- NOT_EXECUTED: required model-dependent execution or a required workflow stage is absent. Preparation, synthetic tests, native preflight and generic compiler success cannot fill this gap.
- NEEDS_HUMAN_REVIEW: actual reported execution was captured but quality is ungraded, or recorded manual-stage evidence needs assessment.

Aggregation precedence is FAIL, NOT_EXECUTED, NEEDS_HUMAN_REVIEW, PASS. Every required case must pass before its criterion/level passes. Per-case rows preserve partial progress. CAPTURED is never an automatic AI-quality PASS. Legacy capability_levels fields remain unchanged for compatibility; the new capability_matrix/standalone matrix is the explicit four-status contract.

## Reproducible collection and review

Prepare the existing real-Vibe harness to a fresh JSON path. The harness embeds a no-grades capability view; its hash covers the JSON.stringify report payload excluding the matrix. For review, use the offline evaluate-capabilities CLI, which binds exact file bytes instead. It never invokes a provider, compiler, patch action or shell.

```sh
node scripts/evaluate-vibe-coder-real.mjs --output src-tauri/target/fresh-prepared.json
node scripts/evaluate-capabilities.mjs --evidence src-tauri/target/fresh-prepared.json --output src-tauri/target/fresh-matrix.json
node scripts/evaluate-capabilities.mjs --evidence src-tauri/target/fresh-prepared.json --template review --output src-tauri/target/fresh-review-draft.json
```

These are recipes, not evidence of execution. Exact executed commands are in the increment verification ledger. Human review packets require schema_version=1, status=reviewed, evaluation_sha256, reviewer, reviewed_at, boolean reviewer_attests_independence and judgments with criterion_id/case_id/status/notes/evidence_refs. A draft has null judgments. Each reference contains a workspace-relative path and SHA256; the CLI verifies exact bytes and rejects escaping/linked paths, stale references and existing outputs. Actual-model review outputs stay under ignored target. Source/citation/private-path notes must not be published blindly.

Optional `--manual` evidence is a separate schema-v1, status=recorded packet bound to evaluation_sha256, with stages for compile/diagnostics/bounded-repair/rollback. Each stage records CAPTURED or FAILED, notes and hashed evidence_refs from user-controlled execution. It does not authorize Apply/Undo or infer compiler/recovery success. These manual stages remain unexecuted until an actual authorized model workflow and user-controlled evidence exist. `--review` applies human judgments only to executed cases. `--mechanical` supplies separate hash-bound deterministic checks; these cannot promote AI-quality status.

File hashes establish evidence identity, not that a provider was truly invoked or a reviewer is independent. A maliciously fabricated local report is not authenticated by this tooling. Raw source/citation existence is mechanical; relevance, factual support, dependencies, API/version/hardware consistency and patch focus require human assessment. Actual generated-C compilation, explicit per-attempt consent and source-hash rollback are separate observations. Hardware and VM results cannot be inferred from any of them.

## Review and regression boundaries

The existing 20-query development packet already includes compiler-error and mixed Thai/English terminology. It remains a blind assistant draft with null corpus judgments, separate from frozen 69 and old development data. Run the existing strict review gate; draft rejection is expected. No API mapping, board compatibility or relevance label is invented. No new retrieval performance is reported without actually running it; historical results retain their original source/binary/date and annotation limitations.

Expand the existing real desktop probe with a debug-only LSP fault fixture for invalid EOF, split-surrogate and reversed result ranges, and IPC-only checks for empty source/empty final line/EOF. These are protocol resilience checks, labelled separately from actual clangd navigation. Preserve real Thai/emoji unsaved definition/reference, header fallback, project isolation, cancellation/timeout and stale-result cases. Repair tests reopen serialized session/journal representations and verify duplicate rejection, three attempts, exact rollback and unrelated-file preservation; this is not an OS reboot/power-loss test or real-model repair.

Release readiness is a separate checklist: actual model-assisted workflow, independent retrieval/citation review, clean Windows VM install/upgrade/uninstall, data preservation, hardware, advisories and cross-platform behavior. Passing deterministic tests alone cannot authorize a new installer or release.
