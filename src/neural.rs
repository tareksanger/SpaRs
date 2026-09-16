use crate::{Doc, Error, Model, Result, TokenIndex};
use serde_json::Value;
pub(crate) type Matrix = Vec<Vec<f32>>;
pub(crate) fn linear(model: &Model, p: &Value, x: &[f32]) -> Vec<f32> {
    let w = model.tensor(&p["W"]);
    let b = model.tensor(&p["b"]);
    w.data
        .chunks_exact(x.len())
        .zip(&b.data)
        .map(|(r, b)| r.iter().zip(x).map(|(a, b)| a * b).sum::<f32>() + b)
        .collect()
}
fn block(model: &Model, p: &Value, x: &[f32]) -> Vec<f32> {
    let w = model.tensor(&p["maxout"]["W"]);
    let b = model.tensor(&p["maxout"]["b"]);
    let pieces = w.shape[1];
    let a: Vec<f32> = w
        .data
        .chunks_exact(x.len())
        .zip(&b.data)
        .map(|(r, b)| r.iter().zip(x).map(|(a, b)| a * b).sum::<f32>() + b)
        .collect();
    let mut y: Vec<f32> = a
        .chunks_exact(pieces)
        .map(|p| p.iter().copied().fold(f32::NEG_INFINITY, f32::max))
        .collect();
    let mean = y.iter().sum::<f32>() / y.len() as f32;
    let var = y.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / y.len() as f32 + 1e-8;
    let inv = 1. / var.sqrt();
    let g = model.tensor(&p["norm"]["G"]);
    let b = model.tensor(&p["norm"]["b"]);
    for (i, v) in y.iter_mut().enumerate() {
        *v = (*v - mean) * inv * g.data[i] + b.data[i]
    }
    y
}
impl Model {
    /// Contextual features from the shared tok2vec network (one row per token).
    pub fn tok2vec(&self, doc: &Doc) -> Vec<Vec<f32>> {
        self.encode(doc, &self.config["tok2vec"])
    }
    pub(crate) fn features(&self, doc: &Doc, i: usize, attrs: &[Value]) -> Vec<u64> {
        let t = &doc.tokens[i];
        let word = doc.token_text(TokenIndex(i)).unwrap();
        let chars: Vec<_> = word.chars().collect();
        attrs
            .iter()
            .map(|a| match a.as_str().unwrap() {
                "NORM" => self.string_id(&t.norm),
                "PREFIX" => self.string_id(&chars[..1].iter().collect::<String>()),
                "SUFFIX" => self.string_id(
                    &chars[chars.len().saturating_sub(3)..]
                        .iter()
                        .collect::<String>(),
                ),
                "SHAPE" => self.string_id(&self.shape(word)),
                "SPACY" => u64::from(t.whitespace),
                "IS_SPACE" => u64::from(word.chars().all(crate::tokenizer::is_space)),
                _ => unreachable!("validated features"),
            })
            .collect()
    }
    pub(crate) fn encode(&self, doc: &Doc, p: &Value) -> Matrix {
        self.encode_traced(doc, p, None)
    }
    pub(crate) fn encode_traced(
        &self,
        doc: &Doc,
        p: &Value,
        mut trace: Option<&mut Vec<Matrix>>,
    ) -> Matrix {
        let width = p["width"].as_u64().unwrap() as usize;
        let n = doc.tokens.len();
        if n == 0 {
            return vec![];
        }
        let attrs = p["attrs"].as_array().unwrap();
        let hashes = p["hashes"].as_array().unwrap();
        let mut mixed = Vec::with_capacity(n);
        for i in 0..n {
            let ids = self.features(doc, i, attrs);
            let mut concat = vec![];
            for (id, h) in ids.iter().zip(hashes) {
                let e = self.tensor(&h["params"]["E"]);
                let rows = e.shape[0];
                let keys = crate::hash::keys(*id, h["seed"].as_u64().unwrap());
                let mut v = vec![0.; width];
                for key in keys {
                    let at = (key as usize % rows) * width;
                    for (a, b) in v.iter_mut().zip(&e.data[at..at + width]) {
                        *a += b
                    }
                }
                concat.extend(v);
            }
            let w = self.tensor(&p["static"]["W"]);
            let v = self.vector(doc.token_text(TokenIndex(i)).unwrap());
            for row in w.data.chunks_exact(w.shape[1]) {
                concat.push(v.map_or(0., |v| row.iter().zip(v).map(|(a, b)| a * b).sum()))
            }
            mixed.push(block(self, &p["mix"], &concat));
        }
        // Thinc with_array pads before the encoder; padding also evolves through residual layers.
        if let Some(t) = trace.as_mut() {
            t.push(mixed.clone());
        }
        let pad = p["pad"].as_u64().unwrap() as usize;
        let mut x = vec![vec![0.; width]; n + pad * 2];
        x[pad..pad + n].clone_from_slice(&mixed);
        for (layer, window) in p["layers"]
            .as_array()
            .unwrap()
            .iter()
            .zip(p["windows"].as_array().unwrap())
        {
            let window = window.as_u64().unwrap() as isize;
            let mut y = x.clone();
            for (i, output) in y.iter_mut().enumerate() {
                let mut input = Vec::with_capacity(width * (2 * window as usize + 1));
                for j in i as isize - window..=i as isize + window {
                    if j < 0 || j >= x.len() as isize {
                        input.resize(input.len() + width, 0.)
                    } else {
                        input.extend_from_slice(&x[j as usize])
                    }
                }
                let delta = block(self, layer, &input);
                for (v, d) in output.iter_mut().zip(delta) {
                    *v += d
                }
            }
            x = y;
            if let Some(t) = trace.as_mut() {
                t.push(x[pad..pad + n].to_vec());
            }
        }
        x[pad..pad + n].to_vec()
    }
}
pub(crate) struct Scorer<'a> {
    model: &'a Model,
    p: &'a Value,
    cache: Matrix,
    nf: usize,
    no: usize,
    np: usize,
}
impl<'a> Scorer<'a> {
    pub fn new(model: &'a Model, p: &'a Value, x: &Matrix) -> Self {
        let w = model.tensor(&p["lower"]["W"]);
        let nf = w.shape[0];
        let no = w.shape[1];
        let np = w.shape[2];
        let mut cache = vec![model.tensor(&p["lower"]["pad"]).data.clone()];
        for row in x {
            let r = linear(model, &p["reduce"], row);
            cache.push(
                w.data
                    .chunks_exact(r.len())
                    .map(|w| w.iter().zip(&r).map(|(a, b)| a * b).sum())
                    .collect(),
            )
        }
        Self {
            model,
            p,
            cache,
            nf,
            no,
            np,
        }
    }
    pub fn scores(&self, ids: &[Option<usize>]) -> Vec<f32> {
        let mut a = vec![0.; self.no * self.np];
        for (f, id) in ids.iter().enumerate().take(self.nf) {
            let row = &self.cache[id.map_or(0, |i| i + 1)];
            for (j, v) in a.iter_mut().enumerate() {
                *v += row[f * self.no * self.np + j]
            }
        }
        for (v, b) in a
            .iter_mut()
            .zip(&self.model.tensor(&self.p["lower"]["b"]).data)
        {
            *v += b
        }
        let hidden: Vec<f32> = a
            .chunks_exact(self.np)
            .map(|p| p.iter().copied().fold(f32::NEG_INFINITY, f32::max))
            .collect();
        linear(self.model, &self.p["upper"], &hidden)
    }
}
pub(crate) fn validate(m: &Model) -> Result<()> {
    let bad = |s: &str| Error::Model(s.into());
    let tensor = |key: &Value| -> Result<&crate::model::Tensor> {
        m.tensors
            .get(
                key.as_str()
                    .ok_or_else(|| bad("invalid tensor reference"))?,
            )
            .ok_or_else(|| bad("missing tensor"))
    };
    let shape = |key: &Value, expected: &[usize]| -> Result<()> {
        if tensor(key)?.shape != expected {
            Err(bad(&format!("shape mismatch {key}, expected {expected:?}")))
        } else {
            Ok(())
        }
    };
    let vectors = m
        .tensors
        .get("vectors")
        .ok_or_else(|| bad("missing vectors"))?;
    if vectors.shape.len() != 2 {
        return Err(bad("vectors rank"));
    }
    if m.vector_keys.values().any(|&r| r >= vectors.shape[0]) {
        return Err(bad("vector row out of bounds"));
    }
    let enc = |p: &Value| -> Result<usize> {
        let width = p["width"]
            .as_u64()
            .filter(|x| *x > 0 && *x <= 4096)
            .ok_or_else(|| bad("invalid width"))? as usize;
        let attrs = p["attrs"].as_array().ok_or_else(|| bad("attrs missing"))?;
        let hashes = p["hashes"]
            .as_array()
            .ok_or_else(|| bad("hashes missing"))?;
        if attrs.len() != hashes.len() || attrs.is_empty() {
            return Err(bad("feature count mismatch"));
        }
        for (a, h) in attrs.iter().zip(hashes) {
            if !["NORM", "PREFIX", "SUFFIX", "SHAPE", "SPACY", "IS_SPACE"]
                .contains(&a.as_str().unwrap_or(""))
            {
                return Err(Error::Unsupported(format!("feature {a}")));
            }
            let e = tensor(&h["params"]["E"])?;
            if e.shape.len() != 2 || e.shape[1] != width || h["seed"].as_u64().is_none() {
                return Err(bad("embedding config"));
            }
        }
        shape(&p["static"]["W"], &[width, vectors.shape[1]])?;
        let block = |b: &Value, input: usize| -> Result<()> {
            let w = tensor(&b["maxout"]["W"])?;
            if w.shape.len() != 3 || w.shape[0] != width || w.shape[2] != input {
                return Err(bad("maxout shape"));
            }
            shape(&b["maxout"]["b"], &[width, w.shape[1]])?;
            shape(&b["norm"]["G"], &[width])?;
            shape(&b["norm"]["b"], &[width])
        };
        block(&p["mix"], width * (attrs.len() + 1))?;
        let layers = p["layers"]
            .as_array()
            .ok_or_else(|| bad("layers missing"))?;
        let windows = p["windows"]
            .as_array()
            .ok_or_else(|| bad("windows missing"))?;
        if layers.len() != windows.len() {
            return Err(bad("window count"));
        }
        let mut total = 0;
        for (b, w) in layers.iter().zip(windows) {
            let w = w
                .as_u64()
                .filter(|x| *x <= 16)
                .ok_or_else(|| bad("invalid window"))? as usize;
            total += w;
            block(b, width * (w * 2 + 1))?
        }
        if p["pad"].as_u64() != Some(total as u64) {
            return Err(bad("encoder padding"));
        }
        Ok(width)
    };
    let width = enc(&m.config["tok2vec"])?;
    let nerwidth = enc(&m.config["ner"]["tok2vec"])?;
    let tag = &m.config["tagger"];
    let labels = tag["labels"]
        .as_array()
        .ok_or_else(|| bad("tagger labels"))?;
    if labels.is_empty() || labels.iter().any(|v| v.as_str().is_none()) {
        return Err(bad("invalid labels"));
    }
    shape(&tag["params"]["W"], &[labels.len(), width])?;
    shape(&tag["params"]["b"], &[labels.len()])?;
    for (name, nf, input) in [("parser", 8, width), ("ner", 3, nerwidth)] {
        let p = &m.config[name];
        let low = tensor(&p["lower"]["W"])?;
        if low.shape.len() != 4 || low.shape[0] != nf {
            return Err(bad("transition features"));
        }
        let no = low.shape[1];
        let np = low.shape[2];
        if np < 2 {
            return Err(Error::Unsupported(
                "transition activation without maxout".into(),
            ));
        }
        let ni = low.shape[3];
        shape(&p["lower"]["b"], &[no, np])?;
        shape(&p["lower"]["pad"], &[1, nf, no, np])?;
        shape(&p["reduce"]["W"], &[ni, input])?;
        shape(&p["reduce"]["b"], &[ni])?;
        let actions = p["actions"]
            .as_array()
            .ok_or_else(|| bad("missing actions"))?;
        if actions.is_empty() || actions.iter().any(|v| v.as_str().is_none()) {
            return Err(bad("invalid actions"));
        }
        shape(&p["upper"]["W"], &[actions.len(), no])?;
        shape(&p["upper"]["b"], &[actions.len()])?;
    }
    let expected = serde_json::json!([
        "tok2vec",
        "tagger",
        "parser",
        "attribute_ruler",
        "lemmatizer",
        "ner"
    ]);
    if m.config["pipeline"] != expected {
        return Err(Error::Unsupported("pipeline order".into()));
    }
    Ok(())
}
