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

    def test_linux_adapter_dispatches_registration_events_before_matching_owned_apps(self):
        from types import SimpleNamespace
        from unittest.mock import patch
        from ui_drivers import LinuxDriver
        apps, dispatched = [], []
        app = SimpleNamespace(get_process_id=lambda: 42)
        def iteration(blocking):
            dispatched.append(blocking)
            apps.append(app)
        context = SimpleNamespace(pending=lambda: not apps, iteration=iteration)
        glib = SimpleNamespace(MainContext=SimpleNamespace(default=lambda: context))
        atspi = SimpleNamespace(Registry=SimpleNamespace(getDesktop=lambda index: apps))
        with patch.dict(sys.modules, {'pyatspi': atspi, 'gi.repository': SimpleNamespace(GLib=glib)}):
            driver = LinuxDriver(42)
        self.assertIs(driver.app, app)
        self.assertEqual(dispatched, [False], 'Native accessibility dispatch must be nonblocking')

    def test_linux_adapter_traverses_nonvisual_application_roots(self):
        from types import SimpleNamespace
        from ui_drivers import LinuxDriver
        focused = []
        bounds = SimpleNamespace(x=10, y=20, width=800, height=600)
        component = SimpleNamespace(getExtents=lambda coords: bounds, grabFocus=lambda: focused.append(True))
        window = SimpleNamespace(name='Player', childCount=0, getState=lambda: SimpleNamespace(contains=lambda flag: True),
                                 queryComponent=lambda: component, getRoleName=lambda: 'frame', get_process_id=lambda: 42)
        def no_component():
            raise NotImplementedError('Application roots do not expose Component')
        app = SimpleNamespace(childCount=1, getChildAtIndex=lambda i: window, queryComponent=no_component,
                              getState=lambda: SimpleNamespace(contains=lambda flag: True))
        driver = LinuxDriver.__new__(LinuxDriver)
        driver.app, driver.pid = app, 42
        driver.context = SimpleNamespace(pending=lambda: False)
        driver.atspi = SimpleNamespace(DESKTOP_COORDS=0, STATE_SHOWING=1)
        result = driver.prepare()
        self.assertEqual(result['window_bounds'], [10, 20, 810, 620])
        self.assertEqual(focused, [True])

    def test_linux_input_clicks_only_positive_visible_owned_accessibility_bounds(self):
        from types import SimpleNamespace
        from ui_drivers import LinuxDriver
        clicks = []
        driver = LinuxDriver.__new__(LinuxDriver)
        driver.pid = 42
        driver.atspi = SimpleNamespace(Registry=SimpleNamespace(generateMouseEvent=lambda *args: clicks.append(args)))
        target = Node(None, 'Media', 'Text', 42, [10,20,110,60], True)
        driver.activate(target)
        self.assertEqual(clicks, [(60,40,'b1c')])
        self.assertEqual(driver.method, 'native-input')
        for invalid in [Node(None,'Media','Text',99,[10,20,110,60],True),
                        Node(None,'Media','Text',42,[10,20,110,60],False),
                        Node(None,'Media','Text',42,[10,20,10,60],True)]:
            with self.assertRaises(RuntimeError):
                driver.activate(invalid)
        self.assertEqual(len(clicks), 1)

    def test_linux_hidden_menu_items_with_zero_area_are_not_visible_evidence(self):
        from types import SimpleNamespace
        from ui_drivers import LinuxDriver
        bounds = SimpleNamespace(x=0,y=0,width=0,height=0)
        item = SimpleNamespace(name='File...', childCount=0, getRoleName=lambda:'menu item', get_process_id=lambda:42,
                               getState=lambda:SimpleNamespace(contains=lambda state:True),
                               queryComponent=lambda:SimpleNamespace(getExtents=lambda coords:bounds))
        driver = LinuxDriver.__new__(LinuxDriver)
        driver.pid, driver.app = 42, item
        driver.context = SimpleNamespace(pending=lambda:False)
        driver.atspi = SimpleNamespace(DESKTOP_COORDS=0,STATE_SHOWING=1)
        self.assertEqual(driver.observe('File...', ['MenuItem']), [])
