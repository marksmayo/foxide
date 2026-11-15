"""Run the native competitor field and preserve verified event measurements.

VM cohorts deliberately use software decoding and null outputs for mpv/VLC/
Foxide. Other frontends get isolated default-launch diagnostics. They cannot
earn playback medals without a validated playback-state adapter.
"""
from __future__ import annotations
import argparse
import base64
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import secrets
import shutil
import signal
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request

from collection_io import EventTail, write_json
from event_benchmarks import measurement_rows, playback_verified, summarize_samples, vlc_software_decode_verified
from telemetry import windows_snapshot, linux_snapshot, mac_snapshot

ROOT = Path(__file__).resolve().parents[1]
CORE = {'mpv', 'vlc', 'foxide'}
SCENARIOS = {'avc1080': 'A', 'hevc4k': 'B', 'av14k': 'C', 'flac': 'D',
             'speed2x': 'A', 'ass': 'A', 'paused': 'A'}


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def locate(player, system, discovered=None):
    if system == 'windows':
        from compare_players import inventory
        return (discovered if discovered is not None else {p['id']: p.get('executable') for p in inventory(fingerprint=False)['players']}).get(player)
    if player == 'foxide':
        binary = ROOT / 'target/release/foxide'
        return str(binary) if binary.is_file() else None
    if system == 'macos':
        apps = {'vlc': ('VLC', 'VLC'), 'iina': ('IINA', 'IINA'), 'kodi': ('Kodi', 'Kodi'),
                'plex': ('Plex', 'Plex'), 'smplayer': ('SMPlayer', 'smplayer'),
                'foobar2000': ('foobar2000', 'foobar2000'), 'infuse': ('Infuse', 'Infuse'),
                'quicktime': ('QuickTime Player', 'QuickTime Player')}
        if player in apps:
            app, exe = apps[player]
            for prefix in ('/Applications', '/System/Applications', str(Path.home() / 'Applications')):
                path = Path(prefix) / (app + '.app/Contents/MacOS') / exe
                if path.is_file():
                    return str(path)
    return shutil.which(player)


def command_for(player, executable, scenario, media, subtitle, port, password, presentation='software-null'):
    script = str(ROOT / 'crates/player-bench/assets/bench.lua')
    if player == 'mpv':
        args = ['--no-config', '--load-scripts=no', '--vo=null', '--ao=null', '--hwdec=no',
                '--no-terminal', '--loop-file=inf', '--script=' + script]
        if presentation == 'native-window':
            args = [a for a in args if not a.startswith(('--vo=', '--ao=', '--hwdec='))] + ['--hwdec=auto-safe', '--force-window=yes']
        if scenario == 'speed2x': args.append('--speed=2')
        if scenario == 'paused': args.append('--pause=yes')
        if scenario == 'ass': args.append('--sub-file=' + str(subtitle))
        return [executable, *args, '--', str(media)]
    if player == 'foxide':
        args = ['--headless', '--loop-playlist', '--benchmark-script', script]
        if presentation == 'native-window':
            args = ['--benchmark-script', script]
        if scenario == 'speed2x': args += ['--speed', '2']
        if scenario == 'paused': args.append('--paused')
        if scenario == 'ass': args += ['--subtitle', str(subtitle)]
        return [executable, *args, '--', str(media)]
    if player == 'vlc':
        args = ['--ignore-config', '--intf=dummy', '--vout=dummy', '--aout=dummy', '--avcodec-hw=none',
                '--codec=avcodec,none', '--verbose=2',
                '--repeat', '--no-video-title-show', '--extraintf=http', '--http-host=127.0.0.1',
                '--http-port=' + str(port), '--http-password=' + password]
        if presentation == 'native-window':
            args = [a for a in args if not a.startswith(('--intf=', '--vout=', '--aout=', '--avcodec-hw=', '--codec=', '--verbose='))] + ['--avcodec-hw=any']
        if sys.platform != 'darwin': args.append('--no-one-instance')
        if scenario == 'speed2x': args.append('--rate=2')
        if scenario == 'paused': args.append('--start-paused')
        if scenario == 'ass': args.append('--sub-file=' + str(subtitle))
        return [executable, *args, '--', str(media)]
    if player in ('mpc-hc', 'mpc-be'):
        return [executable, str(media), '/new', '/play']
    if player == 'kodi': return [executable, '-p', str(media)]
    if player == 'ffplay': return [executable, '-autoexit', '-i', str(media)]
    return [executable, str(media)]


def read_events(path):
    if not path.exists(): return []
    events = []
    for line in path.read_text('utf-8', errors='replace').splitlines():
        try: events.append(json.loads(line))
        except json.JSONDecodeError: events.append({'kind': 'collector-error'})
    return events


