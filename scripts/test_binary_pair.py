"""Verify the native paired runner's ordering, completeness and output-parity guards."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location('binary_pair', Path(__file__).with_name('benchmark-binary-pair.py'))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class BinaryPairTests(unittest.TestCase):
    """An attractive timing is invalid if the work or acoustic output differs."""

    def fixture(self, root):
        """Use real file hashes while mocking subprocess timing and metadata."""
        before, after, output = [root/name for name in ('before', 'after', 'result.json')]
        before.write_bytes(b'before')
        after.write_bytes(b'after')
        argv = ['benchmark-binary-pair.py', str(before), str(after), str(output),
                '--prefix', 'hrtf/44100/moving', '--samples', '10', '--batch-ms', '50']
        return argv, output

    def test_alternates_equal_batches_and_retains_every_sample(self):
        with tempfile.TemporaryDirectory() as directory:
            argv, output = self.fixture(Path(directory))
            calls = []

            def measure(binary, name, iterations):
                calls.append((binary.name, iterations))
                return 100., 42.

            with mock.patch('sys.argv', argv), mock.patch.object(MODULE, 'measure', measure), \
                    mock.patch.object(MODULE.subprocess, 'check_output', return_value='metadata'), \
                    contextlib.redirect_stdout(io.StringIO()):
                MODULE.main()
            report = json.loads(output.read_text())
            self.assertTrue(report['complete'])
            row = report['results'][0]
            self.assertEqual(len(row['before_ms']), 10)
            self.assertEqual(row['before_checksums'], row['after_checksums'])
            expected = [('before', 1), ('before', 1), ('after', 1)]
            for sample in range(10):
                expected += [(name, 1) for name in (['before', 'after'] if sample % 2 == 0 else ['after', 'before'])]
            self.assertEqual(calls, expected)
            with mock.patch('sys.argv', argv), contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit):
                    MODULE.main()
            self.assertEqual(json.loads(output.read_text()), report)

    def test_mismatched_acoustic_output_cannot_complete(self):
        with tempfile.TemporaryDirectory() as directory:
            argv, output = self.fixture(Path(directory))

            def measure(binary, *_):
                return 100., float(binary.name == 'after')

            with mock.patch('sys.argv', argv), mock.patch.object(MODULE, 'measure', measure), \
                    mock.patch.object(MODULE.subprocess, 'check_output', return_value='metadata'):
                with self.assertRaisesRegex(ValueError, 'Acoustic output changed'):
                    MODULE.main()
            self.assertFalse(output.exists())


if __name__ == '__main__':
    unittest.main()
