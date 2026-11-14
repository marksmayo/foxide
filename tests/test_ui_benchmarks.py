from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from ui_benchmarks import action_rows


class UiBenchmarkTests(unittest.TestCase):
    def test_plan_cli_can_generate_a_report_without_launching_players_or_awarding_medals(self):
        import json
        import subprocess
        import shutil
        from uuid import uuid4
        root = Path(__file__).resolve().parents[1]
        scratch = root / 'target/test-tmp'
        scratch.mkdir(parents=True, exist_ok=True)
        directory = scratch / ('ui-report-' + uuid4().hex)
        directory.mkdir()
        try:
            output, report = Path(directory) / 'plan', Path(directory) / 'report'
            subprocess.run([sys.executable, str(root/'scripts/run_ui_benchmarks.py'), '--players', 'foxide',
                            '--actions', 'menu_media', '--output', str(output)], check=True, capture_output=True)
            self.assertEqual(json.loads((output/'measurements.json').read_text()), [])
            subprocess.run([sys.executable, str(root/'scripts/build_medal_report.py'), '--measurements',
                            str(output/'measurements.json'), '--sessions', str(output/'session.json'),
                            '--output', str(report)], check=True, capture_output=True)
            scores = json.loads((report/'medals.json').read_text())
            outcomes = [b['runtime']['foxide']['status'] for b in scores['boards'].values()]
            self.assertIn('Verified UI actions 0/1', outcomes)
            self.assertFalse(any('Verified playback' in value for value in outcomes))
        finally:
            self.assertEqual(directory.resolve().parent, scratch.resolve())
            shutil.rmtree(directory)

    def test_plan_includes_the_entire_native_field_without_claiming_unimplemented_actions(self):
        import json
        from run_ui_benchmarks import plan
        catalog = json.loads((Path(__file__).resolve().parents[1] / 'benchmarks/ui-events.json').read_text())
        rows = plan(catalog, 'windows')
        self.assertEqual({r['player'] for r in rows}, {p['id'] for p in catalog['players'] if 'windows' in p['platforms']})
        statuses = {(r['player'], r['action']): r['status'] for r in rows}
        self.assertEqual(statuses['foxide', 'menu_media'], 'ready-for-interactive-validation')
        self.assertEqual(statuses['mpv', 'menu_media'], 'not-applicable')
        self.assertEqual(statuses['foxide', 'seek_forward'], 'presentation-collector-required')
        self.assertEqual(statuses['potplayer', 'menu_media'], 'adapter-required')

    def test_only_real_input_and_matching_owned_ui_postconditions_can_earn_medals(self):
        scenario = dict(id='menu_media', verification='accessibility-state', condition='media-menu-visible')
        case = dict(player='foxide', platform='windows', profile='native-ui/same-host-and-fixture', trial=0,
                    source='case.json', status='verified', pid=42, input_ns=1000000, completed_ns=14000000,
                    input_method='native-input', verification=dict(kind='accessibility-state', condition='media-menu-visible', passed=True, pid=42),
                    before='before.json', after='after.json')
        rows = action_rows(case, scenario)
        self.assertEqual(rows[0]['event'], 'ui_menu_media_latency')
        self.assertEqual(rows[0]['value'], 13)
        self.assertTrue(rows[0]['verified'])
        for patch in [dict(status='window-created'), dict(input_method='direct-engine-command'),
                      dict(completed_ns=0), dict(verification=dict(kind='accessibility-state', condition='wrong-menu', passed=True, pid=42))]:
            self.assertEqual(action_rows(dict(case, **patch), scenario), [])
        physical = dict(scenario, verification='presented-frame')
        self.assertEqual(action_rows(case, physical), [], 'UI acknowledgement is not video presentation')

    def test_resource_rows_require_valid_values_and_a_verified_ui_action(self):
        scenario = dict(id='menu_audio', verification='accessibility-state', condition='audio-menu-visible')
        case = dict(player='vlc', platform='windows', profile='native-ui/idle', trial=1, source='case.json',
                    status='verified', pid=42, input_ns=10, completed_ns=20, input_method='accessibility-action',
                    verification=dict(kind='accessibility-state', condition='audio-menu-visible', passed=True, pid=42),
                    before='before.json', after='after.json', cpu_time_ms=2.5, rss_mib=80)
        rows = action_rows(case, scenario)
        self.assertEqual([r['event'] for r in rows], ['ui_menu_audio_latency', 'ui_menu_audio_cpu_time', 'ui_menu_audio_rss'])
        self.assertEqual(len(action_rows(dict(case, cpu_time_ms=float('nan'), rss_mib=-1), scenario)), 1)
        for patch in [dict(pid=None), dict(pid=True), dict(input_ns=True), dict(completed_ns=None)]:
            self.assertEqual(action_rows(dict(case, **patch), scenario), [])
