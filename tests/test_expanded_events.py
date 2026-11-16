import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from medal_events import scoreboards
from run_ui_benchmarks import plan

ROOT = Path(__file__).resolve().parents[1]

class ExpandedEventTests(unittest.TestCase):
    def test_new_feature_events_have_unique_sourced_claims(self):
        catalog = json.loads((ROOT/'benchmarks/features.json').read_text())
        expected = ["dual_subtitles","upnp_browse","upnp_server","controller_navigation","webdav_browse","nfs_browse","trakt_sync","plex_server_client","emby_server_client","jellyfin_server_client","skip_intro","audio_visualization"]
        self.assertTrue(set(expected) <= {f['id'] for f in catalog['features']})
        keys = [(c['player'], c['feature'], tuple(c.get('platforms', []))) for c in catalog['claims']]
        self.assertEqual(len(keys), len(set(keys)))
        for feature in expected:
            supported = [c for c in catalog['claims'] if c['feature'] == feature and c['status'] == 'supported']
            self.assertTrue(supported, feature)
            self.assertTrue(all(c.get('sources') and c.get('checked_utc') for c in supported))

    def test_dual_subtitle_support_is_shared_gold_without_backend_inheritance(self):
        catalog = json.loads((ROOT/'benchmarks/features.json').read_text())
        boards = scoreboards(catalog, {'events': []}, [])
        event = next(f for f in boards['windows']['features'] if f['id'] == 'dual_subtitles')
        self.assertEqual(event['awards'].get('mpv'), 'gold')
        self.assertEqual(event['awards'].get('smplayer'), 'gold')
        self.assertNotIn('foxide', event['awards'])
        self.assertFalse(boards['windows']['performance'])

    def test_new_ui_workloads_are_catalogued_but_cannot_claim_measurements(self):
        catalog = json.loads((ROOT/'benchmarks/ui-events.json').read_text())
        protocol = json.loads((ROOT/'benchmarks/performance-events.json').read_text())
        expected = ["dual_subtitle_select","network_browse_1000","server_library_search_10000","cast_connect","cast_disconnect","subtitle_style_preview","audio_device_switch","keyboard_seek_burst_30","playlist_reorder_1000","window_move_displays","screen_reader_traversal","shutdown_during_4k"]
        actions = {a['id']: a for a in catalog['actions']}
        ids = [e['id'] for e in protocol['events']]
        self.assertEqual(len(ids), len(set(ids)))
        for action in expected:
            self.assertEqual(actions[action]['collector'], 'adapter-required')
            self.assertEqual(len([e for e in ids if e.startswith('ui_' + action + '_')]), 6)
        for system in ('windows', 'macos', 'linux'):
            self.assertFalse(any(r['status'] == 'ready-for-interactive-validation' for r in plan(catalog, system) if r['action'] in expected))
