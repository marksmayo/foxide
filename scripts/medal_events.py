"""Medal rules shared by HTML, README, and Actions reports."""
from __future__ import annotations
from collections import defaultdict
import math
import statistics
from urllib.parse import urlparse


def feature_medals(claims):
    """Feature possession is a shared gold; unknown is never a negative claim."""
    return {c['player']: 'gold' for c in claims
            if c['status'] == 'supported' and c.get('sources')
            and all(urlparse(u).scheme in ('https', 'http') and urlparse(u).hostname for u in c['sources'])}


def performance_medals(rows, event, platform, profile, direction='min', resolution=0):
    """Rank medians within one exact host/fixture/scenario measurement cohort.

    All recorded trials for an entrant must be valid. At least three distinct
    trials and two eligible entrants are required. Olympic competition ranking
    gives 1/1/3 to a tie for first. Feature events use a separate shared-gold rule.
    """
    if direction not in ('min', 'max') or not math.isfinite(resolution) or resolution < 0:
        raise ValueError('direction must be min/max and resolution finite and nonnegative')
    grouped = defaultdict(list)
    for row in rows:
        if (row['event'], row['platform'], row['profile']) == (event, platform, profile):
            grouped[row['player']].append(row)
    entrants = []
    for player, trials in grouped.items():
        if len({r['trial'] for r in trials}) != len(trials) or len(trials) < 3:
            continue
        if not all(r.get('verified') is True and r.get('source')
                   and isinstance(r.get('value'), (int, float))
                   and not isinstance(r['value'], bool)
                   and math.isfinite(r['value']) and r['value'] >= 0 for r in trials):
            continue
        value = statistics.median(r['value'] for r in trials)
        entrants.append({'player': player, 'value': value, 'trials': len(trials),
                         'score': round(value / resolution) if resolution else value})
    if len(entrants) < 2:
        return []
    entrants.sort(key=lambda r: r['score'], reverse=direction == 'max')
    rank, previous = 0, None
    for index, entrant in enumerate(entrants, 1):
        if entrant['score'] != previous:
            rank = index
        entrant['rank'] = rank
        entrant['medal'] = {1: 'gold', 2: 'silver', 3: 'bronze'}.get(rank)
        previous = entrant.pop('score')
    return entrants


def scoreboards(catalog, protocol, measurements, sessions=()):
    """One platform board, with documented features and empirical medals apart."""
    boards = {}
    for platform in ('windows', 'macos', 'linux'):
        players = [{**p, 'feature_gold': 0, 'performance_gold': 0,
                    'performance_silver': 0, 'performance_bronze': 0}
                   for p in catalog['players'] if platform in p['platforms']]
        lookup = {p['id']: p for p in players}
        features = []
        for feature in catalog['features']:
            applicable = [p['id'] for p in players if set(p.get('media_types', ['audio', 'video'])) & set(feature.get('media_types', ['audio', 'video']))]
            claims = [c for c in catalog['claims'] if c['feature'] == feature['id']
                      and c['player'] in applicable and platform in c.get('platforms', [platform])]
            awards = feature_medals(claims)
            for player in awards:
                lookup[player]['feature_gold'] += 1
            features.append({**feature, 'awards': awards, 'claims': claims, 'applicable': applicable})
        performances = []
        for event in protocol['events']:
            profiles = sorted({r['profile'] for r in measurements
                               if r['event'] == event['id'] and r['platform'] == platform})
            for profile in profiles:
                ranks = performance_medals([r for r in measurements if r['player'] in lookup],
                                           event['id'], platform, profile, event['direction'],
                                           event.get('resolution', 0))
                for row in ranks:
                    if row['medal']:
                        lookup[row['player']]['performance_' + row['medal']] += 1
                performances.append({**event, 'profile': profile, 'rankings': ranks})
        runtime = {p['id']: {'status': 'Not run in this dataset', 'details': ''} for p in players}
        for player in runtime:
            evidence = [r for r in measurements if r['platform'] == platform and r['player'] == player]
            if evidence:
                cases = {(r['profile'], r['trial']) for r in evidence}
                verified = {(r['profile'], r['trial']) for r in evidence if r.get('verified') is True}
                runtime[player] = {'status': f'Measured {len(cases)} cases; {len(verified)} verified',
                                   'details': 'See source case records; session manifest not supplied.'}
        outcomes = defaultdict(list)
        for session in sessions:
            if session['session']['system'] != platform:
                continue
            for entry in session['inventory']:
                if entry['player'] in runtime:
                    runtime[entry['player']] = {'status': entry['status'], 'details': entry.get('reason', '')}
            for case in session['cases']:
                if case['player'] in runtime:
                    outcomes[case['player']].append(case)
        for player, cases in outcomes.items():
            count = sum(c.get('verified') is True for c in cases)
            runtime[player] = {'status': f'Verified playback {count}/{len(cases)}' if count else f'Unverified / diagnostic ({len(cases)} cases)',
                               'details': ', '.join(sorted({c['status'] for c in cases}))}
        boards[platform] = {'players': players, 'features': features, 'performance': performances, 'runtime': runtime,
                            'research_date': catalog['research_date']}
    return boards
