# Provider-neutral API token usage — 2026-10-28

## Result and supported routes

Token reporting now applies to every existing application provider route rather
than enabling stream usage only for api.openai.com and OpenRouter. It measures
the responses to calls made by this app, not an API key's account-wide balance,
quota, monetary bill or calls made by other applications.

| Application route | Usage handling |
|---|---|
| OpenAI / OpenRouter | Standard completion counters; final usage-only chunks do not require choices/content |
| Custom OpenAI-compatible endpoints | Same negotiation/parser for any hostname, including compatible DeepSeek, Groq, Mistral, xAI, Together or gateway deployments; branding labels are separate from capability detection |
| Google native route | usageMetadata / usage_metadata, cache and thinking counters; preserve the provider total |
| Local compatible servers | Request include_usage on local/LAN endpoints too, including the existing Ollama/LM Studio/vLLM compatible route |

This does not add new authentication or generation protocols. Native Anthropic
Messages, Bedrock and other non-compatible generation endpoints still require
their own transport adapters. Claude through OpenRouter/a compatible gateway
uses that gateway's returned usage. The normalizer additionally understands
split Anthropic usage events and Responses-style envelopes when supplied; that
parser coverage alone is not a new direct-provider connection feature.

## Request negotiation and credential privacy

Every Chat Completions endpoint is initially asked for
`stream_options.include_usage=true`. When an endpoint returns HTTP 400/422 with
an explicit unsupported/unknown/forbidden stream_options or include_usage error,
retry once without only that optional field. No compatibility retry is triggered
for authentication, quota/rate-limit or unrelated model errors. Never replay
generation or tool actions because a successful stream lacked usage.

Remember rejection in a bounded in-memory cache scoped to endpoint, model and
credential digest. One gateway key must not disable usage negotiation for a
different key. Only an opaque digest is retained; no credential value or URL
query is written into usage reports, logs or persisted settings by this code.
Restarting the app clears capability rejections so upgraded servers can be
negotiated again. Request/context budgets and private-source permissions remain
unchanged. No extra usage-accounting API or quota query is called.

## Counter semantics

| Envelope | Recognized fields |
|---|---|
| usage / response.usage | prompt_tokens/input_tokens, completion_tokens/output_tokens, total_tokens, cache/thinking detail fields |
| Google | promptTokenCount, candidatesTokenCount, totalTokenCount, cachedContentTokenCount, thoughtsTokenCount and snake-case variants |
| Groq metadata | x_groq.usage with standard counters |
| Ollama metadata | prompt_eval_count, eval_count, prompt_eval_cached_count |
| Split Anthropic metadata | message_start message.usage followed by cumulative message_delta usage |

Missing fields remain null, never zero. Actual reported zeros remain zero.
Negative, fractional, string or overflowing derived values are not silently
converted into counts. Provider totals take precedence. If both input/output
are known but total is omitted, derive it from the reported components and mark
`total_tokens_derived`; this is arithmetic on actual counters, not tokenization
or a byte/character estimate. Google thinking is included in a derived total
without adding it twice to a provider total. Anthropic input is normalized as
uncached + cache read + cache creation, consistent with its documented split.

Stream counters are cumulative snapshots for one request, not deltas to add.
Merge partial snapshots, invalidate a stale total when later component counts
change without a new total, and emit one final report per request. Distinct
request IDs are summed across tool calls/retries/models; repeated events for
the same request replace its previous report instead of increasing the sum.

## Completion and UI

A scope guard emits a report on normal completion, early return, timeout,
cancellation or stream failure. It retains the last observed actual counters.
Usage-only tail frames without a final newline and `data:` without a space are
accepted for accounting. A null/false/empty error field does not mark success
as an interrupted stream.

Four states are explicit: reported, partial, unavailable and interrupted. The UI
shows input/output/total plus cache/thinking when available, marks known sums
incomplete if another call is unreported/interrupted, and identifies providers.
Per-model/per-request details are available in the counter tooltip. A finished
reply without usage says “not reported by provider” rather than waiting forever
or claiming zero. New replies/chats reset the display; stale events from another
reply are ignored. Error endings also mark known totals as partial.

## Verification and limits

The native suite passed 276 tests. The UI suite passed 232 tests in 23 files;
TypeScript and production build passed. Targeted cases include standard/router
usage, custom/local negotiation, Google thinking/cache, Groq/Ollama envelopes,
split cumulative counters, cache normalization, null/malformed/overflow counts,
scope-guard cancellation, tail recovery, one specific retry, credential isolation,
duplicate event replacement, mixed providers and missing/partial UI states.

Tests use local deterministic fixtures and no real API keys or paid generation.
Actual availability still depends on the selected provider/model/server version
returning usage. A counter from a vendor is not independent verification of its
billing. Native output is in `src-tauri/target/provider-usage-native-final.log`;
source fingerprints/test metadata are in
[validation.json](research/provider-usage-2026-10-28/validation.json).

Primary contracts consulted:

- [OpenAI streaming usage](https://developers.openai.com/api/reference/resources/chat/subresources/completions/streaming-events)
- [Google usage metadata](https://ai.google.dev/api/generate-content#UsageMetadata)
- [OpenRouter streaming documentation source](https://github.com/OpenRouterTeam/docs/blob/main/api_reference/streaming.mdx)
- [Ollama compatible stream_options](https://docs.ollama.com/api/openai-compatibility)
- [Ollama counter definitions](https://docs.ollama.com/api/chat)
- [Groq API reference](https://console.groq.com/docs/api-reference)
- [Anthropic cumulative stream usage](https://platform.claude.com/docs/en/build-with-claude/streaming)
- [Anthropic cache input accounting](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)
