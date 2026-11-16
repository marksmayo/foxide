import os
from pathlib import Path
import subprocess
import unittest

@unittest.skipUnless(os.environ.get("FOXIDE_DESKTOP_TEST_BINARY"), "requires a desktop build")
class AngleRuntimeTests(unittest.TestCase):
    def test_invalid_explicit_runtime_fails_before_opening_a_window(self):
        binary = Path(os.environ['FOXIDE_DESKTOP_TEST_BINARY']).resolve()
        for directory in ('relative-runtime', str(binary.parent / 'missing-angle-runtime')):
            with self.subTest(directory=directory):
                env = dict(os.environ, FOXIDE_ANGLE_RUNTIME=directory)
                result = subprocess.run([str(binary), '--quit-after', '1'], env=env, capture_output=True, text=True, timeout=15)
                self.assertNotEqual(result.returncode, 0)
                self.assertTrue(result.stderr.strip())

if __name__ == '__main__':
    unittest.main()
