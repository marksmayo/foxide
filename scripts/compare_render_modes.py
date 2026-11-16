"""Isolated Windows renderer A/B collection. Diagnostics only; never awards medals."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import random
import re
import statistics
import subprocess

from run_event_benchmarks import collect, digest

def probe(executable, fixture, mode, directory, expected_hw):
    env = dict(os.environ, FOXIDE_RENDER_FIXTURE=str(fixture))
    if mode == 'default':
        env.pop('FOXIDE_ANGLE_RUNTIME', None)
    result = subprocess.run([str(executable), '--exact',
        'gpu::integration::composited_video_is_upright_and_advances_at_video_cadence',
        '--ignored', '--nocapture', '--test-threads=1'], env=env,
        capture_output=True, text=True, timeout=45)
    log = result.stdout + result.stderr
    (directory / (mode + '-pixels.log')).write_text(log, encoding='utf-8')
    if result.returncode or not re.search(r'unique_composited_frames=\d+', log):
        raise RuntimeError(mode + ': compositor pixel acceptance failed')
    if re.search(r'swiftshader|llvmpipe|softpipe|software rasterizer|Microsoft Basic Render|WARP', log, re.I):
        raise RuntimeError(mode + ': software GPU detected; hardware comparison refused')
    if mode == 'angle':
        if not re.search(r'GL_VERSION=OpenGL ES [3-9]', log):
            raise RuntimeError('ANGLE did not provide GLES3 or newer')
        match = re.search(r'decoder=([^\s]+)', log)
        if not match or match[1] != expected_hw:
            raise RuntimeError('ANGLE direct decoder not observed')

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--probe', type=Path, required=True)
    parser.add_argument('--pixel-fixture', type=Path, required=True)
    parser.add_argument('--media-dir', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--repeats', type=int, default=5)
    parser.add_argument('--duration', type=float, default=30)
    parser.add_argument('--warmup', type=float, default=5)
    args = parser.parse_args()
    if os.name != 'nt':
        parser.error('Requires an isolated Windows GPU desktop runner')
    if args.repeats < 3 or not 4 <= args.duration <= 600 or not 0 <= args.warmup < args.duration:
        parser.error('Use at least3 repeats and 4..600 seconds with warmup below duration')
    runtime = os.environ.get('FOXIDE_ANGLE_RUNTIME')
    if not runtime or not Path(runtime).is_absolute():
        parser.error('Set FOXIDE_ANGLE_RUNTIME to an absolute trusted runtime directory')
    args.output.mkdir(parents=True, exist_ok=False)
    manifest = json.loads((args.media_dir / 'manifest.json').read_text('utf-8'))
    fixture = next(f for f in manifest['fixtures'] if f['id'] == 'B')
    media = (args.media_dir / fixture['path']).resolve()
    if digest(media) != fixture['sha256']:
        parser.error('Fixture hash mismatch')
    identity = (media.stat().st_size, media.stat().st_mtime_ns)
    session = dict(utc=datetime.now(timezone.utc).isoformat(), source_commit=os.environ.get('GITHUB_SHA'),
        run_id=os.environ.get('GITHUB_RUN_ID'), presentation='native-window',
        duration=args.duration, warmup=args.warmup, binary_sha256=digest(args.binary),
        runtime_sha256={n:digest(Path(runtime)/n) for n in ('libEGL.dll','libGLESv2.dll')},
        fixture_sha256=fixture['sha256'], order_seed=20261116,
        scope='Exploratory same-runner A/B; engine counters are not scanout proof; no medals')
    (args.output / 'session.json').write_text(json.dumps(session, indent=2), encoding='utf-8')
    for mode in ('default', 'angle'):
        for trial in range(3):
            directory = args.output / ('acceptance-' + str(trial))
            directory.mkdir(exist_ok=True)
            probe(args.probe.resolve(), args.pixel_fixture.resolve(), mode, directory, 'dxva2')
    jobs = [(mode, trial) for mode in ('default', 'angle') for trial in range(args.repeats)]
    random.Random(session['order_seed']).shuffle(jobs)
    cases = []
    try:
        for mode, trial in jobs:
            if mode == 'angle':
                os.environ['FOXIDE_ANGLE_RUNTIME'] = runtime
            else:
                os.environ.pop('FOXIDE_ANGLE_RUNTIME', None)
            if (media.stat().st_size, media.stat().st_mtime_ns) != identity:
                raise RuntimeError('Fixture changed during collection')
            case = collect('foxide', str(args.binary.resolve()), 'hevc4k', media, None,
                args.output / (mode + '-' + str(trial)), dict(session, mode=mode), 'windows', trial)
            case['mode'] = mode
            cases.append(case)
    finally:
        os.environ['FOXIDE_ANGLE_RUNTIME'] = runtime
        (args.output / 'raw.json').write_text(json.dumps(dict(session=session, cases=cases), indent=2), encoding='utf-8')
    summary = {}
    for mode in ('default', 'angle'):
        selected = [c for c in cases if c['mode'] == mode and c['verified']]
        if len(selected) != args.repeats:
            raise RuntimeError(mode + ': missing verified trials; raw evidence retained')
        summary[mode] = dict(trials=len(selected),
            cpu_median=statistics.median(c['metrics']['cpu_hevc4k'] for c in selected),
            rss_mib_median=statistics.median(c['metrics']['rss_hevc4k'] for c in selected))
    (args.output / 'summary.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
    lines = ['# Isolated Windows renderer comparison', '',
        'Same executable, fixture and runner; randomized sequential trials after pixel acceptance.',
        'CPU and RSS are process-tree observations. Frame delivery may differ; these are diagnostic results, with no medals or battery claim.', '',
        '| Mode | Trials | Median CPU (% of one core) | Median RSS (MiB) |',
        '|---|---:|---:|---:|']
    for mode, value in summary.items():
        lines.append(f"| {mode} | {value['trials']} | {value['cpu_median']:.2f} | {value['rss_mib_median']:.2f} |")
    (args.output / 'summary.md').write_text('\n'.join(lines)+'\n', encoding='utf-8')
    print(json.dumps(summary, indent=2))

if __name__ == '__main__':
    main()
