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
        self.config.lexical.char_flag(c, flag)
    }
    pub(crate) fn lower(&self, s: &str) -> String {
        self.config.lexical.lower(s)
    }
    /// A shared handle to this model's language lexical resources, for matchers.
    pub fn lexicon(&self) -> crate::Lexicon {
        self.lexicon.clone()
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
        let r = &*self.config.lexical;
        Ok(Lexeme {
            orth: self.string_id(s),
            norm: self.norm(s),
            shape: self.shape(s),
            prefix: s.chars().take(1).collect(),
            suffix: s
                .chars()
                .skip(s.chars().count().saturating_sub(3))
                .collect(),
            is_alpha: r.is_alpha(s),
            is_digit: r.is_digit(s),
            is_lower: r.is_lower(s),
            is_upper: r.is_upper(s),
            is_title: r.is_title(s),
            is_space: r.is_space(s),
            is_ascii: r.is_ascii(s),
            is_punct: r.is_punct(s),
            is_currency: r.is_currency(s),
            is_stop: r.is_stop(s),
            is_bracket: r.is_bracket(s),
            is_quote: r.is_quote(s),
            is_left_punct: r.is_left_punct(s),
            is_right_punct: r.is_right_punct(s),
            like_num: r.like_num(s),
            like_email: self.lexicon.like_email(s)?,
            like_url: self.lexicon.like_url(s)?,
            has_vector: self.vector(s).is_some(),
        })
    }
}
