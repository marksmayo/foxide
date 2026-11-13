"""Generate Foxide's versioned synthetic fixtures, then probe and SHA-256 each artifact.

Python standard library + FFmpeg/ffprobe with libx264, libx265, libsvtav1 and libopus.
Sources/commands are repeatable; bytes are authoritative only for a pinned toolchain.
Smoke fixtures preserve codec/resolution but are too short for power/seek comparisons.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
RECIPE_VERSION = 1


def execute(command):
    print(subprocess.list2cmdline([str(x) for x in command]), flush=True)
    subprocess.run([str(x) for x in command], check=True)


def sha256(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def probe(ffprobe, path):
    return json.loads(subprocess.check_output([
        ffprobe, '-v', 'error', '-show_format', '-show_streams', '-of', 'json', str(path)
    ], text=True))


def validate(identifier, metadata, seconds):
    streams = metadata['streams']
    video = [s for s in streams if s['codec_type'] == 'video' and not s.get('disposition', {}).get('attached_pic')]
    audio = [s for s in streams if s['codec_type'] == 'audio']
    assert audio, 'Missing audio'
    assert float(metadata['format']['duration']) >= seconds - 0.2, 'Truncated fixture'
    expected_audio = {'A': 'aac', 'B': 'ac3', 'C': 'opus', 'D': 'flac'}[identifier]
    assert audio[0]['codec_name'] == expected_audio, f'Expected {expected_audio}'
    if identifier != 'D':
        v = video[0]
        width, height = (1920, 1080) if identifier == 'A' else (3840, 2160)
        assert (v['width'], v['height']) == (width, height)
        assert v['codec_name'] == {'A': 'h264', 'B': 'hevc', 'C': 'av1'}[identifier]
        assert v['r_frame_rate'] == ('30/1' if identifier == 'A' else '60/1')
    if identifier == 'B':
        assert '10' in video[0]['pix_fmt'], 'Missing 10-bit output'
        assert video[0]['color_transfer'] == 'smpte2084', 'Missing PQ metadata'
        assert video[0]['color_primaries'] == 'bt2020'
        assert {s['codec_name'] for s in streams if s['codec_type'] == 'subtitle'} >= {'ass', 'subrip'}
    if identifier == 'D':
        assert any(s.get('disposition', {}).get('attached_pic') for s in streams), 'Missing cover'


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--profile', choices=['smoke', 'full'], default='smoke')
    p.add_argument('--seconds', type=int, help='Default: smoke=3, full=660')
    p.add_argument('--files', nargs='+', choices=list('ABCD'), default=list('ABCD'))
    p.add_argument('--output', type=Path, default=ROOT / 'media/generated')
    p.add_argument('--ffmpeg', default='ffmpeg')
    p.add_argument('--ffprobe', default='ffprobe')
    p.add_argument('--verify-only', action='store_true')
    args = p.parse_args()
    output = args.output.resolve()
    if args.verify_only:
        manifest = json.loads((output / 'manifest.json').read_text('utf-8'))
        for fixture in manifest['fixtures']:
            path = output / fixture['path']
            if sha256(path) != fixture['sha256']:
                raise ValueError(f'Hash mismatch: {path}')
            validate(fixture['id'], probe(args.ffprobe, path), manifest['seconds'])
        print('All fixture hashes and stream contracts verified.')
        return
    seconds = args.seconds if args.seconds is not None else (3 if args.profile == 'smoke' else 660)
    if not 1 <= seconds <= 3600:
        p.error('seconds must be between 1 and 3600')
    if args.profile == 'full' and seconds < 660:
        p.error('full profile requires at least 660 seconds')
    output.mkdir(parents=True, exist_ok=True)
    if (output / 'manifest.json').exists():
        p.error('Output already has an immutable manifest; choose a new --output or use --verify-only')
    commands = []
    base = [args.ffmpeg, '-hide_banner', '-loglevel', 'warning', '-n']
    common = ['-t', str(seconds), '-map_metadata', '-1', '-metadata', 'creation_time=2026-01-01T00:00:00Z']
    fixtures = []

    def create(identifier, filename, options):
        path = output / filename
        if path.exists():
            p.error(f'Refusing to overwrite {path}; choose a new output directory')
        cmd = base + options + common + [str(path)]
        commands.append(cmd)
        execute(cmd)
        metadata = probe(args.ffprobe, path)
        validate(identifier, metadata, seconds)
        fixtures.append({'id': identifier, 'path': filename, 'sha256': sha256(path),
                         'bytes': path.stat().st_size, 'ffprobe': metadata})

    def sources(size, fps):
        return ['-f', 'lavfi', '-i', f'testsrc2=size={size}:rate={fps}',
                '-f', 'lavfi', '-i', 'sine=frequency=440:sample_rate=48000']

    if 'A' in args.files:
        create('A', 'A-avc-1080p.mp4', sources('1920x1080', 30) + [
            '-map', '0:v', '-map', '1:a', '-c:v', 'libx264', '-preset', 'veryfast',
            '-threads', '1', '-pix_fmt', 'yuv420p', '-g', '60', '-keyint_min', '60', '-sc_threshold', '0',
            '-b:v', '20M', '-minrate', '20M', '-maxrate', '20M', '-bufsize', '40M',
            '-x264-params', 'nal-hrd=cbr:force-cfr=1', '-c:a', 'aac', '-b:a', '192k',
            '-movflags', '+faststart'])
    if 'B' in args.files:
        srt, ass = output / 'subtitles.srt', output / 'subtitles.ass'
        srt.write_text('1\n00:00:00,000 --> 00:10:59,000\nFoxide benchmark SRT\n', encoding='utf-8')
        ass.write_text('[Script Info]\nScriptType: v4.00+\nPlayResX: 3840\nPlayResY: 2160\n'
                       '[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n'
                       'Style: Default,Arial,72,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,2,30,30,30,1\n'
                       '[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n'
                       'Dialogue: 0,0:00:00.00,0:10:59.00,Default,,0,0,0,,Foxide benchmark ASS\n', encoding='utf-8')
        create('B', 'B-hevc-4k60-pq.mkv', sources('3840x2160', 60) + [
            '-i', str(ass), '-i', str(srt), '-map', '0:v', '-map', '1:a', '-map', '2:s', '-map', '3:s',
            '-vf', 'zscale=pin=bt709:tin=bt709:min=bt709:rin=limited:p=bt2020:t=smpte2084:m=bt2020nc:r=limited:npl=100,format=yuv420p10le',
            '-c:v', 'libx265', '-preset', 'ultrafast', '-threads', '1',
            '-b:v', '80M', '-minrate', '80M', '-maxrate', '80M', '-bufsize', '160M',
            '-x265-params', 'pools=none:frame-threads=1:keyint=120:min-keyint=120:scenecut=0:strict-cbr=1:hrd=1:repeat-headers=1:hdr10=1:colorprim=bt2020:transfer=smpte2084:colormatrix=bt2020nc:master-display=G(13250,34500)B(7500,3000)R(34000,16000)WP(15635,16450)L(1000000,1):max-cll=100,100',
            '-color_primaries', 'bt2020', '-color_trc', 'smpte2084', '-colorspace', 'bt2020nc',
            '-c:a', 'ac3', '-b:a', '640k', '-c:s:0', 'ass', '-c:s:1', 'srt',
            '-metadata:s:s:0', 'language=eng', '-metadata:s:s:1', 'language=eng'])
    if 'C' in args.files:
        create('C', 'C-av1-4k60.mkv', sources('3840x2160', 60) + [
            '-map', '0:v', '-map', '1:a', '-c:v', 'libsvtav1', '-preset', '12', '-crf', '35',
            '-svtav1-params', 'lp=2', '-pix_fmt', 'yuv420p10le', '-g', '120',
            '-c:a', 'libopus', '-b:a', '192k'])
    if 'D' in args.files:
        cover = output / 'cover.png'
        execute(base + ['-f', 'lavfi', '-i', 'color=c=0xd66b32:s=600x600', '-frames:v', '1', '-update', '1', str(cover)])
        for track in range(1, 4):
            create('D', f'D-flac-track-{track:02}.flac', [
                '-f', 'lavfi', '-i', f'sine=frequency={220 * track}:sample_rate=48000',
                '-i', str(cover), '-map', '0:a', '-map', '1:v', '-c:a', 'flac',
                '-sample_fmt', 's32', '-c:v', 'copy', '-disposition:v', 'attached_pic',
                '-metadata', 'album=Foxide Synthetic Album', '-metadata', f'track={track}/3',
                '-metadata', f'title=Benchmark Track {track}', '-metadata', 'artist=Foxide'])
    manifest = {'recipe_version': RECIPE_VERSION, 'profile': args.profile, 'seconds': seconds,
                'source': 'FFmpeg lavfi testsrc2/sine/color; synthetic signal, not an authored HDR reference',
                'warning': 'PQ conversion is a 100-nit SDR pattern in HDR10. Add an independently validated high-luminance HDR reference for tone-mapping/color acceptance.',
                'ffmpeg_version': subprocess.check_output([args.ffmpeg, '-version'], text=True),
                'ffprobe_version': subprocess.check_output([args.ffprobe, '-version'], text=True),
                'commands': commands, 'fixtures': fixtures}
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    print(f'Created {len(fixtures)} validated fixtures: {output / "manifest.json"}')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, AssertionError, subprocess.CalledProcessError) as exc:
        print(f'Fixture generation failed: {exc}', file=sys.stderr)
        sys.exit(1)
