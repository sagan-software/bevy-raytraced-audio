#!/usr/bin/env python3
"""Run real rendered applications serially, alternating cache policies across repetitions."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess

APPLICATIONS = ('showcase', 'stress_2d', 'stress_3d')
ENVIRONMENT_KEYS = ('DISPLAY', 'WAYLAND_DISPLAY', 'WAYLAND_SOCKET', 'WGPU_BACKEND',
                    'VK_DRIVER_FILES', 'BEVY_TASK_THREADS', 'ALSA_PLUGIN_DIR',
                    'ALSA_CONFIG_PATH', 'PIPEWIRE_ALSA', 'PIPEWIRE_LATENCY', 'PIPEWIRE_QUANTUM')


def binary_hash(path):
    """Record the executable actually measured, independently of working-tree provenance."""
    with path.open('rb') as binary:
        return hashlib.file_digest(binary, 'sha256').hexdigest()


def cpu_identity():
    """Exclude current CPU frequency and utilization from hardware matching."""
    if platform.system() != 'Linux':
        return platform.processor()
    rows = json.loads(subprocess.check_output(['lscpu', '--json'], text=True, env=dict(os.environ, LC_ALL='C')))['lscpu']
    fields = {'Architecture:', 'CPU(s):', 'Vendor ID:', 'Model name:', 'CPU family:',
              'Model:', 'Stepping:', 'Thread(s) per core:', 'Core(s) per socket:', 'Socket(s):'}
    return {row['field']: row['data'] for row in rows if row['field'] in fields}


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
    if any(args.output.glob('*-off-*.json')) or any(args.output.glob('*-on-*.json')):
        parser.error('Refusing to overwrite an existing frame measurement directory')
    metadata = {'label': args.label, 'platform': platform.platform(), 'machine': platform.machine(),
                'cpu_count': os.cpu_count(), 'git': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                'dirty': subprocess.check_output(['git', 'status', '--porcelain'], text=True),
                'rustc': subprocess.check_output(['rustc', '-Vv'], text=True),
                'environment': {key: os.environ.get(key) for key in ENVIRONMENT_KEYS},
                'cpu_identity': cpu_identity(),
                'binaries_sha256': {app: binary_hash(args.bin_dir/app)
                                    for app in ([args.application] if args.application else APPLICATIONS)}}
    (args.output/'environment.json').write_text(json.dumps(metadata, indent=2)+'\n')
    applications = [args.application] if args.application else APPLICATIONS
    modes = {'off': [False], 'on': [True], 'both': [False, True]}[args.cache]
    invalid = []
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
                assert report['cache_enabled'] == enabled and report['render_schedules'] > 0
                assert report['processed_sinks'] > 0, 'No processed audio is playing'
                if report['audio_errors']:
                    invalid.append(stem)
                print(f"  {report['fps']:.2f} FPS, {len(report['frame_ms'])} frame intervals, "
                      f"{len(report['audio_errors'])} audio errors", flush=True)
    if invalid:
        raise SystemExit(f'Audio underruns invalidate these runs (all retained): {", ".join(invalid)}')


if __name__ == '__main__':
    main()
