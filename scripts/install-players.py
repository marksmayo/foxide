"""Install the requested Windows player field with exact WinGet IDs and audit logs.

Without --install this only writes a plan. A package is called installed only
after WinGet list verifies it. A package-manager failure is never a success.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
FATAL_CODES = {0x8A150001}  # APPINSTALLER_CLI_ERROR_INTERNAL_ERROR


def install_command(winget, package):
    return [winget, 'install', '--exact', '--id', package['id'], '--source', package['source'],
            '--silent', '--disable-interactivity', '--accept-source-agreements', '--accept-package-agreements']


def run(command, timeout=600):
    try:
        result = subprocess.run(command, capture_output=True, text=True, encoding='utf-8', errors='replace', timeout=timeout)
        return {'command': command, 'exit_code': result.returncode,
                'exit_hex': f'0x{result.returncode & 0xffffffff:08X}',
                'stdout': result.stdout, 'stderr': result.stderr}
    except subprocess.TimeoutExpired:
        return {'command': command, 'exit_code': None, 'error': f'Timed out after {timeout}s'}
    except OSError as error:
        return {'command': command, 'exit_code': None, 'error': str(error)}


def install_packages(packages, winget, execute=False, invoke=run, detected=None):
    results = []
    manager_failure = None
    for package in packages:
        entry = {**package, 'command': install_command(winget, package)}
        if execute and package['player'] in (detected or {}):
            entry.update(status='already-present-executable', executable=detected[package['player']])
        elif not execute:
            entry['status'] = 'planned'
        elif manager_failure:
            entry.update(status='blocked-package-manager', reason=manager_failure)
        else:
            check = invoke([winget, 'list', '--exact', '--id', package['id'], '--source', package['source'],
                            '--disable-interactivity', '--accept-source-agreements'])
            entry['before'] = check
            code = check.get('exit_code')
            if code is not None and (code & 0xffffffff) in FATAL_CODES:
                # Record one actual install invocation too, to distinguish a broken
                # 'list' command from a package manager unable to install anything.
                entry['attempt'] = invoke(entry['command'])
                attempt_code = entry['attempt'].get('exit_code')
                if attempt_code == 0:
                    verified = invoke([winget, 'list', '--exact', '--id', package['id'], '--source', package['source'], '--disable-interactivity'])
                    entry['after'] = verified
                    entry['status'] = 'installed' if verified.get('exit_code') == 0 else 'install-unverified'
                else:
                    manager_failure = 'WinGet could not initialize/install; see the recorded exit code and logs.'
                    entry.update(status='blocked-package-manager', reason=manager_failure)
            elif code == 0:
                entry['status'] = 'already-installed'
            else:
                entry['attempt'] = invoke(entry['command'])
                if entry['attempt'].get('exit_code') == 0:
                    entry['after'] = invoke([winget, 'list', '--exact', '--id', package['id'], '--source', package['source'], '--disable-interactivity'])
                    entry['status'] = 'installed' if entry['after'].get('exit_code') == 0 else 'install-unverified'
                else:
                    entry['status'] = 'install-failed'
        results.append(entry)
        print(f"{package['player']}: {entry['status']}", flush=True)
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--install', action='store_true')
    parser.add_argument('--players', nargs='+', help='Catalog player IDs; default is every Windows package')
    parser.add_argument('--output', type=Path, default=ROOT / 'results/local/player-installation.json')
    args = parser.parse_args()
    catalog = json.loads((ROOT / 'benchmarks/installers.json').read_text('utf-8'))
    if args.install and os.name != 'nt':
        parser.error('WinGet installation requires Windows')
    known = {p['player'] for p in catalog['windows'] + catalog['manual']}
    if args.players and not set(args.players) <= known:
        parser.error('Unknown player IDs: ' + ', '.join(sorted(set(args.players) - known)))
    packages = [p for p in catalog['windows'] if not args.players or p['player'] in args.players]
    winget = shutil.which('winget') or 'winget'
    detected = {}
    if args.install:
        from compare_players import inventory
        detected = {p['id']: p['executable'] for p in inventory(fingerprint=False)['players'] if p.get('executable')}
    entries = install_packages(packages, winget, execute=args.install, detected=detected)
    record = {'schema_version': 1, 'utc': datetime.now(timezone.utc).isoformat(),
              'mode': 'install' if args.install else 'plan', 'packages': entries,
              'manual_or_other_platform': [p for p in catalog['manual'] if not args.players or p['player'] in args.players]}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, indent=2), encoding='utf-8')
    print(args.output)
    if args.install and any(p['status'] not in ('installed', 'already-installed', 'already-present-executable') for p in entries):
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
