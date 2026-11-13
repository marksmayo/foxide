"""Passive Windows benchmark telemetry: PDH GPU counters and process-owned window visibility.

No input injection, screenshot capture, UI control, settings changes or elevation.
PDH missing/invalid counter samples return None, never a fabricated zero.
"""
import ctypes as c
from ctypes import wintypes as w
import math
import re


class GpuCounters:
    def __init__(self):
        self.query = c.c_void_p()
        self.handles = {}
        self.errors = {}
        self.ready = False
        self.pdh = c.WinDLL('pdh')
        self.pdh.PdhOpenQueryW.argtypes = [w.LPCWSTR, c.c_size_t, c.POINTER(c.c_void_p)]
        self.pdh.PdhAddEnglishCounterW.argtypes = [c.c_void_p, w.LPCWSTR, c.c_size_t, c.POINTER(c.c_void_p)]
        self.pdh.PdhCollectQueryData.argtypes = [c.c_void_p]
        self.pdh.PdhGetFormattedCounterArrayW.argtypes = [c.c_void_p, w.DWORD, c.POINTER(w.DWORD), c.POINTER(w.DWORD), c.c_void_p]
        self.pdh.PdhCloseQuery.argtypes = [c.c_void_p]
        status = self.pdh.PdhOpenQueryW(None, 0, c.byref(self.query))
        if status:
            self.errors['query'] = hex(status & 0xffffffff)
            return
        for key, counter in {
            'dedicated': r'\GPU Process Memory(*)\Dedicated Usage',
            'shared': r'\GPU Process Memory(*)\Shared Usage',
            'engines': r'\GPU Engine(*)\Utilization Percentage',
        }.items():
            handle = c.c_void_p()
            status = self.pdh.PdhAddEnglishCounterW(self.query, counter, 0, c.byref(handle))
            if status:
                self.errors[key] = hex(status & 0xffffffff)
            else:
                self.handles[key] = handle

    def _array(self, handle):
        class Value(c.Structure):
            _fields_ = [('status', w.DWORD), ('value', c.c_double)]
        class Item(c.Structure):
            _fields_ = [('name', w.LPWSTR), ('value', Value)]
        size, count = w.DWORD(), w.DWORD()
        status = self.pdh.PdhGetFormattedCounterArrayW(handle, 0x200 | 0x8000, c.byref(size), c.byref(count), None)
        if (status & 0xffffffff) != 0x800007d2 or size.value == 0:  # PDH_MORE_DATA
            return []
        buffer = c.create_string_buffer(size.value)
        status = self.pdh.PdhGetFormattedCounterArrayW(handle, 0x200 | 0x8000, c.byref(size), c.byref(count), buffer)
        if status:
            return []
        array = c.cast(buffer, c.POINTER(Item))
        return [(array[i].name, array[i].value.value) for i in range(count.value)
                if array[i].value.status in (0, 1) and math.isfinite(array[i].value.value)]

    def sample(self, pids):
        if not self.query:
            return {'gpu_dedicated_bytes': None, 'gpu_shared_bytes': None, 'gpu_engine_percent': None}
        status = self.pdh.PdhCollectQueryData(self.query)
        if status:
            return {'gpu_dedicated_bytes': None, 'gpu_shared_bytes': None, 'gpu_engine_percent': None}
        selected = {}
        for key, handle in self.handles.items():
            entries = []
            for name, value in self._array(handle):
                match = re.search(r'pid_(\d+)_', name)
                if match and int(match[1]) in pids:
                    entries.append((name, max(0.0, value)))
            selected[key] = entries
        engines = {}
        for name, value in selected.get('engines', []):
            kind = name.split('engtype_', 1)[-1]
            engines[kind] = engines.get(kind, 0.0) + value
        result = {
            'gpu_dedicated_bytes': sum(v for _, v in selected['dedicated']) if selected.get('dedicated') else None,
            'gpu_shared_bytes': sum(v for _, v in selected['shared']) if selected.get('shared') else None,
            # Engine-specific process utilization is not overall GPU utilization.
            'gpu_engine_percent': engines if self.ready and engines else None,
        }
        self.ready = True
        return result

    def close(self):
        if self.query:
            self.pdh.PdhCloseQuery(self.query)
            self.query = c.c_void_p()


