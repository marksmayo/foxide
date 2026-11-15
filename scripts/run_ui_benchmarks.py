"""Plan or collect native UI action evidence on a dedicated interactive desktop.

Default mode only writes a plan. --execute launches fresh owned processes and
invokes their native accessibility actions; it never sends media-engine IPC.
"""
from __future__ import annotations
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import locale
import os
from pathlib import Path
import platform
import socket
import subprocess
import sys
import time

from collection_io import write_json
from run_event_benchmarks import ROOT, digest, locate, stop_owned
from telemetry import windows_snapshot, linux_snapshot, mac_snapshot
from ui_benchmarks import action_cohort, action_rows, has_comparable_ui_measurements


def plan(catalog, system):
    rows = []
    for player in catalog['players']:
        if system not in player['platforms']:
            continue
        for action in catalog['actions']:
            status = 'adapter-required'
            if action['id'] in player.get('not_applicable', []) or not set(action['media_types']) & set(player['media_types']):
                status = 'not-applicable'
            elif action['verification'] == 'presented-frame':
                status = 'presentation-collector-required'
            elif action['id'] in player.get('platform_adapter_required', {}).get(system, []):
                status = 'adapter-required'
            elif action['collector'] == 'native-accessibility' and action['id'] in player['menus']:
                status = 'ready-for-interactive-validation'
            rows.append(dict(player=player['id'], action=action['id'], status=status))
    return rows


def command_for(player, executable):
    # No headless/null-output options. These are real GUI launches in idle state.
    vlc = ['--ignore-config']
    if platform.system() != 'Darwin':
        vlc += ['--no-one-instance', '--no-qt-privacy-ask']
    args = {'vlc': vlc, 'mpc-hc': ['/new'], 'mpc-be': ['/new']}
    return [executable, *args.get(player, [])]


