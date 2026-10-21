# Dependency security follow-up — 2026-10-18

The first complete lockfile lookup is retained in [the before report](rust-advisory-before.json). It found 52 advisory records across 26 package-version entries, including duplicated GHSA/RustSec aliases, inactive platforms and maintenance notices. These are **not 52 independently exploitable application vulnerabilities**.

Patched locked versions include Tauri **2.12.4** (manifest minimum 2.11.1), rustls **0.23.46**, rustls-webpki **0.103.15**, h2 **0.4.20**, ZIP **9.0.5**, anyhow **1.0.104**, event-listener **5.4.2**, serde_with **3.24.0**, quinn-proto **0.11.19**, supported rand patch lines and OpenSSL **0.10.81**. Tauri remains v2; existing providers, UI and configured SDK are retained. The JavaScript Tauri packages/CLI are updated within their existing v2 ranges. After remediation, frontend/native tests and release/policy checks pass, and the rebuilt NSIS is extracted and probed successfully. See verification.json and installer-validation-3.7.3.json; a pre-remediation installer is not final evidence.

The [Tauri upstream advisory](https://github.com/tauri-apps/tauri/security/advisories/GHSA-7gmj-67g7-phm9) confirms the IPC origin-confusion fix begins at 2.11.1. TLS updates address the recorded handshake/certificate/session issues. The [current Rust lookup](rust-advisory-audit.json) records each advisory, source, exact lock hash and selected Windows normal/build graph, so platform reachability is not guessed from a whole-lockfile count.

After remediation the whole lockfile still has **10 advisory records across 8 packages**. Its selected Windows graph has only maintenance notices for `fxhash` (through scraper) and `paste` (a proc macro); there are no non-maintenance advisory matches in that selected graph at lookup time. This does not prove absence of unknown/reachable defects. GTK/glib's iterator issue and old quick-xml behind Linux portal/zvariant dependencies remain in other platform paths. They are not silently dismissed as fixed. Linux support needs upstream-compatible changes/testing; replacing GTK or forking Tauri is outside this increment. Maintenance warnings for the remaining platform dependencies also remain.

The npm direct DOMPurify range previously conflicted with its exact override, causing `npm audit`/fresh dependency operations to fail with EOVERRIDE. The direct dependency now pins the same already-installed 3.4.17; no sanitizer behavior was downgraded. [The full npm audit](npm-audit.json) reports zero advisories after updating the v2 Tauri packages. Neither audit is a penetration test or a universal security certificate.

## ZIP extraction defense

The SDK extractor now reuses the reviewed canonical-path checks, rejects archive links/special files/traversal/device aliases/reparse parents, and writes to an owned temporary file before atomic replacement. Cancellation is checked during copying; partial data does not replace an existing SDK file. Declared/actual expansion limits are enforced (250,000 entries; 2 GiB/file; 16 GiB total). Unix permissions are applied through the temporary file handle and strip SUID/SGID/sticky bits. Empty created directories can remain after a cancelled setup. Windows regression fixtures exercise valid extraction, traversal rejection, cancellation preservation and link rejection. Unix behavior still needs platform execution.

## Privacy/cancellation integration

Automatic project-path/log wrapping is removed from user chat text. Native project metadata is attached only when private-context permission permits it. Workspace-availability hints stay generic and server-owned. The SDK-path tool now needs the same private-context permission; creation success uses a relative workspace marker without export permission. Typed/pasted user text and every possible OS error string are not a complete DLP boundary.

AI Stop/revoked consent cancels only the AI-owned SDK future; its guard requests native tree termination and keeps file exclusion until worker cleanup. Actual worker termination has a regression test. General setup/download, serial and ADB lifecycle coverage remains separate.

Run `python scripts/audit-rust-dependencies.py --date YYYY-MM-DD --output <new-workspace-report.json>` for a read-only OSV lookup of public crate names/versions, and `npm audit --json` for npm. A new output path is required so prior audit evidence is preserved. The Rust lookup succeeding means the query ran; it deliberately records `all_lockfile_clean=false` rather than treating warnings as a clean audit.

## Continuation review without dependency churn

The Cargo/npm lockfiles and packaged embedding model are unchanged in this continuation. Current [RustSec fxhash](https://rustsec.org/advisories/RUSTSEC-2025-0057.html) and [paste](https://rustsec.org/advisories/RUSTSEC-2024-0436.html) pages classify the selected Windows findings as unmaintained, with no patched versions. They require upstream dependency migration, not a fabricated patch-version fix. The [quick-xml advisory](https://rustsec.org/advisories/RUSTSEC-2026-0194.html) identifies >=0.41.0 as patched; the inactive Linux portal path still locks 0.38.4. No Linux security clearance or all-lockfile-clean claim is made, and the dependency architecture is not forked simply to suppress warnings.

New semantic navigation has no added dependency or remote index. It reuses process-tree containment, requires an exact installed GCC driver/CDB, disables clangd project/user config and filters response-file/plugin loading. The user explicitly chooses a trusted clangd executable; arbitrary or compromised local executables and hostile OS races are outside that trust boundary. Provider evidence wrapping/role tests mitigate prompt injection while native review/consent gates remain authoritative; actual adversarial LLM behavior is not measured without a model.

The fresh 2026-10-19 lookup is retained separately in dependency-security-diagnosis-20261019.json: 753 public crate/version queries, ten advisory records across eight whole-lock packages, only fxhash/paste maintenance notices in the selected Windows graph, and zero npm advisories. Initial sandbox network failures and successful permitted retries are recorded. No lockfile, dependency version or production ranker was changed. This does not clear inactive platform findings or certify runtime reachability/absence of unknown defects.


## Evidence-integrity follow-up audit

A fresh public npm/OSV lookup is recorded in evidence-security-20261019.json. Dependency/lock bytes are unchanged; the same platform/maintenance limitations apply. Repair UI consent now binds project/revision and is consumed for each apply attempt. No real-model prompt-injection or general penetration-test result is claimed.
