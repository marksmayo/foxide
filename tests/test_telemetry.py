import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import shutil
from contextlib import contextmanager
from uuid import uuid4
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('telemetry', ROOT / 'scripts/telemetry.py')
telemetry = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(telemetry)
MEDIA_SPEC = importlib.util.spec_from_file_location('media', ROOT / 'scripts/generate-media.py')
media = importlib.util.module_from_spec(MEDIA_SPEC)
MEDIA_SPEC.loader.exec_module(media)


@contextmanager
def scratch_directory():
    # Regular inherited workspace permissions also allow the sampler child to write.
    # Python 3.14's Windows tempfile private DACL can exclude restricted child tokens.
    base = (ROOT / 'target/test-tmp').resolve()
    base.mkdir(parents=True, exist_ok=True)
    path = base / ('foxide sampler ' + uuid4().hex)
    path.mkdir()
    try:
        yield path
    finally:
        assert path.resolve().parent == base
        shutil.rmtree(path)


class TelemetryTests(unittest.TestCase):
    def test_tree_excludes_unrelated_and_handles_nested_children(self):
        self.assertEqual(telemetry.descendants({1: 0, 2: 1, 3: 2, 4: 0}, 1), {1, 2, 3})
        self.assertEqual(telemetry.descendants({1: 0}, 99), set())

    def test_cpu_time_formats(self):
        self.assertEqual(telemetry.cpu_seconds('01:02.50'), 62.5)
        self.assertEqual(telemetry.cpu_seconds('1-01:02:03'), 90123)

    def test_live_process_tree_rss_and_cpu_delta(self):
        program = "import subprocess,sys,time; p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(10)']); print(p.pid,flush=True); time.sleep(10)"
        with subprocess.Popen([sys.executable, '-c', program], stdout=subprocess.PIPE, text=True) as root:
            child_pid = int(root.stdout.readline())
            try:
                with scratch_directory() as temp:
                    output = Path(temp) / 'samples.jsonl'
                    # Allow two process-tree scans on Windows under coverage instrumentation.
                    # A 700ms wall-clock budget can expire after only the first scan.
                    sampler = subprocess.Popen([sys.executable, str(ROOT / 'scripts/telemetry.py'),
                                                '--output', str(output), '--duration', '3',
                                                '--interval-ms', '100'], stdin=subprocess.PIPE,
                                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                    try:
                        ready = sampler.stdout.readline().strip()
                        if ready != 'ready':
                            self.fail('Sampler initialization failed: ' + sampler.stderr.read())
                        stdout, stderr = sampler.communicate(str(root.pid) + '\n', timeout=10)
                        self.assertEqual(sampler.returncode, 0, stderr + stdout)
                        records = [json.loads(line) for line in output.read_text().splitlines()]
                        samples = [r for r in records if r['kind'] == 'sample']
                        self.assertGreaterEqual(len(samples), 2)
                        self.assertGreater(samples[0]['rss_bytes'], 0)
                        self.assertGreaterEqual(samples[0]['process_count'], 2)
                        self.assertIsNone(samples[0]['cpu_percent_one_core'])
                        self.assertIsNotNone(samples[-1]['cpu_percent_one_core'])
                        self.assertLess(samples[0]['elapsed_ms'], samples[-1]['elapsed_ms'])
                    finally:
                        if sampler.poll() is None:
                            sampler.kill()
                        sampler.wait()
                        for stream in (sampler.stdin, sampler.stdout, sampler.stderr):
                            if stream:
                                stream.close()
            finally:
                root.terminate()
                root.wait()
                if os.name == 'nt':
                    import ctypes
                    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
                    kernel.OpenProcess.argtypes = [ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]
                    kernel.OpenProcess.restype = ctypes.c_void_p
                    kernel.TerminateProcess.argtypes = [ctypes.c_void_p, ctypes.c_uint]
                    kernel.CloseHandle.argtypes = [ctypes.c_void_p]
                    handle = kernel.OpenProcess(1, False, child_pid)
                    if handle:
                        kernel.TerminateProcess(handle, 0)
                        kernel.CloseHandle(handle)
                else:
                    import signal
                    try:
                        os.kill(child_pid, signal.SIGTERM)
                    except ProcessLookupError:
                        pass

    def test_fixture_contract_rejects_missing_audio(self):
        with self.assertRaisesRegex(AssertionError, 'Missing audio'):
            media.validate('A', {'streams': [], 'format': {'duration': '3'}}, 3)

    def test_flac_contract_requires_cover_art(self):
        with self.assertRaisesRegex(AssertionError, 'Missing cover'):
            media.validate('D', {'streams': [{'codec_type': 'audio', 'codec_name': 'flac'}],
                                 'format': {'duration': '3'}}, 3)


if __name__ == '__main__':
    unittest.main()