def collect(player, executable, action, selectors, directory, settings, system, trial, snapshot):
    from ui_drivers import connect
    directory.mkdir(parents=True, exist_ok=False)
    case = dict(player=player, platform=system, action=action['id'], trial=trial,
                scope='native-ui', status='unverified', pid=0,
                source=str(directory / 'case.json'), input_method='accessibility-action',
                notes=['Native UI input/accessibility invocation to observed UI state; not physical display presentation.',
                       'Latency includes accessibility polling (10 ms interval) and adapter overhead.',
                       'CPU is process-tree CPU time across the action observation window; RSS is post-action, not peak.'])
    owned, identity, process, driver = {}, None, None, None
    try:
        with (directory / 'player.log').open('wb') as log:
            process = subprocess.Popen(command_for(player, executable), stdout=log, stderr=log)
            case['pid'] = process.pid
            deadline = time.monotonic() + 15
            driver = None
            last_attach_error = 'No owned process snapshot'
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    raise RuntimeError('New process exited; existing-instance attachment is forbidden')
                records = snapshot(process.pid)
                owned.update(records)
                record = records.get(process.pid)
                if record:
                    if identity is None:
                        identity = record[0]
                    if record[0] != identity:
                        raise RuntimeError('Owned process identity changed')
                    try:
                        candidate = connect(system, process.pid)
                        context = candidate.prepare()
                        driver = candidate
                        break
                    except (RuntimeError, StopIteration) as error:
                        last_attach_error = f'{type(error).__name__}: {error}'
                time.sleep(.1)
            if driver is None:
                raise RuntimeError('No native UI adapter attached within startup deadline: ' + last_attach_error)
            # Settle before the timed action; any popup must first be absent.
            time.sleep(1)
            if isinstance(selectors, dict):
                target_name, expected_name = selectors['target'], selectors['expected']
                target_roles = selectors.get('target_roles', {}).get(system, ['MenuItem', 'Button'])
                roles = selectors.get('expected_roles', {}).get(system, ['MenuItem'])
            else:
                target_name, expected_name = selectors
                target_roles = ['MenuItem', 'Button']
                roles = ['Window'] if action['id'] == 'open_dialog' else ['MenuItem']
            case['input_method'] = context['input_method']
            # Linux trials own fresh processes and use observed control bounds.
            if system != 'linux':
                driver.dismiss()
            time.sleep(.1)
            before = driver.observe(expected_name, roles)
            if before:
                raise RuntimeError('Postcondition already true before the action')
            target = driver.find(target_name, target_roles)
            write_json(directory / 'before.json', dict(pid=process.pid, expected=before, target=target.evidence()))
            before_cpu = snapshot(process.pid)
            owned.update(before_cpu)
            if before_cpu.get(process.pid, (None,))[0] != identity or process.poll() is not None:
                raise RuntimeError('Owned process disappeared before input')
            case['input_ns'] = time.perf_counter_ns()
            driver.activate(target)
            case['dispatch_completed_ns'] = time.perf_counter_ns()
            deadline = time.monotonic() + 5
            after = []
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    raise RuntimeError('Owned player exited during action')
                after = driver.observe(expected_name, roles)
                if len(after) == 1:
                    break
                time.sleep(.01)
            case['completed_ns'] = time.perf_counter_ns()
            after_cpu = snapshot(process.pid)
            owned.update(after_cpu)
            if len(after) != 1 or after_cpu.get(process.pid, (None,))[0] != identity:
                raise RuntimeError('Owned visible postcondition was not verified')
            write_json(directory / 'after.json', dict(pid=process.pid, expected=after))
            cohort = action_cohort(settings, context, action['id'])
            case['window_bounds'] = context.get('window_bounds')
            case.update(cohort=cohort, profile='native-ui/' + action['id'] + '/' + hashlib.sha256(
                json.dumps(cohort, sort_keys=True).encode()).hexdigest()[:16],
                status='verified', verified=True, before=str(directory / 'before.json'), after=str(directory / 'after.json'),
                verification=dict(kind=action['verification'], condition=action['condition'], passed=True, pid=process.pid))
            # Omit CPU when the observed tree changed: newly created children
            # cannot be compared against nonexistent baseline counters.
            if before_cpu.keys() == after_cpu.keys() and all(before_cpu[p][0] == after_cpu[p][0] for p in before_cpu):
                delta = sum(after_cpu[p][2] - before_cpu[p][2] for p in before_cpu)
                if delta >= 0:
                    case['cpu_time_ms'] = delta * 1000
            case['rss_before_mib'] = sum(record[1] for record in before_cpu.values()) / 1048576
            case['rss_mib'] = sum(record[1] for record in after_cpu.values()) / 1048576
    except Exception as error:
        case.update(status='unverified', error=f'{type(error).__name__}: {error}')
        if driver is not None:
            try:
                case['visible_controls'] = [node.evidence() for node in driver.nodes() if node.visible][:80]
            except Exception as diagnostic_error:
                case['diagnostic_error'] = str(diagnostic_error)
    finally:
        if process is not None:
            stop_owned(process, owned, snapshot)
        write_json(directory / 'case.json', case)
    return case


