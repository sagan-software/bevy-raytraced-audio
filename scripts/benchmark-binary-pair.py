#!/usr/bin/env python3
"""Interleave preserved native workload drivers with identical operation counts and checksums."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import subprocess
import time


def measure(binary, name, iterations):
    """Include construction and process startup; this complements Criterion's loop timings."""
    start = time.perf_counter_ns()
    output = subprocess.check_output([str(binary), name, str(iterations)], text=True, timeout=30).strip()
    elapsed_ms = (time.perf_counter_ns()-start)/1e6
    checksum = float(output)
    if not math.isfinite(checksum):
        raise ValueError(f'Nonfinite acoustic output: {name}')
    return elapsed_ms, checksum


def main():
    """Retain every paired sample and refuse to overwrite a previous experiment."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--prefix', default='', help='Select a prefix of the frozen shared workload inventory')
    parser.add_argument('--samples', type=int, default=30)
    parser.add_argument('--batch-ms', type=float, default=200)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('Refusing to overwrite existing measurements')
    if args.samples < 10 or not math.isfinite(args.batch_ms) or args.batch_ms < 50:
        parser.error('Require at least ten samples and a calibration target of at least 50 ms')
    source = Path('tests/performance/src/lib.rs').read_text().split('pub const CASES:')[1].split('];', 1)[0]
    cases = [name for name in re.findall(r'"([^"]+)"', source) if name.startswith(args.prefix)]
    if not cases:
        parser.error('Empty workload selection')
    binaries = [args.before.resolve(), args.after.resolve()]
    report = dict(schema=1, measurement_kind='whole_process_native',
                  sequence_alignment='identical-batches-fresh-processes',
                  platform=platform.platform(), cpu_count=os.cpu_count(), prefix=args.prefix,
                  samples=args.samples, calibration_target_ms=args.batch_ms,
                  git=subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                  dirty=subprocess.check_output(['git', 'status', '--porcelain'], text=True),
                  rustc=subprocess.check_output(['rustc', '-Vv'], text=True),
                  binaries_sha256=[hashlib.sha256(binary.read_bytes()).hexdigest() for binary in binaries],
                  results=[])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    for name in cases:
        iterations = 1
        while iterations < 16777216:
            elapsed, _ = measure(binaries[0], name, iterations)
            if elapsed >= args.batch_ms:
                break
            iterations *= 2
        # Both binaries receive the same fresh state, warmup and calibrated batch size.
        for binary in binaries:
            measure(binary, name, iterations)
        times = [[], []]
        checksums = [[], []]
        for sample in range(args.samples):
            for mode in ([0, 1] if sample % 2 == 0 else [1, 0]):
                elapsed, checksum = measure(binaries[mode], name, iterations)
                times[mode].append(elapsed/iterations)
                checksums[mode].append(checksum)
            if checksums[0][-1] != checksums[1][-1]:
                raise ValueError(f'Acoustic output changed: {name}')
        report['results'].append(dict(name=name, iterations=iterations,
                                     before_ms=times[0], after_ms=times[1],
                                     before_checksums=checksums[0], after_checksums=checksums[1]))
        # A stopped or failed experiment retains completed cases and is explicitly incomplete.
        report['complete'] = len(report['results']) == len(cases)
        args.output.write_text(json.dumps(report, indent=2)+'\n')
        print(f'{name}: {sum(times[0])/sum(times[1]):.3f}x throughput', flush=True)


if __name__ == '__main__':
    main()
