"""Publish the sourced feature games and measured performance games together."""
from __future__ import annotations
import argparse
from datetime import datetime, timezone
import html
import json
import os
from pathlib import Path
from urllib.parse import urlparse

from medal_events import scoreboards

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
    top = f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900 {height}" role="img" aria-labelledby="{platform}-chart-title {platform}-chart-desc"><title id="{platform}-chart-title">{LABELS[platform]} medal scores</title><desc id="{platform}-chart-desc">Left: documented feature golds. Right: measured gold, silver, and bronze counts. Documentation coverage varies. Unknown does not mean unsupported.</desc><rect width="900" height="{height}" rx="16" fill="#111b30"/><g font-family="system-ui,sans-serif"><text x="28" y="32" font-size="22" fill="#f4f6fb">{LABELS[platform]} · medal scores</text><text x="215" y="61" font-size="12" fill="#a5b5cf">Documented feature golds</text><text x="590" y="61" font-size="12" fill="#a5b5cf">Measured gold / silver / bronze</text>'
    for index, p in enumerate(rows):
        y = 96 + index * 38
        width = p['feature_gold'] / max(1, len(board['features'])) * 300
        top += f'<text x="28" y="{y+5}" fill="#e0e7f5" font-size="14">{esc(p["name"])}</text><rect x="215" y="{y-12}" width="{width:.2f}" height="22" rx="5" fill="{("#ff995e" if p["id"] == "foxide" else "#e9c669")}"/><text x="{230+width:.2f}" y="{y+4}" fill="#f4f6fb" font-size="13">{p["feature_gold"]}</text>'
        x = 590.0
        total = p['performance_gold'] + p['performance_silver'] + p['performance_bronze']
        for medal, color in [('gold', '#e9c669'), ('silver', '#aec3d9'), ('bronze', '#c58c68')]:
            size = p['performance_'+medal] / max_perf * 230
            top += f'<rect x="{x:.2f}" y="{y-12}" width="{size:.2f}" height="22" fill="{color}"/>'
            x += size
        label = f'{p["performance_gold"]}/{p["performance_silver"]}/{p["performance_bronze"]}' if total else '—'
        top += f'<text x="{x+10:.2f}" y="{y+4}" fill="#c4d1e6" font-size="12">{label}</text>'
    return top + '</g></svg>'


def markdown(boards, relative):
    text = '\n## Player medal scoreboards\n\n'
    text += 'Feature support earns **shared gold** for every documented supporter. Unknown is unverified, not unsupported. Counts reflect the documentation reviewed and do not establish an overall best player. Performance medals require at least three verified, comparable repeats on the same platform, host, fixture, scenario, and settings.\n\n'
    text += f'[Interactive HTML report]({relative}/report.html) · [Raw scores]({relative}/medals.json) · [Feature sources]({relative}/features.json) · [45 performance protocols]({relative}/performance-events.json)\n\n'
    for platform, board in boards.items():
        text += f'### {LABELS[platform]}\n\n![{LABELS[platform]} feature golds]({relative}/{platform}.svg)\n\n'
        text += '| Player | Feature 🥇 | Performance 🥇 | 🥈 | 🥉 | Runtime outcome |\n|---|---:|---:|---:|---:|---|\n'
        for p in board['players']:
            outcome = board['runtime'][p['id']]['status'].replace('|', '\\|').replace('\n', ' ')
            text += f'| {p["name"]} | {p["feature_gold"]} | {p["performance_gold"]} | {p["performance_silver"]} | {p["performance_bronze"]} | {outcome} |\n'
        completed = sum(bool(e['rankings']) for e in board['performance'])
        text += f'\nMeasured competitive event cohorts: **{completed}**. Zero performance medals may mean measurements are pending. VM decode results never establish displayed TTFF, GPU efficiency, or battery performance.\n\n'
    return text


