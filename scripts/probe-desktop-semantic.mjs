// Actual WebView2 + production command registry. No invoke/provider stubs.
import {spawn,execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {createRequire} from 'node:module';
import {mkdir,readFile,writeFile,copyFile,access,utimes,realpath} from 'node:fs/promises';
import {resolve} from 'node:path';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';
import {extname,sep} from 'node:path';
const root=process.cwd();
const preflightOnly=process.argv.includes('--preflight-only');
const require=createRequire(resolve(root,'src-tauri/target/learning-ui-tools/package.json'));
const {chromium}=require('playwright-core');
const run=resolve(root,'src-tauri/target/desktop-evaluation',`run-${Date.now()}`);
await mkdir(run,{recursive:true});
// Fresh evidence is opt-in exported, never overwrites the historical report.
const outputIndex=process.argv.indexOf('--output');
const output=outputIndex<0?null:resolve(root,process.argv[outputIndex+1]);
if(output) {
    const canonicalRoot=await realpath(root),parent=await realpath(resolve(output,'..'));
    if(!output.startsWith(root+sep)||(parent!==canonicalRoot&&!parent.startsWith(canonicalRoot+sep)))throw new Error('Report must stay within workspace');
    await writeFile(output,JSON.stringify({status:'RUNNING',run_directory:run})+'\n',{flag:'wx'});
}
const sdk=resolve(process.env.APPDATA??'', 'th.nectec.vibekidbright/toolchain');
// Development host paths only; never bundled or persisted into user AppData.
const idf=process.env.VIBE_EVAL_IDF??resolve(sdk,'frameworks/esp-idf-v5.5.4');
const tools=process.env.VIBE_EVAL_TOOLS??resolve(sdk,'tools');
const clangd=resolve(root,'src-tauri/target/clangd-evaluation/clangd_23.1.0/bin/clangd.exe');
const projects=['repeat_delay','forever_delay'].map(n=>resolve(root,'src-tauri/target/block-c-evaluation-20261018',n));
const source=projects.map(p=>resolve(p,'main/main.c'));
const saved=await Promise.all(source.map(p=>readFile(p,'utf8')));
const componentSources=projects.map(p=>resolve(p,'components/kb_reference/kb_core.c'));
const savedComponents=await Promise.all(componentSources.map(p=>readFile(p,'utf8')));
const database=resolve(projects[0],'build/compile_commands.json');
const cdb=await readFile(database);
const hash=x=>createHash('sha256').update(x).digest('hex');
const assert=(value,message)=>{if(!value)throw new Error(message);};
const port=19327;
const dist=resolve(root,'dist');
const productionIndex=hash(await readFile(resolve(dist,'index.html')));
const csp=JSON.parse(await readFile(resolve(root,'src-tauri/tauri.conf.json'),'utf8')).app.security.csp;
const server=createServer(async(req,res)=>{
    const path=resolve(dist,'.'+decodeURIComponent(new URL(req.url,'http://localhost').pathname==='/'?'/index.html':new URL(req.url,'http://localhost').pathname));
    if(!path.startsWith(dist+sep)){res.writeHead(403);res.end();return;}
    try{res.setHeader('Content-Security-Policy',csp);res.setHeader('Content-Type',({'.html':'text/html','.js':'text/javascript','.css':'text/css','.png':'image/png','.svg':'image/svg+xml','.ttf':'font/ttf'})[extname(path)]??'application/octet-stream');res.end(await readFile(path));}
    catch{res.writeHead(404);res.end();}
});
await new Promise((ok,fail)=>{server.once('error',fail);server.listen(port+1,'127.0.0.1',ok);});
const child=spawn(resolve(root,'src-tauri/target/debug/examples/desktop_evaluation.exe'),[run,idf,tools,String(port)],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});
let log='';child.stdout.on('data',b=>{log+=b;});child.stderr.on('data',b=>{log+=b;});
const rows=[];let browser,page;
const navigations=[];
const result={scope:'Actual debug native desktop WebView2 with production React/CSP, Tauri IPC, configured SDK, clangd and Monaco. No native/provider stubs or Vite/HMR. No paid credentials. Generated public fixtures only.',production_index_sha256:productionIndex,run_directory:run,status:'FAILED',cases:rows,installed_app_modified:false,installer_built:false};
if(preflightOnly)result.scope='Actual debug-native WebView2/production React/Tauri evidence-listener preflight only. No inference, model outputs, patches, semantic queries, build or Flash. Existing public fixture only.';
const delay=ms=>new Promise(r=>setTimeout(r,ms));
try{
    for(let i=0;i<60;i++){
        if(child.exitCode!==null)throw new Error(`Isolated desktop exited ${child.exitCode} (0x${(child.exitCode>>>0).toString(16)}): `+log.slice(-2000));
        try{browser=await chromium.connectOverCDP(`http://127.0.0.1:${port}`,{timeout:1000});break;}catch{await delay(500);}
    }
    assert(browser,'WebView2 loopback debugging unavailable');
    const context=browser.contexts()[0];page=context.pages()[0]??await context.waitForEvent('page');
    page.on('framenavigated',frame=>{if(frame===page.mainFrame())navigations.push({at_ms:Date.now(),url:frame.url()});});
    // CDP can expose about:blank before navigation commits. Its opaque origin
    // forbids localStorage; wait for our actual app origin before touching UI.
    await page.waitForURL(url=>url.origin===`http://127.0.0.1:${port+1}`,{timeout:15000});
    await page.waitForFunction(()=>document.readyState!=='loading'&&Boolean(window.__TAURI_INTERNALS__?.invoke),null,{timeout:15000});
    const errors=[];page.on('pageerror',e=>errors.push(String(e)));
    // Set local-only initial UI state, retaining real IPC and editor code.
    await page.addInitScript(({project})=>{
        localStorage.setItem('vibe-project-dir',project);localStorage.setItem('vibe-active-view','editor');localStorage.setItem('vibe-ai-panel','false');
        window.MonacoEnvironment={globalAPI:true};
        // Observe view/button transitions without intercepting app events/state.
        window.__semanticProbeTimeline=[];
        document.addEventListener('click',event=>{
            const button=event.target?.closest?.('button');
            if(button){window.__semanticProbeTimeline.push({at_ms:Date.now(),kind:'click',label:(button.getAttribute('aria-label')??button.textContent??'').slice(0,100)});window.__semanticProbeTimeline=window.__semanticProbeTimeline.slice(-200);}
        },true);
        let lastView='';
        new MutationObserver(()=>{
            const button=document.querySelector('nav[aria-label="Workspace views"] button[aria-pressed="true"]');
            const view=button?.textContent??'';
            if(view&&view!==lastView){lastView=view;window.__semanticProbeTimeline.push({at_ms:Date.now(),kind:'view',view});}
        }).observe(document,{subtree:true,attributes:true,attributeFilter:['aria-pressed','hidden']});
    },{project:projects[0]});
    const select=async(project)=>{
        await page.evaluate(p=>{localStorage.setItem('vibe-project-dir',p);localStorage.setItem('vibe-active-view','editor');},project);
        // Reload is restart/project transition; application owns all file loading.
        await page.reload();
        // Init script intentionally starts only project 0; choose project via the
        // actual project-switched event, using the existing app event handler.
        if(project!==projects[0]){
            await page.evaluate(async p=>{
                await window.__TAURI_INTERNALS__.invoke('plugin:event|emit',{event:'force-project-dir',payload:p});
            },project);
            await page.waitForFunction(p=>document.body.innerText.includes('Switched project to: '+p),project);
        }
        await page.getByRole('button',{name:'main.c',exact:true}).first().click();
        await page.waitForFunction(()=>window.monaco?.editor.getEditors().some(e=>e.getModel()&&e.getDomNode()?.offsetParent!==null));
        await page.getByRole('button',{name:'C intelligence',exact:true}).click();
    };
    await select(projects[0]);
    await page.waitForFunction(()=>[...document.querySelectorAll('button')].some(b=>b.textContent?.includes('Build & Flash')&&!b.disabled));
    const readiness=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('check_toolchain'));
    assert(readiness.status==='ready'&&readiness.source==='configured-sdk','Configured SDK did not unlock actual app build controls');
    let sentinel=false;try{await access(resolve(run,'app-data/toolchain/.toolchain_ready'));sentinel=true;}catch{}
    assert(!sentinel,'Evaluation manufactured a managed install marker');
    rows.push({scenario:'configured_external_sdk_ready_in_actual_ui',status:'PASS',source:readiness.source,managed_marker_created:false});
    const preflight=resolve(run,'real-vibe-preflight.json');
    await promisify(execFile)(process.execPath,[resolve(root,'scripts/evaluate-vibe-coder-real.mjs'),'--preflight','--cdp',`http://127.0.0.1:${port}`,'--output',preflight],{cwd:root,windowsHide:true});
    result.real_vibe_harness_preflight=JSON.parse(await readFile(preflight,'utf8'));
    assert(result.real_vibe_harness_preflight.native_event_subscriptions===11,'Actual native evidence listeners were not attached');
    assert(result.real_vibe_harness_preflight.synthetic_native_event_captured===true,'Synthetic native event did not reach the recorder');
    if(!preflightOnly) {
    const enable=async path=>{
        const box=page.getByRole('checkbox',{name:'Enable a trusted local clangd executable'});
        if(!await box.isChecked())await box.check();
        await page.getByRole('textbox',{name:'Clangd executable'}).fill(path);
    };
    const position=async(token,extra='')=>{
        await page.evaluate(({token,extra})=>{
            const e=window.monaco.editor.getEditors().find(e=>e.getDomNode()?.offsetParent!==null);
            if(extra)e.setValue(e.getValue()+extra);
            const text=e.getValue(),offset=text.lastIndexOf(token);
            if(offset<0)throw new Error('Missing fixture identifier '+token);
            e.setPosition(e.getModel().getPositionAt(offset+1));e.focus();
        },{token,extra});
    };
    const query=async(operation,engine='clangd semantic results')=>{
        const started=Date.now();await page.getByRole('button',{name:operation,exact:true}).click();
        await page.getByRole('status').filter({hasText:engine}).waitFor({timeout:30000});
        rows.push({operation,engine,elapsed_ms:Date.now()-started,status:'PASS'});
    };
    await enable(clangd);await position('kb_delay_ms');await query('Find definition');
    const definition=page.getByRole('button',{name:/include\/kb_core\.h:\d+:\d+/});
    await definition.first().click();
    await page.waitForFunction(()=>window.monaco.editor.getEditors().some(e=>e.getDomNode()?.offsetParent!==null&&e.getValue().includes('bool kb_delay_ms')));
    const cursor=await page.evaluate(()=>{const e=window.monaco.editor.getEditors().find(e=>e.getDomNode()?.offsetParent!==null);return {position:e.getPosition(),line:e.getModel().getLineContent(e.getPosition().lineNumber)};});
    assert(cursor.line.includes('kb_delay_ms'),'Monaco did not navigate to the definition');
    rows.push({scenario:'definition_result_to_monaco',status:'PASS',cursor});
    await page.getByRole('button',{name:'C intelligence',exact:true}).click();
    await enable(clangd);
    await query('Find definition','Tree-sitter syntax candidates');
    assert(await page.getByText(/header inference is not enabled/).count()>0,'Header limitation was not labelled');
    rows.push({scenario:'header_navigation_and_exact_cdb_fallback',status:'PASS',semantic_header_inference:false});
    await query('Find references','Tree-sitter syntax candidates');
    assert(await page.locator('section[aria-label="Local C intelligence"] ul button').count()>0,'Open-header syntax references were not returned');
    rows.push({scenario:'header_references_labelled_syntax_fallback',status:'PASS',semantic_header_inference:false});
    await select(projects[0]);await enable(clangd);await position('kb_delay_ms');await query('Find references');
    const references=page.locator('section[aria-label="Local C intelligence"] ul button');
    assert(await references.count()===[...saved[0].matchAll(/\bkb_delay_ms\(/g)].length,'Unexpected opened-TU reference count');
    await references.first().click();
    const referenceCursor=await page.evaluate(()=>{const e=window.monaco.editor.getEditors().find(e=>e.getDomNode()?.offsetParent!==null);return e.getModel().getLineContent(e.getPosition().lineNumber);});
    assert(referenceCursor.includes('if (!kb_delay_ms('),'Reference result did not navigate Monaco');
    rows.push({scenario:'reference_result_to_monaco',status:'PASS',reference_scope:'opened main translation unit only'});
    // Bootstrap the additional tab via the existing native event/open-file app
    // path. This is explicitly a fixture event, never a model patch or review.
    await page.evaluate(async fullPath=>window.__TAURI_INTERNALS__.invoke('plugin:event|emit',{event:'ai-diff-pending',payload:JSON.stringify({fullPath})}),componentSources[0]);
    await page.getByRole('button',{name:'Close kb_core.c',exact:true}).waitFor();
    const pending=await page.evaluate(path=>window.__TAURI_INTERNALS__.invoke('check_pending_diff',{path}),componentSources[0]);
    assert(pending===null,'Tab bootstrap unexpectedly created a patch');
    await page.getByRole('button',{name:'main.c',exact:true}).first().click();
    await page.getByRole('button',{name:'C intelligence',exact:true}).click();
    await enable(clangd);await position('kb_delay_ms');await query('Find references');
    const cross=page.locator('section[aria-label="Local C intelligence"] ul button');
    assert(await cross.count()===[...saved[0].matchAll(/\bkb_delay_ms\(/g)].length+1,'Opened component references incomplete');
    assert(await cross.filter({hasText:'kb_core.c:'}).count()===1,'Component definition missing from references');
    rows.push({scenario:'cross_component_open_translation_unit_references',status:'PASS',tab_bootstrap:'synthetic existing native app event; no patch',source_scope:'main plus opened kb_reference implementation',references:await cross.count()});
    await query('Find definition');
    await page.getByRole('button',{name:/kb_reference\/kb_core\.c:\d+:\d+/}).first().click();
    const componentCursor=await page.evaluate(()=>{const e=window.monaco.editor.getEditors().find(e=>e.getDomNode()?.offsetParent!==null);return e.getModel().getLineContent(e.getPosition().lineNumber);});
    assert(componentCursor.includes('bool kb_delay_ms('),'Cross-component result did not navigate Monaco');
    rows.push({scenario:'cross_component_definition_to_monaco',status:'PASS'});
    await select(projects[0]);await enable(clangd);
    const extra='\nstatic int desktop_unsaved(void) { return 7; }\nint desktop_use(void) { return desktop_unsaved(); }\n';
    await position('desktop_unsaved',extra);await query('Find definition');
    rows.push({scenario:'unsaved_buffer',status:'PASS',disk_unchanged:(await readFile(source[0],'utf8'))===saved[0]});
    await select(projects[0]);await enable(clangd);
    const unicodeExtra='\n/* ไทย😀 */ static int desktop_utf16(void) { return 7; }\nint desktop_utf16_use(void) { /* ไทย😀 */ return desktop_utf16(); }\n';
    await position('desktop_utf16',unicodeExtra);await query('Find definition');
    await page.locator('section[aria-label="Local C intelligence"] ul button').first().click();
    const unicodeCursor=await page.evaluate(()=>{
        const e=window.monaco.editor.getEditors().find(e=>e.getDomNode()?.offsetParent!==null);
        const p=e.getPosition(),line=e.getModel().getLineContent(p.lineNumber);
        return {line,column:p.column,expected:line.indexOf('desktop_utf16')+1};
    });
    assert(unicodeCursor.line.startsWith('/* ไทย😀 */ static int desktop_utf16(')&&unicodeCursor.column===unicodeCursor.expected,'UTF-16 definition did not navigate to the exact Monaco column');
    rows.push({scenario:'thai_emoji_unsaved_definition_exact_monaco_column',status:'PASS',column:unicodeCursor.column,source_written:false});
    await query('Find references');
    assert(await page.locator('section[aria-label="Local C intelligence"] ul button').count()===2,'Unicode unsaved references incomplete');
    rows.push({scenario:'thai_emoji_unsaved_references',status:'PASS',references:2});
    for(const [mode,reason] of [['eof','Invalid source line'],['surrogate','splits a UTF-16 surrogate'],['reversed','Reversed clangd location range']]) {
        const folder=resolve(run,'range-'+mode);await mkdir(folder);
        const fixture=resolve(folder,'clangd.exe');await copyFile(resolve(root,'src-tauri/target/debug/examples/range_lsp_fixture.exe'),fixture);
        await enable(fixture);await query('Find definition','Tree-sitter syntax candidates');
        await page.getByText(new RegExp(reason)).waitFor();
        assert((await page.locator('section[aria-label="Local C intelligence"]').innerText()).includes(reason),'Invalid '+mode+' range did not expose its fallback reason');
        rows.push({scenario:'invalid_lsp_'+mode+'_range_fallback',status:'PASS',protocol_fault_fixture:true,real_clangd:false});
    }
    // Reopen saved project before filesystem fault injection; do not save drafts.
    await select(projects[0]);await enable(clangd);await position('kb_delay_ms');
    try{await writeFile(database,'{ invalid');await query('Find definition','Tree-sitter syntax candidates');}
    finally{await writeFile(database,cdb);}
    rows.push({scenario:'invalid_compilation_database_fallback',status:'PASS'});
    await enable(resolve(run,'missing/clangd.exe'));await query('Find definition','Tree-sitter syntax candidates');
    rows.push({scenario:'clangd_unavailable_fallback',status:'PASS'});
    const stall=resolve(run,'stall/clangd.exe');await mkdir(resolve(run,'stall'));await copyFile(resolve(root,'src-tauri/target/debug/examples/stalling_lsp_fixture.exe'),stall);
    await enable(stall);await page.getByRole('button',{name:'Find definition',exact:true}).click();
    const marker=resolve(run,'stall/stall-pid.txt');
    for(let i=0;i<50;i++){try{await readFile(marker);break;}catch{await delay(50);}}
    const cancelledPid=Number(await readFile(marker,'utf8'));
    await page.getByRole('button',{name:'Stop C intelligence',exact:true}).click();
    await page.getByText('C intelligence cancelled',{exact:true}).waitFor();await delay(300);
    const alive=pid=>{try{process.kill(pid,0);return true;}catch{return false;}};
    assert(!alive(cancelledPid),'Cancelled LSP fixture process survived');
    rows.push({scenario:'ui_cancellation',status:'PASS',child_terminated:true});
    await query('Find definition','Tree-sitter syntax candidates');
    assert(await page.getByText(/timed out/).count()>0,'Timeout did not expose its limitation');
    assert(!alive(Number(await readFile(marker,'utf8'))),'Timed-out LSP fixture process survived');
    rows.push({scenario:'native_20_second_timeout',status:'PASS',fault_fixture:true,child_terminated:true});
    await page.getByRole('button',{name:'Find definition',exact:true}).click();
    let switchPid;
    for(let i=0;i<50;i++){
        const pid=Number(await readFile(marker,'utf8'));
        if(alive(pid)){switchPid=pid;break;}await delay(50);
    }
    assert(switchPid,'Pending switch fixture did not start');
    await page.evaluate(async p=>window.__TAURI_INTERNALS__.invoke('plugin:event|emit',{event:'force-project-dir',payload:p}),projects[1]);
    await page.waitForFunction(p=>document.body.innerText.includes('Switched project to: '+p),projects[1]);
    await page.getByRole('button',{name:'main.c',exact:true}).first().click();
    await delay(300);
    assert(!alive(switchPid),'Switching projects left the old LSP process running');
    await page.getByRole('button',{name:'C intelligence',exact:true}).click();
    assert(await page.getByText('clangd semantic results',{exact:true}).count()===0,'Stale semantic results survived project switch');
    rows.push({scenario:'project_switch_during_pending_request',status:'PASS',old_child_terminated:true,stale_results_discarded:true});
    await enable(clangd);await position('kb_delay_ms');await query('Find references');
    assert(await page.locator('section[aria-label="Local C intelligence"] ul button').count()===[...saved[1].matchAll(/\bkb_delay_ms\(/g)].length,'Project results mixed');
    rows.push({scenario:'project_switch',status:'PASS'});
    // IPC-only safety probes are separately labelled, not complete React paths.
    const boundaries=await page.evaluate(async({project,other})=>{
        try{await window.__TAURI_INTERNALS__.invoke('query_c_intelligence',{projectDir:project,requestId:'desktop-isolation',request:{file_path:other,buffers:[],line:0,character:0,operation:'definition',clangd_path:''}});return false;}catch{return true;}
    },{project:projects[1],other:source[0]});
    assert(boundaries,'Cross-project source was accepted');
    rows.push({scenario:'private_root_boundary',status:'PASS',path:'real IPC only'});
    const invalidPositions=await page.evaluate(async({project,file})=>{
        const text='/* 😀 */ int cursor;',split='/* '.length+1;
        const checks=[{line:1,character:0},{line:0,character:split}];
        return Promise.all(checks.map(async (position,i)=>{
            try{await window.__TAURI_INTERNALS__.invoke('query_c_intelligence',{projectDir:project,requestId:'desktop-invalid-position-'+i,request:{file_path:file,buffers:[{path:file,text}],...position,operation:'definition',clangd_path:''}});return false;}catch{return true;}
        }));
    },{project:projects[1],file:source[1]});
    assert(invalidPositions.every(Boolean),'Invalid source line or split surrogate was accepted');
    rows.push({scenario:'invalid_line_and_split_surrogate_rejected',status:'PASS',path:'real IPC only',cases:2});
    const eofChecks=await page.evaluate(async({project,file})=>{
        const ask=(text,line)=>window.__TAURI_INTERNALS__.invoke('query_c_intelligence',{projectDir:project,requestId:'desktop-eof-'+text.length+'-'+line,request:{file_path:file,buffers:[{path:file,text}],line,character:0,operation:'definition',clangd_path:''}});
        const empty=await ask('',0),final=await ask('int item;\n',1);
        let rejected=false;try{await ask('int item;\n',2);}catch{rejected=true;}
        return empty.engine==='tree_sitter_syntax'&&empty.locations.length===0&&final.engine==='tree_sitter_syntax'&&final.locations.length===0&&rejected;
    },{project:projects[1],file:source[1]});
    assert(eofChecks,'Empty source/final line/EOF boundary failed');
    rows.push({scenario:'empty_source_and_final_line_eof',status:'PASS',path:'real IPC only',cases:3});
    // Real React terminal -> native SDK runner -> configured compiler; build only.
    await utimes(source[1],new Date(),new Date());
    const terminal=page.getByPlaceholder(/Type command/);
    const buildStarted=Date.now();await terminal.fill('idf.py build');await terminal.press('Enter');
    await page.getByText(/Project build complete\. To flash/).waitFor({timeout:120000});
    rows.push({scenario:'react_terminal_to_configured_idf_build',status:'PASS',command:'idf.py build',elapsed_ms:Date.now()-buildStarted,flashed:false});
    result.desktop_firmware_sha256=hash(await readFile(resolve(projects[1],'build/kidbright_block_pilot.bin')));
    }
    result.page_errors=errors;assert(errors.length===0,'Desktop JavaScript errors');
    result.disk_sources_unchanged=(await Promise.all(source.map(p=>readFile(p,'utf8')))).every((s,i)=>s===saved[i]);
    assert(result.disk_sources_unchanged,'Saved source changed');
    result.disk_component_sources_unchanged=(await Promise.all(componentSources.map(p=>readFile(p,'utf8')))).every((s,i)=>s===savedComponents[i]);
    assert(result.disk_component_sources_unchanged,'Saved component source changed');
    result.compilation_database_restored=hash(await readFile(database))===hash(cdb);
    result.executable_sha256=hash(await readFile(resolve(root,'src-tauri/target/debug/examples/desktop_evaluation.exe')));
    result.clangd_sha256=hash(await readFile(clangd));
    result.startup_credentials='Debug-only evaluation guard returns empty credentials before any keyring or legacy-config access';
    await page.screenshot({path:resolve(run,'semantic-results.png')});
    assert(hash(await readFile(resolve(dist,'index.html')))===productionIndex,'Production frontend changed during desktop run');
    result.status='PARTIAL';result.header_status=preflightOnly?'NOT_EXECUTED':'tested navigation and labelled syntax fallback; no inferred header compile command';result.native_provider_stubbed=false;
}catch(error){result.error=String(error);if(page)try{result.body=(await page.locator('body').innerText()).slice(-5000);await page.screenshot({path:resolve(run,'failure.png')});}catch{}}
finally{
    if(page)try{result.ui_transition_timeline=await page.evaluate(()=>window.__semanticProbeTimeline??[]);}catch{}
    result.main_frame_navigations=navigations;
    await writeFile(database,cdb);
    if(page)try{await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('plugin:window|close',{label:'main'}));}catch{}
    if(browser)await browser.close().catch(()=>{});
    if(child.exitCode===null)child.kill();
    await new Promise(ok=>server.close(ok));
    await writeFile(resolve(run,'native.log'),log);
    result.source_sha256=Object.fromEntries(await Promise.all(['src-tauri/src/lib.rs','src-tauri/src/ai/config.rs','src-tauri/src/esp_idf.rs','src-tauri/src/toolchain.rs','src-tauri/src/intelligence/semantic.rs','src-tauri/build.rs','src-tauri/examples/desktop_evaluation.rs','src-tauri/examples/desktop-evaluation.manifest','src-tauri/examples/stalling_lsp_fixture.rs','src-tauri/examples/range_lsp_fixture.rs','src/CIntelligencePanel.tsx','src/CodeEditor.tsx','scripts/probe-desktop-semantic.mjs','scripts/evaluate-vibe-coder-real.mjs','scripts/lib/vibe-evidence.mjs','scripts/lib/capability-matrix.mjs','docs/ai-engineering/real-vibe-coder-cases.json'].map(async p=>[p,hash(await readFile(resolve(root,p)))])));
    await writeFile(resolve(run,'result.json'),JSON.stringify(result,null,2)+'\n');
    if(output)await writeFile(output,JSON.stringify(result,null,2)+'\n');
    console.log(JSON.stringify(result));
}
if(result.error)process.exitCode=1;
