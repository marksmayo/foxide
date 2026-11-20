"""Exercise standalone maintenance commands through their public entry points."""
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import subprocess
import sys
from uuid import uuid4
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))

def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / filename)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

source_lines = load('source_lines_under_test', 'source_lines.py')
session = load('session_manifest_under_test', 'session-manifest.py')

@contextlib.contextmanager
def scratch(root):
    folder = root / uuid4().hex
    folder.mkdir()
    try:
        yield folder
    finally:
        assert folder.resolve().parent == root.resolve()
        shutil.rmtree(folder)

class ToolingTests(unittest.TestCase):
    def setUp(self):
        self.temp_root = ROOT / 'results/local/tooling-tests'
        self.temp_root.mkdir(parents=True, exist_ok=True)

    def test_source_badges_count_only_authored_languages_and_refresh_idempotently(self):
        with scratch(self.temp_root) as folder:
            root = Path(folder)
            for directory in ('crates/demo', 'scripts/__pycache__', 'vendor', 'docs'):
                (root / directory).mkdir(parents=True)
            (root / 'Cargo.toml').write_text('[workspace]\n', encoding='utf-8')
            (root / 'crates/demo/main.rs').write_text('// comment\n\nfn main() {}\n', encoding='utf-8')
            (root / 'scripts/a.py').write_text('print(1)\n', encoding='utf-8')
            (root / 'scripts/__pycache__/skip.py').write_text('ignored\n', encoding='utf-8')
            (root / 'vendor/skip.rs').write_text('ignored\n', encoding='utf-8')
            (root / 'README.md').write_text('# Demo\nPreserve this prose.\n', encoding='utf-8')
            counts = source_lines.counts(root)
            self.assertEqual({key: (v['files'], v['lines']) for key, v in counts.items()},
                             {'Rust': (1, 3), 'Python': (1, 1), 'TOML': (1, 1)})
            with patch.object(source_lines, 'ROOT', root), contextlib.redirect_stdout(io.StringIO()):
                source_lines.main()
                first = (root / 'README.md').read_text('utf-8')
                source_lines.main()
            self.assertEqual((root / 'README.md').read_text('utf-8'), first)
            self.assertIn('Preserve this prose.', first)
            self.assertIn('Rust%20source%20lines-3-dea584', first)
            self.assertEqual(json.loads((root / 'docs/source-lines.json').read_text('utf-8')), counts)

    def test_session_manifest_hashes_inputs_records_probes_and_refuses_to_overwrite(self):
        self.enterContext(patch.multiple(session.platform, platform=lambda: "Test OS", node=lambda: "fixture-host", machine=lambda: "fixture-arch", processor=lambda: "fixture-cpu"))
        with scratch(self.temp_root) as folder:
            root = Path(folder)
            media = root / 'media'
            media.mkdir()
            manifest = media / 'manifest.json'
            manifest.write_text('{"fixtures": []}', encoding='utf-8')
            tool = root / 'ffmpeg'
            tool.write_bytes(b'version fixture')
            output = root / 'session.json'
            args = ['manifest', '--media', str(media), '--output', str(output), '--tool', str(tool), '--notes', 'Controlled fixture']
            probe = subprocess.CompletedProcess([], 7, 'v1', 'diagnostic')
            with patch.object(sys, 'argv', args), patch.object(session.subprocess, 'run', return_value=probe) as run, contextlib.redirect_stdout(io.StringIO()):
                session.main()
                run.assert_called_once_with([str(tool.resolve()), '-version'], capture_output=True, text=True, encoding='utf-8', errors='replace', timeout=5)
                with self.assertRaises(FileExistsError):
                    session.main()
            record = json.loads(output.read_text('utf-8'))
            self.assertEqual(record['suite_sha256'], hashlib.sha256(manifest.read_bytes()).hexdigest())
            self.assertEqual(record['tools'][0]['sha256'], hashlib.sha256(tool.read_bytes()).hexdigest())
            self.assertEqual(record['tools'][0]['bytes'], len(tool.read_bytes()))
            self.assertEqual(record['tools'][0]['version_probe'], {'stdout': 'v1', 'stderr': 'diagnostic', 'returncode': 7})
            self.assertEqual(record['notes'], 'Controlled fixture')
            self.assertIsNone(record['operator_required']['gpu_and_driver'])
            timeout = root / 'timeout.json'
            args[args.index(str(output))] = str(timeout)
            with patch.object(sys, 'argv', args), patch.object(session.subprocess, 'run', side_effect=subprocess.TimeoutExpired('tool', 5)), contextlib.redirect_stdout(io.StringIO()):
                session.main()
            self.assertIn('five seconds', json.loads(timeout.read_text('utf-8'))['tools'][0]['version_probe']['error'])

    @unittest.skipUnless(shutil.which('node'), 'Node is required for the report interaction contract')
    def test_offline_report_contract_runs_and_rejects_remote_scripts(self):
        script = ROOT / 'scripts/check-report.mjs'
        report = ROOT / 'results/windows-comparison/report.html'
        good = subprocess.run(['node', str(script), str(report)], capture_output=True, text=True, timeout=30)
        self.assertEqual(good.returncode, 0, good.stderr)
        self.assertIn('Report checks passed', good.stdout)
        with scratch(self.temp_root) as folder:
            invalid = Path(folder) / 'report.html'
            invalid.write_text(report.read_text('utf-8') + '<script src="https://example.org/tracker.js"></script>', encoding='utf-8')
            bad = subprocess.run(['node', str(script), str(invalid)], capture_output=True, text=True, timeout=30)
            self.assertNotEqual(bad.returncode, 0)
            self.assertIn('without remote scripts', bad.stderr)
