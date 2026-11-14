import json
from pathlib import Path
import sys
import shutil
import uuid
import unittest
from unittest import mock
import contextlib
import io

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from build_medal_report import write_report, main, source_link


class MedalReportTests(unittest.TestCase):
    def test_report_links_cannot_execute_script_or_inject_markup(self):
        self.assertEqual(source_link('javascript:alert(1)', '<unsafe>'), '&lt;unsafe&gt;')

    def test_cli_turns_real_measurement_rows_into_separate_platform_medals(self):
        root = Path(__file__).resolve().parents[1]
        scratch = root / 'results/local'
        scratch.mkdir(parents=True, exist_ok=True)
        tmp = scratch / ('measured-report-' + uuid.uuid4().hex)
        tmp.mkdir()
        try:
            rows = [dict(player=p, value=value, event='cpu_avc1080', platform='windows', profile='host-fixture-scenario-settings', trial=i, verified=True, source='trial.json')
                    for p, value in [('mpv', 10), ('vlc', 20)] for i in range(3)]
            rows += [dict(player='mpv', value=0, event='cpu_avc1080', platform='linux', profile='different-host', trial=0, verified=False, source='failed.json')]
            data = tmp / 'measurements.json'
            data.write_text(json.dumps(rows), encoding='utf-8')
            output = tmp / 'report'
            with mock.patch.object(sys, 'argv', ['build_medal_report.py', '--measurements', str(data), '--output', str(output)]), contextlib.redirect_stdout(io.StringIO()):
                main()
            report = json.loads((output / 'medals.json').read_text('utf-8'))
            windows = {p['id']: p for p in report['boards']['windows']['players']}
            self.assertEqual(windows['mpv']['performance_gold'], 1)
            self.assertEqual(windows['vlc']['performance_silver'], 1)
            self.assertTrue(all(p['performance_gold'] == 0 for p in report['boards']['linux']['players']))
            self.assertIn('Insufficient comparable evidence', (output / 'report.html').read_text('utf-8'))
            self.assertTrue((output / 'features.json').is_file())
            with mock.patch.dict('os.environ', {'GITHUB_REPOSITORY': 'marksmayo/foxide', 'GITHUB_RUN_ID': '42', 'GITHUB_SHA': 'source-sha'}):
                write_report(json.loads((root / 'benchmarks/features.json').read_text('utf-8')),
                             json.loads((root / 'benchmarks/performance-events.json').read_text('utf-8')), rows, output)
            metadata = json.loads((output / 'medals.json').read_text('utf-8'))['provenance']
            self.assertEqual(metadata['run_url'], 'https://github.com/marksmayo/foxide/actions/runs/42')
            self.assertIn('actions/runs/42', (output / 'report.html').read_text('utf-8'))
        finally:
            self.assertEqual(tmp.resolve().parent, scratch.resolve())
            shutil.rmtree(tmp)

    def test_publishes_three_platform_graphs_and_preserves_manual_readme(self):
        root = Path(__file__).resolve().parents[1]
        catalog = json.loads((root / 'benchmarks/features.json').read_text('utf-8'))
        protocol = json.loads((root / 'benchmarks/performance-events.json').read_text('utf-8'))
        scratch = root / 'results/local'
        scratch.mkdir(parents=True, exist_ok=True)
        tmp = scratch / ('report-test-' + uuid.uuid4().hex)
        tmp.mkdir()
        try:
            output = tmp / 'scores'
            readme = tmp / 'README.md'
            readme.write_text('# Foxide\n\nManual introduction.\n', encoding='utf-8')
            write_report(catalog, protocol, [], output, readme)
            self.assertIn('Manual introduction.', readme.read_text('utf-8'))
            page = (output / 'report.html').read_text('utf-8')
            self.assertIn('Shared gold', page)
            self.assertIn('Pending measurement', page)
            self.assertEqual(page.count('<svg '), 3)  # Graphics remain in a shared HTML file.
            for platform in ['windows', 'macos', 'linux']:
                self.assertTrue((output / (platform + '.svg')).is_file())
                self.assertIn(platform + '.svg', readme.read_text('utf-8'))
            # Rebuilding replaces one generated region instead of duplicating it.
            write_report(catalog, protocol, [], output, readme)
            self.assertEqual(readme.read_text('utf-8').count('<!-- foxide-scores:start -->'), 1)
        finally:
            self.assertEqual(tmp.resolve().parent, scratch.resolve())
            shutil.rmtree(tmp)


if __name__ == '__main__':
    unittest.main()
