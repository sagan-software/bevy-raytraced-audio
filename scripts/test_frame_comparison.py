"""Acceptance must reject changes in rendering, acoustic quality, policy, or audio state."""
import copy
import importlib.util
from pathlib import Path
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


if __name__ == '__main__':
    unittest.main()