def render(catalog, protocol, boards, provenance):
    sections = []
    for platform, board in boards.items():
        players = board['players']
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
            cohorts = [e for e in board['performance'] if e['id'] == event['id']]
            if not cohorts:
                performance_rows.append(f'<tr data-search="{esc(event["name"].lower())}"><th>{esc(event["name"])}<small>{esc(event["scenario"])}</small></th><td>{esc(event["unit"])}</td><td colspan="2" class="unknown">Pending measurement</td><td>{esc(event["collector"])}</td></tr>')
            for cohort in cohorts:
                ranks = cohort['rankings']
                maximum = max((r['value'] for r in ranks), default=1) or 1
                bars = []
                for r in ranks:
                    name = next(p['name'] for p in players if p['id'] == r['player'])
                    color = {'gold': '#e9c669', 'silver': '#aec3d9', 'bronze': '#c58c68'}.get(r['medal'], '#6e829f')
                    bars.append(f'<div class="metricbar">{MEDALS[r["medal"]]} {esc(name)} · {r["value"]:.3g}<i style="display:block;height:4px;border-radius:2px;background:{color};width:{r["value"]/maximum*100:.2f}%"></i></div>')
                performance_rows.append(f'<tr data-search="{esc(event["name"].lower())}"><th>{esc(event["name"])}<small>{esc(cohort["profile"])}</small></th><td>{esc(event["unit"])}</td><td style="min-width:240px">{"".join(bars) or "Insufficient comparable evidence"}</td><td>{len(ranks)} eligible entrants</td><td>{esc(event["collector"])}</td></tr>')
        sections.append(f'<section id="{platform}" class="platform" {"" if platform == "windows" else "hidden"}><div class="duo"><article class="panel"><h2>{LABELS[platform]} medal table</h2><p class="muted">Documented support and measured performance have separate tallies.</p><div class="scroll"><table><thead><tr><th>Player</th><th>Feature 🥇</th><th>Perf 🥇</th><th>🥈</th><th>🥉</th><th>Runtime outcome</th></tr></thead><tbody>{scores}</tbody></table></div></article><article class="panel graph">{chart(platform, board)}<p class="muted">Research date: {esc(catalog["research_date"])}. Source breadth differs; this chart is a documentation tally.</p></article></div><article class="panel"><div class="section-title"><div><span class="eyebrow">THE FEATURE GAMES</span><h2>Shared gold. Every supporter.</h2></div><label>Find an event <input class="filter" placeholder="Subtitles, HDR, playlists…" aria-label="Filter feature events"></label></div><p class="muted">Gold links to supporting documentation. “?” means unknown. Documented support has not necessarily been tested on the installed build. Foobar2000 competes in applicable audio events.</p><div class="scroll"><table class="matrix"><thead><tr><th>Feature event</th>{head}</tr></thead><tbody>{"".join(rows)}</tbody></table></div></article><article class="panel"><div class="section-title"><div><span class="eyebrow">THE PERFORMANCE GAMES</span><h2>Measured, repeated, comparable.</h2></div><label>Find an event <input class="filter" placeholder="CPU, startup, seeking…" aria-label="Filter performance events"></label></div><div class="scroll"><table><thead><tr><th>Event / cohort</th><th>Unit</th><th>Podium / outcome</th><th>Evidence</th><th>Required collector</th></tr></thead><tbody>{"".join(performance_rows)}</tbody></table></div></article></section>')
    style = '''*{box-sizing:border-box}html{color-scheme:dark}body{margin:0;background:#0a1120;color:#e9eef8;font:15px/1.6 system-ui,sans-serif}header,main,footer{max-width:1600px;margin:auto;padding:28px 4vw}header{padding-top:55px}a{color:#9dccff;text-decoration:none}a:hover{text-decoration:underline}.brand,.eyebrow{text-transform:uppercase;letter-spacing:2px;font-size:11px;font-weight:750;color:#ff995e}.brand{font-size:15px}h1{font-size:clamp(40px,6vw,78px);line-height:1.05;letter-spacing:-3px;margin:22px 0}h1 span{color:#ff995e}h2{font-size:23px;line-height:1.25;margin:8px 0 18px}p{max-width:1050px}.lede{font-size:18px;color:#aebdd5}.stats{display:flex;gap:40px;margin:30px 0}.stats b{font-size:32px;display:block;color:#f4f7fd}.stats span,.muted{color:#95a6c2;font-size:13px}.notice{border-left:3px solid #e9c669;background:#192238;padding:20px;border-radius:0 12px 12px 0}.tabs{display:flex;gap:8px;margin:25px 0;position:sticky;top:0;padding:14px 0;background:#0a1120ed;z-index:3}button{border:1px solid #30415e;background:#152039;color:#b9c8df;padding:11px 25px;border-radius:24px;cursor:pointer;font:inherit}button[aria-pressed=true]{background:#ff995e;border-color:#ff995e;color:#161c2c;font-weight:750}.duo{display:grid;grid-template-columns:1fr 1.2fr;gap:22px}.panel{background:#111b30;border:1px solid #23314c;border-radius:16px;padding:26px;margin-bottom:24px;min-width:0}.graph svg{width:100%;height:auto}.scroll{overflow:auto}table{border-collapse:collapse;width:100%;font-size:13px}td,th{padding:12px 14px;text-align:left;border-bottom:1px solid #24314a;white-space:nowrap}thead th{font-size:11px;color:#9fb0ca;text-transform:uppercase;letter-spacing:.5px}th[scope=row]{font-weight:550}small{display:block;color:#889bb8;font-size:10px;font-weight:400}.matrix td{text-align:center;min-width:110px}.matrix th:first-child{position:sticky;left:0;background:#111b30;z-index:1}.yes{color:#e9c669;font-size:19px}.yes a{font-size:10px;color:#c4d5ec}.unknown{color:#7f92b1}.section-title{display:flex;justify-content:space-between;align-items:center;gap:20px}.filter{display:block;margin-top:6px;border:1px solid #354565;border-radius:8px;padding:9px 13px;background:#0b1426;color:#fff}label{font-size:11px;color:#a3b5cf}footer{color:#93a7c5;font-size:12px;padding-bottom:45px}[hidden]{display:none!important}@media(max-width:950px){.duo{grid-template-columns:1fr}.section-title{display:block}.panel{padding:18px}.stats{gap:23px}header,main,footer{padding-left:18px;padding-right:18px}}'''
    script = '''document.querySelectorAll('[data-platform]').forEach(b=>b.addEventListener('click',()=>{document.querySelectorAll('[data-platform]').forEach(x=>x.setAttribute('aria-pressed',String(x===b)));document.querySelectorAll('.platform').forEach(s=>s.hidden=s.id!==b.dataset.platform);location.hash=b.dataset.platform}));document.querySelectorAll('.filter').forEach(i=>i.addEventListener('input',()=>i.closest('article').querySelectorAll('tbody tr').forEach(r=>r.hidden=!r.dataset.search.includes(i.value.toLowerCase()))));const selected=document.querySelector('[data-platform="'+location.hash.slice(1).replace(/[^a-z]/g,'')+'"]');if(selected)selected.click();'''
    buttons = ''.join(f'<button data-platform="{p}" aria-pressed="{str(p == "windows").lower()}">{label}</button>' for p, label in LABELS.items())
    return f'<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Foxide · The Player Games</title><style>{style}</style></head><body><header><div class="brand">🦊 Foxide / Research &amp; Engineering</div><h1>The Player <span>Games.</span></h1><p class="lede">A transparent scoreboard for desktop media players. Documented capabilities earn shared golds. Real measurements decide performance podiums.</p><div class="stats"><div><b>{len(catalog["players"])}</b><span>players in the field</span></div><div><b>{len(catalog["features"])}</b><span>feature medal events</span></div><div><b>{len(protocol["events"])}</b><span>performance events</span></div><div><b>3</b><span>platform boards</span></div></div><aside class="notice"><strong>Evidence before medals.</strong> Gold features are documentation findings; unknown never means absent. Performance requires ≥3 verified repeats and ≥2 comparable entrants in one cohort. Hosted VM software/null-output results are separate from physical GPU, battery, and displayed-frame measurements. Foxide currently uses an external mpv engine.</aside></header><main><nav class="tabs" aria-label="Platform">{buttons}</nav>{"".join(sections)}</main><footer>{source_link(provenance["run_url"], "Actions run") if provenance["run_url"] else "Local validation"} · Source commit: {esc(provenance["source_commit"] or "working tree")} · {esc(provenance["generated_utc"])}<br>Generated from <a href="medals.json">versioned score data</a>. No synthetic benchmark results. Feature research: {esc(catalog["research_date"])}. Performance ties use competition ranks (1, 1, 3); feature possession always shares gold.</footer><script>{script}</script></body></html>'


