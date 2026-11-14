"""Native accessibility adapters. No engine IPC, guessed coordinates or focus theft.

These adapters must run on an isolated, logged-in interactive benchmark desktop.
No accessibility/screen-recording permissions are granted or changed here.
"""
from dataclasses import dataclass


def normalized(text):
    return str(text).replace('&', '').replace('_', '').replace('…', '...').strip().casefold()


@dataclass
class Node:
    native: object
    name: str
    role: str
    pid: int
    bounds: list
    visible: bool

    def evidence(self):
        return dict(name=self.name, role=self.role, pid=self.pid, bounds=self.bounds, visible=self.visible)


class Driver:
    method = 'accessibility-action'

    def find(self, name, roles):
        expected = normalized(name)
        matches = [node for node in self.nodes() if node.pid == self.pid and node.visible
                   and node.role in roles and normalized(node.name) == expected]
        if len(matches) != 1:
            raise RuntimeError(f'Expected one visible owned {roles} named {name!r}; found {len(matches)}')
        return matches[0]

    def observe(self, name, roles):
        expected = normalized(name)
        return [node.evidence() for node in self.nodes() if node.pid == self.pid and node.visible
                and node.role in roles and normalized(node.name) == expected]


class WindowsDriver(Driver):
    backend = 'windows-uia'

    def __init__(self, pid):
        from pywinauto import Desktop
        self.pid, self.desktop = pid, Desktop(backend='uia')

    def windows(self):
        return self.desktop.windows(process=self.pid, visible_only=True)

    def prepare(self):
        windows = self.windows()
        if not windows:
            raise RuntimeError('No visible window owned by the new process; do not attach to an existing instance')
        target = max(windows, key=lambda w: w.rectangle().width() * w.rectangle().height())
        if target.element_info.process_id != self.pid:
            raise RuntimeError('Window ownership changed')
        target.set_focus()
        rect = target.rectangle()
        return dict(backend=self.backend, input_method=self.method,
                    window_bounds=[rect.left, rect.top, rect.right, rect.bottom])

    def nodes(self):
        nodes = []
        for window in self.windows():
            for element in [window, *window.descendants()]:
                info = element.element_info
                if info.process_id != self.pid:
                    continue
                rect = element.rectangle()
                nodes.append(Node(element, info.name or '', info.control_type, info.process_id,
                                  [rect.left, rect.top, rect.right, rect.bottom], element.is_visible()))
        return nodes

    def activate(self, node):
        if node.pid != self.pid or not node.visible:
            raise RuntimeError('Refusing an unowned or hidden control')
        # Invoke/ExpandCollapse are real native UI actions, not mouse injection or
        # engine commands. Their latency has its own explicitly labeled cohort.
        try:
            node.native.iface_expand_collapse.Expand()
        except Exception:
            node.native.iface_invoke.Invoke()

    def dismiss(self):
        windows = self.windows()
        if windows:
            window = windows[-1]
            if window.element_info.process_id != self.pid:
                raise RuntimeError('Dialog ownership changed')
            window.set_focus()
            window.type_keys('{ESC}', set_foreground=True)


class LinuxDriver(Driver):
    backend = 'linux-atspi'

    def __init__(self, pid):
        import pyatspi
        self.atspi, self.pid = pyatspi, pid
        self.app = next((app for app in pyatspi.Registry.getDesktop(0) if app.get_process_id() == pid), None)
        if self.app is None:
            raise RuntimeError('Owned application is absent from the AT-SPI desktop bus')

    def nodes(self):
        pending, nodes, budget = [self.app], [], 5000
        roles = {'menu item': 'MenuItem', 'menu': 'MenuItem', 'push button': 'Button', 'frame': 'Window', 'dialog': 'Window'}
        while pending and budget:
            item = pending.pop(); budget -= 1
            try:
                state = item.getState()
                bounds = item.queryComponent().getExtents(self.atspi.DESKTOP_COORDS)
                nodes.append(Node(item, item.name, roles.get(item.getRoleName(), item.getRoleName()),
                                  item.get_process_id(), [bounds.x, bounds.y, bounds.x+bounds.width, bounds.y+bounds.height],
                                  state.contains(self.atspi.STATE_SHOWING)))
                pending.extend(item.getChildAtIndex(i) for i in range(item.childCount))
            except Exception:
                continue
        return nodes

    def prepare(self):
        windows = [node for node in self.nodes() if node.role == 'Window' and node.visible]
        if not windows:
            raise RuntimeError('No visible owned AT-SPI window')
        windows[0].native.queryComponent().grabFocus()
        return dict(backend=self.backend, input_method=self.method, window_bounds=windows[0].bounds)

    def activate(self, node):
        actions = node.native.queryAction()
        for i in range(actions.nActions):
            if actions.getName(i).casefold() in ('click', 'press', 'activate', 'open'):
                if actions.doAction(i):
                    return
        raise RuntimeError('Control exposes no supported native accessibility action')

    def dismiss(self):
        active = any(node.role == 'Window' and node.native.getState().contains(self.atspi.STATE_ACTIVE) for node in self.nodes())
        if not active:
            raise RuntimeError('Owned benchmark window is not active; refusing global keyboard injection')
        self.atspi.Registry.generateKeyboardEvent(0xff1b, None, self.atspi.KEY_SYM)


