"""Publish the sourced feature games and measured performance games together."""
from __future__ import annotations
import argparse
from datetime import datetime, timezone
import html
import hashlib
import math
import json
import os
from pathlib import Path
from urllib.parse import urlparse

from medal_events import scoreboards
from collection_io import write_json
from publication import public_evidence

ROOT = Path(__file__).resolve().parents[1]
LABELS = {'windows': 'Windows', 'macos': 'macOS', 'linux': 'Linux'}
MEDALS = {'gold': '🥇', 'silver': '🥈', 'bronze': '🥉', None: '—'}


def esc(value):
    return html.escape(str(value), quote=True)


def source_link(url, label, note=''):
    if urlparse(url).scheme not in ('https', 'http'):
        return esc(label)
    return f'<a href="{esc(url)}" title="{esc(note)}" target="_blank" rel="noopener noreferrer">{esc(label)}</a>'


def chart(platform, board):
    rows = sorted(board['players'], key=lambda p: (-p['feature_gold'], p['name']))
    height = 110 + 38 * len(rows)
    max_perf = max(1, max((p['performance_gold'] + p['performance_silver'] + p['performance_bronze'] for p in rows), default=0))
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900 {height}" role="img" aria-labelledby="{platform}-chart-title {platform}-chart-desc"><title id="{platform}-chart-title">{LABELS[platform]} medal scores</title><desc id="{platform}-chart-desc">Left: documented feature golds. Right: measured gold, silver, and bronze counts. Documentation coverage varies. Unknown does not mean unsupported.</desc><rect width="900" height="{height}" rx="8" class="bg"/><style>{SVG_STYLE}</style><g><text class="title" x="28" y="32" font-size="22" >{LABELS[platform]} · medal scores</text><text x="215" y="61" font-size="12" class="sub">Documented feature golds</text><text x="590" y="61" font-size="12" class="sub">Measured gold / silver / bronze</text>']
    for index, p in enumerate(rows):
        y = 96 + index * 38
        width = p['feature_gold'] / max(1, len(board['features'])) * 300
        parts.append(f'<text x="28" y="{y+5}" font-size="12">{esc(p["name"])}</text><rect x="215" y="{y-12}" width="{width:.2f}" height="22" rx="5" fill="{("#2a78d6" if p["id"] == "foxide" else "#8f8e86")}"/><text x="{230+width:.2f}" y="{y+4}"  font-size="13">{p["feature_gold"]}</text>')
        x = 590.0
        total = p['performance_gold'] + p['performance_silver'] + p['performance_bronze']
        for medal, color in [('gold', '#e9c669'), ('silver', '#aec3d9'), ('bronze', '#c58c68')]:
            size = p['performance_'+medal] / max_perf * 230
            parts.append(f'<rect x="{x:.2f}" y="{y-12}" width="{size:.2f}" height="22" fill="{color}"/>')
            x += size
        label = f'{p["performance_gold"]}/{p["performance_silver"]}/{p["performance_bronze"]}' if total else '—'
        parts.append(f'<text x="{x+10:.2f}" y="{y+4}" class="sub" font-size="12">{label}</text>')
    return ''.join(parts) + '</g></svg>'


def medal_rows(boards, kind='overall'):
    rows = {}
    features = {}
    for board in boards.values():
        for p in board['players']:
            row = rows.setdefault(p['id'], dict(id=p['id'], name=p['name'], gold=0, silver=0, bronze=0))
            if kind != 'features':
                for medal in ('gold', 'silver', 'bronze'):
                    row[medal] += p['performance_' + medal]
        for feature in board['features']:
            for player in feature['awards']:
                features.setdefault(player, set()).add(feature['id'])
    if kind != 'performance':
        for player, ids in features.items():
            rows[player]['gold'] += len(ids)
    return sorted(rows.values(), key=lambda p: (-p['gold'], -p['silver'], -p['bronze'], p['name']))


