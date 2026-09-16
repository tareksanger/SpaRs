use crate::{Doc, Model, Result, Token};
use fancy_regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
pub(crate) struct Tokenizer {
    prefix: Regex,
    suffix: Regex,
    infix: Regex,
    url: Regex,
    rules: HashMap<String, Vec<Value>>,
    max_exception_bytes: usize,
}
impl Tokenizer {
    pub(crate) fn url_match(&self, s: &str) -> Result<bool> {
        Ok(self.url.is_match(s)?)
    }
    pub fn new(v: &Value) -> Result<Self> {
        if !v["token_match"].is_null() {
            return Err(crate::Error::Unsupported("token_match".into()));
        }
        let re = |k: &str| -> Result<Regex> {
            Ok(Regex::new(v[k].as_str().ok_or_else(|| {
                crate::Error::Model(format!("missing tokenizer {k}"))
            })?)?)
        };
        Ok(Self {
            prefix: re("prefix")?,
            suffix: re("suffix")?,
            infix: re("infix")?,
            url: re("url")?,
            rules: serde_json::from_value(v["rules"].clone())?,
            max_exception_bytes: v["rules"]
                .as_object()
                .unwrap()
                .keys()
                .map(String::len)
                .max()
                .unwrap_or(0),
        })
    }
    fn split(
        &self,
        s: &str,
        offset: usize,
        out: &mut Vec<(usize, usize, Option<String>)>,
    ) -> Result<()> {
        let mut start = 0;
        let mut end = s.len();
        let mut suffixes = vec![];
        while start < end && !self.rules.contains_key(&s[start..end]) {
            let text = &s[start..end];
            let p = self.prefix.find(text)?.map_or(0, |m| m.end() - m.start());
            if p > 0 && self.rules.contains_key(&text[p..]) {
                out.push((offset + start, offset + start + p, None));
                start += p;
                break;
            }
            let z = self
                .suffix
                .find(&text[p..])?
                .map_or(0, |m| m.end() - m.start());
            if z > 0 && self.rules.contains_key(&text[..text.len() - z]) {
                suffixes.push((offset + end - z, offset + end, None));
                end -= z;
                break;
            }
            if p == 0 && z == 0 {
                break;
            }
            if p > 0 {
                out.push((offset + start, offset + start + p, None));
                start += p
            }
            if z > 0 {
                suffixes.push((offset + end - z, offset + end, None));
                end -= z
            }
        }
        if start < end {
            let text = &s[start..end];
            if let Some(rule) = self.rules.get(text) {
                let mut at = offset + start;
                for t in rule {
                    let len = t["ORTH"].as_str().unwrap().len();
                    out.push((at, at + len, t["NORM"].as_str().map(str::to_owned)));
                    at += len;
                }
            } else if self.url.is_match(text)? {
                out.push((offset + start, offset + end, None))
            } else {
                let mut at = 0;
                for m in self.infix.find_iter(text) {
                    let m = m?;
                    if m.start() == 0 {
                        continue;
                    }
                    if m.start() != at {
                        out.push((offset + start + at, offset + start + m.start(), None))
                    }
                    if m.end() != m.start() {
                        out.push((offset + start + m.start(), offset + start + m.end(), None))
                    }
                    at = m.end();
                }
                if at < text.len() {
                    out.push((offset + start + at, offset + end, None))
                }
            }
        }
        out.extend(suffixes.into_iter().rev());
        Ok(())
    }
}
pub(crate) fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}
impl Model {
    pub fn tokenize(&self, text: &str) -> Result<Doc> {
        let mut raw = vec![];
        let mut start = 0;
        let mut in_ws = text.chars().next().is_some_and(is_space);
        for (i, c) in text.char_indices() {
            if is_space(c) != in_ws {
                if start < i {
                    self.tokenizer.split(&text[start..i], start, &mut raw)?
                }
                start = if c == ' ' { i + 1 } else { i };
                in_ws = !in_ws
            }
        }
        if start < text.len() {
            self.tokenizer.split(&text[start..], start, &mut raw)?
        }
        // Apply multi-token exceptions after affix/infix segmentation, longest first.
        let mut matches = vec![];
        for (i, (a, _, _)) in raw.iter().enumerate() {
            for (j, (_, b, _)) in raw.iter().enumerate().skip(i) {
                if *b - *a > self.tokenizer.max_exception_bytes {
                    break;
                }
                if let Some(rule) = self.tokenizer.rules.get(&text[*a..*b]) {
                    matches.push((j + 1 - i, i, j + 1, rule));
                }
            }
        }
        matches.sort_by_key(|(len, i, _, _)| (std::cmp::Reverse(*len), *i));
        let mut used = vec![false; raw.len()];
        let mut selected = HashMap::new();
        for (_, i, j, r) in matches {
            if !used[i..j].iter().any(|x| *x) {
                used[i..j].fill(true);
                selected.insert(i, (j, r));
            }
        }
        let mut replaced = vec![];
        let mut i = 0;
        while i < raw.len() {
            if let Some((j, r)) = selected.get(&i) {
                let mut a = raw[i].0;
                for t in *r {
                    let word = t["ORTH"].as_str().unwrap();
                    replaced.push((a, a + word.len(), t["NORM"].as_str().map(str::to_owned)));
                    a += word.len();
                    if text.as_bytes().get(a) == Some(&b' ') && a < raw[*j - 1].1 {
                        a += 1
                    }
                }
                i = *j;
            } else {
                replaced.push(raw[i].clone());
                i += 1
            }
        }
        let mut tokens = vec![];
        let mut cp = 0;
        let mut prev = 0;
        for (a, b, norm) in replaced {
            cp += text[prev..a].chars().count();
            let mut t = Token::new(a, b, cp, norm.unwrap_or_else(|| self.norm(&text[a..b])));
            t.whitespace = text.as_bytes().get(b) == Some(&b' ');
            tokens.push(t);
            prev = a;
        }
        Ok(Doc {
            text: text.into(),
            tokens,
            entities: None,
            sentences: None,
            noun_chunks: None,
        })
    }
}
#[cfg(test)]
impl Tokenizer {
    pub(crate) fn regex_spans(&self, text: &str, kind: &str) -> Result<Vec<[usize; 2]>> {
        let re = match kind {
            "prefix" => &self.prefix,
            "suffix" => &self.suffix,
            "infix" => &self.infix,
            "url" => &self.url,
            _ => unreachable!(),
        };
        let convert = |m: fancy_regex::Match<'_>| {
            [
                text[..m.start()].chars().count(),
                text[..m.end()].chars().count(),
            ]
        };
        if kind == "infix" {
            Ok(re
                .find_iter(text)
                .map(|m| m.map(convert).map_err(crate::Error::from))
                .collect::<std::result::Result<_, _>>()?)
        } else {
            Ok(re.find(text)?.map(convert).into_iter().collect())
        }
    }
}
