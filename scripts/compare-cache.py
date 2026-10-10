#!/usr/bin/env python3
"""Report cached/uncached cost ratios for every dynamic case, without hiding regressions."""
import argparse
import json
import math
from pathlib import Path
from statistics import mean


def main():
    """Read native Criterion estimates or interleaved real-browser samples."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--criterion', type=Path)
    parser.add_argument('--baseline', default='cache-baseline')
    parser.add_argument('--browser', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    rows = {}
    if args.browser:
        data = json.loads(args.browser.read_text())
        if data['hidden']:
            raise ValueError('Hidden browser measurement')
        for row in data['results']:
            item = rows.setdefault(row['name'], {'name': row['name']})
            mode = 'on' if row['cache'] else 'off'
            samples = row['samples_ms']
            if len(samples) < 10 or not all(math.isfinite(x) and x > 0 for x in samples):
                raise ValueError('Invalid or insufficient browser samples')
            item[mode+'_ms'] = mean(samples)
            item[mode+'_statistics'] = row['statistics']
    elif args.criterion:
        for path in args.criterion.rglob(f'{args.baseline}/estimates.json'):
            # Criterion sanitizes slashes in the BenchmarkId function name.
            name, mode = path.parent.parent.parent.name, path.parent.parent.name
            item = rows.setdefault(name, {'name': name})
            estimate = json.loads(path.read_text())['mean']
            item[mode+'_ms'] = estimate['point_estimate']/1e6
            item[mode+'_interval_ms'] = [estimate['confidence_interval'][key]/1e6 for key in ('lower_bound', 'upper_bound')]
    else:
        parser.error('Select --criterion or --browser')
    if len(rows) != 16:
        raise ValueError(f'Expected 16 matched cases, got {len(rows)}')
    for row in rows.values():
        if not all(math.isfinite(row[mode+'_ms']) and row[mode+'_ms'] > 0 for mode in ('on', 'off')):
            raise ValueError('Invalid paired estimate')
        if 'on_interval_ms' in row:
            row['cache_overhead_range_percent'] = [100*(row['on_interval_ms'][0]/row['off_interval_ms'][1]-1),
                                                   100*(row['on_interval_ms'][1]/row['off_interval_ms'][0]-1)]
        row['cache_cost_ratio'] = row['on_ms']/row['off_ms']
        row['cache_overhead_percent'] = 100*(row['cache_cost_ratio']-1)
    report = {'interpretation': 'Positive overhead means caching is slower; negative means faster. No static-weighted aggregate.',
              'cases': sorted(rows.values(), key=lambda row: row['name'])}
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    for row in report['cases']:
        print(f"{row['name']:24} off {row['off_ms']:9.4f} ms  on {row['on_ms']:9.4f} ms  overhead {row['cache_overhead_percent']:+7.2f}%")


if __name__ == '__main__':
    main()
