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
        self.assertIn('**12.5** %', text)
        self.assertIn('same-cohort', text)
        self.assertIn('| Feature | ? |', text)
        self.assertIn('Download optional HTML', text)

    def test_virtuoso_layout_deduplicates_global_feature_golds_and_separates_fields(self):
        player = dict(id='foxide', name='Foxide', feature_gold=1, performance_gold=1,
                      performance_silver=0, performance_bronze=0)
        board = dict(players=[player], runtime={'foxide': dict(status='Verified playback')},
                     features=[dict(id='f', name='Feature', applicable=['foxide'], awards={'foxide': 'gold'}, claims=[])],
                     performance=[dict(id='cpu', name='CPU', profile='native-default-window/avc1080/123', unit='%',
                                       rankings=[dict(player='foxide', value=12.5, medal='gold', trials=3)])])
        text = markdown({'windows': board, 'linux': board}, 'docs/scoreboards')
        self.assertIn('**Overall**', text)
        self.assertIn('**Performance**', text)
        self.assertIn('| Player | 🥇 | 🥈 | 🥉 | Total |', text)
        self.assertIn('| Foxide | 3 | 0 | 0 | 3 |', text, 'One feature gold plus two platform performance golds')
        self.assertIn('scenarios.svg', text)
        self.assertIn('Windowed playback', text)

    def test_failed_field_keeps_date_and_host_without_medals(self):
        from build_medal_report import fields_for, scenario_chart
        board = dict(players=[], performance=[])
        sessions = [dict(session=dict(system="windows", utc="2026-11-15T00:00:00Z", host="failed host", presentation="native-window"), cases=[])]
        field = fields_for("windows", board, sessions)[0]
        self.assertEqual(field["host"], "failed host")
        self.assertEqual(field["utc"], "2026-11-15T00:00:00Z")
        self.assertIn("Windowed", field["label"])
        self.assertIn("Pending measurement", scenario_chart(field))

    def test_diagnostic_frontend_profiles_do_not_create_empty_score_fields(self):
        from build_medal_report import fields_for
        board = dict(players=[], performance=[dict(profile="frontend-default-diagnostic/avc1080/x", rankings=[])])
        field = fields_for("windows", board)[0]
        self.assertEqual(field["mode"], "pending")

    def test_archived_order_is_not_mislabeled_as_randomized(self):
        board = dict(players=[], features=[], runtime={}, performance=[])
        session = dict(session=dict(system="windows", utc="2026-11-14T00:00:00Z", host="archived", duration=8, warmup=2, run_id="42"), cases=[])
        text = markdown({"windows": board}, ".", [session])
        self.assertIn("original collection order", text)
        self.assertNotIn("randomized order", text)
        self.assertIn("Hosted", text)
        session["session"]["order_seed"] = 12
        text = markdown({"windows": board}, ".", [session])
        self.assertIn("randomized order (seed 12)", text)

    def test_decoder_table_reports_actual_paths_without_inventing_vlc_state(self):
        from build_medal_report import decoder_table
        info = dict(system="windows")
        field = dict(info=info, names={"foxide": "Foxide", "vlc": "VLC"})
        session = dict(session=info, cases=[dict(player="foxide", scenario="avc1080", playback_events=[dict(kind="stats", hwdec="d3d11va-copy")]), dict(player="vlc", scenario="avc1080"), dict(player="foxide", scenario="flac")])
        text = decoder_table(field, [dict(session={"system": "linux"}, cases=[]), session])
        self.assertIn("| 1080p H.264 | d3d11va-copy | Not reported |", text)
        self.assertIn("not zero-copy", text)
        self.assertEqual(decoder_table(field, []), "")
