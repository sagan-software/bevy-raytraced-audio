"""The workload statistics must match paired cache policies and preserve finite results."""
import json
from pathlib import Path
import sys

rows = json.loads(Path(sys.argv[1]).read_text())
assert len(rows) == 32
pairs = {}
for row in rows:
    assert row['statistics']['hits'] + row['statistics']['computations'] == row['iterations']
    pairs.setdefault(row['name'], {})[row['cache']] = row
for name, pair in pairs.items():
    off, on = pair[False], pair[True]
    assert off['checksum'] == on['checksum'], name
    assert off['statistics']['forced'] == off['iterations'], name
    assert off['statistics']['hits'] == 0, name
    if not name.endswith(('static', 'mixed_idle')):
        assert on['statistics']['hits'] == 0, name
print(f'Validated {len(pairs)} paired mutation sequences and invalidation counts')
