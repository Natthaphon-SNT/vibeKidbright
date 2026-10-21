"""Report paired fixed-policy runs; never changes a ranker or relevance label."""
from pathlib import Path
import hashlib
import json
import statistics

ROOT=Path(__file__).resolve().parents[1]
DOCS=ROOT/'docs/ai-engineering'
SHA=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()

def summarize(report):
    result={}
    for lang in sorted({q['language'] for q in report['queries']}):
        rows=[q for q in report['queries'] if q['language']==lang]
        latencies=sorted(q['latency_ms'] for q in rows)
        result[lang]={'queries':len(rows),'metrics':{key:statistics.mean(q['metrics'][key] for q in rows) for key in report['macro_average']},'latency_ms':{'median':statistics.median(latencies),'p95':latencies[int((len(latencies)-1)*.95)]}}
    return result

def main():
    read=lambda n:json.loads((DOCS/n).read_text('utf-8'))
    freeze=read('evaluation-heldout-v1-freeze.json')
    assert SHA(DOCS/'evaluation-heldout-v1.json')==freeze['sha256']
    decision=read('expansion-development-decision.json')
    for p,h in decision['hashes'].items():assert SHA(ROOT/p)==h,p
    before=read('expansion-heldout-baseline.json');after=read('expansion-heldout-expanded.json')
    assert before['dataset_sha256']==after['dataset_sha256']==freeze['sha256']
    assert before['model']==after['model']
    pairs=list(zip(before['queries'],after['queries']))
    assert all(a['id']==b['id'] and a['query']==b['query'] and a['relevant_files']==b['relevant_files'] for a,b in pairs)
    assert all(q['query_plan']['original']==q['query'] for q in after['queries'])
    exact=[(a,b) for a,b in pairs if a['query_plan']['exact_api']]
    assert all(a['ranked_files']==b['ranked_files'] for a,b in exact)
    result={'status':'MEASURED_NOT_INDEPENDENTLY_REVIEWED','dataset_sha256':freeze['sha256'],'query_count':len(pairs),'model_unchanged':True,'production_embedding_model_replaced':False,'vocabulary_or_weight_tuned_after_frozen_run':False,'development_decision':'expansion-development-decision.json','runner_executable_sha256':SHA(ROOT/'src-tauri/target/debug/examples/knowledge_eval.exe'),'runner_source_sha256':SHA(ROOT/'src-tauri/examples/knowledge_eval.rs'),'original_queries_and_qrels_preserved':True,'exact_api_queries':len(exact),'exact_api_rankings_unchanged':True,'before':{'macro':before['macro_average'],'language':summarize(before),'index_ms':before['index_latency_ms']},'after':{'macro':after['macro_average'],'language':summarize(after),'index_ms':after['index_latency_ms']},'paired_recall_regressions':[{'id':a['id'],'language':a['language'],'before':a['metrics']['recall_at_5'],'after':b['metrics']['recall_at_5']} for a,b in pairs if b['metrics']['recall_at_5']<a['metrics']['recall_at_5']],'paired_recall_improvements':sum(b['metrics']['recall_at_5']>a['metrics']['recall_at_5'] for a,b in pairs),'limits':['Assistant-authored non-exhaustive file labels; no human-independent accuracy claim','Single runs on a development host with concurrent tasks; latency is diagnostic, not controlled performance evidence','No new per-process RAM sampling in this paired run; historical model/resource comparison remains separate','Unfiltered frozen evaluation includes unknown board/SDK metadata, not compatibility proof; native regression separately tests strict filters','Expanded query channel is lexical only; original query embedding, text and API search are unchanged','Protocol deviation and corrected disjoint development controls are disclosed in development decision']}
    (DOCS/'query-expansion-comparison.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result,indent=2))

if __name__=='__main__':main()
