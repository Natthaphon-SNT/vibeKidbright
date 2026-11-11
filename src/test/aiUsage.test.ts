import { describe, expect, it } from 'vitest';
import { summarizeUsage, upsertUsageReport, usageDetails, usageReportFromEvent, usageText } from '../aiUsage';

const report = (id: string, provider: string, usage: unknown, status = 'reported') => usageReportFromEvent({ request_id: id, provider, model: 'test-model', usage, status }, 'legacy')!;
describe('provider-neutral token reports', () => {
    it('adds independent calls/providers but replaces duplicate request snapshots', () => {
        let reports = [report('1', 'Google', { input_tokens: 100, output_tokens: 10, total_tokens: 130, reasoning_tokens: 20 })];
        reports = upsertUsageReport(reports, report('2', 'Local', { input_tokens: 20, output_tokens: 5, total_tokens: 25 }));
        reports = upsertUsageReport(reports, report('1', 'Google', { input_tokens: 100, output_tokens: 15, total_tokens: 140, reasoning_tokens: 25 }));
        expect(reports).toHaveLength(2);
        expect(summarizeUsage(reports)).toMatchObject({ total_tokens: 165, reasoning_tokens: 25, providers: ['Google', 'Local'], partial: false });
        expect(usageDetails(reports)).toContain('Google / test-model');
    });
    it('never converts absent reports or partial fields to zero', () => {
        const reports = [report('1', 'Custom', { input_tokens: 7 }, 'partial'), report('2', 'Local', null, 'unavailable')];
        expect(usageText(reports, true)).toBe('Tokens this reply: ? · input 7 · output ? · partial usage');
        expect(usageText([report('3', 'Local', null, 'unavailable')], true)).toBe('Tokens: not reported by provider');
        expect(usageText([report('4', 'Google', null, 'interrupted')], true)).toBe('Tokens: unavailable (stream interrupted)');
        expect(usageText([], false)).toBe('Tokens: awaiting provider usage');
        expect(usageText([], true)).toBe('Tokens: not reported by provider');
    });
    it('retains explicit zeros and rejects negative, string or unsafe counters', () => {
        expect(usageText([report('1', 'Custom', { input_tokens: 0, output_tokens: 0, total_tokens: 0 })], true)).toBe('Tokens this reply: 0 · input 0 · output 0');
        expect(usageReportFromEvent({ usage: { input_tokens: -1, output_tokens: '4', total_tokens: Number.MAX_SAFE_INTEGER + 1 } }, 'bad')).toBeNull();
    });
    it('marks known sums incomplete when another call was interrupted or unreported', () => {
        const reports = [report('1', 'OpenRouter', { input_tokens: 100, output_tokens: 10, total_tokens: 110, cached_input_tokens: 40 }),
            report('2', 'Custom', null, 'interrupted')];
        expect(usageText(reports, true)).toBe('Tokens this reply: 110 · input 100 · output 10 · cached 40 · partial usage');
        expect(usageText([reports[0]], true, true)).toBe('Tokens this reply: 110 · input 100 · output 10 · cached 40 · partial usage');
    });
});
