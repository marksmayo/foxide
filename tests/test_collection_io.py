import json
from pathlib import Path
import sys
import tempfile
import uuid
import shutil
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from collection_io import EventTail


class CollectionIOTests(unittest.TestCase):
    def test_final_partial_record_invalidates_evidence_and_non_objects_are_errors(self):
        scratch = Path(__file__).resolve().parents[1] / 'results/local'
        scratch.mkdir(parents=True, exist_ok=True)
        path = scratch / ('partial-' + uuid.uuid4().hex)
        path.write_bytes(b'[]\n{"kind":')
        tail = EventTail(path)
        try:
            self.assertEqual(tail.read(final=True), [{'kind': 'collector-error'}]*2)
            self.assertEqual(tail.read(final=True), [{'kind': 'collector-error'}]*2)
        finally:
            tail.close()
            path.unlink()

    def test_tail_preserves_partial_records_and_never_duplicates_old_events(self):
        scratch = Path(__file__).resolve().parents[1] / 'results/local'
        scratch.mkdir(parents=True, exist_ok=True)
        folder = scratch / ('tail-' + uuid.uuid4().hex)
        folder.mkdir()
        try:
            path = folder / 'events.jsonl'
            tail = EventTail(path)
            self.assertEqual(tail.read(), [])
            path.write_bytes(b'{"kind":"stats"}\n{"kind":')
            self.assertEqual(tail.read(), [{'kind': 'stats'}])
            self.assertEqual(tail.read(), [{'kind': 'stats'}])
            with path.open('ab') as out:
                out.write(b'"end-file"}\ninvalid\n')
            self.assertEqual(tail.read(), [{'kind': 'stats'}, {'kind': 'end-file'}, {'kind': 'collector-error'}])
            self.assertEqual(tail.read(), tail.read())
            tail.close()
        finally:
            shutil.rmtree(folder)


if __name__ == '__main__':
    unittest.main()
