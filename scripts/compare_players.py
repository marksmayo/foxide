"""Run a reproducible Windows player comparison and preserve raw per-trial evidence.

The selected ten are a representative field, not a market-share ranking. Missing
players and metrics remain unavailable. Foxide currently depends on external mpv.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import random
import re
import shutil
import statistics
import subprocess
import sys
import time

from telemetry import windows_snapshot
from windows_metrics import GpuCounters, visible_windows, resource_users

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def resolve(candidate, folders=None):
    folders = {} if folders is None else folders
    def folder(match):
        name = match[1]
        if name in folders:
            return folders[name]
        if os.environ.get(name):
            return os.environ[name]
        # Restricted launch environments can omit these standard variables.
        # Ask Windows for the user's actual known folder instead of assuming C:.
        ids = {'ProgramFiles': 0x26, 'ProgramFiles(x86)': 0x2a, 'LOCALAPPDATA': 0x1c}
        if os.name == 'nt' and name in ids:
            import ctypes
            value = ctypes.create_unicode_buffer(32768)
            if ctypes.windll.shell32.SHGetFolderPathW(None, ids[name], None, 0, value) == 0:
                folders[name] = value.value
                return value.value
        return match[0]
    candidate = re.sub(r'\{([^}]+)\}', folder, candidate)
    direct = Path(candidate)
    if not direct.is_absolute():
        direct = ROOT / direct
    if direct.is_file():
        return direct.resolve()
    found = shutil.which(candidate)
    return Path(found).resolve() if found else None


def inventory(fingerprint=True):
    folders, paths, fingerprints = {}, {}, {}
    def cached_resolve(candidate):
        if candidate not in paths:
            paths[candidate] = resolve(candidate, folders)
        return paths[candidate]
    catalog = json.loads((ROOT / 'benchmarks/windows-players.json').read_text('utf-8'))
    for player in catalog['players']:
        if 'windows' not in player.get('platforms', ['windows']):
            player.update(executable=None, status='other-platform',
                          reason='Available on ' + ', '.join(player['platforms']) + '; not installed or benchmarked as a Windows app.')
            continue
        path = next((path for candidate in player['paths'] if (path := cached_resolve(candidate))), None)
        player['executable'] = str(path) if path else None
        player['status'] = 'available' if path else 'not-detected'
        player['reason'] = None if path else 'No executable found in configured paths or PATH.'
        if path and fingerprint:
            if path not in fingerprints:
                fingerprints[path] = digest(path), path.stat().st_size
            player['sha256'], player['executable_bytes'] = fingerprints[path]
        # Native frontend automation is not inferred from the fact a binary exists.
        if player['id'] not in ('vlc', 'mpv', 'foxide', 'ffplay', 'wmp-legacy') and path:
            player['status'] = 'adapter-needed'
            player['reason'] = 'Executable discovered; validated launch/playback adapter is required before ranking.'
    lookup = {p['id']: p for p in catalog['players']}
    if not lookup['mpv']['executable']:
        lookup['foxide']['status'] = 'blocked-dependency'
        lookup['foxide']['reason'] = 'Foxide 0.0.1 launches external mpv; mpv.exe is not available. No independent Foxide media engine exists yet.'
    return catalog


def prepare_media(output):
    directory = ROOT / 'media/generated/comparison'
    manifest_path = directory / 'manifest.json'
    if manifest_path.exists():
        manifest = json.loads(manifest_path.read_text('utf-8'))
        for fixture in manifest['fixtures']:
            if digest(directory / fixture['path']) != fixture['sha256']:
                raise ValueError('Comparison fixture hash mismatch: ' + fixture['path'])
        return directory, manifest
    directory.mkdir(parents=True, exist_ok=True)
    fixtures = []
    specs = [
        ('A', ROOT / 'media/generated/quick/A-avc-1080p.mp4', 'A-avc-1080p.mp4', '1080p AVC · AAC', False),
        ('B', ROOT / 'media/generated/smoke/B-hevc-4k60-pq.mkv', 'B-hevc-4k60-pq.mkv', '4K60 HEVC Main10 · PQ · AC3', True),
        ('C', ROOT / 'media/generated/smoke/C-av1-4k60.mkv', 'C-av1-4k60.mkv', '4K60 AV1 10-bit · Opus', False),
        ('D', ROOT / 'media/generated/smoke/D-flac-track-01.flac', 'D-audio.flac', 'FLAC · 48 kHz · embedded artwork', False),
    ]
    for identifier, source, filename, label, strip_subtitles in specs:
        if not source.is_file():
            raise FileNotFoundError(f'Generate starter fixtures first: {source}')
        if strip_subtitles:
            # Starter subtitle packets may extend beyond video duration. Strip them
            # before looping so the muxer does not introduce multi-minute gaps.
            base = directory / 'B-loop-source.mkv'
            if not base.exists():
                subprocess.run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-n', '-i', str(source),
                                '-map', '0:v:0', '-map', '0:a:0', '-c', 'copy', '-t', '3', str(base)], check=True)
            loop_source = base
        else:
            loop_source = source
        destination = directory / filename
        if destination.exists():
            raise FileExistsError(f'Unmanifested fixture already exists: {destination}')
        command = ['ffmpeg', '-hide_banner', '-loglevel', 'warning', '-n', '-stream_loop', '-1',
                   '-i', str(loop_source), '-map', '0:v?', '-map', '0:a', '-sn', '-c', 'copy', '-t', '75']
        if identifier == 'D':
            # Loop the audio only; cover is copied once as an attached picture.
            command = ['ffmpeg', '-hide_banner', '-loglevel', 'warning', '-n', '-stream_loop', '-1',
                       '-i', str(source), '-map', '0:a', '-map', '0:v?', '-c:a', 'flac', '-c:v', 'copy',
                       '-disposition:v', 'attached_pic', '-t', '75']
        if identifier == 'A':
            command += ['-movflags', '+faststart']
        command.append(str(destination))
        print('Preparing', label, flush=True)
        with (output / f'fixture-{identifier}.log').open('w', encoding='utf-8') as log:
            subprocess.run(command, check=True, stdout=log, stderr=log)
        probe = json.loads(subprocess.check_output(['ffprobe', '-v', 'error', '-show_streams', '-show_format',
                                                    '-of', 'json', str(destination)], text=True))
        if not 74 <= float(probe['format']['duration']) <= 77:
            raise ValueError('Unexpected comparison duration')
        fixtures.append({'id': identifier, 'path': filename, 'label': label, 'sha256': digest(destination),
                         'source_sha256': digest(source), 'bytes': destination.stat().st_size,
                         'command': command, 'probe': probe})
    manifest = {'recipe': '75s repeated synthetic bitstreams; B subtitle-free performance variant; subtitles disabled for video comparisons',
                'seconds': 75, 'fixtures': fixtures}
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    return directory, manifest


def command_for(player, media, catalog, trial_dir):
    exe = player['executable']
    if player['id'] == 'vlc':
        return [exe, '--ignore-config', '--no-one-instance', '--no-video-title-show', '--play-and-exit',
                '--no-qt-video-autoresize', '--width=1280', '--height=720', '--sub-track=-1', '--verbose=2', '--', str(media)]
    if player['id'] == 'ffplay':
        return [exe, '-hide_banner', '-loglevel', 'info', '-stats', '-autoexit', '-x', '1280', '-y', '720', '-i', str(media)]
    if player['id'] == 'wmp-legacy':
        return [exe, '/play', str(media)]
    if player['id'] == 'mpv':
        return [exe, '--no-config', '--load-scripts=no', '--no-terminal', '--geometry=1280x720', '--sid=no',
                '--script=' + str(ROOT / 'crates/player-bench/assets/bench.lua'), '--', str(media)]
    if player['id'] == 'foxide':
        mpv = next(p for p in catalog['players'] if p['id'] == 'mpv')
        return [exe, '--mpv', mpv['executable'] or 'mpv', str(media)]
    raise ValueError('No validated adapter for ' + player['id'])


def playback_evidence(player_id, stderr, stdout, fixture_id, events):
    text = stderr + '\n' + stdout
    if player_id == 'vlc':
        video = re.search(r'codec \((h264|hevc|av1)\) started', text)
        audio = re.search(r'(codec \((flac|aac|ac3|opus)\) started|using audio decoder module|using packetizer module "flac")', text)
        return {'verified': bool(audio if fixture_id == 'D' else video),
                'method': 'VLC engine diagnostic log; not screen presentation',
                'hardware': 'D3D11VA reported' if 'Using D3D11VA' in text else (
                    'Hardware capability/selection logged; final path unverified' if 'd3d11va' in text.lower() else 'Not established')}
    if player_id == 'ffplay':
        positions = [float(m[1]) for m in re.finditer(r'(?<![\d.])(\d+\.\d+)\s+[AMV]-[AMV]:', text)]
        return {'verified': bool(positions and max(positions) >= 2),
                'method': 'FFplay advancing A/V clock in runtime stats; not screen presentation',
                'last_clock_seconds': max(positions) if positions else None,
                'hardware': 'Default FFplay configuration; no hardware decode requested'}
    if player_id == 'mpv':
        return {'verified': any(e.get('kind') == 'first-playback-restart-proxy' for e in events),
                'method': 'mpv internal playback-restart event; not screen presentation', 'hardware': 'See mpv events'}
    return {'verified': False, 'method': 'No verified playback-state collector for this frontend', 'hardware': 'Not established'}


def one_trial(player, fixture, media, catalog, output, duration, warmup, number, prime=False):
    trial_id = f"{player['id']}-{fixture['id']}-{'prime' if prime else number}"
    directory = output / 'runs' / trial_id
    directory.mkdir(parents=True, exist_ok=False)
    snapshot = windows_snapshot()
    gpu = GpuCounters()
    command = command_for(player, media, catalog, directory)
    env = dict(os.environ, FOXIDE_BENCH_EVENTS=str(directory / 'events.jsonl'), FOXIDE_BENCH_SEEKS='0')
    result = {'id': trial_id, 'player': player['id'], 'fixture': fixture['id'], 'prime': prime,
              'command': command, 'duration_seconds': duration, 'warmup_seconds': warmup,
              'sample_interval_ms': 250, 'cache': 'warm-primed' if not prime else 'priming',
              'started_utc': datetime.now(timezone.utc).isoformat(), 'samples': [], 'window_ready_ms': None,
              'ttff_display_ms': None, 'seek_display_ms': None, 'power_watts': None,
              'frame_pacing_ms': None, 'status': 'pending', 'gpu_counter_errors': gpu.errors,
              'notes': ['RSS tree sum can count shared pages more than once.',
                        'Window-ready measures an enumerated visible window, not video TTFF.',
                        'GPU memory counts allocation, not exclusively resident VRAM. No CPU-power attribution.']}
    previous, previous_time, root_identity = {}, None, None
    try:
        with (directory / 'stdout.log').open('wb') as stdout, (directory / 'stderr.log').open('wb') as stderr:
            started = time.perf_counter()
            process = subprocess.Popen(command, stdout=stdout, stderr=stderr, stdin=subprocess.DEVNULL, env=env, cwd=ROOT)
            result['pid'] = process.pid
            next_sample, next_gpu, gpu_sample = started, started, {}
            last_windows = []
            try:
                while time.perf_counter() - started < duration:
                    now = time.perf_counter()
                    if now >= next_sample:
                        records = snapshot(process.pid)
                        if process.pid in records:
                            identity = records[process.pid][0]
                            if root_identity is None:
                                root_identity = identity
                            if identity != root_identity:
                                result['status'] = 'pid-reused'
                                break
                        pids = set(records)
                        if now >= next_gpu:
                            gpu_sample = gpu.sample(pids)
                            next_gpu = now + 1
                        elapsed = time.perf_counter() - started
                        common = [pid for pid in pids if pid in previous and records[pid][0] == previous[pid][0]]
                        cpu = None if previous_time is None else sum(max(0, records[pid][2] - previous[pid][2]) for pid in common) / (now - previous_time) * 100
                        if records:
                            result['samples'].append({'elapsed_ms': elapsed * 1000, 'rss_bytes': sum(r[1] for r in records.values()),
                                'cpu_percent_one_core': cpu, 'process_count': len(records), **gpu_sample})
                        previous, previous_time = records, now
                        next_sample = now + .25
                    # Polling includes only the benchmark process tree, never unrelated windows.
                    if result['window_ready_ms'] is None or player['id'] == 'wmp-legacy':
                        windows = visible_windows(set(previous) | {process.pid})
                        if windows:
                            last_windows = windows
                            if result['window_ready_ms'] is None:
                                result['window_ready_ms'] = (time.perf_counter() - started) * 1000
                    if process.poll() is not None:
                        result['status'] = 'early-exit'
                        result['exit_code'] = process.returncode
                        break
                    time.sleep(.01 if result['window_ready_ms'] is None else .025)
                else:
                    result['status'] = 'sampled'
                result['windows'] = last_windows
            finally:
                if player['id'] == 'wmp-legacy':
                    result['source_ownership'] = resource_users(media)
                    result['root_creation_identity'] = root_identity
                # Kill only process identities belonging to the newly launched tree.
                # Native single-instance launchers must not be attached to an unrelated process.
                if process.poll() is None:
                    records = snapshot(process.pid)
                    if records and (root_identity is None or records.get(process.pid, (None,))[0] == root_identity):
                        for pid, record in records.items():
                            if pid != process.pid:
                                terminate_owned(pid, record[0])
                    process.terminate()
                    result['terminated_by_harness'] = True
                process.wait(timeout=10)
    except OSError as exc:
        result['status'] = 'launch-error'
        result['error'] = str(exc)
    finally:
        gpu.close()
    events_path = directory / 'events.jsonl'
    events = []
    if events_path.exists():
        for line in events_path.read_text('utf-8', errors='replace').splitlines():
            try:
                events.append(json.loads(line))
            except json.JSONDecodeError:
                result['notes'].append('Truncated final engine event ignored; raw file retained')
    result['events'] = events
    stderr_text = (directory / 'stderr.log').read_text('utf-8', errors='replace')
    stdout_text = (directory / 'stdout.log').read_text('utf-8', errors='replace')
    result['playback_evidence'] = playback_evidence(player['id'], stderr_text, stdout_text, fixture['id'], events)
    if player['id'] == 'wmp-legacy':
        owners = result.get('source_ownership', {}).get('processes', [])
        source_open = any(p['pid'] == result.get('pid') and p['created'] == root_identity for p in owners)
        active_seconds = {int(s['elapsed_ms'] / 1000) for s in result['samples']
                          if any(v > 0.1 for engine, v in (s.get('gpu_engine_percent') or {}).items()
                                 if engine.lower() == 'videodecode')}
        result['playback_evidence'] = {
            'verified': source_open and len(active_seconds) >= 3 and fixture['id'] != 'D',
            'method': 'Restart Manager confirms this PID/creation-time holds the exact fixture, plus VideoDecode GPU activity in at least three one-second buckets; not screen presentation',
            'source_file_open': source_open, 'active_decode_second_buckets': sorted(active_seconds),
            'hardware': 'Windows GPU VideoDecode activity observed' if active_seconds else 'No active VideoDecode evidence',
        }
    if result['status'] == 'sampled' and not result['playback_evidence']['verified']:
        result['status'] = 'playback-unverified'
    steady = [s for s in result['samples'] if s['elapsed_ms'] >= warmup * 1000]
    def mean(key):
        values = [s[key] for s in steady if s.get(key) is not None]
        return statistics.mean(values) if values else None
    result['rss_steady_bytes'] = mean('rss_bytes')
    result['cpu_steady_percent'] = mean('cpu_percent_one_core')
    result['gpu_dedicated_steady_bytes'] = mean('gpu_dedicated_bytes')
    result['gpu_shared_steady_bytes'] = mean('gpu_shared_bytes')
    result['rss_peak_bytes'] = max((s['rss_bytes'] for s in result['samples']), default=None)
    (directory / 'run.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
    print(f"{trial_id}: {result['status']} · samples={len(result['samples'])}", flush=True)
    return result


def terminate_owned(pid, identity):
    """Terminate only a child whose creation timestamp still matches the sample."""
    import ctypes as c
    from ctypes import wintypes as w
    kernel = c.WinDLL('kernel32', use_last_error=True)
    kernel.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
    kernel.OpenProcess.restype = w.HANDLE
    kernel.GetProcessTimes.argtypes = [w.HANDLE] + [c.POINTER(w.FILETIME)] * 4
    kernel.TerminateProcess.argtypes = [w.HANDLE, w.UINT]
    kernel.CloseHandle.argtypes = [w.HANDLE]
    handle = kernel.OpenProcess(1 | 0x0400, False, pid)
    if not handle:
        return
    try:
        created, ended, kern, user = (w.FILETIME() for _ in range(4))
        if kernel.GetProcessTimes(handle, c.byref(created), c.byref(ended), c.byref(kern), c.byref(user)):
            if (created.dwHighDateTime << 32) | created.dwLowDateTime == identity:
                kernel.TerminateProcess(handle, 0)
    finally:
        kernel.CloseHandle(handle)


def private_output(output):
    """Raw process logs and paths must never be written to tracked publication folders."""
    output = output.resolve()
    if not output.is_relative_to((ROOT / 'results/local').resolve()):
        raise ValueError('Raw comparison output must stay under results/local; publish a redacted export separately')
    return output


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output', type=Path, default=ROOT / 'results/local/windows-comparison')
    p.add_argument('--duration', type=float, default=20)
    p.add_argument('--warmup', type=float, default=5)
    p.add_argument('--repeat', type=int, default=5)
    p.add_argument('--files', nargs='+', choices=list('ABCD'), default=list('ABCD'))
    p.add_argument('--players', nargs='+', default=['vlc', 'ffplay', 'mpv', 'foxide', 'wmp-legacy'])
    p.add_argument('--inventory-only', action='store_true')
    args = p.parse_args()
    if os.name != 'nt':
        p.error('This comparison collector targets Windows')
    if not math.isfinite(args.duration) or not 1 <= args.duration <= 65 or not 0 <= args.warmup < args.duration or not 1 <= args.repeat <= 30:
        p.error('duration 1–65s, 0 <= warmup < duration, repeat 1–30 required')
    try:
        output = private_output(args.output)
    except ValueError as error:
        p.error(str(error))
    output.mkdir(parents=True, exist_ok=True)
    if (output / 'comparison.json').exists():
        p.error('Comparison output already exists; choose a fresh output directory')
    catalog = inventory()
    payload = {'schema_version': 1, 'created_utc': datetime.now(timezone.utc).isoformat(),
               'catalog': catalog, 'settings': {'duration': args.duration, 'warmup': args.warmup,
                   'repeat': args.repeat, 'order_seed': 20261113, 'window_request': '1280×720 for VLC/mpv/FFplay',
                   'subtitles': 'Disabled for matched performance cases', 'audio': 'Enabled at player defaults; volume/route not normalized',
                   'classification': 'Exploratory warm-cache process comparison, not battery/display certification'},
               'environment': environment(), 'trials': []}
    path = output / 'comparison.json'
    def save():
        path.write_text(json.dumps(payload, indent=2, allow_nan=False), encoding='utf-8')
    if args.inventory_only:
        save()
        print(path)
        return
    media_dir, manifest = prepare_media(output)
    payload['media'] = manifest
    players = [player for player in catalog['players'] if player['id'] in args.players]
    if 'foxide' in args.players:
        foxide = next(p for p in players if p['id'] == 'foxide')
        if foxide['status'] == 'blocked-dependency' and foxide['executable']:
            attempt = subprocess.run([foxide['executable'], '--mpv', 'mpv', str(media_dir / manifest['fixtures'][0]['path'])],
                                     capture_output=True, text=True, timeout=10)
            foxide['launch_attempt'] = {'exit_code': attempt.returncode, 'stdout': attempt.stdout, 'stderr': attempt.stderr}
    ready = [player for player in players if player['status'] == 'available']
    # Legacy WMP may show onboarding or delegate to another process. One diagnostic
    # attempt is enough until playback can be verified; never award it an idle medal.
    legacy = next((player for player in ready if player['id'] == 'wmp-legacy'), None)
    if legacy:
        fixture = manifest['fixtures'][0]
        trial = one_trial(legacy, fixture, media_dir / fixture['path'], catalog, output, 8, 2, 'diagnostic')
        payload['trials'].append(trial)
        if trial['status'] != 'sampled':
            legacy['status'] = 'playback-unverified'
            legacy['reason'] = 'Diagnostic launch captured process/window evidence but could not verify active playback; excluded from performance medals.'
            ready.remove(legacy)
        save()
    randomizer = random.Random(20261113)
    for fixture in manifest['fixtures']:
        if fixture['id'] not in args.files:
            continue
        media = media_dir / fixture['path']
        for player in ready:
            trial = one_trial(player, fixture, media, catalog, output, 3, 0, 0, prime=True)
            payload['trials'].append(trial)
            save()
        for number in range(1, args.repeat + 1):
            order = ready[:]
            randomizer.shuffle(order)
            for player in order:
                trial = one_trial(player, fixture, media, catalog, output, args.duration, args.warmup, number)
                payload['trials'].append(trial)
                save()
    save()
    print(path, flush=True)


def environment():
    import platform
    data = {'os': platform.platform(), 'architecture': platform.machine(), 'logical_cpus': os.cpu_count(),
            'python': platform.python_version(), 'background_workloads_controlled': False,
            'power_source_plan_display_calibrated': False,
            'download_blocker': 'Configured terminal proxy unavailable; browser app access timed out.'}
    command = "Get-CimInstance Win32_Processor | Select-Object Name; Get-CimInstance Win32_VideoController | Select-Object Name,DriverVersion"
    try:
        result = subprocess.run(['powershell.exe', '-NoProfile', '-Command', command], capture_output=True, text=True, timeout=15)
        data['hardware_inventory'] = result.stdout.strip()
    except (OSError, subprocess.TimeoutExpired):
        data['hardware_inventory'] = 'Unavailable under current process permissions'
    if not data['hardware_inventory']:
        data['hardware_inventory'] = 'CIM inventory unavailable under current process permissions'
    try:
        import winreg
        with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, r'HARDWARE\DESCRIPTION\System\CentralProcessor\0') as key:
            data['cpu_model'] = winreg.QueryValueEx(key, 'ProcessorNameString')[0].strip()
        data['hardware_inventory'] = data['cpu_model'] + '\n' + str(data['logical_cpus']) + ' logical CPUs'
    except OSError:
        data['cpu_model'] = None
    return data


if __name__ == '__main__':
    main()
