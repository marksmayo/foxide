"""Remove local account and machine identifiers at the publication boundary.

Local measurement files remain useful for diagnosis; public evidence retains all
measurements, profiles, hashes, dates and source references.
"""
import re

USER_PATH = re.compile(r"""(?i)([a-z]:[\\/]+(?:Users|Documents and Settings)[\\/]+)[^\\/\r\n"<>]+|(/(?:Users|home)/)[^/\r\n"<>]+""")


def public_evidence(collection):
    info = collection.get('session', {})
    alias = str(info.get('system', 'unknown')) + ('-hosted' if info.get('run_id') else '-local')
    identifying = {'host', 'hostname', 'computer_name'}
    opaque = {'system', 'platform', 'profile', 'sha256', 'hash', 'source_commit',
              'commit', 'event', 'id', 'run_id', 'source', 'source_ref'}
    textual = {'path', 'executable', 'command', 'args', 'notes', 'stderr', 'stdout',
               'error', 'message', 'text', 'description', 'reason', 'log', 'cwd',
               'media', 'suite_manifest'}
    hosts = set()

    def gather(value):
        if isinstance(value, dict):
            for key, item in value.items():
                if key in identifying and isinstance(item, str) and item:
                    hosts.add(item)
                else:
                    gather(item)
        elif isinstance(value, list):
            for item in value:
                gather(item)
    gather(collection)
    host_pattern = (re.compile(r'(?<![\w.-])(?:' + '|'.join(
        re.escape(host) for host in sorted(hosts, key=len, reverse=True)) + r')(?![\w.-])', re.IGNORECASE)
        if hosts else None)

    def clean(value, key=None):
        if isinstance(value, dict):
            return {name: alias if name in identifying and isinstance(item, str)
                    else clean(item, name) for name, item in value.items()}
        if isinstance(value, list):
            return [clean(item, key) for item in value]
        if isinstance(value, str):
            if key in opaque or value.startswith(('https://', 'http://')):
                return value
            value = USER_PATH.sub(lambda match: (match[1] or match[2]) + '<user>', value)
            # Only known identifying values are replaced, at complete token/path
            # boundaries. Platform/profile/hash fields and source URLs stay opaque.
            if host_pattern and (key in textual or (key and key.endswith(('_path', '_log')))):
                value = host_pattern.sub(alias, value)
        return value

    return clean(collection)
