"""Create a self-contained offline HTML report from actual Windows comparison trials."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import statistics
from collections import defaultdict

ROOT = Path(__file__).resolve().parents[1]
METRICS = [
    {'id': 'memory', 'key': 'rss_steady_bytes', 'label': 'Steady resident RAM', 'unit': 'MiB', 'scale': 1048576, 'decimals': 1, 'ranked': True,
     'method': 'Median of per-trial steady-state process-tree resident-set means. First five seconds excluded in the default run.'},
    {'id': 'peak', 'key': 'rss_peak_bytes', 'label': 'Peak resident RAM', 'unit': 'MiB', 'scale': 1048576, 'decimals': 1, 'ranked': True,
     'method': 'Median of each trial’s maximum sampled resident set. Sampling can miss short allocation peaks.'},
    {'id': 'cpu', 'key': 'cpu_steady_percent', 'label': 'CPU cost', 'unit': '% of one core', 'scale': 1, 'decimals': 2, 'ranked': True,
     'method': 'Median of per-trial steady-state process-tree CPU means. 100% represents one logical CPU; values may exceed 100%.'},
    {'id': 'gpu', 'key': 'gpu_dedicated_steady_bytes', 'label': 'Dedicated GPU allocation', 'unit': 'MiB', 'scale': 1048576, 'decimals': 1, 'ranked': True,
     'method': 'PDH GPU Process Memory Dedicated Usage, summed over this process tree and GPUs. Allocation is not proof of resident VRAM. Shared allocation is shown separately.'},
    {'id': 'window', 'key': 'window_ready_ms', 'label': 'Warm window readiness', 'unit': 'ms', 'scale': 1, 'decimals': 1, 'ranked': True,
     'method': 'Spawn-to-first-visible non-console process-owned window. Polling is about 10 ms but can be delayed by process/GPU collection. This is a diagnostic proxy, not time to first video frame.'},
    {'id': 'gpu-shared', 'key': 'gpu_shared_steady_bytes', 'label': 'Shared GPU allocation', 'unit': 'MiB', 'scale': 1048576, 'decimals': 1, 'ranked': True,
     'method': 'PDH GPU Process Memory Shared Usage. Common on integrated GPUs. CPU RSS and shared GPU allocations may overlap; do not add them as independent memory totals.'},
    {'id': 'cold-ttff', 'key': 'cold_ttff_ms', 'label': 'Cold display TTFF', 'unit': 'ms', 'scale': 1, 'decimals': 1, 'ranked': False,
     'method': 'Unmeasured: requires controlled cold cache/reboot and content-aware display observation.'},
    {'id': 'warm-ttff', 'key': 'ttff_display_ms', 'label': 'Warm display TTFF', 'unit': 'ms', 'scale': 1, 'decimals': 1, 'ranked': False,
     'method': 'Unmeasured: a window or playback-start event does not prove that video content reached the display.'},
    {'id': 'seek', 'key': 'seek_display_ms', 'label': 'Seek-to-displayed-frame', 'unit': 'ms', 'scale': 1, 'decimals': 1, 'ranked': False,
     'method': 'Unmeasured: no comparable input-to-content-presentation collector for the available players.'},
    {'id': 'power', 'key': 'power_watts', 'label': 'Package power / battery', 'unit': 'W', 'scale': 1, 'decimals': 2, 'ranked': False,
     'method': 'Unmeasured: no calibrated package-energy/power sensor, matched idle window or ten-minute power trial. CPU percentage is not Watts.'},
    {'id': 'pacing', 'key': 'frame_pacing_ms', 'label': 'Displayed frame pacing', 'unit': 'ms', 'scale': 1, 'decimals': 2, 'ranked': False,
     'method': 'Unmeasured: engine logs do not provide a common displayed-content frame-drop or pacing census.'},
    {'id': 'footprint', 'key': 'installed_bytes', 'label': 'Complete installed footprint', 'unit': 'MiB', 'scale': 1048576, 'decimals': 1, 'ranked': False,
     'method': 'Unranked: complete dependency inventories are not available for the selected field. A tiny launcher executable cannot count as a complete player.'},
]


def eligible(trial, settings):
    return (not trial.get('prime') and trial.get('status') == 'sampled'
            and trial.get('playback_evidence', {}).get('verified') is True
            and trial.get('duration_seconds') == settings['duration']
            and trial.get('warmup_seconds') == settings['warmup'])


def summarize(payload):
    profiles = payload.get('media', {}).get('fixtures', [])
    summary = {}
    trials_by_fixture_player = defaultdict(list)
    # Preserve trial order and eligibility rules while avoiding a full scan for
    # every fixture/player pair.
    for trial in payload['trials']:
        trials_by_fixture_player[trial['fixture'], trial['player']].append(trial)
    for profile in profiles:
        player_values = {}
        for player in payload['catalog']['players']:
            trials = [t for t in trials_by_fixture_player[profile['id'], player['id']] if eligible(t, payload['settings'])]
            metrics = {}
            for metric in METRICS:
                values = [t.get(metric['key']) for t in trials if isinstance(t.get(metric['key']), (int, float))]
                # Incomplete metric coverage never silently competes with full trials.
                complete = len(values) == len(trials) and len(values) >= 3 and metric['ranked']
                if values:
                    sorted_values = sorted(values)
                    metrics[metric['id']] = {'median': statistics.median(values) / metric['scale'],
                        'min': min(values) / metric['scale'], 'max': max(values) / metric['scale'],
                        'p95': sorted_values[max(0, __import__('math').ceil(.95 * len(values)) - 1)] / metric['scale'],
                        'n': len(values), 'eligible': complete}
            if trials:
                player_values[player['id']] = {'n': len(trials), 'metrics': metrics,
                    'trial_ids': [t['id'] for t in trials],
                    'hardware': sorted({t['playback_evidence'].get('hardware', 'Unknown') for t in trials})}
        rankings = {}
        for metric in METRICS:
            entries = [{'player': pid, **data['metrics'][metric['id']]} for pid, data in player_values.items()
                       if data['metrics'].get(metric['id'], {}).get('eligible')]
            # A one-player field never awards itself a gold medal.
            if len(entries) >= 2:
                entries.sort(key=lambda entry: entry['median'])
                rank, previous = 0, None
                for index, entry in enumerate(entries):
                    quantized = round(entry['median'], metric['decimals'])
                    if previous is None or quantized != previous:
                        rank = index + 1
                    entry['rank'] = rank
                    entry['medal'] = {1: 'gold', 2: 'silver', 3: 'bronze'}.get(rank)
                    previous = quantized
                rankings[metric['id']] = entries
            else:
                rankings[metric['id']] = []
        summary[profile['id']] = {'players': player_values, 'rankings': rankings}
    return summary


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('input', type=Path)
    p.add_argument('--output', type=Path)
    args = p.parse_args()
    payload = json.loads(args.input.read_text('utf-8'))
    if payload.get('schema_version') != 1:
        p.error('Unsupported comparison schema')
    view = {**payload, 'metrics': METRICS, 'summary': summarize(payload)}
    # Keep useful time series embedded, with the raw 250 ms samples in comparison.json.
    view['trials'] = [{**t, 'samples': t.get('samples', [])[::2]} for t in payload['trials']]
    data = json.dumps(view, separators=(',', ':'), ensure_ascii=True, allow_nan=False).replace('<', '\\u003c').replace('>', '\\u003e').replace('&', '\\u0026')
    template = (ROOT / 'reports/windows-comparison.template.html').read_text('utf-8')
    html = template.replace('<!-- REPORT_DATA -->', data)
    destination = args.output or args.input.with_name('report.html')
    destination.write_text(html, encoding='utf-8')
    summary_path = destination.with_name('summary.json')
    summary_path.write_text(json.dumps(view['summary'], indent=2), encoding='utf-8')
    print(f'{destination} ({destination.stat().st_size:,} bytes)')


if __name__ == '__main__':
    main()
