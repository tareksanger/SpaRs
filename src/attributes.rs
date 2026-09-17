use crate::{
    config::{AttributeRule, Constraint, OutputAttribute},
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
            if !included {
                return Ok(false);
            }
            let excluded = ops
                .not_in
                .as_ref()
                .is_none_or(|v| v.iter().all(|s| s != actual));
            if !excluded {
                return Ok(false);
            }
            match &ops.regex {
                None => true,
                Some(re) => re.is_match(actual)?,
            }
        }
    })
}
impl Model {
    pub(crate) fn attributes(&self, d: &mut Doc) -> Result<()> {
        apply_rules(&self.config.attribute_rules, d, |word| self.lower(word))
    }
}

fn apply_rules(rules: &[AttributeRule], d: &mut Doc, lower: impl Fn(&str) -> String) -> Result<()> {
    // Collect matches before applying any output attributes.
    let mut found = vec![];
    let mut lowercase = vec![None; d.tokens.len()];
    for (r, rule) in rules.iter().enumerate() {
        for pat in &rule.patterns {
            for start in 0..=d.tokens.len().saturating_sub(pat.len()) {
                if start + pat.len() > d.tokens.len() {
                    continue;
                }
                let mut yes = true;
                for (off, p) in pat.iter().enumerate() {
                    let t = &d.tokens[start + off];
                    if !matches(&p.tag, t.tag.as_deref().unwrap_or(""))?
                        || !matches(&p.dep, t.dep.as_deref().unwrap_or(""))?
                    {
                        yes = false;
                        break;
                    }
                    let word = d.token_text(TokenIndex(start + off))?;
                    if p.lower.is_some() {
                        let lower = lowercase[start + off].get_or_insert_with(|| lower(word));
                        if !matches(&p.lower, lower)? {
                            yes = false;
                            break;
                        }
                    }
                    if !p
                        .is_space
                        .is_none_or(|v| v == word.chars().all(crate::tokenizer::is_space))
                    {
                        yes = false;
                        break;
                    }
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
        for (key, v) in &rules[r].attrs {
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

#[cfg(test)]
mod tests;