def visible_windows(pids):
    user = c.WinDLL('user32')
    callback_type = c.WINFUNCTYPE(w.BOOL, w.HWND, w.LPARAM)
    user.EnumWindows.argtypes = [callback_type, w.LPARAM]
    user.IsWindowVisible.argtypes = [w.HWND]
    user.GetWindowThreadProcessId.argtypes = [w.HWND, c.POINTER(w.DWORD)]
    user.GetWindowTextW.argtypes = [w.HWND, w.LPWSTR, c.c_int]
    user.GetClassNameW.argtypes = [w.HWND, w.LPWSTR, c.c_int]
    result = []
    @callback_type
    def visit(hwnd, unused):
        pid = w.DWORD()
        user.GetWindowThreadProcessId(hwnd, c.byref(pid))
        if pid.value in pids and user.IsWindowVisible(hwnd):
            title, kind = c.create_unicode_buffer(512), c.create_unicode_buffer(256)
            user.GetWindowTextW(hwnd, title, len(title))
            user.GetClassNameW(hwnd, kind, len(kind))
            # Ignore startup console/helper windows; this is still not video TTFF.
            if kind.value not in ('ConsoleWindowClass', 'PseudoConsoleWindow'):
                result.append({'pid': pid.value, 'title': title.value, 'class': kind.value})
        return True
    user.EnumWindows(visit, 0)
    return result


def resource_users(path):
    """Read-only Restart Manager file ownership query; never shuts down an app."""
    class UniqueProcess(c.Structure):
        _fields_ = [('pid', w.DWORD), ('started', w.FILETIME)]
    class ProcessInfo(c.Structure):
        _fields_ = [('process', UniqueProcess), ('app', w.WCHAR * 256), ('service', w.WCHAR * 64),
                    ('kind', w.DWORD), ('status', w.ULONG), ('session', w.DWORD), ('restartable', w.BOOL)]
    rm = c.WinDLL('rstrtmgr')
    rm.RmStartSession.argtypes = [c.POINTER(w.DWORD), w.DWORD, w.LPWSTR]
    rm.RmRegisterResources.argtypes = [w.DWORD, w.UINT, c.POINTER(w.LPCWSTR), w.UINT, c.c_void_p, w.UINT, c.c_void_p]
    rm.RmGetList.argtypes = [w.DWORD, c.POINTER(w.UINT), c.POINTER(w.UINT), c.POINTER(ProcessInfo), c.POINTER(w.DWORD)]
    rm.RmEndSession.argtypes = [w.DWORD]
    session, key = w.DWORD(), c.create_unicode_buffer(33)
    status = rm.RmStartSession(c.byref(session), 0, key)
    if status:
        return {'error': status, 'processes': []}
    try:
        files = (w.LPCWSTR * 1)(str(path))
        status = rm.RmRegisterResources(session, 1, files, 0, None, 0, None)
        if status:
            return {'error': status, 'processes': []}
        needed, count, reasons = w.UINT(), w.UINT(), w.DWORD()
        status = rm.RmGetList(session, c.byref(needed), c.byref(count), None, c.byref(reasons))
        if status == 0:
            return {'error': None, 'processes': []}
        if status != 234 or needed.value > 256:
            return {'error': status, 'processes': []}
        records = (ProcessInfo * needed.value)()
        count.value = needed.value
        status = rm.RmGetList(session, c.byref(needed), c.byref(count), records, c.byref(reasons))
        if status:
            return {'error': status, 'processes': []}
        return {'error': None, 'processes': [{'pid': records[i].process.pid,
                  'created': (records[i].process.started.dwHighDateTime << 32) | records[i].process.started.dwLowDateTime,
                  'app': records[i].app} for i in range(count.value)]}
    finally:
        rm.RmEndSession(session)
