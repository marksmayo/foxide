"""GitHub-hosted VM smoke measurements, kept separate from physical-player medals.

mpv/VLC run software decode with null outputs. IINA gets a GUI frontend diagnostic
on macOS; process lifetime does not establish playback or displayed TTFF.
"""
from __future__ import annotations
import argparse
from datetime import datetime, timezone
import html
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def windows_mpv_candidates():
    roots = [Path(os.environ.get('ProgramFiles', 'C:/Program Files')) / 'WinGet/Links']
    if os.environ.get('LOCALAPPDATA'):
        roots.append(Path(os.environ['LOCALAPPDATA']) / 'Microsoft/WinGet/Links')
    return [root / 'mpv.exe' for root in roots]


def locate(player):
    suffix = '.exe' if os.name == 'nt' else ''
    if player == 'iina':
        path = Path('/Applications/IINA.app/Contents/MacOS/IINA')
        return path if path.is_file() else None
    candidates = []
    if player == 'mpv':
        candidates.append(ROOT / ('tools/mpv/mpv' + suffix))
        if os.name == 'nt':
            # Elevated WinGet installs on hosted Windows VMs use machine scope.
            candidates.extend(windows_mpv_candidates())
    if player == 'vlc':
        candidates += [Path('/Applications/VLC.app/Contents/MacOS/VLC'),
                       Path(os.environ.get('ProgramFiles', 'C:/Program Files')) / 'VideoLAN/VLC/vlc.exe']
    found = shutil.which(player)
    if found:
        candidates.append(Path(found))
    return next((path for path in candidates if path.is_file()), None)


def specs():
    result = [('mpv', 'mpv', ['--arg=--vo=null', '--arg=--ao=null', '--arg=--hwdec=no']),
              ('vlc', 'vlc', ['--arg=--intf=dummy', '--arg=--vout=dummy', '--arg=--aout=dummy', '--arg=--avcodec-hw=none'])]
    if sys.platform == 'darwin':
        result.append(('iina', 'external', []))
    return result


def build_reports(record, output):
    rows = []
    for case in record['cases']:
        runs = [json.loads(path.read_text('utf-8')) for path in (output / case['player']).rglob('run.json')]
        peak = max((r['rss_peak_bytes'] for r in runs if r.get('rss_peak_bytes') is not None), default=None)
        rows.append([case['player'], case['status'], str(len(runs)),
                     f'{peak/1048576:.2f}' if peak is not None else 'N/A', case['scope']])
    text = '# GitHub-hosted VM smoke measurements\n\n'
    text += f"Platform: {record['os']} / {record['arch']}. Run: {record['run_url'] or 'local validation'}.\n\n"
    text += 'These are CI diagnostics, excluded from the physical-machine medal report. mpv/VLC use software decoding and null audio/video outputs. IINA is a GUI launch diagnostic; alive does not mean playing. No GPU, battery, displayed-TTFF or cross-platform winner is claimed.\n\n'
    text += '| Player | Outcome | Recorded runs | Max sampled RSS MiB | Scope |\n|---|---|---:|---:|---|\n'
    for row in rows:
        text += '| ' + ' | '.join(value.replace('|', '\\|').replace('\n', ' ') for value in row) + ' |\n'
    (output / 'summary.md').write_text(text, encoding='utf-8')
    escaped_rows = ''.join('<tr>' + ''.join('<td>' + html.escape(value) + '</td>' for value in row) + '</tr>' for row in rows)
    page = '<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Foxide — hosted CI diagnostics</title><style>body{font:15px system-ui;margin:45px auto;padding:20px;max-width:1100px;color:#263346;background:#f6f7fa}h1{font-size:36px;letter-spacing:-1px}aside{background:#fff0df;padding:20px;border-radius:10px;line-height:1.7}table{border-collapse:collapse;background:white;width:100%;margin-top:25px}td,th{padding:15px;border-bottom:1px solid #e3e7ee;text-align:left}a{color:#345daf}.scroll{overflow:auto}</style><h1>Foxide / hosted CI diagnostics</h1>'
    page += '<aside>Separate CI smoke dataset. mpv/VLC use software decoding with null outputs. IINA receives a GUI launch diagnostic. These results cannot award native-player, power, GPU or displayed-startup medals.</aside>'
    page += '<p>' + html.escape(record['os'] + ' / ' + record['arch']) + '</p><div class="scroll"><table><thead><tr><th>Player</th><th>Outcome</th><th>Recorded runs</th><th>Max RSS MiB</th><th>Scope</th></tr></thead><tbody>' + escaped_rows + '</tbody></table></div><p><a href="hosted.json">Raw session manifest</a> · <a href="summary.md">Markdown summary</a></p></html>'
    (output / 'report.html').write_text(page, encoding='utf-8')


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--media', required=True, type=Path)
    p.add_argument('--output', type=Path, default=ROOT / 'results/local/hosted')
    p.add_argument('--plan', action='store_true', help='Write commands only; does not launch players')
    args = p.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    bench = ROOT / 'target/release' / ('player-bench.exe' if os.name == 'nt' else 'player-bench')
    record = {'schema_version': 1, 'measurement_class': 'github-hosted-vm-software-decode' if os.environ.get('GITHUB_ACTIONS') == 'true' else 'local-null-output-validation',
              'utc': datetime.now(timezone.utc).isoformat(), 'os': platform.platform(), 'arch': platform.machine(),
              'cpu_count': os.cpu_count(), 'image_version': os.environ.get('ImageVersion'),
              'run_url': f"https://github.com/{os.environ['GITHUB_REPOSITORY']}/actions/runs/{os.environ['GITHUB_RUN_ID']}" if os.environ.get('GITHUB_RUN_ID') else None,
              'native_medal_eligible': False, 'cases': []}
    if not args.plan and (not args.media.is_file() or not bench.is_file()):
        p.error('Build player-bench and generate the media fixture first')
    failed = False
    for player, adapter, options in specs():
        executable = locate(player)
        scope = 'GUI frontend launch only; playback unverified' if player == 'iina' else 'Software decode / null outputs'
        case = {'player': player, 'scope': scope, 'executable': str(executable) if executable else None}
        command = [str(bench), 'run', '--player', adapter, '--exe', str(executable or player),
                   '--media', str(args.media.resolve()), '--duration', '8', '--warmup', '2',
                   '--repeat', '1' if player == 'iina' else '3', '--cache', 'warm', '--output', str(output / player)] + options
        case['command'] = command
        if args.plan:
            case['status'] = 'planned'
        elif executable is None:
            case['status'] = 'missing dependency'
            failed = True
        else:
            try:
                with (output / (player + '-runner.log')).open('w', encoding='utf-8') as log:
                    result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=100)
                case['exit_code'] = result.returncode
                case['status'] = ('process sampled; playback unverified' if player == 'iina' else 'smoke run completed') if result.returncode == 0 else 'smoke / display session failed'
                if result.returncode != 0 and player != 'iina':
                    failed = True
            except (OSError, subprocess.TimeoutExpired) as exc:
                case.update(status='runner error', error=str(exc))
                failed |= player != 'iina'
        record['cases'].append(case)
        print(player + ': ' + case['status'], flush=True)
        (output / 'hosted.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
    build_reports(record, output)
    print(output / 'summary.md')
    return 1 if failed else 0


if __name__ == '__main__':
    sys.exit(main())
