use crate::config::{Dtype, Manifest, TensorRef};
use crate::{tokenizer::Tokenizer, Error, Result};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::Path};
#[derive(Debug)]
pub(crate) struct Tensor {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}
pub struct Model {
    pub(crate) config: Manifest,
    pub(crate) tensors: HashMap<String, Tensor>,
    pub(crate) tokenizer: Tokenizer,
    pub(crate) vector_keys: HashMap<u64, usize>,
    pub(crate) norms: HashMap<u64, String>,
    pub(crate) email_regex: fancy_regex::Regex,
    pub(crate) symbols: HashMap<String, u64>,
}
impl Model {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let config: Manifest = serde_json::from_slice(&std::fs::read(path.join("manifest.json"))?)?;
        match config.format_version {
            1 if config.model == "en_core_web_md"
                && config.model_version == "3.8.0"
                && config.capabilities.is_none()
                && config.tok2vec.static_vectors.is_some()
                && config.ner.tok2vec.static_vectors.is_some() => {}
            1 => {
                return Err(Error::Unsupported(
                    "legacy v1 requires en_core_web_md 3.8.0 and its static encoders".into(),
                ))
            }
            2 => {
                config
                    .capabilities
                    .as_ref()
                    .ok_or_else(|| {
                        Error::Unsupported("v2 requires explicit runtime capabilities".into())
                    })?
                    .validate();
                if config.model.trim().is_empty() || config.model_version.trim().is_empty() {
                    return Err(Error::Model("missing model identity".into()));
                }
            }
            _ => return Err(Error::Unsupported("format_version".into())),
        }
        crate::validation::resources(&config)?;
        let bytes = std::fs::read(path.join("weights.safetensors"))?;
        if Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
            != config.weights_sha256
        {
            return Err(Error::Model("weights checksum mismatch".into()));
        }
        let st = safetensors::SafeTensors::deserialize(&bytes)
            .map_err(|e| Error::Model(e.to_string()))?;
        let specs = &config.tensors;
        let mut tensors = HashMap::new();
        for (name, spec) in specs {
            let t = st.tensor(name).map_err(|e| Error::Model(e.to_string()))?;
            let shape = spec.shape.clone();
            if spec.dtype != Dtype::F32
                || t.dtype() != safetensors::Dtype::F32
                || shape != t.shape()
                || (shape.contains(&0)
                    && !(config.format_version == 2 && name == "vectors" && shape == [0, 0]))
            {
                return Err(Error::Model(format!("invalid shape/dtype for {name}")));
            }
            let data: Vec<f32> = t
                .data()
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                .collect();
            if data.iter().any(|v| !v.is_finite()) {
                return Err(Error::Model(format!("nonfinite {name}")));
            }
            tensors.insert(name.clone(), Tensor { shape, data });
        }
        drop(st);
        drop(bytes);
        let tokenizer = Tokenizer::new(&config.tokenizer)?;
        let vector_keys = config.vector_keys.clone();
        let norms = config.norms.clone();
        let symbols = config.symbols.clone();
        let email_regex = fancy_regex::Regex::new(&config.lexical.email_regex)?;
        let model = Self {
            email_regex,
            config,
            tensors,
            tokenizer,
            vector_keys,
            norms,
            symbols,
        };
        model.validate()?;
        Ok(model)
    }
    pub(crate) fn tensor(&self, key: &TensorRef) -> &Tensor {
        &self.tensors[&key.0]
    }
    pub(crate) fn string_id(&self, s: &str) -> u64 {
        if s.is_empty() {
            0
        } else {
            self.symbols
                .get(s)
                .copied()
                .unwrap_or_else(|| crate::hash::hash(s))
        }
    }
    pub(crate) fn norm(&self, s: &str) -> String {
        self.norms
            .get(&crate::hash::hash(s))
            .cloned()
            .unwrap_or_else(|| self.lower(s))
    }
    pub fn vector(&self, word: &str) -> Option<&[f32]> {
        let row = *self.vector_keys.get(&self.string_id(word))?;
        let t = &self.tensors["vectors"];
        let d = t.shape[1];
        Some(&t.data[row * d..(row + 1) * d])
    }
    pub fn document_vector(&self, doc: &crate::Doc) -> Vec<f32> {
        self.span_vector(
            doc,
            crate::TokenIndex(0),
            crate::TokenIndex(doc.tokens.len()),
        )
        .expect("valid document span")
    }
    pub fn span_vector(
        &self,
        doc: &crate::Doc,
        start: crate::TokenIndex,
        end: crate::TokenIndex,
    ) -> Result<Vec<f32>> {
        doc.span_text(start, end)?;
        let static_width = self.tensors["vectors"].shape[1];
        let contextual = static_width == 0 && start != end && !doc.tensor.is_empty();
        let mut v = vec![
            0.;
            if contextual {
                doc.tensor[0].len()
            } else {
                static_width
            }
        ];
        for i in start.0..end.0 {
            let vector = if contextual {
                Some(doc.tensor[i].as_slice())
            } else {
                self.vector(doc.token_text(crate::TokenIndex(i))?)
            };
            if let Some(x) = vector {
                for (a, b) in v.iter_mut().zip(x) {
                    *a += b
                }
            }
        }
        if start != end {
            for x in &mut v {
                *x /= (end.0 - start.0) as f32
            }
        }
        Ok(v)
    }
    pub fn similarity(&self, a: &crate::Doc, b: &crate::Doc) -> f32 {
        if a.tokens.len() == b.tokens.len()
            && (0..a.tokens.len()).all(|i| {
                a.token_text(crate::TokenIndex(i)).unwrap()
                    == b.token_text(crate::TokenIndex(i)).unwrap()
            })
        {
            return 1.;
        }
        let a = self.document_vector(a);
        let b = self.document_vector(b);
        let dot: f32 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
        let norm =
            (a.iter().map(|x| x * x).sum::<f32>() * b.iter().map(|x| x * x).sum::<f32>()).sqrt();
        if norm == 0. {
            0.
        } else {
            dot / norm
        }
    }
    fn validate(&self) -> Result<()> {
        crate::neural::validate(self)
    }
}
impl Model {
    pub fn token_vector<'a>(&'a self, token: crate::TokenView<'a>) -> Option<&'a [f32]> {
        if self.tensors["vectors"].shape[1] == 0 {
            token
                .span()
                .doc
                .tensor
                .get(token.index().0)
                .map(Vec::as_slice)
        } else {
            self.vector(token.text())
        }
    }
    pub fn span_similarity(&self, a: crate::SpanView<'_>, b: crate::SpanView<'_>) -> f32 {
        if a.end.0 - a.start.0 == b.end.0 - b.start.0
            && a.tokens()
                .zip(b.tokens())
                .all(|(a, b)| a.text() == b.text())
        {
            return 1.;
        }
        let a = self
            .span_vector(a.doc, a.start, a.end)
            .expect("checked span");
        let b = self
            .span_vector(b.doc, b.start, b.end)
            .expect("checked span");
        let dot: f32 = a.iter().zip(&b).map(|(a, b)| a * b).sum();
        let norm =
            (a.iter().map(|a| a * a).sum::<f32>() * b.iter().map(|b| b * b).sum::<f32>()).sqrt();
        if norm == 0. {
            0.
        } else {
            dot / norm
        }
    }
}
