"""Install a pinned Windows x64 libmpv development DLL with archive verification."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import shutil
import subprocess
import urllib.request
from uuid import uuid4

ROOT = Path(__file__).resolve().parents[1]
URL = 'https://github.com/shinchiro/mpv-winbuild-cmake/releases/download/20261111/mpv-dev-x86_64-20261111-git-bcde6b93cc.7z'
SHA256 = '273c079b11ea5575822f972756f4c9a71149a5c10130353abeb8586738938452'


def install(archive, tools, seven_zip):
    with archive.open('rb') as stream:
        if hashlib.file_digest(stream, 'sha256').hexdigest() != SHA256:
            raise ValueError('libmpv archive SHA-256 mismatch')
    tools = tools.resolve()
    target = tools / 'libmpv'
    if target.exists():
        raise FileExistsError('Refusing to overwrite tools/libmpv; inspect the existing installation')
    stage = tools / ('.libmpv-stage-' + uuid4().hex)
    stage.mkdir()
    try:
        # Extract only the exact DLL to stdout: no archive-controlled filesystem paths.
        dll = stage / 'libmpv-2.dll'
        with dll.open('wb') as output:
            result = subprocess.run([str(seven_zip), 'e', '-so', str(archive.resolve()), 'libmpv-2.dll'],
                                    stdout=output, stderr=subprocess.PIPE, timeout=120)
        if result.returncode or dll.stat().st_size == 0:
            raise RuntimeError('7-Zip did not extract the expected DLL')
        with dll.open('rb') as stream:
            dll_hash = hashlib.file_digest(stream, 'sha256').hexdigest()
        (stage / 'installation.json').write_text(json.dumps(dict(source=URL, archive_sha256=SHA256,
            dll_sha256=dll_hash, notes='Third-party development build; redistribution license audit required.'), indent=2)+'\n')
        stage.rename(target)
    finally:
        if stage.exists() and stage.resolve().parent == tools:
            shutil.rmtree(stage)
    return target


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path)
    parser.add_argument('--seven-zip', type=Path)
    args = parser.parse_args()
    if platform.system() != 'Windows' or platform.machine().lower() not in ('amd64', 'x86_64'):
        parser.error('This pinned build is Windows x64; use your native package manager on macOS/Linux')
    tools = ROOT / 'tools'
    seven_zip = args.seven_zip or next((p for p in [tools/'7zip/7z.exe', Path('C:/Program Files/7-Zip/7z.exe')]
                                       if p.is_file()), None)
    if seven_zip is None or not seven_zip.is_file():
        parser.error('Install 7-Zip or supply --seven-zip')
    tools.mkdir(exist_ok=True)
    archive = args.archive
    if archive is None:
        downloads = tools / 'downloads'
        downloads.mkdir(exist_ok=True)
        archive = downloads / 'libmpv-20261111.7z'
        if not archive.exists():
            urllib.request.urlretrieve(URL, archive)
    print(install(archive, tools, seven_zip))


if __name__ == '__main__':
    main()
