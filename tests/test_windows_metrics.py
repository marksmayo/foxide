"""Deterministic fixtures at the Win32 DLL boundary; no native GUI or PDH required."""
import ctypes as c
from ctypes import wintypes as w
from pathlib import Path
import sys
import types
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
import windows_metrics as metrics

class Value(c.Structure):
    _fields_ = [('status', w.DWORD), ('value', c.c_double)]
class Item(c.Structure):
    _fields_ = [('name', c.c_void_p), ('value', Value)]

class Pdh:
    def __init__(self, open_status=0, shared_status=0):
        self.closed = 0
        self.collected = 0
        self.fail_collect = False
        self.names = []
        self.rows = {
            1: [('pid_42_luid_a', 0, 100), ('pid_99_luid_a', 0, 9999), ('pid_42_bad', 9, 777), ('pid_42_infinite', 0, float('inf'))],
            2: [('pid_42_luid_a', 1, 200)],
            3: [('pid_42_luid_a_engtype_VideoDecode', 0, 40), ('pid_42_luid_a_engtype_Copy', 0, -2)],
        }
        def opened(source, flags, query):
            if not open_status:
                query._obj.value = 7
            return open_status
        def add(query, name, flags, handle):
            kind = 1 if 'Dedicated' in name else 2 if 'Shared' in name else 3
            if kind == 2 and shared_status:
                return shared_status
            handle._obj.value = kind
            return 0
        def collect(query):
            self.collected += 1
            return 5 if self.fail_collect else 0
        def array(handle, flags, size, count, buffer):
            rows = self.rows[handle.value]
            size._obj.value = c.sizeof(Item) * len(rows)
            count._obj.value = len(rows)
            if buffer is None:
                return 0x800007d2
            records = c.cast(buffer, c.POINTER(Item))
            for i, (name, status, value) in enumerate(rows):
                name_buffer = c.create_unicode_buffer(name)
                self.names.append(name_buffer)
                records[i].name = c.addressof(name_buffer)
                records[i].value.status = status
                records[i].value.value = value
            return 0
        def close(query):
            self.closed += 1
            return 0
        self.PdhOpenQueryW = opened
        self.PdhAddEnglishCounterW = add
        self.PdhCollectQueryData = collect
        self.PdhGetFormattedCounterArrayW = array
        self.PdhCloseQuery = close

class WindowsMetricsTests(unittest.TestCase):
    def test_gpu_samples_filter_ownership_invalid_values_and_warmup_without_faking_missing_data(self):
        dll = Pdh()
        with patch.object(metrics.c, 'WinDLL', return_value=dll, create=True):
            counters = metrics.GpuCounters()
            first = counters.sample({42})
            self.assertEqual(first, dict(gpu_dedicated_bytes=100, gpu_shared_bytes=200, gpu_engine_percent=None))
            second = counters.sample({42})
            self.assertEqual(second['gpu_engine_percent'], {'VideoDecode': 40, 'Copy': 0})
            self.assertEqual(counters.sample({1000}), dict(gpu_dedicated_bytes=None, gpu_shared_bytes=None, gpu_engine_percent=None))
            dll.fail_collect = True
            self.assertEqual(counters.sample({42}), dict(gpu_dedicated_bytes=None, gpu_shared_bytes=None, gpu_engine_percent=None))
            counters.close()
            counters.close()
        self.assertEqual(dll.closed, 1)

    def test_missing_query_and_counter_errors_remain_unavailable(self):
        dll = Pdh(open_status=5)
        with patch.object(metrics.c, 'WinDLL', return_value=dll, create=True):
            counters = metrics.GpuCounters()
            self.assertEqual(counters.errors, {'query': '0x5'})
            self.assertIsNone(counters.sample({42})['gpu_dedicated_bytes'])
            counters.close()
        self.assertEqual(dll.closed, 0)
        dll = Pdh(shared_status=5)
        with patch.object(metrics.c, 'WinDLL', return_value=dll, create=True):
            counters = metrics.GpuCounters()
            self.assertEqual(counters.errors, {'shared': '0x5'})
            sample = counters.sample({42})
            self.assertEqual(sample['gpu_dedicated_bytes'], 100)
            self.assertIsNone(sample['gpu_shared_bytes'])
            counters.close()

    def test_window_probe_excludes_unowned_hidden_and_console_windows(self):
        windows = {1: (42, True, 'Movie', 'Player'), 2: (42, False, 'Hidden', 'Player'),
                   3: (42, True, 'Console', 'ConsoleWindowClass'), 4: (99, True, 'Other', 'Player')}
        def enumerate_windows(callback, unused):
            for handle in windows:
                callback(handle, 0)
        def pid(handle, pointer):
            pointer._obj.value = windows[handle][0]
        def title(handle, buffer, length):
            buffer.value = windows[handle][2]
        def kind(handle, buffer, length):
            buffer.value = windows[handle][3]
        api = types.SimpleNamespace(EnumWindows=enumerate_windows, GetWindowThreadProcessId=pid,
                                    IsWindowVisible=lambda handle: windows[handle][1],
                                    GetWindowTextW=title, GetClassNameW=kind)
        with patch.object(metrics.c, 'WinDLL', return_value=api, create=True), \
             patch.object(metrics.c, 'WINFUNCTYPE', getattr(c, 'WINFUNCTYPE', c.CFUNCTYPE), create=True):
            self.assertEqual(metrics.visible_windows({42}), [{'pid': 42, 'title': 'Movie', 'class': 'Player'}])

    def test_restart_manager_is_read_only_and_always_closes_its_session(self):
        for failure in (None, 'register', 'list'):
            ended = []
            def start(pointer, flags, key):
                pointer._obj.value = 123
                return 0
            def register(session, count, files, *unused):
                self.assertEqual(files[0], 'movie.mkv')
                return 5 if failure == 'register' else 0
            def listing(session, needed, count, records, reasons):
                if failure == 'list':
                    return 5
                if records is None:
                    needed._obj.value = 1
                    return 234
                count._obj.value = 1
                records[0].process.pid = 42
                records[0].process.started.dwHighDateTime = 1
                records[0].process.started.dwLowDateTime = 2
                records[0].app = 'Player'
                return 0
            api = types.SimpleNamespace(RmStartSession=start, RmRegisterResources=register,
                                        RmGetList=listing, RmEndSession=lambda session: ended.append(session.value))
            with self.subTest(failure=failure), patch.object(metrics.c, 'WinDLL', return_value=api, create=True):
                result = metrics.resource_users('movie.mkv')
                self.assertEqual(ended, [123])
                if failure:
                    self.assertEqual(result, {'error': 5, 'processes': []})
                else:
                    self.assertEqual(result, {'error': None, 'processes': [{'pid': 42, 'created': (1 << 32) | 2, 'app': 'Player'}]})
