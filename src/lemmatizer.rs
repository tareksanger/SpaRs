use crate::{Doc, Model, TokenIndex};
use serde_json::Value;
impl Model {
    pub(crate) fn lemmatize(&self, d: &mut Doc) {
        for i in 0..d.tokens.len() {
            if d.tokens[i].lemma.is_some() {
                continue;
            }
            let word = d.token_text(TokenIndex(i)).unwrap();
            let lower = self.lower(word);
            let t = &d.tokens[i];
            let pos = t.pos.as_deref().unwrap_or("").to_lowercase();
            let morph = t.morphology.as_deref().unwrap_or("");
            let base = (pos == "noun" && morph.contains("Number=Sing"))
                || (pos == "verb"
                    && morph.contains("VerbForm=Fin")
                    && morph.contains("Tense=Pres")
                    && !morph.contains("Number="))
                || morph.contains("VerbForm=Inf")
                || morph.contains("VerbForm=None")
                || morph.contains("Degree=Pos");
            let lemmas = &self.config["lemmas"];
            let index = &lemmas["lemma_index"][&pos];
            let exc = &lemmas["lemma_exc"][&pos];
            let rules = &lemmas["lemma_rules"][&pos];
            let lemma = if base {
                lower
            } else if rules.as_array().is_none_or(|v| v.is_empty())
                && index.as_array().is_none_or(|v| v.is_empty())
                && exc.as_object().is_none_or(|v| v.is_empty())
            {
                if pos == "propn" {
                    word.into()
                } else {
                    lower
                }
            } else {
                let mut forms = vec![];
                let mut oov = vec![];
                if let Some(rules) = rules.as_array() {
                    for rule in rules {
                        let old = rule[0].as_str().unwrap();
                        let new = rule[1].as_str().unwrap();
                        if let Some(root) = lower.strip_suffix(old) {
                            let form = format!("{root}{new}");
                            if !form.is_empty() {
                                let known = index
                                    .as_array()
                                    .is_some_and(|v| v.contains(&Value::from(form.clone())));
                                if known {
                                    forms.insert(0, form)
                                } else if !form.chars().all(|c| self.char_flag(c, "alpha")) {
                                    forms.push(form)
                                } else {
                                    oov.push(form)
                                }
                            }
                        }
                    }
                }
                if let Some(exceptions) = exc[&lower].as_array() {
                    for e in exceptions {
                        let form = e.as_str().unwrap().to_string();
                        if !forms.contains(&form) {
                            forms.insert(0, form)
                        }
                    }
                }
                forms
                    .into_iter()
                    .next()
                    .or_else(|| oov.into_iter().next())
                    .unwrap_or_else(|| word.into())
            };
            d.tokens[i].lemma = Some(lemma);
        }
    }
}
