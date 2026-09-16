use crate::{Error, Result};
use serde_json::Value;
use std::collections::HashMap;
fn bad(message: &str) -> Error {
    Error::Model(message.into())
}
fn strings(v: &Value) -> Result<Vec<String>> {
    Ok(serde_json::from_value(v.clone())?)
}
pub(crate) fn resources(c: &Value) -> Result<()> {
    if c["tokenizer"]["regex_dialect"] != "fancy-regex-explicit-python-unicode-v1" {
        return Err(Error::Unsupported("regex dialect".into()));
    }
    for (p, v) in [("spacy", "3.8.14"), ("thinc", "8.3.13")] {
        if c["versions"][p] != v {
            return Err(Error::Unsupported(format!("reference version {p}")));
        }
    }
    let _: HashMap<String, u64> = serde_json::from_value(c["symbols"].clone())?;
    let _: HashMap<u64, String> = serde_json::from_value(c["norms"].clone())?;
    let rules = c["tokenizer"]["rules"]
        .as_object()
        .ok_or_else(|| bad("tokenizer rules missing"))?;
    for (word, rule) in rules {
        if word.is_empty() {
            return Err(bad("empty exception key"));
        }
        let rule = rule
            .as_array()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| bad("empty exception"))?;
        let mut offset = 0;
        for (i, t) in rule.iter().enumerate() {
            let orth = t["ORTH"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| bad("invalid exception ORTH"))?;
            if !word.get(offset..).is_some_and(|s| s.starts_with(orth)) {
                return Err(bad("exception does not reconstruct source"));
            }
            offset += orth.len();
            if i + 1 < rule.len()
                && word.as_bytes().get(offset) == Some(&b' ')
                && !rule[i + 1]["ORTH"].as_str().unwrap_or("").starts_with(' ')
            {
                offset += 1
            }
            if !t["NORM"].is_null() && t["NORM"].as_str().is_none() {
                return Err(bad("invalid exception NORM"));
            }
            if t.as_object()
                .is_none_or(|o| o.keys().any(|k| k != "ORTH" && k != "NORM"))
            {
                return Err(Error::Unsupported("exception attribute".into()));
            }
        }
        if offset != word.len() {
            return Err(bad("exception length mismatch"));
        }
    }
    let lex = &c["lexical"];
    let _: HashMap<String, String> = serde_json::from_value(lex["lower"].clone())?;
    for flag in [
        "alpha",
        "digit",
        "lower",
        "upper",
        "title",
        "punct",
        "currency",
        "case_ignorable",
    ] {
        let ranges: Vec<[u32; 2]> = serde_json::from_value(lex["ranges"][flag].clone())?;
        let mut prev = None;
        for [a, b] in ranges {
            if a > b || b > 0x10ffff || prev.is_some_and(|p| a <= p) {
                return Err(bad("invalid Unicode ranges"));
            }
            prev = Some(b)
        }
    }
    for key in [
        "stops",
        "number_words",
        "tlds",
        "is_bracket",
        "is_quote",
        "is_left_punct",
        "is_right_punct",
    ] {
        strings(&lex[key])?;
    }
    let rules = c["attribute_rules"]
        .as_array()
        .ok_or_else(|| bad("missing attribute rules"))?;
    for r in rules {
        let index = r["index"].as_i64().ok_or_else(|| bad("attribute index"))?;
        for pat in r["patterns"]
            .as_array()
            .ok_or_else(|| bad("attribute patterns"))?
        {
            let pat = pat
                .as_array()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| bad("empty attribute pattern"))?;
            if index >= pat.len() as i64 || index < -(pat.len() as i64) {
                return Err(bad("attribute index out of range"));
            }
            for token in pat {
                for (attr, val) in token
                    .as_object()
                    .ok_or_else(|| bad("attribute token pattern"))?
                {
                    if !["LOWER", "TAG", "DEP", "IS_SPACE"].contains(&attr.as_str()) {
                        return Err(Error::Unsupported(format!("attribute {attr}")));
                    }
                    if let Some(ops) = val.as_object() {
                        for (op, v) in ops {
                            match op.as_str() {
                                "IN" | "NOT_IN" => {
                                    strings(v)?;
                                }
                                "REGEX" => {
                                    fancy_regex::Regex::new(
                                        v.as_str().ok_or_else(|| bad("pattern regex"))?,
                                    )?;
                                }
                                _ => {
                                    return Err(Error::Unsupported(format!(
                                        "pattern operator {op}"
                                    )))
                                }
                            }
                        }
                    } else if (attr == "IS_SPACE" && val.as_bool().is_none())
                        || (attr != "IS_SPACE" && val.as_str().is_none())
                    {
                        return Err(bad("attribute value type"));
                    }
                }
            }
        }
        for (key, value) in r["attrs"]
            .as_object()
            .ok_or_else(|| bad("rule output missing"))?
        {
            if !["POS", "TAG", "MORPH", "LEMMA", "DEP"].contains(&key.as_str())
                || value.as_str().is_none()
            {
                return Err(Error::Unsupported("attribute output".into()));
            }
        }
    }
    for pos in ["noun", "verb", "adj", "adv", "punct"] {
        let ix = &c["lemmas"]["lemma_index"][pos];
        if ix.as_object().is_some_and(|o| o.is_empty()) {
        } else {
            strings(ix)?;
        }
        let _: HashMap<String, Vec<String>> =
            serde_json::from_value(c["lemmas"]["lemma_exc"][pos].clone())?;
        let r = &c["lemmas"]["lemma_rules"][pos];
        if r.as_object().is_some_and(|o| o.is_empty()) {
        } else {
            let _: Vec<[String; 2]> = serde_json::from_value(r.clone())?;
        }
    }
    for name in ["parser", "ner"] {
        for action in strings(&c[name]["actions"])? {
            let (kind, label) = action.split_once('-').unwrap_or((&action, ""));
            let valid = if name == "parser" {
                (kind == "S" || kind == "D") && label.is_empty()
                    || ["L", "R", "B"].contains(&kind) && !label.is_empty()
            } else {
                ["B", "I", "L", "U"].contains(&kind) || kind == "O" && label.is_empty()
            };
            if !valid {
                return Err(Error::Unsupported(format!("{name} action {action}")));
            }
        }
    }
    Ok(())
}
