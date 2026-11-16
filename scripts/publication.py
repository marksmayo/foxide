"""Remove local account and machine identifiers at the publication boundary.

Local measurement files remain useful for diagnosis; public evidence retains all
measurements, profiles, hashes, dates and source references.
"""
import re

USER_PATH = re.compile(r"""(?i)([a-z]:[\\/]+(?:Users|Documents and Settings)[\\/]+)[^\\/\r\n"<>]+|(/(?:Users|home)/)[^/\r\n"<>]+""")


def public_evidence(collection):
    info = collection.get('session', {})
    alias = str(info.get('system', 'unknown')) + ('-hosted' if info.get('run_id') else '-local')

    # Hostnames are identifiers only in host fields. Substring replacement in
    # arbitrary strings corrupts platform names, profiles and opaque hashes.
    def clean(value):
        if isinstance(value, dict):
            return {key: alias if key == 'host' and isinstance(item, str) else clean(item)
                    for key, item in value.items()}
        if isinstance(value, list):
            return [clean(item) for item in value]
        if isinstance(value, str):
            if value.startswith(('https://', 'http://')):
                return value
            value = USER_PATH.sub(lambda match: (match[1] or match[2]) + '<user>', value)
        return value

    return clean(collection)
