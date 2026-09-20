use super::*;

fn tensor(shape: &[u64], bytes: &[u8]) -> Value {
    Value::Map(vec![
        (Value::Binary(b"nd".to_vec()), Value::Boolean(true)),
        (Value::Binary(b"type".to_vec()), Value::from("<f4")),
        (Value::Binary(b"kind".to_vec()), Value::Binary(Vec::new())),
        (
            Value::Binary(b"shape".to_vec()),
            Value::Array(shape.iter().copied().map(Value::from).collect()),
        ),
        (
            Value::Binary(b"data".to_vec()),
            Value::Binary(bytes.to_vec()),
        ),
    ])
}

fn model(parameter: Value) -> Value {
    Value::Map(vec![
        (Value::from("nodes"), Value::Array(vec![Value::Nil])),
        (Value::from("attrs"), Value::Array(vec![Value::Nil])),
        (Value::from("shims"), Value::Array(vec![Value::Nil])),
        (
            Value::from("params"),
            Value::Array(vec![Value::Map(vec![(Value::from("W"), parameter)])]),
        ),
    ])
}

fn pack(value: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    rmpv::encode::write_value(&mut bytes, value).unwrap();
    bytes
}

fn npy(header: &str, bytes: &[u8]) -> Vec<u8> {
    let mut result = b"\x93NUMPY\x01\x00".to_vec();
    result.extend_from_slice(&u16::try_from(header.len()).unwrap().to_le_bytes());
    result.extend_from_slice(header.as_bytes());
    result.extend_from_slice(bytes);
    result
}

#[test]
fn thinc_keeps_shape_and_every_float_bit() {
    // Known independent IEEE-754 encodings: 1.0, -2.0, 0.0, 0.5, 4.0, -0.0.
    let bytes = [
        0, 0, 128, 63, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 63, 0, 0, 128, 64, 0, 0, 0, 128,
    ];
    let mut parameters = decode_thinc(&pack(&model(tensor(&[2, 3], &bytes)))).unwrap();
    assert_eq!(
        parameters.take(0, "W").unwrap(),
        TensorData {
            shape: vec![2, 3],
            bytes: bytes.to_vec(),
        }
    );
    assert!(matches!(
        parameters.take(0, "W"),
        Err(TensorError::MissingParameter { .. })
    ));
    assert!(matches!(
        parameters.take(1, "W"),
        Err(TensorError::MissingParameter { .. })
    ));
}

#[test]
fn thinc_rejects_wrong_lengths_types_duplicates_and_trailing_data() {
    let value = tensor(&[2, 3], &[0; 24]);
    for field in ["nd", "type", "kind", "shape", "data"] {
        let mut changed = value.clone();
        let Value::Map(fields) = &mut changed else {
            panic!()
        };
        let (_, entry) = fields
            .iter_mut()
            .find(|(key, _)| key == &Value::Binary(field.as_bytes().to_vec()))
            .unwrap();
        *entry = Value::Nil;
        assert!(decode_thinc(&pack(&model(changed))).is_err(), "{field}");
    }
    for invalid in [
        tensor(&[2, 3], &[0; 20]),
        tensor(&[2, 3], &[0; 28]),
        tensor(&[u64::MAX, 2], &[]),
        Value::Boolean(false),
    ] {
        assert!(decode_thinc(&pack(&model(invalid))).is_err());
    }
    let mut duplicate = value.clone();
    let Value::Map(fields) = &mut duplicate else {
        panic!()
    };
    fields.push((Value::from("data"), Value::Binary(vec![0; 24])));
    assert!(decode_thinc(&pack(&model(duplicate))).is_err());
    let mut trailing = pack(&model(value));
    trailing.push(0);
    assert!(decode_thinc(&trailing).is_err());
    assert!(decode_thinc(&trailing[..3]).is_err());
}

#[test]
fn thinc_rejects_endianness_and_misaligned_nodes() {
    let mut wrong_endian = tensor(&[1], &[0; 4]);
    let Value::Map(fields) = &mut wrong_endian else {
        panic!()
    };
    fields[1].1 = Value::from(">f4");
    assert!(matches!(
        decode_thinc(&pack(&model(wrong_endian))),
        Err(TensorError::Unsupported(_))
    ));
    let mut misaligned = model(tensor(&[1], &[0; 4]));
    let Value::Map(fields) = &mut misaligned else {
        panic!()
    };
    fields[0].1 = Value::Array(Vec::new());
    assert!(decode_thinc(&pack(&misaligned)).is_err());
    let Value::Map(fields) = &mut misaligned else {
        panic!()
    };
    fields[0].1 = Value::Array(vec![Value::Nil]);
    fields.push((Value::from("nodes"), Value::Array(vec![Value::Nil])));
    assert!(decode_thinc(&pack(&misaligned)).is_err());
}

