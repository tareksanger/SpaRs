use serde::{Deserialize, Deserializer};

/// Processing metadata from the preserved upstream config; no model-name branches.
pub(crate) struct ProcessingDefaults {
    pub batch_size: u32,
}
impl Default for ProcessingDefaults {
    fn default() -> Self {
        Self { batch_size: 1000 }
    }
}
impl<'de> Deserialize<'de> for ProcessingDefaults {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let config = String::deserialize(deserializer)?;
        parse(&config).map_err(serde::de::Error::custom)
    }
}
fn parse(config: &str) -> Result<ProcessingDefaults, &'static str> {
    let mut in_nlp = false;
    let mut seen_nlp = false;
    let mut batch_size = None;
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            let (header, trailing) = line
                .split_once(']')
                .ok_or("malformed config section header")?;
            let trailing = trailing.trim();
            if !trailing.is_empty() && !trailing.starts_with(['#', ';']) {
                return Err("unsupported text after config section header");
            }
            in_nlp = header == "[nlp";
            if in_nlp && seen_nlp {
                return Err("duplicate [nlp] config section");
            }
            seen_nlp |= in_nlp;
            continue;
        }
        if !in_nlp {
            continue;
        }
        let Some((key, value)) = line.split_once(['=', ':']) else {
            if line.split_whitespace().next() == Some("batch_size") {
                return Err("nlp.batch_size requires an assignment");
            }
            continue;
        };
        if key.trim() != "batch_size" {
            continue;
        }
        if batch_size.is_some() {
            return Err("duplicate nlp.batch_size");
        }
        let value = value.trim();
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(
                "nlp.batch_size must be a literal positive u32; interpolation is unsupported",
            );
        }
        let size = value
            .parse::<u32>()
            .map_err(|_| "nlp.batch_size exceeds u32")?;
        if size == 0 {
            return Err("nlp.batch_size must be positive");
        }
        batch_size = Some(size);
    }
    Ok(ProcessingDefaults {
        batch_size: batch_size.unwrap_or(1000),
    })
}
#[cfg(test)]
mod tests;
