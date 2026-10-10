#!/usr/bin/env python3
"""Compare identical real-browser workloads; reject hidden runs and changed environments."""
import argparse
import json
import math
from pathlib import Path


def compare(before, after):
    """Return equal-weight speedups without conflating different browsers or CPU counts."""
    for field in ['userAgent', 'hardwareConcurrency']:
        if before[field] != after[field]:
            raise ValueError(f'Environment changed: {field}')
    if before['hidden'] or after['hidden']:
        raise ValueError('A timing run was hidden')
    suites = []
    for report in [before, after]:
        suite = {row['name']: row for row in report['results']}
        if not suite or len(suite) != len(report['results']):
            raise ValueError('Empty or duplicated inventory')
        for row in suite.values():
            if not all(math.isfinite(x) and x > 0 for x in row['samples_ms']):
                raise ValueError('Invalid browser timing sample')
        suites.append(suite)
    old, new = suites
    if old.keys() != new.keys():
        raise ValueError('Benchmark inventory changed')
    rows = [{'case': key, 'before_ms': old[key]['median_ms'], 'after_ms': new[key]['median_ms'],
             'speedup': old[key]['median_ms']/new[key]['median_ms']} for key in sorted(old)]
    return {'case_count': len(rows), 'geomean_speedup': math.exp(sum(math.log(r['speedup']) for r in rows)/len(rows)), 'cases': rows}


def main():
    """Write every ratio alongside the aggregate; this is a descriptive browser comparison."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    report = compare(json.loads(args.before.read_text()), json.loads(args.after.read_text()))
    rendered = json.dumps(report, indent=2)+'\n'
    if args.output:
        args.output.write_text(rendered)
    print(rendered)


if __name__ == '__main__':
    main()
