from pathlib import Path
import hashlib
import importlib.util
import json
import shutil
import uuid
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('prepare_media', ROOT / 'scripts/prepare-event-media.py')
prepare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)


class PreparedMediaTests(unittest.TestCase):
    def test_restored_fixture_requires_both_content_hash_and_seed_identity(self):
        folder = ROOT / 'results/local' / ('restored-' + uuid.uuid4().hex)
        folder.mkdir(parents=True)
        try:
            media = folder / 'clip.bin'
            media.write_bytes(b'fixture')
            fixture = dict(id='A', path='clip.bin', source_sha256='seed', sha256=hashlib.sha256(b'fixture').hexdigest())
            (folder / 'manifest.json').write_text(json.dumps({'fixtures': [fixture]}))
            prepare.verify_restored(folder, {'A': 'seed'})
            with self.assertRaises(ValueError):
                prepare.verify_restored(folder, {'A': 'different-seed'})
            media.write_bytes(b'changed')
            with self.assertRaises(ValueError):
                prepare.verify_restored(folder, {'A': 'seed'})
        finally:
            shutil.rmtree(folder)