def write_report(catalog, protocol, measurements, output, readme=None, sessions=()):
    output = Path(output)
    output.mkdir(parents=True, exist_ok=True)
    boards = scoreboards(catalog, protocol, measurements, sessions)
    repository, run_id = os.environ.get('GITHUB_REPOSITORY'), os.environ.get('GITHUB_RUN_ID', '')
    provenance = dict(generated_utc=datetime.now(timezone.utc).isoformat(), source_commit=os.environ.get('GITHUB_SHA'),
                      run_url=f'https://github.com/{repository}/actions/runs/{run_id}' if repository and run_id.isdigit() else None)
    (output / 'medals.json').write_text(json.dumps({'provenance': provenance, 'boards': boards, 'measurements': measurements}, indent=2, allow_nan=False) + '\n', encoding='utf-8')
    (output / 'features.json').write_text(json.dumps(catalog, indent=2) + '\n', encoding='utf-8')
    (output / 'performance-events.json').write_text(json.dumps(protocol, indent=2) + '\n', encoding='utf-8')
    for platform, board in boards.items():
        (output / (platform + '.svg')).write_text(chart(platform, board), encoding='utf-8')
    (output / 'report.html').write_text(render(catalog, protocol, boards, provenance), encoding='utf-8')
    (output / 'scoreboards.md').write_text(markdown(boards, '.'), encoding='utf-8')
    if readme:
        readme = Path(readme)
        relative = Path(os.path.relpath(output, readme.parent)).as_posix()
        start, end = '<!-- foxide-scores:start -->', '<!-- foxide-scores:end -->'
        text = readme.read_text('utf-8') if readme.exists() else '# Foxide\n'
        generated = start + markdown(boards, relative) + end
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
