"""Standard-library process-tree sampler. No power/VRAM estimates or player control.

Spawned before a timed launch; writes 'ready' then reads root PID from stdin.
Windows: Toolhelp + process working set/CPU time. Linux: /proc. macOS: ps RSS/CPU time.
"""
from __future__ import annotations

import argparse
import ctypes as c
from ctypes import wintypes as w
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import time


def windows_snapshot():
    class Entry(c.Structure):
        _fields_ = [('size', w.DWORD), ('usage', w.DWORD), ('pid', w.DWORD),
                    ('heap', c.c_size_t), ('module', w.DWORD), ('threads', w.DWORD),
                    ('parent', w.DWORD), ('priority', w.LONG), ('flags', w.DWORD),
                    ('exe', w.WCHAR * 260)]

    class Memory(c.Structure):
        _fields_ = [('size', w.DWORD), ('faults', w.DWORD)] + [
            (name, c.c_size_t) for name in ('peak', 'rss', 'peak_paged', 'paged',
                                         'peak_nonpaged', 'nonpaged', 'pagefile', 'peak_pagefile')]

    kernel = c.WinDLL('kernel32', use_last_error=True)
    psapi = c.WinDLL('psapi', use_last_error=True)
    kernel.CreateToolhelp32Snapshot.argtypes = [w.DWORD, w.DWORD]
    kernel.CreateToolhelp32Snapshot.restype = w.HANDLE
    kernel.Process32FirstW.argtypes = [w.HANDLE, c.POINTER(Entry)]
    kernel.Process32NextW.argtypes = [w.HANDLE, c.POINTER(Entry)]
    kernel.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
    kernel.OpenProcess.restype = w.HANDLE
    kernel.CloseHandle.argtypes = [w.HANDLE]
    kernel.GetProcessTimes.argtypes = [w.HANDLE] + [c.POINTER(w.FILETIME)] * 4
    psapi.GetProcessMemoryInfo.argtypes = [w.HANDLE, c.POINTER(Memory), w.DWORD]

    def stamp(t):
        return (t.dwHighDateTime << 32) | t.dwLowDateTime

    def snapshot(root):
        handle = kernel.CreateToolhelp32Snapshot(2, 0)  # TH32CS_SNAPPROCESS
        if handle == c.c_void_p(-1).value:
            raise c.WinError(c.get_last_error())
        entries = {}
        entry = Entry()
        entry.size = c.sizeof(entry)
        try:
            valid = kernel.Process32FirstW(handle, c.byref(entry))
            while valid:
                entries[int(entry.pid)] = int(entry.parent)
                valid = kernel.Process32NextW(handle, c.byref(entry))
        finally:
            kernel.CloseHandle(handle)
        result = {}
        memory = Memory()
        memory.size = c.sizeof(memory)
        created, exited, kern, user = (w.FILETIME() for _ in range(4))
        for pid in descendants(entries, root):
            process = kernel.OpenProcess(0x0400 | 0x0010, False, pid)
            if not process:
                continue
            try:
                if (psapi.GetProcessMemoryInfo(process, c.byref(memory), memory.size)
                        and kernel.GetProcessTimes(process, c.byref(created), c.byref(exited),
                                                   c.byref(kern), c.byref(user))):
                    result[pid] = (stamp(created), memory.rss, (stamp(kern) + stamp(user)) / 1e7)
            finally:
                kernel.CloseHandle(process)
        return result
    return snapshot


def descendants(parents, root):
    if root not in parents:
        return set()
    children = {}
    for pid, parent in parents.items():
        children.setdefault(parent, []).append(pid)
    selected, pending = {root}, [root]
    while pending:
        for pid in children.get(pending.pop(), ()):
            if pid not in selected:
                selected.add(pid)
                pending.append(pid)
    return selected


