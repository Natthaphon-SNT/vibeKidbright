"""Read-only paired-run diagnosis. Never exports ranker rules or edits qrels."""
import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import re
import statistics

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / 'docs/ai-engineering'
HARDWARE = {'iaip', 'pins', 'mc3479', 'kxtj3', 'external-adc', 'oled', 'balance', 'formula', 'minibike', 'mai', 'schematic'}
METRICS = ('recall_at_5', 'mrr', 'ndcg_at_5', 'precision_at_5')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def kinds(case):
    query = case['query']
    exact = bool(re.fullmatch(r'[A-Za-z][A-Za-z0-9]*_[A-Za-z0-9_]+', query))
    result = ['exact_symbol'] if exact else ['mixed_language' if re.search(r'[\u0e00-\u0e7f]', query) and re.search(r'[A-Za-z]', query) else 'natural_language']
    if re.search(r'error:|undefined reference|fatal error|not declared', query, re.I):
        result.append('compiler_error')
    if any(case['id'] == f'heldout-{topic}-{case["language"]}' for topic in HARDWARE):
        result.append('hardware')
    return result


def metrics(files, relevant):
    # Match the frozen runner: only the returned five chunks, then deduplicate;
    # do not refill document ranks from an unseen larger candidate list.
    ranked = list(dict.fromkeys(files[:5]))
    relevant = set(relevant)
    positions = [i for i, name in enumerate(ranked) if name in relevant]
    ideal = sum(1 / math.log2(i + 2) for i in range(min(5, len(relevant))))
    return {'recall_at_5': len(positions) / max(1, len(relevant)), 'precision_at_5': len(positions) / 5,
            'mrr': 1 / (positions[0] + 1) if positions else 0,
            'ndcg_at_5': sum(1 / math.log2(i + 2) for i in positions) / ideal if ideal else 0}


