// Offline evidence checks, not model-quality grades. Only the caller's public
// fixture and bundled corpus are inspected; linked/escaping files are rejected.
import {createHash} from 'node:crypto';
import {readFile,readdir,lstat,realpath} from 'node:fs/promises';
import {resolve,sep,isAbsolute} from 'node:path';

export const sha256 = data => createHash('sha256').update(data).digest('hex');
const inside = (root,path) => path===root || path.startsWith(root+sep);
async function safeFile(root,name,maxBytes=1024*1024) {
    if(typeof name!=='string'||!name||isAbsolute(name)||/[\\:\0]/.test(name)||name.split('/').some(p=>p==='..'||p==='.'||!p))throw new Error('Unsafe evidence path');
    const base=await realpath(root),path=resolve(base,name);
    if(!inside(base,path)||!inside(base,await realpath(path)))throw new Error('Evidence path escapes root');
    // Reject symlinks/junctions in every segment, even ones targeting this root.
    let part=base;
    for(const segment of name.split('/')){part=resolve(part,segment);if((await lstat(part)).isSymbolicLink())throw new Error('Linked evidence path');}
    const stat=await lstat(path);
    if(!stat.isFile()||stat.size>maxBytes)throw new Error('Evidence file missing or exceeds size limit');
    return {path,bytes:await readFile(path)};
}
export {safeFile as readEvidenceFile};

export async function snapshotProject(project) {
    const base=await realpath(project),files={};
    async function walk(dir,depth=0) {
        if(depth>12)throw new Error('Fixture depth limit');
        for(const entry of await readdir(resolve(base,dir),{withFileTypes:true})) {
            const name=dir?dir+'/'+entry.name:entry.name;
            if(entry.isSymbolicLink())throw new Error('Linked fixture entry');
            if(entry.isDirectory()){await walk(name,depth+1);continue;}
            const {bytes}=await safeFile(base,name);files[name]=sha256(bytes);
            if(Object.keys(files).length>256)throw new Error('Fixture file limit');
        }
    }
    // This is a source/config snapshot, not a build-output or OS filesystem scan.
    for(const dir of ['main','components']) {
        try{if((await lstat(resolve(base,dir))).isSymbolicLink())throw new Error('Linked fixture directory');await walk(dir);}
        catch(error){if(error.code!=='ENOENT')throw error;}
    }
    for(const name of ['CMakeLists.txt','sdkconfig','sdkconfig.defaults','kidbright-project.json']) {
        try{files[name]=sha256((await safeFile(base,name)).bytes);}catch(error){if(error.code!=='ENOENT')throw error;}
    }
    return Object.fromEntries(Object.entries(files).sort(([a],[b])=>a.localeCompare(b)));
}

export function sourceChanges(before,after) {
    return [...new Set([...Object.keys(before),...Object.keys(after)])].sort()
        .filter(name=>before[name]!==after[name])
        .map(file=>({file,before_sha256:before[file]??null,after_sha256:after[file]??null}));
}

export function decodePayload(value) {
    if(typeof value!=='string')return value;
    try{return JSON.parse(value);}catch{return value;}
}

export async function checkSearchHit(hit,corpus) {
    const result={citation:hit?.citation??null,file:hit?.file??null,source_valid:false,claim_support:null};
    try {
        const {bytes}=await safeFile(corpus,hit.file),text=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes);
        const lines=text.split(/\r?\n/);if(lines.at(-1)==='')lines.pop();
        if(!Number.isSafeInteger(hit.start_line)||!Number.isSafeInteger(hit.end_line)||hit.start_line<1||hit.end_line<hit.start_line||hit.end_line>lines.length)throw new Error('Invalid source line range');
        if(hit.citation!==`${hit.file}:${hit.start_line}-${hit.end_line}`)throw new Error('Citation differs from structured range');
        let excerpt;
        if(hit.parser?.startsWith('tree-sitter-c@')) {
            if(!Number.isSafeInteger(hit.start_byte)||!Number.isSafeInteger(hit.end_byte)||hit.start_byte<0||hit.end_byte<=hit.start_byte||hit.end_byte>bytes.length)throw new Error('Invalid source byte range');
            const startLine=bytes.subarray(0,hit.start_byte).filter(b=>b===10).length+1;
            const endLine=bytes.subarray(0,hit.end_byte-1).filter(b=>b===10).length+1;
            if(startLine!==hit.start_line||endLine!==hit.end_line)throw new Error('Source byte and line ranges disagree');
            excerpt=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes.subarray(hit.start_byte,hit.end_byte));
        }else excerpt=lines.slice(hit.start_line-1,hit.end_line).join('\n');
        if(sha256(excerpt)!==hit.content_hash)throw new Error('Source excerpt hash differs');
        if(typeof hit.content!=='string'||!hit.content||!excerpt.startsWith(hit.content))throw new Error('Returned content differs from source');
        result.source_valid=true;result.source_sha256=sha256(bytes);
        result.excerpt_truncated=excerpt!==hit.content;
    }catch(error){result.error=error.message;}
    return result;
}