def linux_snapshot(root):
    records, parents = {}, {}
    page_size, clock_ticks = os.sysconf('SC_PAGE_SIZE'), os.sysconf('SC_CLK_TCK')
    for path in Path('/proc').iterdir():
        if not path.name.isdigit():
            continue
        try:
            # comm can contain spaces and ')'; fields after the final ')' start at field 3.
            fields = (path / 'stat').read_text().rsplit(')', 1)[1].split()
            pid = int(path.name)
            parents[pid] = int(fields[1])
            records[pid] = (int(fields[19]), int(fields[21]) * page_size,
                            (int(fields[11]) + int(fields[12])) / clock_ticks)
        except (OSError, ValueError, IndexError):
            continue
    return {pid: records[pid] for pid in descendants(parents, root)}


def cpu_seconds(value):
    days, sep, value_tail = value.partition('-')
    if not sep:
        value_tail, days = days, '0'
    total = 0.0
    for part in value_tail.split(':'):
        total = total * 60 + float(part)
    return total + int(days) * 86400


def mac_snapshot(root):
    result = subprocess.run(['ps', '-axo', 'pid=,ppid=,rss=,time=,lstart='],
                            check=True, capture_output=True, text=True)
    raw, parents = {}, {}
    for line in result.stdout.splitlines():
        fields = line.split(None, 4)
        if len(fields) != 5:
            continue
        pid, parent, rss, cpu, started = fields
        parents[int(pid)] = int(parent)
        raw[int(pid)] = started, rss, cpu
    return {pid: (raw[pid][0], int(raw[pid][1]) * 1024, cpu_seconds(raw[pid][2]))
            for pid in descendants(parents, root)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--duration', required=True, type=float)
    parser.add_argument('--interval-ms', required=True, type=int)
    args = parser.parse_args()
    if args.duration <= 0 or args.interval_ms < 100:
        parser.error('duration must be positive and interval >= 100ms')
    sampler = windows_snapshot() if os.name == 'nt' else (
        linux_snapshot if sys.platform.startswith('linux') else mac_snapshot)
    interval = args.interval_ms / 1000
    encoder = json.JSONEncoder(allow_nan=False, separators=(',', ':'))
    with args.output.open('x', encoding='utf-8') as output:
        def emit(record):
            output.write(encoder.encode(record) + '\n')
            output.flush()
        emit({'kind': 'host', 'platform': platform.platform(), 'machine': platform.machine(),
              'python': platform.python_version(), 'logical_cpus': os.cpu_count(),
              'clock': time.get_clock_info('perf_counter').implementation,
              'memory_scope': 'sum of resident sets; shared pages can be counted more than once'})
        print('ready', flush=True)
        root = int(sys.stdin.readline())
        started, previous_time, previous = time.perf_counter(), None, {}
        root_identity = None
        while time.perf_counter() - started < args.duration:
            tick = time.perf_counter()
            records = sampler(root)
            now = time.perf_counter()
            if root not in records:
                # A new process may not yet be visible in the first snapshot.
                if now - started > 1.0:
                    emit({'kind': 'root-exited-or-inaccessible'})
                    break
            else:
                if root_identity is None:
                    root_identity = records[root][0]
                elif root_identity != records[root][0]:
                    emit({'kind': 'pid-reused'})
                    break
                common = (pid for pid, record in records.items()
                          if pid in previous and previous[pid][0] == record[0])
                cpu = None if previous_time is None else sum(
                    max(0.0, records[pid][2] - previous[pid][2]) for pid in common
                ) / (now - previous_time) * 100
                emit({'kind': 'sample', 'elapsed_ms': (now - started) * 1000,
                      'rss_bytes': sum(record[1] for record in records.values()),
                      'cpu_percent_one_core': cpu, 'process_count': len(records),
                      'observer_sample_ms': (now - tick) * 1000})
            previous, previous_time = records, now
            time.sleep(max(0.0, interval - (time.perf_counter() - tick)))


if __name__ == '__main__':
    main()