def stop_owned(process, records, snapshot):
    # Every PID is rechecked against the creation identity observed in this tree.
    for pid, record in sorted(records.items(), reverse=True):
        if pid == process.pid: continue
        if os.name == 'nt':
            from compare_players import terminate_owned
            terminate_owned(pid, record[0])
        else:
            current = snapshot(pid).get(pid)
            if current and current[0] == record[0]:
                try: os.kill(pid, signal.SIGTERM)
                except ProcessLookupError: pass
    if process.poll() is None:
        process.terminate()
    try: process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()


def collect(player, executable, scenario, media, subtitle, directory, settings, system, trial, media_hash=None, subtitle_hash=None, snapshot=None):
    directory.mkdir(parents=True, exist_ok=False)
    port, password = 0, ''
    if player == 'vlc':
        with socket.socket() as reservation:
            reservation.bind(('127.0.0.1', 0))
            port = reservation.getsockname()[1]
        password = secrets.token_urlsafe(18)
    presentation = settings.get('presentation', 'software-null')
    command = command_for(player, executable, scenario, media, subtitle, port, password, presentation)
    cohort = dict(settings, media_sha256=media_hash or digest(media), scenario=scenario,
                  mode=('native-default-window' if presentation == 'native-window' else 'software-null-output') if player in CORE else 'frontend-default-diagnostic')
    if scenario == 'ass': cohort['subtitle_sha256'] = subtitle_hash or digest(subtitle)
    profile = cohort['mode'] + '/' + scenario + '/' + hashlib.sha256(json.dumps(cohort, sort_keys=True).encode()).hexdigest()[:16]
    case = dict(player=player, scenario=scenario, platform=system, profile=profile, trial=trial,
                source=str(directory / 'case.json'), cohort=cohort, command=[a.replace(password, '<ephemeral>') if password else a for a in command],
                verified=False, status='pending', metrics={}, samples=[], playback_events=[], vlc_status=[],
                notes=['CPU 100% means one fully utilized logical core.',
                       'Steady sampling begins after verified playback readiness plus warmup. Cache state is uncontrolled.',
                       'RSS is a process-tree sum; shared pages may be counted twice.',
                       'VM null outputs do not exercise native GPU presentation, subtitle composition, display timing, or power.',
                       'mpv Lua / VLC HTTP observation overhead is included and differs by adapter.'])
    if presentation == 'native-window':
        case['notes'][3] = 'Native output at player-default window dimensions and audio route; hardware fallback is recorded, not normalized. Engine state is not physical frame presentation or power.'
    snapshot = snapshot or (windows_snapshot() if os.name == 'nt' else (linux_snapshot if system == 'linux' else mac_snapshot))
    env = dict(os.environ, FOXIDE_BENCH_EVENTS=str(directory / 'events.jsonl'), FOXIDE_BENCH_SEEKS='0')
    event_path = directory / 'events.jsonl'
    tail = EventTail(event_path)
    auth = base64.b64encode((':' + password).encode()).decode() if password else ''
    status_url = f'http://127.0.0.1:{port}/requests/status.json'
    previous, previous_time, owned, root_identity = {}, None, {}, None
    started = time.perf_counter()
    process = None
    next_sample, next_status, startup_proxy = started, started, None
    try:
        with (directory / 'stdout.log').open('wb') as out, (directory / 'stderr.log').open('wb') as err:
            started = time.perf_counter()
            process = subprocess.Popen(command, env=env, stdin=subprocess.DEVNULL, stdout=out, stderr=err)
            case['pid'] = process.pid
            ready_elapsed = None if player in CORE else 0.0
            deadline = started + (30 if player in CORE else settings['duration'])
            while time.perf_counter() < deadline:
                now = time.perf_counter()
                if process.poll() is not None:
                    case.update(status='early-exit', exit_code=process.returncode)
                    break
                if now >= next_sample:
                    records = snapshot(process.pid)
                    if process.pid in records:
                        identity = records[process.pid][0]
                        if root_identity is None: root_identity = identity
                        if identity != root_identity:
                            case['status'] = 'pid-reused'
                            break
                        owned.update(records)
                        common = (p for p in records if p in previous and records[p][0] == previous[p][0])
                        cpu = None if previous_time is None else sum(max(0, records[p][2]-previous[p][2]) for p in common) / (now-previous_time) * 100
                        case['samples'].append(dict(elapsed=now-started, rss=sum(r[1] for r in records.values()), cpu=cpu))
                        previous, previous_time = records, now
                    next_sample = now + .25
                if player in ('mpv', 'foxide') and startup_proxy is None:
                    events = tail.read()
                    if any(e.get('kind') == 'first-playback-restart-proxy' for e in events):
                        startup_proxy = (time.perf_counter()-started)*1000
                if player == 'vlc' and now >= next_status:
                    request = urllib.request.Request(status_url, headers={'Authorization': 'Basic '+auth})
                    try:
                        with urllib.request.urlopen(request, timeout=.15) as response:
                            case['vlc_status'].append(json.load(response))
                    except (OSError, ValueError): pass
                    next_status = now + .5
                if ready_elapsed is None and playback_verified(player, scenario, read_events(directory / 'events.jsonl'), case['vlc_status'], presentation):
                    ready_elapsed = time.perf_counter() - started
                    case['playback_ready_elapsed'] = ready_elapsed
                    deadline = started + ready_elapsed + settings['duration']
                time.sleep(.005 if player in ('mpv', 'foxide') and startup_proxy is None else .025)
            else:
                case['status'] = 'sampled' if ready_elapsed is not None else 'playback-readiness-timeout'
    except OSError as error:
        case.update(status='launch-error', error=str(error))
    finally:
        if process is not None: stop_owned(process, owned, snapshot)
    case['playback_events'] = read_events(directory / 'events.jsonl')
    verified = playback_verified(player, scenario, case['playback_events'], case['vlc_status'], presentation)
    case['verified'] = verified and case['status'] == 'sampled' and scenario != 'ass'
    if player == 'vlc' and presentation == 'software-null':
        decoder_log = (directory / 'stderr.log').read_text('utf-8', errors='replace') if (directory / 'stderr.log').exists() else ''
        case['software_decode_verified'] = vlc_software_decode_verified(decoder_log, scenario)
        case['verified'] = case['verified'] and case['software_decode_verified']
        if not case['software_decode_verified']:
            case['notes'].append('Software decoder not verified from VLC module logs; excluded from software medals.')
    if scenario == 'ass': case['notes'].append('Subtitle composition is disabled by null output; no ASS performance medal is eligible.')
    if case['status'] == 'sampled': case['status'] = 'verified-playback' if case['verified'] else 'sampled-playback-unverified'
    summary = summarize_samples(case['samples'], case.get('playback_ready_elapsed', 0), settings['warmup'])
    case['metrics'] = {'cpu_'+scenario: summary['cpu'], 'rss_'+scenario: summary['rss_mib']}
    if scenario == 'avc1080':
        case['metrics']['rss_peak'] = summary['rss_peak_mib']
        if player in ('mpv', 'foxide'): case['metrics']['startup_proxy'] = startup_proxy
    write_json(directory / 'case.json', case)
    return case


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--media-dir', type=Path, required=True)
    parser.add_argument('--output', type=Path, default=ROOT / 'results/local/events')
    parser.add_argument('--players', nargs='+')
    parser.add_argument('--presentation', choices=['software-null', 'native-window'], default='software-null')
    parser.add_argument('--order-seed', type=int, default=20261115)
    parser.add_argument('--scenarios', nargs='+', choices=list(SCENARIOS), default=list(SCENARIOS))
    parser.add_argument('--repeat', type=int, default=3)
    parser.add_argument('--duration', type=float, default=8)
    parser.add_argument('--warmup', type=float, default=2)
    parser.add_argument('--plan', action='store_true')
    parser.add_argument('--require-core', action='store_true')
    args = parser.parse_args()
    if args.presentation == 'native-window' and 'ass' in args.scenarios:
        args.scenarios.remove('ass')
        print('ASS composition requires a validated presentation adapter; excluded from native field.', flush=True)
    if 'ass' in args.scenarios and not (args.media_dir / 'subtitles.ass').is_file():
        parser.error('ASS requires an existing subtitles.ass fixture; select the six other scenarios for a subtitle-free corpus')
    import math
    if not (math.isfinite(args.duration) and math.isfinite(args.warmup) and 4 <= args.duration <= 600 and 0 <= args.warmup < args.duration and args.repeat >= 3):
        parser.error('Use >=3 repeats, 4..600 seconds duration, and finite 0 <= warmup < duration')
    system = 'windows' if os.name == 'nt' else 'macos' if sys.platform == 'darwin' else 'linux'
    catalog = json.loads((ROOT / 'benchmarks/features.json').read_text('utf-8'))
    args.players = set(args.players) if args.players else None
    if args.players and set(args.players) - {p['id'] for p in catalog['players']}:
        parser.error('Unknown player ID')
    manifest = json.loads((args.media_dir / 'manifest.json').read_text('utf-8'))
    media = {f['id']: args.media_dir / f['path'] for f in manifest['fixtures']}
    for fixture in manifest['fixtures']:
        if digest(args.media_dir / fixture['path']) != fixture['sha256']:
            parser.error('Fixture hash mismatch: ' + fixture['path'])
    media = {key: value.resolve() for key, value in media.items()}
    hashes = {f['id']: f['sha256'] for f in manifest['fixtures']}
    identities = {key: (stat.st_size, stat.st_mtime_ns) for key, value in media.items() for stat in [value.stat()]}
    collector_paths = [Path(__file__), ROOT / 'scripts/telemetry.py', ROOT / 'scripts/collection_io.py', ROOT / 'scripts/event_benchmarks.py', ROOT / 'crates/player-bench/assets/bench.lua']
    collector_digest = hashlib.sha256(''.join(digest(path) for path in collector_paths).encode()).hexdigest()
    subtitle = (args.media_dir / 'subtitles.ass').resolve()
    subtitle_hash = digest(subtitle) if subtitle.exists() else None
    snapshot = windows_snapshot() if os.name == 'nt' else (linux_snapshot if system == 'linux' else mac_snapshot)
    discovered = None
    if os.name == 'nt':
        from compare_players import inventory as windows_inventory
        discovered = {p['id']: p.get('executable') for p in windows_inventory(fingerprint=False)['players']}
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    session = dict(utc=datetime.now(timezone.utc).isoformat(), system=system, os=platform.platform(),
                   arch=platform.machine(), cpu_count=os.cpu_count(), host=platform.node(),
                   runner_image=os.environ.get('ImageVersion'), run_id=os.environ.get('GITHUB_RUN_ID'),
                   duration=args.duration, warmup=args.warmup, script_sha256=collector_digest,
                   presentation=args.presentation, source_commit=os.environ.get('GITHUB_SHA'),
                   order_seed=args.order_seed,
                   window_policy='Player default window; not fullscreen or normalized video surface dimensions' if args.presentation == 'native-window' else 'No audio/video output')
    cases, inventory, rows, jobs = [], [], [], []
    installed = {}
    installation = output / 'player-installation.json'
    if installation.exists():
        installed = {p['player']: p['status'] for p in json.loads(installation.read_text('utf-8'))['packages']}
    for player in catalog['players']:
        pid = player['id']
        if system not in player['platforms'] or args.players and pid not in args.players: continue
        executable = locate(pid, system, discovered)
        if pid == 'foxide' and args.presentation == 'native-window':
            desktop = ROOT / ('target/release/foxide-desktop' + ('.exe' if system == 'windows' else ''))
            executable = str(desktop) if desktop.is_file() else None
        entry = dict(player=pid, executable=executable, status='available' if executable else 'not-detected',
                     binary_sha256=digest(executable) if executable else None)
        inventory.append(entry)
        if not executable:
            entry['reason'] = 'No native executable found; packaged-app/account/Store activation may need a dedicated adapter.'
            if installed.get(pid) in ('installed', 'already-installed'):
                entry.update(status='installed-adapter-needed', reason='Package registration verified; native activation/executable discovery adapter is required.')
            print(pid + ': not detected', flush=True)
            continue
        scenarios = args.scenarios if pid in CORE else ['flac'] if player.get('media_types') == ['audio'] else ['avc1080']
        for scenario in scenarios:
            fixture = media.get(SCENARIOS[scenario])
            if fixture is None: continue
            for trial in range(args.repeat):
                jobs.append((pid, executable, scenario, fixture, trial))
    random.Random(args.order_seed).shuffle(jobs)
    for pid, executable, scenario, fixture, trial in jobs:
        if args.plan:
            cases.append(dict(player=pid, scenario=scenario, trial=trial, status='planned'))
            continue
        identity = (fixture.stat().st_size, fixture.stat().st_mtime_ns)
        if identity != identities[SCENARIOS[scenario]]:
            raise ValueError('Immutable fixture changed during session: ' + str(fixture))
        case = collect(pid, executable, scenario, fixture, subtitle, output / 'cases' / f'{pid}-{scenario}-{trial}', session, system, trial, hashes[SCENARIOS[scenario]], subtitle_hash, snapshot)
        if (fixture.stat().st_size, fixture.stat().st_mtime_ns) != identity:
            raise ValueError('Immutable fixture changed during trial: ' + str(fixture))
        cases.append(case)
        rows.extend(measurement_rows(case))
        print(f'{pid} / {scenario} / {trial+1}: {case["status"]}', flush=True)
        write_json(output / 'measurements.json', rows, compact=True)
    (output / 'measurements.json').write_text(json.dumps(rows, indent=2, allow_nan=False), encoding='utf-8')
    write_json(output / 'session.json', dict(session=session, inventory=inventory, cases=cases))
    if args.require_core and not args.plan:
        for pid in CORE:
            if sum(c.get('verified', False) for c in cases if c['player'] == pid and c['scenario'] == 'avc1080') < args.repeat:
                print(pid + ': required AVC playback verification failed', file=sys.stderr)
                return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