export async function summarizeTrace(events,corpus,project=null) {
    const tools=[],hits=[],projectHits=[],errors=[],evidenceErrors=[],answers=[];let done=false,finalAnswer=null;
    for(const event of events) {
        const data=decodePayload(event.payload);
        if(event.event==='ai-chat-tool-result') {
            tools.push({name:data?.name??null,id:data?.id??null});
            if(data?.name==='knowledge_search') {
                const value=decodePayload(data.result);
                if(Array.isArray(value))for(const hit of value)hits.push(await checkSearchHit(hit,corpus));
                else evidenceErrors.push('knowledge_search did not return a hit array');
            }
            if(data?.name==='project_search') {
                const value=decodePayload(data.result);
                if(!project)evidenceErrors.push('project_search has no authorized fixture root');
                else if(Array.isArray(value?.results))for(const hit of value.results)projectHits.push(await checkSearchHit(hit,project));
                else evidenceErrors.push('project_search did not return structured results');
            }
        }
        if(event.event==='ai-chat-delta'&&typeof event.payload==='string')answers.push(event.payload);
        if(event.event==='ai-chat-done'){done=true;if(typeof data?.text==='string')finalAnswer=data.text;}
        if(event.event==='ai-chat-error')errors.push(data);
    }
    const text=finalAnswer??answers.join('');
    for(const hit of [...hits,...projectHits])hit.literal_citation_in_answer=typeof hit.citation==='string'&&text.includes(hit.citation);
    // A bare filename:line range is checked for presence among retrieved hits.
    // Other citation formats and factual support still require human review.
    const citations=[...new Set(text.match(/[\w./-]+\.(?:c|h|md|txt|cmake):\d+-\d+/g)??[])];
    return {completed_event:done,errors,evidence_errors:evidenceErrors,tool_results:tools,search_hits:hits,project_hits:projectHits,answer:text,streamed_text:answers.join(''),
        unmatched_literal_citations:citations.filter(c=>![...hits,...projectHits].some(h=>h.citation===c)),
        retrieval_relevance:null,citation_support:null,project_understanding:null,compiler_validity:null,
        note:'Source integrity and literal citation presence are mechanical checks. They do not grade relevance, factual support, project understanding or generated C.'};
}

// These are evidence/instruction checks, never an automatic factual-support or
// model-quality grade. Invalid source evidence must not be labelled CAPTURED.
export function mechanicalEvidenceFailures(summary,requirements={}) {
    const failures=[...summary.evidence_errors];
    if(!summary.completed_event)failures.push('No completion event');
    if(summary.errors.length)failures.push('Native model/tool error event');
    if(!summary.answer.trim())failures.push('Empty final answer');
    for(const [scope,hits] of [['knowledge',summary.search_hits],['project',summary.project_hits]]) {
        for(const hit of hits)if(!hit.source_valid)failures.push(`Invalid ${scope} source: ${hit.file??'<missing>'}: ${hit.error}`);
    }
    for(const citation of summary.unmatched_literal_citations)failures.push(`Unretrieved literal citation: ${citation}`);
    for(const tool of requirements.tools??[])if(!summary.tool_results.some(t=>t.name===tool))failures.push(`Required tool evidence missing: ${tool}`);
    for(const [flag,hits] of [['cited_knowledge_hit',summary.search_hits],['cited_project_hit',summary.project_hits]]) {
        if(requirements[flag]&&!hits.some(h=>h.source_valid&&h.literal_citation_in_answer))failures.push(`Required valid cited source missing: ${flag}`);
    }
    return [...new Set(failures)];
}

export function reviewBoundaryViolations(events,kind) {
    const forbidden=new Set(['run_command','create_project_workspace','install_idf_library','web_search']);
    if(kind==='read_only')forbidden.add('write_file');
    const violations=[];
    for(const event of events) {
        const data=decodePayload(event.payload);
        if(['ai-chat-tool-start','ai-chat-tool-result'].includes(event.event)&&forbidden.has(data?.name))violations.push(data.name);
        if(kind==='read_only'&&['ai-diff-pending','ai-project-review-pending'].includes(event.event))violations.push(event.event);
    }
    return [...new Set(violations)];
}
