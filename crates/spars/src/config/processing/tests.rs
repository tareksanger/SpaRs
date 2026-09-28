use super::*;

#[test]
fn upstream_batch_size_is_typed_and_section_scoped() {
    assert_eq!(parse("").unwrap().batch_size, 1000);
    assert_eq!(parse("[nlp]\nlang = \"en\"\n").unwrap().batch_size, 1000);
    assert_eq!(
        parse("[nlp]\nbatch_size = 256\n[other]\nbatch_size = 99")
            .unwrap()
            .batch_size,
        256
    );
    assert_eq!(
        parse("[nlp.tokenizer]\nbatch_size = 99")
            .unwrap()
            .batch_size,
        1000
    );
    assert_eq!(
        parse(" [nlp] \n batch_size = 17 \n").unwrap().batch_size,
        17
    );
    for value in [
        "0",
        "-1",
        "1.5",
        "true",
        "null",
        "\"256\"",
        "${other.size}",
        "4294967296",
        "",
    ] {
        assert!(
            parse(&format!("[nlp]\nbatch_size = {value}")).is_err(),
            "{value}"
        );
    }
    assert!(parse("[nlp]\nbatch_size=1\nbatch_size=2").is_err());
    assert!(parse("[nlp]\nbatch_size=1\n[nlp]\nbatch_size=2").is_err());
    assert!(serde_json::from_str::<ProcessingDefaults>("null").is_err());
    assert!(serde_json::from_str::<ProcessingDefaults>("42").is_err());
}

#[test]
fn config_delimiters_and_headers_cannot_silently_change_the_section() {
    assert_eq!(
        parse("[nlp] # comment\nbatch_size = 256")
            .unwrap()
            .batch_size,
        256
    );
    assert_eq!(
        parse("[nlp]\n[other] # comment\nbatch_size = 17")
            .unwrap()
            .batch_size,
        1000
    );
    assert_eq!(
        parse("[nlp] ; comment\nbatch_size: 256")
            .unwrap()
            .batch_size,
        256
    );
    assert!(parse("[nlp] trailing text\nbatch_size = 17").is_err());
    assert!(parse("[nlp\nbatch_size = 17").is_err());
}

#[test]
fn malformed_batch_size_declarations_do_not_use_the_default() {
    assert!(parse("[nlp]\nbatch_size").is_err());
    assert!(parse("[nlp]\nbatch_size 17").is_err());
}
