use crate::{
    config::{Constraint, OutputAttribute},
    Doc, Model, Result, TokenIndex,
};
fn matches(constraint: &Option<Constraint>, actual: &str) -> Result<bool> {
    Ok(match constraint {
        None => true,
        Some(Constraint::Exact(expected)) => actual == expected,
        Some(Constraint::Operators(ops)) => {
            let included = ops
                .included
                .as_ref()
                .is_none_or(|v| v.iter().any(|s| s == actual));
            let excluded = ops
                .not_in
                .as_ref()
                .is_none_or(|v| v.iter().all(|s| s != actual));
            let regex = match &ops.regex {
                None => true,
                Some(re) => fancy_regex::Regex::new(re)?.is_match(actual)?,
            };
            included && excluded && regex
        }
    })
}
impl Model {
    pub(crate) fn attributes(&self, d: &mut Doc) -> Result<()> {
        // Collect matches before applying any output attributes.
        let mut found = vec![];
        for (r, rule) in self.config.attribute_rules.iter().enumerate() {
            for pat in &rule.patterns {
                for start in 0..=d.tokens.len().saturating_sub(pat.len()) {
                    if start + pat.len() > d.tokens.len() {
                        continue;
                    }
                    let mut yes = true;
                    for (off, p) in pat.iter().enumerate() {
                        let t = &d.tokens[start + off];
                        let word = d.token_text(TokenIndex(start + off))?;
                        yes &= matches(&p.tag, t.tag.as_deref().unwrap_or(""))?;
                        yes &= matches(&p.dep, t.dep.as_deref().unwrap_or(""))?;
                        yes &= matches(&p.lower, &self.lower(word))?;
                        yes &= p
                            .is_space
                            .is_none_or(|v| v == word.chars().all(crate::tokenizer::is_space));
                    }
                    if yes {
                        let at = if rule.index < 0 {
                            (start + pat.len()) as i64 + rule.index
                        } else {
                            start as i64 + rule.index
                        };
                        found.push((r, at as usize));
                    }
                }
            }
        }
        for (r, i) in found {
            for (key, v) in &self.config.attribute_rules[r].attrs {
                let value = Some(if *key == OutputAttribute::Morph && v == "_" {
                    String::new()
                } else {
                    v.clone()
                });
                let t = &mut d.tokens[i];
                match key {
                    OutputAttribute::Pos => t.pos = value,
                    OutputAttribute::Tag => t.tag = value,
                    OutputAttribute::Dep => t.dep = value,
                    OutputAttribute::Morph => t.morphology = value,
                    OutputAttribute::Lemma => t.lemma = value,
                }
            }
        }
        for t in &mut d.tokens {
            if t.morphology.is_none() {
                t.morphology = Some(String::new());
            }
        }
        Ok(())
    }
}
