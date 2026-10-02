//! Language lexical resources shared by a loaded model and the matchers built from it.
//! The flag functions below are the single implementation used by both
//! [`crate::Model::lexeme`] and matcher lexical-flag conditions.
use crate::config::{CharFlag, Lexical};
use crate::Result;
use fancy_regex::Regex;
use std::{fmt, sync::Arc};

/// A shared handle to a loaded model's language lexical resources.
///
/// Cloning is cheap and the handle stays valid after the model is dropped. Matchers
/// created with a lexicon evaluate lexical flags such as `LIKE_NUM` with the same
/// language rules as [`crate::Model::lexeme`], as spaCy matchers use their vocabulary.
#[derive(Clone)]
pub struct Lexicon {
    inner: Arc<Inner>,
}

struct Inner {
    resources: Arc<Lexical>,
    email: Regex,
    // Shared with the tokenizer, which compiles it once.
    url: Arc<Regex>,
}

impl fmt::Debug for Lexicon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Lexicon").finish_non_exhaustive()
    }
}

impl Lexicon {
    pub(crate) fn new(resources: Arc<Lexical>, url: Arc<Regex>) -> Result<Self> {
        // Python `re.match` only tries position 0; anchoring avoids retrying later starts.
        let email = Regex::new(&format!(r"\A(?:{})", resources.email_regex))?;
        // Lookups binary-search these lists; the exporter writes them sorted, and Rust's
        // byte order for UTF-8 equals Python's code-point order.
        for (name, list) in [("stops", &resources.stops), ("tlds", &resources.tlds)] {
            if !list.is_sorted() {
                return Err(crate::Error::Model(format!(
                    "lexical {name} are not sorted"
                )));
            }
        }
        Ok(Self {
            inner: Arc::new(Inner {
                resources,
                email,
                url,
            }),
        })
    }
    pub(crate) fn resources(&self) -> &Lexical {
        &self.inner.resources
    }
    /// spaCy `like_email`: the model's email pattern matches at the start of the text.
    pub(crate) fn like_email(&self, s: &str) -> Result<bool> {
        Ok(self.inner.email.is_match(s)?)
    }
    /// spaCy `like_url`, with the model's top-level domains and URL pattern.
    // TODO(multilingual): spaCy's `like_url` uses the language-independent URL_MATCH,
    // not the tokenizer's pattern. They are equal for the supported models (checked by
    // tools/test_dependency_match_reference.py); export URL_MATCH as its own resource
    // before supporting pipelines whose tokenizer overrides or omits `url_match`.
    // Tracked in docs/COMPATIBILITY.md.
    // TODO(regex): Python's `$` also matches before a final newline, so spaCy reports
    // `like_url` for "example.co\n"; the translated pattern does not. Tokenizer output
    // never ends in a newline, so only direct `Model::lexeme` calls differ. Fixing it
    // changes pattern translation in tools/lexical.py and needs a re-export; tracked in
    // docs/COMPATIBILITY.md.
    pub(crate) fn like_url(&self, s: &str) -> Result<bool> {
        let r = self.resources();
        if s.starts_with("http://")
            || s.starts_with("https://")
            || (s.starts_with("www.") && s.len() >= 5)
        {
            return Ok(true);
        }
        if s.starts_with('.') || s.ends_with('.') || s.contains('@') {
            return Ok(false);
        }
        let Some((_, tld)) = s.rsplit_once('.') else {
            return Ok(false);
        };
        let tld = tld.split(':').next().unwrap_or(tld);
        Ok(tld.ends_with('/')
            || (tld.chars().all(|c| r.char_flag(c, CharFlag::Alpha))
                && r.tlds.binary_search_by(|v| v.as_str().cmp(tld)).is_ok())
            || self.inner.url.is_match(s)?)
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
    fn cased(&self, c: char) -> bool {
        self.char_flag(c, CharFlag::Lower)
            || self.char_flag(c, CharFlag::Upper)
            || self.char_flag(c, CharFlag::Title)
    }
    /// Python `str.islower()`: at least one cased character, and all of them lowercase.
    pub(crate) fn is_lower(&self, s: &str) -> bool {
        let mut cased = s.chars().filter(|c| self.cased(*c)).peekable();
        cased.peek().is_some() && cased.all(|c| self.char_flag(c, CharFlag::Lower))
    }
    /// Python `str.isupper()`: at least one cased character, and all of them uppercase.
    pub(crate) fn is_upper(&self, s: &str) -> bool {
        let mut cased = s.chars().filter(|c| self.cased(*c)).peekable();
        cased.peek().is_some() && cased.all(|c| self.char_flag(c, CharFlag::Upper))
    }
    /// Python `str.istitle()`: uppercase or titlecase characters start each cased run.
    pub(crate) fn is_title(&self, s: &str) -> bool {
        let mut previous_cased = false;
        let mut any_cased = false;
        for c in s.chars() {
            if self.char_flag(c, CharFlag::Upper) || self.char_flag(c, CharFlag::Title) {
                if previous_cased {
                    return false;
                }
                previous_cased = true;
                any_cased = true;
            } else if self.char_flag(c, CharFlag::Lower) {
                if !previous_cased {
                    return false;
                }
                previous_cased = true;
                any_cased = true;
            } else {
                previous_cased = false;
            }
        }
        any_cased
    }
    /// spaCy `is_ascii`.
    pub(crate) fn is_ascii(&self, s: &str) -> bool {
        !s.is_empty() && s.is_ascii()
    }
    /// spaCy `is_currency`: every character is in the currency-symbol category.
    pub(crate) fn is_currency(&self, s: &str) -> bool {
        self.all(s, CharFlag::Currency)
    }
    /// spaCy `is_stop`: the lowercase text is one of the language's stop words.
    /// `stops` must be sorted; `Lexicon::new` rejects models where it is not.
    pub(crate) fn is_stop(&self, s: &str) -> bool {
        let find = |lower: &str| {
            self.stops
                .binary_search_by(|v| v.as_str().cmp(lower))
                .is_ok()
        };
        if s.is_ascii() {
            // ASCII lowercasing matches Python `str.lower()` and needs no table lookups.
            if s.bytes().any(|b| b.is_ascii_uppercase()) {
                find(&s.to_ascii_lowercase())
            } else {
                find(s)
            }
        } else {
            find(&self.lower(s))
        }
    }
    pub(crate) fn is_bracket(&self, s: &str) -> bool {
        self.is_bracket.iter().any(|v| v == s)
    }
    pub(crate) fn is_quote(&self, s: &str) -> bool {
        self.is_quote.iter().any(|v| v == s)
    }
    pub(crate) fn is_left_punct(&self, s: &str) -> bool {
        self.is_left_punct.iter().any(|v| v == s)
    }
    pub(crate) fn is_right_punct(&self, s: &str) -> bool {
        self.is_right_punct.iter().any(|v| v == s)
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
