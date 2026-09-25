use serde::{Deserialize, Serialize};
use spars::{Doc, Span, Token, TokenMatch, TokenMatcher, TokenPattern};
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Fixture {
    versions: BTreeMap<String, String>,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    id: String,
    text: String,
    tokens: Vec<Token>,
    sentences: Vec<Span>,
    entities: Vec<Span>,
    noun_chunks: Vec<Span>,
    rules: Vec<Rule>,
    expected: Vec<TokenMatch>,
}
#[derive(Deserialize)]
struct Rule {
    name: String,
    patterns: Vec<TokenPattern>,
}
#[derive(Serialize)]
struct Snapshot<'a> {
    format_version: u32,
    document: Storage<'a>,
}
#[derive(Serialize)]
struct Storage<'a> {
    text: &'a str,
    tokens: &'a [Token],
    sentences: &'a [Span],
    entities: &'a [Span],
    noun_chunks: &'a [Span],
}

#[test]
fn official_token_patterns_repetition_and_order() {
    check(
        "../../fixtures/token-match-v1.expected.json",
        10,
        38,
        480,
        704,
    );
    check(
        "../../fixtures/token-match-exhaustive-v1.expected.json",
        31,
        98,
        4805,
        6261,
    );
}
#[test]
fn official_long_branching_patterns() {
    check(
        "../../fixtures/token-match-branching-v1.expected.json",
        31,
        98,
        248,
        288,
    );
}
fn check(path: &str, cases: usize, tokens: usize, rules: usize, matches: usize) {
    let fixture: Fixture = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(fixture.versions["spacy"], "3.8.14");
    assert_eq!(fixture.versions["thinc"], "8.3.13");
    assert_eq!(fixture.cases.len(), cases);
    assert_eq!(
        fixture.cases.iter().map(|c| c.tokens.len()).sum::<usize>(),
        tokens
    );
    assert_eq!(
        fixture.cases.iter().map(|c| c.rules.len()).sum::<usize>(),
        rules
    );
    assert_eq!(
        fixture
            .cases
            .iter()
            .map(|c| c.expected.len())
            .sum::<usize>(),
        matches
    );
    for case in fixture.cases {
        let doc = Doc::from_json(
            &serde_json::to_string(&Snapshot {
                format_version: 1,
                document: Storage {
                    text: &case.text,
                    tokens: &case.tokens,
                    sentences: &case.sentences,
                    entities: &case.entities,
                    noun_chunks: &case.noun_chunks,
                },
            })
            .unwrap(),
        )
        .unwrap();
        let mut matcher = TokenMatcher::new();
        for rule in case.rules {
            matcher.add(rule.name, rule.patterns).unwrap();
        }
        let actual = matcher.find_matches(&doc).unwrap();
        assert_eq!(actual, case.expected, "{}: {}", case.id, case.text);
        assert_eq!(
            matcher.find_matches(&doc).unwrap(),
            actual,
            "repeated {}",
            case.id
        );
        std::thread::scope(|scope| {
            let concurrent = scope.spawn(|| matcher.find_matches(&doc).unwrap());
            assert_eq!(concurrent.join().unwrap(), actual);
        });
    }
}
