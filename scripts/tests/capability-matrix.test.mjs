import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,writeFile,readFile,rm} from 'node:fs/promises';
import {resolve,sep} from 'node:path';
import {spawnSync} from 'node:child_process';
import {CAPABILITIES,aggregateStatus,buildCapabilityMatrix,reviewTemplate} from '../lib/capability-matrix.mjs';
import {sha256} from '../lib/vibe-evidence.mjs';
const hash='a'.repeat(64),ref={path:'src-tauri/target/fixture.json',sha256:'b'.repeat(64)};
const validEntry=id=>({id,status:'CAPTURED',evidence:{completed_event:true},mechanical_evidence_failures:[],review_boundary_violations:[],source_changes:[]});
const real=()=>({evaluation_mode:'authorized_local_model',provider:'local',isolated_desktop_identifier:'th.nectec.vibekidbright.desktop-evaluation',submitted_user_turns:1,outputs:[validEntry('grounding-th')]});
const review=()=>({schema_version:1,status:'reviewed',evaluation_sha256:hash,reviewer:'human-fixture-reviewer',reviewer_attests_independence:true,reviewed_at:'2026-10-19',judgments:[{criterion_id:'retrieval_relevance',case_id:'grounding-th',status:'PASS',notes:'Synthetic fixture judgment, not actual annotation',evidence_refs:[ref]}]});
test('all sixteen criteria distinguish preparation, mechanical observations and model quality',()=>{
    const mechanical={schema_version:1,evaluation_sha256:hash,checks:[{criterion_id:'per_attempt_approval',status:'PASS',scope:'Deterministic test fixture only',evidence_refs:[ref]}]};
    const m=buildCapabilityMatrix({evaluation_mode:'preparation',status:'CAPTURED'},hash,{mechanical});
    assert.equal(m.criteria.length,16);assert.equal(new Set(CAPABILITIES.map(c=>c.id)).size,16);
    assert.ok(m.criteria.every(c=>c.status==='NOT_EXECUTED'&&c.ai_quality_status==='NOT_EXECUTED'));
    assert.equal(m.criteria.find(c=>c.id==='per_attempt_approval').mechanical_observation.status,'PASS');
    for(const mode of ['preparation','native_preflight',undefined])assert.ok(buildCapabilityMatrix({...real(),evaluation_mode:mode},hash).criteria.every(c=>c.status==='NOT_EXECUTED'));
});
test('CAPTURED does not grant AI PASS and missing workflow stages remain unexecuted',()=>{
    const m=buildCapabilityMatrix(real(),hash),row=m.criteria.find(c=>c.id==='retrieval_relevance');
    assert.equal(row.status,'NEEDS_HUMAN_REVIEW');assert.equal(row.ai_quality_status,'NEEDS_HUMAN_REVIEW');assert.equal(row.cases[0].mechanical_status,'PASS');
    assert.equal(m.levels.knowledge,'NOT_EXECUTED');assert.equal(m.levels.build,'NOT_EXECUTED');
    const bad=real();bad.outputs[0].mechanical_evidence_failures=['stale hash'];
    const fail=buildCapabilityMatrix(bad,hash).criteria[0];assert.equal(fail.status,'FAIL');assert.equal(fail.ai_quality_status,'NEEDS_HUMAN_REVIEW');
    assert.throws(()=>buildCapabilityMatrix(bad,hash,{review:review()}),/Cannot approve/);
    const bare=real();bare.outputs=[{id:'grounding-th',status:'CAPTURED'}];assert.equal(buildCapabilityMatrix(bare,hash).criteria[0].status,'FAIL');
});
test('human judgments require matching evidence identity, independent attestation and actual case capture',()=>{
    const m=buildCapabilityMatrix(real(),hash,{review:review()});assert.equal(m.criteria[0].ai_quality_status,'PASS');
    const fail=review();fail.judgments[0].status='FAIL';assert.equal(buildCapabilityMatrix(real(),hash,{review:fail}).criteria[0].status,'FAIL');
    for(const patch of [{evaluation_sha256:'c'.repeat(64)},{reviewer:'codex'},{reviewer_attests_independence:'true'},{status:'draft'}])assert.throws(()=>buildCapabilityMatrix(real(),hash,{review:{...review(),...patch}}));
    assert.throws(()=>buildCapabilityMatrix({evaluation_mode:'preparation'},hash,{review:review()}),/unexecuted/);
    const unexecuted=review();unexecuted.judgments[0].case_id='generation-th';unexecuted.judgments[0].criterion_id='patch_scope';assert.throws(()=>buildCapabilityMatrix(real(),hash,{review:unexecuted}),/unexecuted/);
    assert.equal(reviewTemplate(hash).judgments.every(j=>j.status===null),true);
    assert.equal(aggregateStatus(['PASS','NEEDS_HUMAN_REVIEW']),'NEEDS_HUMAN_REVIEW');assert.equal(aggregateStatus(['PASS','NOT_EXECUTED']),'NOT_EXECUTED');
});
test('manual build stages remain ungraded, hash-bound observations, not automatic compilation passes',()=>{
    const manual={schema_version:1,status:'recorded',evaluation_sha256:hash,stages:[{case_id:'compile',status:'CAPTURED',notes:'Synthetic manual fixture',evidence_refs:[ref]}]};
    const m=buildCapabilityMatrix(real(),hash,{manual});const c=m.criteria.find(c=>c.id==='compilation_success').cases[0];
    assert.equal(c.status,'NEEDS_HUMAN_REVIEW');assert.equal(c.mechanical_status,'NEEDS_HUMAN_REVIEW');
    assert.equal(buildCapabilityMatrix({evaluation_mode:'preparation'},hash,{manual}).levels.build,'NOT_EXECUTED');
    assert.throws(()=>buildCapabilityMatrix(real(),hash,{manual:{...manual,evaluation_sha256:'c'.repeat(64)}}));
});
test('offline CLI preserves reports and rejects stale evidence references before writing output',async()=>{
    const target=resolve('src-tauri/target'),dir=await mkdtemp(resolve(target,'capability-unit-'));
    try {
        const evidence=resolve(dir,'report.json'),output=resolve(dir,'matrix.json'),packet=resolve(dir,'mechanical.json'),proof=resolve(dir,'proof.txt');
        const body=JSON.stringify({evaluation_mode:'preparation'});await writeFile(evidence,body);await writeFile(proof,'actual fixture bytes');
        const run=(...args)=>spawnSync(process.execPath,['scripts/evaluate-capabilities.mjs','--evidence',evidence,'--output',output,...args],{encoding:'utf8'});
        assert.equal(run().status,0);const before=await readFile(output);assert.notEqual(run().status,0);assert.deepEqual(await readFile(output),before);
        await rm(output);
        await writeFile(packet,JSON.stringify({schema_version:1,evaluation_sha256:sha256(body),checks:[{criterion_id:'per_attempt_approval',status:'PASS',scope:'fixture',evidence_refs:[{path:proof.slice(process.cwd().length+1).split(sep).join('/'),sha256:sha256('wrong bytes')}]}]}));
        const bad=run('--mechanical',packet);assert.notEqual(bad.status,0);assert.match(bad.stderr,/hash mismatch/);await assert.rejects(readFile(output));
        const outside=spawnSync(process.execPath,['scripts/evaluate-capabilities.mjs','--evidence',evidence,'--output','../outside.json'],{encoding:'utf8'});assert.notEqual(outside.status,0);
    }finally{assert.ok(dir.startsWith(target+sep));await rm(dir,{recursive:true,force:true});}
});
