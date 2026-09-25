/// Row-major storage for inference batches.
pub(super) struct Dense {
    pub(super) rows: usize,
    pub(super) cols: usize,
    pub(super) data: Vec<f32>,
}

fn checked_len(rows: usize, cols: usize) -> usize {
    let length = rows.checked_mul(cols).expect("matrix dimensions overflow");
    assert!(rows <= isize::MAX as usize && cols <= isize::MAX as usize);
    assert!(length <= isize::MAX as usize / size_of::<f32>());
    length
}

impl Dense {
    pub(super) fn zeroed(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; checked_len(rows, cols)],
        }
    }

    pub(super) fn from_rows(rows: &[Vec<f32>]) -> Self {
        let cols = rows.first().map_or(0, Vec::len);
        let mut result = Self::zeroed(rows.len(), cols);
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row.len(), cols, "matrix rows must have equal lengths");
            result.row_mut(index).copy_from_slice(row);
        }
        result
    }

    pub(super) fn row(&self, index: usize) -> &[f32] {
        assert!(index < self.rows, "matrix row out of bounds");
        let start = index.checked_mul(self.cols).expect("row offset overflow");
        &self.data[start..start + self.cols]
    }

    pub(super) fn row_mut(&mut self, index: usize) -> &mut [f32] {
        assert!(index < self.rows, "matrix row out of bounds");
        let start = index.checked_mul(self.cols).expect("row offset overflow");
        &mut self.data[start..start + self.cols]
    }

    pub(super) fn into_rows(self) -> Vec<Vec<f32>> {
        assert_eq!(self.data.len(), checked_len(self.rows, self.cols));
        (0..self.rows)
            .map(|index| self.row(index).to_vec())
            .collect()
    }
}

/// Multiply a batch by weights stored as output-columns × input-columns.
pub(super) fn product_transposed(input: &Dense, weights: &[f32], output_columns: usize) -> Dense {
    assert_eq!(input.data.len(), checked_len(input.rows, input.cols));
    assert_eq!(weights.len(), checked_len(output_columns, input.cols));
    let mut output = Dense::zeroed(input.rows, output_columns);
    if input.rows == 0 || input.cols == 0 || output_columns == 0 {
        return output;
    }
    let input_stride = isize::try_from(input.cols).expect("input stride overflow");
    let output_stride = isize::try_from(output_columns).expect("output stride overflow");
    // SAFETY: all dimensions, lengths, byte offsets and strides were checked above.
    // A is contiguous rows × cols; B is the transpose of contiguous output × cols.
    // C is a fresh, disjoint allocation with distinct row-major elements. No input
    // aliases C, and every pointer access stays inside its corresponding slice.
    unsafe {
        matrixmultiply::sgemm(
            input.rows,
            input.cols,
            output_columns,
            1.0,
            input.data.as_ptr(),
            input_stride,
            1,
            weights.as_ptr(),
            1,
            input_stride,
            0.0,
            output.data.as_mut_ptr(),
            output_stride,
            1,
        );
    }
    output
}

#[cfg(test)]
mod tests;
