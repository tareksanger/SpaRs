"""Check benchmark count and timing contracts independently of measured speed."""
import unittest
from pathlib import Path

from compare_python import Measurements, Options, compare_measurements, reproduction_command, texts_for


class ComparisonTests(unittest.TestCase):
    def test_throughput(self) -> None:
        result = Measurements(.1, 2., 3, 10, .5, 1., 100.)
        self.assertEqual(result.tokens_per_second, 5.)

    def test_invalid_measurements(self) -> None:
        for duration in (0., -1., float('inf'), float('nan')):
            with self.subTest(duration=duration), self.assertRaises(ValueError):
                Measurements(.1, duration, 3, 10, .5, 1., 100.)
        with self.assertRaises(ValueError):
            Measurements(.1, 1., 0, 10, .5, 1., 100.)
        with self.assertRaises(ValueError):
            Measurements(.1, 1., 1, -1, .5, 1., 100.)

    def test_reproduction_command(self) -> None:
        options = Options()
        options.rounds = 7
        options.warmup = 2
        options.output = Path('target/reports/custom.json')
        self.assertEqual(reproduction_command(options), ['.venv/bin/python', 'tools/compare_python.py', '--rounds', '7', '--warmup', '2', '--output', 'target/reports/custom.json'])

    def test_mismatched_workload(self) -> None:
        native = Measurements(.1, 1., 3, 10, .5, 1., 100.)
        for documents, tokens in ((2, 10), (3, 11)):
            with self.subTest(documents=documents, tokens=tokens), self.assertRaises(ValueError):
                compare_measurements(native, Measurements(.1, 1., documents, tokens, .5, 1., 100.))

    def test_corpus_partition(self) -> None:
        self.assertEqual(len(texts_for('short')), 96)
        self.assertEqual(len(texts_for('long')), 2)


if __name__ == '__main__':
    unittest.main()
