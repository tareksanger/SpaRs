use super::*;
#[test]
fn oversized_manifest_fails_before_reading() {
    let path = std::env::temp_dir().join(format!("spars-huge-manifest-{}", std::process::id()));
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(64 * 1024 * 1024 + 1).unwrap();
    drop(file);
    assert!(read_manifest(&path).is_err());
    std::fs::remove_file(path).unwrap();
}
