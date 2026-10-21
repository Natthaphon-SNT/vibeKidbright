import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,rm,symlink} from 'node:fs/promises';
import {resolve,sep} from 'node:path';
import {snapshotProject,sourceChanges,checkSearchHit,summarizeTrace,sha256,reviewBoundaryViolations,mechanicalEvidenceFailures} from '../lib/vibe-evidence.mjs';

const target=resolve('src-tauri/target');
async function fixture(fn) {
    const root=await mkdtemp(resolve(target,'vibe-evidence-test-'));
    try{await fn(root);}finally {
        assert.ok(resolve(root).startsWith(target+sep));
        await rm(root,{recursive:true,force:true});
    }
}
const hit=(file,content)=>({file,content,start_line:1,end_line:2,content_hash:sha256(content),citation:`${file}:1-2`,parser:'lexical-fallback'});

test('source evidence rejects invented/stale citations and recognizes clipped source excerpts',()=>fixture(async root=>{
    const content='Delay uses seconds.\nUse an output pointer.';
    await writeFile(resolve(root,'api.md'),content+'\n');
    const valid=hit('api.md',content);
    assert.equal((await checkSearchHit(valid,root)).source_valid,true);
    assert.equal((await checkSearchHit({...valid,content:content.slice(0,7)},root)).excerpt_truncated,true);
    for(const bad of [{file:'missing.md'},{file:'../api.md'},{file:'api.md:stream'},{end_line:99},{citation:'api.md:1-1'},{content_hash:sha256('stale')},{content:'Invented semantics'}]) {
        assert.equal((await checkSearchHit({...valid,...bad},root)).source_valid,false,JSON.stringify(bad));
    }
}));

test('completed model traces with invalid retrieval or invented citations fail mechanical checks',()=>fixture(async root=>{
    const content='Delay uses seconds.\nUse an output pointer.';
    await writeFile(resolve(root,'api.md'),content+'\n');
    const trace=result=>[
        {event:'ai-chat-tool-result',payload:{name:'knowledge_search',result}},
        {event:'ai-chat-done',payload:{text:'Reference api.md:1-2.'}},
    ];
    const requirements={tools:['knowledge_search'],cited_knowledge_hit:true};
    const valid=await summarizeTrace(trace([hit('api.md',content)]),root);
    assert.deepEqual(mechanicalEvidenceFailures(valid,requirements),[]);
    for(const result of [[{...hit('api.md',content),content_hash:sha256('stale')}],[],{error:'unavailable'},null,'Error: denied']) {
        const summary=await summarizeTrace(trace(result),root);
        assert.ok(mechanicalEvidenceFailures(summary,requirements).length,JSON.stringify(result));
        for(const key of ['retrieval_relevance','citation_support','project_understanding','compiler_validity'])assert.equal(summary[key],null);
    }
    const fabricated=await summarizeTrace([...trace([hit('api.md',content)]).slice(0,1),{event:'ai-chat-done',payload:{text:'Reference missing.md:1-2.'}}],root);
    assert.ok(mechanicalEvidenceFailures(fabricated,requirements).some(s=>s.includes('Unretrieved literal citation')));
    const fakeHash=await summarizeTrace(trace([{...hit('api.md',content),content_hash:sha256('stale')}]),root);
    assert.ok(mechanicalEvidenceFailures(fakeHash,requirements).some(s=>s.includes('Invalid knowledge source')));
}));

test('case contracts require requested tools and cited sources without granting quality grades',()=>fixture(async root=>{
    const content='void function(void);\n// definition';await writeFile(resolve(root,'api.h'),content);
    const events=[{event:'ai-chat-tool-result',payload:{name:'project_search',result:{results:[hit('api.h',content)]}}},
        {event:'ai-chat-done',payload:{text:'Reference api.h:1-2.'}}];
    const summary=await summarizeTrace(events,root,root);
    assert.deepEqual(mechanicalEvidenceFailures(summary,{tools:['project_search'],cited_project_hit:true}),[]);
    assert.deepEqual(mechanicalEvidenceFailures(summary,{tools:['read_file']}),['Required tool evidence missing: read_file']);
    assert.ok(mechanicalEvidenceFailures(await summarizeTrace(events,root)).includes('project_search has no authorized fixture root'));
    const empty=await summarizeTrace([{event:'ai-chat-done',payload:{text:'  '}}],root);
    assert.ok(mechanicalEvidenceFailures(empty).includes('Empty final answer'));
    const stopped=await summarizeTrace([{event:'ai-chat-error',payload:'cancelled'}],root);
    assert.ok(mechanicalEvidenceFailures(stopped).includes('No completion event'));
    assert.ok(mechanicalEvidenceFailures(stopped).includes('Native model/tool error event'));
}));

