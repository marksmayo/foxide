import importlib.util
import os
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('hosted', ROOT / 'scripts/hosted-benchmarks.py')
hosted = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(hosted)


class HostedDiscoveryTests(unittest.TestCase):
    def test_winget_user_and_machine_scopes_are_both_supported(self):
        with patch.dict(os.environ, {'ProgramFiles': 'C:/Program Files', 'LOCALAPPDATA': 'C:/Users/runner/AppData/Local'}):
            paths = [p.as_posix() for p in hosted.windows_mpv_candidates()]
        self.assertIn('C:/Program Files/WinGet/Links/mpv.exe', paths)
        self.assertIn('C:/Users/runner/AppData/Local/Microsoft/WinGet/Links/mpv.exe', paths)

    def test_portable_mpv_takes_precedence_over_path(self):
        expected = hosted.ROOT / 'tools/mpv' / ('mpv.exe' if os.name == 'nt' else 'mpv')
        with patch.object(Path, 'is_file', lambda p: p == expected), patch.object(hosted.shutil, 'which', return_value='/unrelated/mpv'):
            self.assertEqual(hosted.locate('mpv'), expected)


if __name__ == '__main__':
    unittest.main()
