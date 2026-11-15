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
