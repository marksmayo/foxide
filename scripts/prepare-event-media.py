"""Extend verified short seed bitstreams to 90s without re-encoding 4K video."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--seed', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    seed = json.loads((args.seed / 'manifest.json').read_text('utf-8'))
    if (args.output / 'manifest.json').exists():
        parser.error('Choose a fresh output; media manifests are immutable')
    args.output.mkdir(parents=True, exist_ok=True)
    fixtures = []
    for fixture in seed['fixtures']:
        if any(f['id'] == fixture['id'] for f in fixtures): continue
        source = args.seed / fixture['path']
        if digest(source) != fixture['sha256']:
            raise ValueError('Seed hash mismatch: ' + str(source))
        output = args.output / fixture['path']
        # Subtitle packets in B can extend to 659s; performance loops omit them.
        # The ASS workload uses the same explicit external file for every player.
        command = ['ffmpeg', '-hide_banner', '-loglevel', 'warning', '-n', '-stream_loop', '-1',
                   '-i', str(source), '-map', '0:v?', '-map', '0:a', '-sn', '-c', 'copy', '-t', '90']
        if fixture['id'] == 'D':
            command = ['ffmpeg', '-hide_banner', '-loglevel', 'warning', '-n', '-stream_loop', '-1',
                       '-i', str(source), '-map', '0:a', '-map', '0:v?', '-c:a', 'flac', '-c:v', 'copy',
                       '-disposition:v', 'attached_pic', '-t', '90']
        command.append(str(output))
        subprocess.run(command, check=True)
        probe = json.loads(subprocess.check_output(['ffprobe', '-v', 'error', '-show_format', '-of', 'json', str(output)], text=True))
        if not 89 <= float(probe['format']['duration']) <= 92:
            raise ValueError('Unexpected duration: ' + str(output))
        fixtures.append(dict(id=fixture['id'], path=output.name, sha256=digest(output),
                             source_sha256=fixture['sha256'], bytes=output.stat().st_size, command=command))
    subtitle = args.seed / 'subtitles.ass'
    if subtitle.exists(): shutil.copyfile(subtitle, args.output / 'subtitles.ass')
    (args.output / 'manifest.json').write_text(json.dumps(dict(seconds=90, recipe='Repeated short synthetic bitstreams; not 90s of unique content. B subtitles omitted; ASS event uses external subtitles.', fixtures=fixtures), indent=2), encoding='utf-8')


if __name__ == '__main__':
    main()
