// Real application evaluation. Does not read paid credentials or press Keep,
// Approve, Undo or Flash. Reports live under ignored target/ by default.
import {readFile,writeFile,realpath} from 'node:fs/promises';
import {resolve,sep,dirname} from 'node:path';
import {createRequire} from 'node:module';
import {snapshotProject,sourceChanges,summarizeTrace,sha256,reviewBoundaryViolations,mechanicalEvidenceFailures} from './lib/vibe-evidence.mjs';
import {buildCapabilityMatrix} from './lib/capability-matrix.mjs';
const root=await realpath(process.cwd()),args=process.argv.slice(2);
// Fail closed on unsupported budgets/options before reserving evidence or
// connecting to the desktop/provider. Silently ignoring --max-requests would
// misrepresent a caller's intended limit; this harness does not enforce one.
const valueOptions=new Set(['--endpoint','--model','--case','--workflow','--output','--cdp']);
const flagOptions=new Set(['--run','--preflight','--authorize-local-inference']);
const options=new Map();
for(let i=0;i<args.length;i++) {
    const name=args[i];
    if(!valueOptions.has(name)&&!flagOptions.has(name))throw new Error(`Unknown evaluation option: ${name}. No provider request budget is implemented; do not assume an unsupported limit is enforced.`);
    if(options.has(name))throw new Error(`Duplicate evaluation option: ${name}`);
    if(flagOptions.has(name)){options.set(name,true);continue;}
    const value=args[++i];
    if(!value||value.startsWith('--'))throw new Error(`Missing value for evaluation option: ${name}`);
    options.set(name,value);
}
const option=name=>options.get(name)??null;
const caseFile=await readFile(resolve(root,'docs/ai-engineering/real-vibe-coder-cases.json'));
const cases=JSON.parse(caseFile.toString('utf8'));
const endpoint=option('--endpoint'),model=option('--model'),selected=option('--case'),workflow=option('--workflow');
const run=options.has('--run'),preflight=options.has('--preflight');
const report={schema_version:2,status:'NOT_EXECUTED',inference_requests:0,submitted_user_turns:0,paid_credential_access:false,patches_approved_by_harness:0,cases:cases.cases,grading:cases.grading,
    cases_sha256:sha256(caseFile),capability_levels:{knowledge_aware:'NOT_EVALUATED',project_aware:'NOT_EVALUATED',build_aware:'NOT_EVALUATED'}};
report.evaluation_mode=run?'authorized_local_model':preflight?'native_preflight':'preparation';
const output=resolve(root,option('--output')??'src-tauri/target/real-vibe-coder-evaluation.json');
if(!output.startsWith(root+sep))throw new Error('Output must stay in the workspace');
const resolvedParent=await realpath(dirname(output));
if(resolvedParent!==root&&!resolvedParent.startsWith(root+sep))throw new Error('Output parent escapes the workspace');
if(run&&preflight)throw new Error('Preflight must not run inference');
if(selected&&!cases.cases.some(test=>test.id===selected&&test.prompt))throw new Error('Unknown or non-prompt case');
if(workflow&&(workflow!==cases.workflow?.id||selected))throw new Error('Unknown workflow or conflicting --case');
const selectedCases=workflow?cases.workflow.stages.map(id=>cases.cases.find(test=>test.id===id)).filter(test=>test?.prompt)
    :cases.cases.filter(test=>selected?test.id===selected:test.kind==='read_only');
report.requested_workflow=workflow??null;
if(run) {
    if(!options.has('--authorize-local-inference')||!endpoint||!model)throw new Error('Explicit local inference authorization, endpoint and model required');
    const url=new URL(endpoint);
    if(url.protocol!=='http:'||!['localhost','127.0.0.1','[::1]'].includes(url.hostname)||url.username||url.password||url.search||url.hash)throw new Error('Only explicit local HTTP endpoints without credentials are allowed');
    // Raw transcripts may contain local SDK paths. Never publish them by default.
    const target=await realpath(resolve(root,'src-tauri/target'));
    if(!resolvedParent.startsWith(target+sep)&&resolvedParent!==target)throw new Error('Raw model evidence must stay under ignored src-tauri/target');
    // A UI-turn count cannot enforce the user's HTTP/retry or USD ceiling.
    // Until the isolated native transport has verified accounting and a cost
    // bound, fail before CDP, /models, settings writes or inference. There is
    // deliberately no CLI/environment bypass for this incomplete safeguard.
    throw new Error('Inference blocked: native HTTP-request/retry and USD budget enforcement is not implemented. No desktop connection or model discovery was attempted. Endpoint/model authorization alone cannot enforce these limits.');
}

