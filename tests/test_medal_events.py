import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from medal_events import feature_medals, performance_medals, scoreboards


class MedalEventsTests(unittest.TestCase):
    def test_ui_verification_is_not_mislabeled_as_verified_playback(self):
        catalog = dict(features=[], claims=[], research_date='2026-11-15',
                       players=[dict(id='one', name='One', platforms=['windows'])])
        sessions = [dict(session={'system': 'windows'}, inventory=[], cases=[
            dict(player='one', scope='native-ui', verified=True, status='verified')])]
        status = scoreboards(catalog, {'events': []}, [], sessions)['windows']['runtime']['one']['status']
        self.assertIn('Verified UI actions 1/1', status)
        self.assertNotIn('playback', status)

    def test_every_documented_supporter_gets_gold(self):
        claims = [
            {'player': 'one', 'status': 'supported', 'sources': ['https://example.org/docs']},
            {'player': 'two', 'status': 'supported', 'sources': ['https://example.org/docs']},
            {'player': 'three', 'status': 'unknown', 'sources': []},
        ]
        self.assertEqual(feature_medals(claims), {'one': 'gold', 'two': 'gold'})

    def test_performance_requires_verified_repeated_matching_trials(self):
        rows = []
        for player, value in [('one', 10), ('two', 20), ('failed', 0)]:
            for trial in range(3):
                rows.append(dict(player=player, value=value, event='cpu', platform='windows',
                                 profile='same-host-and-fixture', trial=trial,
                                 verified=player != 'failed', source='run.json'))
        rows.append(dict(player='mac-only', value=0, event='cpu', platform='macos',
                         profile='same-host-and-fixture', trial=0, verified=True, source='run.json'))
        ranks = performance_medals(rows, 'cpu', 'windows', 'same-host-and-fixture')
        self.assertEqual([(r['player'], r['medal']) for r in ranks], [('one', 'gold'), ('two', 'silver')])

    def test_invalid_ranking_configuration_is_rejected(self):
        for direction, resolution in [('fastest', 0), ('min', -1), ('min', float('nan'))]:
            with self.subTest(direction=direction, resolution=resolution), self.assertRaises(ValueError):
                performance_medals([], 'cpu', 'windows', 'profile', direction, resolution)

    def test_missing_trials_and_bad_samples_cannot_sneak_onto_podium(self):
        rows = []
        for player, values in [('a', [0, 0, 0]), ('b', [0, 0, 0]), ('c', [2, 2, 2]),
                               ('short', [0]), ('null', [None]*3), ('nan', [float('nan')]*3),
                               ('bool', [True]*3), ('negative', [-1]*3)]:
            rows.extend(dict(player=player, value=v, trial=i, event='cpu', platform='linux',
                             profile='p', verified=True, source='run.json') for i, v in enumerate(values))
        duplicate = [dict(r, player='duplicate', trial=0) for r in rows[:3]]
        ranks = performance_medals(rows + duplicate, 'cpu', 'linux', 'p')
        self.assertEqual([(r['player'], r['medal']) for r in ranks], [('a', 'gold'), ('b', 'gold'), ('c', 'bronze')])
        self.assertEqual(performance_medals(rows[:3], 'cpu', 'linux', 'p'), [])
        high = performance_medals(rows, 'cpu', 'linux', 'p', direction='max', resolution=1)
        self.assertEqual(high[0]['player'], 'c')

    def test_claims_need_real_documentation_links(self):
        self.assertEqual(feature_medals([{'player': 'fake', 'status': 'supported', 'sources': ['javascript:alert(1)']}]), {})

    def test_platform_scoreboards_separate_features_from_measured_events(self):
        catalog = {'features': [{'id': 'speed', 'name': 'Speed'}],
                   'players': [{'id': 'one', 'name': 'One', 'platforms': ['windows']},
                               {'id': 'two', 'name': 'Two', 'platforms': ['macos']}],
                   'claims': [{'player': 'one', 'feature': 'speed', 'status': 'supported', 'sources': ['https://example.org/docs']}],
                   'research_date': '2026-11-14'}
        boards = scoreboards(catalog, {'events': [{'id': 'cpu', 'name': 'CPU', 'direction': 'min'}]}, [])
        self.assertEqual(boards['windows']['players'][0]['feature_gold'], 1)
        self.assertEqual(boards['macos']['players'][0]['feature_gold'], 0)
        self.assertEqual(boards['windows']['players'][0]['performance_gold'], 0)
        self.assertEqual(boards['windows']['performance'], [])
        self.assertEqual(boards['linux']['players'], [])

    def test_runtime_table_explains_why_frontend_diagnostics_have_no_medals(self):
        catalog = dict(features=[], claims=[], research_date='2026-11-14',
                       players=[dict(id='one', name='One', platforms=['windows']),
                                dict(id='two', name='Two', platforms=['windows'])])
        sessions = [dict(session={'system': 'windows'}, inventory=[dict(player='two', status='installed-adapter-needed')],
                         cases=[dict(player='one', verified=True, status='verified-playback'),
                                dict(player='two', verified=False, status='sampled-playback-unverified')])]
        result = scoreboards(catalog, {'events': []}, [], sessions)
        self.assertIn('1/1', result['windows']['runtime']['one']['status'])
        self.assertIn('Unverified', result['windows']['runtime']['two']['status'])

    def test_audio_only_entrants_have_inapplicable_video_events(self):
        catalog = dict(features=[dict(id='hdr', name='HDR', media_types=['video'])],
                       claims=[dict(player='audio', feature='hdr', status='supported', sources=['https://example.org/docs'])],
                       research_date='2026-11-14', players=[dict(id='audio', name='Audio', platforms=['windows'], media_types=['audio'])])
        event = scoreboards(catalog, {'events': []}, [])['windows']['features'][0]
        self.assertEqual(event['awards'], {})
        self.assertEqual(event['applicable'], [])

    def test_medians_resolution_ties_and_fourth_place_have_exact_competition_ranks(self):
        rows = []
        for player, values in [('a', [0.4, 0.4, 900]), ('b', [0.49]*3), ('c', [2]*3), ('d', [3]*3)]:
            rows.extend(dict(player=player, value=value, trial=i, event='cpu', platform='linux', profile='p', verified=True, source='run.json') for i, value in enumerate(values))
        ranks = performance_medals(rows, 'cpu', 'linux', 'p', resolution=1)
        self.assertEqual([(r['player'], r['value'], r['rank'], r['medal'], r['trials']) for r in ranks],
                         [('a', 0.4, 1, 'gold', 3), ('b', 0.49, 1, 'gold', 3), ('c', 2, 3, 'bronze', 3), ('d', 3, 4, None, 3)])
        maximum = performance_medals(rows, 'cpu', 'linux', 'p', direction='max')
        self.assertEqual([(r['player'], r['rank'], r['medal']) for r in maximum],
                         [('d', 1, 'gold'), ('c', 2, 'silver'), ('b', 3, 'bronze'), ('a', 4, None)])
        self.assertTrue(all('score' not in r for r in ranks))

    def test_one_invalid_or_mismatched_trial_never_creates_a_medal(self):
        base = [dict(player=p, value=v, trial=i, event='cpu', platform='linux', profile='p', verified=True, source='run.json')
                for p, v in [('a', 10), ('b', 20)] for i in range(3)]
        bad_changes = [dict(verified=1), dict(verified=False), dict(source=''), dict(value=float('inf')),
                       dict(value='0'), dict(value=True), dict(value=-0.1), dict(value=None),
                       dict(event='other'), dict(platform='windows'), dict(profile='other'), dict(trial=0)]
        for change in bad_changes:
            candidate = [dict(row, player='invalid', value=0) for row in base[:3]]
            candidate[1].update(change)
            with self.subTest(change=change):
                ranks = performance_medals(base + candidate, 'cpu', 'linux', 'p')
                self.assertEqual([r['player'] for r in ranks], ['a', 'b'])
        for resolution in (float('inf'), -0.001):
            with self.assertRaises(ValueError):
                performance_medals(base, 'cpu', 'linux', 'p', resolution=resolution)

    def test_feature_gold_requires_every_source_and_explicit_support(self):
        claims = [dict(player='valid', status='supported', sources=['http://example.org/a', 'https://example.org/b']),
                  dict(player='mixed', status='supported', sources=['https://example.org', 'file:///local']),
                  dict(player='hostless', status='supported', sources=['https:///missing-host']),
                  dict(player='empty', status='supported', sources=[]),
                  dict(player='no', status='unsupported', sources=['https://example.org'])]
        self.assertEqual(feature_medals(claims), {'valid': 'gold'})
        self.assertEqual(feature_medals(claims[:1], valid_source=lambda url: url.endswith('/a')), {})


if __name__ == '__main__':
    unittest.main()
