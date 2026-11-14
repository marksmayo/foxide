from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from build_medal_report import markdown


class ReadmeScoresTests(unittest.TestCase):
    def test_github_readme_contains_numeric_podiums_and_unknown_feature_cells(self):
        board = dict(players=[dict(id='a', name='Alpha', feature_gold=1, performance_gold=1,
                                   performance_silver=0, performance_bronze=0)],
                     runtime={'a': dict(status='Verified playback')},
                     features=[dict(id='f', name='Feature', applicable=['a'], awards={}, claims=[])],
                     performance=[dict(name='CPU', profile='same-cohort', unit='%',
                                       rankings=[dict(player='a', value=12.5, medal='gold', trials=3)])])
        text = markdown({'windows': board}, 'docs/scoreboards')
        self.assertIn('Every measured metric', text)
        self.assertIn('🥇 **12.5**', text)
        self.assertIn('same-cohort', text)
        self.assertIn('| Feature | ? |', text)
        self.assertIn('Download optional HTML', text)
