import {useEffect,useRef,useState} from "react";
import {invoke} from "@tauri-apps/api/core";

export interface CBuffer {path:string;text:string}
interface Location {path:string;line:number;character:number;end_line:number;end_character:number}
interface Answer {engine:string;locations:Location[];limitation:string;buffer_sha256:string;compile_commands_sha256:string|null}
let nextRequest=0;
export default function CIntelligencePanel({projectDir,filePath,buffers,getPosition,onNavigate}:{
    projectDir:string;filePath:string;buffers:CBuffer[];
    getPosition:()=>{lineNumber:number;column:number}|null;
    onNavigate:(path:string,line:number,column:number)=>void;
}) {
    const [expanded,setExpanded]=useState(false);
    const [enabled,setEnabled]=useState(false);
    const [clangdPath,setClangdPath]=useState(()=>localStorage.getItem("vibe-clangd-path")??"");
    const [answer,setAnswer]=useState<Answer|null>(null);
    const [error,setError]=useState("");const [busy,setBusy]=useState(false);
    const sequence=useRef(0);const active=useRef<string|null>(null);
    const context=JSON.stringify([projectDir,filePath,buffers]);
    useEffect(()=>{
        ++sequence.current;setAnswer(null);setError("");setBusy(false);
        return ()=>{++sequence.current;const id=active.current;active.current=null;if(id)void invoke("cancel_c_intelligence",{requestId:id}).catch(()=>{});};
    },[context]);
    const stop=()=>{++sequence.current;const id=active.current;active.current=null;setBusy(false);setError("C intelligence cancelled");if(id)void invoke("cancel_c_intelligence",{requestId:id}).catch(()=>{});};
    const query=async(operation:"definition"|"references")=>{
        const position=getPosition();if(!position){setError("Place the cursor on a C identifier first.");return;}
        const serial=++sequence.current;const id=`c-intelligence-${Date.now()}-${++nextRequest}`;
        active.current=id;setBusy(true);setAnswer(null);setError("");
        try {
            const result=await invoke<Answer>("query_c_intelligence",{projectDir,requestId:id,request:{
                file_path:filePath,buffers,line:position.lineNumber-1,character:position.column-1,
                operation,clangd_path:enabled?clangdPath:"",
            }});
            if(serial===sequence.current)setAnswer(result);
        }catch(error){if(serial===sequence.current)setError(String(error));}
        finally{if(serial===sequence.current){active.current=null;setBusy(false);}}
    };
    return <div className="absolute top-1 left-12 z-20 text-xs" style={{maxWidth:520}}>
        <button type="button" className="px-2 py-1 rounded" style={{background:'var(--bg-modal)',color:'var(--text-secondary)'}} onClick={()=>setExpanded(!expanded)} aria-expanded={expanded}>C intelligence</button>
        {expanded&&<section aria-label="Local C intelligence" className="p-3 rounded shadow-xl space-y-2" style={{background:'var(--bg-modal)',border:'1px solid var(--border-color)',color:'var(--text-primary)'}}>
            <p>Local source navigation. Syntax candidates work without clangd. No source is sent to AI.</p>
            <label className="block"><input type="checkbox" checked={enabled} disabled={busy} onChange={event=>{setEnabled(event.target.checked);setAnswer(null);}}/> Enable a trusted local clangd executable</label>
            {enabled&&<label className="block">Clangd executable<input aria-label="Clangd executable" className="w-full p-1 border rounded" value={clangdPath} disabled={busy} onChange={event=>{setClangdPath(event.target.value);localStorage.setItem("vibe-clangd-path",event.target.value);}}/></label>}
            <div className="flex gap-3">
                <button type="button" disabled={busy} onClick={()=>void query("definition")}>Find definition</button>
                <button type="button" disabled={busy} onClick={()=>void query("references")}>Find references</button>
                {busy&&<button type="button" onClick={stop}>Stop C intelligence</button>}
            </div>
            {error&&<p role="status">{error}</p>}
            {answer&&<><p role="status">{answer.engine==='clangd'?'clangd semantic results':'Tree-sitter syntax candidates'}</p><p>{answer.limitation}</p>
                <ul className="max-h-44 overflow-auto">{answer.locations.map((location,index)=><li key={index}><button type="button" onClick={()=>onNavigate(location.path,location.line+1,location.character+1)}>{location.path.split(/[\\/]/).slice(-2).join('/')}:{location.line+1}:{location.character+1}</button></li>)}</ul>
                {!answer.locations.length&&<p>No matching locations in this scope.</p>}
            </>}
        </section>}
    </div>;
}
