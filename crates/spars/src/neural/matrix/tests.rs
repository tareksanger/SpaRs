use super::{product_transposed, Dense};

fn check_product(rows: usize, inner: usize, columns: usize) {
    let mut state = 173_u32;
    let mut sample = || {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        ((state >> 16) as f32 / 65536.0 - 0.5) * 0.25
    };
    let input = Dense {
        rows,
        cols: inner,
        data: (0..rows * inner).map(|_| sample()).collect(),
    };
    let weights: Vec<f32> = (0..columns * inner).map(|_| sample()).collect();
    let actual = product_transposed(&input, &weights, columns);
    assert_eq!((actual.rows, actual.cols), (rows, columns));
    for row in 0..rows {
        for column in 0..columns {
            // An independent f64 scalar sum checks layout and floating-point error.
            let expected: f64 = (0..inner)
                .map(|k| {
                    f64::from(input.data[row * inner + k]) * f64::from(weights[column * inner + k])
                })
                .sum();
            let error = (f64::from(actual.row(row)[column]) - expected).abs();
            assert!(error <= 2e-5 + 2e-5 * expected.abs());
        }
    }
}

#[test]
fn rectangular_and_multitile_products_match_scalar_oracle() {
    for (rows, inner, columns) in [(1, 3, 2), (7, 11, 5), (37, 97, 65), (129, 301, 193)] {
        check_product(rows, inner, columns);
    }
}

#[test]
fn empty_dimensions_preserve_shape() {
    for (rows, inner, columns) in [(0, 3, 2), (2, 0, 3), (2, 3, 0)] {
        check_product(rows, inner, columns);
    }
    assert_eq!(Dense::zeroed(3, 0).into_rows(), vec![Vec::<f32>::new(); 3]);
}

#[test]
fn row_conversion_and_mutation_preserve_layout() {
    let mut matrix = Dense::from_rows(&[vec![1., 2.], vec![3., 4.]]);
    matrix.row_mut(1)[0] = 5.;
    assert_eq!(matrix.into_rows(), vec![vec![1., 2.], vec![5., 4.]]);
    assert!(Dense::from_rows(&[]).into_rows().is_empty());
}

#[test]
fn malformed_dimensions_are_rejected_before_kernel_call() {
    assert!(std::panic::catch_unwind(|| Dense::from_rows(&[vec![1.], vec![]])).is_err());
    assert!(std::panic::catch_unwind(|| Dense::zeroed(usize::MAX, 2)).is_err());
    assert!(std::panic::catch_unwind(|| Dense::zeroed(0, usize::MAX)).is_err());
    let invalid = Dense {
        rows: 2,
        cols: 3,
        data: vec![0.; 5],
    };
    assert!(std::panic::catch_unwind(|| product_transposed(&invalid, &[0.; 6], 2)).is_err());
    let valid = Dense::zeroed(2, 3);
    assert!(std::panic::catch_unwind(|| product_transposed(&valid, &[0.; 5], 2)).is_err());
    assert!(std::panic::catch_unwind(|| valid.row(2)).is_err());
}
