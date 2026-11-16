"""Create the Windows preview ZIP, provenance and checksums; never bundle an unaudited DLL."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import zipfile

ROOT = Path(__file__).resolve().parents[1]
VERSION = re.compile(r'\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?')

def sha256(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

def package(binary, metadata, output, version, commit):
    if not VERSION.fullmatch(version) or not re.fullmatch('[0-9a-f]{40}', commit):
        raise ValueError('Expected semantic version and full source commit SHA')
    if not binary.is_file() or binary.read_bytes()[:2] != b'MZ':
        raise ValueError('Expected a Windows PE executable')
    dependencies = json.loads(metadata.read_text('utf-8-sig'))['packages']
    output.mkdir(parents=True, exist_ok=False)
    name = f'Foxide-{version}-windows-x64'
    archive = output / (name + '.zip')
    inventory = []
    with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED, strict_timestamps=False) as bundle:
        def add(path, destination):
            bundle.write(path, name + '/' + destination)
        add(binary, 'Foxide.exe')
        add(ROOT / 'LICENSE', 'LICENSE')
        add(ROOT / 'scripts/install-libmpv.py', 'scripts/install-libmpv.py')
        add(ROOT / 'packaging/Install-PlaybackRuntime.ps1', 'Install-PlaybackRuntime.ps1')
        add(ROOT / 'packaging/WINDOWS-README.txt', 'README.txt')
        for dependency in dependencies:
            manifest = Path(dependency['manifest_path']).resolve()
            directory = manifest.parent
            identity = hashlib.sha256(dependency['id'].encode()).hexdigest()[:8]
            folder = re.sub(r'[^A-Za-z0-9_.-]', '_', dependency['name'] + '-' + dependency['version']) + '-' + identity
            notices = set()
            for path in directory.iterdir():
                if path.is_file() and path.name.upper().startswith(('LICENSE', 'COPYING', 'NOTICE')):
                    notices.add(path)
                elif path.is_dir() and path.name.upper() in ('LICENSES', 'LICENSE'):
                    notices.update(p for p in path.rglob('*') if p.is_file())
            if dependency.get('license_file'):
                notices.add(directory / dependency['license_file'])
            published = []
            for path in sorted(notices):
                resolved = path.resolve()
                if not resolved.is_relative_to(directory) or path.is_symlink():
                    raise ValueError('License path escapes dependency root')
                relative = path.relative_to(directory).as_posix()
                add(path, 'notices/' + folder + '/' + relative)
                published.append(relative)
            inventory.append(dict(name=dependency['name'], version=dependency['version'],
                license=dependency.get('license'), repository=dependency.get('repository'),
                source=dependency.get('source'), notices=published))
        provenance = dict(version=version, source_commit=commit, architecture='windows-x64',
            executable_sha256=sha256(binary), dependencies=inventory,
            playback_runtime='Installed separately by the pinned, archive-verified installer; no libmpv DLL is redistributed in this ZIP.')
        bundle.writestr(name + '/BUILD-INFO.json', json.dumps(provenance, indent=2)+'\n')
    (output / 'SHA256SUMS.txt').write_text(sha256(archive) + '  ' + archive.name + '\n', encoding='utf-8')
    (output / 'BUILD-INFO.json').write_text(json.dumps(provenance, indent=2)+'\n', encoding='utf-8')
    (output / 'RELEASE_NOTES.md').write_text(
        '# Foxide ' + version + ' — Windows x64 preview\n\n'
        'Extract the ZIP, then run Install-PlaybackRuntime.ps1 once from PowerShell to install the verified playback runtime. Python3.11+ and7-Zip are required for that setup step. Launch Foxide.exe afterward.\n\n'
        'The ZIP includes Foxide, runtime setup scripts, source provenance and dependency notices. The playback DLL is downloaded separately; its redistribution audit remains open. This draft is unsigned and must be reviewed before publication.\n\n'
        'Source commit: ' + commit + '\n', encoding='utf-8')
    return archive

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--metadata', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--version', required=True)
    parser.add_argument('--commit', required=True)
    args = parser.parse_args()
    print(package(args.binary, args.metadata, args.output, args.version, args.commit))

if __name__ == '__main__':
    main()
