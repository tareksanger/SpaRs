use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
mod traversal;
pub use traversal::{Ancestors, Children, DependencyError, Subtree};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteOffset(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodePointOffset(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenIndex(pub usize);
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub start: ByteOffset,
    pub end: ByteOffset,
    pub idx: CodePointOffset,
    pub whitespace: bool,
    pub norm: String,
    pub tag: Option<String>,
    pub pos: Option<String>,
    pub morphology: Option<String>,
    pub lemma: Option<String>,
    pub head: Option<TokenIndex>,
    pub dep: Option<String>,
    pub sentence_start: Option<bool>,
    pub entity_iob: Option<String>,
    pub entity_type: Option<String>,
}
impl Token {
    pub(crate) fn new(start: usize, end: usize, idx: usize, norm: String) -> Self {
        Self {
            start: ByteOffset(start),
            end: ByteOffset(end),
            idx: CodePointOffset(idx),
            whitespace: false,
            norm,
            tag: None,
            pos: None,
            morphology: None,
            lemma: None,
            head: None,
            dep: None,
            sentence_start: None,
            entity_iob: None,
            entity_type: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub start: TokenIndex,
    pub end: TokenIndex,
    pub label: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Doc {
    pub(crate) text: String,
    pub(crate) tokens: Vec<Token>,
    pub(crate) entities: Option<Vec<Span>>,
    pub(crate) sentences: Option<Vec<Span>>,
    pub(crate) noun_chunks: Option<Vec<Span>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) tensor: Vec<Vec<f32>>,
    // Public traversal begins after pipeline writes finish. Any future mutation
    // of dependency heads must invalidate this cache before exposing the document.
    #[serde(skip)]
    pub(crate) dependency_index:
        OnceLock<std::result::Result<traversal::DependencyIndex, DependencyError>>,
}
impl PartialEq for Doc {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
            && self.tokens == other.tokens
            && self.entities == other.entities
            && self.sentences == other.sentences
            && self.noun_chunks == other.noun_chunks
            && self.tensor == other.tensor
    }
}
impl Doc {
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }
    pub fn token_text(&self, i: TokenIndex) -> Result<&str> {
        let t = self.tokens.get(i.0).ok_or(Error::Bounds)?;
        Ok(&self.text[t.start.0..t.end.0])
    }
    pub fn span_text(&self, start: TokenIndex, end: TokenIndex) -> Result<&str> {
        if start.0 > end.0 || end.0 > self.tokens.len() {
            return Err(Error::Bounds);
        }
        if start == end {
            return Ok("");
        }
        Ok(&self.text[self.tokens[start.0].start.0..self.tokens[end.0 - 1].end.0])
    }
    pub fn entities(&self) -> Option<&[Span]> {
        self.entities.as_deref()
    }
    pub fn sentences(&self) -> Option<&[Span]> {
        self.sentences.as_deref()
    }
    pub fn noun_chunks(&self) -> Option<&[Span]> {
        self.noun_chunks.as_deref()
    }
    pub fn codepoint_offset(&self, byte: ByteOffset) -> Result<CodePointOffset> {
        self.text
            .get(..byte.0)
            .map(|s| CodePointOffset(s.chars().count()))
            .ok_or(Error::Bounds)
    }
    pub fn byte_offset(&self, point: CodePointOffset) -> Result<ByteOffset> {
        self.text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(self.text.len()))
            .nth(point.0)
            .map(ByteOffset)
            .ok_or(Error::Bounds)
    }
}
impl Doc {
    /// Serialize an immutable document snapshot, preserving unavailable annotations.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(
            &serde_json::json!({"format_version":if self.tensor.is_empty() { 1 } else { 2 },"document":self}),
        )?)
    }
    /// Restore a snapshot after validating offsets, token indices and spans.
    pub fn from_json(json: &str) -> Result<Self> {
        #[derive(Deserialize)]
        struct Snapshot {
            format_version: u32,
            document: Storage,
        }
        #[derive(Deserialize)]
        struct Storage {
            text: String,
            tokens: Vec<Token>,
            entities: Option<Vec<Span>>,
            sentences: Option<Vec<Span>>,
            noun_chunks: Option<Vec<Span>>,
            #[serde(default)]
            tensor: Vec<Vec<f32>>,
        }
        let s: Snapshot = serde_json::from_str(json)?;
        if ![1, 2].contains(&s.format_version) {
            return Err(Error::Unsupported("document format version".into()));
        }
        if s.format_version == 1 && !s.document.tensor.is_empty() {
            return Err(Error::Unsupported(
                "contextual vectors require document format v2".into(),
            ));
        }
        let s = s.document;
        if !s.tensor.is_empty() {
            let width = s.tensor[0].len();
            if s.tensor.len() != s.tokens.len()
                || width == 0
                || width > 4096
                || s.tensor
                    .iter()
                    .any(|row| row.len() != width || row.iter().any(|x| !x.is_finite()))
            {
                return Err(Error::Model("invalid document tensor".into()));
            }
        }
        let mut end = 0;
        let mut cp = 0;
        for (i, t) in s.tokens.iter().enumerate() {
            if t.start.0 != end
                || t.end.0 <= t.start.0
                || !s.text.is_char_boundary(t.start.0)
                || !s.text.is_char_boundary(t.end.0)
                || t.idx.0 != cp
                || t.head.is_some_and(|h| h.0 >= s.tokens.len())
            {
                return Err(Error::Bounds);
            }
            cp += s.text[t.start.0..t.end.0].chars().count();
            end = t.end.0;
            if t.whitespace {
                if s.text.as_bytes().get(end) != Some(&b' ') {
                    return Err(Error::Bounds);
                }
                end += 1;
                cp += 1
            }
            if let Some(next) = s.tokens.get(i + 1) {
                if next.start.0 != end {
                    return Err(Error::Bounds);
                }
            }
        }
        if end != s.text.len() {
            return Err(Error::Bounds);
        }
        for spans in [&s.entities, &s.sentences, &s.noun_chunks]
            .into_iter()
            .flatten()
        {
            let mut end = 0;
            for span in spans {
                if span.start.0 < end || span.start.0 >= span.end.0 || span.end.0 > s.tokens.len() {
                    return Err(Error::Bounds);
                }
                end = span.end.0;
            }
        }
        Ok(Self {
            text: s.text,
            tokens: s.tokens,
            entities: s.entities,
            sentences: s.sentences,
            noun_chunks: s.noun_chunks,
            tensor: s.tensor,
            dependency_index: OnceLock::new(),
        })
    }
}
/// Borrowed, checked token access without self-referential document storage.
#[derive(Clone, Copy)]
pub struct TokenView<'a> {
    doc: &'a Doc,
    index: TokenIndex,
}
impl<'a> TokenView<'a> {
    pub fn text(self) -> &'a str {
        self.doc.token_text(self.index).expect("checked token")
    }
    pub fn index(self) -> TokenIndex {
        self.index
    }
    pub fn annotations(self) -> &'a Token {
        &self.doc.tokens[self.index.0]
    }
    pub fn head(self) -> Option<Self> {
        self.annotations().head.map(|index| Self {
            doc: self.doc,
            index,
        })
    }
    pub fn span(self) -> SpanView<'a> {
        SpanView {
            doc: self.doc,
            start: self.index,
            end: TokenIndex(self.index.0 + 1),
        }
    }
}
/// Borrowed half-open token span; an empty span has empty text and a zero vector.
#[derive(Clone, Copy)]
pub struct SpanView<'a> {
    pub(crate) doc: &'a Doc,
    pub(crate) start: TokenIndex,
    pub(crate) end: TokenIndex,
}
impl<'a> SpanView<'a> {
    pub fn text(self) -> &'a str {
        self.doc
            .span_text(self.start, self.end)
            .expect("checked span")
    }
    pub fn start(self) -> TokenIndex {
        self.start
    }
    pub fn end(self) -> TokenIndex {
        self.end
    }
    pub fn tokens(self) -> impl Iterator<Item = TokenView<'a>> {
        (self.start.0..self.end.0).map(move |i| TokenView {
            doc: self.doc,
            index: TokenIndex(i),
        })
    }
}
impl Doc {
    pub fn token(&self, index: TokenIndex) -> Result<TokenView<'_>> {
        self.token_text(index)?;
        Ok(TokenView { doc: self, index })
    }
    pub fn span(&self, start: TokenIndex, end: TokenIndex) -> Result<SpanView<'_>> {
        self.span_text(start, end)?;
        Ok(SpanView {
            doc: self,
            start,
            end,
        })
    }
}
