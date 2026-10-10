#!/usr/bin/env python3
"""Compare three fixed rendered workloads with result caching disabled; retain every run."""
import argparse
import json
import math
from pathlib import Path
from statistics import median

APPLICATIONS = {'showcase', 'stress_2d', 'stress_3d'}


def read_runs(directory):
    """Reject incomparable or invalid runs before considering any aggregate gain."""
    result = {}
    for path in Path(directory).glob('*.json'):
        row = json.loads(path.read_text())
        if 'application' not in row or row.get('cache_enabled') is not False:
            continue
        if row['application'] not in APPLICATIONS:
            raise ValueError(f'Unexpected application: {path}')
        if row.get('hidden', False) or row['profiled'] or row.get('audio_errors'):
            raise ValueError(f'Hidden or profiled timing: {path}')
        samples = row['frame_ms']
        if not samples or not all(math.isfinite(x) and x > 0 for x in samples):
            raise ValueError(f'Invalid frame samples: {path}')
        stats = dict(zip(row['statistics_fields'], row['statistics'], strict=True))
        if (stats['hits'] or stats['unchanged'] or stats['forced'] != stats['computations']
                or stats['forced'] < len(samples)-2):
            raise ValueError(f'Result reuse occurred in uncached run: {path}')
        if row['render_schedules'] < len(samples)-3 or row['processed_sinks'] <= 0:
            raise ValueError(f'Rendering or audio missing: {path}')
        contexts = row.get('audioContexts')
        if contexts is not None and not any(c['state'] == 'running' for c in contexts):
            raise ValueError(f'Browser audio was suspended: {path}')
        row['fps'] = len(samples)*1000/sum(samples)
        row['source_file'] = str(path)
        result.setdefault(row['application'], []).append(row)
    if set(result) != APPLICATIONS or any(len(runs) < 3 for runs in result.values()):
        raise ValueError('Require all three applications and at least three repetitions each')
    return result


def compare(before, after):
    """Equal application weights; a conservative range uses slowest/best whole-run ratios."""
    rows = []
    for app in sorted(APPLICATIONS):
        old, new = before[app], after[app]
        reference = old[0]
        fields = ['quality', 'resolution', 'platform', 'scenario', 'present_mode', 'emitters_2d', 'emitters_3d', 'processed_sinks']
        for run in old+new:
            for field in fields:
                if run[field] != reference[field]:
                    raise ValueError(f'{app}: changed {field}')
            for field in ['userAgent', 'devicePixelRatio', 'gpu']:
                if run.get(field) != reference.get(field):
                    raise ValueError(f'{app}: changed {field}')
            for field in ['warmup_s', 'duration_s', 'interval_s']:
                if run['configuration'].get(field) != reference['configuration'].get(field):
                    raise ValueError(f'{app}: changed sampling configuration {field}')
        old_fps, new_fps = [[run['fps'] for run in runs] for runs in (old, new)]
        rows.append({'application': app, 'before_fps': old_fps, 'after_fps': new_fps,
                     'speedup': median(new_fps)/median(old_fps), 'lower': min(new_fps)/max(old_fps),
                     'upper': max(new_fps)/min(old_fps)})
    mean = lambda key: math.exp(sum(math.log(row[key]) for row in rows)/len(rows))
    return {'applications': rows, 'geomean_speedup': mean('speedup'), 'conservative_lower': mean('lower'),
            'conservative_upper': mean('upper'), 'passed': mean('lower') >= 1.5,
            'required_speedup': 1.5, 'note': 'Range across complete repetitions, not a joint statistical confidence interval.'}


def main():
    """Write every application's result and fail if the complete suite misses the gate."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    report = compare(read_runs(args.before), read_runs(args.after))
    rendered = json.dumps(report, indent=2)+'\n'
    if args.output:
        args.output.write_text(rendered)
    print(rendered)
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
