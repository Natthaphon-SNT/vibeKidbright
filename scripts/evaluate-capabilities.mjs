// Offline, additive evidence/review qualification. Never invokes a provider,
// compiler, patch command or shell. Existing reports are not rewritten.
import {realpath,lstat,open} from 'node:fs/promises';
import {resolve,relative,dirname,sep} from 'node:path';
import {readEvidenceFile,sha256} from './lib/vibe-evidence.mjs';
import {buildCapabilityMatrix,reviewTemplate} from './lib/capability-matrix.mjs';
const root=await realpath(process.cwd()),args=process.argv.slice(2),options=new Map();
for(let i=0;i<args.length;i+=2) {
    if(!['--evidence','--review','--mechanical','--manual','--output','--template'].includes(args[i])||!args[i+1]||options.has(args[i]))throw new Error('Use unique --evidence/--output and optional --review/--mechanical/--manual/--template review');
    options.set(args[i],args[i+1]);
}
if(!options.has('--evidence')||!options.has('--output')||options.has('--template')&&options.get('--template')!=='review')throw new Error('Explicit evidence and fresh output required');
const name=path=>relative(root,resolve(root,path)).split(sep).join('/');
async function load(path) {
    const file=name(path);
    if(!/^(docs\/ai-engineering|src-tauri\/target)\//.test(file))throw new Error('Evaluation inputs must stay in engineering evidence or ignored target');
    const {bytes}=await readEvidenceFile(root,file,16*1024*1024);
    return {data:JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(bytes)),sha256:sha256(bytes),path:file};
}
const source=await load(options.get('--evidence')),extras={},inputs=[{path:source.path,sha256:source.sha256}];
const verifiedRefs=new Map();let proofBytes=0;
for(const key of ['review','mechanical','manual'])if(options.has('--'+key)) {
    const loaded=await load(options.get('--'+key));extras[key]=loaded.data;inputs.push({path:loaded.path,sha256:loaded.sha256});
    const rows=loaded.data.judgments??loaded.data.checks??loaded.data.stages??[];
    if(!Array.isArray(rows)||rows.length>64)throw new Error('Review observation limit');
    if(rows.some(row=>row.evidence_refs!==undefined&&(!Array.isArray(row.evidence_refs)||row.evidence_refs.length>16)))throw new Error('Evidence reference limit');
    for(const row of rows)for(const ref of row.evidence_refs??[]) {
        if(!/^(docs\/ai-engineering|src-tauri\/target|resources\/knowledge_base|knowledge_base)\//.test(ref.path))throw new Error('Evidence reference outside allowed public/evaluation roots');
        if(verifiedRefs.has(ref.path)) {if(verifiedRefs.get(ref.path)!==ref.sha256)throw new Error('Conflicting evidence reference hashes');continue;}
        const {bytes}=await readEvidenceFile(root,ref.path,16*1024*1024);
        proofBytes+=bytes.length;if(proofBytes>64*1024*1024)throw new Error('Evidence proof byte budget');
        if(sha256(bytes)!==ref.sha256)throw new Error('Referenced evidence hash mismatch');
        verifiedRefs.set(ref.path,ref.sha256);
    }
}
const result=options.has('--template')?reviewTemplate(source.sha256):buildCapabilityMatrix(source.data,source.sha256,extras);
result.hash_scope='Exact --evidence file bytes';
result.inputs=inputs;
const output=name(options.get('--output'));
if(!/^(docs\/ai-engineering|src-tauri\/target)\/.+\.json$/.test(output))throw new Error('Output must be a fresh engineering/target JSON file');
if(source.data.evaluation_mode==='authorized_local_model'&&!output.startsWith('src-tauri/target/'))throw new Error('Actual-model review evidence must remain under ignored target');
const parent=await realpath(dirname(resolve(root,output)));
if(parent!==root&&!parent.startsWith(root+sep))throw new Error('Output parent escapes workspace');
let path=root;
for(const segment of output.split('/').slice(0,-1)) {path=resolve(path,segment);if((await lstat(path)).isSymbolicLink())throw new Error('Linked output parent');}
const handle=await open(resolve(root,output),'wx');
try{await handle.writeFile(JSON.stringify(result,null,2)+'\n','utf8');await handle.sync();}finally{await handle.close();}
console.log(JSON.stringify({output,levels:result.levels??null,template:options.has('--template'),provider_requests:0}));
