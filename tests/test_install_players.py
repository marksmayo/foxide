import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('installer', ROOT / 'scripts/install-players.py')
installer = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(installer)


class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.packages = [{'player':'one','id':'Vendor.One','source':'winget'}, {'player':'two','id':'Vendor.Two','source':'winget'}]

    def test_plan_never_runs_installers(self):
        def fail(command):
            raise AssertionError('A plan must not invoke subprocesses')
        result = installer.install_packages(self.packages, 'winget', invoke=fail)
        self.assertEqual([r['status'] for r in result], ['planned', 'planned'])

    def test_manager_failure_blocks_remaining_without_false_success(self):
        calls = []
        def invoke(command):
            calls.append(command)
            return {'exit_code': 0x8A150001}
        result = installer.install_packages(self.packages, 'winget', execute=True, invoke=invoke)
        self.assertEqual([r['status'] for r in result], ['blocked-package-manager'] * 2)
        self.assertEqual(len(calls), 2)  # list + one genuine install attempt, not a repeated batch of failures

    def test_install_requires_a_successful_post_install_verification(self):
        results = iter([{'exit_code': 1}, {'exit_code': 0}, {'exit_code': 1}])
        actual = installer.install_packages(self.packages[:1], 'winget', execute=True, invoke=lambda command: next(results))
        self.assertEqual(actual[0]['status'], 'install-unverified')

    def test_existing_executable_is_preserved(self):
        def fail(command):
            raise AssertionError('Existing player must not be changed')
        result = installer.install_packages(self.packages[:1], 'winget', execute=True, invoke=fail, detected={'one':'C:/players/one.exe'})
        self.assertEqual(result[0]['status'], 'already-present-executable')

    def test_exact_id_and_source_without_disabling_security_checks(self):
        command = installer.install_command('winget', self.packages[0])
        self.assertIn('--exact', command)
        self.assertIn('--source', command)
        self.assertNotIn('--ignore-security-hash', command)
        self.assertNotIn('--ignore-local-archive-malware-scan', command)


if __name__ == '__main__':
    unittest.main()
