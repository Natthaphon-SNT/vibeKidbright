# AI context growth and token usage — 2026-10-28

## Problem and change

Each provider iteration previously replayed the full accumulated conversation,
including old tool arguments containing complete source files and their large
responses. Classic also received a 67,515-byte guide with code templates on
every call, plus the full common/support/compatibility documents. A single
user reply can involve multiple model calls, so repeated input contributes to
cumulative token usage even when the visible answer is short.

The provider now sends a bounded payload copy. The displayed conversation and
native proposal/review/authorization state are preserved. This implementation
does not call a second model to summarize history or change the selected model.

## Context policy

- Runtime rules are concise; full guides remain in `knowledge_base/guides/` and
  the installed bundle for targeted retrieval. Model/board corrections still
  use the original conversation and validated native state.
- History under 48 KiB is preserved exactly. Above that limit, keep recent user
  turns, the latest request and the newest current-turn tool call/response group.
  Older complete tool exchanges become untrusted receipts, never orphan calls.
  Google thought signatures and the protected exchange's source/proposal contents
  remain intact. Large older assistant answers become marked excerpts.
- A bounded archive carries the original request excerpt, recent substantive
  requests, recent choices/outcomes and source/path/hash receipts. This is a
  deterministic partial memory, not a semantic summary guaranteeing every old
  requirement. Missing exact requirements must be retrieved or clarified. It
  does not authorize tools, Keep, repair attempts or hardware execution.
- The latest usable user prose is preserved for code-only followups. Language
  annotations use the compacted message numbering, while host routing continues
  to inspect the original allowed dialogue.
- The serialized entire request, including system instructions and tool schemas,
  is capped at 96 KiB. If the latest request/current exchange cannot fit, stop
  before that HTTP call and explain how to reduce it. Do not silently truncate
  a current proposal. Previous calls in the same turn may already have usage.
- `read_file` and `read_knowledge_file` support `start_line` and `max_lines`;
  default 200 lines, at most 500 lines and 16 KiB per page. Responses include
  total/end lines, next_start_line, complete_file/truncated and the full-source
  hash. Fetch missing pages before a complete-file replacement. A line too large
  for the page is an explicit error, not a silently cut source line.
- `kidbright_reference` returns a compact index by default. `catalog_board`
  retrieves exact board pins/conflicts/sources; profile+recipe still returns
  complete deterministic files. A narrow call no longer dumps the full catalog
  and support documents.

The limits above are UTF-8/serialized **bytes**, not tokenizer counts or a
universal model context limit. Multimodal or unusually large source requests
may hit the guard and need a smaller scope. Smaller requests do not establish
unchanged model-answer quality without live evaluations.

## Measurements and usage display

Provider reporting was subsequently expanded to all existing custom/local/native
Google routes, with partial/interrupted states, credential-scoped compatibility
negotiation and per-request deduplication. See
[provider-neutral usage](AI_PROVIDER_TOKEN_USAGE_2026-10-28.md) for the current
accounting behavior; the byte budgets and measurements below remain applicable.

| Item | Before | After |
|---|---:|---:|
| Common runtime file | 17,154 bytes | 5,099 bytes |
| Classic runtime file | 67,515 bytes | 3,433 bytes |
| Plus runtime file | 10,832 bytes | 3,261 bytes |
| Synthetic long tool history | 1,210,911 bytes | 3,894 bytes |

These are source/payload measurements, not measured token or billing savings.
The fixture retains original motor timing, latest user request and the current
tool pair. Full source files remain available as paged references.

The UI separately shows the latest serialized request size and actual
provider-reported tokens accumulated across calls **for the current reply**.
OpenAI/OpenRouter can request a final streaming usage chunk; Google usageMetadata
is parsed when present. Multiple snapshots within one response are not summed:
only the final observed request usage is emitted once. Missing/aborted streams
stay unknown and can make the displayed usage incomplete. Cached tokens are
not subtracted or converted into an invented monetary cost. This is not an
account-wide bill or an exact token estimate derived from bytes.

Provider contracts: [OpenAI Docs — streaming usage](https://developers.openai.com/api/reference/resources/chat/subresources/completions/streaming-events),
[Google usage metadata](https://ai.google.dev/api/generate-content#UsageMetadata).

## Verification

Native tests cover exact short history, bounded long history, preserved current
tool pairs/thought signatures, no mutation of stored history, rejection of an
oversized current request/proposal, UTF-8 lossless file paging, code-only language
continuity, compact board references, both provider payloads and optional usage.
Complete bundled C recipe results and all 128 Plus test-reference results are
checked against the current-exchange budget.

Frontend tests cover request bytes versus actual token totals, accumulation
across tool calls, unknown usage, stale-event isolation and reset per new reply.
The full native/UI suites and production build are run; raw native output is in
`src-tauri/target/context-budget-final.log`. The normal KB, public-resource,
standalone and document/coverage checks also apply. Measurements and source
fingerprints are in [measurement.json](research/context-budget-2026-10-28/measurement.json).

No paid/live API request, live model-quality comparison or physical-board test
was performed in this change. Cumulative usage still grows with additional
calls; per-request history no longer grows without a bound just because the
conversation grows longer.
