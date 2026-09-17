"""Check validation at the untyped reference-library boundary."""
import unittest
from dataclasses import dataclass

import numpy as np

from lexical import string_set
from reference_types import float_rows, integer, strings, uint_rows, is_float_array, predict_docs, predict_array, FloatArray, Doc


@dataclass
class FixedPrediction:
    output: object
    name: str = 'boundary-test'

    def predict(self, inputs: list[Doc] | FloatArray) -> object:
        return self.output


class ReferenceBoundaryTests(unittest.TestCase):
    def test_integer_rejects_strings(self) -> None:
        with self.assertRaises(TypeError):
            integer('96')
        with self.assertRaises(TypeError):
            integer(True)
        self.assertEqual(integer(96), 96)

    def test_string_sequence_checks_every_member(self) -> None:
        self.assertEqual(strings(('NORM', 'PREFIX')), ['NORM', 'PREFIX'])
        for invalid in ['NORM', ['NORM', 96], None]:
            with self.subTest(value=invalid), self.assertRaises(TypeError):
                strings(invalid)

    def test_string_set_checks_every_member(self) -> None:
        self.assertEqual(string_set({'org'}), ['org'])
        with self.assertRaises(TypeError):
            string_set({'org', 7})

    def test_array_guard_rejects_other_dtypes(self) -> None:
        self.assertTrue(is_float_array(np.array([1], dtype=np.float32)))
        self.assertFalse(is_float_array(np.array([1], dtype=np.float64)))
        self.assertFalse(is_float_array(np.array([1], dtype=np.uint64)))
        self.assertFalse(is_float_array([1.0]))

    def test_prediction_validates_shape_dtype_and_document_count(self) -> None:
        inputs = np.zeros((0, 2), dtype=np.float32)
        with self.assertRaises(TypeError):
            predict_array(FixedPrediction(np.zeros((0, 2), dtype=np.uint64)), inputs)
        with self.assertRaises(TypeError):
            predict_array(FixedPrediction(np.zeros(2, dtype=np.float32)), inputs)
        with self.assertRaises(ValueError):
            predict_docs(FixedPrediction([inputs]), [])
        self.assertEqual(predict_docs(FixedPrediction([]), []), [])
        self.assertIs(predict_array(FixedPrediction(inputs), inputs), inputs)

    def test_numeric_rows_preserve_values(self) -> None:
        self.assertEqual(float_rows(np.array([[0.25, -2]], dtype=np.float32)), [[0.25, -2.0]])
        self.assertEqual(uint_rows(np.array([[2**64 - 1]], dtype=np.uint64)), [[2**64 - 1]])


if __name__ == '__main__':
    unittest.main()
