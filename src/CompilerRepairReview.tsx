import {useEffect,useRef,useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";
import DiffReview from "./DiffReview";
import {toast} from "./Toast";

interface RepairReview {
    revision:string;
    session:{root:string;phase:string;attempts:number;max_attempts:number;last_error:string|null;
        diagnostics:{severity:string;category:string;message:string}[];
        changes:{path:string;before:string|null;proposed:string}[]};
}
export default function CompilerRepairReview({projectDir,isBusy,hasUnsavedChanges,onRetry}:{projectDir:string;isBusy:boolean;hasUnsavedChanges:boolean;onRetry:()=>void}) {
    const [review,setReview]=useState<RepairReview|null>(null);
    const [busy,setBusy]=useState(false);
    const [approval,setApproval]=useState<{projectDir:string;revision:string}|null>(null);
    const approved=!!review&&approval?.projectDir===projectDir&&approval.revision===review.revision;
    useEffect(()=>{
        // Binding blocks stale consent during render; this cleanup also prevents
        // old consent resurfacing if a previous revision is later returned.
        setApproval(current=>current&&(current.projectDir!==projectDir||current.revision!==review?.revision)?null:current);
    },[projectDir,review?.revision]);
    const projectRef=useRef(projectDir);projectRef.current=projectDir;
    useEffect(()=>{
        let active=true;let unlisten:(()=>void)|undefined;
        setReview(null);setApproval(null);
        if(!projectDir||projectDir===".")return;
        let request=0;
        const refresh=()=>{const current=++request;return invoke<RepairReview|null>("get_compiler_repair",{projectDir}).then(r=>{if(active&&current===request)setReview(r?.session?r:null);}).catch(e=>{if(active&&current===request)toast(`Repair recovery unavailable: ${String(e)}`,"error");});};
        refresh();
        listen("compiler-repair-changed",refresh).then(fn=>{if(active)unlisten=fn;else fn();}).catch(()=>{});
        return()=>{active=false;unlisten?.();};
    },[projectDir]);
    if(!review)return null;
    const {session}=review;
    const running=session.phase==="applying"||session.phase==="building";
    const action=async(command:string,extra:Record<string,unknown>={})=>{
        const stopping=command==="cancel_compiler_repair"&&review.session.phase==="building";
        if(!stopping&&(busy||isBusy||hasUnsavedChanges))return;
        if(command==="apply_compiler_repair"&&!approved)return;
        if(command==="apply_compiler_repair")setApproval(null);
        setBusy(true);const selected=projectDir;
        try{const r=await invoke<RepairReview>(command,{projectDir,expectedRevision:review.revision,...extra});if(projectRef.current===selected)setReview(r);}
        catch(e){toast(String(e),"error");if(projectRef.current===selected)invoke<RepairReview|null>("get_compiler_repair",{projectDir}).then(setReview).catch(()=>{});}
        finally{setBusy(false);}
    };
    const disabled=busy||isBusy||hasUnsavedChanges;
    return <section aria-label="Compiler Repair" className="p-3 border-t text-xs max-h-[45vh] overflow-auto" style={{borderColor:"var(--border-color)",background:"var(--bg-sidebar)"}}>
        <strong>Compiler Repair · {session.attempts}/{session.max_attempts}</strong>
        <p role="status">{session.phase.replace(/_/g," ")}</p>
        {session.attempts===0&&['awaiting_proposal','awaiting_approval'].includes(session.phase)&&<label>Maximum attempts <select aria-label="Maximum repair attempts" value={session.max_attempts} disabled={disabled} onChange={e=>action("set_compiler_repair_limit",{maxAttempts:Number(e.target.value)})}>{[1,2,3].map(n=><option key={n} value={n}>{n}</option>)}</select></label>}
        {session.last_error&&<pre className="whitespace-pre-wrap max-h-24 overflow-auto">{session.last_error}</pre>}
        {hasUnsavedChanges&&<p>Save or discard editor drafts before reviewing a repair.</p>}
        {session.changes.map(change=><details key={change.path} className="my-2"><summary>{change.path.replace(/\\/g,"/").split("/").slice(-2).join("/")}</summary><DiffReview original={change.before??""} modified={change.proposed}/></details>)}
        {session.phase==="awaiting_approval"&&<>
            <label className="flex gap-2 my-2"><input type="checkbox" checked={approved} onChange={e=>setApproval(e.target.checked?{projectDir,revision:review.revision}:null)} disabled={disabled}/>I reviewed all files in this attempt</label>
            <button disabled={disabled||!approved} onClick={()=>action("apply_compiler_repair")} className="px-2 py-1 border rounded">Apply this patch &amp; verify build</button>
        </>}
        {session.phase==="awaiting_proposal"&&<button disabled={disabled} onClick={onRetry} className="px-2 py-1 border rounded">Ask AI for a proposal</button>}
        {session.changes.length>0&&!running&&<button disabled={disabled} onClick={()=>action("rollback_compiler_repair")} className="px-2 py-1 border rounded ml-2">Undo this attempt</button>}
        {!['succeeded','cancelled','exhausted'].includes(session.phase)&&<button disabled={session.phase==="applying"||(!running&&disabled)} onClick={()=>action("cancel_compiler_repair")} className="px-2 py-1 border rounded ml-2">{running?"Stop verification":"Cancel repair"}</button>}
        {session.phase==="succeeded"&&<p>Configured ESP-IDF build passed. Physical behavior remains unverified.</p>}
        {session.phase==="exhausted"&&<p>Attempt limit reached. Review the diagnostics manually.</p>}
    </section>;
}
