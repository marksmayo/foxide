import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from event_benchmarks import measurement_rows, playback_verified, summarize_samples


class EventBenchmarkTests(unittest.TestCase):
    def test_native_window_evidence_is_separate_from_software_decode(self):
        events = [{'kind': 'first-playback-restart-proxy', 'video': '1'},
                  *[{'kind': 'stats', 'position': p, 'hwdec': 'd3d11va'} for p in (0, 2)]]
        self.assertFalse(playback_verified('foxide', 'avc1080', events, []))
        self.assertTrue(playback_verified('foxide', 'avc1080', events, [], presentation='native-window'))
        self.assertFalse(playback_verified('foxide', 'avc1080', events[:2], [], presentation='native-window'))
        requested = [dict(e, hwdec='auto-safe') if e['kind'] == 'stats' else e for e in events]
        self.assertFalse(playback_verified('foxide', 'avc1080', requested, [], presentation='native-window'))
        from run_event_benchmarks import command_for
        args = command_for('foxide', 'foxide-desktop.exe', 'avc1080', Path('movie.mp4'), None, 0, '', presentation='native-window')
        self.assertIn('--benchmark-script', args)
        self.assertNotIn('--headless', args)
        self.assertTrue(any('--vo=null' in a for a in command_for('mpv', 'mpv', 'avc1080', Path('movie.mp4'), None, 0, '')))

    def test_unverified_playback_keeps_diagnostics_but_cannot_earn_a_medal(self):
        case = dict(player='plex', scenario='avc1080', platform='windows', profile='host', trial=0,
                    verified=False, source='case.json', metrics={'cpu_avc1080': 0.0, 'rss_avc1080': 20})
        rows = measurement_rows(case)
        self.assertEqual(len(rows), 2)
        self.assertTrue(all(r['verified'] is False for r in rows))
        self.assertEqual(rows[0]['value'], 0.0)

    def test_playback_requires_advancing_engine_clock_or_verified_paused_state(self):
        events = [{'kind': 'first-playback-restart-proxy', 'video': '1'}, {'kind': 'stats', 'position': 0, 'hwdec': 'no'}]
        self.assertFalse(playback_verified('mpv', 'avc1080', events, []))
        events.append({'kind': 'stats', 'position': 2, 'hwdec': 'no'})
        self.assertTrue(playback_verified('mpv', 'avc1080', events, []))
        self.assertFalse(playback_verified('plex', 'avc1080', events, []))
        self.assertTrue(playback_verified('foxide', 'paused', [dict(e, paused=True) for e in events], []))
        self.assertFalse(playback_verified('mpv', 'avc1080', [{'kind': 'end-file', 'reason': 'error'}], []))
        states = [{'state': 'playing', 'time': 0, 'stats': {'decodedvideo': 1}},
                  {'state': 'playing', 'time': 2, 'stats': {'decodedvideo': 20}}]
        self.assertTrue(playback_verified('vlc', 'avc1080', [], states))
        self.assertFalse(playback_verified('vlc', 'avc1080', [], states[:1]))

    def test_audio_only_fallback_and_wrong_speed_cannot_win_video_workloads(self):
        stats = [{'kind': 'stats', 'position': n, 'hwdec': 'no', 'speed': 1} for n in [0, 2]]
        self.assertFalse(playback_verified('mpv', 'avc1080', [{'kind': 'first-playback-restart-proxy', 'video': 'no'}, *stats], []))
        video = [{'kind': 'first-playback-restart-proxy', 'video': '1'}, *stats]
        self.assertFalse(playback_verified('mpv', 'speed2x', video, []))
        self.assertFalse(playback_verified('mpv', 'avc1080', [*video, {'kind': 'collector-error'}], []))
        self.assertTrue(playback_verified('mpv', 'avc1080', [{'kind': 'stats'}, *video], []))

    def test_steady_metrics_start_after_playback_readiness_and_warmup(self):
        samples = [dict(elapsed=2, rss=90*1048576, cpu=0),
                   dict(elapsed=6, rss=60*1048576, cpu=50),
                   dict(elapsed=8, rss=30*1048576, cpu=20)]
        self.assertEqual(summarize_samples(samples, ready_elapsed=5, warmup=2),
                         {'cpu': 20, 'rss_mib': 30, 'rss_peak_mib': 90})
        self.assertIsNone(summarize_samples([], ready_elapsed=0, warmup=0)['cpu'])


if __name__ == '__main__':
    unittest.main()