async function connectIsolated() {
    const require=createRequire(resolve(root,'src-tauri/target/learning-ui-tools/package.json'));
    const {chromium}=require('playwright-core');
    const cdp=new URL(option('--cdp')??'http://127.0.0.1:19327');
    if(cdp.protocol!=='http:'||cdp.hostname!=='127.0.0.1'||cdp.username||cdp.password||cdp.search||cdp.hash)throw new Error('CDP must be loopback');
    const browser=await chromium.connectOverCDP(cdp.href);
    try {
        const page=browser.contexts()[0].pages()[0];
        const id=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('plugin:app|identifier'));
        if(id!=='th.nectec.vibekidbright.desktop-evaluation')throw new Error('Use the credential-isolated evaluation desktop only');
        return {browser,page,id};
    }catch(error){await browser.close();throw error;}
}

// Subscribe to the existing native event API, without replacing invoke/provider
// functions. Event limits are explicit; truncated evidence cannot certify a gate.
async function attachRecorder(page) {
    const events=[];let size=0,dropped=0;
    await page.exposeBinding('__collectVibeEvidence',(_source,event)=>{
        const bytes=Buffer.byteLength(JSON.stringify(event));
        if(events.length>=4000||size+bytes>8*1024*1024){dropped++;return;}
        size+=bytes;events.push(event);
    });
    const count=await page.evaluate(async()=>{
        const api=window.__TAURI_INTERNALS__;
        window.__vibeEvidenceState={done:false,error:false};window.__vibeEvidenceListeners=[];
        for(const event of ['ai-active-model','ai-chat-tool-start','ai-chat-tool-result','ai-chat-delta','ai-chat-error','ai-chat-done','ai-diff-pending','ai-project-review-pending','ai-project-review-resolved','file-modified','terminal-output']) {
            const callback=api.transformCallback(message=>{
                if(event==='ai-chat-done')window.__vibeEvidenceState.done=true;
                if(event==='ai-chat-error')window.__vibeEvidenceState.error=true;
                if(event==='terminal-output')window.__vibeEvidenceState.last_terminal=message.payload;
                void window.__collectVibeEvidence({event,payload:message.payload,at_ms:Date.now()});
            });
            const eventId=await api.invoke('plugin:event|listen',{event,target:{kind:'Any'},handler:callback});
            window.__vibeEvidenceListeners.push({event,eventId});
        }
        return window.__vibeEvidenceListeners.length;
    });
    return {count,events,reset:()=>{events.length=0;size=0;dropped=0;},dropped:()=>dropped,
        stop:()=>page.evaluate(async()=>{
            for(const {event,eventId} of window.__vibeEvidenceListeners??[]) {
                window.__TAURI_EVENT_PLUGIN_INTERNALS__.unregisterListener(event,eventId);
                await window.__TAURI_INTERNALS__.invoke('plugin:event|unlisten',{event,eventId});
            }
            window.__vibeEvidenceListeners=[];
            delete window.__collectVibeEvidence;
            delete window.__vibeEvidenceState;
        })};
}

