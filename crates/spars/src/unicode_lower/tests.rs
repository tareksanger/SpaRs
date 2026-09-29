use super::*;

#[test]
fn lower_sigma_context_scales_across_long_ignorable_sequences() {
    let ignorables = "\u{301}".repeat(10000);
    assert_eq!(lower(&format!("AΣ{ignorables}")), format!("aς{ignorables}"));
    assert_eq!(
        lower(&format!("AΣ{ignorables}A")),
        format!("aσ{ignorables}a")
    );
    assert_eq!(lower("İẞ𐐀"), "i\u{307}ß𐐨");
    assert_eq!(lower("ßﬀ"), "ßﬀ");
}
