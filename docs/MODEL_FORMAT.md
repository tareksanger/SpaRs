# Native model format v1

A model directory contains `manifest.json`, `weights.safetensors`, model LICENSE,
LICENSES_SOURCES, upstream source/license notices and source-lock.json. All numeric
parameters and static vectors are F32, row-major, in SafeTensors. The manifest's
SHA256 authenticates the weights against that manifest (it is not a digital
signature for an untrusted manifest). Acquisition separately pins the official
wheel SHA256.

Required top-level fields:

- format_version=1; model=en_core_web_md; model_version=3.8.0; versions with spaCy
  3.8.14 and Thinc 8.3.13; official source URL, wheel digest, metadata/license,
  original config text and ordered pipeline.
- tensors: map of tensor keys to {shape: positive dimension array, dtype: F32}.
  SafeTensors validates offsets/lengths; the loader checks actual shapes, dtypes,
  finite values and operation-specific cross-dimension constraints.
- tokenizer: prefix/suffix/infix/url/token_match; rules map source strings to
  ordered ORTH/NORM entries. python_patterns retain original Python regexes.
  regex_dialect identifies explicit Python-Unicode shorthand translation.
- lexical: pinned Unicode version, sorted inclusive property ranges, lowercase
  mapping, stop words, number words, punctuation sets, TLDs, email regex.
- symbols: reserved spaCy IDs. norms: hashed lexical keys to string replacements,
  with model lookups overriding BASE_NORMS. IDs use JSON integer values or decimal
  string map keys; they must not be rounded through JavaScript Number.
- tok2vec: width, ordered attrs, hash table references and seeds, static projection,
  mix maxout/normalization parameters, windows, encoder layers and padding count.
- tagger: ordered labels and affine parameter references.
- parser/ner: ordered action names, reduction/lower/upper tensor references; NER
  additionally contains its independent tok2vec. Format v1 specifies greedy
  maxout transition inference with no unseen-class mask.
- attribute_rules: ordered patterns/attributes/index; only the pinned model's
  finite TAG/DEP/LOWER/IS_SPACE predicates with equality/IN/NOT_IN/REGEX are accepted.
- lemmas: lemma_index, lemma_exc, lemma_rules per POS.
- vector_keys: lexical hash to row; vectors tensor provides row width/count.

Dimensions and label counts are loaded, never inferred from filenames or the WASM
bundle. The pinned architecture semantics define concatenation, zero padding,
learned parser padding, residual order, layer normalization epsilon 1e-8, and
first-index tie breaking. Lower transition weights are [feature, output, piece,
input]; pad is [1, feature, output, piece]. Unnormalized tagger scores are used.

Unsupported versions, components, feature types or operators fail explicitly.
Changing this representation or numerical semantics requires a new format version
and fresh compatibility evidence. Model acquisition is never performed by loading
or processing. The manifest uses ordinary JSON; [model-v1.schema.json](model-v1.schema.json)
specifies its structural contract. The loader enforces structural and tensor
constraints in [validation.rs](../src/validation.rs) and the `validate` function
in [neural.rs](../src/neural.rs).