// Reserve evidence before any discovery/UI submission. An existing report must
// fail before spending inference, rather than failing only after running tests.
await writeFile(output,JSON.stringify(report,null,2)+'\n',{flag:'wx'});
let browser,page,recorder;
try {
    if(preflight||run) {
        ({browser,page,id:report.isolated_desktop_identifier}=await connectIsolated());
        recorder=await attachRecorder(page);report.native_event_subscriptions=recorder.count;
    }
    if(preflight) {
        const marker='[Evaluation] Synthetic recorder preflight '+Date.now()+'; no inference.';
        await page.evaluate(payload=>window.__TAURI_INTERNALS__.invoke('plugin:event|emit',{event:'terminal-output',payload}),marker);
        await page.waitForFunction(value=>window.__vibeEvidenceState.last_terminal===value,marker,{timeout:5000});
        await page.evaluate(()=>window.__collectVibeEvidence({event:'collector-barrier',payload:null,at_ms:Date.now()}));
        report.synthetic_native_event_captured=recorder.events.some(event=>event.event==='terminal-output'&&event.payload===marker);
        if(!report.synthetic_native_event_captured)throw new Error('Synthetic native event was not collected');
        report.reason='Actual isolated desktop IPC/event subscription, synthetic marker delivery and cleanup preflight. No model discovery, inference or patch approval; this marker is not model/tool evidence.';
    }else if(!run) {
        report.reason='Prepared only. Run requires explicitly authorized loopback model and isolated desktop. No capability level is certified by preparation.';
    }else {
        const response=await fetch(new URL(endpoint.replace(/\/$/,'')+'/models'),{redirect:'error',signal:AbortSignal.timeout(3000)});
        const available=await response.json();
        if(!response.ok||!available.data?.some(item=>item.id===model))throw new Error('Selected local model is not available');
        const project=await page.evaluate(()=>localStorage.getItem('vibe-project-dir'));
        const allowed=await Promise.all(['repeat_delay','forever_delay'].map(async name=>{
            const path=resolve(root,'src-tauri/target/block-c-evaluation-20261018',name);
            if(await realpath(path)!==path)throw new Error('Public evaluation fixture must not resolve through a link or redirected root');
            return path;
        }));
        if(!project||!allowed.includes(await realpath(project)))throw new Error('Select a generated public evaluation fixture before running inference');
        report.project_relative=project.slice(root.length+1).replaceAll('\\','/');
        await page.evaluate(async({endpoint,model})=>{
            const invoke=window.__TAURI_INTERNALS__.invoke;
            await invoke('set_provider',{provider:'local'});await invoke('set_base_url',{url:endpoint});await invoke('set_model',{model});
        },{endpoint,model});
        if(!await page.getByPlaceholder('Ask about your code...').isVisible())await page.getByRole('button',{name:/Vibe Coder/}).first().click();
        // A fresh chat avoids inheriting earlier model/tool evidence. Selected
        // cases share the same task conversation, under explicit prompts.
        await page.getByTitle('➕ แชทใหม่',{exact:true}).click();
        const input=page.getByPlaceholder('Ask about your code...');
        report.status='PARTIAL';report.provider='local';report.model=model;report.endpoint=endpoint;report.outputs=[];
        report.inference_requests=null;report.inference_count_note='UI turns counted below; native tool turns, HTTP retries and provider token usage are not instrumented.';
        for(const test of selectedCases) {
            const entry={id:test.id,prompt:test.prompt,status:'RUNNING',before:await snapshotProject(project),human_grade:null};
            report.outputs.push(entry);recorder.reset();
            await page.evaluate(()=>{window.__vibeEvidenceState={done:false,error:false};});
            const started=Date.now();
            try {
                await input.fill(test.prompt);await input.press('Enter');report.submitted_user_turns++;
                await page.waitForFunction(()=>window.__vibeEvidenceState.done||window.__vibeEvidenceState.error,null,{timeout:180000});
                // Drain native callback bindings queued before done/error.
                await page.evaluate(()=>window.__collectVibeEvidence({event:'collector-barrier',payload:null,at_ms:Date.now()}));
                entry.status='CAPTURED';
            }catch(error) {
                entry.status='FAILED';entry.error=String(error);
                await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('stop_ai_generation')).catch(()=>{});
            }
            entry.elapsed_ms=Date.now()-started;entry.events=recorder.events.slice();entry.events_dropped=recorder.dropped();
            entry.after=await snapshotProject(project);entry.source_changes=sourceChanges(entry.before,entry.after);
            entry.evidence=await summarizeTrace(entry.events,resolve(root,'resources/knowledge_base'),project);
            entry.mechanical_evidence_failures=mechanicalEvidenceFailures(entry.evidence,test.mechanical_requirements);
            entry.body_after=await page.locator('body').innerText();
            entry.review_boundary_violations=reviewBoundaryViolations(entry.events,test.kind);
            if(entry.events_dropped||entry.mechanical_evidence_failures.length||entry.source_changes.length||entry.review_boundary_violations.length)entry.status='FAILED';
            await writeFile(output,JSON.stringify(report,null,2)+'\n');
            if(entry.status==='FAILED')throw new Error('Case failed, was interrupted, lost evidence or changed saved sources; inspect preserved report. No automatic rollback attempted.');
            if(test.kind!=='read_only')break; // Stop at the user's patch review boundary.
        }
        report.reason='Native tool/answer traces and source hashes captured. Source/citation integrity is separate from human quality grading. No build, patch application or repair success inferred. Review-required cases stop before user approval.';
    }
}catch(error){report.status='FAILED';report.error=String(error);process.exitCode=1;}
finally {
    if(recorder)await recorder.stop().catch(error=>{report.cleanup_error=String(error);report.status='FAILED';process.exitCode=1;});
    if(browser)await browser.close();
    // Bind this embedded view to the report payload before adding the matrix.
    try {
        report.capability_matrix=buildCapabilityMatrix(report,sha256(JSON.stringify({...report,capability_matrix:undefined})));
        report.capability_matrix.hash_scope='Report JSON.stringify payload excluding capability_matrix; standalone review CLI binds the exact file bytes instead.';
    }catch(error){report.capability_matrix_error=String(error);report.status='FAILED';process.exitCode=1;}
    await writeFile(output,JSON.stringify(report,null,2)+'\n');
}
console.log(JSON.stringify({status:report.status,submitted_user_turns:report.submitted_user_turns,output}));
