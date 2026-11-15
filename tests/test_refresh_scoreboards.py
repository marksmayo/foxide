import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))


class RefreshTests(unittest.TestCase):
    def test_cli_preserves_versioned_field_evidence_and_renders_native_results(self):
        import contextlib
        import io
        import shutil
        from uuid import uuid4
        from unittest.mock import patch
        from refresh_scoreboards import main
        root = Path(__file__).resolve().parents[1]
        temp = root/'results/local'/('refresh-'+uuid4().hex)
        (temp/'bundles').mkdir(parents=True)
        (temp/'platforms/windows').mkdir(parents=True)
        try:
            cases = [dict(player=p, profile='native-default-window/avc1080/test', trial=i,
                          status='verified-playback', verified=True, source='original-case.json')
                     for p in ('mpv', 'vlc') for i in range(3)]
            session = dict(session=dict(system='windows', utc='2026-11-15T00:00:00Z',
                                        host='local host', presentation='native-window', source_commit='abc'),
                           inventory=[], cases=cases)
            rows = [dict(player=c['player'], profile=c['profile'], trial=c['trial'], platform='windows',
                         event='cpu_avc1080', value=10 if c['player']=='mpv' else 20, verified=True, source=c['source']) for c in cases]
            (temp/'platforms/windows/session.json').write_text(json.dumps(session), encoding='utf-8')
            (temp/'platforms/windows/measurements.json').write_text(json.dumps(rows), encoding='utf-8')
            args = ['refresh_scoreboards.py', '--bundles', str(temp/'bundles'), '--platform-dir', str(temp/'platforms'),
                    '--output', str(temp/'report'), '--readme', str(temp/'README.md')]
            with patch.object(sys, 'argv', args), contextlib.redirect_stdout(io.StringIO()):
                main()
            evidence = json.loads((temp/'bundles/windows-local-native-window.json').read_text('utf-8'))
            self.assertIn('#/cases/', evidence['measurements'][0]['source'])
            self.assertEqual(evidence['cases'][0]['original_source'], 'original-case.json')
            self.assertIn('Windowed playback', (temp/'README.md').read_text('utf-8'))
            self.assertIn('local host', (temp/'README.md').read_text('utf-8'))
            with patch.object(sys, 'argv', args), contextlib.redirect_stdout(io.StringIO()):
                main()
            self.assertEqual(len(list((temp/'bundles').glob('*.json'))), 1)
        finally:
            self.assertEqual(temp.resolve().parent, (root/'results/local').resolve())
            shutil.rmtree(temp)

    def test_latest_session_replaces_same_field_without_hiding_failed_runs(self):
        from refresh_scoreboards import latest_collections
        def field(date, presentation='software-null', hosted=False, verified=True):
            return dict(session=dict(system='windows', utc=date, presentation=presentation,
                                     run_id='42' if hosted else None),
                        cases=[dict(verified=verified)], measurements=[])
        old = field('2026-11-13T00:00:00Z')
        latest = field('2026-11-15T00:00:00+00:00', verified=False)
        desktop = field('2026-11-15T00:00:00Z', 'native-window')
        hosted = field('2026-11-14T00:00:00Z', hosted=True)
        selected = latest_collections([old, latest, desktop, hosted])
        self.assertNotIn(old, selected)
        self.assertIn(latest, selected, 'A newer failure must not fall back to older wins')
        self.assertIn(desktop, selected)
        self.assertIn(hosted, selected)
