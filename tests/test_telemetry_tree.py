from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from telemetry import descendants, cpu_seconds


class TreeTests(unittest.TestCase):
    def test_deep_tree_cycles_and_unrelated_processes_preserve_membership(self):
        parents = {i: i - 1 for i in range(1, 1500)}
        parents.update({1: 0, 9000: 9001, 9001: 9000})
        self.assertEqual(descendants(parents, 1), set(range(1, 1500)))
        self.assertEqual(descendants(parents, 9000), {9000, 9001})
        self.assertEqual(descendants(parents, 8000), set())

    def test_cpu_time_supports_days_and_fractional_seconds(self):
        self.assertEqual(cpu_seconds('2-01:02:03.5'), 176523.5)
        self.assertEqual(cpu_seconds('00:02.5'), 2.5)
