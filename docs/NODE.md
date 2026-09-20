# Use SpaRs from Node.js

The separate `spars-node` crate exposes native Rust inference to Node.js through Node-API. Load a model once, then process text or batches with a typed TypeScript API. The binding runs model loading and inference on Node's background worker threads. It uses the same Rust library and official model assets as Rust applications.

## Build and install a model

Use Node.js 24 or newer, npm, and Rust 1.88 or newer. Run these commands from the repository root:

```sh
npm --prefix bindings/node ci --ignore-scripts --no-audit --no-fund
cargo fetch --locked --manifest-path bindings/node/Cargo.toml
npm --prefix bindings/node run build
cargo build --locked --release --manifest-path installer/Cargo.toml
model_dir=$(installer/target/release/spars-model install --version 3.8.0 --root target/models)
export SPARS_MODEL="$model_dir"
```

Installation requires no Python. See [model installation](MODEL_INSTALLATION.md) for local archives and checksums. The binding loads an existing model directory and never downloads assets during inference. Build the addon locally for your operating system and CPU; prebuilt npm packages and Windows verification remain future work. Node dependencies are confined to `bindings/node/` and do not enter ordinary Rust consumer builds.

## Process text

Save this example as `bindings/node/analyze.mts`, then run `node bindings/node/analyze.mts` from the repository root. For reference development, the example also accepts the standard exported assets directory when `SPARS_MODEL` is unset. Verification executes this code directly from this guide.

```typescript
import assert from 'node:assert/strict';
import { loadModel } from './index.js';

const model = await loadModel(process.env.SPARS_MODEL ?? 'assets/en_core_web_md-3.8.0');
const doc = await model.process('Alice works in London.');
assert.equal(doc.tokens[0]?.text, 'Alice');
assert.ok(doc.entities);
const places = doc.entities.filter(entity => entity.label === 'GPE').map(entity => {
  const first = doc.tokens[entity.start];
  const last = doc.tokens[entity.end - 1];
  assert.ok(first && last);
  return doc.text.slice(first.utf16Start, last.utf16End);
});
assert.deepEqual(places, ['London']);
console.log(places);
```

This prints `[ 'London' ]`. `GPE` labels a country, city, or similar political area. Entity, sentence, and noun-chunk spans use token indices: `start` is included and `end` is excluded. Tokens include text, trailing whitespace, normalization, grammatical tags, part-of-speech labels, morphology, base forms (lemmas), dependency heads and labels, sentence starts, and entity annotations. Returned objects own their data; editing them cannot change the model or later results.

Use `utf16Start` and `utf16End` with JavaScript's `slice`. `byteStart` and `byteEnd` count UTF-8 bytes; `codePointStart` and `codePointEnd` count Unicode code points. TypeScript treats these as distinct types to prevent accidental interchange. Offsets and token indices are checked unsigned 32-bit values. Text containing an unpaired UTF-16 surrogate is rejected because it cannot be preserved as valid Rust text.

## Batches, stages, and vectors

This example can also be saved beside `index.js` and run from the repository root:

```typescript
import assert from 'node:assert/strict';
import { loadModel } from './index.js';

const model = await loadModel(process.env.SPARS_MODEL ?? 'assets/en_core_web_md-3.8.0');
const docs = await model.processBatch(['😀 café', ''], 'Tokenizer');
assert.equal(docs[0]?.tokens[0]?.utf16End, 2);
assert.equal(docs[0]?.tokens[0]?.byteEnd, 4);
assert.equal(docs[0]?.entities, null);
assert.deepEqual((await model.process('')).entities, []);
const vector = model.vector('dog');
assert.ok(vector instanceof Float32Array);
assert.equal(vector.length, 300);
assert.equal(model.vector('spars_unknown_🙂_lexeme'), null);
```

`process(text, stage?)` and `processBatch(texts, stage?)` return promises. The default runs the full pipeline. A stage selects an ordered prefix: `Tokenizer`, `Tagger`, `Parser`, `AttributeRuler`, `Lemmatizer`, or `Ner`. Later annotations remain `null`; computed empty lists remain `[]`. The early-stage unavailable-value contract follows SpaRs and does not reproduce every default value in a partially processed Python Doc.

Each batch runs sequentially in one worker job and retains input order. A failure rejects the entire batch. Concurrent calls share the immutable model and own their processing state. Node's worker pool limits active jobs, but queued inputs and complete batch results still occupy memory. Keep batches and concurrent submissions bounded for your workload. Input copying and result creation take place on the JavaScript thread; large batches can still delay it. This API does not stream results or support cancellation.

`vector(word)` is a synchronous, case-sensitive static-vector lookup. It returns a copied `Float32Array`, or `null` when the lexical key has no row. Modifying the array is safe. Document/span vectors, similarities, traversal helpers, matchers, and model installation are not yet exposed through this binding; their Rust APIs remain available separately.

## Errors and verification

Model and processing failures reject their promises with an `Error` containing a `code` and message. Codes are `SPARS_IO`, `SPARS_INVALID_MODEL`, `SPARS_UNSUPPORTED`, `SPARS_INVALID_TEXT`, `SPARS_BOUNDS`, and `SPARS_INFERENCE`. Invalid JavaScript argument types or unknown stages throw immediately through Node-API validation. `vector` also throws immediately for malformed text. Use `loadModel`; constructing `Model` directly is unsupported and its TypeScript constructor is private.

After the [reference setup](DEVELOPMENT.md#set-up-the-project), run:

```sh
npm --prefix bindings/node run build
npm --prefix bindings/node run typecheck
npm --prefix bindings/node test
.venv/bin/python tools/check_docs.py
```

Tests execute against the required model assets; missing assets fail. The binding compares all discrete outputs on the frozen 190-document / 7,029-token suite, checks Unicode offsets, eight static-vector lookup cases, stage availability, error paths, mutation isolation, batches, concurrent jobs, and model lifetime during garbage collection. Type checking checks generated declarations and distinct offset types. The full [acceptance command](QUALITY.md#run-the-acceptance-checks) includes these checks. Generated mismatch reports belong under `target/reports/` and CI retains them as artifacts.

The implementation follows [napi-rs worker tasks](https://napi.rs/docs/concepts/async-task) and [object conversion](https://napi.rs/docs/concepts/object). Rust and npm lockfiles pin the binding dependencies. The build corrects napi-rs's missing private-constructor declaration and fails if that generated class changes unexpectedly.

For repeatable local measurements, this command processes the short-document portion of the frozen evaluation corpus with one warmup pass and three measured passes:

```sh
UV_THREADPOOL_SIZE=1 node bindings/node/scripts/measure.mts assets/en_core_web_md-3.8.0 fixtures/evaluation-v1.json short single 3 1 96 Ner
```

Use `long` for the two long documents, `batch` or `concurrent` instead of `single`, and `Tokenizer` instead of `Ner` to measure tokenization alone. The argument `96` limits each batch or group of concurrent calls to 96 documents; a smaller corpus produces a smaller group. Output separates model loading, awaited processing, peak process memory, submission time, and maximum timer gap. Awaited processing includes scheduling and result conversion; it is not a measurement of Rust inference alone. Repeat runs without competing workloads and record hardware, build mode, thread limits, and corpus identity as described in [performance measurements](PERFORMANCE.md).
