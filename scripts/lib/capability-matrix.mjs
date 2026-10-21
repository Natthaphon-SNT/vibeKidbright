// Qualification gates, not an automatic factual or AI-quality grader.
export const STATUSES=Object.freeze(['PASS','FAIL','NOT_EXECUTED','NEEDS_HUMAN_REVIEW']);
const define=(id,level,label,cases,acceptance)=>({id,level,label,required_cases:cases,acceptance});
export const CAPABILITIES=Object.freeze([
    define('retrieval_relevance','knowledge','Retrieval relevance',['grounding-th'],'Independent reviewer judges the retrieved evidence useful for the exact question; unjudged is not irrelevant.'),
    define('citation_existence','knowledge','Citation existence',['grounding-th','generation-th'],'Every cited literal file/range exists in the exact source revision; alternate formats require review.'),
    define('citation_support','knowledge','Citation factual support',['grounding-th','generation-th'],'Review each material claim against its cited excerpt; existence alone is insufficient.'),
    define('api_version_correctness','knowledge','ESP-IDF API/version correctness',['grounding-th','generation-th'],'Verify API spelling, units, constraints and configured SDK version; disclose unknowns.'),
    define('hardware_metadata','knowledge','Hardware metadata consistency',['generation-th'],'Match the actual fixture board/profile to authoritative metadata without inventing compatibility.'),
    define('project_identity','project','Correct project identification',['project-th'],'Identify the selected canonical public project, not another workspace.'),
    define('file_symbol_selection','project','Correct file and symbol selection',['project-th'],'Locate actual declarations, definitions and callers with source evidence.'),
    define('dependency_understanding','project','Understanding of dependencies',['project-th','generation-th'],'Explain actual CMake/component relationships and justify proposed dependency changes.'),
    define('patch_scope','project','Patch scope accuracy',['generation-th'],'Review the proposed diff against the requested existing-project task.'),
    define('unrelated_code_preserved','project','Preservation of unrelated code',['generation-th','rollback'],'Verify approved changes and rollback leave unrelated code intact; a pending diff alone is insufficient.'),
    define('compiler_classification','build','Accurate compiler error classification',['diagnostics'],'Compare model classification with actual diagnostics from the same source revision.'),
    define('focused_repair','build','Focused patch generation',['bounded-repair'],'Review the model patch against the actual fault, without unrelated changes.'),
    define('per_attempt_approval','build','Per-attempt user approval',['bounded-repair'],'Each application has separate project/revision-bound consumed consent; no harness approval.'),
    define('compilation_success','build','Compilation success',['compile','bounded-repair'],'Configured idf.py exits 0 for the reviewed source; record SDK, CDB and firmware hashes.'),
    define('rollback_correctness','build','Rollback correctness',['rollback'],'User-controlled Undo restores exact source hashes and preserves later conflicting edits/recovery records.'),
    define('bounded_attempts','build','Bounded repair attempts',['bounded-repair'],'Configured limit is 1..3, failed duplicate patches rejected, state survives restart.'),
]);
const levels=['knowledge','project','build'];
const hash=value=>typeof value==='string'&&/^[a-f0-9]{64}$/.test(value);
const nonempty=value=>typeof value==='string'&&!!value.trim();
export function aggregateStatus(values) {
    if(!values.length||values.some(v=>!STATUSES.includes(v)))throw new Error('Missing/invalid capability status');
    return ['FAIL','NOT_EXECUTED','NEEDS_HUMAN_REVIEW','PASS'].find(s=>values.includes(s));
}
export function reviewTemplate(evaluationSha256) {
    if(!hash(evaluationSha256))throw new Error('Evaluation SHA256 required');
    return {schema_version:1,status:'draft',evaluation_sha256:evaluationSha256,reviewer:'',reviewed_at:'',reviewer_attests_independence:false,
        note:'Blank assistant-prepared form, not an independent review. File hashes bind evidence but do not authenticate reviewer identity or actual execution.',
        judgments:CAPABILITIES.flatMap(c=>c.required_cases.map(case_id=>({criterion_id:c.id,case_id,status:null,notes:'',evidence_refs:[]})))};
}
function refs(items) {
    return Array.isArray(items)&&items.length>0&&items.every(r=>nonempty(r.path)&&hash(r.sha256));
}
export function buildCapabilityMatrix(report,evaluationSha256,{review=null,mechanical=null,manual=null}={}) {
    if(!hash(evaluationSha256))throw new Error('Evaluation SHA256 required');
    const observations=new Map(),annotations=new Map();
    if(manual) {
        if(manual.schema_version!==1||manual.evaluation_sha256!==evaluationSha256||manual.status!=='recorded')throw new Error('Manual evidence must bind this exact evaluation');
        for(const stage of manual.stages??[]) {
            if(!['compile','diagnostics','bounded-repair','rollback'].includes(stage.case_id)||observations.has(stage.case_id)||!['CAPTURED','FAILED'].includes(stage.status)||!refs(stage.evidence_refs)||!nonempty(stage.notes))throw new Error('Invalid/duplicate manual stage evidence');
            observations.set(stage.case_id,{...stage,id:stage.case_id});
        }
    }
    for(const entry of report.outputs??[]) {
        if(!nonempty(entry.id)||observations.has(entry.id))throw new Error('Duplicate/missing evaluation case');
        observations.set(entry.id,entry);
    }
    const real=report.evaluation_mode==='authorized_local_model'&&report.provider==='local'&&
        report.isolated_desktop_identifier==='th.nectec.vibekidbright.desktop-evaluation'&&Number.isSafeInteger(report.submitted_user_turns)&&report.submitted_user_turns>0;
    if(review) {
        if(review.schema_version!==1||review.status!=='reviewed'||review.evaluation_sha256!==evaluationSha256||!nonempty(review.reviewer)||['assistant','codex','agent','gpt','unknown'].includes(review.reviewer.trim().toLowerCase())||review.reviewer_attests_independence!==true||!nonempty(review.reviewed_at))throw new Error('Identified self-attested independent review bound to this evaluation required');
        for(const row of review.judgments??[]) {
            const criterion=CAPABILITIES.find(c=>c.id===row.criterion_id),key=row.criterion_id+':'+row.case_id;
            if(!criterion?.required_cases.includes(row.case_id)||annotations.has(key)||!['PASS','FAIL','NEEDS_HUMAN_REVIEW'].includes(row.status)||!nonempty(row.notes)||!refs(row.evidence_refs))throw new Error('Invalid/duplicate/unsubstantiated capability judgment');
            if(!real||!observations.has(row.case_id))throw new Error('Cannot grade an unexecuted model-dependent case');
            annotations.set(key,row);
        }
    }
    const checks=new Map();
    if(mechanical) {
        if(mechanical.schema_version!==1||mechanical.evaluation_sha256!==evaluationSha256)throw new Error('Mechanical observations must bind this evaluation');
        for(const check of mechanical.checks??[]) {
            if(!CAPABILITIES.some(c=>c.id===check.criterion_id)||checks.has(check.criterion_id)||!STATUSES.includes(check.status)||!nonempty(check.scope)||!refs(check.evidence_refs))throw new Error('Invalid mechanical observation');
            checks.set(check.criterion_id,check);
        }
    }
    const criteria=CAPABILITIES.map(c=>{
        const cases=c.required_cases.map(id=>{
            const entry=observations.get(id),annotation=annotations.get(c.id+':'+id);
            let status='NOT_EXECUTED',quality='NOT_EXECUTED',mechanics='NOT_EXECUTED';
            if(real&&entry) {
                const manualStage=!!entry.case_id;
                const collected=manualStage||entry.evidence?.completed_event===true&&Array.isArray(entry.mechanical_evidence_failures)&&Array.isArray(entry.review_boundary_violations)&&Array.isArray(entry.source_changes);
                const valid=collected&&entry.status==='CAPTURED'&&!entry.events_dropped&&!entry.error&&!(entry.mechanical_evidence_failures?.length)&&!(entry.review_boundary_violations?.length)&&!(entry.source_changes?.length);
                mechanics=valid?(manualStage?'NEEDS_HUMAN_REVIEW':'PASS'):'FAIL';quality='NEEDS_HUMAN_REVIEW';status=valid?'NEEDS_HUMAN_REVIEW':'FAIL';
                if(annotation) {
                    if(!valid&&annotation.status==='PASS')throw new Error('Cannot approve failed/incomplete mechanical evidence');
                    quality=annotation.status;status=annotation.status==='FAIL'?'FAIL':status;
                    if(valid)status=quality;
                }
            }
            return {case_id:id,status,ai_quality_status:quality,mechanical_status:mechanics,review:annotation??null};
        });
        return {...c,status:aggregateStatus(cases.map(x=>x.status)),ai_quality_status:aggregateStatus(cases.map(x=>x.ai_quality_status)),
            cases,mechanical_observation:checks.get(c.id)??{status:'NOT_EXECUTED',scope:'No separate deterministic observation supplied',evidence_refs:[]}};
    });
    return {schema_version:1,scope:'thai-delay-existing-project-v1; qualification and mechanical evidence are separate',evaluation_sha256:evaluationSha256,
        statuses:STATUSES,real_model_execution_reported:real,criteria,
        levels:Object.fromEntries(levels.map(level=>[level,aggregateStatus(criteria.filter(c=>c.level===level).map(c=>c.status))])),
        limitations:['CAPTURED never becomes an automatic AI-quality PASS. All model-dependent cases require actual reported execution and independent human judgment.',
            'A failure can concern mechanical evidence rather than inaccurate model content; consult per-case ai_quality_status.',
            'Manual stages and reviewer identity are self-attested, not authenticated. Referenced bytes must be separately hash-verified by the CLI.',
            'Deterministic tests, fixtures, compiler success and source hashes do not prove AI quality, hardware correctness or clean-VM behavior.']};
}
