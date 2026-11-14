"""Cross-platform event collection. Unverified frontends remain diagnostics."""
from __future__ import annotations
import statistics


def measurement_rows(case):
    return [dict(player=case['player'], event=event, value=value, platform=case['platform'],
                 profile=case['profile'], trial=case['trial'], verified=case['verified'],
                 source=case['source']) for event, value in case['metrics'].items()]


def playback_verified(player, scenario, events, statuses):
    if any(e.get('kind') == 'collector-error' or (e.get('kind') == 'end-file' and e.get('reason') == 'error') for e in events):
        return False
    if player in ('mpv', 'foxide'):
        stats = [e for e in events if e.get('kind') == 'stats']
        started = any(e.get('kind') == 'first-playback-restart-proxy' and
                      (scenario == 'flac' or e.get('video') not in (None, 'no', 'auto')) for e in events)
        hardware = [e['hwdec'] for e in stats if e.get('hwdec') not in (None, '')]
        if not started or not hardware or any(value != 'no' for value in hardware):
            return False
        if scenario == 'speed2x' and not all(e.get('speed') == 2 for e in stats if e.get('position') is not None):
            return False
        if scenario == 'paused':
            return all(e.get('paused') is True for e in stats)
        positions = [e['position'] for e in stats if isinstance(e.get('position'), (int, float))]
        return bool(positions and max(positions) - min(positions) >= 1)
    if player == 'vlc':
        kind = 'decodedaudio' if scenario == 'flac' else 'decodedvideo'
        decoded = any(s.get('stats', {}).get(kind, 0) > 0 for s in statuses)
        if scenario == 'speed2x' and not all(abs(s.get('rate', 0)-2) < .01 for s in statuses if s.get('state') == 'playing'):
            return False
        if scenario == 'paused':
            return decoded and any(s.get('state') == 'paused' for s in statuses)
        times = [s['time'] for s in statuses if s.get('state') == 'playing' and isinstance(s.get('time'), (int, float))]
        return decoded and bool(times and max(times) - min(times) >= 1)
    return False


def summarize_samples(samples, ready_elapsed, warmup):
    steady = [s for s in samples if s['elapsed'] >= ready_elapsed + warmup]
    cpu = [s['cpu'] for s in steady if s['cpu'] is not None]
    return dict(cpu=statistics.mean(cpu) if cpu else None,
                rss_mib=statistics.mean(s['rss'] for s in steady)/1048576 if steady else None,
                rss_peak_mib=max((s['rss'] for s in samples), default=0)/1048576 if samples else None)
