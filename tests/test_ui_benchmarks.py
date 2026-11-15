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
            session = json.loads((output/'session.json').read_text())['session']
            self.assertTrue(session['host'])
            self.assertEqual(session['presentation'], 'native-ui')
            self.assertIn('utc', session)
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

    def test_input_dispatch_and_state_observation_are_timed_separately(self):
        scenario = dict(id='menu_audio', verification='accessibility-state', condition='audio-menu-visible')
        case = dict(player='vlc', platform='linux', profile='native-ui/idle', trial=1, source='case.json',
                    status='verified', pid=42, input_ns=1000000, dispatch_completed_ns=4000000, completed_ns=14000000,
                    input_method='accessibility-action',
                    verification=dict(kind='accessibility-state', condition='audio-menu-visible', passed=True, pid=42),
                    before='before.json', after='after.json')
        values = {r['event']: r['value'] for r in action_rows(case, scenario)}
        self.assertEqual(values['ui_menu_audio_dispatch_latency'], 3)
        self.assertEqual(values['ui_menu_audio_settle_latency'], 10)
        self.assertEqual(values['ui_menu_audio_latency'], 13)
        for stamp in (None, True, 0, 15000000):
            rows = action_rows(dict(case, dispatch_completed_ns=stamp), scenario)
            self.assertEqual([r['event'] for r in rows], ['ui_menu_audio_latency'])

    def test_rss_increase_requires_measured_before_and_after_memory(self):
        scenario = dict(id='menu_media', verification='accessibility-state', condition='media-menu-visible')
        case = dict(player='foxide', platform='linux', profile='native-ui/idle', trial=0, source='case.json',
                    status='verified', pid=42, input_ns=10, completed_ns=20, input_method='accessibility-action',
                    verification=dict(kind='accessibility-state', condition='media-menu-visible', passed=True, pid=42),
                    before='before.json', after='after.json', rss_before_mib=70, rss_mib=80)
        values = {r['event']: r['value'] for r in action_rows(case, scenario)}
        self.assertEqual(values['ui_menu_media_rss_increase'], 10)
        values = {r['event']: r['value'] for r in action_rows(dict(case, rss_before_mib=90), scenario)}
        self.assertEqual(values['ui_menu_media_rss_increase'], 0)
        for before in (None, True, -1, float('nan')):
            rows = action_rows(dict(case, rss_before_mib=before), scenario)
            self.assertNotIn('ui_menu_media_rss_increase', [r['event'] for r in rows])

    def test_default_window_positions_do_not_split_identical_ui_workloads(self):
        from ui_benchmarks import action_cohort
        settings = dict(host='same-host', environment='1920x1080, DPI96, English', runner_sha256='code')
        context = dict(backend='linux-atspi', input_method='accessibility-action', window_bounds=[10,20,810,620])
        one = action_cohort(settings, context, 'menu_media')
        two = action_cohort(settings, dict(context, window_bounds=[100,200,1100,900]), 'menu_media')
        self.assertEqual(one, two)
        self.assertEqual(one['window_policy'], 'application-default')
        self.assertNotEqual(one, action_cohort(settings, dict(context, backend='different'), 'menu_media'))
        self.assertNotEqual(one, action_cohort(settings, context, 'menu_audio'))

    def test_invalid_native_action_stops_its_batch_without_hiding_the_failure(self):
        from unittest.mock import patch
        from run_ui_benchmarks import collect_batch
        scenario = dict(id='menu_media', verification='accessibility-state', condition='media-menu-visible')
        failed = dict(player='foxide', status='unverified', error='No visible owned UI')
        with patch('run_ui_benchmarks.collect', return_value=failed) as collector:
            cases, rows = collect_batch('foxide', 'binary', scenario, ['Media', 'Open'],
                                        Path('unused-test-output'), {}, 'linux', 10, None)
        self.assertEqual(collector.call_count, 1)
        self.assertEqual(cases, [failed])
        self.assertEqual(rows, [])

    def test_verified_native_action_requires_the_entire_batch_before_emitting_rows(self):
        from unittest.mock import patch
        from run_ui_benchmarks import collect_batch
        scenario = dict(id='menu_media', verification='accessibility-state', condition='media-menu-visible')
        good = dict(player='foxide', platform='linux', profile='native-ui/same', trial=0,
                    source='case.json', status='verified', verified=True, pid=42, input_ns=0, completed_ns=100,
                    input_method='accessibility-action', before='before.json', after='after.json',
                    verification=dict(kind='accessibility-state', condition='media-menu-visible', passed=True, pid=42))
        failed = dict(player='foxide', status='unverified', error='Postcondition absent')
        with patch('run_ui_benchmarks.collect', side_effect=[good, failed]) as collector:
            cases, rows = collect_batch('foxide', 'binary', scenario, ['Media', 'Open'],
                                        Path('unused-test-output'), {}, 'linux', 3, None)
        self.assertEqual(len(cases), 2)
        self.assertEqual(rows, [], 'Never publish the successful subset of an invalid batch')
        with patch('run_ui_benchmarks.collect', side_effect=[dict(good, trial=i) for i in range(3)]):
            cases, rows = collect_batch('foxide', 'binary', scenario, ['Media', 'Open'],
                                        Path('unused-test-output'), {}, 'linux', 3, None)
        self.assertEqual(len(rows), 3)
        self.assertEqual([r['trial'] for r in rows], [0, 1, 2])

    def test_ui_quality_gate_needs_two_players_in_the_same_verified_cohort(self):
        from ui_benchmarks import has_comparable_ui_measurements
        rows = [dict(player=p, platform='linux', profile='native-ui/shared', trial=i, event='ui_menu_media_latency',
                     value=1, verified=True, source='case.json') for p in ('foxide', 'vlc') for i in range(3)]
        self.assertTrue(has_comparable_ui_measurements(rows))
        self.assertFalse(has_comparable_ui_measurements(rows[:3]))
        self.assertFalse(has_comparable_ui_measurements([dict(r, profile=r['player']) for r in rows]))
        self.assertFalse(has_comparable_ui_measurements([dict(r, verified=False) if r['player']=='vlc' else r for r in rows]))

    def test_vlc_launch_uses_only_options_present_on_the_selected_platform(self):
        from unittest.mock import patch
        from run_ui_benchmarks import command_for
        with patch('run_ui_benchmarks.platform.system', return_value='Linux'):
            linux = command_for('vlc', 'vlc')
        self.assertIn('--no-qt-privacy-ask', linux)
        self.assertNotIn('--no-qt-updates-notif', linux, 'Updater options depend on UPDATE_CHECK builds')
        with patch('run_ui_benchmarks.platform.system', return_value='Darwin'):
            mac = command_for('vlc', 'vlc')
        self.assertIn('--ignore-config', mac)
        self.assertNotIn('--no-qt-privacy-ask', mac, 'Native macOS VLC has no Qt interface')

    def test_incomplete_resource_measurements_do_not_rank_a_successful_subset(self):
        from unittest.mock import patch
        from run_ui_benchmarks import collect_batch
        scenario = dict(id='menu_media', verification='accessibility-state', condition='media-menu-visible')
        cases = [dict(player='foxide',platform='linux',profile='native-ui/shared',trial=i,source='case.json',
                      status='verified',verified=True,pid=42,input_ns=0,completed_ns=100,input_method='native-input',
                      before='before.json',after='after.json',cpu_time_ms=1,
                      verification=dict(kind='accessibility-state',condition='media-menu-visible',passed=True,pid=42)) for i in range(3)]
        del cases[1]['cpu_time_ms']
        with patch('run_ui_benchmarks.collect',side_effect=cases):
            observed, rows = collect_batch('foxide','binary',scenario,['Media','Open'],Path('unused-test-output'),{},'linux',3,None)
        self.assertEqual(len(observed),3)
        self.assertEqual([r['event'] for r in rows],['ui_menu_media_latency']*3)

    def test_portal_dialog_requires_a_broker_owner_adapter_only_on_linux(self):
        import json
        from run_ui_benchmarks import plan
        catalog=json.loads((Path(__file__).resolve().parents[1]/'benchmarks/ui-events.json').read_text())
        statuses={s:next(r['status'] for r in plan(catalog,s) if r['player']=='foxide' and r['action']=='open_dialog') for s in ('linux','windows')}
        self.assertEqual(statuses['linux'],'adapter-required')
        self.assertEqual(statuses['windows'],'ready-for-interactive-validation')
