// Production React smoke with an explicit in-memory native/provider fixture.
// This verifies UI/CSP wiring; it is NOT a live-provider or native-IPC quality eval.
import {createServer} from 'node:http';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import {resolve,extname,sep,dirname} from 'node:path';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {probeBoardGuide} from './probe-board-guide.mjs';
import {probePromptExamples} from './probe-prompt-examples.mjs';
const root=process.cwd();
const args=process.argv.slice(2);
if(args.length && (args.length!==2||args[0]!=='--report'))throw new Error('Usage: check-production-workflows.mjs [--report <workspace-path.json>]');
const reportPath=resolve(root,args[1]??'docs/ai-engineering/production-workflow-probe.json');
if(!reportPath.startsWith(root+sep))throw new Error('Report must stay inside the workspace');
const require=createRequire(resolve(root,'src-tauri/target/learning-ui-tools/package.json'));
const {chromium}=require('playwright-core');
const executable=['C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe','C:/Program Files/Microsoft/Edge/Application/msedge.exe','C:/Program Files/Google/Chrome/Application/chrome.exe'].find(existsSync);
if(!executable)throw new Error('A local Chromium browser is needed for this development probe');
const config=JSON.parse(await readFile('src-tauri/tauri.conf.json','utf8'));
const dist=resolve(root,'dist');
const server=createServer(async(req,res)=>{
    const path=resolve(dist,'.'+decodeURIComponent((req.url??'/').split('?')[0]==='/'?'/index.html':req.url.split('?')[0]));
    if(!path.startsWith(dist+sep)){res.writeHead(403);res.end();return;}
    try{const bytes=await readFile(path);res.setHeader('Content-Security-Policy',config.app.security.csp);
        res.setHeader('Content-Type',({'.html':'text/html','.js':'text/javascript','.css':'text/css','.svg':'image/svg+xml','.png':'image/png','.ttf':'font/ttf','.json':'application/json'})[extname(path)]??'application/octet-stream');res.end(bytes);
    }catch{res.writeHead(404);res.end();}
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const address=`http://127.0.0.1:${server.address().port}`;
const browser=await chromium.launch({executablePath:executable,headless:true});
const page=await browser.newPage({viewport:{width:1440,height:1000}});
const errors=[],external=[];
page.on('pageerror',e=>errors.push(String(e)));
await page.route('**/*',route=>{if(route.request().url().startsWith(address))return route.continue();external.push(route.request().url());return route.abort();});
try{
    await page.addInitScript(()=>{
        const project='D:\\fixture';const path=project+'\\main\\main.c';
        const old='void app_main(void) { int revision = 0; }\n';const proposed='void app_main(void) { int revision = 1; }\n';
        let source=old,pending=null,next=1;const callbacks=new Map(),listeners=new Map(),commands=[];
        const emit=(name,payload)=>{for(const id of listeners.get(name)??[])callbacks.get(id)?.({event:name,id,payload});};
        window.__workflowProbe={commands,state:()=>({source,pending}),emit};
        window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
        localStorage.clear();localStorage.setItem('vibe-project-dir',project);
        window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},
            transformCallback:(fn,once=false)=>{const id=next++;callbacks.set(id,once?x=>{callbacks.delete(id);fn(x);}:fn);return id;},
            unregisterCallback:id=>callbacks.delete(id),
            invoke:async(cmd,args={})=>{
                commands.push({cmd,args});
                if(cmd==='plugin:event|listen'){const set=listeners.get(args.event)??new Set();set.add(args.handler);listeners.set(args.event,set);return args.handler;}
                if(cmd==='plugin:event|unlisten'){listeners.get(args.event)?.delete(args.eventId);return;}
                if(cmd==='check_toolchain')return {status:'ready',version:'fixture'};
                if(cmd==='get_idf_custom_paths')return {idf_path:'C:\\fixture-sdk',tools_path:'C:\\fixture-tools'};
                if(cmd==='validate_idf_project')return true;
                if(cmd==='list_project_files')return [{name:'main',path:project+'\\main',is_dir:true,children:[{name:'main.c',path,is_dir:false}]}];
                if(cmd==='read_project_file')return source;
                if(cmd==='check_pending_diff')return pending;
                if(cmd==='get_pending_project_reviews'||cmd==='get_knowledge_base_files'||cmd==='list_serial_ports'||cmd==='list_adb_devices')return [];
                if(cmd==='get_compiler_repair')return null;
                if(cmd==='get_private_context_consent')return false;
                if(cmd==='get_provider')return 'openai';
                if(cmd==='get_base_url')return 'http://127.0.0.1:1234/v1';
                if(cmd==='get_model'||cmd==='get_google_model'||cmd==='get_openrouter_model')return 'local-fixture';
                if(cmd==='send_ai_message'){
                    const text=JSON.stringify(args.messages);const edit=text.includes('Update main');const name=edit?'write_file':'knowledge_search';
                    setTimeout(()=>emit('ai-chat-tool-start',JSON.stringify({name})),20);
                    setTimeout(()=>{
                        if(edit){pending={revision:'fixture-r1',before:source,proposed,status:'pending'};emit('ai-diff-pending',JSON.stringify({fullPath:path}));}
                        emit('ai-chat-tool-result',JSON.stringify({name,result:edit?{result:'Proposal awaiting review'}:[{file:'compatibility/kb_core.c',citation:'compatibility/kb_core.c:15-20',content:'checked delay conversion'}]}));
                    },60);
                    setTimeout(()=>emit('ai-chat-delta',edit?'Changes proposed. Review before Keep.':'Reference: compatibility/kb_core.c:15-20'),100);
                    setTimeout(()=>emit('ai-chat-done',JSON.stringify({history:[]})),160);return;
                }
                if(cmd==='accept_diff'){
                    if(args.expectedRevision!==pending?.revision)throw new Error('Stale fixture revision');source=pending.proposed;pending=null;
                    emit('file-modified',JSON.stringify({path}));return 'Reviewed';
                }
                if(cmd.startsWith('get_'))return '';
                return 'Ready';
            }};
    });
    await page.goto(address);
    await page.getByRole('button',{name:'main.c',exact:true}).waitFor({timeout:10000});
    await page.locator('nav[aria-label="Workspace views"] button').nth(1).click();
    const input=page.getByPlaceholder('Ask about your code...');await input.waitFor();
    await input.fill('Explain delay');await input.press('Enter');
    await page.getByText('Reference: compatibility/kb_core.c:15-20',{exact:true}).waitFor();
    await page.getByTitle(/หยุดการทำงาน/).waitFor({state:'hidden'});
    await input.fill('Update main');await input.press('Enter');
    await page.getByText('Review AI Changes',{exact:true}).waitFor();
    await page.getByTitle(/หยุดการทำงาน/).waitFor({state:'hidden'});
    const before=await page.evaluate(()=>window.__workflowProbe.state());
    if(!before.source.includes('revision = 0')||!before.pending)throw new Error('Source changed before review');
    await page.getByRole('button',{name:/Keep/}).click();
    await page.waitForFunction(()=>window.__workflowProbe.state().source.includes('revision = 1')&&!window.__workflowProbe.state().pending);
    const calls=await page.evaluate(()=>window.__workflowProbe.commands);
    const accepts=calls.filter(c=>c.cmd==='accept_diff');
    const sent=calls.filter(c=>c.cmd==='send_ai_message');
    if(sent[0]?.args.messages.at(-1)?.content!=='Explain delay')throw new Error('Synthetic context was inserted into the user text');
    if(accepts.length!==1||accepts[0].args.expectedRevision!=='fixture-r1')throw new Error('Approval did not bind the pending revision');
    const boardGuide=await probeBoardGuide(page,root);
    const promptExamples=await probePromptExamples(page,root);
    if(errors.length||external.length)throw new Error(JSON.stringify({errors,external}));
    const result={production_dist:true,strict_production_csp:true,native_backend_stubbed:true,model_response_fixture:true,live_provider:false,
        knowledge_citation_rendered:true,code_edit_reviewed:true,source_unchanged_before_keep:true,revision_bound_keep:true,
        automatic_project_path_export:false,
        board_preparation_guide:boardGuide,
        prompt_examples:promptExamples,
        external_requests_attempted:external,page_errors:errors,scope:'Actual compiled React UI with in-memory event/native fixture; actual native Engine/journal and installer probes are separate evidence'};
    result.production_index_sha256=createHash('sha256').update(await readFile(resolve(dist,'index.html'))).digest('hex');
    result.scope+='; current development dist, not a replacement NSIS qualification';
    await mkdir(dirname(reportPath),{recursive:true});await writeFile(reportPath,JSON.stringify(result,null,2)+'\n');
    console.log(JSON.stringify(result));
}catch(error){
    const debug=await page.evaluate(()=>({body:document.body.innerText,commands:window.__workflowProbe?.commands}));
    await writeFile('src-tauri/target/production-workflow-debug.json',JSON.stringify({errors,external,...debug},null,2));
    console.error(JSON.stringify({errors,external,body:debug.body.slice(0,800),commands:debug.commands?.slice(0,12)}));throw error;
}finally{await browser.close();await new Promise(resolve=>server.close(resolve));}
