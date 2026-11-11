"""Install and launch the original 3.7.1 DMG. Does not certify SDK or USB flash."""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--installer', type=Path, required=True)
parser.add_argument('--architecture', required=True)
parser.add_argument('--report', type=Path, required=True)
args = parser.parse_args()
args.report.parent.mkdir(parents=True, exist_ok=True)
record = {'version':'3.7.1','architecture':args.architecture,'status':'failed','sdk_build_tested':False,'hardware_flashed':False}
try:
    image = (args.installer / 'VibeKidbright-IDE_3.7.1_universal.dmg').resolve()
    with image.open('rb') as stream:
        digest = hashlib.file_digest(stream,'sha256').hexdigest()
    expected = f'{digest}  {image.name}'
    assert expected in (args.installer / 'SHA256SUMS-macos-3.7.1.txt').read_text().splitlines(), 'Checksum mismatch'
    with tempfile.TemporaryDirectory(prefix='vibekidbright-371-') as temporary:
        scratch = Path(temporary)
        mount = scratch / 'mount'
        mount.mkdir()
        subprocess.run(['hdiutil','verify',str(image)],check=True,capture_output=True)
        subprocess.run(['hdiutil','attach','-readonly','-nobrowse','-mountpoint',str(mount),str(image)],check=True,capture_output=True)
        try:
            app, = mount.glob('*.app')
            installed = scratch / 'Installed Applications' / app.name
            installed.parent.mkdir()
            subprocess.run(['ditto',str(app),str(installed)],check=True)
        finally:
            subprocess.run(['hdiutil','detach',str(mount)],check=True,capture_output=True)
        with (installed/'Contents/Info.plist').open('rb') as stream:
            info = plistlib.load(stream)
        assert info['CFBundleShortVersionString']=='3.7.1','Wrong app version'
        binary = installed/'Contents/MacOS'/info['CFBundleExecutable']
        slices = subprocess.check_output(['lipo','-archs',str(binary)],text=True).split()
        assert set(slices)=={'arm64','x86_64'},'Not a universal application'
        subprocess.run(['codesign','--verify','--deep','--strict',str(installed)],check=True,capture_output=True)
        with args.report.with_suffix('.startup.log').open('w') as log:
            child = subprocess.Popen(['/usr/bin/arch','-'+args.architecture,str(binary)],cwd=scratch,stdout=log,stderr=log)
            try:
                try:
                    child.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    pass
                else:
                    raise RuntimeError(f'Application exited during startup: {child.returncode}')
            finally:
                if child.poll() is None:
                    child.terminate()
                    try:
                        child.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        child.kill()
                        child.wait()
        record.update(status='passed',sha256=digest,universal_slices=slices,signature_verified=True,copied_from_dmg=True,startup_seconds=15,
                      source_revision=(args.installer/'macos-3.7.1-source-revision.txt').read_text().strip())
except Exception as error:
    record['error']=str(error)
    if isinstance(error,subprocess.CalledProcessError):
        record['stderr']=(error.stderr or b'').decode(errors='replace')[-4000:]
args.report.write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps(record,indent=2))
raise SystemExit(0 if record['status']=='passed' else 1)
