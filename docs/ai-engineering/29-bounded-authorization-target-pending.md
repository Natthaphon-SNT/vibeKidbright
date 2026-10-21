# Bounded pilot authorization, target pending — 2026-10-19

The user granted a conditional actual-model pilot scope: public test-fixture source
and nonprivate KB only; at most **10 HTTP requests including retries** and **1 USD**;
only a designated test credential channel; separate project/revision approval for
every applied patch; no Flash, commit, push, publication or deployment.

Provider, endpoint and model/version in that reply are still literal placeholders.
No test credential channel is actually designated. No endpoint/model was inferred
from app defaults, historical discovery or existing keys. A follow-up asks only
for these missing values and the paid provider's hard-spend mechanism; permission
for public-source export and the two ceilings is retained without requesting it
again. No key value is requested in chat.

## Work performed offline

- Verified the current **318-file** manifest before changes, captured Git status/
  diff summary and inspected existing isolated-desktop and native send/retry code.
  Read only source code, not runtime configuration, keyring or credentials.
- The native loops can retry and make tool-round requests. The existing UI-turn
  counter does not meter these HTTP attempts or enforce a monetary ceiling.
  Provider/model pricing and a guaranteed cost upper bound remain unknown.
- Added a **fail-closed run gate** in the existing evaluation harness. Even valid
  local endpoint/model CLI syntax plus --authorize-local-inference is rejected
  before report reservation, CDP, /models, configuration writes or inference.
  There is no flag/environment bypass. Preparation and the existing no-inference
  preflight code remain; no preflight/network connection was executed here.
- This deliberately prevents an unmetered evaluation run. It does **not** implement
  a native ten-request counter, a 1 USD meter, pricing rules or full serialized
  request capture. Actual execution must stay blocked until these are verified
  for the concrete authorized route.
- Added a regression asserting the early refusal and no output creation. Updated
  the existing-report test to retain its byte-preservation requirement through
  both early run refusal and exclusive preparation output. The first targeted
  run had one failure because it expected EEXIST before the new earlier gate;
  that log is retained. The final targeted suite passes eight tests.

No production provider, application architecture/configuration, SDK, model/ranking,
bundled corpus, frozen queries/qrels, Block schema/IDs, lockfile, installer or user
project was modified. Only the offline harness, its existing Python test module,
additive reports/evidence and generated rule inventory change. No dependencies
were added and no application/installer build was performed.

## Evidence and remaining work

Fresh full offline results: **Python 24 passed; Node 12 passed**. The targeted
harness subset has eight passing tests; it is part of the Python total.
These are mechanical tests only. Exact commands, exit codes, source/log hashes
and the retained failed run are in ten-request-verification-20261019.json.

Actual model HTTP requests **0**, metadata probes **0**, pilot API charges **0 USD**,
actual-model successes/failures **0**, patch approvals/applies/builds/rollbacks **0**.
The seven required workflow stages and all sixteen AI-quality/workflow-mechanical
criteria remain NOT_EXECUTED in the previous exact-byte matrix. Its hashes and
the original 20-query/null-judgment review packet remain unchanged. Independent
reviewers/judgments remain zero. No new metrics or review grades are produced.

| Missing prerequisite | Smallest next step |
|---|---|
| Concrete provider / endpoint / model-version | User replaces the bracketed template values; no endpoint probing to discover them. |
| Designated test credentials | User names/prepares the isolated test channel, or explicitly specifies a keyless local service; never search existing user keys. |
| Ten-request enforcement | Count and reserve each actual native HTTP attempt, including retries and metadata, and control redirects/transport behavior on the isolated route; prove refusal before an eleventh attempt offline before enabling inference. |
| Guaranteed 1 USD upper bound | Establish the actual model's pricing/output/input bounds and applicable charging behavior or a verified hard spend limit. Response usage reported after a charge is insufficient to guarantee the ceiling. Keyless localhost does not by itself prove nonbillable operation. |
| Redacted exact request capture | Capture actual serialized requests and responses/counts on the same native route without authorization headers, URL keys, private source or SDK paths; existing native event/UI-turn traces are incomplete. |
| Public-source-only outbound scope | Verify the actual request/tool context before transmission. The existing loopback permission policy can include installed SDK paths; a local endpoint alone does not narrow that to the user's public-fixture-only grant. Keep the run blocked until the isolated route enforces the narrower scope. No such outbound request was observed/executed here. |
| Actual-model and independent quality evidence | Run the original pilot only after the safeguards above; stop for real user patch approval and keep CAPTURED ungraded until independent review. |

Current/frozen corpus drift, clean-VM lifecycle, hardware, cross-platform and
dependency-review blockers remain as recorded in report 28. Release stays
**NOT_READY**, and all eight original requirements remain **PARTIAL**. Historical
NSIS 3.7.3 stays byte-identical. No inference was used to test authorization.
