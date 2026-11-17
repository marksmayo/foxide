import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from publication import public_evidence


class PublicationTests(unittest.TestCase):
    def test_account_paths_are_redacted_without_mutating_raw_evidence(self):
        data = dict(session=dict(system='windows', host='private-machine'),
                    command=[json.dumps(r'C:\Users\private-account\escaped.mp4'), r'C:\Users\private-account\movie.mp4', 'C:/Users/private-account/movie.mp4',
                             '/Users/private-account/movie.mp4', '/home/private-account/movie.mp4', r'C:\Users\Private Account\movie.mp4'],
                    cases=[dict(cohort=dict(host='private-machine'), value=12.25, sha256='abc', profile='native/opaque')],
                    measurements=[dict(value=12.25)], count=4, missing=None)
        result = public_evidence(data)
        self.assertNotIn('private-account', str(result))
        self.assertNotIn('Private Account', str(result))
        self.assertNotIn(' Account', str(result))
        self.assertNotIn('private-machine', str(result))
        self.assertIn('private-account', str(data))
        self.assertEqual(result['measurements'], data['measurements'])
        self.assertEqual(result['cases'][0]['value'], 12.25)
        self.assertEqual(result['cases'][0]['profile'], 'native/opaque')
        self.assertEqual(result['cases'][0]['sha256'], 'abc')

    def test_public_source_urls_remain_clickable_and_unchanged(self):
        source = 'https://example.org/home/contributor/media'
        self.assertEqual(public_evidence(dict(source=source))['source'], source)

    def test_public_alias_is_stable_across_refreshes_and_preserves_hosted_identity(self):
        field = dict(session=dict(system='linux', host='machine-id', run_id='42'))
        public = public_evidence(field)
        self.assertEqual(public['session']['host'], 'linux-hosted')
        self.assertEqual(public['session']['run_id'], '42')
        self.assertEqual(public_evidence(public), public)

    def test_hostname_collisions_preserve_field_identity_and_opaque_metadata(self):
        from refresh_scoreboards import field_key
        for host in ('linux', 'a', 'abc'):
            for run_id in (None, '42'):
                with self.subTest(host=host, run_id=run_id):
                    data = dict(
                        session=dict(system='linux', host=host, run_id=run_id,
                                     presentation='native-ui', source_commit='abcdef'),
                        cases=[dict(cohort=dict(host=host),
                                    profile='linux/native/abc', sha256='abcdef',
                                    command=['/home/private-account/movie.mp4'],
                                    source='https://example.org/linux/abc')],
                        measurements=[dict(platform='linux', event='cpu_avc1080',
                                           value=12.25, profile='linux/native/abc')])
                    original = json.loads(json.dumps(data))
                    result = public_evidence(data)
                    alias = 'linux-hosted' if run_id else 'linux-local'
                    self.assertEqual(result['session'], dict(original['session'], host=alias))
                    self.assertEqual(result['cases'][0], dict(
                        original['cases'][0], cohort=dict(host=alias),
                        command=['/home/<user>/movie.mp4']))
                    self.assertEqual(result['measurements'], original['measurements'])
                    self.assertEqual(field_key(result),
                                     ('linux', 'hosted' if run_id else 'local', 'native-ui'))
                    self.assertEqual(public_evidence(result), result)
                    self.assertEqual(data, original)

    def test_hostnames_in_paths_and_diagnostics_are_redacted_without_changing_identifiers(self):
        data = dict(session=dict(system='linux', host='private-pc'),
                    path=r'\\private-pc\media\movie.mp4',
                    notes=['Decoder failed on private-pc', '/mnt/private-pc/movie.mp4'],
                    sha256='private-pc', profile='private-pc/profile', source='https://example.org/private-pc')
        result = public_evidence(data)
        self.assertEqual(result['path'], r'\\linux-local\media\movie.mp4')
        self.assertEqual(result['notes'], ['Decoder failed on linux-local', '/mnt/linux-local/movie.mp4'])
        for key in ('sha256', 'profile', 'source'):
            self.assertEqual(result[key], data[key])
        self.assertEqual(public_evidence(result), result)

    def test_published_comparison_archive_and_inventory_have_no_private_home_paths(self):
        import zipfile
        root = Path(__file__).resolve().parents[1]
        files = [root / 'docs/local-players.md']
        for path in (root / 'results').rglob('*'):
            if 'local' in path.relative_to(root / 'results').parts or not path.is_file():
                continue
            if path.suffix.lower() in ('.json', '.jsonl', '.log', '.html', '.md', '.svg'):
                files.append(path)
            elif path.suffix.lower() == '.zip':
                with zipfile.ZipFile(path) as archive:
                    for member in archive.infolist():
                        text = archive.read(member).decode('utf-8', errors='replace')
                        self.assertTrue(public_evidence(dict(text=text))['text'] == text,
                                         f'Private path in {path.name}/{member.filename}')
        for path in files:
            text = path.read_text(encoding='utf-8').replace(r'\u003c', '<').replace(r'\u003e', '>')
            self.assertTrue(public_evidence(dict(text=text))['text'] == text,
                             f'Private path in {path.relative_to(root)}')

    def test_hostname_alias_does_not_rewrite_unrelated_measurement_labels(self):
        data = dict(session=dict(system='linux', host='a'),
                    measurements=[dict(algorithm='a', fixture='a', unit='a')],
                    notes=['Failure on a'], path='/mnt/a/movie.mp4')
        result = public_evidence(data)
        self.assertEqual(result['measurements'], data['measurements'])
        self.assertEqual(result['notes'], ['Failure on linux-local'])
        self.assertEqual(result['path'], '/mnt/linux-local/movie.mp4')
