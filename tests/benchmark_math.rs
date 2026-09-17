#[path = "../examples/measure/stats.rs"]
mod stats;

#[test]
fn reported_median_handles_even_odd_and_empty_samples() {
    assert_eq!(stats::median(&[1., 1., 1., 5., 5., 5.]), Some(3.));
    assert_eq!(stats::median(&[1., 2., 8.]), Some(2.));
    assert_eq!(stats::median(&[7.]), Some(7.));
    assert_eq!(stats::median(&[]), None);
}
