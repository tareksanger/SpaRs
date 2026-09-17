use crate::{config::CharFlag, Model};
use serde::Serialize;
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Lexeme {
    pub orth: u64,
    pub norm: String,
    pub shape: String,
    pub prefix: String,
    pub suffix: String,
    pub is_alpha: bool,
    pub is_digit: bool,
    pub is_lower: bool,
    pub is_upper: bool,
    pub is_title: bool,
    pub is_space: bool,
    pub is_ascii: bool,
    pub is_punct: bool,
    pub is_currency: bool,
    pub is_stop: bool,
    pub is_bracket: bool,
    pub is_quote: bool,
    pub is_left_punct: bool,
    pub is_right_punct: bool,
    pub like_num: bool,
    pub like_email: bool,
    pub like_url: bool,
    pub has_vector: bool,
}
impl Model {
    pub(crate) fn char_flag(&self, c: char, flag: CharFlag) -> bool {
        let ranges = self.config.lexical.ranges.get(flag);
        let cp = c as u32;
        let idx = ranges.partition_point(|v| v[1] < cp);
        ranges.get(idx).is_some_and(|r| r[0] <= cp)
    }
    pub(crate) fn lower(&self, s: &str) -> String {
        let chars: Vec<char> = s.chars().collect();
        let mut out = String::new();
        let cased = |c| {
            self.char_flag(c, CharFlag::Lower)
                || self.char_flag(c, CharFlag::Upper)
                || self.char_flag(c, CharFlag::Title)
        };
        for (i, &c) in chars.iter().enumerate() {
            if c == 'Σ'
                && chars[..i]
                    .iter()
                    .rev()
                    .find(|c| !self.char_flag(**c, CharFlag::CaseIgnorable))
                    .is_some_and(|c| cased(*c))
                && !chars[i + 1..]
                    .iter()
                    .find(|c| !self.char_flag(**c, CharFlag::CaseIgnorable))
                    .is_some_and(|c| cased(*c))
            {
                out.push('ς');
            } else if let Some(lower) = self.config.lexical.lower.get(&c.to_string()) {
                out.push_str(lower)
            } else {
                out.push(c)
            }
        }
        out
    }
    pub(crate) fn shape(&self, s: &str) -> String {
        if s.chars().count() >= 100 {
            return "LONG".into();
        }
        let mut out = String::new();
        let mut last = '\0';
        let mut count = 0;
        for c in s.chars() {
            let x = if self.char_flag(c, CharFlag::Alpha) {
                if self.char_flag(c, CharFlag::Upper) {
                    'X'
                } else {
                    'x'
                }
            } else if self.char_flag(c, CharFlag::Digit) {
                'd'
            } else {
                c
            };
            if x == last {
                count += 1
            } else {
                count = 0;
                last = x
            }
            if count < 4 {
                out.push(x)
            }
        }
        out
    }
    /// Lexical properties use the Unicode classifications exported by the pinned reference.
    pub fn lexeme(&self, s: &str) -> crate::Result<Lexeme> {
        let r = &self.config.lexical;
        let member = |values: &[String], text: &str| values.iter().any(|v| v == text);
        let all = |flag: CharFlag| !s.is_empty() && s.chars().all(|c| self.char_flag(c, flag));
        let cased: Vec<char> = s
            .chars()
            .filter(|c| {
                self.char_flag(*c, CharFlag::Lower)
                    || self.char_flag(*c, CharFlag::Upper)
                    || self.char_flag(*c, CharFlag::Title)
            })
            .collect();
        let is_lower =
            !cased.is_empty() && cased.iter().all(|c| self.char_flag(*c, CharFlag::Lower));
        let is_upper =
            !cased.is_empty() && cased.iter().all(|c| self.char_flag(*c, CharFlag::Upper));
        let mut prev = false;
        let mut is_title = !cased.is_empty();
        for c in s.chars() {
            if self.char_flag(c, CharFlag::Upper) || self.char_flag(c, CharFlag::Title) {
                if prev {
                    is_title = false
                }
                prev = true
            } else if self.char_flag(c, CharFlag::Lower) {
                if !prev {
                    is_title = false
                }
                prev = true
            } else {
                prev = false
            }
        }
        let stripped = s
            .strip_prefix(['+', '-', '±', '~'])
            .unwrap_or(s)
            .replace([',', '.'], "");
        let lower = self.lower(&stripped);
        let digit =
            |v: &str| !v.is_empty() && v.chars().all(|c| self.char_flag(c, CharFlag::Digit));
        let like_num = digit(&stripped)
            || stripped
                .split_once('/')
                .is_some_and(|(a, b)| digit(a) && digit(b))
            || member(&r.number_words, &lower)
            || ["st", "nd", "rd", "th"]
                .iter()
                .any(|end| lower.strip_suffix(end).is_some_and(digit));
        let tld = s
            .rsplit_once('.')
            .map(|(_, v)| v.split(':').next().unwrap());
        let like_url = if s.starts_with("http://")
            || s.starts_with("https://")
            || (s.starts_with("www.") && s.len() >= 5)
        {
            true
        } else if s.starts_with('.') || s.ends_with('.') || s.contains('@') {
            false
        } else if let Some(tld) = tld {
            tld.ends_with('/')
                || (tld.chars().all(|c| self.char_flag(c, CharFlag::Alpha)) && member(&r.tlds, tld))
                || self.tokenizer.url_match(s)?
        } else {
            false
        };
        Ok(Lexeme {
            orth: self.string_id(s),
            norm: self.norm(s),
            shape: self.shape(s),
            prefix: s.chars().take(1).collect(),
            suffix: s
                .chars()
                .skip(s.chars().count().saturating_sub(3))
                .collect(),
            is_alpha: all(CharFlag::Alpha),
            is_digit: all(CharFlag::Digit),
            is_lower,
            is_upper,
            is_title,
            is_space: !s.is_empty() && s.chars().all(crate::tokenizer::is_space),
            is_ascii: !s.is_empty() && s.is_ascii(),
            is_punct: !s.is_empty() && s.chars().all(|c| self.char_flag(c, CharFlag::Punct)),
            is_currency: !s.is_empty() && s.chars().all(|c| self.char_flag(c, CharFlag::Currency)),
            is_stop: member(&r.stops, &self.lower(s)),
            is_bracket: member(&r.is_bracket, s),
            is_quote: member(&r.is_quote, s),
            is_left_punct: member(&r.is_left_punct, s),
            is_right_punct: member(&r.is_right_punct, s),
            like_num,
            like_email: self.email_regex.find(s)?.is_some_and(|m| m.start() == 0),
            like_url,
            has_vector: self.vector(s).is_some(),
        })
    }
}
