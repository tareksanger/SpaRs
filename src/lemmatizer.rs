use crate::config::CharFlag;
use crate::{Doc, Model, TokenIndex};
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
            let lemmas = &self.config.lemmas;
            let index = lemmas.lemma_index.get(&pos);
            let exc = lemmas.lemma_exc.get(&pos);
            let rules = lemmas.lemma_rules.get(&pos);
            let lemma = if base {
                lower
            } else if rules.is_none_or(|v| v.is_empty())
                && index.is_none_or(|v| v.is_empty())
                && exc.is_none_or(|v| v.is_empty())
            {
                if pos == "propn" {
                    word.into()
                } else {
                    lower
                }
            } else {
                let mut forms = vec![];
                let mut oov = vec![];
                if let Some(rules) = rules {
                    for rule in rules {
                        let old = rule[0].as_str();
                        let new = rule[1].as_str();
                        if let Some(root) = lower.strip_suffix(old) {
                            let form = format!("{root}{new}");
                            if !form.is_empty() {
                                let known = index.is_some_and(|v| v.contains(&form));
                                if known {
                                    forms.insert(0, form)
                                } else if !form.chars().all(|c| self.char_flag(c, CharFlag::Alpha))
                                {
                                    forms.push(form)
                                } else {
                                    oov.push(form)
                                }
                            }
                        }
                    }
                }
                if let Some(exceptions) = exc.and_then(|v| v.get(&lower)) {
                    for e in exceptions {
                        let form = e.clone();
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
