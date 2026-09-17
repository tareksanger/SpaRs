"""Check exact annotations, numerical limits, shapes, and diagnostic paths."""
import unittest

from check_reference import Counts, NumericalKind, ReferenceMismatch, compare
from json_types import JsonValue


class ReferenceComparisonTests(unittest.TestCase):
    def assert_mismatch(self, expected: JsonValue, actual: JsonValue, path: tuple[str | int, ...], reason: str) -> None:
        with self.assertRaises(ReferenceMismatch) as caught:
            compare(expected, actual, Counts(), path)
        self.assertIn(reason, caught.exception.mismatch.reason)
        self.assertIn('expected', str(caught.exception))
        self.assertIn('actual', str(caught.exception))

    def test_allowed_numerical_differences_and_counts(self) -> None:
        cases = [
            (('cases', 0, 'tok2vec', 0, 0), NumericalKind.ACTIVATION, 3e-5),
            ((0, 'ner_stages', 0, 0, 0), NumericalKind.ACTIVATION, 3e-5),
            ((0, 'parser', 0, 'scores', 0), NumericalKind.SCORE, 2.1e-4),
            (('cases', 0, 'vector', 0), NumericalKind.VECTOR, 3e-6),
        ]
        for path, kind, delta in cases:
            with self.subTest(kind=kind):
                counts = Counts()
                compare(1.0, 1.0 + delta, counts, path)
                self.assertEqual(counts.numbers[kind].compared, 1)
                self.assertAlmostEqual(counts.numbers[kind].maximum_absolute_difference, delta)
                self.assert_mismatch(1.0, 1.0 + delta * 2, path, 'exceeds')

    def test_discrete_values_are_never_tolerant(self) -> None:
        for expected, actual in [(1000000, 1000001), (True, False), ('NN', 'VB')]:
            self.assert_mismatch(expected, actual, ('cases', 0, 'tok2vec', 0), 'discrete values')
        self.assert_mismatch(1, True, (), 'types')
        self.assert_mismatch(1, 1.0, (), 'types')

    def test_unknown_float_field_requires_exact_equality(self) -> None:
        self.assert_mismatch(1.0, 1.0000000001, ('unknown', 'scores'), 'exceeds')

    def test_nonfinite_values_are_rejected(self) -> None:
        for value in [float('nan'), float('inf'), float('-inf')]:
            self.assert_mismatch(value, value, ('cases', 0, 'vector', 0), 'nonfinite')
            self.assert_mismatch(0.0, value, (), 'nonfinite')

    def test_shapes_and_fields_are_exact(self) -> None:
        self.assert_mismatch([[1.0]], [[1.0, 2.0]], ('cases', 0, 'tok2vec'), 'lengths')
        self.assert_mismatch({'tag': 'NN'}, {'tag': 'NN', 'pos': 'NOUN'}, (), 'fields')
        self.assert_mismatch([1], {'0': 1}, (), 'types')

    def test_nested_error_identifies_first_changed_path(self) -> None:
        expected: JsonValue = {'tokens': [{'tag': 'NN'}, {'tag': 'VB'}]}
        actual: JsonValue = {'tokens': [{'tag': 'NN'}, {'tag': 'NN'}]}
        with self.assertRaises(ReferenceMismatch) as caught:
            compare(expected, actual, Counts())
        self.assertEqual(caught.exception.mismatch.path, '$["tokens"][1]["tag"]')
        self.assertEqual(caught.exception.mismatch.expected, "'VB'")
        self.assertEqual(caught.exception.mismatch.actual, "'NN'")


if __name__ == '__main__':
    unittest.main()