#[test]
fn thinc_accepts_empty_scalar_and_unset_parameters() {
    let mut empty = decode_thinc(&pack(&model(tensor(&[0, 3], &[])))).unwrap();
    assert_eq!(empty.take(0, "W").unwrap().shape, [0, 3]);
    let mut scalar = decode_thinc(&pack(&model(tensor(&[], &[0, 0, 128, 63])))).unwrap();
    assert_eq!(scalar.take(0, "W").unwrap().shape, Vec::<usize>::new());
    let mut unset = decode_thinc(&pack(&model(Value::Nil))).unwrap();
    assert!(matches!(
        unset.take(0, "W"),
        Err(TensorError::MissingParameter { .. })
    ));
}

#[test]
fn numpy_reads_rectangular_empty_and_scalar_arrays() {
    let header = "{'descr': '<f4', 'fortran_order': False, 'shape': (2, 3), }        \n";
    let bytes = [
        0, 0, 128, 63, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 63, 0, 0, 128, 64, 0, 0, 0, 128,
    ];
    assert_eq!(
        decode_npy(&npy(header, &bytes)).unwrap(),
        TensorData {
            shape: vec![2, 3],
            bytes: bytes.to_vec()
        }
    );
    let empty = "{'shape': (0,), 'descr': '<f4', 'fortran_order': False}\n";
    assert_eq!(decode_npy(&npy(empty, &[])).unwrap().shape, [0]);
    let scalar = "{\"shape\": (), \"fortran_order\": False, \"descr\": \"<f4\"}\n";
    assert!(decode_npy(&npy(scalar, &[0; 4])).unwrap().shape.is_empty());
}

#[test]
fn numpy_rejects_malformed_or_unsupported_headers_without_evaluation() {
    for header in [
        "{'descr': '>f4', 'fortran_order': False, 'shape': (1,)}\n",
        "{'descr': '<f8', 'fortran_order': False, 'shape': (1,)}\n",
        "{'descr': '<f4', 'fortran_order': True, 'shape': (1,)}\n",
        "{'descr': '<f4', 'fortran_order': False, 'shape': (1)}\n",
        "{'descr': '<f4', 'fortran_order': False, 'shape': (-1,)}\n",
        "{'descr': '<f4', 'fortran_order': False, 'shape': (18446744073709551616,)}\n",
        "{'descr': '<f4', 'fortran_order': False, 'shape': (1,), 'shape': (1,)}\n",
        "{'descr': '<f4', 'fortran_order': False, 'shape': (1,), 'unknown': 1}\n",
        "{'descr': '<f4', 'fortran_order': False}\n",
        "{'descr': '<f4', 'shape': (1,)}\n",
        "{'fortran_order': False, 'shape': (1,)}\n",
        "{'descr': '<f4', 'fortran_order': False, 'shape': (1,)}; print('bad')\n",
        "{'descr': '<f4', 'fortran_order': False, 'shape': __import__('os')}\n",
        "{'descr': '<f4', 'fortran_order': False, 'shape': (1,)}",
    ] {
        assert!(decode_npy(&npy(header, &[0; 4])).is_err(), "{header}");
    }
}

#[test]
fn numpy_rejects_truncation_trailing_bytes_and_unknown_versions() {
    let header = "{'descr': '<f4', 'fortran_order': False, 'shape': (1,)}\n";
    let valid = npy(header, &[0; 4]);
    for length in [0, 5, 8, 9, 10, 12, valid.len() - 1] {
        assert!(decode_npy(&valid[..length]).is_err(), "{length}");
    }
    let mut extra = valid.clone();
    extra.push(0);
    assert!(decode_npy(&extra).is_err());
    let mut version = valid;
    version[6] = 2;
    assert!(matches!(
        decode_npy(&version),
        Err(TensorError::Unsupported(_))
    ));
}
