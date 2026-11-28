import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys
import unittest
from uuid import uuid4
import zipfile
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
import package_windows

class WindowsPackagingTests(unittest.TestCase):
    def setUp(self):
        self.root = ROOT / 'results/local' / ('package-test-' + uuid4().hex)
        self.root.mkdir(parents=True)
        (self.root / 'packaging').mkdir()
        (self.root / 'scripts').mkdir()
        for name in ('LICENSE', 'packaging/Install-PlaybackRuntime.ps1', 'packaging/WINDOWS-README.txt', 'scripts/install-libmpv.py'):
            (self.root / name).write_text('test fixture', encoding='utf-8')
        self.binary = self.root / 'app.exe'
        self.binary.write_bytes(b'MZ' + b'packaging fixture')
        dep = self.root / 'dependency'
        dep.mkdir()
        (dep / 'Cargo.toml').write_text('', encoding='utf-8')
        (dep / 'LICENSE').write_text('Dependency notice', encoding='utf-8')
        self.metadata = self.root / 'metadata.json'
        self.metadata.write_text(json.dumps(dict(packages=[dict(id='registry-test-dep', name='dep', version='1.0.0', manifest_path=str(dep / 'Cargo.toml'), license='MIT', repository='https://example.org/dep', source='registry+test')])), encoding='utf-8')

    def tearDown(self):
        self.assertEqual(self.root.resolve().parent, (ROOT / 'results/local').resolve())
        shutil.rmtree(self.root)

    def build(self, version='0.0.1-preview.1', commit='a'*40):
        with patch.object(package_windows, 'ROOT', self.root):
            return package_windows.package(self.binary, self.metadata, self.root/'dist', version, commit)

    def test_zip_has_executable_setup_notices_and_matching_checksum(self):
        archive = self.build()
        with zipfile.ZipFile(archive) as bundle:
            names = bundle.namelist()
            self.assertTrue(any(n.endswith('/Foxide.exe') for n in names))
            self.assertTrue(any(n.endswith('/Install-PlaybackRuntime.ps1') for n in names))
            self.assertFalse(any(n.endswith('/scripts/install-libmpv.py') for n in names))
            self.assertTrue(any('/notices/' in n and n.endswith('/LICENSE') for n in names))
            self.assertFalse(any(n.endswith('.dll') for n in names))
            info = json.loads(bundle.read(next(n for n in names if n.endswith('/BUILD-INFO.json'))))
            self.assertEqual(info['source_commit'], 'a'*40)
            self.assertEqual(info['executable_sha256'], package_windows.sha256(self.binary))
            self.assertNotIn(str(self.root), json.dumps(info))
        self.assertEqual((self.root/'dist/SHA256SUMS.txt').read_text().split()[0], package_windows.sha256(archive))
        with self.assertRaises(FileExistsError):
            self.build()

    def test_installer_uses_the_existing_foxide_logo_for_setup_and_wizard_branding(self):
        script = (ROOT / 'packaging/Foxide.iss').read_text(encoding='utf-8')
        self.assertIn('SetupIconFile=', script)
        self.assertIn('WizardImageFile=Foxide-wizard.bmp', script)
        self.assertIn('WizardSmallImageFile=Foxide-small.bmp', script)
        self.assertIn('UninstallDisplayIcon={app}\\Foxide.ico', script)
        self.assertIn('Source: "Foxide.ico"; DestDir: "{app}"', script)
        self.assertIn('IconFilename: "{app}\\Foxide.ico"', script)
        icon_line = next(line for line in script.splitlines() if line.startswith('SetupIconFile='))
        relative_icon = icon_line.partition('=')[2].strip().strip('"')
        icon = (ROOT / 'packaging' / relative_icon).resolve()
        self.assertTrue(icon.is_relative_to((ROOT / 'packaging').resolve()))
        self.assertTrue(icon.is_file())
        data = icon.read_bytes()
        self.assertGreaterEqual(len(data), 22)
        self.assertEqual(data[:4], b'\x00\x00\x01\x00')
        self.assertEqual(int.from_bytes(data[4:6], 'little'), 1)
        payload_size = int.from_bytes(data[14:18], 'little')
        payload_offset = int.from_bytes(data[18:22], 'little')
        png = data[payload_offset:payload_offset + payload_size]
        self.assertEqual(png[:8], b'\x89PNG\r\n\x1a\n')
        self.assertEqual(int.from_bytes(png[16:20], 'big'), 256)
        self.assertEqual(int.from_bytes(png[20:24], 'big'), 256)
        for name, width, height in [('Foxide-wizard.bmp', 164, 314), ('Foxide-small.bmp', 55, 55)]:
            bitmap = (ROOT / 'packaging' / name).read_bytes()
            self.assertEqual(bitmap[:2], b'BM')
            self.assertEqual(int.from_bytes(bitmap[18:22], 'little', signed=True), width)
            self.assertEqual(int.from_bytes(bitmap[22:26], 'little', signed=True), height)

    def test_invalid_version_commit_and_non_windows_file_fail_before_packaging(self):
        for version, commit in [('../escape', 'a'*40), ('0.0.1', 'bad')]:
            with self.subTest(version=version), self.assertRaises(ValueError):
                self.build(version, commit)
        self.binary.write_bytes(b'not a PE executable')
        with self.assertRaises(ValueError):
            self.build()
        self.assertFalse((self.root/'dist').exists())

    def test_registry_notices_with_unix_epoch_timestamps_can_be_packaged(self):
        os.utime(self.root / "dependency/LICENSE", (1, 1))
        archive = self.build()
        with zipfile.ZipFile(archive) as bundle:
            info = next(i for i in bundle.infolist() if "/notices/" in i.filename)
            self.assertGreaterEqual(info.date_time[0], 1980)

    def test_license_file_cannot_escape_dependency_directory(self):
        data = json.loads(self.metadata.read_text())
        data['packages'][0]['license_file'] = '../LICENSE'
        self.metadata.write_text(json.dumps(data), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'escapes dependency'):
            self.build()

if __name__ == '__main__':
    unittest.main()
