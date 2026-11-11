// Real WebView2/React/Monaco + native IPC. Public scratch fixtures only.
// No mocked invoke/provider/device responses; no automatic board run or flash.
import {spawn,execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {createRequire} from 'node:module';
import {mkdir,readFile,writeFile,access} from 'node:fs/promises';
import {resolve,extname,sep} from 'node:path';
import {createHash} from 'node:crypto';
import {createServer} from 'node:http';

const root=process.cwd();
const require=createRequire(resolve(root,'src-tauri/target/learning-ui-tools/package.json'));
const {chromium}=require('playwright-core');
const run=resolve(root,'src-tauri/target/desktop-evaluation',`workspace-${Date.now()}`);
await mkdir(run,{recursive:true});
const output=resolve(run,'result.json');
const sdk=resolve(process.env.APPDATA??'','th.nectec.vibekidbright/toolchain');
const idf=resolve(sdk,'frameworks/esp-idf-v5.5.4'),tools=resolve(sdk,'tools');
const port=19447,origin=`http://127.0.0.1:${port+1}`,dist=resolve(root,'dist');
const csp=JSON.parse(await readFile(resolve(root,'src-tauri/tauri.conf.json'),'utf8')).app.security.csp;
const hash=value=>createHash('sha256').update(value).digest('hex');
const report={schema_version:1,date:'2026-10-27',scope:'Actual isolated debug-native desktop using production frontend, real Tauri IPC, real Monaco and real scratch filesystem. Not an installed-release GUI test.',run_directory:run,status:'RUNNING',cases:[],blockers:[],page_errors:[],hardware_run:false,hardware_flash:false,paid_model_called:false,native_ipc_mocked:false};
await writeFile(output,JSON.stringify(report,null,2)+'\n');
const server=createServer(async(req,res)=>{
 const path=resolve(dist,'.'+decodeURIComponent(new URL(req.url,origin).pathname==='/'?'/index.html':new URL(req.url,origin).pathname));
 if(!path.startsWith(dist+sep)){res.writeHead(403);res.end();return;}
 try{res.setHeader('Content-Security-Policy',csp);res.setHeader('Content-Type',({'.html':'text/html','.js':'text/javascript','.css':'text/css','.png':'image/png','.svg':'image/svg+xml','.ttf':'font/ttf'})[extname(path)]??'application/octet-stream');res.end(await readFile(path));}
 catch{res.writeHead(404);res.end();}
});
await new Promise((ok,fail)=>{server.once('error',fail);server.listen(port+1,'127.0.0.1',ok);});
const child=spawn(resolve(root,'src-tauri/target/debug/examples/desktop_evaluation.exe'),[run,idf,tools,String(port)],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});
let nativeLog='';child.stdout.on('data',b=>nativeLog+=b);child.stderr.on('data',b=>nativeLog+=b);
let browser,page;const delay=ms=>new Promise(ok=>setTimeout(ok,ms));
const assert=(value,message)=>{if(!value)throw new Error(message);};
const native=async(command,args)=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
const saveReport=()=>writeFile(output,JSON.stringify(report,null,2)+'\n');
const passed=async(id,details={})=>{report.cases.push({id,status:'PASS',...details});await saveReport();console.log(JSON.stringify({case:id,status:'PASS'}));};
const picker=async(action,directory='')=>{
 const {stdout}=await promisify(execFile)('powershell.exe',['-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',resolve(root,'scripts/lib/workspace-picker.ps1'),'-AppProcessId',String(child.pid),'-Action',action,'-Directory',directory],{cwd:root,windowsHide:true,timeout:15000,maxBuffer:200000});
 return stdout.trim();
};
try{
 for(let i=0;i<60;i++){
  if(child.exitCode!==null)throw new Error(`Isolated app exited ${child.exitCode}: ${nativeLog.slice(-1200)}`);
  try{browser=await chromium.connectOverCDP(`http://127.0.0.1:${port}`,{timeout:1000});break;}catch{await delay(300);}
 }
 assert(browser,'WebView2 loopback CDP unavailable');
 const context=browser.contexts()[0];page=context.pages()[0]??await context.waitForEvent('page');
 page.setDefaultTimeout(10000);
 page.on('pageerror',e=>report.page_errors.push(String(e)));
 await page.waitForURL(url=>url.origin===origin,{timeout:15000});
 await page.waitForFunction(()=>document.readyState!=='loading'&&!!window.__TAURI_INTERNALS__?.invoke,null,{timeout:15000});
 await passed('native_desktop_launch',{executable_sha256:hash(await readFile(resolve(root,'src-tauri/target/debug/examples/desktop_evaluation.exe'))),frontend_index_sha256:hash(await readFile(resolve(dist,'index.html')))});
 await page.addInitScript(()=>{window.MonacoEnvironment={globalAPI:true};});
 await page.evaluate(()=>{localStorage.setItem('vibe-active-view','editor');localStorage.setItem('vibe-ai-panel','false');});
 await page.reload();
 await page.getByRole('button',{name:'OPEN',exact:true}).waitFor();
 await page.getByRole('button',{name:'OPEN',exact:true}).click();
 const controls=await picker('inspect');
 await writeFile(resolve(run,'picker-controls.json'),controls+'\n');
 await picker('cancel');
 await passed('open_native_folder_picker_and_cancel');
 const projects=resolve(run,'projects');await mkdir(projects);
 const a=await native('create_uai_plus_project',{path:projects,name:'plus_a'});
 const b=await native('create_uai_plus_project',{path:projects,name:'plus_b'});
 const c=await native('create_idf_project',{path:projects,name:'classic'});
 const entry=p=>resolve(p,'main/main.py');
 const codeA='# Desktop workspace fixture A\nprint("PROJECT_A")\n';
 const codeB='# Desktop workspace fixture B\nprint("PROJECT_B")\n';
 await writeFile(entry(a),codeA);await writeFile(entry(b),codeB);
 const calibration=Buffer.from('BANDS = {"released": (2448, 2480), "s1": (0, 13), "s2": (362, 402)}\n');
 await writeFile(resolve(b,'main/button_calibration.py'),calibration);
 await passed('native_new_plus_and_classic_fixtures',{projects:[a,b,c]});
 const current=()=>page.evaluate(()=>localStorage.getItem('vibe-project-dir'));
 const same=(left,right)=>left?.replace(/\\/g,'/').toLowerCase()===right.replace(/\\/g,'/').toLowerCase();
 const waitProject=async p=>page.waitForFunction(p=>localStorage.getItem('vibe-project-dir')?.replace(/\\/g,'/').toLowerCase()===p.replace(/\\/g,'/').toLowerCase(),p,{timeout:15000});
 const open=async p=>{report.active_step='native Open '+p;await page.getByRole('button',{name:'OPEN',exact:true}).click();await picker('select',p);await waitProject(p);};
 const model=()=>page.evaluate(()=>{const e=window.monaco?.editor.getEditors().find(e=>e.getModel()&&e.getDomNode()?.offsetParent!==null);return e?{text:e.getValue(),path:decodeURIComponent(e.getModel().uri.toString()).replace(/^file:\/\/\//,''),model_uri:e.getModel().uri.toString()}:null;});
 const waitModel=async text=>page.waitForFunction(text=>window.monaco?.editor.getEditors().some(e=>e.getModel()&&e.getDomNode()?.offsetParent!==null&&e.getValue()===text),text,{timeout:15000});
 await open(a);report.active_step='wait for Monaco A';await waitModel(codeA);
 const modelA=await model();report.model_a=modelA;assert(same(modelA.path,entry(a)),'Project A Monaco identity is wrong');
 await open(b);await waitModel(codeB);
 const modelB=await model();assert(same(modelB.path,entry(b)),'Project B Monaco identity is wrong');
 await page.screenshot({path:resolve(run,'01-plus-b-open.png')});
 await passed('native_open_two_plus_projects_same_filename',{project_a_model:modelA.path,project_b_model:modelB.path,expected_editor_text_verified:true});
 const edit=async text=>{await page.evaluate(()=>window.monaco.editor.getEditors().find(e=>e.getModel()&&e.getDomNode()?.offsetParent!==null).focus());await page.keyboard.press('Control+A');await page.keyboard.insertText(text);await waitModel(text);};
 const waitDisk=async(path,text)=>{for(let i=0;i<80;i++){if(await readFile(path,'utf8')===text)return;await delay(100);}throw new Error('Saved disk bytes differ at '+path);};
 const changedB='# Saved from real Monaco keyboard input\nprint("SAVED_B")\n';
 await edit(changedB);await page.keyboard.press('Control+s');await waitDisk(entry(b),changedB);
 assert(await readFile(entry(a),'utf8')===codeA,'Saving B changed A');
 await passed('real_monaco_edit_ctrl_s_writes_only_active_project');
 const dirtyB='# Unsaved B kept while switching\nprint("DIRTY_B")\n';
 await edit(dirtyB);await open(a);await waitModel(codeA);await open(b);await waitModel(dirtyB);
 assert(await readFile(entry(b),'utf8')===changedB,'Unsaved text was unexpectedly written');
 await passed('unsaved_buffer_survives_native_open_away_and_back');
 await page.keyboard.press('Control+s');await waitDisk(entry(b),dirtyB);
 const diskB='# Changed on disk before reopening the same folder\nprint("DISK_B")\n';
 await writeFile(entry(b),diskB);await open(b);await waitModel(diskB);
 await passed('reopen_same_folder_refreshes_saved_file_from_disk');
 await page.reload();await page.getByRole('button',{name:'main.py',exact:true}).first().click();await waitModel(diskB);
 assert(same(await current(),b),'Saved project root not restored after WebView reload');
 await passed('webview_reload_restores_selected_root_and_file_bytes',{full_process_restart:false});
 const caseDir=resolve(projects,'งาน มีช่องว่าง');await mkdir(caseDir);
 await page.getByRole('button',{name:'NEW',exact:true}).click();
 await page.getByRole('button',{name:'Browse',exact:true}).click();await picker('select',caseDir);
 await page.locator('input[value="my_esp_project"]').fill('new_plus');
 await page.getByRole('button',{name:'Create Project',exact:true}).click();
 const newly=resolve(caseDir,'new_plus');await waitProject(newly);
 await page.waitForFunction(()=>window.monaco?.editor.getEditors().some(e=>e.getModel()&&e.getDomNode()?.offsetParent!==null&&e.getModel().uri.fsPath.endsWith('main.py')));
 assert(await native('get_project_kind',{path:newly})==='kidbright_uai_plus','New used wrong board type');
 await passed('real_new_plus_with_thai_and_space_parent_path');
 await open(b);await waitModel(diskB);
 let dialogError=null;const dialogMessages=[];
 page.on('dialog',async dialog=>{dialogMessages.push(dialog.message());try{await dialog.accept();}catch(e){dialogError=String(e);}});
 const copies=resolve(run,'copies');await mkdir(copies);
 await page.getByRole('button',{name:'Save Project',exact:true}).click();await picker('select',copies);
 const copied=resolve(copies,'plus_b');await waitProject(copied);await waitModel(diskB);
 assert(await native('get_project_kind',{path:copied})==='kidbright_uai_plus','Save As lost Plus manifest');
 assert(hash(await readFile(resolve(copied,'main/button_calibration.py')))===hash(calibration),'Save As lost helper/calibration');
 assert(await readFile(entry(copied),'utf8')===diskB,'Copied main file mismatch');
 await passed('real_save_as_copies_main_helpers_metadata_and_switches',{destination:copied,files_verified:['main/main.py','main/button_calibration.py','.vibekidbright/project.json']});
 await page.screenshot({path:resolve(run,'02-save-as-result.png')});
 const nested=resolve(copied,'nested');await mkdir(nested);
 await page.getByRole('button',{name:'Save Project',exact:true}).click();await picker('select',nested);
 await page.getByText(/Choose a destination outside the source project/).first().waitFor();
 assert(same(await current(),copied),'Nested copy changed active root');
 let recursive=false;try{await access(resolve(nested,'plus_b'));recursive=true;}catch{}
 assert(!recursive,'Nested copy created recursive destination');
 await passed('real_save_as_rejects_source_descendant_without_creating_it');
 const original=await readFile(entry(copied));
 let rejected=false;try{await native('create_uai_plus_project',{path:copies,name:'plus_b'});}catch{rejected=true;}
 assert(rejected&&hash(await readFile(entry(copied)))===hash(original),'Duplicate New damaged existing code');
 await passed('native_duplicate_new_keeps_existing_code');
 await page.getByTitle('New File in main',{exact:true}).click();
 const inline=page.getByRole('region',{name:'Project files'}).locator('input');await inline.fill('probe_helper.py');await inline.press('Enter');
 await page.getByRole('button',{name:'probe_helper.py',exact:true}).first().waitFor();await waitModel('');
 const helper='# Saved helper fixture\nVALUE = 17\n';await edit(helper);await page.keyboard.press('Control+s');await waitDisk(resolve(copied,'main/probe_helper.py'),helper);
 await page.getByRole('button',{name:'probe_helper.py',exact:true}).first().click({button:'right'});
 await page.getByText('Rename',{exact:false}).last().click();
 await inline.fill('probe_renamed.py');await inline.press('Enter');
 await page.getByRole('button',{name:'probe_renamed.py',exact:true}).first().waitFor();
 assert(await readFile(resolve(copied,'main/probe_renamed.py'),'utf8')===helper,'Rename changed file bytes');
 await page.getByRole('button',{name:'probe_renamed.py',exact:true}).first().click({button:'right'});await page.getByText('Delete',{exact:false}).last().click();
 for(let i=0;i<60;i++){try{await access(resolve(copied,'main/probe_renamed.py'));await delay(100);}catch{break;}}
 let exists=false;try{await access(resolve(copied,'main/probe_renamed.py'));exists=true;}catch{}
 assert(!exists,'Delete did not remove the test helper');
 await passed('real_sidebar_create_save_rename_delete_helper');
 await open(c);await page.getByRole('button',{name:'main.c',exact:true}).first().waitFor();
 const diskC=await readFile(resolve(c,'main/main.c'),'utf8');await waitModel(diskC);
 assert(await native('get_project_kind',{path:c})==='esp_idf','Classic kind incorrect');
 assert(await page.getByRole('button',{name:/Build & Flash/}).count()===1,'Classic board controls not selected');
 await passed('native_open_classic_selects_c_editor_and_build_controls');
 await open(b);await waitModel(diskB);
 assert(await page.getByRole('button',{name:'Connect Plus',exact:true}).count()===1,'Plus board controls not selected');
 await passed('native_open_back_to_plus_selects_python_controls');
 try{const fingerprint=await native('probe_uai_plus');report.board_probe={status:'REACHABLE',tls_fingerprint:fingerprint,execution_requested:false};}
 catch(error){report.board_probe={status:'BLOCKED',reason:String(error),execution_requested:false};report.blockers.push({id:'physical_plus_run_stop_io',reason:'Board transport probe did not succeed: '+String(error),manual:true});}
 report.serial_ports=await native('list_serial_ports');
 report.blockers.push({id:'installed_release_gui_and_upgrade',reason:'This session uses the actual native debug harness and production frontend; no system installer was executed.',manual:true});
 report.blockers.push({id:'live_ai_create_keep_undo',reason:'Credential-isolated harness disables paid credentials/production AI generation. No model API called.',manual:true});
 report.blockers.push({id:'physical_classic_flash',reason:'No board selected for flashing; GUI Build & Flash would program hardware.',manual:true});
 report.blockers.push({id:'process_restart_with_unsaved_buffers',reason:'Buffers are documented as session-only. WebView reload was tested after saving; unsaved recovery across process restart is not promised.',manual:true});
 report.dialog_messages=dialogMessages;assert(!dialogError,dialogError);assert(report.page_errors.length===0,'Desktop JavaScript errors occurred');
 await page.screenshot({path:resolve(run,'03-final-plus.png')});
 report.status='PASS_WITH_MANUAL_ITEMS';
}catch(error){report.error=String(error);report.error_stack=error.stack;report.status='FAILED';if(page)try{await page.screenshot({path:resolve(run,'failure.png')});report.body=(await page.locator('body').innerText()).slice(-5000);}catch{}}
finally{
 await saveReport();
 if(browser)await browser.close().catch(()=>{});
 if(child.exitCode===null)child.kill();
 await new Promise(ok=>server.close(ok));
 await writeFile(resolve(run,'native.log'),nativeLog);
 console.log(JSON.stringify({status:report.status,output,cases:report.cases.length,error:report.error}));
}
if(report.status==='FAILED')process.exitCode=1;
