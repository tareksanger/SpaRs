use crate::chunks::chunks;
use crate::config::Component;
use crate::{neural::linear_rows, Doc, Error, Model, Result};
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
pub(crate) fn validate_order(pipeline: &[Component]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for component in pipeline {
        let required: &[Component] = match component {
            Component::Tok2vec | Component::Ner => &[],
            Component::Tagger | Component::Parser => &[Component::Tok2vec],
            Component::AttributeRuler => &[Component::Tagger, Component::Parser],
            Component::Lemmatizer => &[Component::AttributeRuler],
        };
        if !required.iter().all(|dependency| seen.contains(dependency)) {
            return Err(Error::Unsupported(format!(
                "missing preceding dependency for {component:?}"
            )));
        }
        if !seen.insert(*component) {
            return Err(Error::Unsupported(format!(
                "duplicate pipeline component {component:?}"
            )));
        }
    }
    Ok(())
}

impl Model {
    pub fn process(&self, text: &str) -> Result<Doc> {
        self.execute(text, None)
    }
    /// Run the declared pipeline through the requested component, inclusive.
    pub fn process_until(&self, text: &str, stage: Stage) -> Result<Doc> {
        if stage == Stage::Tokenizer {
            return self.tokenize(text);
        }
        let component = match stage {
            Stage::Tokenizer => unreachable!(),
            Stage::Tagger => Component::Tagger,
            Stage::Parser => Component::Parser,
            Stage::AttributeRuler => Component::AttributeRuler,
            Stage::Lemmatizer => Component::Lemmatizer,
            Stage::Ner => Component::Ner,
        };
        if !self.config.pipeline.contains(&component) {
            return Err(Error::Unsupported(format!(
                "pipeline does not contain {component:?}"
            )));
        }
        self.execute(text, Some(component))
    }
    fn execute(&self, text: &str, until: Option<Component>) -> Result<Doc> {
        let mut doc = self.tokenize(text)?;
        let mut shared = Vec::new();
        for component in &self.config.pipeline {
            match component {
                Component::Tok2vec => shared = self.tok2vec(&doc),
                Component::Tagger => {
                    for (token, scores) in doc.tokens.iter_mut().zip(linear_rows(
                        self,
                        &self.config.tagger.params,
                        &shared,
                    )) {
                        let i = best(&scores, |_| true)?;
                        token.tag = Some(self.config.tagger.labels[i].clone());
                    }
                }
                Component::Parser => self.parse(&mut doc, &shared)?,
                Component::AttributeRuler => {
                    self.attributes(&mut doc)?;
                    chunks(&mut doc);
                }
                Component::Lemmatizer => self.lemmatize(&mut doc),
                Component::Ner => self.ner(&mut doc)?,
            }
            if Some(*component) == until {
                break;
            }
        }
        if self.tensors["vectors"].shape[1] == 0 {
            doc.tensor = shared;
        }
        Ok(doc)
    }
    pub fn pipe<'a>(
        &'a self,
        texts: impl IntoIterator<Item = &'a str> + 'a,
    ) -> impl Iterator<Item = Result<Doc>> + 'a {
        texts.into_iter().map(|s| self.process(s))
    }
}
