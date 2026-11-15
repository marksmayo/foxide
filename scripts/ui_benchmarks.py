"""Native UI action evidence, separate from engine or displayed-frame proxies."""
import math


def action_cohort(settings, context, action):
    # Window placement varies between otherwise identical fresh launches. Keep
    # bounds in raw evidence, not in the workload identity. Each app uses its
    # default window size; these are not equal-viewport measurements.
    return dict(settings, backend=context['backend'], input_method=context['input_method'],
                scenario=action, fixture='idle-no-media', window_policy='application-default')


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
    baseline, current = case.get('rss_before_mib'), case.get('rss_mib')
    if all(isinstance(v, (int, float)) and not isinstance(v, bool) and math.isfinite(v) and v >= 0
           for v in (baseline, current)):
        values.append(('rss_increase', max(0, current-baseline)))
    dispatched = case.get('dispatch_completed_ns')
    if isinstance(dispatched, int) and not isinstance(dispatched, bool) and start <= dispatched <= end:
        values.extend([('dispatch_latency', (dispatched-start)/1000000),
                       ('settle_latency', (end-dispatched)/1000000)])
    return [dict(player=case['player'], platform=case['platform'], profile=case['profile'],
                 trial=case['trial'], event='ui_' + scenario['id'] + '_' + suffix, value=value,
                 verified=True, source=case['source']) for suffix, value in values]


def has_comparable_ui_measurements(rows):
    from medal_events import performance_medals
    cohorts = {(r['event'], r['platform'], r['profile']) for r in rows}
    return any(performance_medals(rows, event, platform, profile)
               for event, platform, profile in cohorts)
