"""Acceptance tests for the sourced desktop feature medal events."""
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from medal_events import scoreboards


class DesktopFeatureEventsTests(unittest.TestCase):
    def boards(self):
        catalog = json.loads((ROOT / 'benchmarks/features.json').read_text('utf-8'))
        return scoreboards(catalog, {'events': []}, [])

    def test_chromecast_support_awards_every_documented_desktop_player_gold(self):
        events = {event['id']: event for event in self.boards()['windows']['features']}
        self.assertEqual(events['chromecast_output']['awards']['vlc'], 'gold')
        self.assertEqual(events['chromecast_output']['awards']['smplayer'], 'gold')
        self.assertNotIn('foxide', events['chromecast_output']['awards'])

    def test_receiver_and_sender_medals_do_not_conflate_airplay_capabilities(self):
        events = {event['id']: event for event in self.boards()['linux']['features']}
        self.assertEqual(events['airplay_audio_receiver']['awards']['kodi'], 'gold')
        self.assertNotIn('kodi', events['airplay_video_output']['awards'])

    def test_mac_only_app_claims_never_create_windows_feature_medals(self):
        windows = self.boards()['windows']
        self.assertNotIn('infuse', {player['id'] for player in windows['players']})
        self.assertTrue(all('infuse' not in feature['awards'] for feature in windows['features']))

    def test_new_features_keep_a_source_and_an_explicit_foxide_status(self):
        catalog = json.loads((ROOT / 'benchmarks/features.json').read_text('utf-8'))
        features = [f for f in catalog['features'] if f.get('research_batch') == 'desktop-2026-11-16']
        self.assertGreaterEqual(len(features), 25)
        for feature in features:
            claims = [c for c in catalog['claims'] if c['feature'] == feature['id']]
            self.assertTrue(any(c['status'] == 'supported' and c.get('sources') for c in claims), feature['id'])
            self.assertTrue(any(c['player'] == 'foxide' and c['status'] in ('supported', 'planned', 'unknown', 'unsupported') for c in claims), feature['id'])
