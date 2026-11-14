"""Native UI action evidence, separate from engine or displayed-frame proxies."""
import math


def action_rows(case, scenario):
    verification = case.get('verification', {})
    if (case.get('status') != 'verified' or case.get('input_method') not in ('native-input', 'accessibility-action')
            or verification.get('kind') != scenario['verification']
            or verification.get('condition') != scenario['condition']
            or verification.get('passed') is not True or verification.get('pid') != case.get('pid')
            or not isinstance(case.get('pid'), int) or isinstance(case.get('pid'), bool) or case['pid'] <= 0
            or not case.get('before') or not case.get('after') or not case.get('source')):
        return []
    start, end = case.get('input_ns'), case.get('completed_ns')
    if any(not isinstance(t, int) or isinstance(t, bool) or t < 0 for t in (start, end)) or end < start:
        return []
    values = [('latency', (end-start)/1000000)]
    for suffix, key in [('cpu_time', 'cpu_time_ms'), ('rss', 'rss_mib')]:
        value = case.get(key)
        if isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and value >= 0:
            values.append((suffix, value))
    return [dict(player=case['player'], platform=case['platform'], profile=case['profile'],
                 trial=case['trial'], event='ui_' + scenario['id'] + '_' + suffix, value=value,
                 verified=True, source=case['source']) for suffix, value in values]