def diagnose(cases, before, after, corpus):
    expected = {case['id']: case for case in cases}
    if len(expected) != len(cases):
        raise ValueError('Duplicate dataset ID')
    runs = []
    for run in (before, after):
        indexed = {row['id']: row for row in run['queries']}
        if set(indexed) != set(expected) or len(indexed) != len(run['queries']):
            raise ValueError('Run case IDs differ from the frozen dataset')
        for key, row in indexed.items():
            case = expected[key]
            if any(row[field] != case[field] for field in ('query', 'language', 'relevant_files')):
                raise ValueError('Run query/labels differ from frozen data')
            if row.get('query_plan', {}).get('original') != row['query']:
                raise ValueError('Original query was not preserved')
            score = metrics(row['ranked_files'], case['relevant_files'])
            if any(not math.isclose(score[m], row['metrics'][m], abs_tol=1e-10) for m in METRICS):
                raise ValueError('Stored metrics differ from frozen metric definition')
        runs.append(indexed)
    rows = []
    for case in cases:
        a, b = (run[case['id']] for run in runs)
        flags = []
        if b['metrics']['recall_at_5'] == 0:
            flags.append('no_qrel_hit_in_returned_top5')
        duplicate_slots = len(b['ranked_files']) - len(set(b['ranked_files']))
        if duplicate_slots:
            flags.append('duplicate_file_slots')
        if case['language'] == 'th' and not b['query_plan']['concepts']:
            flags.append('thai_query_without_fixed_alias_match')
        missing = [name for name in case['relevant_files'] if not (corpus / name).is_file()]
        if missing:
            flags.append('qrel_file_missing_from_corpus')
        delta = {m: b['metrics'][m] - a['metrics'][m] for m in METRICS}
        if delta['mrr'] < -1e-10:
            flags.append('reciprocal_rank_regression')
        rows.append({'id': case['id'], 'query': case['query'], 'language': case['language'], 'kinds': kinds(case),
                     'qrels': case['relevant_files'], 'missing_qrel_files': missing,
                     'before_files': a['ranked_files'], 'after_files': b['ranked_files'],
                     'before_metrics': a['metrics'], 'after_metrics': b['metrics'], 'delta': delta,
                     'expansion_plan': b['query_plan'], 'duplicate_file_slots': duplicate_slots,
                     'diagnostic_flags': flags, 'before_latency_ms': a['latency_ms'], 'after_latency_ms': b['latency_ms']})
    def group(values):
        if not values:
            return {'queries': 0, 'metrics': None, 'latency_ms': None, 'coverage_gap': True}
        return {'queries': len(values), 'before': {m: statistics.mean(r['before_metrics'][m] for r in values) for m in METRICS},
                'after': {m: statistics.mean(r['after_metrics'][m] for r in values) for m in METRICS},
                'regressed_mrr_cases': [r['id'] for r in values if 'reciprocal_rank_regression' in r['diagnostic_flags']],
                'latency_ms': {phase: {'median': statistics.median(r[phase + '_latency_ms'] for r in values),
                                     'p95': sorted(r[phase + '_latency_ms'] for r in values)[int((len(values)-1)*.95)]} for phase in ('before', 'after')}}
    return {'queries': rows, 'language': {lang: group([r for r in rows if r['language'] == lang]) for lang in sorted({r['language'] for r in rows})},
            'query_kinds': {kind: group([r for r in rows if kind in r['kinds']]) for kind in ('exact_symbol', 'natural_language', 'mixed_language', 'compiler_error', 'hardware')},
            'flags': dict(Counter(flag for r in rows for flag in r['diagnostic_flags'])),
            'limits': ['Diagnosis of fixed historical runs only, not a new accuracy measurement or ranker change',
                       'All qrels are assistant-authored and non-exhaustive; flags are observations, not established causal failure labels',
                       'Kinds overlap; hardware membership is the previously declared topic set, not inferred hardware compatibility',
                       'No compiler-error probes exist in this frozen set; do not fabricate category metrics',
                       'No refill after duplicate top-five chunks; missing counterpart files may still share relevant evidence',
                       'Single-host historical latency is uncontrolled; paired expansion RAM was not measured',
                       'Corpus contains examples/helpers and incomplete authoritative SDK documentation',
                       'Do not derive expansion vocabulary or tune ranker parameters from these final labels']}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    read = lambda name: json.loads((DOCS/name).read_text(encoding='utf-8'))
    freeze = read('evaluation-heldout-v1-freeze.json')
    before, after = read('expansion-heldout-baseline.json'), read('expansion-heldout-expanded.json')
    if digest(DOCS/'evaluation-heldout-v1.json') != freeze['sha256'] or any(run['dataset_sha256'] != freeze['sha256'] for run in (before, after)):
        raise ValueError('Frozen dataset hash mismatch')
    if before['model'] != after['model']:
        raise ValueError('Paired runs use different models')
    result = diagnose(read('evaluation-heldout-v1.json'), before, after, ROOT/'knowledge_base')
    result.update(status='DIAGNOSED_NOT_HUMAN_REVIEWED', dataset_sha256=freeze['sha256'], ranker_changed=False,
                  historical_multilingual_resources=read('evaluation-heldout-comparison.json')['runs'],
                  source_sha256={str(p.relative_to(ROOT)).replace('\\', '/'): digest(p) for p in [Path(__file__), DOCS/'evaluation-heldout-v1.json', DOCS/'expansion-heldout-baseline.json', DOCS/'expansion-heldout-expanded.json']})
    output = args.output.resolve()
    if not output.is_relative_to(ROOT.resolve()):
        raise ValueError('Output must stay in workspace')
    with output.open('x', encoding='utf-8') as file:
        json.dump(result, file, ensure_ascii=False, indent=2); file.write('\n')
    print(json.dumps({'queries': len(result['queries']), 'flags': result['flags'], 'compiler_error_cases': result['query_kinds']['compiler_error']['queries']}))


if __name__ == '__main__':
    main()
