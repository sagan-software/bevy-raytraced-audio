"""Regression tests for performance claims and suite integrity."""
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("compare", Path(__file__).with_name("compare-benchmarks.py"))
COMPARE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COMPARE)


class ComparisonTests(unittest.TestCase):
    """Ensure regressions and missing cases cannot be hidden by aggregation."""

    def test_reciprocal_speedups_cancel(self):
        report = COMPARE.compare({"a": [4, 4, 4], "b": [1, 1, 1]}, {"a": [1, 1, 1], "b": [4, 4, 4]})
        self.assertEqual(report["geomean_speedup"], 1)

    def test_missing_case_fails(self):
        with self.assertRaises(ValueError):
            COMPARE.compare({"a": [1, 1, 1]}, {})

    def test_uncertainty_is_conservative(self):
        report = COMPARE.compare({"a": [150, 140, 160]}, {"a": [100, 90, 110]})
        self.assertEqual(report["geomean_speedup"], 1.5)
        self.assertLess(report["conservative_lower"], 1.5)


BROWSER_SPEC = importlib.util.spec_from_file_location('browser', Path(__file__).with_name('compare-browser-benchmarks.py'))
BROWSER = importlib.util.module_from_spec(BROWSER_SPEC)
BROWSER_SPEC.loader.exec_module(BROWSER)


class BrowserComparisonTests(unittest.TestCase):
    """Hidden tabs and browser changes cannot silently become performance evidence."""

    def fixture(self):
        return {'userAgent': 'browser', 'hardwareConcurrency': 4, 'hidden': False,
                'results': [{'name': 'test', 'median_ms': 1, 'samples_ms': [1]*30}]}

    def test_same_report_has_unity_speed(self):
        self.assertEqual(BROWSER.compare(self.fixture(), self.fixture())['geomean_speedup'], 1)

    def test_hidden_run_rejected(self):
        hidden = self.fixture()
        hidden['hidden'] = True
        with self.assertRaises(ValueError):
            BROWSER.compare(self.fixture(), hidden)

    def test_browser_change_rejected(self):
        changed = self.fixture()
        changed['userAgent'] = 'different browser'
        with self.assertRaises(ValueError):
            BROWSER.compare(self.fixture(), changed)
