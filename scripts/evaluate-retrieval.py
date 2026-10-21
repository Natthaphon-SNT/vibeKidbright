"""Fixed-policy local embedding runs into a fresh evidence directory.

No ranker tuning. Public corpus only; one subprocess/model at a time. Windows
peak working set includes ONNX. Historical reports are never overwritten.
"""
import argparse
import ctypes
from ctypes import wintypes
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT/'docs/ai-engineering'
MODES = {'lexical': [], 'bundled': ['--dense'], 'bundled-expanded': ['--dense', '--expand'],
         'bundled-routed': ['--dense', '--routing'],
         **{name: ['--dense', '--model-directory', str(ROOT/'src-tauri/target/retrieval-candidates'/name)]
            for name in ('multilingual-e5-small', 'multilingual-minilm')}}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def new_output_directory(path):
    path = path.absolute()
    if not path.parent.resolve().is_relative_to(ROOT.resolve()):
        raise ValueError('Evidence output must stay within workspace')
    path.mkdir()  # Exclusive directory reservation before any evaluation.
    return path


def run_mode(binary, dataset, mode, directory, date, timeout):
    report = directory/(mode+'.json')
    command = [str(binary), '--dataset', str(dataset), '--output', str(report), '--date', date, *MODES[mode]]
    started = time.monotonic(); working_set = private_bytes = 0
    with (directory/(mode+'.stdout.log')).open('xb') as stdout, (directory/(mode+'.stderr.log')).open('xb') as stderr:
        process = subprocess.Popen(command, cwd=ROOT, stdout=stdout, stderr=stderr)
        if os.name == 'nt':
            class Memory(ctypes.Structure):
                _fields_ = [('cb', wintypes.DWORD), ('PageFaultCount', wintypes.DWORD),
                            *[(name, ctypes.c_size_t) for name in ['PeakWorkingSetSize', 'WorkingSetSize', 'QuotaPeakPagedPoolUsage', 'QuotaPagedPoolUsage', 'QuotaPeakNonPagedPoolUsage', 'QuotaNonPagedPoolUsage', 'PagefileUsage', 'PeakPagefileUsage', 'PrivateUsage']]]
            psapi = ctypes.WinDLL('psapi', use_last_error=True)
            psapi.GetProcessMemoryInfo.argtypes = [wintypes.HANDLE, ctypes.POINTER(Memory), wintypes.DWORD]
        try:
            while process.poll() is None:
                if time.monotonic()-started > timeout:
                    raise TimeoutError('Evaluation subprocess exceeded timeout')
                if os.name == 'nt':
                    memory = Memory(); memory.cb = ctypes.sizeof(memory)
                    if psapi.GetProcessMemoryInfo(int(process._handle), ctypes.byref(memory), memory.cb):
                        working_set = max(working_set, memory.PeakWorkingSetSize)
                        private_bytes = max(private_bytes, memory.PrivateUsage)
                time.sleep(.1)
        except BaseException:
            process.kill(); process.wait(); raise
    outcome = {'mode': mode, 'command': command, 'exit_code': process.returncode,
               'resources': {'peak_working_set_bytes': working_set or None, 'peak_sampled_private_bytes': private_bytes or None,
                             'wall_seconds': time.monotonic()-started, 'sampling_interval_ms': 100,
                             'method': 'Windows GetProcessMemoryInfo; single process and fixed-policy run; host load not experimentally controlled'}}
    if process.returncode:
        return outcome
    result = json.loads(report.read_text(encoding='utf-8'))
    if result['dataset_sha256'] != digest(dataset):
        raise ValueError('Subprocess evaluated a different dataset')
    latencies = sorted(row['latency_ms'] for row in result['queries'])
    outcome.update(report=report.name, report_sha256=digest(report), metrics=result['macro_average'],
                   latency_ms={'median': statistics.median(latencies), 'p95': latencies[int((len(latencies)-1)*.95)]},
                   index_ms=result['index_latency_ms'], model_bytes=result['model_artifact_bytes'],
                   language={lang: {'queries': sum(row['language'] == lang for row in result['queries']),
                                    'metrics': {key: statistics.mean(row['metrics'][key] for row in result['queries'] if row['language'] == lang) for key in result['macro_average']},
                                    'latency_median_ms': statistics.median(row['latency_ms'] for row in result['queries'] if row['language'] == lang)}
                             for lang in sorted({row['language'] for row in result['queries']})})
    return outcome


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output-directory', required=True, type=Path)
    parser.add_argument('--modes', choices=sorted(MODES), nargs='+', default=['lexical', 'bundled-expanded'])
    parser.add_argument('--date', required=True, help='Client-local measurement date, not a model/version inference')
    parser.add_argument('--timeout-seconds', type=int, default=900)
    args = parser.parse_args()
    if not 1 <= args.timeout_seconds <= 3600 or len(set(args.modes)) != len(args.modes):
        parser.error('Use unique modes and a 1..3600 second timeout')
    dataset = DOCS/'evaluation-heldout-v1.json'
    freeze = json.loads((DOCS/'evaluation-heldout-v1-freeze.json').read_text(encoding='utf-8'))
    if digest(dataset) != freeze['sha256']:
        raise ValueError('Frozen dataset changed')
    for name, expected in freeze['corpus_sha256'].items():
        if digest(ROOT/'knowledge_base'/name) != expected:
            raise ValueError('Frozen corpus differs: '+name)
    binary = ROOT/'src-tauri/target/debug/examples/knowledge_eval.exe'
    directory = new_output_directory(args.output_directory)
    record = {'date': args.date, 'status': 'INCOMPLETE', 'dataset_sha256': freeze['sha256'], 'modes_predeclared': args.modes,
              'ranker_changed': False, 'model_replaced': False, 'labels_independently_reviewed': False,
              'binary_sha256': digest(binary), 'runner_sha256': digest(Path(__file__)), 'runs': [],
              'limits': ['Fixed historical evaluation labels; no tuning or candidate acceptance from this run',
                         'No LLM, hardware, firmware build or installer-size qualification',
                         'Subprocess timeout/cancellation terminates this owned embedding process; no SDK child tree is involved',
                         'Private bytes are sampled; peak working set is process-wide, not per-language incremental RAM']}
    output = directory/'comparison.json'
    try:
        for mode in args.modes:
            print('Evaluating '+mode, flush=True)
            result = run_mode(binary, dataset, mode, directory, args.date, args.timeout_seconds)
            record['runs'].append(result)
            output.write_text(json.dumps(record, indent=2)+'\n', encoding='utf-8')
            print(json.dumps({key: result.get(key) for key in ('mode', 'exit_code', 'metrics', 'resources')}), flush=True)
        record['status'] = 'MEASURED_NOT_INDEPENDENTLY_REVIEWED' if all(row['exit_code'] == 0 for row in record['runs']) else 'FAILED'
    except BaseException as error:
        record['status'] = 'FAILED'; record['error'] = type(error).__name__+': '+str(error)
        raise
    finally:
        output.write_text(json.dumps(record, indent=2)+'\n', encoding='utf-8')
    raise SystemExit(0 if record['status'] != 'FAILED' else 1)


if __name__ == '__main__':
    main()
