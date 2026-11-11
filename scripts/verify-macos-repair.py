"""Verify the 3.7.1 Mac backport from the installed universal DMG."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import subprocess
import tempfile

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--installer',type=Path,required=True)
parser.add_argument('--architecture',choices=['arm64','x86_64'],required=True)
parser.add_argument('--report',type=Path,required=True)
args=parser.parse_args()
args.report.parent.mkdir(parents=True,exist_ok=True)
record={'version':'3.7.1','architecture':args.architecture,'status':'failed','hardware_flashed':False}
try:
    image=(args.installer/'VibeKidbright-IDE_3.7.1_macos-clean_universal.dmg').resolve()
    with image.open('rb') as stream:digest=hashlib.file_digest(stream,'sha256').hexdigest()
    assert f'{digest}  {image.name}' in (args.installer/'SHA256SUMS-macos-3.7.1-clean.txt').read_text().splitlines()
    with tempfile.TemporaryDirectory(prefix='vibekidbright-371-repair-') as temporary:
        scratch=Path(temporary).resolve()
        mount=scratch/'mount';mount.mkdir()
        subprocess.run(['hdiutil','verify',str(image)],check=True,capture_output=True)
        subprocess.run(['hdiutil','attach','-readonly','-nobrowse','-mountpoint',str(mount),str(image)],check=True,capture_output=True)
        try:
            app,=mount.glob('*.app')
            installed=scratch/'Installed Applications'/app.name
            installed.parent.mkdir()
            subprocess.run(['ditto',str(app),str(installed)],check=True)
        finally:subprocess.run(['hdiutil','detach',str(mount)],check=True,capture_output=True)
        with (installed/'Contents/Info.plist').open('rb') as stream:info=plistlib.load(stream)
        assert info['CFBundleShortVersionString']=='3.7.1','App version was changed'
        binary=installed/'Contents/MacOS'/info['CFBundleExecutable']
        slices=subprocess.check_output(['lipo','-archs',str(binary)],text=True).split()
        assert set(slices)=={'arm64','x86_64'}
        subprocess.run(['codesign','--verify','--deep','--strict',str(installed)],check=True,capture_output=True)
        working=scratch/'separate-working-directory';working.mkdir()
        env=os.environ.copy()
        for key in list(env):
            if key.startswith(('IDF_','VIBEKIDBRIGHT_','DYLD_')) or key in ('PYTHONHOME','PYTHONPATH','VIRTUAL_ENV'):env.pop(key,None)
        env['PATH']='/usr/bin:/bin:/usr/sbin:/sbin'
        sdk_report=scratch/'sdk-report.json'
        home=Path.home()
        sdk_root=home/'.vibekidbright/macos-sdk'
        current=json.loads((sdk_root/'current.json').read_text())
        generation=current['directory']
        assert generation.startswith('install-') and all(c.isascii() and (c.isalnum() or c=='-') for c in generation)
        active=sdk_root/generation
        assert not (active/'old-install-marker').exists()
        with args.report.with_suffix('.sdk.log').open('w') as log:
            result=subprocess.run([str(binary),'--sdk-self-check',str(sdk_report),str(scratch/'esp32-build'),str(active/'esp-idf'),str(active/'.espressif')],cwd=working,env=env,stdout=log,stderr=log,timeout=660)
        sdk=json.loads(sdk_report.read_text()) if sdk_report.is_file() else {}
        expected='aarch64' if args.architecture=='arm64' else 'x86_64'
        assert result.returncode==0 and sdk.get('native_sdk_build') and sdk.get('architecture')==expected, 'Native firmware build failed: '+json.dumps(sdk)
        assert sdk['firmware_bytes']>0 and sdk['version']=='3.7.1'
        with args.report.with_suffix('.startup.log').open('w') as log:
            child=subprocess.Popen(['/usr/bin/arch','-'+args.architecture,str(binary)],cwd=working,env=env,stdout=log,stderr=log)
            try:
                try:child.wait(timeout=15)
                except subprocess.TimeoutExpired:pass
                else:raise RuntimeError(f'GUI exited during startup: {child.returncode}')
            finally:
                if child.poll() is None:
                    child.terminate()
                    try:child.wait(timeout=10)
                    except subprocess.TimeoutExpired:child.kill();child.wait()
        record.update(status='passed',sha256=digest,sdk_build=sdk,signature_verified=True,universal_slices=slices,gui_startup_seconds=15,copied_from_dmg=True)
except Exception as error:
    record['error']=str(error)
    if isinstance(error,subprocess.CalledProcessError):record['stderr']=(error.stderr or b'').decode(errors='replace')[-3000:]
args.report.write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps(record,indent=2))
raise SystemExit(0 if record['status']=='passed' else 1)
