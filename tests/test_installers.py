"""Offline adversarial tests for the public installer entry points."""
import contextlib
import io
import sys
import hashlib
import importlib.util
from pathlib import Path
import stat
import subprocess
import shutil
from uuid import uuid4
import types
import unittest
from unittest.mock import patch
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def load(name):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), ROOT / 'scripts' / (name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


mpv = load('install-mpv')
libmpv = load('install-libmpv')


class InstallerTests(unittest.TestCase):
    def setUp(self):
        folder = ROOT / 'results/local'
        folder.mkdir(parents=True, exist_ok=True)
        self.root = folder / ('installer-test-' + uuid4().hex)
        self.root.mkdir()
        self.addCleanup(shutil.rmtree, self.root)
        self.tools = self.root / 'tools'
        self.tools.mkdir()
        # A test must never silently contact the network.
        for module in (mpv, libmpv):
            guard = patch.object(module.urllib.request, 'urlopen', side_effect=AssertionError('network forbidden'))
            guard.start()
            self.addCleanup(guard.stop)

    def archive(self, members=None):
        path = self.root / 'mpv.zip'
        with zipfile.ZipFile(path, 'w') as stream:
            for name, content in (members or {'mpv.exe': b'not executable'}).items():
                stream.writestr(name, content)
        return path, {'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'url': 'https://example.org/pinned.zip'}

    def test_mpv_rejects_changed_download_before_extracting_or_running(self):
        archive, package = self.archive()
        archive.write_bytes(archive.read_bytes() + b'tampered')
        with patch.object(mpv.zipfile, 'ZipFile') as extract, patch.object(mpv.subprocess, 'run') as run:
            with self.assertRaisesRegex(ValueError, 'SHA-256'):
                mpv.install(archive, package, self.tools)
            extract.assert_not_called()
            run.assert_not_called()
        self.assertEqual(list(self.tools.iterdir()), [])

    def test_libmpv_rejects_changed_download_before_running_extractor(self):
        archive = self.root / 'libmpv.7z'
        archive.write_bytes(b'untrusted download')
        with patch.object(libmpv.subprocess, 'run') as run:
            with self.assertRaisesRegex(ValueError, 'SHA-256'):
                libmpv.install(archive, self.tools, Path('7z'))
            run.assert_not_called()
        self.assertEqual(list(self.tools.iterdir()), [])

    def test_mpv_install_publishes_only_a_verified_and_probed_archive(self):
        archive, package = self.archive()
        with patch.object(mpv.subprocess, 'run', return_value=types.SimpleNamespace(returncode=0, stdout='mpv fixture', stderr='')) as run:
            result = mpv.install(archive, package, self.tools)
        self.assertEqual(result['archive_sha256_verified'], package['sha256'])
        self.assertEqual((self.tools / 'mpv/mpv.exe').read_bytes(), b'not executable')
        self.assertTrue((self.tools / 'mpv/foxide-installation.json').is_file())
        self.assertEqual(run.call_args.args[0][1:], ['--version'])
        self.assertEqual([p.name for p in self.tools.iterdir()], ['mpv'])

    def test_mpv_keeps_existing_installation(self):
        archive, package = self.archive()
        target = self.tools / 'mpv'
        target.mkdir()
        (target / 'sentinel').write_bytes(b'existing')
        with self.assertRaises(FileExistsError):
            mpv.install(archive, package, self.tools)
        self.assertEqual((target / 'sentinel').read_bytes(), b'existing')

    def test_mpv_rejects_archive_paths_escaping_stage(self):
        for name in ('../outside', '/absolute', 'C:/absolute', 'folder/../../outside', r'..\outside'):
            with self.subTest(name=name):
                archive, package = self.archive({name: b'payload', 'mpv.exe': b'placeholder'})
                with patch.object(mpv.subprocess, 'run') as run:
                    with self.assertRaises(ValueError):
                        mpv.install(archive, package, self.tools)
                    run.assert_not_called()
                self.assertEqual(list(self.tools.iterdir()), [])

    def test_mpv_rejects_archive_symlinks(self):
        archive, package = self.archive()
        with zipfile.ZipFile(archive, 'a') as stream:
            link = zipfile.ZipInfo('link')
            link.create_system = 3
            link.external_attr = (stat.S_IFLNK | 0o777) << 16
            stream.writestr(link, 'outside')
        package['sha256'] = hashlib.sha256(archive.read_bytes()).hexdigest()
        with self.assertRaisesRegex(ValueError, 'symlinks'):
            mpv.install(archive, package, self.tools)
        self.assertEqual(list(self.tools.iterdir()), [])

    def test_mpv_failed_probe_missing_binary_and_corrupt_zip_leave_no_install(self):
        archive, package = self.archive()
        for result in (types.SimpleNamespace(returncode=1, stdout='', stderr='failed'),
                       types.SimpleNamespace(returncode=0, stdout='another program', stderr=''),
                       subprocess.TimeoutExpired('mpv', 10)):
            with self.subTest(result=result):
                kwargs = {'side_effect': result} if isinstance(result, Exception) else {'return_value': result}
                with patch.object(mpv.subprocess, 'run', **kwargs):
                    with self.assertRaises((RuntimeError, subprocess.TimeoutExpired)):
                        mpv.install(archive, package, self.tools)
                self.assertEqual(list(self.tools.iterdir()), [])
        archive, package = self.archive({'readme': b'missing binary'})
        with self.assertRaisesRegex(ValueError, 'mpv.exe'):
            mpv.install(archive, package, self.tools)
        archive.write_bytes(b'not a zip')
        package['sha256'] = hashlib.sha256(archive.read_bytes()).hexdigest()
        with self.assertRaises(zipfile.BadZipFile):
            mpv.install(archive, package, self.tools)
        self.assertEqual(list(self.tools.iterdir()), [])

    def test_libmpv_success_records_dll_hash_and_never_extracts_archive_paths(self):
        archive = self.root / 'libmpv.7z'
        archive.write_bytes(b'controlled archive')
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        def extract(args, **kwargs):
            self.assertEqual(args[1:3], ['e', '-so'])
            self.assertEqual(args[-1], 'libmpv-2.dll')
            kwargs['stdout'].write(b'controlled dll')
            return types.SimpleNamespace(returncode=0)
        with patch.object(libmpv, 'SHA256', digest), patch.object(libmpv.subprocess, 'run', side_effect=extract):
            target = libmpv.install(archive, self.tools, Path('7z'))
        metadata = libmpv.json.loads((target / 'installation.json').read_text())
        self.assertEqual(metadata['dll_sha256'], hashlib.sha256(b'controlled dll').hexdigest())
        self.assertEqual([p.name for p in self.tools.iterdir()], ['libmpv'])
        with patch.object(libmpv, 'SHA256', digest):
            with self.assertRaises(FileExistsError):
                libmpv.install(archive, self.tools, Path('7z'))

    def test_libmpv_extraction_failure_cleans_staging(self):
        archive = self.root / 'libmpv.7z'
        archive.write_bytes(b'controlled archive')
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        for code in (0, 2):
            with self.subTest(code=code), patch.object(libmpv, 'SHA256', digest), patch.object(libmpv.subprocess, 'run', return_value=types.SimpleNamespace(returncode=code)):
                with self.assertRaises(RuntimeError):
                    libmpv.install(archive, self.tools, Path('7z'))
                self.assertEqual(list(self.tools.iterdir()), [])


    def call_mpv(self, args=(), probe=None):
        with patch.object(mpv, 'ROOT', self.root), patch.object(mpv, 'os', types.SimpleNamespace(name='nt')), \
             patch.object(mpv.sysconfig, 'get_platform', return_value='win-amd64'), \
             patch.object(sys, 'argv', ['install-mpv.py', *map(str, args)]), \
             patch.object(mpv.subprocess, 'run', return_value=probe or types.SimpleNamespace(returncode=0, stdout='mpv fixture', stderr='')), \
             contextlib.redirect_stdout(io.StringIO()):
            return mpv.main()

    def catalog(self, package):
        (self.root / 'benchmarks').mkdir(exist_ok=True)
        (self.root / 'benchmarks/mpv-windows.json').write_text(mpv.json.dumps(package))

    def test_mpv_cli_downloads_and_verifies_before_reporting_success(self):
        archive, package = self.archive()
        self.catalog(package)
        with patch.object(mpv.urllib.request, 'urlopen', return_value=io.BytesIO(archive.read_bytes())) as download:
            self.assertEqual(self.call_mpv(), 0)
        download.assert_called_once_with(package['url'], timeout=30)
        result = mpv.json.loads((self.root / 'results/local/mpv-installation.json').read_text())
        self.assertEqual(result['status'], 'installed')
        self.assertFalse(list((self.tools / 'downloads').glob('*.part')))
        self.assertEqual(self.call_mpv(), 0)

    def test_mpv_cli_records_failed_hash_and_failed_existing_probe(self):
        archive, package = self.archive()
        package['sha256'] = '0' * 64
        self.catalog(package)
        self.assertEqual(self.call_mpv(['--archive', archive]), 1)
        result = mpv.json.loads((self.root / 'results/local/mpv-installation.json').read_text())
        self.assertEqual(result['status'], 'failed')
        self.assertIn('SHA-256', result['error'])
        target = self.tools / 'mpv'
        target.mkdir()
        (target / 'mpv.exe').write_bytes(b'fixture')
        self.assertEqual(self.call_mpv(probe=types.SimpleNamespace(returncode=1)), 1)

    def test_mpv_cli_failed_download_removes_partial_archive(self):
        archive, package = self.archive()
        self.catalog(package)
        class FailedDownload(io.BytesIO):
            def read(self, *args):
                raise OSError('interrupted download')
        with patch.object(mpv.urllib.request, 'urlopen', return_value=FailedDownload(b'payload')):
            self.assertEqual(self.call_mpv(), 1)
        self.assertEqual(list((self.tools / 'downloads').iterdir()), [])

    def test_mpv_cli_rejects_incompatible_platform(self):
        with patch.object(sys, 'argv', ['install-mpv.py']), patch.object(mpv, 'os', types.SimpleNamespace(name='posix')), \
             contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
            mpv.main()
        self.assertEqual(error.exception.code, 2)

    def call_libmpv(self, args=()):
        with patch.object(libmpv, 'ROOT', self.root), patch.object(libmpv.platform, 'system', return_value='Windows'), \
             patch.object(libmpv.platform, 'machine', return_value='AMD64'), \
             patch.object(sys, 'argv', ['install-libmpv.py', *map(str, args)]), contextlib.redirect_stdout(io.StringIO()):
            return libmpv.main()

    def test_libmpv_cli_installs_verified_download_with_local_extractor(self):
        seven = self.tools / '7zip/7z.exe'
        seven.parent.mkdir()
        seven.write_bytes(b'not executed')
        payload = b'controlled archive'
        digest = hashlib.sha256(payload).hexdigest()
        def download(url, path):
            self.assertEqual(url, libmpv.URL)
            path.write_bytes(payload)
        def extract(args, **kwargs):
            kwargs['stdout'].write(b'controlled dll')
            return types.SimpleNamespace(returncode=0)
        with patch.object(libmpv, 'SHA256', digest), patch.object(libmpv.urllib.request, 'urlretrieve', side_effect=download) as retrieve, \
             patch.object(libmpv.subprocess, 'run', side_effect=extract):
            self.call_libmpv()
            retrieve.assert_called_once()
        self.assertEqual((self.tools / 'libmpv/libmpv-2.dll').read_bytes(), b'controlled dll')

    def test_libmpv_cli_accepts_offline_archive_without_download(self):
        seven = self.root / '7z.exe'
        seven.write_bytes(b'not executed')
        archive = self.root / 'offline.7z'
        archive.write_bytes(b'controlled archive')
        def extract(args, **kwargs):
            kwargs['stdout'].write(b'controlled dll')
            return types.SimpleNamespace(returncode=0)
        with patch.object(libmpv, 'SHA256', hashlib.sha256(archive.read_bytes()).hexdigest()), \
             patch.object(libmpv.urllib.request, 'urlretrieve') as download, \
             patch.object(libmpv.subprocess, 'run', side_effect=extract):
            self.call_libmpv(['--archive', archive, '--seven-zip', seven])
            download.assert_not_called()
        self.assertEqual((self.tools / 'libmpv/libmpv-2.dll').read_bytes(), b'controlled dll')

    def test_libmpv_cli_missing_extractor_and_incompatible_platform_fail_early(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
            self.call_libmpv(['--seven-zip', self.root / 'missing'])
        self.assertEqual(error.exception.code, 2)
        with patch.object(sys, 'argv', ['install-libmpv.py']), patch.object(libmpv.platform, 'system', return_value='Linux'), \
             contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
            libmpv.main()
        self.assertEqual(error.exception.code, 2)
