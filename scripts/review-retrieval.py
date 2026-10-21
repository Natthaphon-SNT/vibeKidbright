"""Blind annotation gates and stratified summaries, without changing frozen qrels.

This does not run models or select retrieval hyperparameters. A reviewer packet is
data, never instructions. Independent review is self-attested, not authenticated.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics
import unicodedata

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / 'docs/ai-engineering'
HARDWARE_TOPICS = {'iaip', 'pins', 'mc3479', 'kxtj3', 'external-adc', 'oled', 'balance', 'formula', 'minibike', 'mai', 'schematic'}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_corpus(directory, declared):
    root = directory.resolve()
    for name, expected in declared.items():
        path = (root/name).resolve()
        if not path.is_relative_to(root) or digest(path) != expected:
            raise ValueError('Current source bytes no longer match the reviewed frozen corpus')


def groups(case):
    result = [case['language']]
    if case['id'].startswith('heldout-symbol-'):
        result.append('exact_api')
    if any(case['id'] == f'heldout-{topic}-{case["language"]}' for topic in HARDWARE_TOPICS):
        result.append('hardware')
    return result


def validate_review(packet, corpus_files):
    if packet.get('schema_version') != 1 or packet.get('status') != 'reviewed':
        raise ValueError('A draft/unreviewed packet cannot become independent evaluation evidence')
    author, reviewer = packet.get('query_author', '').strip(), packet.get('reviewer', '').strip()
    if not author or not reviewer or author.casefold() == reviewer.casefold() or reviewer.casefold() in {'assistant', 'codex', 'agent', 'gpt', 'unknown'}:
        raise ValueError('An identified independent reviewer is required')
    if packet.get('reviewer_attests_independence') is not True or not packet.get('reviewed_at') or not packet.get('review_notes'):
        raise ValueError('Review date, notes and independence attestation are required')
    queries = packet.get('queries', [])
    if len(queries) < 15:
        raise ValueError('At least 15 independently reviewed cases are required')
    seen = set()
    for case in queries:
        if not case.get('id') or case['id'] in seen or not case.get('query', '').strip():
            raise ValueError('Missing/duplicate case ID or query')
        seen.add(case['id'])
        if case.get('language') not in {'th', 'en', 'mixed', 'technical'} or case.get('category') not in {'documentation', 'hardware', 'exact_api'}:
            raise ValueError('Explicit language and category required')
        if not case.get('review_notes') or not isinstance(case.get('judgments'), dict) or set(case['judgments']) != set(corpus_files):
            raise ValueError('Each public corpus file needs an explicit judgment and case review notes')
        if any(value not in (0, 1, 2) or isinstance(value, bool) for value in case['judgments'].values()):
            raise ValueError('Judgments must be 0=irrelevant, 1=useful, 2=direct; unjudged is not irrelevant')
        if not any(value > 0 for value in case['judgments'].values()):
            raise ValueError('No-answer questions require a separate abstention evaluation')
    if not {'th', 'en', 'mixed', 'technical'} <= {q['language'] for q in queries} or not {'hardware', 'exact_api', 'documentation'} <= {q['category'] for q in queries}:
        raise ValueError('The reviewed set must cover all language/category strata')
    return [{'id': q['id'], 'query': q['query'], 'language': q['language'],
             'relevant_files': sorted(path for path, grade in q['judgments'].items() if grade > 0)} for q in queries]


def validate_development_review(packet, corpus_files, excluded_cases):
    """Separate reviewed development, not human-authored independent evaluation."""
    if packet.get('schema_version') != 2 or packet.get('purpose') != 'retrieval-development':
        raise ValueError('Explicit development review schema/purpose required')
    queries = packet.get('queries', [])
    if len(queries) < 20:
        raise ValueError('At least twenty reviewed development cases are required')
    normalize = lambda text: ' '.join(unicodedata.normalize('NFKC', text).casefold().split())
    excluded_ids = {case['id'] for case in excluded_cases}
    excluded_text = {normalize(case['query']) for case in excluded_cases}
    seen = set()
    for case in queries:
        text = normalize(case.get('query', ''))
        if case.get('id') in excluded_ids or text in excluded_text or text in seen:
            raise ValueError('Development queries must be distinct from each other and frozen/prior development cases')
        seen.add(text)
        if any(key in case for key in ('ranked_files', 'model_answer', 'relevant_files')):
            raise ValueError('Blind review packet must not contain rankings, model answers or preselected qrels')
    required = {'exact_symbol', 'natural_language', 'mixed_language', 'compiler_error', 'hardware'}
    if {case.get('query_kind') for case in queries} != required:
        raise ValueError('All five declared query kinds are required')
    # Reuse the existing strict identity, complete judgments, language/category,
    # notes and independence gate without weakening final-evaluation validation.
    compatible = {**packet, 'schema_version': 1}
    result = validate_review(compatible, corpus_files)
    for exported, original in zip(result, queries, strict=True):
        exported['query_kind'] = original['query_kind']
    return result


def summarize():
    dataset = DOCS / 'evaluation-heldout-v1.json'
    freeze = json.loads((DOCS / 'evaluation-heldout-v1-freeze.json').read_text('utf-8'))
    if digest(dataset) != freeze['sha256']:
        raise ValueError('Frozen dataset hash mismatch')
    cases = json.loads(dataset.read_text('utf-8'))
    comparison = json.loads((DOCS / 'evaluation-heldout-comparison.json').read_text('utf-8'))
    output = {'dataset_sha256': freeze['sha256'], 'measurement': 'Post-processing of existing measured runs; no model rerun, qrel edit or tuning',
              'annotation_status': 'assistant-authored, not independent human review', 'groups_overlap': True,
              'hardware_topics_declared': sorted(HARDWARE_TOPICS), 'runs': [],
              'uncertainties': ['Labels are representative file targets, not exhaustive graded judgments',
                                'Translated topic triplets share intent; queries are not statistically independent',
                                'Chunk top-five can repeat a file; file ranks are deduplicated without refill',
                                'Version/board metadata is often unknown; retrieval relevance is not hardware compatibility',
                                'Exact-API cases are nine probes; hardware category is a declared subset, not all physical functions']}
    for run in comparison['runs']:
        record = json.loads((DOCS / run['report']).read_text('utf-8'))
        if record['dataset_sha256'] != freeze['sha256'] or {q['id'] for q in record['queries']} != {q['id'] for q in cases}:
            raise ValueError('Ranker run does not match frozen cases')
        strata = {}
        for group in ['th', 'en', 'mixed', 'exact_api', 'hardware']:
            ids = {q['id'] for q in cases if group in groups(q)}
            values = [q for q in record['queries'] if q['id'] in ids]
            metrics = {key: statistics.mean(q['metrics'][key] for q in values) for key in record['macro_average']}
            if not all(math.isfinite(v) and 0 <= v <= 1 for v in metrics.values()):
                raise ValueError('Invalid measured metrics')
            latencies = sorted(q['latency_ms'] for q in values)
            strata[group] = {'queries': len(values), 'metrics': metrics, 'latency_median_ms': statistics.median(latencies),
                             'latency_p95_ms': latencies[int((len(latencies) - 1) * .95)]}
        output['runs'].append({'mode': run['mode'], 'strata': strata, 'cold_index_ms': run['index_ms'],
                               'process_resources': run['resources'], 'model_bytes': run['model_bytes']})
    installer = json.loads((DOCS / 'installer-validation-3.7.3.json').read_text('utf-8'))
    artifact = ROOT / installer['artifact']['path']
    if digest(artifact) != installer['artifact']['sha256']:
        raise ValueError('Preserved installer hash mismatch')
    output['preserved_installer'] = {**installer['artifact'], 'candidate_installers_built': False,
                                     'note': 'Actual NSIS bytes only; model assets are not compressed installer-size measurements'}
    (DOCS / 'retrieval-stratified-review.json').write_text(json.dumps(output, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'runs': len(output['runs']), 'groups': list(output['runs'][0]['strata']), 'frozen_hash_unchanged': True}))


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('summarize')
    validate = sub.add_parser('validate-review')
    validate.add_argument('packet', type=Path)
    validate.add_argument('--output', required=True, type=Path)
    development = sub.add_parser('validate-development')
    development.add_argument('packet', type=Path)
    development.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    if args.command == 'summarize':
        summarize()
    else:
        packet = json.loads(args.packet.read_text('utf-8'))
        corpus = json.loads((DOCS / 'evaluation-heldout-v1-freeze.json').read_text('utf-8'))['corpus_sha256']
        if packet.get('corpus_sha256') != corpus:
            raise ValueError('Review must identify the exact frozen corpus')
        verify_corpus(ROOT/'knowledge_base', corpus)
        if args.command == 'validate-development':
            frozen = json.loads((DOCS/'evaluation-heldout-v1.json').read_text('utf-8'))
            prior = json.loads((DOCS/'evaluation-expansion-dev-v1.json').read_text('utf-8'))
            cases = validate_development_review(packet, corpus, [*frozen, *prior])
        else:
            cases = validate_review(packet, corpus)
        if args.output.resolve().is_relative_to(DOCS.resolve()) and args.output.name.startswith('evaluation-heldout-v1'):
            raise ValueError('Cannot overwrite the existing frozen evaluation set')
        with args.output.open('x', encoding='utf-8') as out:
            out.write(json.dumps(cases, ensure_ascii=False, indent=2) + '\n')
        print('Validated self-attested independent review; freeze this new set before any ranker run')


if __name__ == '__main__':
    main()
