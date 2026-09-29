//! Language lexical resources shared by a loaded model and the matchers built from it.
//! The flag functions below are the single implementation used by both
//! [`crate::Model::lexeme`] and matcher lexical-flag conditions.
use crate::config::{CharFlag, Lexical};
use std::{fmt, sync::Arc};

/// A shared handle to a loaded model's language lexical resources.
///
/// Cloning is cheap and the handle stays valid after the model is dropped. Matchers
/// created with a lexicon evaluate lexical flags such as `LIKE_NUM` with the same
/// language rules as [`crate::Model::lexeme`], as spaCy matchers use their vocabulary.
#[derive(Clone)]
pub struct Lexicon {
    resources: Arc<Lexical>,
}

impl fmt::Debug for Lexicon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Lexicon").finish_non_exhaustive()
    }
}

impl Lexicon {
    pub(crate) fn new(resources: Arc<Lexical>) -> Self {
        Self { resources }
    }
    pub(crate) fn resources(&self) -> &Lexical {
        &self.resources
    }
}

impl Lexical {
    pub(crate) fn char_flag(&self, c: char, flag: CharFlag) -> bool {
        let ranges = self.ranges.get(flag);
        let cp = c as u32;
        let idx = ranges.partition_point(|v| v[1] < cp);
        ranges.get(idx).is_some_and(|r| r[0] <= cp)
    }
    fn all(&self, s: &str, flag: CharFlag) -> bool {
        !s.is_empty() && s.chars().all(|c| self.char_flag(c, flag))
    }
    /// Python `str.lower()` from the model's pinned Unicode resources.
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
            } else if let Some(lower) = self.lower.get(&c.to_string()) {
                out.push_str(lower)
            } else {
                out.push(c)
            }
        }
        out
    }
    /// Python `str.isalpha()`.
    pub(crate) fn is_alpha(&self, s: &str) -> bool {
        self.all(s, CharFlag::Alpha)
    }
    /// Python `str.isdigit()`.
    pub(crate) fn is_digit(&self, s: &str) -> bool {
        self.all(s, CharFlag::Digit)
    }
    /// spaCy `is_punct`: every character is in a Unicode punctuation category.
    pub(crate) fn is_punct(&self, s: &str) -> bool {
        self.all(s, CharFlag::Punct)
    }
    /// Python `str.isspace()`.
    pub(crate) fn is_space(&self, s: &str) -> bool {
        !s.is_empty() && s.chars().all(crate::tokenizer::is_space)
    }
    /// spaCy's English `like_num`: the base rule plus the model's number and ordinal
    /// words and digit ordinal suffixes. Other languages define different rules, so
    /// loading is limited to the English language capability until this rule becomes
    /// a typed per-language resource.
    // TODO(multilingual): replace this English rule with a typed per-language `like_num`
    // capability before non-English models can load. Tracked in docs/PROGRESS.md
    // (model extensibility plan) and docs/COMPATIBILITY.md.
    pub(crate) fn like_num(&self, s: &str) -> bool {
        let stripped = s
            .strip_prefix(['+', '-', '±', '~'])
            .unwrap_or(s)
            .replace([',', '.'], "");
        let lower = self.lower(&stripped);
        self.is_digit(&stripped)
            || stripped
                .split_once('/')
                .is_some_and(|(a, b)| self.is_digit(a) && self.is_digit(b))
            || self.number_words.contains(&lower)
            || ["st", "nd", "rd", "th"]
                .iter()
                .any(|end| lower.strip_suffix(end).is_some_and(|v| self.is_digit(v)))
    }
}
