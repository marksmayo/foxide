"""Cross-platform event collection. Unverified frontends remain diagnostics."""
from __future__ import annotations
import statistics
import re


def measurement_rows(case):
    return [dict(player=case['player'], event=event, value=value, platform=case['platform'],
                 profile=case['profile'], trial=case['trial'], verified=case['verified'],
                 source=case['source']) for event, value in case['metrics'].items()]


def playback_verified(player, scenario, events, statuses, presentation='software-null'):
    if presentation not in ('software-null', 'native-window'):
        return False
    if any(e.get('kind') == 'collector-error' or (e.get('kind') == 'end-file' and e.get('reason') == 'error') for e in events):
        return False
    if player in ('mpv', 'foxide'):
        stats = [e for e in events if e.get('kind') == 'stats']
        started = any(e.get('kind') == 'first-playback-restart-proxy' and
                      (scenario == 'flac' or e.get('video') not in (None, 'no', 'auto')) for e in events)
        hardware = [e['hwdec'] for e in stats if e.get('hwdec') not in (None, '')]
        if any(not isinstance(value, str) or value.startswith('auto') for value in hardware):
            return False
        if not started or not hardware or (presentation == 'software-null' and any(value != 'no' for value in hardware)):
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
    threshold = ready_elapsed + warmup
    has_steady = any(s['elapsed'] >= threshold for s in samples)
    has_cpu = any(s['elapsed'] >= threshold and s['cpu'] is not None for s in samples)
    return dict(cpu=statistics.mean(s['cpu'] for s in samples if s['elapsed'] >= threshold and s['cpu'] is not None) if has_cpu else None,
                rss_mib=statistics.mean(s['rss'] for s in samples if s['elapsed'] >= threshold) / 1048576 if has_steady else None,
                rss_peak_mib=max((s['rss'] for s in samples), default=0) / 1048576 if samples else None)


def vlc_software_decode_verified(log, scenario):
    """Require selected decoder evidence, not only --avcodec-hw=none.

    VLC's independent VideoToolbox module ignores that avcodec option.
    Unknown/missing logs remain unverified rather than earning medals.
    """
    kind = 'audio' if scenario == 'flac' else 'video'
    selected = re.findall(r'using ' + kind + r' decoder module "([^"\r\n]+)"', log)
    hardware = re.search(r'using hw decoder module|for hardware decoding|videotoolbox decoder:', log, re.IGNORECASE)
    failed = re.search(r'Failed to get pixel format|Get current frame error|decoder error:', log, re.IGNORECASE)
    return bool(selected) and set(selected) == {'avcodec'} and hardware is None and failed is None
