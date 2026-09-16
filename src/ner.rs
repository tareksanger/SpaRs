use crate::pipeline::best;
use crate::{neural::Scorer, Doc, Model, Result, Span, TokenIndex};
use serde_json::Value;
impl Model {
    pub(crate) fn ner(&self, d: &mut Doc) -> Result<()> {
        self.ner_traced(d, None)
    }
    pub(crate) fn ner_traced(&self, d: &mut Doc, mut trace: Option<&mut Vec<Value>>) -> Result<()> {
        let p = &self.config["ner"];
        let x = self.encode(d, &p["tok2vec"]);
        let scorer = Scorer::new(self, p, &x);
        let actions = p["actions"].as_array().unwrap();
        let mut open: Option<(usize, String)> = None;
        let mut ents = vec![];
        for i in 0..d.tokens.len() {
            let ids = [
                Some(i),
                open.as_ref().map(|(i, _)| *i),
                open.as_ref().map(|_| i - 1),
            ];
            let scores = scorer.scores(&ids);
            let space = d
                .token_text(TokenIndex(i))?
                .chars()
                .all(crate::tokenizer::is_space);
            let next = i + 1 < d.tokens.len();
            let boundary = next && d.tokens[i + 1].sentence_start == Some(true);
            let valid: Vec<bool> = (0..scores.len())
                .map(|j| {
                    let a = actions[j].as_str().unwrap();
                    let label = a.get(2..).unwrap_or("");
                    match a.as_bytes()[0] {
                        b'B' => open.is_none() && next && !boundary && !space && !label.is_empty(),
                        b'I' => open.as_ref().is_some_and(|(_, l)| l == label) && next && !boundary,
                        b'L' => open.as_ref().is_some_and(|(_, l)| l == label),
                        b'U' => open.is_none() && !space && !label.is_empty(),
                        b'O' => open.is_none(),
                        _ => false,
                    }
                })
                .collect();
            let a = best(&scores, |i| valid[i])?;
            if let Some(t) = trace.as_mut() {
                t.push(serde_json::json!({"ids":ids.map(|v|v.map_or(-1,|i|i as i64)),"scores":scores,"valid":valid,"action":a}));
            }
            let a = actions[a].as_str().unwrap();
            let label = a.get(2..).unwrap_or("");
            let kind = a.as_bytes()[0];
            d.tokens[i].entity_iob = Some(
                if kind == b'O' {
                    "O"
                } else if kind == b'B' || kind == b'U' {
                    "B"
                } else {
                    "I"
                }
                .into(),
            );
            d.tokens[i].entity_type = Some(label.into());
            match kind {
                b'B' => open = Some((i, label.into())),
                b'L' => {
                    let (s, l) = open.take().unwrap();
                    ents.push(Span {
                        start: TokenIndex(s),
                        end: TokenIndex(i + 1),
                        label: l,
                    })
                }
                b'U' => ents.push(Span {
                    start: TokenIndex(i),
                    end: TokenIndex(i + 1),
                    label: label.into(),
                }),
                _ => {}
            }
        }
        d.entities = Some(ents);
        Ok(())
    }
}
