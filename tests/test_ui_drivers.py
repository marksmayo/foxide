"""Portable locator contract tests; no operating-system automation is invoked."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from ui_drivers import Driver, Node


class FixtureDriver(Driver):
    pid = 42

    def __init__(self, nodes):
        self.controls = nodes

    def nodes(self):
        return self.controls


class LocatorTests(unittest.TestCase):
    def test_lookup_matches_owned_visible_controls_and_rejects_ambiguity(self):
        owned = Node(None, '&Open…', 'MenuItem', 42, [1, 2, 30, 40], True)
        driver = FixtureDriver([
            Node(None, 'Open...', 'MenuItem', 99, [], True),
            Node(None, 'Open...', 'MenuItem', 42, [], False),
            Node(None, 'Open...', 'Window', 42, [], True), owned,
        ])
        self.assertIs(driver.find('open...', ['MenuItem']), owned)
        self.assertEqual(driver.observe('open...', ['MenuItem']), [owned.evidence()])
        driver.controls.append(Node(None, '_Open...', 'MenuItem', 42, [], True))
        with self.assertRaises(RuntimeError):
            driver.find('open...', ['MenuItem'])
        self.assertEqual(len(driver.observe('open...', ['MenuItem'])), 2)
        with self.assertRaises(RuntimeError):
            driver.find('Missing', ['MenuItem'])
