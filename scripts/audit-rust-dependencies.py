"""Read-only public advisory lookup. Sends crates.io names/versions only, no source.
Includes inactive/build/cross-platform lock entries; findings need reachability review.
"""
import json
from pathlib import Path
import tomllib
import hashlib
import re
import subprocess
import urllib.request
import argparse

ROOT=Path(__file__).resolve().parents[1]
def request(path,data=None,timeout=15):
    body=json.dumps(data).encode('utf-8') if data is not None else None
    with urllib.request.urlopen(urllib.request.Request('https://api.osv.dev/v1/'+path,data=body,headers={'Content-Type':'application/json','User-Agent':'public-crate-security-evaluation'}),timeout=timeout) as response:
        return json.load(response)

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--date',required=True)
    args=parser.parse_args()
    if not args.output.resolve().is_relative_to(ROOT.resolve()):parser.error('Output must stay within workspace')
    with args.output.open('x',encoding='utf-8') as out:out.write('{"query_succeeded":false,"status":"INCOMPLETE"}\n')
    lock=tomllib.loads((ROOT/'src-tauri/Cargo.lock').read_text(encoding='utf-8'))
    packages=[p for p in lock['package'] if p.get('source','').startswith('registry+')]
    answer=request('querybatch',{'queries':[{'package':{'name':p['name'],'ecosystem':'crates.io'},'version':p['version']} for p in packages]})
    affected=[];details={}
    for package,result in zip(packages,answer['results'],strict=True):
        ids=[v['id'] for v in result.get('vulns',[])]
        if not ids:continue
        affected.append({'name':package['name'],'version':package['version'],'advisories':ids})
        for identifier in ids:
            if identifier not in details:details[identifier]=request('vulns/'+identifier)
    report={'date':args.date,'source':'https://osv.dev/','lock_packages_queried':len(packages),'network_payload':'Public crates.io names and versions only',
            'scope':'Entire Cargo.lock including build/development/inactive platform entries. Advisory presence alone does not establish exploitable reachability.',
            'affected_packages':affected,'advisories':details}
    tree=subprocess.run(['cargo','tree','--manifest-path',str(ROOT/'src-tauri/Cargo.toml'),'--offline','--locked',
                         '--target','x86_64-pc-windows-msvc','--edges','normal,build','--prefix','none'],capture_output=True,text=True,check=True)
    windows={(match[1],match[2]) for line in tree.stdout.splitlines() if (match:=re.match(r'([A-Za-z0-9_-]+) v([^\s]+)',line))}
    selected=[p for p in affected if (p['name'],p['version']) in windows]
    report['windows_selected_graph_findings']=selected
    report['windows_selected_graph_nonmaintenance_advisories']=sorted({identifier for p in selected for identifier in p['advisories']
        if not any(term in details[identifier].get('summary','').lower() for term in ['unmaintained','no longer maintained'])})
    report['lock_sha256']=hashlib.sha256((ROOT/'src-tauri/Cargo.lock').read_bytes()).hexdigest()
    report['query_succeeded']=True;report['all_lockfile_clean']=not affected
    args.output.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'queried':len(packages),'affected':affected,'advisory_count':len(details)}))

if __name__=='__main__':main()
