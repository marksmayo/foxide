"""Refresh authored source-line badges; count physical lines including comments and blanks."""
import json
from pathlib import Path
from urllib.parse import quote

ROOT = Path(__file__).resolve().parents[1]
LANGUAGES = {'.rs': ('Rust', 'dea584'), '.py': ('Python', '3776ab'), '.slint': ('Slint', '7c3aed'), '.ps1': ('PowerShell', '5391fe'), '.lua': ('Lua', '000080'), '.yml': ('YAML', 'cb171e'), '.yaml': ('YAML', 'cb171e'), '.toml': ('TOML', '9c4221')}

def counts(root):
    totals = {}
    paths = [p for prefix in ('crates', 'scripts', 'tests', '.github') for p in sorted((root / prefix).rglob('*'))]
    paths.append(root / 'Cargo.toml')
    for path in paths:
        if not path.is_file() or path.suffix not in LANGUAGES or '__pycache__' in path.parts:
            continue
        language, color = LANGUAGES[path.suffix]
        item = totals.setdefault(language, dict(lines=0, files=0, color=color))
        item['lines'] += len(path.read_text('utf-8').splitlines())
        item['files'] += 1
    return totals

def main():
    totals = counts(ROOT)
    badges = []
    for language, item in sorted(totals.items()):
        url = 'https://img.shields.io/badge/' + quote(language + ' source lines', safe='') + '-' + str(item['lines']) + '-' + item['color']
        badges.append('![' + language + ' source lines](' + url + ')')
    body = '\n'.join(badges) + '\n\nPhysical source lines include comments and blank lines in crates/, scripts/, tests/, .github/ and the root Cargo manifest. Vendored code, lockfiles, generated reports and build output are excluded. Refresh with python scripts/source_lines.py.\n'
    readme = ROOT / 'README.md'
    text = readme.read_text('utf-8')
    begin, end = '<!-- source-lines:start -->', '<!-- source-lines:end -->'
    block = begin + '\n' + body + end
    if begin in text:
        text = text[:text.index(begin)] + block + text[text.index(end) + len(end):]
    else:
        first = text.index('\n')
        text = text[:first+1] + '\n' + block + '\n' + text[first+1:]
    readme.write_text(text, encoding='utf-8')
    (ROOT / 'docs/source-lines.json').write_text(json.dumps(totals, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(totals, indent=2))

if __name__ == '__main__':
    main()
