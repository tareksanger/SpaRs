use crate::{Doc, Span, TokenIndex};
pub(crate) fn left_edge(d: &Doc, i: usize) -> usize {
    let mut min = i;
    let mut stack = vec![i];
    let mut seen = vec![false; d.tokens.len()];
    while let Some(h) = stack.pop() {
        if seen[h] {
            continue;
        }
        seen[h] = true;
        min = min.min(h);
        for (j, t) in d.tokens.iter().enumerate() {
            if j != h && t.head == Some(TokenIndex(h)) {
                stack.push(j)
            }
        }
    }
    min
}
pub(crate) fn chunks(d: &mut Doc) {
    let labels = [
        "oprd",
        "nsubj",
        "dobj",
        "nsubjpass",
        "pcomp",
        "pobj",
        "dative",
        "appos",
        "attr",
        "ROOT",
    ];
    let mut result = vec![];
    let mut end = 0;
    for i in 0..d.tokens.len() {
        let t = &d.tokens[i];
        if !["NOUN", "PROPN", "PRON"].contains(&t.pos.as_deref().unwrap_or("")) {
            continue;
        }
        let left = left_edge(d, i);
        if left < end {
            continue;
        }
        let dep = t.dep.as_deref().unwrap_or("");
        let mut include = labels.contains(&dep);
        if dep == "conj" {
            let mut h = t.head.unwrap().0;
            for _ in 0..d.tokens.len() {
                let t = &d.tokens[h];
                if t.dep.as_deref() != Some("conj") || t.head.unwrap().0 >= h {
                    break;
                }
                h = t.head.unwrap().0;
            }
            include = labels.contains(&d.tokens[h].dep.as_deref().unwrap_or(""))
        }
        if include {
            end = i + 1;
            result.push(Span {
                start: TokenIndex(left),
                end: TokenIndex(end),
                label: "NP".into(),
            })
        }
    }
    d.noun_chunks = Some(result);
}