class MacDriver(Driver):
    backend = 'macos-ax'

    def __init__(self, pid):
        import ApplicationServices as AX
        import Quartz
        if not AX.AXIsProcessTrusted():
            raise PermissionError('Accessibility permission is required for this benchmark process; not changed automatically')
        self.ax, self.quartz, self.pid = AX, Quartz, pid
        self.app = AX.AXUIElementCreateApplication(pid)

    def attribute(self, item, name):
        code, value = self.ax.AXUIElementCopyAttributeValue(item, name, None)
        return value if code == 0 else None

    def nodes(self):
        # Collapsed AX menu children can still exist. Require an actual on-screen
        # owned popup window for submenu postconditions, not merely AXChildren.
        Q = self.quartz
        popups = [w for w in Q.CGWindowListCopyWindowInfo(Q.kCGWindowListOptionOnScreenOnly, Q.kCGNullWindowID)
                  if w.get(Q.kCGWindowOwnerPID) == self.pid and w.get(Q.kCGWindowLayer, 0) > 0]
        pending = [self.app]; nodes = []; budget = 5000
        roles = {'AXMenuBarItem': 'MenuItem', 'AXMenuItem': 'MenuItem', 'AXButton': 'Button', 'AXWindow': 'Window'}
        while pending and budget:
            item = pending.pop(); budget -= 1
            role = self.attribute(item, 'AXRole')
            title = self.attribute(item, 'AXTitle') or ''
            visible = self.attribute(item, 'AXHidden') is not True
            bounds = []
            position, size = self.attribute(item, 'AXPosition'), self.attribute(item, 'AXSize')
            if position is not None and size is not None:
                p_ok, p = self.ax.AXValueGetValue(position, self.ax.kAXValueCGPointType, None)
                s_ok, s = self.ax.AXValueGetValue(size, self.ax.kAXValueCGSizeType, None)
                if p_ok and s_ok:
                    bounds = [p.x, p.y, p.x+s.width, p.y+s.height]
            if role == 'AXMenuItem':
                # A collapsed item in another menu is not evidence. Its positive
                # rectangle must lie inside an owned on-screen popup rectangle.
                visible = visible and bool(bounds) and bounds[2] > bounds[0] and bounds[3] > bounds[1]
                visible = visible and any(
                    (b := w.get(Q.kCGWindowBounds, {})).get('X', float('inf')) <= bounds[0]
                    and b.get('Y', float('inf')) <= bounds[1]
                    and b.get('X', 0)+b.get('Width', 0) >= bounds[2]
                    and b.get('Y', 0)+b.get('Height', 0) >= bounds[3]
                    for w in popups)
            nodes.append(Node(item, str(title), roles.get(role, str(role)), self.pid, bounds, visible))
            children = self.attribute(item, 'AXChildren') or []
            pending.extend(children)
            if item is self.app:
                bar = self.attribute(item, 'AXMenuBar')
                if bar is not None:
                    pending.append(bar)
        return nodes

    def prepare(self):
        result = self.ax.AXUIElementSetAttributeValue(self.app, 'AXFrontmost', True)
        if result != 0:
            raise RuntimeError('Could not activate owned application')
        windows = [n for n in self.nodes() if n.role == 'Window' and n.visible and n.bounds]
        if not windows:
            raise RuntimeError('Owned AX window geometry is unavailable')
        return dict(backend=self.backend, input_method=self.method, window_bounds=windows[0].bounds)

    def activate(self, node):
        code, actions = self.ax.AXUIElementCopyActionNames(node.native, None)
        for action in ('AXPress', 'AXShowMenu'):
            if code == 0 and action in actions and self.ax.AXUIElementPerformAction(node.native, action) == 0:
                return
        raise RuntimeError('Owned AX control exposes no supported action')

    def dismiss(self):
        if self.attribute(self.app, 'AXFrontmost') is not True:
            raise RuntimeError('Owned application is not frontmost; refusing global keyboard injection')
        Q = self.quartz
        for down in (True, False):
            event = Q.CGEventCreateKeyboardEvent(None, 53, down)
            Q.CGEventPost(Q.kCGHIDEventTap, event)


def connect(system, pid):
    return {'windows': WindowsDriver, 'linux': LinuxDriver, 'macos': MacDriver}[system](pid)
