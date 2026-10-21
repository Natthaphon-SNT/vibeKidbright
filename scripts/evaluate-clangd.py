"""Evaluate pinned official clangd against a real generated ESP-IDF compile DB.
Does not modify flags, install into the app, enable configs, or export private code.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import queue
import subprocess
import threading
import time
import urllib.request
import urllib.parse
import zipfile

ROOT=Path(__file__).resolve().parents[1]
VERSION='23.1.0'
DIGEST='23412a240756a162e7b98a282f36aa2a23a88db5ce16a0cbc4fef7253768c810'
URL=f'https://github.com/clangd/clangd/releases/download/{VERSION}/clangd-windows-{VERSION}.zip'

def lsp_probe(binary,project,database,driver,cache):
    log=(cache/'lsp.log').open('wb')
    child=subprocess.Popen([str(binary),'--compile-commands-dir='+str(database.parent),'--query-driver='+str(driver),
        '--enable-config=false','--background-index=false','--log=error'],cwd=project,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=log)
    messages=queue.Queue()
    def reader():
        while True:
            header={}
            while line:=child.stdout.readline():
                if line in (b'\r\n',b'\n'):break
                key,_,value=line.decode('ascii').partition(':');header[key.lower()]=value.strip()
            if not header:return
            length=int(header.get('content-length',0))
            body=child.stdout.read(length)
            if len(body)!=length:return
            messages.put(json.loads(body))
    threading.Thread(target=reader,daemon=True).start()
    def send(method,params,identifier=None):
        msg={'jsonrpc':'2.0','method':method,'params':params}
        if identifier is not None:msg['id']=identifier
        data=json.dumps(msg).encode('utf-8');child.stdin.write(f'Content-Length: {len(data)}\r\n\r\n'.encode('ascii')+data);child.stdin.flush()
    def response(identifier,timeout=30):
        deadline=time.monotonic()+timeout
        while time.monotonic()<deadline:
            message=messages.get(timeout=max(.01,deadline-time.monotonic()))
            if message.get('id')==identifier:
                if 'error' in message:raise RuntimeError(str(message['error']))
                return message.get('result')
        raise TimeoutError('clangd LSP response timeout')
    try:
        send('initialize',{'processId':None,'rootUri':project.as_uri(),'capabilities':{}},1);response(1)
        send('initialized',{})
        paths=[project/'main/main.c',project/'components/kb_reference/kb_core.c']
        for path in paths:send('textDocument/didOpen',{'textDocument':{'uri':path.as_uri(),'languageId':'c','version':1,'text':path.read_text(encoding='utf-8')}})
        ready=set();errors=[];deadline=time.monotonic()+30
        while len(ready)<len(paths) and time.monotonic()<deadline:
            message=messages.get(timeout=max(.01,deadline-time.monotonic()))
            if message.get('method')=='textDocument/publishDiagnostics':
                ready.add(message['params']['uri'])
                errors.extend(d for d in message['params']['diagnostics'] if d.get('severity')==1)
        source=paths[0].read_text(encoding='utf-8');offset=source.index('kb_delay_ms')
        prefix=source[:offset];position={'line':prefix.count('\n'),'character':len(prefix.rsplit('\n',1)[-1].encode('utf-16-le'))//2}
        params={'textDocument':{'uri':paths[0].as_uri()},'position':position}
        send('textDocument/definition',params,2);definitions=response(2)
        send('textDocument/references',{**params,'context':{'includeDeclaration':True}},3);references=response(3)
        def locations(values):
            result=[]
            for location in values if isinstance(values,list) else ([values] if values else []):
                uri=location.get('uri',location.get('targetUri',''));name=urllib.parse.unquote(urllib.parse.urlparse(uri).path)
                if re.match(r'^/[A-Za-z]:',name):name=name[1:]
                path=Path(name).resolve();relative=path.relative_to(project).as_posix() if path.is_relative_to(project) else '<outside-project>'
                result.append({'file':relative,'range':location.get('range',location.get('targetRange'))})
            return result
        result={'method':'actual local LSP initialize/didOpen/definition/references against unmodified SDK CDB; background index off',
                'symbol':'kb_delay_ms','opened_files':len(paths),'diagnostic_errors':errors,'definitions':locations(definitions),'references':locations(references),
                'scope':'two opened generated fixture translation units only; no all-project reference completeness claim'}
        send('shutdown',None,4);response(4,10);send('exit',None);child.wait(timeout=10)
        return result
    finally:
        if child.poll() is None:child.kill();child.wait(timeout=10)
        log.close()

def prepare():
    cache=ROOT/'src-tauri/target/clangd-evaluation';cache.mkdir(exist_ok=True)
    archive=cache/'clangd.zip'
    if not archive.exists() or hashlib.sha256(archive.read_bytes()).hexdigest()!=DIGEST:
        with urllib.request.urlopen(urllib.request.Request(URL,headers={'User-Agent':'vibeKidbright-evaluation'}),timeout=120) as response, archive.open('wb') as out:
            while data:=response.read(1024*1024):out.write(data)
    if hashlib.sha256(archive.read_bytes()).hexdigest()!=DIGEST:raise RuntimeError('Official clangd artifact digest mismatch')
    with zipfile.ZipFile(archive) as source:
        for entry in source.infolist():
            target=(cache/entry.filename).resolve()
            if not target.is_relative_to(cache.resolve()) or (entry.external_attr>>16)&0o170000==0o120000:raise RuntimeError('Unsafe archive member')
        source.extractall(cache)
    binary=next(cache.rglob('clangd.exe'))
    print('Prepared verified official clangd '+VERSION,flush=True)
    return binary

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--prepare',action='store_true');parser.add_argument('--project',type=Path)
    args=parser.parse_args()
    cache=ROOT/'src-tauri/target/clangd-evaluation'
    binary=prepare() if args.prepare else next(cache.rglob('clangd.exe'))
    if not args.project:return
    project=args.project.resolve()
    # This evaluation is deliberately restricted to generated public fixtures.
    if not project.is_relative_to((ROOT/'src-tauri/target/block-c-evaluation-20261018').resolve()):raise RuntimeError('Use a generated public fixture, not a private project')
    database=project/'build/compile_commands.json';commands=json.loads(database.read_text(encoding='utf-8'))
    main=project/'main/main.c';entry=next(c for c in commands if Path(c['file']).resolve()==main)
    match=re.match(r'^(?:"([^"]+)"|(\S+))',entry['command']);driver=Path(match[1] or match[2]).resolve()
    tools=Path.home()/'AppData/Roaming/th.nectec.vibekidbright/toolchain/tools'
    if not driver.is_relative_to(tools.resolve()) or driver.name!='xtensa-esp32-elf-gcc.exe':raise RuntimeError('Unexpected compiler; do not execute query-driver without verifying it')
    args=[str(binary),'--check='+str(main),'--compile-commands-dir='+str(database.parent),'--query-driver='+str(driver),
          '--enable-config=false','--background-index=false','--log=verbose']
    run=subprocess.run(args,cwd=project,capture_output=True,timeout=120)
    log=run.stdout+run.stderr
    (cache/'check.log').write_bytes(log)
    text=log.decode('utf-8',errors='replace')
    result={'version':VERSION,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'official_archive_sha256':DIGEST,'source_url':URL,
            'project':'src-tauri/target/block-c-evaluation-20261018/'+project.name,'database_entries':len(commands),
            'database_sha256':hashlib.sha256(database.read_bytes()).hexdigest(),'main_sha256':hashlib.sha256(main.read_bytes()).hexdigest(),
            'unmodified_compile_commands':True,'query_driver':'verified configured xtensa-esp32-elf-gcc.exe only',
            'remote_index':False,'project_configs_enabled':False,'exit_code':run.returncode,
            'diagnostic_excerpt':[line for line in text.splitlines() if line.startswith('E[') or 'All checks completed' in line][-30:],
            'semantic_references_integrated':False,'reference_accuracy_measured':False,
            'conclusion':'Check passed' if run.returncode==0 else 'Check reports errors; classify them separately from compiler parsing and the LSP probe'}
    try:result['lsp']=lsp_probe(binary,project,database,driver,cache)
    except Exception as error:result['lsp']={'error':str(error)}
    (ROOT/'docs/ai-engineering/clangd-evaluation.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result),flush=True)

if __name__=='__main__':main()
