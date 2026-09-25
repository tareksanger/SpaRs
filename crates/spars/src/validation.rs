use crate::{config::*, Error, Result};
fn bad(message: &str) -> Error {
    Error::Model(message.into())
}
pub(crate) fn resources(c: &Manifest) -> Result<()> {
    if c.tokenizer.regex_dialect != "fancy-regex-explicit-python-unicode-v1" {
        return Err(Error::Unsupported("regex dialect".into()));
    }
    for (p, v) in [("spacy", "3.8.14"), ("thinc", "8.3.13")] {
        if c.versions.get(p).map(String::as_str) != Some(v) {
            return Err(Error::Unsupported(format!("reference version {p}")));
        }
    }
    for (word, rule) in &c.tokenizer.rules {
        if word.is_empty() || rule.is_empty() {
            return Err(bad("empty exception"));
        }
        let mut offset = 0;
        for (i, t) in rule.iter().enumerate() {
            if t.orth.is_empty() || !word.get(offset..).is_some_and(|s| s.starts_with(&t.orth)) {
                return Err(bad("exception does not reconstruct source"));
            }
            offset += t.orth.len();
            if i + 1 < rule.len()
                && word.as_bytes().get(offset) == Some(&b' ')
                && !rule[i + 1].orth.starts_with(' ')
            {
                offset += 1;
            }
        }
        if offset != word.len() {
            return Err(bad("exception length mismatch"));
        }
    }
    for flag in [
        CharFlag::Alpha,
        CharFlag::Digit,
        CharFlag::Lower,
        CharFlag::Upper,
        CharFlag::Title,
        CharFlag::Punct,
        CharFlag::Currency,
        CharFlag::CaseIgnorable,
    ] {
        let mut prev = None;
        for &[a, b] in c.lexical.ranges.get(flag) {
            if a > b || b > 0x10ffff || prev.is_some_and(|p| a <= p) {
                return Err(bad("invalid Unicode ranges"));
            }
            prev = Some(b);
        }
    }
    for r in &c.attribute_rules {
        for pat in &r.patterns {
            if pat.is_empty() || r.index >= pat.len() as i64 || r.index < -(pat.len() as i64) {
                return Err(bad("attribute index out of range"));
            }
        }
    }
    for pos in ["noun", "verb", "adj", "adv", "punct"] {
        if !c.lemmas.lemma_index.contains_key(pos)
            || !c.lemmas.lemma_exc.contains_key(pos)
            || !c.lemmas.lemma_rules.contains_key(pos)
        {
            return Err(bad("missing lemma POS table"));
        }
    }
    for (name, actions) in [
        ("parser", &c.parser.actions),
        ("ner", &c.ner.transition.actions),
    ] {
        for action in actions {
            let valid = if name == "parser" {
                matches!(action.kind, ActionKind::Shift | ActionKind::Reduce)
                    && action.label.is_empty()
                    || matches!(
                        action.kind,
                        ActionKind::Left | ActionKind::Right | ActionKind::Begin
                    ) && !action.label.is_empty()
            } else {
                matches!(
                    action.kind,
                    ActionKind::Begin | ActionKind::Inside | ActionKind::Left | ActionKind::Unit
                ) || action.kind == ActionKind::Outside && action.label.is_empty()
            };
            if !valid {
                return Err(Error::Unsupported(format!("{name} action {action:?}")));
            }
        }
    }
    Ok(())
}
