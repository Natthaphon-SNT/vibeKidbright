# Evidence integrity and UTF-16 semantic validation — 2026-10-19

Start from the matched 242-file manifest in verification.json and the inspected pre-existing Git diff. Preserve the historical NSIS 3.7.3 hash, bundled model/corpus, nine Block IDs/schema v1, production query paths/ranking, existing integration and diagnosis reports. The user explicitly requires keeping ranking because no independent reviewer is available. No endpoint/model inference authorization has been supplied; do not read provider credentials or run inference.

## Source-backed defects

The real-Vibe evidence collector checks source hashes, but its case failure condition previously ignored invalid source hits and unmatched literal citations. Thus completed evidence could be labelled CAPTURED despite known integrity failures. Reject malformed search payloads, invalid source evidence, unmatched literal citations, missing requested tool evidence and empty answers. Keep human support/relevance/quality/compiler grades null. CAPTURED means mechanically collected, ungraded evidence, never a capability pass. Successful read-only tool contracts and strict failure cases must be tested without model inference.

Semantic source-position conversion appended an artificial end-of-file line start, accepting one nonexistent line beyond a non-newline-terminated source (and an extra line beyond a terminated source). Remove that phantom line while preserving the valid empty final line after a newline. Validate in-scope clangd result ranges against the exact unsaved UTF-16 snapshot, or a size-bounded saved source when unopened. Reject reversed ranges and split surrogates; retain project containment and labelled syntax fallback. Do not infer header CDBs or introduce F12 in this increment.

## Verification boundaries

Expand actual React/Tauri/clangd/Monaco desktop evidence with Thai plus emoji before identifiers in unsaved buffers, exact returned Monaco columns and references; separately label IPC-only invalid-position checks. Rebuild the debug-native probe before execution. Preserve all raw failures and new outputs. Existing synthetic read-only component-tab bootstrap remains labelled and is not a model/edit result.

Run available frontend, Rust, Python, Node, host-C, native semantic, configured ESP-IDF build, release configuration and policy checks. Record exact commands/exits and source/resource hashes. No retrieval reranking/model measurement is needed when retrieval code/assets are byte-identical to the latest fixed-policy measurements; reference those measurements with their date and scope, without claiming a new accuracy gain. No physical, clean-VM, paid-provider, new installer, RC, commit, push or publication claim.
