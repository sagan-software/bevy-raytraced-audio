"""Acceptance must reject changes in rendering, acoustic quality, policy, or audio state."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('compare_frames', Path(__file__).with_name('compare-frames.py'))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


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
                            {'audio_errors': ['underrun']}, {'hidden': True}):
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


if __name__ == '__main__':
    unittest.main()
