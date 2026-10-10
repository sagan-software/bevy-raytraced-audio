#!/usr/bin/env python3
"""Run real rendered applications serially, alternating cache policies across repetitions."""
import argparse
import json
import os
from pathlib import Path
import platform
import subprocess

APPLICATIONS = ('showcase', 'stress_2d', 'stress_3d')


def main():
    """Keep raw frame times, GPU/audio startup logs, configuration, and source provenance."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--label', required=True)
    parser.add_argument('--bin-dir', type=Path, default=Path('target/application-bench/examples'))
    parser.add_argument('--repetitions', type=int, default=3)
    parser.add_argument('--warmup', type=float, default=15)
    parser.add_argument('--duration', type=float, default=20)
    parser.add_argument('--cache', choices=('off', 'on', 'both'), default='both')
    parser.add_argument('--application', choices=APPLICATIONS)
    args = parser.parse_args()
    if args.repetitions < 1 or args.duration <= 0 or args.warmup < 0:
        parser.error('Invalid sampling duration/repetitions')
    args.output.mkdir(parents=True, exist_ok=True)
    metadata = {'label': args.label, 'platform': platform.platform(), 'machine': platform.machine(),
                'cpu_count': os.cpu_count(), 'git': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                'dirty': subprocess.check_output(['git', 'status', '--porcelain'], text=True),
                'rustc': subprocess.check_output(['rustc', '-Vv'], text=True),
                'environment': {key: os.environ.get(key) for key in ('DISPLAY', 'WAYLAND_DISPLAY', 'WGPU_BACKEND', 'VK_DRIVER_FILES', 'BEVY_TASK_THREADS')}}
    (args.output/'environment.json').write_text(json.dumps(metadata, indent=2)+'\n')
    applications = [args.application] if args.application else APPLICATIONS
    modes = {'off': [False], 'on': [True], 'both': [False, True]}[args.cache]
    for repetition in range(args.repetitions):
        for application in applications:
            for enabled in modes[::1 if repetition % 2 == 0 else -1]:
                stem = f'{application}-{"on" if enabled else "off"}-{repetition}'
                output = (args.output/f'{stem}.json').resolve()
                config = {'cache': enabled, 'warmup_s': args.warmup, 'duration_s': args.duration,
                          'output': str(output), 'label': args.label, 'interval_s': 0.0}
                env = dict(os.environ, ACOUSTIC_FRAME_BENCH=json.dumps(config),
                           BEVY_ASSET_ROOT=str(Path('examples/raytraced-audio').resolve()))
                print(f'Measuring {stem}', flush=True)
                with (args.output/f'{stem}.log').open('w') as log:
                    subprocess.run([str((args.bin_dir/application).resolve())], env=env, stdout=log,
                                   stderr=subprocess.STDOUT, check=True, timeout=args.warmup+args.duration+120)
                report = json.loads(output.read_text())
                measured_log = (args.output/f'{stem}.log').read_text().split('ACOUSTIC_MEASUREMENT_STARTED', 1)
                if len(measured_log) != 2:
                    raise ValueError('Measurement-window marker missing')
                report['audio_errors'] = [line for line in measured_log[1].splitlines() if 'audio stream error' in line]
                output.write_text(json.dumps(report)+'\n')
                assert not report['audio_errors'], 'Audio underrun during measurement; keep this run as invalid evidence'
                assert report['cache_enabled'] == enabled and report['render_schedules'] > 0
                assert report['processed_sinks'] > 0, 'No processed audio is playing'
                print(f"  {report['fps']:.2f} FPS, {len(report['frame_ms'])} frame intervals", flush=True)


if __name__ == '__main__':
    main()
