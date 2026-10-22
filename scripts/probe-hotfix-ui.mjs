// Compiled UI probe with an in-memory IPC fixture; never installs a real SDK.
import {createServer} from "node:http";
import {readFile, writeFile, mkdir} from "node:fs/promises";
import {existsSync} from "node:fs";
import {createRequire} from "node:module";
import {resolve, extname, sep} from "node:path";
const root=process.cwd();
const fixtureTools=process.env.VIBE_PROBE_TOOLS;
if(!fixtureTools) throw new Error("Set VIBE_PROBE_TOOLS to a local package directory containing playwright-core.");
const require=createRequire(resolve(fixtureTools,"package.json"));
const {chromium}=require("playwright-core");
const executable=["C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe","C:/Program Files/Microsoft/Edge/Application/msedge.exe","C:/Program Files/Google/Chrome/Application/chrome.exe"].find(existsSync);
if(!executable) throw new Error("Local Chromium browser unavailable");
const config=JSON.parse(await readFile("src-tauri/tauri.conf.json","utf8"));
const dist=resolve(root,"dist");
const server=createServer(async(req,res)=>{
    const file=resolve(dist,"."+decodeURIComponent((req.url??"/").split("?")[0]==="/"?"/index.html":req.url.split("?")[0]));
    if(!file.startsWith(dist+sep)){res.writeHead(403);res.end();return;}
    try{
        const data=await readFile(file);
        res.setHeader("Content-Security-Policy",config.app.security.csp);
        res.setHeader("Content-Type",({".html":"text/html",".js":"text/javascript",".css":"text/css",".ttf":"font/ttf",".svg":"image/svg+xml"})[extname(file)]??"application/octet-stream");
        res.end(data);
    }catch{res.writeHead(404);res.end();}
});
await new Promise(r=>server.listen(0,"127.0.0.1",r));
const address=`http://127.0.0.1:${server.address().port}`;
const browser=await chromium.launch({executablePath:executable,headless:true});
const page=await browser.newPage();
const errors=[],external=[];
page.on("pageerror",e=>errors.push(String(e)));
await page.route("**/*",route=>route.request().url().startsWith(address)?route.continue():(external.push(route.request().url()),route.abort()));
try{
    await page.addInitScript(()=>{
        localStorage.clear();
        localStorage.setItem("vibe-selected-board","miuaiplus");
        const listeners=new Map(),callbacks=new Map(),calls=[];let next=1,installed=false;
        window.__hotfixProbe={calls,installed:()=>installed};
        window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
        window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:"main"},currentWebview:{label:"main"}},
            transformCallback:fn=>{const id=next++;callbacks.set(id,fn);return id;},unregisterCallback:id=>callbacks.delete(id),
            invoke:async(name,args={})=>{
                calls.push(name);
                if(name==="plugin:event|listen"){listeners.set(args.event,args.handler);return args.handler;}
                if(name==="plugin:event|unlisten"){listeners.delete(args.event);return;}
                if(name==="check_toolchain")return {status:installed?"ready":"not_installed",version:installed?"1.0.1":null,path:installed?"fixture-toolchain":null};
                if(name==="download_toolchain"){installed=true;return "installed";}
                if(name==="check_esp_idf"){if(!installed)throw new Error("fixture SDK not installed");return "Ready: fixture SDK";}
                if(name==="get_idf_custom_paths")return {idf_path:"",tools_path:""};
                if(["get_knowledge_base_files","list_project_files","list_serial_ports"].includes(name))return [];
                if(name==="get_provider")return "local";
                if(name==="get_base_url")return "http://127.0.0.1:1234/v1";
                if(name.startsWith("get_"))return "";
                return null;
            }};
    });
    await page.goto(address);
    const board=page.locator('select[title="Select Board Type"]');
    await board.waitFor();
    if(await board.inputValue()!=="kidbright32")throw new Error("Restored experimental target leaked into hotfix");
    if(await board.locator("option").count()!==1)throw new Error("Unexpected board target");
    await page.waitForFunction(()=>window.__hotfixProbe.installed(),undefined,{timeout:15000});
    await page.getByRole("button",{name:/Build & Flash/}).waitFor();
    await page.waitForFunction(()=>[...document.querySelectorAll("button")].some(b=>b.textContent.includes("Build & Flash")&&!b.disabled),undefined,{timeout:5000});
    await page.getByText("Ready",{exact:true}).waitFor();
    const calls=await page.evaluate(()=>window.__hotfixProbe.calls);
    if(calls.filter(n=>n==="download_toolchain").length!==1)throw new Error("Duplicate auto installation");
    if(calls.lastIndexOf("check_esp_idf")<calls.indexOf("download_toolchain"))throw new Error("SDK status was not refreshed after activation");
    if(calls.some(n=>n.includes("adb")))throw new Error("AI-board IPC invoked");
    if(errors.length||external.length)throw new Error(JSON.stringify({errors,external}));
    const report={version:"3.7.1",compiled_ui:true,native_ipc_fixture:true,classic_only:true,legacy_board_preference_normalized:true,auto_install_once:true,ready_without_reload:true,missed_progress_event_handled:true,sdk_status_refreshed:true,page_errors:errors,external_requests:external,live_provider:false,real_sdk_installed:false};
    await mkdir("src-tauri/target",{recursive:true});
    await writeFile("src-tauri/target/hotfix-ui-verification.json",JSON.stringify(report,null,2)+"\n");
    console.log(JSON.stringify(report));
}finally{await browser.close();await new Promise(r=>server.close(r));}
