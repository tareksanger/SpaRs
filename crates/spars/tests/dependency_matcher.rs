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
    check(
        "../../fixtures/dependency-match-v1.expected.json",
        6,
        Some(52),
    );
    check(
        "../../fixtures/dependency-match-regressions-v1.expected.json",
        2,
        None,
    );
}
#[test]
#[ignore = "requires official export; mandatory CI"]
fn official_lexical_flag_conditions() {
    let path = "../../fixtures/dependency-match-flags-v1.expected.json";
    let fixture: Fixture = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let total = |count: fn(&Case) -> usize| fixture.cases.iter().map(count).sum::<usize>();
    assert_eq!(total(|c| c.tokens.len()), 65);
    assert_eq!(total(|c| c.rules.len()), 42);
    assert_eq!(total(|c| c.expected.len()), 376);
    for size in ["sm", "md", "lg"] {
        let lexicon = spars::Model::load(format!("../../assets/en_core_web_{size}-3.8.0"))
            .unwrap()
            .lexicon();
        check_with(path, 3, None, Some(&lexicon));
    }
}
#[test]
#[ignore = "requires official export; mandatory CI"]
fn reused_flag_matcher_keeps_documents_independent() {
    let model = spars::Model::load("../../assets/en_core_web_md-3.8.0").unwrap();
    let pattern: DependencyPattern = serde_json::from_str(
        r#"{"nodes":[{"id":"n","constraints":[{"attribute":"like_num","predicate":{"kind":"flag","value":true}}],"link":null}]}"#,
    )
    .unwrap();
    let mut matcher = DependencyMatcher::with_lexicon(model.lexicon());
    matcher.add("number", vec![pattern]).unwrap();
    let found = |text: &str| {
        let doc = model.process(text).unwrap();
        matcher
            .find_matches(&doc)
            .unwrap()
            .into_iter()
            .map(|m| m.tokens.iter().map(|t| t.0).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    };
    assert_eq!(found("ten apples"), [vec![0]]);
    assert_eq!(found("I ate 5 of ten"), [vec![2], vec![4]]);
    assert_eq!(found("ten apples"), [vec![0]]);
}
#[test]
fn official_unicode_lower_conditions() {
    let path = "../../fixtures/dependency-match-lower-v1.expected.json";
    let fixture: Fixture = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let total = |count: fn(&Case) -> usize| fixture.cases.iter().map(count).sum::<usize>();
    assert_eq!(total(|c| c.tokens.len()), 43);
    assert_eq!(total(|c| c.rules.len()), 160);
    assert_eq!(total(|c| c.expected.len()), 223);
    check(path, 4, None);
}
fn check(path: &str, cases: usize, additions: Option<usize>) {
    check_with(path, cases, additions, None);
}
fn check_with(
    path: &str,
    cases: usize,
    additions: Option<usize>,
    lexicon: Option<&spars::Lexicon>,
) {
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
        let mut matcher = lexicon.map_or_else(DependencyMatcher::new, |lexicon| {
            DependencyMatcher::with_lexicon(lexicon.clone())
        });
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
