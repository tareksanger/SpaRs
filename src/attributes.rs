use crate::{Doc, Error, Model, Result, TokenIndex};
use serde_json::Value;
impl Model {
    pub(crate) fn attributes(&self, d: &mut Doc) -> Result<()> {
        // Matches are collected before attributes change, then applied in rule order.
        let mut matches = vec![];
        for (r, rule) in self.config["attribute_rules"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            for pattern in rule["patterns"].as_array().unwrap() {
                let pat = pattern.as_array().unwrap();
                for start in 0..=d.tokens.len().saturating_sub(pat.len()) {
                    if start + pat.len() > d.tokens.len() {
                        continue;
                    }
                    let mut yes = true;
                    for (off, p) in pat.iter().enumerate() {
                        for (key, value) in p.as_object().unwrap() {
                            let t = &d.tokens[start + off];
                            let actual = match key.as_str() {
                                "TAG" => Value::from(t.tag.as_deref().unwrap_or("")),
                                "DEP" => Value::from(t.dep.as_deref().unwrap_or("")),
                                "LOWER" => {
                                    Value::from(self.lower(d.token_text(TokenIndex(start + off))?))
                                }
                                "IS_SPACE" => Value::from(
                                    d.token_text(TokenIndex(start + off))?
                                        .chars()
                                        .all(crate::tokenizer::is_space),
                                ),
                                _ => {
                                    return Err(Error::Unsupported(format!("rule attribute {key}")))
                                }
                            };
                            let good = if let Some(obj) = value.as_object() {
                                let mut good = true;
                                for (op, v) in obj {
                                    good &= match op.as_str() {
                                        "IN" => v.as_array().unwrap().contains(&actual),
                                        "NOT_IN" => !v.as_array().unwrap().contains(&actual),
                                        "REGEX" => fancy_regex::Regex::new(v.as_str().unwrap())?
                                            .is_match(actual.as_str().unwrap_or(""))?,
                                        _ => {
                                            return Err(Error::Unsupported(format!(
                                                "rule operator {op}"
                                            )))
                                        }
                                    };
                                }
                                good
                            } else {
                                actual == *value
                            };
                            yes &= good;
                        }
                    }
                    if yes {
                        let index = rule["index"].as_i64().unwrap();
                        let at = if index < 0 {
                            (start + pat.len()) as i64 + index
                        } else {
                            start as i64 + index
                        };
                        if at < start as i64 || at >= (start + pat.len()) as i64 {
                            return Err(Error::Model("attribute rule index".into()));
                        }
                        matches.push((r, at as usize));
                    }
                }
            }
        }
        for (r, i) in matches {
            for (key, v) in self.config["attribute_rules"][r]["attrs"]
                .as_object()
                .unwrap()
            {
                let value = Some(if key == "MORPH" && v == "_" {
                    String::new()
                } else {
                    v.as_str().unwrap().into()
                });
                let t = &mut d.tokens[i];
                match key.as_str() {
                    "POS" => t.pos = value,
                    "TAG" => t.tag = value,
                    "DEP" => t.dep = value,
                    "MORPH" => t.morphology = value,
                    "LEMMA" => t.lemma = value,
                    _ => return Err(Error::Unsupported(format!("rule output {key}"))),
                }
            }
        }
        for t in &mut d.tokens {
            if t.morphology.is_none() {
                t.morphology = Some(String::new())
            }
        }
        Ok(())
    }
}
