import os
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from compare_render_modes import probe

class RendererComparisonTests(unittest.TestCase):
    def invoke(self, log, code=0):
        with patch('compare_render_modes.subprocess.run', return_value=subprocess.CompletedProcess([], code, log, '')) as run, patch.object(Path, 'write_text'):
            probe(Path('probe.exe'), Path('fixture.mp4'), 'angle', Path('unused'), 'dxva2')
            return run.call_args.kwargs['env']

    def test_only_successful_hardware_compositor_runs_pass(self):
        log = 'GL_VERSION=OpenGL ES 3.0 GL_RENDERER=ANGLE Intel D3D11\ndecoder=dxva2\nupright=150, inverted=0, unique_composited_frames=150'
        self.invoke(log)
        for bad in (log.replace('Intel D3D11', 'Microsoft Basic Render Driver'), log.replace('3.0', '2.0'), log.replace('decoder=dxva2', 'decoder=d3d11va-copy'), 'running 0 tests'):
            with self.subTest(log=bad), self.assertRaises(RuntimeError):
                self.invoke(bad)
        with self.assertRaises(RuntimeError):
            self.invoke(log, code=1)

    def test_default_probe_removes_angle_override(self):
        result = subprocess.CompletedProcess([], 0, 'unique_composited_frames=150', '')
        with patch.dict(os.environ, {'FOXIDE_ANGLE_RUNTIME': 'explicit-runtime'}), patch('compare_render_modes.subprocess.run', return_value=result) as run, patch.object(Path, 'write_text'):
            probe(Path('probe.exe'), Path('fixture.mp4'), 'default', Path('unused'), 'dxva2')
            self.assertNotIn('FOXIDE_ANGLE_RUNTIME', run.call_args.kwargs['env'])
            self.assertEqual(os.environ['FOXIDE_ANGLE_RUNTIME'], 'explicit-runtime')

if __name__ == '__main__':
    unittest.main()
