import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from medal_events import scoreboards
from build_medal_report import markdown

class MedalGapReportTests(unittest.TestCase):
    def test_universal_features_leave_the_contest_but_unknown_support_stays(self):
        players = [dict(id='a', name='A', platforms=['windows'], media_types=['video']),
                   dict(id='b', name='B', platforms=['windows'], media_types=['video']),
                   dict(id='audio', name='Audio', platforms=['windows'], media_types=['audio'])]
        features = [dict(id='shared', name='Shared', media_types=['video']),
                    dict(id='unknown', name='Unknown', media_types=['video']),
                    dict(id='broad', name='Broad', media_types=['video','audio'])]
        claims = [dict(player=p, feature=f, status='supported', sources=['https://example.org/docs'])
                  for p in ['a','b'] for f in ['shared','broad']]
        claims += [dict(player='a', feature='unknown', status='supported', sources=['https://example.org/docs'])]
        catalog = dict(players=players, features=features, claims=claims, research_date='2026-11-16')
        board = scoreboards(catalog, {'events': []}, [])['windows']
        self.assertEqual([f['id'] for f in board['features']], ['unknown','broad'])
        self.assertEqual([f['id'] for f in board['omitted_universal_features']], ['shared'])
        self.assertEqual(board['players'][0]['feature_gold'], 2)
        self.assertEqual(board['players'][1]['feature_gold'], 1)
        self.assertEqual(len(catalog['features']), 3, 'Source evidence must survive report filtering')
