import copy
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('report_builder', ROOT / 'scripts/build_comparison_report.py')
report = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(report)


def dataset():
    d = {'settings': {'duration': 20, 'warmup': 5}, 'media': {'fixtures': [{'id': 'A'}, {'id': 'B'}]},
         'catalog': {'players': [{'id': 'one'}, {'id': 'two'}, {'id': 'unverified'}]}, 'trials': []}
    for pid, value in [('one', 100), ('two', 200), ('unverified', 0)]:
        for number in range(3):
            d['trials'].append({'id': f'{pid}-{number}', 'player': pid, 'fixture': 'A', 'prime': False,
                'status': 'sampled', 'duration_seconds': 20, 'warmup_seconds': 5,
                'playback_evidence': {'verified': pid != 'unverified'},
                'rss_steady_bytes': value, 'cpu_steady_percent': value,
                'gpu_dedicated_steady_bytes': 0})
    return d


class ComparisonReportTests(unittest.TestCase):
    def test_unverified_low_resource_process_cannot_win(self):
        result = report.summarize(dataset())['A']
        self.assertNotIn('unverified', result['players'])
        ranks = result['rankings']['cpu']
        self.assertEqual([r['player'] for r in ranks], ['one', 'two'])
        self.assertEqual([r['medal'] for r in ranks], ['gold', 'silver'])

    def test_no_solo_gold_or_cross_profile_comparison(self):
        d = dataset()
        d['trials'] = [t for t in d['trials'] if t['player'] == 'one']
        result = report.summarize(d)
        self.assertEqual(result['A']['rankings']['cpu'], [])
        self.assertEqual(result['B']['rankings']['cpu'], [])

    def test_zero_is_data_missing_is_not_and_ties_share_gold(self):
        result = report.summarize(dataset())['A']
        self.assertEqual([r['medal'] for r in result['rankings']['gpu']], ['gold', 'gold'])
        self.assertEqual(result['rankings']['power'], [])
        self.assertNotIn('power', result['players']['one']['metrics'])

    def test_incomplete_metric_and_mismatched_trial_cannot_medal(self):
        d = dataset()
        d['trials'][0]['cpu_steady_percent'] = None
        d['trials'][3]['duration_seconds'] = 8
        result = report.summarize(d)['A']
        self.assertEqual(result['rankings']['cpu'], [])
        self.assertFalse(result['players']['one']['metrics']['cpu']['eligible'])

    def test_priming_and_early_exit_never_count(self):
        d = dataset()
        for t in d['trials']:
            if t['player'] == 'one':
                t['prime'] = True
            elif t['player'] == 'two':
                t['status'] = 'early-exit'
        self.assertEqual(report.summarize(d)['A']['players'], {})


if __name__ == '__main__':
    unittest.main()
