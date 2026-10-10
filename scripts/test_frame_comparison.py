"""Acceptance must reject changes in rendering, acoustic quality, policy, or audio state."""
import copy
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location('compare_frames', Path(__file__).with_name('compare-frames.py'))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
DRIVER_SPEC = importlib.util.spec_from_file_location('benchmark_frames', Path(__file__).with_name('benchmark-frames.py'))
DRIVER = importlib.util.module_from_spec(DRIVER_SPEC)
DRIVER_SPEC.loader.exec_module(DRIVER)


class FrameComparisonTests(unittest.TestCase):
    """Changing workloads or collecting only a fast application cannot satisfy the gate."""

    def fixture(self, fps):
        """Construct three identical repetitions for every required application."""
        row = dict(fps=fps, quality={'ray_count': 96}, resolution=[1280, 800], platform={'os': 'linux'},
                   scenario='active-v1', present_mode='AutoNoVsync', emitters_2d=0, emitters_3d=20,
                   processed_sinks=8, gpu='test', configuration={'warmup_s': 15, 'duration_s': 20, 'interval_s': 0})
        return {app: [copy.deepcopy(row) for _ in range(3)] for app in MODULE.APPLICATIONS}

    def test_gate_uses_all_applications(self):
        before, after = self.fixture(100), self.fixture(200)
        self.assertTrue(MODULE.compare(before, after)['passed'])
        for row in after['showcase']:
            row['fps'] = 50
        self.assertFalse(MODULE.compare(before, after)['passed'])

    def test_best_run_cannot_hide_slow_repetitions(self):
        before, after = self.fixture(100), self.fixture(200)
        for rows in after.values():
            rows[0]['fps'] = 90
        self.assertFalse(MODULE.compare(before, after)['passed'])

    def test_environment_and_quality_changes_rejected(self):
        for field, value in [('resolution', [640, 400]), ('quality', {'ray_count': 16}), ('processed_sinks', 0), ('gpu', 'different')]:
            before, after = self.fixture(100), self.fixture(200)
            after['showcase'][0][field] = value
            with self.assertRaises(ValueError):
                MODULE.compare(before, after)

    def test_paused_and_audio_suspended_runs_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            for app, rows in self.fixture(100).items():
                for i, row in enumerate(rows):
                    row.update(application=app, cache_enabled=False, profiled=False,
                               frame_ms=[10]*2000, render_schedules=2000,
                               statistics_fields=['hits', 'unchanged', 'forced', 'computations'],
                               statistics=[0, 0, 2000, 2000], audio_continuous=True)
                    Path(directory, f'{app}-{i}.json').write_text(json.dumps(row))
            MODULE.read_runs(directory)
            path = Path(directory, 'showcase-0.json')
            valid = json.loads(path.read_text())
            for changes in ({'frame_ms': []}, {'frame_ms': [10]},
                            {'frame_ms': [10]*1800+[2000]}, {'audio_continuous': False},
                            {'audio_errors': ['underrun']}, {'hidden': True},
                            {'external_interference': 'concurrent filesystem inventory'}):
                path.write_text(json.dumps(dict(valid, **changes)))
                with self.assertRaises(ValueError):
                    MODULE.read_runs(directory)

    def test_native_audio_and_hardware_configuration_must_match(self):
        with tempfile.TemporaryDirectory() as before, tempfile.TemporaryDirectory() as after:
            metadata = dict(platform='Linux', machine='x86_64', cpu_count=12,
                            cpu_identity={'Model name:': 'test'}, rustc='fixed toolchain',
                            environment={'PIPEWIRE_ALSA': None})
            old, new = [Path(directory, 'environment.json') for directory in (before, after)]
            old.write_text(json.dumps(metadata))
            with self.assertRaises(ValueError):
                MODULE.compare_environments(before, after)
            new.write_text(json.dumps(metadata))
            MODULE.compare_environments(before, after)
            for changes in ({'cpu_identity': {'Model name:': 'different'}},
                            {'environment': {'PIPEWIRE_ALSA': 'changed'}}):
                new.write_text(json.dumps(dict(metadata, **changes)))
                with self.assertRaises(ValueError):
                    MODULE.compare_environments(before, after)

    def test_reference_driver_alternates_builds_and_preserves_each_run(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for build in ('old', 'new'):
                (root/build).mkdir()
                for app in DRIVER.APPLICATIONS:
                    (root/build/app).write_bytes(build.encode())
            order = []

            def run(command, *, env, stdout, **_):
                config = json.loads(env['ACOUSTIC_FRAME_BENCH'])
                order.append(Path(command[0]).parent.name)
                Path(config['output']).write_text(json.dumps(dict(
                    cache_enabled=config['cache'], render_schedules=100, processed_sinks=8,
                    fps=100, frame_ms=[10]*100)))
                stdout.write('ACOUSTIC_MEASUREMENT_STARTED\n')

            def metadata(command, **_):
                return '{"lscpu":[]}' if command[0] == 'lscpu' else 'test metadata'

            argv = ['benchmark-frames.py', str(root/'measurements'), '--label', 'test',
                    '--bin-dir', str(root/'new'), '--reference-bin-dir', str(root/'old'),
                    '--cache', 'off', '--repetitions', '2']
            with mock.patch('sys.argv', argv), mock.patch.object(DRIVER.subprocess, 'run', run), \
                    mock.patch.object(DRIVER.subprocess, 'check_output', metadata), \
                    mock.patch.object(DRIVER.platform, 'platform', return_value='test platform'), \
                    mock.patch.object(DRIVER, 'cpu_identity', return_value='test CPU'), \
                    contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                DRIVER.main()
                with self.assertRaises(SystemExit):
                    DRIVER.main()  # Existing evidence must never be silently overwritten.
            self.assertEqual(order, ['old', 'new', 'new', 'old', 'old', 'new',
                                     'new', 'old', 'old', 'new', 'new', 'old'])
            for build in ('before', 'after'):
                self.assertEqual(len(list((root/'measurements'/build).glob('*-off-*.json'))), 6)


if __name__ == '__main__':
    unittest.main()
