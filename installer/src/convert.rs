use crate::{
    archive::Wheel,
    invalid, lookups,
    recipe::{Recipe, TensorSource},
    tensors::{self, TensorData},
    Result,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};
#[derive(Deserialize)]
struct TemplateMetadata {
    weights_sha256: crate::Digest,
    tensors: BTreeMap<String, TensorSpec>,
}
#[derive(Deserialize)]
struct TensorSpec {
    shape: Vec<usize>,
    dtype: Dtype,
}
#[derive(Deserialize)]
enum Dtype {
    F32,
}
#[derive(Serialize)]
struct Manifest<'a> {
    // Opaque, checksum-pinned JSON fragments are preserved only at serialization.
    // The runtime's typed manifest loader validates all fields before installation.
    #[serde(flatten)]
    template: &'a BTreeMap<String, Box<RawValue>>,
    vector_keys: BTreeMap<u64, usize>,
    lemmas: lookups::Lemmas,
}
pub(crate) fn convert(wheel: &mut Wheel, recipe: &Recipe, destination: &Path) -> Result<()> {
    let metadata: TemplateMetadata = serde_json::from_slice(recipe.bundle.template)?;
    let mut parameters = BTreeMap::new();
    let mut tensors: BTreeMap<String, TensorData> = BTreeMap::new();
    for (name, source) in &recipe.sources {
        let data = match source {
            TensorSource::Thinc {
                entry,
                node,
                parameter,
            } => {
                if !parameters.contains_key(entry) {
                    let bytes = wheel.read(entry, &recipe.inputs[entry])?;
                    parameters.insert(entry.clone(), tensors::decode_thinc(&bytes)?);
                }
                parameters
                    .get_mut(entry)
                    .ok_or_else(|| invalid("missing decoded component"))?
                    .take(*node, parameter)?
            }
            TensorSource::Npy { entry } => {
                tensors::decode_npy(&wheel.read(entry, &recipe.inputs[entry])?)?
            }
        };
        let spec = metadata
            .tensors
            .get(name)
            .ok_or_else(|| invalid("unknown tensor mapping"))?;
        if data.shape != spec.shape || !matches!(spec.dtype, Dtype::F32) {
            return Err(invalid(format!("tensor shape/dtype mismatch: {name}")));
        }
        tensors.insert(name.clone(), data);
    }
    if tensors.len() != metadata.tensors.len() {
        return Err(invalid("missing tensor mapping"));
    }
    drop(parameters);
    let rows = tensors
        .get("vectors")
        .and_then(|t| t.shape.first())
        .copied()
        .ok_or_else(|| invalid("missing vectors"))?;
    let views = tensors
        .iter()
        .map(|(name, t)| {
            safetensors::tensor::TensorView::new(safetensors::Dtype::F32, t.shape.clone(), &t.bytes)
                .map(|v| (name.as_str(), v))
                .map_err(|e| invalid(e.to_string()))
        })
        .collect::<Result<Vec<_>>>()?;
    let weights_path = destination.join("weights.safetensors");
    safetensors::tensor::serialize_to_file(views, None, &weights_path)
        .map_err(|e| invalid(e.to_string()))?;
    if crate::archive::hash_file(&weights_path)? != metadata.weights_sha256 {
        return Err(invalid(
            "converted weights differ from official reference export",
        ));
    }
    drop(tensors);
    let vector_keys = lookups::vector_keys(
        &wheel.read(
            &recipe.vector_keys_entry,
            &recipe.inputs[&recipe.vector_keys_entry],
        )?,
        rows,
    )?;
    let lemmas = lookups::lemmas(
        &wheel.read(&recipe.lemmas_entry, &recipe.inputs[&recipe.lemmas_entry])?,
        &recipe.lemma_pos,
    )?;
    let template: BTreeMap<String, Box<RawValue>> = serde_json::from_slice(recipe.bundle.template)?;
    let mut output = BufWriter::new(File::create(destination.join("manifest.json"))?);
    serde_json::to_writer(
        &mut output,
        &Manifest {
            template: &template,
            vector_keys,
            lemmas,
        },
    )?;
    output.flush()?;
    drop(output);
    for name in recipe
        .resources
        .keys()
        .filter(|name| name.as_str() != "manifest-template.json")
    {
        std::fs::write(destination.join(name), recipe.bundle.resource(name)?)?;
    }
    for name in ["LICENSE", "LICENSES_SOURCES"] {
        let entry = format!("{}{name}", recipe.prefix());
        std::fs::write(
            destination.join(name),
            wheel.read(&entry, &recipe.inputs[&entry])?,
        )?;
    }
    Ok(())
}
