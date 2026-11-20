"""Public comparison policy must never turn discovery or logs into display metrics."""
from pathlib import Path
import json
import shutil
import sys
import unittest
from unittest.mock import patch
from uuid import uuid4

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
import compare_players as comparison


class ComparisonTests(unittest.TestCase):
    def setUp(self):
        self.root = ROOT / 'results/local' / ('comparison-test-' + uuid4().hex)
        self.root.mkdir(parents=True)
        self.addCleanup(shutil.rmtree, self.root)

    def test_playback_evidence_requires_the_collectors_actual_signal(self):
        for player in ('mpv', 'vlc', 'ffplay', 'foxide', 'wmp-legacy', 'unknown'):
            with self.subTest(player=player):
                self.assertFalse(comparison.playback_evidence(player, '', '', 'A', [])['verified'])
        self.assertTrue(comparison.playback_evidence('mpv', '', '', 'A',
                         [dict(kind='first-playback-restart-proxy')])['verified'])
        self.assertFalse(comparison.playback_evidence('mpv', '', '', 'A',
                          [dict(kind='process-started')])['verified'])
        self.assertTrue(comparison.playback_evidence('vlc', 'codec (h264) started', '', 'A', [])['verified'])
        self.assertFalse(comparison.playback_evidence('vlc', 'codec (aac) started', '', 'A', [])['verified'])
        self.assertTrue(comparison.playback_evidence('vlc', 'codec (flac) started', '', 'D', [])['verified'])
        self.assertFalse(comparison.playback_evidence('ffplay', '0.01 A-V:', '', 'A', [])['verified'])
        evidence = comparison.playback_evidence('ffplay', '2.25 A-V:', '', 'A', [])
        self.assertTrue(evidence['verified'])
        self.assertEqual(evidence['last_clock_seconds'], 2.25)
        self.assertIn('not screen presentation', evidence['method'])
        self.assertIn('D3D11VA', comparison.playback_evidence('vlc', 'Using D3D11VA', '', 'A', [])['hardware'])
        self.assertIn('unverified', comparison.playback_evidence('vlc', 'd3d11va', '', 'A', [])['hardware'])

    def test_resolve_handles_known_folders_and_missing_binaries(self):
        binary = self.root / 'player.exe'
        binary.write_bytes(b'fixture')
        self.assertEqual(comparison.resolve('{ProgramFiles}/player.exe',
                                           {'ProgramFiles': str(self.root)}), binary.resolve())
        with patch.object(comparison.shutil, 'which', return_value=None):
            self.assertIsNone(comparison.resolve(str(self.root / 'absent.exe')))
        with patch.object(comparison.shutil, 'which', return_value=str(binary)):
            self.assertEqual(comparison.resolve('a-command-not-in-the-repo'), binary.resolve())

    def test_inventory_distinguishes_presence_dependency_and_adapter_support(self):
        (self.root / 'benchmarks').mkdir()
        binary = self.root / 'player.exe'
        binary.write_bytes(b'fixture')
        catalog = dict(players=[dict(id=p, paths=[str(binary)]) for p in ('mpv', 'foxide', 'potplayer')])
        catalog['players'].append(dict(id='iina', paths=[], platforms=['macos']))
        path = self.root / 'benchmarks/windows-players.json'
        path.write_text(json.dumps(catalog))
        with patch.object(comparison, 'ROOT', self.root):
            records = {p['id']: p for p in comparison.inventory()['players']}
        self.assertEqual(records['mpv']['status'], 'available')
        self.assertEqual(records['foxide']['status'], 'available')
        self.assertEqual(records['potplayer']['status'], 'adapter-needed')
        self.assertEqual(records['iina']['status'], 'other-platform')
        self.assertEqual(records['mpv']['sha256'], records['potplayer']['sha256'])
        catalog['players'][0]['paths'] = [str(self.root / 'missing.exe')]
        path.write_text(json.dumps(catalog))
        with patch.object(comparison, 'ROOT', self.root):
            records = {p['id']: p for p in comparison.inventory(False)['players']}
        self.assertEqual(records['foxide']['status'], 'blocked-dependency')

    def test_changed_restored_media_is_rejected_before_benchmark_launch(self):
        generated = self.root / 'media/generated/comparison'
        generated.mkdir(parents=True)
        media = generated / 'fixture.mp4'
        media.write_bytes(b'original')
        manifest = dict(fixtures=[dict(path=media.name, sha256=comparison.digest(media))])
        (generated / 'manifest.json').write_text(json.dumps(manifest))
        with patch.object(comparison, 'ROOT', self.root):
            directory, restored = comparison.prepare_media(self.root)
            self.assertEqual(directory, generated)
            self.assertEqual(restored, manifest)
            media.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'hash mismatch'):
                comparison.prepare_media(self.root)

    def test_unvalidated_frontends_cannot_silently_receive_an_adapter(self):
        with self.assertRaisesRegex(ValueError, 'No validated adapter'):
            comparison.command_for(dict(id='unvalidated', executable='player'), 'media.mp4', {}, self.root)

    def test_raw_comparison_outputs_are_restricted_to_ignored_local_storage(self):
        with patch.object(comparison, 'ROOT', self.root):
            accepted = self.root / 'results/local/comparison'
            self.assertEqual(comparison.private_output(accepted), accepted.resolve())
            for path in (self.root / 'results/windows-comparison', self.root.parent / 'public-results'):
                with self.assertRaisesRegex(ValueError, 'results/local'):
                    comparison.private_output(path)
