"""Install the pinned official x64 mpv CLI archive locally, with SHA-256 verification.

Supports an already-downloaded --archive when network access is unavailable.
Never changes PATH, file associations, system proxy settings or existing installs.
"""
from __future__ import annotations
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import platform
import shutil
import stat
import subprocess
import sys
import urllib.request
from uuid import uuid4
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def validate_members(archive, destination):
    root = destination.resolve()
    for info in archive.infolist():
        normalized = info.filename.replace('\\', '/')
        member = PurePosixPath(normalized)
        if member.is_absolute() or '..' in member.parts or PureWindowsPath(normalized).drive:
            raise ValueError('Archive entry escapes installation directory: ' + info.filename)
        if stat.S_ISLNK(info.external_attr >> 16):
            raise ValueError('Archive symlinks are not accepted')
        if not (root / normalized).resolve().is_relative_to(root):
            raise ValueError('Archive path is outside installation directory')


def install(archive_path, package, tools):
    with archive_path.open('rb') as stream:
        actual = hashlib.file_digest(stream, 'sha256').hexdigest()
    if actual != package['sha256']:
        raise ValueError('mpv archive SHA-256 does not match the pinned official distribution')
    tools = tools.resolve()
    target = tools / 'mpv'
    if target.exists():
        raise FileExistsError('Refusing to overwrite existing tools/mpv; inspect that installation first')
    stage = tools / ('.mpv-staging-' + uuid4().hex)
    stage.mkdir()
    try:
        with zipfile.ZipFile(archive_path) as archive:
            validate_members(archive, stage)
            archive.extractall(stage)
        executable = stage / 'mpv.exe'
        if not executable.is_file():
            raise ValueError('The archive does not contain the expected mpv.exe')
        probe = subprocess.run([str(executable), '--version'], capture_output=True, text=True,
                               encoding='utf-8', errors='replace', timeout=10)
        if probe.returncode != 0 or not probe.stdout.startswith('mpv '):
            raise RuntimeError('Extracted mpv did not pass --version: ' + probe.stderr)
        metadata = {**package, 'installed_utc': datetime.now(timezone.utc).isoformat(),
                    'archive_sha256_verified': actual, 'version_output': probe.stdout}
        (stage / 'foxide-installation.json').write_text(json.dumps(metadata, indent=2), encoding='utf-8')
        # Both resolved paths are task-owned children of tools; never replace an existing install.
        if stage.resolve().parent != tools or target.resolve().parent != tools:
            raise ValueError('Installation paths escaped the tools directory')
        stage.rename(target)
        return metadata
    finally:
        if stage.exists():
            if stage.resolve().parent != tools or not stage.name.startswith('.mpv-staging-'):
                raise ValueError('Unsafe staging cleanup target')
            shutil.rmtree(stage)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--archive', type=Path, help='Use an existing pinned zip instead of downloading')
    p.add_argument('--output', type=Path, default=ROOT / 'results/local/mpv-installation.json')
    args = p.parse_args()
    if os.name != 'nt' or platform.machine().lower() not in ('amd64', 'x86_64'):
        p.error('The pinned package is for Windows x64; macOS CI uses Homebrew')
    package = json.loads((ROOT / 'benchmarks/mpv-windows.json').read_text('utf-8'))
    tools = ROOT / 'tools'
    tools.mkdir(exist_ok=True)
    record = {'status': 'pending', 'package': package}
    try:
        if (tools / 'mpv/mpv.exe').is_file():
            probe = subprocess.run([str(tools / 'mpv/mpv.exe'), '--version'], capture_output=True, text=True, timeout=10)
            if probe.returncode != 0:
                raise RuntimeError('Existing local mpv failed its version check')
            record.update(status='already-present', version_output=probe.stdout)
        else:
            archive_path = args.archive.resolve() if args.archive else tools / 'downloads/mpv-0.41.0-windows-x64.zip'
            if args.archive is None and not archive_path.exists():
                archive_path.parent.mkdir(parents=True, exist_ok=True)
                part = archive_path.with_name(archive_path.name + '.' + uuid4().hex + '.part')
                try:
                    with urllib.request.urlopen(package['url'], timeout=30) as source, part.open('xb') as out:
                        shutil.copyfileobj(source, out)
                    part.rename(archive_path)
                finally:
                    if part.exists():
                        part.unlink()
            metadata = install(archive_path, package, tools)
            record.update(status='installed', metadata=metadata)
    except (OSError, ValueError, RuntimeError, zipfile.BadZipFile, subprocess.TimeoutExpired) as error:
        record.update(status='failed', error=str(error))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, indent=2), encoding='utf-8')
    print(json.dumps(record, indent=2))
    return 0 if record['status'] in ('installed', 'already-present') else 1


if __name__ == '__main__':
    sys.exit(main())
