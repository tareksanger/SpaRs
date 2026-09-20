use crate::{invalid, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Cursor};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredLemmas {
    lemma_index: BTreeMap<u64, Vec<String>>,
    lemma_exc: BTreeMap<u64, BTreeMap<String, Vec<String>>>,
    lemma_rules: BTreeMap<u64, Vec<[String; 2]>>,
}
#[derive(Serialize)]
#[serde(untagged)]
enum Table<T> {
    Values(Vec<T>),
    Empty(BTreeMap<String, ()>),
}
#[derive(Serialize)]
pub(crate) struct Lemmas {
    lemma_index: BTreeMap<String, Table<String>>,
    lemma_exc: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    lemma_rules: BTreeMap<String, Table<[String; 2]>>,
}
fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let mut cursor = Cursor::new(bytes);
    let value =
        rmpv::decode::read_value(&mut cursor).map_err(|e| invalid(format!("MessagePack: {e}")))?;
    if cursor.position() != bytes.len() as u64 {
        return Err(invalid("trailing MessagePack data"));
    }
    rmpv::ext::from_value(value).map_err(|e| invalid(format!("lookup schema: {e}")))
}
pub(crate) fn vector_keys(bytes: &[u8], rows: usize) -> Result<BTreeMap<u64, usize>> {
    let keys: BTreeMap<u64, usize> = decode(bytes)?;
    if keys.values().any(|&row| row >= rows) {
        return Err(invalid("vector row outside tensor"));
    }
    Ok(keys)
}
pub(crate) fn lemmas(bytes: &[u8], pos: &BTreeMap<String, u64>) -> Result<Lemmas> {
    let mut stored: StoredLemmas = decode(bytes)?;
    let mut result = Lemmas {
        lemma_index: BTreeMap::new(),
        lemma_exc: BTreeMap::new(),
        lemma_rules: BTreeMap::new(),
    };
    for (name, id) in pos {
        result.lemma_index.insert(
            name.clone(),
            stored
                .lemma_index
                .remove(id)
                .map(Table::Values)
                .unwrap_or_else(|| Table::Empty(BTreeMap::new())),
        );
        result.lemma_rules.insert(
            name.clone(),
            stored
                .lemma_rules
                .remove(id)
                .map(Table::Values)
                .unwrap_or_else(|| Table::Empty(BTreeMap::new())),
        );
        result.lemma_exc.insert(
            name.clone(),
            stored.lemma_exc.remove(id).unwrap_or_default(),
        );
    }
    // The pinned exporter selects these five POS tables; remaining upstream
    // lookup tables are unused by this native lemmatizer.
    Ok(result)
}
#[cfg(test)]
mod tests;
