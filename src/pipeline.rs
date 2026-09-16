use crate::chunks::chunks;
use crate::{neural::linear, Doc, Error, Model, Result};
pub(crate) fn best(scores: &[f32], valid: impl Fn(usize) -> bool) -> Result<usize> {
    if scores.iter().any(|s| !s.is_finite()) {
        return Err(Error::Model("nonfinite inference score".into()));
    }
    let mut choice = None;
    for (i, s) in scores.iter().enumerate() {
        if valid(i) && choice.is_none_or(|j| *s > scores[j]) {
            choice = Some(i)
        }
    }
    choice.ok_or_else(|| Error::Model("no valid transition".into()))
}
/// Ordered execution prefixes preserve required component dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Tokenizer,
    Tagger,
    Parser,
    AttributeRuler,
    Lemmatizer,
    Ner,
}
impl Model {
    pub fn process(&self, text: &str) -> Result<Doc> {
        self.process_until(text, Stage::Ner)
    }
    /// Run an ordered prefix of the official pipeline. Later annotations remain unavailable.
    pub fn process_until(&self, text: &str, stage: Stage) -> Result<Doc> {
        let mut doc = self.tokenize(text)?;
        if stage == Stage::Tokenizer {
            return Ok(doc);
        }
        let x = self.tok2vec(&doc);
        for (t, row) in doc.tokens.iter_mut().zip(&x) {
            let scores = linear(self, &self.config["tagger"]["params"], row);
            let i = best(&scores, |_| true)?;
            t.tag = Some(self.config["tagger"]["labels"][i].as_str().unwrap().into());
        }
        if stage == Stage::Tagger {
            return Ok(doc);
        }
        self.parse(&mut doc, &x)?;
        if stage == Stage::Parser {
            return Ok(doc);
        }
        self.attributes(&mut doc)?;
        chunks(&mut doc);
        if stage == Stage::AttributeRuler {
            return Ok(doc);
        }
        self.lemmatize(&mut doc);
        if stage == Stage::Lemmatizer {
            return Ok(doc);
        }
        self.ner(&mut doc)?;
        Ok(doc)
    }
    pub fn pipe<'a>(
        &'a self,
        texts: impl IntoIterator<Item = &'a str> + 'a,
    ) -> impl Iterator<Item = Result<Doc>> + 'a {
        texts.into_iter().map(|s| self.process(s))
    }
}
