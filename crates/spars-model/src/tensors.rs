//! Readers for the pinned Thinc/srsly arrays and NumPy vector file.
//!
//! These are data readers, not Python pickle readers. They accept only C-order,
//! little-endian float32 arrays used by the reviewed model package.

use std::collections::BTreeMap;
use std::fmt;
use std::io::Cursor;

use rmpv::Value;

#[derive(Debug, PartialEq, Eq)]
pub enum TensorError {
    Malformed(String),
    Unsupported(String),
    MissingParameter { node: usize, parameter: String },
}

impl fmt::Display for TensorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(reason) => write!(formatter, "malformed tensor data: {reason}"),
            Self::Unsupported(reason) => write!(formatter, "unsupported tensor data: {reason}"),
            Self::MissingParameter { node, parameter } => {
                write!(
                    formatter,
                    "missing tensor parameter {parameter:?} at node {node}"
                )
            }
        }
    }
}

impl std::error::Error for TensorError {}

type Result<T> = std::result::Result<T, TensorError>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TensorData {
    pub(crate) shape: Vec<usize>,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct ThincParameters {
    nodes: Vec<BTreeMap<String, TensorData>>,
}

impl ThincParameters {
    /// Move the selected parameter out without copying its allocation.
    pub(crate) fn take(&mut self, node: usize, parameter: &str) -> Result<TensorData> {
        self.nodes
            .get_mut(node)
            .and_then(|parameters| parameters.remove(parameter))
            .ok_or_else(|| TensorError::MissingParameter {
                node,
                parameter: parameter.to_owned(),
            })
    }
}

fn malformed(reason: impl Into<String>) -> TensorError {
    TensorError::Malformed(reason.into())
}

fn checked_tensor(shape: Vec<usize>, bytes: Vec<u8>) -> Result<TensorData> {
    let size = shape.iter().try_fold(4_usize, |size, &dimension| {
        size.checked_mul(dimension)
            .ok_or_else(|| malformed("array shape overflows byte length"))
    })?;
    if size != bytes.len() {
        return Err(malformed(format!(
            "shape requires {size} bytes, received {}",
            bytes.len()
        )));
    }
    Ok(TensorData { shape, bytes })
}

fn map(value: Value, context: &str) -> Result<BTreeMap<String, Value>> {
    let Value::Map(entries) = value else {
        return Err(malformed(format!("{context} must be a map")));
    };
    let mut result = BTreeMap::new();
    for (key, value) in entries {
        let key = match key {
            Value::String(key) => key
                .into_str()
                .ok_or_else(|| malformed("invalid UTF-8 key"))?,
            Value::Binary(key) => {
                String::from_utf8(key).map_err(|_| malformed("invalid UTF-8 key"))?
            }
            _ => return Err(malformed(format!("{context} has a non-string key"))),
        };
        if result.insert(key.clone(), value).is_some() {
            return Err(malformed(format!("duplicate {context} key {key:?}")));
        }
    }
    Ok(result)
}

fn remove(map: &mut BTreeMap<String, Value>, key: &str) -> Result<Value> {
    map.remove(key)
        .ok_or_else(|| malformed(format!("missing field {key:?}")))
}

fn array(value: Value, context: &str) -> Result<Vec<Value>> {
    match value {
        Value::Array(items) => Ok(items),
        _ => Err(malformed(format!("{context} must be an array"))),
    }
}

fn ndarray(value: Value) -> Result<TensorData> {
    let mut fields = map(value, "ndarray")?;
    if remove(&mut fields, "nd")? != Value::Boolean(true) {
        return Err(TensorError::Unsupported("NumPy scalar or object".into()));
    }
    let dtype = remove(&mut fields, "type")?;
    if dtype.as_str() != Some("<f4") {
        return Err(TensorError::Unsupported(format!(
            "dtype {dtype}; expected little-endian float32 (<f4)"
        )));
    }
    if remove(&mut fields, "kind")? != Value::Binary(Vec::new()) {
        return Err(TensorError::Unsupported("structured NumPy dtype".into()));
    }
    let shape = array(remove(&mut fields, "shape")?, "ndarray shape")?
        .into_iter()
        .map(|dimension| {
            dimension
                .as_u64()
                .and_then(|dimension| usize::try_from(dimension).ok())
                .ok_or_else(|| malformed("shape dimension must be a nonnegative integer"))
        })
        .collect::<Result<Vec<_>>>()?;
    let Value::Binary(bytes) = remove(&mut fields, "data")? else {
        return Err(malformed("ndarray data must be binary"));
    };
    if !fields.is_empty() {
        return Err(TensorError::Unsupported("unknown ndarray fields".into()));
    }
    checked_tensor(shape, bytes)
}

/// Decode once per component. Parameter indices follow `Model.walk()` as stored
/// by the pinned Thinc `Model.to_dict()` implementation.
pub(crate) fn decode_thinc(bytes: &[u8]) -> Result<ThincParameters> {
    let mut cursor = Cursor::new(bytes);
    let value = rmpv::decode::read_value_with_max_depth(&mut cursor, 64)
        .map_err(|error| malformed(format!("MessagePack: {error}")))?;
    if cursor.position() != bytes.len() as u64 {
        return Err(malformed("trailing MessagePack data"));
    }
    let mut fields = map(value, "Thinc model")?;
    let parameters = array(remove(&mut fields, "params")?, "parameters")?;
    for key in ["nodes", "attrs", "shims"] {
        let values = array(remove(&mut fields, key)?, key)?;
        if values.len() != parameters.len() {
            return Err(malformed(format!(
                "{key} and parameters have different node counts"
            )));
        }
    }
    if !fields.is_empty() {
        return Err(TensorError::Unsupported(
            "unknown Thinc model fields".into(),
        ));
    }
    let nodes = parameters
        .into_iter()
        .map(|parameters| {
            map(parameters, "parameters")?
                .into_iter()
                .filter(|(_, value)| !value.is_nil())
                .map(|(name, value)| ndarray(value).map(|tensor| (name, tensor)))
                .collect::<Result<BTreeMap<_, _>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ThincParameters { nodes })
}

/// Read NumPy's literal header without evaluating Python code. The supported
/// format is v1.0, which is the format stored in the pinned official package.
pub(crate) fn decode_npy(bytes: &[u8]) -> Result<TensorData> {
    if bytes.get(..6) != Some(b"\x93NUMPY") {
        return Err(malformed("missing NumPy file signature"));
    }
    if bytes.get(6..8) != Some(&[1, 0]) {
        return Err(TensorError::Unsupported(
            "NumPy version; expected 1.0".into(),
        ));
    }
    let length = bytes
        .get(8..10)
        .ok_or_else(|| malformed("truncated NumPy header length"))?;
    let header_end = 10 + usize::from(u16::from_le_bytes([length[0], length[1]]));
    let header = bytes
        .get(10..header_end)
        .ok_or_else(|| malformed("truncated NumPy header"))?;
    if header.last() != Some(&b'\n') {
        return Err(malformed("NumPy header must end in a newline"));
    }
    let text = std::str::from_utf8(header).map_err(|_| malformed("non-ASCII NumPy header"))?;
    let mut parser = HeaderParser { rest: text };
    let shape = parser.parse()?;
    checked_tensor(shape, bytes[header_end..].to_vec())
}

struct HeaderParser<'a> {
    rest: &'a str,
}

impl HeaderParser<'_> {
    fn trim(&mut self) {
        self.rest = self
            .rest
            .trim_start_matches(|c: char| c.is_ascii_whitespace());
    }

    fn consume(&mut self, value: &str) -> bool {
        self.trim();
        if let Some(rest) = self.rest.strip_prefix(value) {
            self.rest = rest;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, value: &str) -> Result<()> {
        if self.consume(value) {
            Ok(())
        } else {
            Err(malformed(format!("expected {value:?} in NumPy header")))
        }
    }

    fn string(&mut self) -> Result<String> {
        self.trim();
        let quote = self
            .rest
            .chars()
            .next()
            .ok_or_else(|| malformed("missing header string"))?;
        if quote != '\'' && quote != '"' {
            return Err(malformed("expected a quoted header string"));
        }
        self.rest = &self.rest[1..];
        let end = self
            .rest
            .find(quote)
            .ok_or_else(|| malformed("unterminated header string"))?;
        let value = self.rest[..end].to_owned();
        if !value.is_ascii() || value.contains('\\') {
            return Err(TensorError::Unsupported(
                "escaped or non-ASCII header string".into(),
            ));
        }
        self.rest = &self.rest[end + 1..];
        Ok(value)
    }

    fn shape(&mut self) -> Result<Vec<usize>> {
        self.expect("(")?;
        let mut shape = Vec::new();
        if self.consume(")") {
            return Ok(shape);
        }
        loop {
            self.trim();
            let end = self.rest.bytes().take_while(u8::is_ascii_digit).count();
            if end == 0 {
                return Err(malformed("shape dimension must be a nonnegative integer"));
            }
            shape.push(
                self.rest[..end]
                    .parse()
                    .map_err(|_| malformed("shape dimension overflows usize"))?,
            );
            self.rest = &self.rest[end..];
            if self.consume(")") {
                if shape.len() == 1 {
                    return Err(malformed("one-dimensional shape requires a tuple comma"));
                }
                break;
            }
            self.expect(",")?;
            if self.consume(")") {
                break;
            }
        }
        Ok(shape)
    }

    fn parse(&mut self) -> Result<Vec<usize>> {
        self.expect("{")?;
        let mut dtype = None;
        let mut fortran_order = None;
        let mut shape = None;
        loop {
            let key = self.string()?;
            self.expect(":")?;
            let duplicate = match key.as_str() {
                "descr" => dtype.replace(self.string()?).is_some(),
                "fortran_order" => {
                    let value = if self.consume("False") {
                        false
                    } else {
                        self.expect("True")?;
                        true
                    };
                    fortran_order.replace(value).is_some()
                }
                "shape" => shape.replace(self.shape()?).is_some(),
                _ => {
                    return Err(TensorError::Unsupported(format!(
                        "NumPy header field {key:?}"
                    )))
                }
            };
            if duplicate {
                return Err(malformed(format!("duplicate NumPy header field {key:?}")));
            }
            if self.consume("}") {
                break;
            }
            self.expect(",")?;
            if self.consume("}") {
                break;
            }
        }
        self.trim();
        if !self.rest.is_empty() {
            return Err(malformed("trailing NumPy header content"));
        }
        match dtype.as_deref() {
            Some("<f4") => {}
            Some(dtype) => return Err(TensorError::Unsupported(format!("NumPy dtype {dtype:?}"))),
            None => return Err(malformed("missing NumPy dtype")),
        }
        match fortran_order {
            Some(false) => {}
            Some(true) => return Err(TensorError::Unsupported("Fortran-order NumPy array".into())),
            None => return Err(malformed("missing NumPy order")),
        }
        shape.ok_or_else(|| malformed("missing NumPy shape"))
    }
}

#[cfg(test)]
mod tests;
