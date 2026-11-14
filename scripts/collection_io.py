"""Incremental complete-line event collection; raw evidence remains on disk."""
import json


def write_json(path, value, compact=False):
    """Stream serialization without a second full-sized in-memory text copy."""
    with path.open('w', encoding='utf-8') as stream:
        json.dump(value, stream, allow_nan=False, indent=None if compact else 2,
                  separators=(',', ':') if compact else None)
        stream.write('\n')


class EventTail:
    def __init__(self, path):
        self.path = path
        self.stream = None
        self.pending = b''
        self.events = []

    def read(self, final=False):
        if self.stream is None:
            try:
                self.stream = self.path.open('rb')
            except FileNotFoundError:
                return self.events
        data = self.stream.read()
        if data:
            lines = (self.pending + data).split(b'\n')
            self.pending = lines.pop()
            for line in lines:
                try:
                    event = json.loads(line.decode('utf-8'))
                    self.events.append(event if isinstance(event, dict) else {'kind': 'collector-error'})
                except (UnicodeError, ValueError):
                    self.events.append({'kind': 'collector-error'})
        if final and self.pending:
            self.events.append({'kind': 'collector-error'})
            self.pending = b''
        return self.events

    def close(self):
        if self.stream:
            self.stream.close()
