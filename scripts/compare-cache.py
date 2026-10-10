#!/usr/bin/env python3
"""Report cached/uncached cost ratios for every dynamic case, without hiding regressions."""
import argparse
import json
import math
from pathlib import Path
import random
from statistics import mean


def main():
    """Read native Criterion estimates or interleaved real-browser samples."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--criterion', type=Path)
    parser.add_argument('--baseline', default='cache-baseline')
    parser.add_argument('--browser', '--paired', dest='paired', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    rows = {}
    alignment = 'criterion-adaptive-sequences'
    if args.paired:
        data = json.loads(args.paired.read_text())
        alignment = data.get('sequence_alignment', 'independent-calibration')
        if data['hidden']:
            raise ValueError('Hidden browser measurement')
        for row in data['results']:
            item = rows.setdefault(row['name'], {'name': row['name']})
            mode = 'on' if row['cache'] else 'off'
            if mode+'_ms' in item:
                raise ValueError('Duplicate paired policy')
            samples = row['samples_ms']
            if len(samples) < 10 or not all(math.isfinite(x) and x > 0 for x in samples):
                raise ValueError('Invalid or insufficient paired samples')
            item[mode+'_ms'] = mean(samples)
            item[mode+'_samples_ms'] = samples
            item[mode+'_statistics'] = row['statistics']
            item[mode+'_checksum'] = row['checksum']
            item[mode+'_counter_frames'] = row['counterFrames']
    elif args.criterion:
        for path in args.criterion.rglob(f'{args.baseline}/estimates.json'):
            # Criterion sanitizes slashes in the BenchmarkId function name.
            name, mode = path.parent.parent.parent.name, path.parent.parent.name
            item = rows.setdefault(name, {'name': name})
            estimate = json.loads(path.read_text())['mean']
            item[mode+'_ms'] = estimate['point_estimate']/1e6
            item[mode+'_interval_ms'] = [estimate['confidence_interval'][key]/1e6 for key in ('lower_bound', 'upper_bound')]
    else:
        parser.error('Select --criterion, --browser or --paired')
    if len(rows) != 16:
        raise ValueError(f'Expected 16 matched cases, got {len(rows)}')
    for row in rows.values():
        if not all(math.isfinite(row[mode+'_ms']) and row[mode+'_ms'] > 0 for mode in ('on', 'off')):
            raise ValueError('Invalid paired estimate')
        if 'on_interval_ms' in row:
            row['cache_overhead_range_percent'] = [100*(row['on_interval_ms'][0]/row['off_interval_ms'][1]-1),
                                                   100*(row['on_interval_ms'][1]/row['off_interval_ms'][0]-1)]
        if 'on_samples_ms' in row:
            on, off = [row[mode+'_samples_ms'] for mode in ('on', 'off')]
            if len(on) != len(off) or row['on_counter_frames'] != row['off_counter_frames'] or row['on_checksum'] != row['off_checksum']:
                raise ValueError('Mismatched paired samples or acoustic results')
            # Resample paired indices together to retain short-term frequency/thermal covariance.
            rng = random.Random(0)
            ratios = []
            for _ in range(5000):
                indices = rng.choices(range(len(on)), k=len(on))
                ratios.append(sum(on[i] for i in indices)/sum(off[i] for i in indices))
            ratios.sort()
            row['paired_bootstrap_overhead_interval_percent'] = [100*(ratios[i]-1) for i in (125, 4874)]
        row['cache_cost_ratio'] = row['on_ms']/row['off_ms']
        row['cache_overhead_percent'] = 100*(row['cache_cost_ratio']-1)
    report = {'sequence_alignment': alignment,
              'limitation': None if alignment == 'common-batches-reset-after-calibration' else 'Adaptive batches can sample different phases of motion; use aligned interleaved results to isolate cache overhead.',
              'interpretation': 'Positive overhead means caching is slower; negative means faster. No static-weighted aggregate.',
              'cases': sorted(rows.values(), key=lambda row: row['name'])}
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    for row in report['cases']:
        print(f"{row['name']:24} off {row['off_ms']:9.4f} ms  on {row['on_ms']:9.4f} ms  overhead {row['cache_overhead_percent']:+7.2f}%")


if __name__ == '__main__':
    main()