def medal_table(rows):
    parts = ['| Player | 🥇 | 🥈 | 🥉 | Total |\n|---|---:|---:|---:|---:|\n']
    for p in rows:
        parts.append(f'| {md(p["name"])} | {p["gold"]} | {p["silver"]} | {p["bronze"]} | {p["gold"]+p["silver"]+p["bronze"]} |\n')
    return ''.join(parts)


def md(value):
    return esc(value).replace('|', '\\|').replace('\n', ' ').replace('\r', ' ')


def fields_for(platform, board, sessions=()):
    metadata, fields = {}, {}
    for session in sessions:
        info = session['session']
        if info['system'] != platform or info.get('scope') == 'inventory-only':
            continue
        modes = set()
        for case in session['cases']:
            if case.get('profile'):
                metadata[case['profile']] = info
                mode = case['profile'].split('/')[0]
                if mode != 'frontend-default-diagnostic':
                    modes.add(mode)
        if not modes:
            modes.add({'native-window': 'native-default-window', 'native-ui': 'native-ui'}.get(info.get('presentation'), 'software-null-output'))
        for mode in sorted(modes):
            key = (info.get('host', 'Unrecorded host'), info.get('utc', ''), mode)
            fields.setdefault(key, dict(host=key[0], utc=key[1], mode=mode, info=info, events=[]))
    for event in board['performance']:
        info = metadata.get(event['profile'], {})
        mode = event['profile'].split('/')[0]
        if mode == 'frontend-default-diagnostic':
            continue
        key = (info.get('host', 'Unrecorded host'), info.get('utc', ''), mode)
        field = fields.setdefault(key, dict(host=key[0], utc=key[1], mode=mode, info=info, events=[]))
        field['events'].append(event)
    if not fields:
        fields[('', '', 'pending')] = dict(host='Not measured', utc='', mode='pending', info={}, events=[])
    for key, field in fields.items():
        field['slug'] = platform + '-' + hashlib.sha256(repr(key).encode()).hexdigest()[:10]
        field['names'] = {p['id']: p['name'] for p in board['players']}
        field['label'] = ('Windowed playback · Foxide Desktop' if field['mode'] == 'native-default-window'
                          else 'Software decode · Foxide CLI' if 'null' in field['mode']
                          else 'Native UI actions · default application windows' if field['mode'] == 'native-ui'
                          else 'Pending measurement' if field['mode'] == 'pending' else 'Measured events')
        if field['info']:
            field['label'] += ' · Hosted' if field['info'].get('run_id') else ' · Local'
    return list(fields.values())


def decoder_table(field, sessions):
    observed = {}
    for session in sessions:
        if session['session'] != field['info']:
            continue
        for case in session['cases']:
            if case.get('scenario') not in ('avc1080', 'hevc4k', 'av14k'):
                continue
            key = case['scenario'], case['player']
            observed.setdefault(key, set()).update(e['hwdec'] for e in case.get('playback_events', ())
                                                  if e.get('kind') == 'stats' and isinstance(e.get('hwdec'), str) and e['hwdec'])
    if not observed:
        return ''
    players = [p for p in field['names'] if any(k[1] == p for k in observed)]
    text = '| Observed decoder | ' + ' | '.join(md(field['names'][p]) for p in players) + ' |\n|---|' + '---|'*len(players) + '\n'
    for scenario, label in [('avc1080', '1080p H.264'), ('hevc4k', '4K60 HEVC'), ('av14k', '4K60 AV1')]:
        text += '| ' + label + ' | ' + ' | '.join(md(', '.join(sorted(observed.get((scenario, p), ()))) or 'Not reported') for p in players) + ' |\n'
    return text + '\nEngine observations, not scanout proof. The [mpv manual](https://mpv.io/manual/stable/#options-hwdec) identifies <code>d3d11va-copy</code> as copying decoded video into system RAM; this is not zero-copy.\n\n'