test('Tree-sitter source offsets are bytes and must agree with line ranges',()=>fixture(async root=>{
    const prefix='// ไทย\r\n',content='void app_main(void) {\r\n  return;\r\n}';
    await writeFile(resolve(root,'main.c'),prefix+content+'\r\n');
    const value={...hit('main.c',content),start_line:2,end_line:4,citation:'main.c:2-4',parser:'tree-sitter-c@0.24.2/tree-sitter@0.27.1',start_byte:Buffer.byteLength(prefix),end_byte:Buffer.byteLength(prefix+content)};
    assert.equal((await checkSearchHit(value,root)).source_valid,true);
    assert.equal((await checkSearchHit({...value,start_byte:prefix.length},root)).source_valid,false);
    assert.equal((await checkSearchHit({...value,start_line:1,citation:'main.c:1-4'},root)).source_valid,false);
}));

test('source manifests detect edits/additions/deletions and exclude build outputs',()=>fixture(async root=>{
    await mkdir(resolve(root,'main'));await mkdir(resolve(root,'build'));
    await writeFile(resolve(root,'main/main.c'),'void app_main(void) {}');
    await writeFile(resolve(root,'main/CMakeLists.txt'),'idf_component_register(SRCS "main.c")');
    await writeFile(resolve(root,'build/ignored.c'),'generated output');
    const before=await snapshotProject(root);
    assert.deepEqual(Object.keys(before),['main/CMakeLists.txt','main/main.c']);
    await writeFile(resolve(root,'main/main.c'),'void app_main(void) { return; }');
    await writeFile(resolve(root,'main/new.h'),'#pragma once');
    const after=await snapshotProject(root),changes=sourceChanges(before,after);
    assert.deepEqual(changes.map(c=>c.file),['main/main.c','main/new.h']);
    assert.equal(changes[1].before_sha256,null);
    assert.equal(sourceChanges(after,before)[1].after_sha256,null);
}));

test('linked source directories cannot smuggle private files into fixture evidence',()=>fixture(async root=>{
    await mkdir(resolve(root,'actual'));await writeFile(resolve(root,'actual/a.c'),'private');
    await symlink(resolve(root,'actual'),resolve(root,'main'),'junction');
    await assert.rejects(snapshotProject(root),/Linked fixture directory/);
    assert.equal((await checkSearchHit(hit('main/a.c','private'),root)).source_valid,false);
}));

test('trace checks never convert successful tool events into model/build success',()=>fixture(async root=>{
    const content='void api(void);\n// comment';await writeFile(resolve(root,'api.h'),content);
    const events=[{event:'ai-chat-tool-result',payload:JSON.stringify({name:'knowledge_search',id:'t1',result:[hit('api.h',content)]})},
        {event:'ai-chat-delta',payload:'Reference api.h:1-2 and missing.c:1-8.'},{event:'ai-chat-done',payload:'{}'}];
    const summary=await summarizeTrace(events,root);
    assert.equal(summary.search_hits[0].source_valid,true);
    assert.equal(summary.search_hits[0].literal_citation_in_answer,true);
    assert.deepEqual(summary.unmatched_literal_citations,['missing.c:1-8']);
    for(const key of ['retrieval_relevance','citation_support','project_understanding','compiler_validity'])assert.equal(summary[key],null);
    const interrupted=await summarizeTrace([{event:'ai-chat-error',payload:'stopped'}],root);
    assert.equal(interrupted.completed_event,false);assert.deepEqual(interrupted.errors,['stopped']);
    const final=await summarizeTrace([{event:'ai-chat-delta',payload:'100'},{event:'ai-chat-done',payload:'{"text":"final answer"}'}],root);
    assert.equal(final.streamed_text,'100');assert.equal(final.answer,'final answer');
    const write=[{event:'ai-chat-tool-start',payload:'{"name":"write_file"}'},{event:'ai-diff-pending',payload:'{}'}];
    assert.deepEqual(reviewBoundaryViolations(write,'read_only'),['write_file','ai-diff-pending']);
    assert.deepEqual(reviewBoundaryViolations(write,'review_required'),[]);
    assert.deepEqual(reviewBoundaryViolations([{event:'ai-chat-tool-start',payload:'{"name":"run_command"}'}],'review_required'),['run_command']);
    const projectTrace=await summarizeTrace([{event:'ai-chat-tool-result',payload:{name:'project_search',result:{results:[hit('api.h',content)]}}}],resolve(root,'not-a-corpus'),root);
    assert.equal(projectTrace.search_hits.length,0);assert.equal(projectTrace.project_hits[0].source_valid,true);
    assert.equal(projectTrace.project_understanding,null);
}));
