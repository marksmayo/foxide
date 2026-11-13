"""Record suite/tool hashes and machine identity without reading secret environment values."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--media', required=True, type=Path)
    p.add_argument('--output', required=True, type=Path)
    p.add_argument('--tool', action='append', type=Path, default=[])
    p.add_argument('--notes', default='')
    args = p.parse_args()
    suite = args.media / 'manifest.json'
    tools = []
    for tool in args.tool:
        tool = tool.resolve(strict=True)
        try:
            if os.name == 'nt' and tool.name.lower() == 'vlc.exe':
                # Windows VLC can launch a GUI/write vlc-help.txt instead of exiting.
                version = {'note': 'Record VLC version from diagnostic logs or executable file properties; no version process launched'}
            else:
                flag = '-version' if tool.stem.lower() in ('ffmpeg', 'ffprobe') else '--version'
                probe = subprocess.run([str(tool), flag], capture_output=True, text=True,
                                       encoding='utf-8', errors='replace', timeout=5)
                version = {'stdout': probe.stdout[:16000], 'stderr': probe.stderr[:16000],
                           'returncode': probe.returncode}
        except subprocess.TimeoutExpired:
            version = {'error': 'Version probe exceeded five seconds; record version manually'}
        tools.append({'path': str(tool), 'bytes': tool.stat().st_size,
                      'sha256': digest(tool), 'version_probe': version})
    record = {'schema_version': 1, 'utc': datetime.now(timezone.utc).isoformat(),
              'host': platform.node(), 'platform': platform.platform(),
              'architecture': platform.machine(), 'processor': platform.processor(),
              'logical_cpus': os.cpu_count(), 'python': platform.python_version(),
              'suite_manifest': str(suite.resolve()), 'suite_sha256': digest(suite),
              'suite': json.loads(suite.read_text('utf-8')), 'tools': tools,
              'notes': args.notes, 'operator_required': {
                  'gpu_and_driver': None, 'display_resolution_refresh_hdr_brightness': None,
                  'power_source_plan_battery_state': None, 'thermal_starting_state': None,
                  'audio_route_volume': None, 'capture_tools_and_calibration': None,
                  'native_app_versions': None, 'installed_footprints_and_ownership': None}}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x', encoding='utf-8') as output:
        json.dump(record, output, indent=2)
    print(args.output)


if __name__ == '__main__':
    main()