def collect_batch(player, executable, action, selectors, directory, settings, system, repeats, snapshot):
    cases = []
    for trial in range(repeats):
        case = collect(player, executable, action, selectors,
                       directory / f'{player}-{action["id"]}-{trial}', settings, system, trial, snapshot)
        cases.append(case)
        if case.get('verified') is not True:
            # A failed trial invalidates this whole batch. Further launches
            # cannot restore eligibility; retain the failure and try the next action.
            break
    rows = []
    if len(cases) == repeats and all(c.get('verified') is True for c in cases):
        for case in cases:
            rows.extend(action_rows(case, action))
        # Optional resource counters must cover the entire batch too. A missing
        # CPU/RSS sample must not produce medals from the successful subset.
        counts = Counter(row['event'] for row in rows)
        rows = [row for row in rows if counts[row['event']] == repeats]
    return cases, rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--execute', action='store_true', help='Run only on an isolated logged-in interactive desktop')
    parser.add_argument('--require-comparable', action='store_true', help='Fail if no two players have eligible measurements in a shared UI cohort')
    parser.add_argument('--players', nargs='+')
    parser.add_argument('--actions', nargs='+', default=['menu_media', 'menu_playback', 'menu_audio', 'menu_subtitles', 'open_dialog'])
    parser.add_argument('--repeats', type=int, default=10)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--catalog', type=Path, default=ROOT / 'benchmarks/ui-events.json')
    parser.add_argument('--foxide', type=Path, help='Explicit release desktop binary')
    parser.add_argument('--environment-label', help='Fixed display/DPI/theme/session description; required for execution')
    args = parser.parse_args()
    if args.repeats < 3:
        parser.error('At least three repeats are required')
    if args.require_comparable and not args.execute:
        parser.error('--require-comparable requires --execute')
    if args.execute and not args.environment_label:
        parser.error('--execute requires --environment-label (screen, DPI, theme and desktop session)')
    system = {'Windows': 'windows', 'Darwin': 'macos', 'Linux': 'linux'}[platform.system()]
    catalog = json.loads(args.catalog.read_text('utf-8'))
    rows = plan(catalog, system)
    players = {p['id']: p for p in catalog['players']}
    actions = {a['id']: a for a in catalog['actions']}
    if set(args.players or []) - players.keys() or set(args.actions) - actions.keys():
        parser.error('Unknown player or action')
    selected = [r for r in rows if (not args.players or r['player'] in args.players) and r['action'] in args.actions]
    args.output.mkdir(parents=True, exist_ok=False)
    write_json(args.output / 'plan.json', dict(platform=system, actions=rows, execute=args.execute))
    settings = dict(host=socket.gethostname(), os=platform.platform(), locale=locale.getlocale(),
                    environment=args.environment_label, adapter_sha256=digest(ROOT / 'scripts/ui_drivers.py'),
                    runner_sha256=digest(__file__), catalog_sha256=digest(args.catalog))
    measurements, cases = [], []
    snapshot = windows_snapshot() if system == 'windows' else linux_snapshot if system == 'linux' else mac_snapshot
    discovered = None
    if args.execute and system == 'windows':
        from compare_players import inventory
        discovered = {p['id']: p.get('executable') for p in inventory(fingerprint=False)['players']}
    for entry in selected:
        if not args.execute or entry['status'] != 'ready-for-interactive-validation':
            cases.append(dict(entry, scope='native-ui'))
            continue
        player, action_id = entry['player'], entry['action']
        executable = (str(args.foxide or ROOT / ('target/release/foxide-desktop' + ('.exe' if system == 'windows' else '')))
                      if player == 'foxide' else locate(player, system, discovered))
        if not executable or not Path(executable).is_file():
            cases.append(dict(entry, status='not-installed', scope='native-ui'))
            continue
        batch, rows = collect_batch(player, executable, actions[action_id], players[player]['menus'][action_id],
                                    args.output, settings, system, args.repeats, snapshot)
        cases.extend(batch)
        measurements.extend(rows)
        print(f'{player} / {action_id}: {len(batch)}/{args.repeats} trials; '
              f'{sum(c.get("verified") is True for c in batch)} verified; {len(rows)} metric rows', flush=True)
        for failed in (c for c in batch if c.get('error')):
            print(failed['error'], flush=True)
            if failed.get('visible_controls'):
                print('Visible controls: ' + json.dumps(failed['visible_controls']), flush=True)
    write_json(args.output / 'measurements.json', measurements)
    write_json(args.output / 'session.json', dict(scope='native-ui',
               session=dict(system=system, host=socket.gethostname(), utc=datetime.now(timezone.utc).isoformat(),
                            source_commit=os.environ.get('GITHUB_SHA'),
                            run_id=os.environ.get('GITHUB_RUN_ID'), presentation='native-ui'),
               inventory=[], cases=cases, settings=settings))
    print(f'{len(cases)} action outcomes; {len(measurements)} verified metric rows. Output: {args.output}')
    if args.require_comparable and not has_comparable_ui_measurements(measurements):
        print('No comparable verified native UI measurements; evidence retained without medals.', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
