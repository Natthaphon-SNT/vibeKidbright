/** Actual provider counters only. Null means unknown, including partial streams. */
export type UsageStatus = 'reported' | 'partial' | 'unavailable' | 'interrupted';
export interface TokenCounters {
    input_tokens: number | null;
    output_tokens: number | null;
    total_tokens: number | null;
    cached_input_tokens: number | null;
    cache_write_input_tokens: number | null;
    reasoning_tokens: number | null;
}
export interface TokenUsageReport {
    requestId: string;
    provider: string;
    model: string;
    status: UsageStatus;
    usage: TokenCounters | null;
    derivedTotal: boolean;
}
const fields = ['input_tokens', 'output_tokens', 'total_tokens', 'cached_input_tokens', 'cache_write_input_tokens', 'reasoning_tokens'] as const;
const counter = (value: unknown): number | null => typeof value === 'number' && Number.isSafeInteger(value) && value >= 0 ? value : null;

export function usageReportFromEvent(data: any, legacyId: string): TokenUsageReport | null {
    if (!data || typeof data !== 'object') return null;
    let usage: TokenCounters | null = null;
    if (data.usage && typeof data.usage === 'object') {
        const normalized = Object.fromEntries(fields.map(field => [field, counter(data.usage[field])])) as unknown as TokenCounters;
        if (fields.some(field => normalized[field] !== null)) usage = normalized;
    }
    const status: UsageStatus = data.status === 'interrupted' ? 'interrupted' : !usage ? 'unavailable'
        : [usage.input_tokens, usage.output_tokens, usage.total_tokens].some(value => value === null) || data.status === 'partial' ? 'partial' : 'reported';
    // Unrecognized messages are not fabricated "unavailable" requests. Explicit
    // native status reports may legitimately have usage=null.
    if (!usage && !['reported', 'unavailable', 'interrupted', 'partial'].includes(data.status)) return null;
    return {
        requestId: typeof data.request_id === 'string' ? data.request_id : legacyId,
        provider: typeof data.provider === 'string' ? data.provider : 'Provider',
        model: typeof data.model === 'string' ? data.model : '', status, usage,
        derivedTotal: data.usage?.total_tokens_derived === true,
    };
}

export function upsertUsageReport(reports: TokenUsageReport[], report: TokenUsageReport): TokenUsageReport[] {
    const index = reports.findIndex(existing => existing.requestId === report.requestId);
    if (index < 0) return [...reports, report];
    return reports.map((existing, i) => i === index ? report : existing);
}

export function summarizeUsage(reports: TokenUsageReport[]) {
    const totals = Object.fromEntries(fields.map(field => {
        const known = reports.flatMap(report => report.usage?.[field] == null ? [] : [report.usage[field]!]);
        const sum = known.reduce((total, value) => total + value, 0);
        return [field, known.length && Number.isSafeInteger(sum) ? sum : null];
    })) as unknown as TokenCounters;
    return {
        ...totals, calls: reports.length,
        partial: reports.some(report => report.status !== 'reported'),
        interrupted: reports.filter(report => report.status === 'interrupted').length,
        providers: [...new Set(reports.map(report => report.provider))],
    };
}
const display = (value: number | null) => value === null ? '?' : value.toLocaleString();
export function usageText(reports: TokenUsageReport[], finished: boolean, requestFailed = false): string {
    if (!reports.length) return finished ? 'Tokens: not reported by provider' : 'Tokens: awaiting provider usage';
    const summary = summarizeUsage(reports);
    if ([summary.total_tokens, summary.input_tokens, summary.output_tokens, summary.cached_input_tokens, summary.cache_write_input_tokens, summary.reasoning_tokens].every(value => value === null)) {
        return summary.interrupted ? 'Tokens: unavailable (stream interrupted)' : 'Tokens: not reported by provider';
    }
    let text = `Tokens this reply: ${display(summary.total_tokens)} · input ${display(summary.input_tokens)} · output ${display(summary.output_tokens)}`;
    if (summary.cached_input_tokens !== null) text += ` · cached ${display(summary.cached_input_tokens)}`;
    if (summary.cache_write_input_tokens !== null) text += ` · cache write ${display(summary.cache_write_input_tokens)}`;
    if (summary.reasoning_tokens !== null) text += ` · thinking ${display(summary.reasoning_tokens)}`;
    if (summary.partial || requestFailed) text += ' · partial usage';
    return text;
}
export function usageDetails(reports: TokenUsageReport[]): string {
    return [
        'Actual counters from requests made by this app for this reply; not account-wide quota, other applications or a monetary bill.',
        'Unknown values are ?. Partial/interrupted or unreported calls make the known totals incomplete. Cache/thinking are not added twice; providers differ on output/thinking breakdown.',
        ...reports.map(report => `${report.provider}${report.model ? ` / ${report.model}` : ''}: ${display(report.usage?.total_tokens ?? null)} tokens (${report.status}${report.derivedTotal ? ', total computed from reported counters' : ''})`),
    ].join('\n');
}
