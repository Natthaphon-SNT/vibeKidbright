import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[2]

class RealVibeHarnessTests(unittest.TestCase):
    def run_case(self,*args):
        return subprocess.run(["node","scripts/evaluate-vibe-coder-real.mjs",*args],cwd=ROOT,text=True,capture_output=True)

    def test_unknown_budget_duplicate_and_missing_options_fail_before_output(self):
        with tempfile.TemporaryDirectory(dir=ROOT/'src-tauri/target') as directory:
            output=Path(directory)/'must-not-exist.json'
            for args, message in [
                (['--max-requests','0'], 'Unknown evaluation option'),
                (['--run','--run'], 'Duplicate evaluation option'),
                (['--model'], 'Missing value'),
                (['--model','--run'], 'Missing value'),
            ]:
                result=self.run_case('--output',str(output),*args)
                self.assertNotEqual(result.returncode,0)
                self.assertIn(message,result.stderr)
                self.assertFalse(output.exists())
            # Even explicit CLI authorization cannot bypass an unsupported cap:
            # rejection occurs before desktop connection/model discovery.
            result=self.run_case('--output',str(output),'--run','--authorize-local-inference',
                '--endpoint','http://127.0.0.1:1234/v1','--model','fixture','--max-requests','0')
            self.assertNotEqual(result.returncode,0)
            self.assertIn('Unknown evaluation option',result.stderr)
            self.assertFalse(output.exists())

    def test_prepare_has_null_grades_and_never_overwrites_evidence(self):
        with tempfile.TemporaryDirectory(dir=ROOT/'src-tauri/target') as directory:
            output=Path(directory)/'result.json'
            first=self.run_case('--output',str(output))
            self.assertEqual(first.returncode,0,first.stderr)
            before=output.read_bytes();report=json.loads(before)
            self.assertEqual(report['status'],'NOT_EXECUTED')
            self.assertEqual(report['inference_requests'],0)
            self.assertEqual(report['patches_approved_by_harness'],0)
            self.assertIsNone(report['grading']['model_output_quality'])
            self.assertTrue(all(value=='NOT_EVALUATED' for value in report['capability_levels'].values()))
            self.assertNotEqual(self.run_case('--output',str(output)).returncode,0)
            self.assertEqual(output.read_bytes(),before)

    def test_existing_report_blocks_run_before_discovery_or_desktop_connection(self):
        with tempfile.TemporaryDirectory(dir=ROOT/'src-tauri/target') as directory:
            output=Path(directory)/'existing.json';output.write_text('preserved',encoding='utf-8')
            result=self.run_case('--run','--authorize-local-inference','--endpoint','http://127.0.0.1:1/v1','--model','unused','--output',str(output))
            self.assertNotEqual(result.returncode,0)
            # The unimplemented transport/cost safeguard now rejects run mode
            # even earlier than output reservation. Both paths preserve bytes.
            self.assertIn('Inference blocked:',result.stderr)
            self.assertEqual(output.read_text(encoding='utf-8'),'preserved')
            prepared=self.run_case('--output',str(output))
            self.assertNotEqual(prepared.returncode,0)
            self.assertIn('EEXIST',prepared.stderr)
            self.assertEqual(output.read_text(encoding='utf-8'),'preserved')

    def test_unknown_case_is_rejected_instead_of_claiming_empty_success(self):
        result=self.run_case('--case','imaginary')
        self.assertNotEqual(result.returncode,0)
        self.assertIn('Unknown or non-prompt case',result.stderr)
        result=self.run_case('--workflow','imaginary')
        self.assertNotEqual(result.returncode,0)
        self.assertIn('Unknown workflow',result.stderr)

    def test_run_requires_separate_explicit_inference_authorization(self):
        result=self.run_case('--run','--endpoint','http://127.0.0.1:1234/v1','--model','test')
        self.assertNotEqual(result.returncode,0)
        self.assertIn('Explicit local inference authorization',result.stderr)

    def test_authorized_cli_flags_cannot_run_without_native_request_and_cost_limits(self):
        with tempfile.TemporaryDirectory(dir=ROOT/'src-tauri/target') as directory:
            output=Path(directory)/'must-not-exist.json'
            result=self.run_case('--output',str(output),'--run','--authorize-local-inference',
                '--endpoint','http://127.0.0.1:1234/v1','--model','public-fixture')
            self.assertNotEqual(result.returncode,0)
            self.assertIn('native HTTP-request/retry and USD budget enforcement is not implemented',result.stderr)
            self.assertIn('No desktop connection or model discovery was attempted',result.stderr)
            self.assertFalse(output.exists())

    def test_paid_or_remote_endpoint_is_rejected_before_any_request(self):
        result=self.run_case('--run','--authorize-local-inference','--endpoint','https://api.openai.com/v1','--model','test')
        self.assertNotEqual(result.returncode,0)
        self.assertIn('Only explicit local HTTP endpoints',result.stderr)

    def test_output_cannot_escape_workspace(self):
        result=self.run_case('--output',str(ROOT.parent/'not-allowed.json'))
        self.assertNotEqual(result.returncode,0)
        self.assertIn('Output must stay',result.stderr)

if __name__=='__main__':unittest.main()
