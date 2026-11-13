use serde::{Deserialize, Serialize};
use spars::{Doc, Greedy, Span, Token, TokenMatch, TokenMatcher, TokenPattern, TokenRuleOptions};
use std::collections::BTreeMap;
mod support;

#[derive(Deserialize)]
struct Fixture<C = Case> {
    versions: BTreeMap<String, String>,
    cases: Vec<C>,
}
#[derive(Deserialize)]
struct Case {
    id: String,
    text: String,
    tokens: Vec<Token>,
    sentences: Vec<Span>,
    noun_chunks: Vec<Span>,
    rules: Vec<Rule>,
    expected: Vec<TokenMatch>,
}
#[derive(Deserialize, PartialEq)]
struct Rule {
    name: String,
    patterns: Vec<TokenPattern>,
    // Recorded only by the greedy fixture; the other fixtures register every rule without it.
    greedy: Option<RecordedGreedy>,
}
/// The `greedy` argument of spaCy's `Matcher.add`, in spaCy's spelling.
#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
enum RecordedGreedy {
    First,
    Longest,
}
impl From<RecordedGreedy> for Greedy {
    fn from(recorded: RecordedGreedy) -> Self {
        match recorded {
            RecordedGreedy::First => Self::First,
            RecordedGreedy::Longest => Self::Longest,
        }
    }
}
/// A case of the greedy fixture: `case.expected` is spaCy's result of the registrations as
/// recorded, `unfiltered` its result when every registration is made without `greedy`.
#[derive(Deserialize)]
struct GreedyCase {
    #[serde(flatten)]
    case: Case,
    unfiltered: Vec<TokenMatch>,
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
    entities: Vec<Span>,
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
#[test]
fn official_length_conditions() {
    check(
        "../../fixtures/token-match-length-v1.expected.json",
        3,
        28,
        105,
        386,
    );
}
#[test]
fn official_set_comparisons() {
    check(
        "../../fixtures/token-match-sets-v1.expected.json",
        3,
        21,
        558,
        653,
    );
}
#[test]
fn official_unicode_lower_conditions() {
    check(
        "../../fixtures/token-match-lower-v1.expected.json",
        4,
        45,
        172,
        280,
    );
}
#[test]
#[ignore = "requires official export; mandatory CI"]
fn official_lexical_flag_conditions() {
    // Flags depend only on token text, so every supported English size shares the
    // same expectations. Each lexicon stays valid after its model is dropped.
    for size in ["sm", "md", "lg"] {
        let lexicon = spars::Model::load(format!("../../assets/en_core_web_{size}-3.8.0"))
            .unwrap()
            .lexicon();
        check_with(
            "../../fixtures/token-match-flags-v1.expected.json",
            FLAG_CASES,
            FLAG_TOKENS,
            FLAG_RULES,
            FLAG_MATCHES,
            Some(&lexicon),
        );
    }
}
#[test]
#[ignore = "requires official export; mandatory CI"]
fn official_more_lexical_flag_conditions() {
    for size in ["sm", "md", "lg"] {
        let lexicon = spars::Model::load(format!("../../assets/en_core_web_{size}-3.8.0"))
            .unwrap()
            .lexicon();
        check_with(
            "../../fixtures/token-match-more-flags-v1.expected.json",
            3,
            98,
            99,
            1228,
            Some(&lexicon),
        );
    }
}
const FLAG_CASES: usize = 3;
const FLAG_TOKENS: usize = 65;
const FLAG_RULES: usize = 51;
const FLAG_MATCHES: usize = 442;
#[test]
#[ignore = "requires official export; mandatory CI"]
fn reused_flag_matcher_keeps_documents_independent() {
    let nlp = spars::Model::load("../../assets/en_core_web_md-3.8.0").unwrap();
    let pattern: TokenPattern = serde_json::from_str(
        r#"{"tokens":[{"constraints":[{"attribute":"like_num","predicate":{"kind":"flag","value":true}}],"repetition":{"kind":"once"}}]}"#,
    )
    .unwrap();
    let mut matcher = TokenMatcher::with_lexicon(nlp.lexicon());
    matcher.add("number", vec![pattern]).unwrap();
    let found = |text: &str| {
        let doc = nlp.process(text).unwrap();
        matcher
            .find_matches(&doc)
            .unwrap()
            .into_iter()
            .map(|m| (m.start.0, m.end.0))
            .collect::<Vec<_>>()
    };
    assert_eq!(found("ten apples"), [(0, 1)]);
    assert_eq!(found("I ate 5 of ten"), [(2, 3), (4, 5)]);
    assert_eq!(found("no numbers"), []);
    assert_eq!(found("ten apples"), [(0, 1)]);
}
#[test]
fn reused_length_matcher_with_mixed_rules_keeps_documents_independent() {
    let snapshot = |words: &[&str]| {
        let text = words.join(" ");
        let mut start = 0;
        let tokens: Vec<Token> = words
            .iter()
            .map(|word| {
                let idx = text[..start].chars().count();
                let token: Token = serde_json::from_value(serde_json::json!({
                    "start": start, "end": start + word.len(), "idx": idx,
                    "whitespace": start + word.len() < text.len(), "norm": word,
                }))
                .unwrap();
                start += word.len() + 1;
                token
            })
            .collect();
        Doc::from_json(
            &serde_json::json!({"format_version": 1, "document": {"text": text, "tokens": tokens}})
                .to_string(),
        )
        .unwrap()
    };
    let pattern = |raw: &str| -> TokenPattern { serde_json::from_str(raw).unwrap() };
    let mut matcher = TokenMatcher::new();
    // Only the second rule needs lengths; the per-call values must still include them.
    matcher
        .add(
            "text",
            vec![pattern(
                r#"{"tokens":[{"constraints":[{"attribute":"text","predicate":{"kind":"equals","value":"x"}}],"repetition":{"kind":"once"}}]}"#,
            )],
        )
        .unwrap();
    matcher
        .add(
            "long",
            vec![pattern(
                r#"{"tokens":[{"constraints":[{"attribute":"length","predicate":{"kind":"compare","operator":">=","value":3}}],"repetition":{"kind":"once"}}]}"#,
            )],
        )
        .unwrap();
    let found = |doc: &Doc| {
        matcher
            .find_matches(doc)
            .unwrap()
            .into_iter()
            .map(|m| (m.rule, m.start.0, m.end.0))
            .collect::<Vec<_>>()
    };
    let first = snapshot(&["ab", "abcd"]);
    let second = snapshot(&["abcd", "x", "abc"]);
    assert_eq!(found(&first), [("long".into(), 1, 2)]);
    assert_eq!(
        found(&second),
        [
            ("long".into(), 0, 1),
            ("text".into(), 1, 2),
            ("long".into(), 2, 3)
        ]
    );
    assert_eq!(found(&first), [("long".into(), 1, 2)]);
}
#[test]
fn length_on_a_later_item_uses_that_items_checks() {
    let words = ["x", "ab", "x", "abcd"];
    let text = words.join(" ");
    let mut start = 0;
    let tokens: Vec<Token> = words
        .iter()
        .map(|word| {
            let idx = text[..start].chars().count();
            let token: Token = serde_json::from_value(serde_json::json!({
                "start": start, "end": start + word.len(), "idx": idx,
                "whitespace": start + word.len() < text.len(), "norm": word,
            }))
            .unwrap();
            start += word.len() + 1;
            token
        })
        .collect();
    let doc = Doc::from_json(
        &serde_json::json!({"format_version": 1, "document": {"text": text, "tokens": tokens}})
            .to_string(),
    )
    .unwrap();
    let x = r#"{"constraints":[{"attribute":"text","predicate":{"kind":"equals","value":"x"}}],"repetition":{"kind":"once"}}"#;
    let skipped = r#"{"constraints":[{"attribute":"text","predicate":{"kind":"equals","value":"x"}}],"repetition":{"kind":"range","min":0,"max":0}}"#;
    let long = r#"{"constraints":[{"attribute":"length","predicate":{"kind":"compare","operator":">=","value":3}}],"repetition":{"kind":"once"}}"#;
    for (name, items, expected) in [
        ("plain_then_long", format!("[{x},{long}]"), vec![(2, 4)]),
        (
            "zero_count_first",
            format!("[{skipped},{x},{long}]"),
            vec![(2, 4)],
        ),
    ] {
        let pattern: TokenPattern =
            serde_json::from_str(&format!(r#"{{"tokens":{items}}}"#)).unwrap();
        let mut matcher = TokenMatcher::new();
        matcher.add(name, vec![pattern]).unwrap();
        let found: Vec<_> = matcher
            .find_matches(&doc)
            .unwrap()
            .into_iter()
            .map(|m| (m.start.0, m.end.0))
            .collect();
        assert_eq!(found, expected, "{name}");
    }
}
#[test]
fn reused_lower_matcher_keeps_documents_independent() {
    let snapshot = |words: &[&str]| {
        let text = words.join(" ");
        let mut start = 0;
        let tokens: Vec<Token> = words
            .iter()
            .map(|word| {
                let idx = text[..start].chars().count();
                let token: Token = serde_json::from_value(serde_json::json!({
                    "start": start, "end": start + word.len(), "idx": idx,
                    "whitespace": start + word.len() < text.len(), "norm": word,
                }))
                .unwrap();
                start += word.len() + 1;
                token
            })
            .collect();
        Doc::from_json(
            &serde_json::json!({"format_version": 1, "document": {"text": text, "tokens": tokens}})
                .to_string(),
        )
        .unwrap()
    };
    let pattern: TokenPattern = serde_json::from_str(
        r#"{"tokens":[{"constraints":[{"attribute":"lower","predicate":{"kind":"equals","value":"ritz"}}],"repetition":{"kind":"once"}}]}"#,
    )
    .unwrap();
    let mut matcher = TokenMatcher::new();
    matcher.add("ritz", vec![pattern]).unwrap();
    let first = snapshot(&["the", "RITZ"]);
    let second = snapshot(&["ΟΣ", "Ritz", "x", "ritz"]);
    let found = |doc: &Doc| {
        matcher
            .find_matches(doc)
            .unwrap()
            .into_iter()
            .map(|m| (m.start.0, m.end.0))
            .collect::<Vec<_>>()
    };
    assert_eq!(found(&first), [(1, 2)]);
    assert_eq!(found(&second), [(1, 2), (3, 4)]);
    assert_eq!(found(&snapshot(&["the"])), []);
    assert_eq!(found(&first), [(1, 2)]);
}
fn check(path: &str, cases: usize, tokens: usize, rules: usize, matches: usize) {
    check_with(path, cases, tokens, rules, matches, None);
}
fn check_with(
    path: &str,
    cases: usize,
    tokens: usize,
    rules: usize,
    matches: usize,
    lexicon: Option<&spars::Lexicon>,
) {
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
        let doc = document(&case);
        let mut matcher = lexicon.map_or_else(TokenMatcher::new, |lexicon| {
            TokenMatcher::with_lexicon(lexicon.clone())
        });
        for rule in case.rules {
            // Only the greedy fixture records registrations with `greedy`.
            assert!(rule.greedy.is_none(), "{}: {}", case.id, rule.name);
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
fn document(case: &Case) -> Doc {
    Doc::from_json(
        &serde_json::to_string(&Snapshot {
            format_version: 1,
            document: Storage {
                text: &case.text,
                tokens: &case.tokens,
                sentences: &case.sentences,
                entities: support::entities_from_tags(&case.tokens),
                noun_chunks: &case.noun_chunks,
            },
        })
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn official_greedy_registrations_made_without_options_return_every_match() {
    for GreedyCase { case, unfiltered } in greedy_fixture().cases {
        let doc = document(&case);
        let mut matcher = TokenMatcher::new();
        for rule in case.rules {
            matcher.add(rule.name, rule.patterns).unwrap();
        }
        assert_same_matches(
            &format!("{}: {:?}", case.id, case.text),
            &matcher.find_matches(&doc).unwrap(),
            &unfiltered,
        );
    }
}
#[test]
fn official_greedy_first_and_longest_keep_non_overlapping_matches() {
    for GreedyCase { case, .. } in greedy_fixture().cases {
        let doc = document(&case);
        let mut matcher = TokenMatcher::new();
        for rule in &case.rules {
            register(&mut matcher, rule);
        }
        assert_same_matches(
            &format!("{}: {:?}", case.id, case.text),
            &matcher.find_matches(&doc).unwrap(),
            &case.expected,
        );
    }
}
#[test]
fn reused_greedy_matcher_keeps_documents_independent() {
    // These cases apply one list of registrations to different documents.
    let cases: Vec<Case> = greedy_fixture()
        .cases
        .into_iter()
        .map(|greedy_case| greedy_case.case)
        .filter(|case| case.id.starts_with("greedy-rules-"))
        .collect();
    assert_eq!(cases.len(), 9);
    assert!(cases.iter().all(|case| case.rules == cases[0].rules));
    let mut matcher = TokenMatcher::new();
    for rule in &cases[0].rules {
        register(&mut matcher, rule);
    }
    for case in cases.iter().chain(cases.iter().rev()) {
        assert_same_matches(
            &format!("reused {}", case.id),
            &matcher.find_matches(&document(case)).unwrap(),
            &case.expected,
        );
    }
    std::thread::scope(|scope| {
        let searches: Vec<_> = cases
            .iter()
            .map(|case| scope.spawn(|| matcher.find_matches(&document(case)).unwrap()))
            .collect();
        for (case, search) in cases.iter().zip(searches) {
            assert_same_matches(
                &format!("concurrent {}", case.id),
                &search.join().unwrap(),
                &case.expected,
            );
        }
    });
}
#[test]
fn rejected_registration_leaves_the_greedy_filter_in_place() {
    let run = run_case();
    let empty = TokenPattern { tokens: Vec::new() };
    let mut matcher = TokenMatcher::new();
    matcher
        .add_with("r", run.pattern.clone(), greedy_options(Greedy::Longest))
        .unwrap();
    assert!(matches!(
        matcher.add("r", vec![empty.clone()]),
        Err(spars::Error::Pattern(_))
    ));
    assert!(matches!(
        matcher.add_with("r", vec![empty], greedy_options(Greedy::First)),
        Err(spars::Error::Pattern(_))
    ));
    assert_eq!(spans(&matcher, &run.doc), run.longest);
}
#[test]
fn registration_after_removal_decides_the_greedy_filter() {
    let run = run_case();
    let mut matcher = TokenMatcher::new();
    matcher
        .add_with("r", run.pattern.clone(), greedy_options(Greedy::Longest))
        .unwrap();
    matcher.remove("r").unwrap();
    matcher.add("r", run.pattern.clone()).unwrap();
    assert_eq!(spans(&matcher, &run.doc), run.every);
    matcher.remove("r").unwrap();
    matcher
        .add_with("r", run.pattern.clone(), greedy_options(Greedy::First))
        .unwrap();
    assert_eq!(spans(&matcher, &run.doc), run.first);
}
#[test]
fn removing_a_rule_leaves_the_greedy_filters_of_other_rules() {
    let run = run_case();
    let mut matcher = TokenMatcher::new();
    for name in ["run_longest", "plain_a", "run_first"] {
        register(&mut matcher, run.rule(name));
    }
    matcher.remove("run_longest").unwrap();
    let mut remaining = TokenMatcher::new();
    for name in ["plain_a", "run_first"] {
        register(&mut remaining, run.rule(name));
    }
    assert_eq!(
        matcher.find_matches(&run.doc).unwrap(),
        remaining.find_matches(&run.doc).unwrap()
    );
    let kept_by_run_first: Vec<_> = matcher
        .find_matches(&run.doc)
        .unwrap()
        .into_iter()
        .filter(|found| found.rule == "run_first")
        .map(|found| (found.start.0, found.end.0))
        .collect();
    assert_eq!(kept_by_run_first, run.first);
}
#[test]
fn registration_with_default_options_returns_every_match() {
    let run = run_case();
    let mut matcher = TokenMatcher::new();
    matcher
        .add_with("r", run.pattern.clone(), greedy_options(Greedy::Longest))
        .unwrap();
    matcher
        .add_with("r", Vec::new(), TokenRuleOptions::default())
        .unwrap();
    assert_eq!(spans(&matcher, &run.doc), run.every);
}
#[test]
fn registration_without_patterns_registers_a_new_rule() {
    let run = run_case();
    let mut matcher = TokenMatcher::new();
    matcher
        .add_with("new", Vec::new(), greedy_options(Greedy::First))
        .unwrap();
    assert!(matcher.contains("new"));
    assert_eq!(matcher.len(), 1);
    assert_eq!(matcher.get("new"), Some(&[][..]));
    assert!(spans(&matcher, &run.doc).is_empty());
}
/// Make the registration as spaCy's `Matcher.add` call was made: without `greedy`, or with it.
fn register(matcher: &mut TokenMatcher, rule: &Rule) {
    let (name, patterns) = (rule.name.clone(), rule.patterns.clone());
    match rule.greedy {
        None => matcher.add(name, patterns),
        Some(greedy) => matcher.add_with(name, patterns, greedy_options(greedy.into())),
    }
    .unwrap();
}
fn greedy_options(greedy: Greedy) -> TokenRuleOptions {
    let mut options = TokenRuleOptions::default();
    options.greedy = Some(greedy);
    options
}
/// The fixture case `greedy-rules-runs`: the document `a a b a a a`, its registrations, and
/// spaCy's spans for the pattern `a+` without `greedy`, with `FIRST` and with `LONGEST`, taken
/// from the recorded results of the rules `run_longest` and `run_first`.
struct RunCase {
    doc: Doc,
    rules: Vec<Rule>,
    pattern: Vec<TokenPattern>,
    every: Vec<(usize, usize)>,
    first: Vec<(usize, usize)>,
    longest: Vec<(usize, usize)>,
}
impl RunCase {
    /// The first registration of `name` in the case.
    fn rule(&self, name: &str) -> &Rule {
        self.rules.iter().find(|rule| rule.name == name).unwrap()
    }
}
fn run_case() -> RunCase {
    let GreedyCase { case, unfiltered } = greedy_fixture()
        .cases
        .into_iter()
        .find(|greedy_case| greedy_case.case.id == "greedy-rules-runs")
        .unwrap();
    assert_eq!(case.text, "a a b a a a");
    let spans_of = |matches: &[TokenMatch], rule: &str| -> Vec<(usize, usize)> {
        matches
            .iter()
            .filter(|found| found.rule == rule)
            .map(|found| (found.start.0, found.end.0))
            .collect()
    };
    let run_longest = case
        .rules
        .iter()
        .find(|rule| rule.name == "run_longest")
        .unwrap();
    let run_first = case
        .rules
        .iter()
        .find(|rule| rule.name == "run_first")
        .unwrap();
    assert!(run_longest.greedy == Some(RecordedGreedy::Longest));
    assert!(run_first.greedy == Some(RecordedGreedy::First));
    assert!(run_first.patterns == run_longest.patterns);
    let run = RunCase {
        doc: document(&case),
        pattern: run_longest.patterns.clone(),
        every: spans_of(&unfiltered, "run_longest"),
        first: spans_of(&case.expected, "run_first"),
        longest: spans_of(&case.expected, "run_longest"),
        rules: case.rules,
    };
    // The three results differ, so a test cannot pass by getting the wrong one.
    assert!(run.first != run.longest && run.first != run.every && run.longest != run.every);
    run
}
fn spans(matcher: &TokenMatcher, doc: &Doc) -> Vec<(usize, usize)> {
    matcher
        .find_matches(doc)
        .unwrap()
        .into_iter()
        .map(|found| (found.start.0, found.end.0))
        .collect()
}
fn greedy_fixture() -> Fixture<GreedyCase> {
    let fixture: Fixture<GreedyCase> = serde_json::from_slice(
        &std::fs::read("../../fixtures/token-match-greedy-v1.expected.json").unwrap(),
    )
    .unwrap();
    assert_eq!(fixture.versions["spacy"], "3.8.14");
    assert_eq!(fixture.versions["thinc"], "8.3.13");
    let total = |count: fn(&GreedyCase) -> usize| fixture.cases.iter().map(count).sum::<usize>();
    let registered = |greedy: fn(&Rule) -> bool| {
        fixture
            .cases
            .iter()
            .flat_map(|greedy_case| &greedy_case.case.rules)
            .filter(|rule| greedy(rule))
            .count()
    };
    assert_eq!(fixture.cases.len(), 40);
    assert_eq!(total(|greedy_case| greedy_case.case.tokens.len()), 145);
    assert_eq!(total(|greedy_case| greedy_case.case.rules.len()), 4242);
    assert_eq!(total(|greedy_case| greedy_case.case.expected.len()), 4357);
    assert_eq!(total(|greedy_case| greedy_case.unfiltered.len()), 8361);
    assert_eq!(
        registered(|rule| matches!(rule.greedy, Some(RecordedGreedy::First))),
        2085
    );
    assert_eq!(
        registered(|rule| matches!(rule.greedy, Some(RecordedGreedy::Longest))),
        2094
    );
    fixture
}
/// Fail with the first position where the two lists differ, since a case can hold hundreds of
/// matches.
fn assert_same_matches(case: &str, actual: &[TokenMatch], expected: &[TokenMatch]) {
    if actual == expected {
        return;
    }
    let position = actual
        .iter()
        .zip(expected)
        .take_while(|(found, wanted)| found == wanted)
        .count();
    panic!(
        "{case}: spaCy returns {} matches, the matcher {}; at position {position} spaCy has {:?}, the matcher {:?}",
        expected.len(),
        actual.len(),
        expected.get(position),
        actual.get(position)
    );
}
#[test]
fn noncanonical_document_morphology_is_rejected_by_find_matches() {
    let doc = |morphology: &str| {
        Doc::from_json(
            &serde_json::json!({"format_version": 1, "document": {"text": "a", "tokens": [{
                "start": 0, "end": 1, "idx": 0, "whitespace": false, "norm": "a",
                "morphology": morphology,
            }]}})
            .to_string(),
        )
        .unwrap()
    };
    let pattern: TokenPattern = serde_json::from_str(
        r#"{"tokens":[{"constraints":[{"attribute":"morphology","predicate":{"kind":"intersects","values":["Case=Nom"]}}],"repetition":{"kind":"once"}}]}"#,
    )
    .unwrap();
    let mut matcher = TokenMatcher::new();
    matcher.add("rule", vec![pattern]).unwrap();
    for noncanonical in [
        "Case=Nom|Case2=Acc",
        "Case=Acc|Case=Nom",
        "poſ=NOUN",
        "POS=noun",
    ] {
        assert!(
            matches!(
                matcher.find_matches(&doc(noncanonical)),
                Err(spars::Error::Unsupported(_))
            ),
            "{noncanonical}"
        );
    }
    assert_eq!(
        matcher
            .find_matches(&doc("Case2=Acc|Case=Nom"))
            .unwrap()
            .len(),
        1
    );
}
