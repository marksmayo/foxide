"""Medal rules shared by HTML, README, and Actions reports."""
from __future__ import annotations
from collections import defaultdict
import math
import statistics
from urllib.parse import urlparse


def feature_medals(claims, valid_source=None):
    """Feature possession is a shared gold; unknown is never a negative claim."""
    if valid_source is None:
        def valid_source(url):
            parsed = urlparse(url)
            return parsed.scheme in ('https', 'http') and bool(parsed.hostname)
    return {c['player']: 'gold' for c in claims
            if c['status'] == 'supported' and c.get('sources')
            and all(valid_source(u) for u in c['sources'])}


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
    medals = {1: 'gold', 2: 'silver', 3: 'bronze'}
    for index, entrant in enumerate(entrants, 1):
        if entrant['score'] != previous:
            rank = index
        entrant['rank'] = rank
        entrant['medal'] = medals.get(rank)
        previous = entrant.pop('score')
    return entrants


def scoreboards(catalog, protocol, measurements, sessions=()):
    """One platform board, with documented features and empirical medals apart."""
    boards = {}
    claims_by_feature = defaultdict(list)
    measurements_by_platform = defaultdict(list)
    measurements_by_player = defaultdict(list)
    cohorts = defaultdict(list)
    profiles = defaultdict(set)
    for claim in catalog['claims']:
        claims_by_feature[claim['feature']].append(claim)
    for row in measurements:
        key = row['platform'], row['event'], row['profile']
        cohorts[key].append(row)
        profiles[key[:2]].add(row['profile'])
        measurements_by_platform[row['platform']].append(row)
        measurements_by_player[row['platform'], row['player']].append(row)
    source_validity = {}
    def valid_source(url):
        if url not in source_validity:
            parsed = urlparse(url)
            source_validity[url] = parsed.scheme in ('https', 'http') and bool(parsed.hostname)
        return source_validity[url]
    for platform in ('windows', 'macos', 'linux'):
        players = [{**p, 'feature_gold': 0, 'performance_gold': 0,
                    'performance_silver': 0, 'performance_bronze': 0}
                   for p in catalog['players'] if platform in p['platforms']]
        lookup = {p['id']: p for p in players}
        features = []
        media_types = {p['id']: set(p.get('media_types', ['audio', 'video'])) for p in players}
        for feature in catalog['features']:
            types = set(feature.get('media_types', ['audio', 'video']))
            applicable = [p['id'] for p in players if media_types[p['id']] & types]
            applicable_set = set(applicable)
            claims = [c for c in claims_by_feature[feature['id']] if c['player'] in applicable_set and platform in c.get('platforms', [platform])]
            awards = feature_medals(claims, valid_source)
            for player in awards:
                lookup[player]['feature_gold'] += 1
            features.append({**feature, 'awards': awards, 'claims': claims, 'applicable': applicable})
        performances = []
        for event in protocol['events']:
            for profile in sorted(profiles[platform, event['id']]):
                ranks = performance_medals([r for r in cohorts[platform, event['id'], profile] if r['player'] in lookup],
                                           event['id'], platform, profile, event['direction'],
                                           event.get('resolution', 0))
                for row in ranks:
                    if row['medal']:
                        lookup[row['player']]['performance_' + row['medal']] += 1
                performances.append({**event, 'profile': profile, 'rankings': ranks})
        runtime = {p['id']: {'status': 'Not run in this dataset', 'details': ''} for p in players}
        for player in runtime:
            evidence = measurements_by_player[platform, player]
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
            playback = [c for c in cases if c.get('scope') != 'native-ui']
            ui = [c for c in cases if c.get('scope') == 'native-ui']
            count = sum(c.get('verified') is True for c in playback)
            parts = []
            if playback:
                parts.append(f'Verified playback {count}/{len(playback)}' if count else f'Unverified / diagnostic ({len(playback)} cases)')
            if ui:
                parts.append(f'Verified UI actions {sum(c.get("verified") is True for c in ui)}/{len(ui)}')
            runtime[player] = {'status': '; '.join(parts),
                               'details': ', '.join(sorted({c['status'] for c in cases}))}
        boards[platform] = {'players': players, 'features': features, 'performance': performances, 'runtime': runtime,
                            'research_date': catalog['research_date']}
    return boards
