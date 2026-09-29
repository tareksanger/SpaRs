use super::*;
#[test]
fn typed_options_reject_unsupported_attributes() {
    assert!(serde_json::from_str::<PhraseAttribute>("\"LEMMA\"").is_err());
    assert_eq!(
        serde_json::from_str::<PhraseAttribute>("\"ORTH\"").unwrap(),
        PhraseAttribute::Orth
    );
    assert_eq!(
        PhraseMatcher::with_attribute(PhraseAttribute::Text).attribute(),
        PhraseAttribute::Text
    );
}
#[test]
fn symbol_ids_follow_spacy() {
    assert_eq!(rule_key(""), 0);
    assert_eq!(rule_key("IS_ALPHA"), 1);
    assert_eq!(rule_key("ORTH"), 65);
}
fn doc(text: &str) -> Doc {
    Doc {
        text: text.into(),
        tokens: text
            .char_indices()
            .enumerate()
            .map(|(cp, (i, c))| crate::Token::new(i, i + c.len_utf8(), cp, c.to_string()))
            .collect(),
        entities: None,
        sentences: None,
        noun_chunks: None,
        tensor: Vec::new(),
        dependency_index: OnceLock::new(),
    }
}
#[test]
fn lookup_is_unique_nonempty_ordered_and_resets() {
    let mut matcher = PhraseMatcher::new();
    matcher
        .add("r", &[&doc("b"), &doc("a"), &doc("b"), &doc("")])
        .unwrap();
    matcher.add("r", &[&doc("c"), &doc("a")]).unwrap();
    assert_eq!(
        matcher
            .get("r")
            .unwrap()
            .iter()
            .map(|p| p.tokens().to_vec())
            .collect::<Vec<_>>(),
        vec![vec!["b"], vec!["a"], vec!["c"]]
    );
    matcher.remove("r").unwrap();
    assert!(matcher.get("r").is_none());
    matcher.add("r", &[&doc("a")]).unwrap();
    assert_eq!(matcher.get("r").unwrap()[0].tokens(), &["a"]);
    assert_eq!(matcher.get("r").unwrap().len(), 1);
}
#[test]
fn text_alias_is_exact_and_annotation_independent() {
    let mut orth = PhraseMatcher::new();
    let mut text = PhraseMatcher::with_attribute(PhraseAttribute::Text);
    for matcher in [&mut orth, &mut text] {
        matcher.add("r", &[&doc("é\n🙂")]).unwrap();
    }
    let mut input = doc("É\n🙂é\n🙂");
    input.tokens[4].sentence_start = Some(true);
    let expected = vec![PhraseMatch {
        rule: "r".into(),
        start: TokenIndex(3),
        end: TokenIndex(6),
    }];
    assert_eq!(orth.find_matches(&input).unwrap(), expected);
    assert_eq!(text.find_matches(&input).unwrap(), expected);
    let restored = Doc::from_json(&input.to_json().unwrap()).unwrap();
    assert_eq!(text.find_matches(&restored).unwrap(), expected);
}
#[test]
fn collision_rejection_is_atomic_and_recoverable() {
    let mut matcher = PhraseMatcher::new();
    matcher.add("r", &[&doc("a")]).unwrap();
    let before = matcher.find_matches(&doc("ab")).unwrap();
    let patterns = matcher.get("r").unwrap().to_vec();
    matcher
        .identities
        .insert(rule_key("candidate"), "different".into());
    assert!(matches!(
        matcher.add("candidate", &[&doc("b")]),
        Err(Error::Pattern(_))
    ));
    assert_eq!(matcher.len(), 1);
    assert_eq!(matcher.get("r").unwrap(), patterns);
    assert!(!matcher.contains("candidate"));
    assert_eq!(matcher.find_matches(&doc("ab")).unwrap(), before);
    matcher.add("usable", &[&doc("b")]).unwrap();
    assert_eq!(matcher.find_matches(&doc("ab")).unwrap().len(), 2);
}
#[test]
fn malformed_document_rejected_before_registration() {
    let matcher = PhraseMatcher::new();
    let invalid = doc("é")
        .to_json()
        .unwrap()
        .replace("\"end\":2", "\"end\":1");
    assert!(Doc::from_json(&invalid).is_err());
    assert!(matcher.is_empty());
}
#[test]
fn long_patterns_drop_and_remove_without_recursive_stack_growth() {
    let input = doc(&"a".repeat(10000));
    let mut matcher = PhraseMatcher::new();
    matcher.add("long", &[&input]).unwrap();
    matcher.remove("long").unwrap();
    assert!(matcher.nodes[0].children.is_empty());
    assert!(matcher
        .nodes
        .iter()
        .all(|n| n.children.is_empty() && n.terminal.is_none()));
}

#[test]
fn populated_deep_trie_drops_on_small_stack() {
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let input = doc(&"a".repeat(10000));
            let mut matcher = PhraseMatcher::new();
            matcher.add("deep", &[&input]).unwrap();
            drop(matcher);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn lower_lookup_uses_unique_normalized_keys_and_preserves_exact_mode() {
    let lower_attribute = serde_json::from_str::<PhraseAttribute>("\"LOWER\"").unwrap();
    let mut lower = PhraseMatcher::with_attribute(lower_attribute);
    lower.add("r", &[&doc("Ab"), &doc("ab")]).unwrap();
    assert_eq!(lower.get("r").unwrap().len(), 1);
    assert_eq!(lower.get("r").unwrap()[0].tokens(), &["a", "b"]);
    assert_eq!(lower.find_matches(&doc("AB")).unwrap().len(), 1);
    let mut exact = PhraseMatcher::new();
    exact.add("r", &[&doc("ab")]).unwrap();
    assert!(exact.find_matches(&doc("AB")).unwrap().is_empty());
}
