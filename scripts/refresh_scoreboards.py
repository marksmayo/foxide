"""Build README graphs/tables from latest archived and freshly collected fields."""
import argparse
from datetime import datetime, timezone
import json
from pathlib import Path

from build_medal_report import ROOT, write_report
from collection_io import write_json
from publication import public_evidence


def field_key(collection):
    info = collection['session']
    system, presentation = info['system'], info.get('presentation', 'software-null')
    if system not in ('windows', 'macos', 'linux') or presentation not in ('software-null', 'native-window', 'native-ui'):
        raise ValueError('Unsupported field identity')
    return system, 'hosted' if info.get('run_id') else 'local', presentation


def latest_collections(collections):
    latest = {}
    for collection in collections:
        info = collection['session']
        key = field_key(collection)
        date = datetime.fromisoformat(info['utc'].replace('Z', '+00:00'))
        if date.tzinfo is None:
            date = date.replace(tzinfo=timezone.utc)
        if key not in latest or date >= latest[key][0]:
            latest[key] = (date, collection)
    return [value[1] for value in latest.values()]



def enforce_software_decoder_evidence(collection):
    """Retire legacy VLC software medals whose decoder was never observed."""
    if collection['session'].get('presentation', 'software-null') != 'software-null':
        return
    proven = set()
    for case in collection.get('cases', []):
        if case.get('player') != 'vlc':
            continue
        if case.get('software_decode_verified') is True:
            proven.add((case['profile'], case['trial']))
        else:
            case['verified'] = False
            if case.get('status') == 'verified-playback':
                case['status'] = 'software-decoder-unverified'
    for row in collection.get('measurements', []):
        if row.get('player') == 'vlc' and (row['profile'], row['trial']) not in proven:
            row['verified'] = False


def refresh(bundles, platform_dir, output, readme, runtime_dir=None):
    collections = [json.loads(path.read_text('utf-8')) for path in sorted(bundles.glob('*.json'))]
    if platform_dir.exists():
        for path in sorted(platform_dir.rglob('session.json')):
            rows = path.with_name('measurements.json')
            if rows.is_file():
                collection = json.loads(path.read_text('utf-8'))
                collection['measurements'] = json.loads(rows.read_text('utf-8'))
                collections.append(collection)
    selected = latest_collections([public_evidence(c) for c in collections])
    if not selected:
        raise ValueError('No benchmark evidence found; refusing to replace reports with empty results')
    bundles.mkdir(parents=True, exist_ok=True)
    for collection in selected:
        destination = bundles / ('-'.join(field_key(collection)) + '.json')
        references = {}
        for index, case in enumerate(collection['cases']):
            case.setdefault('original_source', case.get('source'))
            reference = destination.resolve()
            if reference.is_relative_to(ROOT):
                reference = reference.relative_to(ROOT)
            case['source'] = reference.as_posix() + '#/cases/' + str(index)
            if case.get('profile'):
                references[case['player'], case['profile'], case['trial']] = case['source']
        for row in collection['measurements']:
            row['source'] = references.get((row['player'], row['profile'], row['trial']), row['source'])
        write_json(destination, collection, compact=True)
    runtime = [public_evidence(json.loads(path.read_text('utf-8'))) for path in sorted(runtime_dir.glob('*.json'))] if runtime_dir else []
    if any(s['session'].get('scope') != 'inventory-only' or any(c.get('verified') is True for c in s['cases']) for s in runtime):
        raise ValueError('Runtime supplements must be inventory-only and cannot assert verified playback')
    boards = write_report(json.loads((ROOT/'benchmarks/features.json').read_text('utf-8')),
                          json.loads((ROOT/'benchmarks/performance-events.json').read_text('utf-8')),
                          [row for c in selected for row in c['measurements']], output, readme, selected + runtime)
    print(f'{len(selected)} independently collected fields; ' + str(sum(len(c['measurements']) for c in selected)) + ' measurement rows')
    return boards


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bundles', type=Path, default=ROOT/'benchmarks/results')
    parser.add_argument('--platform-dir', type=Path, default=ROOT/'results/local/platforms')
    parser.add_argument('--output', type=Path, default=ROOT/'docs/scoreboards')
    parser.add_argument('--readme', type=Path)
    parser.add_argument('--runtime-dir', type=Path, default=ROOT/'benchmarks/runtime')
    args = parser.parse_args()
    refresh(args.bundles, args.platform_dir, args.output, args.readme, args.runtime_dir)


if __name__ == '__main__':
    main()
