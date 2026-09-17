use serde::{Deserialize, Serialize};
use spars::{DependencyMatch, DependencyMatcher, DependencyPattern, Doc, Span, Token};
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
    expected: Vec<DependencyMatch>,
}
#[derive(Deserialize)]
struct Rule {
    name: String,
    patterns: Vec<DependencyPattern>,
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
fn official_dependency_pattern_order_and_annotations() {
    check("fixtures/dependency-match-v1.expected.json", 6, Some(52));
    check(
        "fixtures/dependency-match-regressions-v1.expected.json",
        2,
        None,
    );
}
fn check(path: &str, cases: usize, additions: Option<usize>) {
    let fixture: Fixture = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(fixture.versions["spacy"], "3.8.14");
    assert_eq!(fixture.versions["thinc"], "8.3.13");
    assert_eq!(fixture.cases.len(), cases);
    let mut comparisons = 0;
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
        let mut matcher = DependencyMatcher::new();
        if let Some(additions) = additions {
            assert_eq!(case.rules.len(), additions);
        }
        comparisons += case.rules.len();
        for rule in case.rules {
            matcher.add(rule.name, rule.patterns).unwrap();
        }
        if additions.is_some() {
            assert_eq!(matcher.len(), 51);
        }
        let actual = matcher.find_matches(&doc).unwrap();
        assert_eq!(actual, case.expected, "{}", case.id);
        assert_eq!(
            matcher.find_matches(&doc).unwrap(),
            actual,
            "repeated {}",
            case.id
        );
        std::thread::scope(|scope| {
            let first = scope.spawn(|| matcher.find_matches(&doc).unwrap());
            assert_eq!(first.join().unwrap(), actual);
        });
    }
    assert!(comparisons > 0);
    if additions.is_some() {
        assert_eq!(comparisons, 312);
    }
}
