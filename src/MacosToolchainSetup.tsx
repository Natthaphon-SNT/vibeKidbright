import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import MacosSdkSetup from "./MacosSdkSetup";

export default function MacosToolchainSetup({onReady}:{onReady:()=>void}) {
  const [error,setError]=useState("");
  const [checking,setChecking]=useState(true);
  const check=async()=>{
    setChecking(true);
    try {
      const status=await invoke<{status:string;error?:string}>("check_toolchain");
      if(status.status==="ready") onReady();
      else setError(status.error??"Install the native macOS SDK before Build & Flash.");
    } catch(error){setError(String(error));}
    finally{setChecking(false);}
  };
  useEffect(()=>{void check();},[]);
  return <div style={{position:"fixed",bottom:16,left:16,zIndex:9998,width:360,maxHeight:"75vh",overflowY:"auto",padding:18,borderRadius:12,background:"#111827",color:"#e2e8f0",boxShadow:"0 8px 28px #0008"}}>
    <strong>{checking?"Checking native macOS SDK…":"macOS SDK Setup"}</strong>
    {error&&<p role="alert" style={{fontSize:12,overflowWrap:"anywhere",color:"#fca5a5"}}>{error}</p>}
    <MacosSdkSetup onRecheck={check}/>
  </div>;
}