SVG_STYLE = '''text{font:13px system-ui,-apple-system,'Segoe UI',sans-serif;fill:#0b0b0b}.sub{fill:#52514e;font-size:11px}.title{font-size:16px;font-weight:600}.bg{fill:#fcfcfb}.bar{fill:#8f8e86}.foxide{fill:#2a78d6}.winner{font-weight:700}@media(prefers-color-scheme:dark){.bg{fill:#1a1a19}text{fill:#f0efec}.sub{fill:#c3c2b7}.foxide{fill:#3987e5}}'''


def scenario_chart(field):
    events = [e for e in field['events'] if e['rankings']]
    panel_height = 74 + 24 * max((len(e['rankings']) for e in events), default=1)
    height = 95 + max(1, math.ceil(len(events)/2)) * panel_height
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900 {height}" role="img" aria-label="{esc(field["label"])}: metric medians"><style>{SVG_STYLE}</style><rect class="bg" width="900" height="{height}" rx="8"/><text class="title" x="16" y="29">{esc(field["label"])}</text><text class="sub" x="16" y="49">{esc(field["host"])} · {esc(field["utc"][:10])}</text><text class="sub" x="16" y="69">Raw medians; each panel is one comparable metric/cohort. CPU and RAM are not watts or displayed frame timing.</text>']
    if not events:
        parts.append('<text x="16" y="120">Pending measurement — no eligible comparable results.</text>')
    for i, event in enumerate(events):
        x, y = 16 + (i % 2) * 445, 104 + (i // 2) * panel_height
        label = event['name']
        parts.append(f'<text x="{x}" y="{y}" font-weight="600">{esc(label[:55])}</text><text class="sub" x="{x}" y="{y+17}">{esc(event["unit"])} · {"higher" if event.get("direction") == "max" else "lower"} is better</text>')
        maximum = max((r['value'] for r in event['rankings']), default=1) or 1
        for row, rank in enumerate(event['rankings']):
            yy = y + 40 + 24 * row
            width = rank['value'] / maximum * 228
            name = field.get('names', {}).get(rank['player'], rank['player'])
            parts.append(f'<text x="{x+83}" y="{yy}" text-anchor="end">{esc(name)}</text><rect class="bar {"foxide" if rank["player"] == "foxide" else ""}" x="{x+92}" y="{yy-13}" width="{width:.2f}" height="17" rx="3"><title>{esc(name)}: {rank["value"]:.4g} {esc(event["unit"])}, {rank["trials"]} trials</title></rect><text class="{"winner" if rank["medal"] == "gold" else ""}" x="{x+326}" y="{yy}">{rank["value"]:.3g}</text>')
    return ''.join(parts) + '</svg>'


def markdown(boards, relative, sessions=(), provenance=None):
    parts = ['\n## Benchmark\n\n### Latest results\n\n']
    parts.append('Every metric is a separate medal event within one platform, host, fixture and settings cohort. Performance requires ≥3 verified repeats and ≥2 comparable entrants. Documented feature support earns shared gold; unknown does not mean unsupported.\n\n')
    parts.append('Overall feature gold counts each feature once across applicable platforms; performance totals count every eligible field. These totals are not an overall speed ranking.\n\n')
    for title, kind in [('Overall', 'overall'), ('Performance', 'performance'), ('Features', 'features')]:
        parts.append(f'**{title}**\n\n' + medal_table(medal_rows(boards, kind)) + '\n')
    parts.append(f'[Raw scores]({relative}/medals.json) · [Feature sources]({relative}/features.json) · [Performance protocols]({relative}/performance-events.json) · [Download optional HTML]({relative}/report.html)\n\n')
    parts.append('Windowed fields use **Foxide Desktop**; software-only fields use the **Foxide CLI** launcher. Engine playback evidence is distinct from physical frame presentation. UI-action metrics, displayed TTFF, power and battery results remain pending unless explicitly measured.\n\n')
    for platform, board in boards.items():
        parts.append(f'### {LABELS[platform]}\n\n![{LABELS[platform]} medal scores]({relative}/{platform}.svg)\n\n')
        parts.append(medal_table([dict(name=p['name'], gold=p['feature_gold']+p['performance_gold'], silver=p['performance_silver'], bronze=p['performance_bronze']) for p in sorted(board['players'], key=lambda p: (-(p['feature_gold']+p['performance_gold']), -p['performance_silver'], -p['performance_bronze'], p['name']))]))
        for field in fields_for(platform, board, sessions):
            parts.append(f'\n**{field["label"]}**\n\n')
            info = field['info']
            details = [field['host'], info.get('os', ''), info.get('arch', ''),
                       str(info['cpu_count']) + ' logical CPUs' if info.get('cpu_count') else '',
                       field['utc'] or 'Collection date unavailable']
            parts.append(md(' · '.join(x for x in details if x)) + '.\n\n')
            if info.get('run_id'):
                parts.append(f'[Actions run](https://github.com/marksmayo/foxide/actions/runs/{md(info["run_id"])}) · ')
            if info.get('source_commit'):
                parts.append(f'Source: `{md(info["source_commit"])}`. ')
            parts.append('Freshness is per field; older platform results are retained with their original collection dates.\n\n')
            if info.get('duration'):
                parts.append(f"Sampling: {md(info['duration'])}s after engine readiness, {md(info.get('warmup', 0))}s warmup excluded; {('randomized order (seed ' + str(info['order_seed']) + ')') if info.get('order_seed') is not None else 'original collection order'}; cache state uncontrolled.\n\n")
            if field['mode'] == 'native-default-window':
                parts.append('Exploratory native playback at each player’s default window size and audio route; dimensions are not normalized. Hardware fallback and observer overhead differ. This field does not establish displayed FPS or battery efficiency.\n\n')
                parts.append(decoder_table(field, sessions))
            podium = {}
            for event in field['events']:
                for rank in event['rankings']:
                    row = podium.setdefault(rank['player'], dict(name=field['names'][rank['player']], gold=0, silver=0, bronze=0))
                    if rank['medal']:
                        row[rank['medal']] += 1
            if podium:
                parts.append(medal_table(sorted(podium.values(), key=lambda p: (-p['gold'], -p['silver'], -p['bronze'], p['name']))) + '\n')
            parts.append(f'![{LABELS[platform]}: every measured metric]({relative}/{field["slug"]}-scenarios.svg)\n\n')
            parts.append('<details><summary>Every measured metric — median values and podiums</summary>\n\n')
            entrants = [p for p in board['players'] if any(r['player'] == p['id'] for e in field['events'] for r in e['rankings'])]
            if entrants:
                parts.append('| Metric | ' + ' | '.join(md(p['name']) for p in entrants) + ' |\n|---|' + '---:|'*len(entrants) + '\n')
                for event in field['events']:
                    if not event['rankings']: continue
                    ranks = {r['player']: r for r in event['rankings']}
                    cells = []
                    for p in entrants:
                        r = ranks.get(p['id'])
                        value = f'{r["value"]:.3g}' if r else '—'
                        if r and r['medal'] == 'gold': value = f'**{value}**'
                        cells.append(value + (' ' + md(event['unit']) if r else ''))
                    parts.append('| ' + md(event['name']) + ' | ' + ' | '.join(cells) + ' |\n')
            else:
                parts.append('No eligible comparable measurements.\n')
            parts.append('\nBold values are gold medians. — means no eligible comparable result. Trial evidence and exact cohort IDs:\n\n')
            parts.append('| Metric | Cohort | Trials per eligible entrant |\n|---|---|---:|\n')
            for e in field['events']:
                if e['rankings']:
                    parts.append(f'| {md(e["name"])} | `{md(e["profile"])}` | ' + ', '.join(str(r['trials']) for r in e['rankings']) + ' |\n')
            parts.append('\n</details>\n\n')
        parts.append('<details><summary>Runtime outcomes and feature events</summary>\n\n| Player | Runtime outcome | Evidence / limitation |\n|---|---|---|\n')
        for p in board['players']:
            parts.append(f'| {md(p["name"])} | {md(board["runtime"][p["id"]]["status"])} | {md(board["runtime"][p["id"]].get("details", ""))} |\n')
        parts.append('\n| Feature | ' + ' | '.join(md(p['name']) for p in board['players']) + ' |\n|---|' + '---:|'*len(board['players']) + '\n')
        for f in board['features']:
            parts.append('| '+md(f['name'])+' | '+' | '.join('N/A' if p['id'] not in f['applicable'] else '🥇' if p['id'] in f['awards'] else '?' for p in board['players'])+' |\n')
        parts.append('\n? = undocumented/unverified. N/A = inapplicable media type.\n\n</details>\n\n')
    return ''.join(parts)


def render(catalog, protocol, boards, provenance, charts=None):
    sections = []
    charts = charts or {platform: chart(platform, board) for platform, board in boards.items()}
    for platform, board in boards.items():
        players = board['players']
        names = {p['id']: p['name'] for p in players}
        performance_by_event = {}
        for cohort in board['performance']:
            performance_by_event.setdefault(cohort['id'], []).append(cohort)
        scores = ''.join(f'<tr><th scope="row">{esc(p["name"])}</th><td>{p["feature_gold"]}</td><td>{p["performance_gold"]}</td><td>{p["performance_silver"]}</td><td>{p["performance_bronze"]}</td><td title="{esc(board["runtime"][p["id"]]["details"])}">{esc(board["runtime"][p["id"]]["status"])}</td></tr>' for p in players)
        head = ''.join(f'<th scope="col">{esc(p["name"])}</th>' for p in players)
        rows = []
        for event in board['features']:
            claims = {c['player']: c for c in event['claims']}
            cells = []
            for p in players:
                claim = claims.get(p['id'], {})
                if p['id'] not in event['applicable']:
                    cells.append('<td class="unknown" title="Not applicable to this media type">N/A</td>')
                elif p['id'] in event['awards']:
                    links = ' '.join(source_link(u, 'Source ' + str(i+1), claim.get('note', '')) for i, u in enumerate(claim['sources']))
                    cells.append(f'<td class="yes"><span aria-label="gold">🥇</span><small>{links}</small></td>')
                else:
                    cells.append(f'<td class="unknown" title="{esc(claim.get("note", "Documentation or acceptance test pending"))}">{esc(claim.get("status", "?"))}</td>')
            rows.append(f'<tr data-search="{esc(event["name"].lower())}"><th scope="row">{esc(event["name"])}<small>{esc(event.get("group", "Feature"))} · Tier {event.get("tier", "—")}</small></th>' + ''.join(cells) + '</tr>')
        performance_rows = []
        for event in protocol['events']:
            cohorts = performance_by_event.get(event['id'], ())
            if not cohorts:
                performance_rows.append(f'<tr data-search="{esc(event["name"].lower())}"><th>{esc(event["name"])}<small>{esc(event["scenario"])}</small></th><td>{esc(event["unit"])}</td><td colspan="2" class="unknown">Pending measurement</td><td>{esc(event["collector"])}</td></tr>')
            for cohort in cohorts:
                ranks = cohort['rankings']
                maximum = max((r['value'] for r in ranks), default=1) or 1
                bars = []
                for r in ranks:
                    name = names[r['player']]
                    color = {'gold': '#e9c669', 'silver': '#aec3d9', 'bronze': '#c58c68'}.get(r['medal'], '#6e829f')
                    bars.append(f'<div class="metricbar">{MEDALS[r["medal"]]} {esc(name)} · {r["value"]:.3g}<i style="display:block;height:4px;border-radius:2px;background:{color};width:{r["value"]/maximum*100:.2f}%"></i></div>')
                performance_rows.append(f'<tr data-search="{esc(event["name"].lower())}"><th>{esc(event["name"])}<small>{esc(cohort["profile"])}</small></th><td>{esc(event["unit"])}</td><td style="min-width:240px">{"".join(bars) or "Insufficient comparable evidence"}</td><td>{len(ranks)} eligible entrants</td><td>{esc(event["collector"])}</td></tr>')
        sections.append(f'<section id="{platform}" class="platform" {"" if platform == "windows" else "hidden"}><div class="duo"><article class="panel"><h2>{LABELS[platform]} medal table</h2><p class="muted">Documented support and measured performance have separate tallies.</p><div class="scroll"><table><thead><tr><th>Player</th><th>Feature 🥇</th><th>Perf 🥇</th><th>🥈</th><th>🥉</th><th>Runtime outcome</th></tr></thead><tbody>{scores}</tbody></table></div></article><article class="panel graph">{charts[platform]}<p class="muted">Research date: {esc(catalog["research_date"])}. Source breadth differs; this chart is a documentation tally.</p></article></div><article class="panel"><div class="section-title"><div><span class="eyebrow">THE FEATURE GAMES</span><h2>Shared gold. Every supporter.</h2></div><label>Find an event <input class="filter" placeholder="Subtitles, HDR, playlists…" aria-label="Filter feature events"></label></div><p class="muted">Gold links to supporting documentation. “?” means unknown. Documented support has not necessarily been tested on the installed build. Foobar2000 competes in applicable audio events.</p><div class="scroll"><table class="matrix"><thead><tr><th>Feature event</th>{head}</tr></thead><tbody>{"".join(rows)}</tbody></table></div></article><article class="panel"><div class="section-title"><div><span class="eyebrow">THE PERFORMANCE GAMES</span><h2>Measured, repeated, comparable.</h2></div><label>Find an event <input class="filter" placeholder="CPU, startup, seeking…" aria-label="Filter performance events"></label></div><div class="scroll"><table><thead><tr><th>Event / cohort</th><th>Unit</th><th>Podium / outcome</th><th>Evidence</th><th>Required collector</th></tr></thead><tbody>{"".join(performance_rows)}</tbody></table></div></article></section>')
    style = '''*{box-sizing:border-box}html{color-scheme:dark}body{margin:0;background:#0a1120;color:#e9eef8;font:15px/1.6 system-ui,sans-serif}header,main,footer{max-width:1600px;margin:auto;padding:28px 4vw}header{padding-top:55px}a{color:#9dccff;text-decoration:none}a:hover{text-decoration:underline}.brand,.eyebrow{text-transform:uppercase;letter-spacing:2px;font-size:11px;font-weight:750;color:#ff995e}.brand{font-size:15px}h1{font-size:clamp(40px,6vw,78px);line-height:1.05;letter-spacing:-3px;margin:22px 0}h1 span{color:#ff995e}h2{font-size:23px;line-height:1.25;margin:8px 0 18px}p{max-width:1050px}.lede{font-size:18px;color:#aebdd5}.stats{display:flex;gap:40px;margin:30px 0}.stats b{font-size:32px;display:block;color:#f4f7fd}.stats span,.muted{color:#95a6c2;font-size:13px}.notice{border-left:3px solid #e9c669;background:#192238;padding:20px;border-radius:0 12px 12px 0}.tabs{display:flex;gap:8px;margin:25px 0;position:sticky;top:0;padding:14px 0;background:#0a1120ed;z-index:3}button{border:1px solid #30415e;background:#152039;color:#b9c8df;padding:11px 25px;border-radius:24px;cursor:pointer;font:inherit}button[aria-pressed=true]{background:#ff995e;border-color:#ff995e;color:#161c2c;font-weight:750}.duo{display:grid;grid-template-columns:1fr 1.2fr;gap:22px}.panel{background:#111b30;border:1px solid #23314c;border-radius:16px;padding:26px;margin-bottom:24px;min-width:0}.graph svg{width:100%;height:auto}.scroll{overflow:auto}table{border-collapse:collapse;width:100%;font-size:13px}td,th{padding:12px 14px;text-align:left;border-bottom:1px solid #24314a;white-space:nowrap}thead th{font-size:11px;color:#9fb0ca;text-transform:uppercase;letter-spacing:.5px}th[scope=row]{font-weight:550}small{display:block;color:#889bb8;font-size:10px;font-weight:400}.matrix td{text-align:center;min-width:110px}.matrix th:first-child{position:sticky;left:0;background:#111b30;z-index:1}.yes{color:#e9c669;font-size:19px}.yes a{font-size:10px;color:#c4d5ec}.unknown{color:#7f92b1}.section-title{display:flex;justify-content:space-between;align-items:center;gap:20px}.filter{display:block;margin-top:6px;border:1px solid #354565;border-radius:8px;padding:9px 13px;background:#0b1426;color:#fff}label{font-size:11px;color:#a3b5cf}footer{color:#93a7c5;font-size:12px;padding-bottom:45px}[hidden]{display:none!important}@media(max-width:950px){.duo{grid-template-columns:1fr}.section-title{display:block}.panel{padding:18px}.stats{gap:23px}header,main,footer{padding-left:18px;padding-right:18px}}'''
    script = '''const buttons=[...document.querySelectorAll('[data-platform]')],sections=[...document.querySelectorAll('.platform')];let active=null;buttons.forEach(b=>b.addEventListener('click',()=>{if(active===b)return;buttons.forEach(x=>x.setAttribute('aria-pressed',String(x===b)));sections.forEach(s=>{const hidden=s.id!==b.dataset.platform;if(s.hidden!==hidden)s.hidden=hidden});active=b;location.hash=b.dataset.platform}));document.querySelectorAll('.filter').forEach(i=>{const rows=[...i.closest('article').querySelectorAll('tbody tr')];let previous=null;i.addEventListener('input',()=>{const query=i.value.toLowerCase();if(query===previous)return;previous=query;rows.forEach(r=>{const hidden=!r.dataset.search.includes(query);if(r.hidden!==hidden)r.hidden=hidden})})});const selected=buttons.find(b=>b.dataset.platform===location.hash.slice(1));if(selected)selected.click();'''

    buttons = ''.join(f'<button data-platform="{p}" aria-pressed="{str(p == "windows").lower()}">{label}</button>' for p, label in LABELS.items())
    return f'<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Foxide · The Player Games</title><style>{style}</style></head><body><header><div class="brand">🦊 Foxide / Research &amp; Engineering</div><h1>The Player <span>Games.</span></h1><p class="lede">A transparent scoreboard for desktop media players. Documented capabilities earn shared golds. Real measurements decide performance podiums.</p><div class="stats"><div><b>{len(catalog["players"])}</b><span>players in the field</span></div><div><b>{len(catalog["features"])}</b><span>feature medal events</span></div><div><b>{len(protocol["events"])}</b><span>performance events</span></div><div><b>3</b><span>platform boards</span></div></div><aside class="notice"><strong>Evidence before medals.</strong> Gold features are documentation findings; unknown never means absent. Performance requires ≥3 verified repeats and ≥2 comparable entrants in one cohort. Hosted VM software/null-output results are separate from physical GPU, battery, and displayed-frame measurements. Windowed Foxide fields use the Slint/libmpv desktop; software-null fields use its CLI launcher.</aside></header><main><nav class="tabs" aria-label="Platform">{buttons}</nav>{"".join(sections)}</main><footer>{source_link(provenance["run_url"], "Actions run") if provenance["run_url"] else "Local validation"} · Source commit: {esc(provenance["source_commit"] or "working tree")} · {esc(provenance["generated_utc"])}<br>Generated from <a href="medals.json">versioned score data</a>. No synthetic benchmark results. Feature research: {esc(catalog["research_date"])}. Performance ties use competition ranks (1, 1, 3); feature possession always shares gold.</footer><script>{script}</script></body></html>'


def write_report(catalog, protocol, measurements, output, readme=None, sessions=()):
    sessions = [public_evidence(session) for session in sessions]
    measurements = public_evidence({'measurements': measurements})['measurements']
    output = Path(output)
    output.mkdir(parents=True, exist_ok=True)
    boards = scoreboards(catalog, protocol, measurements, sessions)
    repository, run_id = os.environ.get('GITHUB_REPOSITORY'), os.environ.get('GITHUB_RUN_ID', '')
    provenance = dict(generated_utc=datetime.now(timezone.utc).isoformat(), source_commit=os.environ.get('GITHUB_SHA'),
                      run_url=f'https://github.com/{repository}/actions/runs/{run_id}' if repository and run_id.isdigit() else None)
    provenance['fields'] = [session['session'] for session in sessions]
    write_json(output / 'medals.json', {'provenance': provenance, 'boards': boards, 'measurements': measurements})
    (output / 'features.json').write_text(json.dumps(catalog, indent=2) + '\n', encoding='utf-8')
    (output / 'performance-events.json').write_text(json.dumps(protocol, indent=2) + '\n', encoding='utf-8')
    charts = {platform: chart(platform, board) for platform, board in boards.items()}
    for platform, svg in charts.items():
        (output / (platform + '.svg')).write_text(svg, encoding='utf-8')
    field_index = []
    for platform, board in boards.items():
        for field in fields_for(platform, board, sessions):
            (output / (field['slug'] + '-scenarios.svg')).write_text(scenario_chart(field), encoding='utf-8')
            field_index.append({k: field[k] for k in ('slug', 'label', 'mode', 'host', 'utc', 'info')})
    write_json(output / 'fields.json', field_index)
    (output / 'report.html').write_text(render(catalog, protocol, boards, provenance, charts), encoding='utf-8')
    (output / 'scoreboards.md').write_text(markdown(boards, '.', sessions, provenance), encoding='utf-8')
    if readme:
        readme = Path(readme)
        relative = Path(os.path.relpath(output, readme.parent)).as_posix()
        start, end = '<!-- foxide-scores:start -->', '<!-- foxide-scores:end -->'
        text = readme.read_text('utf-8') if readme.exists() else '# Foxide\n'
        generated = start + markdown(boards, relative, sessions, provenance) + end
        if start in text and end in text:
            text = text[:text.index(start)] + generated + text[text.index(end) + len(end):]
        else:
            text = text.rstrip() + '\n\n' + generated + '\n'
        readme.write_text(text, encoding='utf-8')
    return boards


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--features', type=Path, default=ROOT / 'benchmarks/features.json')
    parser.add_argument('--events', type=Path, default=ROOT / 'benchmarks/performance-events.json')
    parser.add_argument('--measurements', type=Path, nargs='*', default=[])
    parser.add_argument('--output', type=Path, default=ROOT / 'docs/scoreboards')
    parser.add_argument('--readme', type=Path)
    parser.add_argument('--sessions', type=Path, nargs='*', default=[])
    args = parser.parse_args()
    rows = [row for path in args.measurements for row in json.loads(path.read_text('utf-8'))]
    sessions = [json.loads(path.read_text('utf-8')) for path in args.sessions]
    write_report(json.loads(args.features.read_text('utf-8')), json.loads(args.events.read_text('utf-8')), rows, args.output, args.readme, sessions)
    print(args.output / 'report.html')


if __name__ == '__main__':
    main()
