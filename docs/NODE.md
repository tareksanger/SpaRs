# Use SpaRs from Node.js

The separate `spars-node` crate exposes native Rust inference to Node.js through Node-API. Load a model once, then process text or batches with a typed TypeScript API. The binding runs model loading and inference on Node's background worker threads. It uses the same Rust library and official model assets as Rust applications.

## Build and install a model

Use Node.js 24 or newer, npm, and Rust 1.88 or newer. Run these commands from the repository root:

```sh
npm --prefix bindings/node ci --ignore-scripts --no-audit --no-fund
cargo fetch --locked --manifest-path bindings/node/Cargo.toml
npm --prefix bindings/node run build
export SPARS_MODEL_DIR="./target/models"
node bindings/node/bin/spars.mjs download en_core_web_lg
```

Installation requires no Python. See [model installation](MODEL_INSTALLATION.md) for local archives and checksums. The Node package includes the `spars` CLI and an asynchronous `downloadModel` API backed by the native Rust installer. No separate installer executable is needed. `loadModel("en_core_web_lg")` resolves the installation under `SPARS_MODEL_DIR` and never downloads assets during loading or inference. Build the addon locally for your operating system and CPU. The [npm release workflow](RELEASING.md#publish-the-node-package) packages Linux x64 glibc and macOS ARM64 builds and tests the resulting tarballs before optional publication. Windows verification remains future work; the workflow alone does not establish registry availability. Node dependencies are confined to `bindings/node/` and do not enter ordinary Rust consumer builds.

## Download and load by name

Set `SPARS_MODEL_DIR` in your shell, service, or application environment before downloading or loading. The library does not read `.env` files automatically or write project configuration. Without this variable, the shared default is `Library/Caches/spars/models` under the user home on macOS, `spars/models` under `LOCALAPPDATA` on Windows, and `spars/models` under `XDG_CACHE_HOME` (or `.cache` under the user home) on other systems. Relative values use the process working directory; an absolute path avoids differences between launch directories.

Once the Node package is installed locally, run `npx spars download en_core_web_lg`. From the source checkout, use `node bindings/node/bin/spars.mjs download en_core_web_lg`. Both accept `--path`, `--version`, and `--archive`. Omitting the version selects the catalog's single pinned release, never a remote latest release. `--path` affects that command only; set `SPARS_MODEL_DIR` or pass `{path: "models"}` to later `loadModel` calls to use the same custom directory.

The following executable example uses the official wheel acquired by the reference setup to run offline. Omit `archive` to download from the pinned official URL. Guide verification configures a temporary model store before running this example.

```typescript
import assert from 'node:assert/strict';
import { downloadModel, loadModel } from './index.js';

await downloadModel('en_core_web_sm', {
  archive: 'assets/en_core_web_sm-3.8.0-py3-none-any.whl',
});
const model = await loadModel('en_core_web_sm');
assert.equal((await model.process('Alice visits London.')).tokens[0]?.text, 'Alice');
```

`downloadModel(name, {path?, version?, archive?})` resolves to the installed directory. Existing installations are verified and reused. `loadModel(name, {path?})` loads by name; an explicit directory such as `./models/export` remains supported. Bare names select installed models even if a same-named directory exists in the working directory. Missing models reject with a download hint. Async calls capture environment and relative paths when called, before work is queued. Downloads use Node's worker pool and share the Rust installer's checksum verification, archive limits, staging, and locking; cancellation and progress callbacks are not implemented.

## Process text

Save this example as `bindings/node/analyze.mts`, then run `node bindings/node/analyze.mts` from the repository root. `SPARS_MODEL` optionally selects another model name or an explicit export directory. Reference verification supplies the medium-model export through this variable. Verification executes this code directly from this guide.

```typescript
import assert from 'node:assert/strict';
import { configureExecution, loadModel } from './index.js';

configureExecution({ maxActive: 2, maxQueued: 32 });
const model = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');
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

const model = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');
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

Each batch runs sequentially in one worker job and retains input order. A failure rejects the entire batch. Concurrent calls share the immutable model and own their processing state. The package submits at most two inference jobs to Node's worker pool and holds up to 32 more in a first-in, first-out queue in JavaScript by default. Excess `process` and `processBatch` calls reject with `SPARS_BUSY` without submitting native work. Queued batches capture their input strings at submission. A batch occupies one slot until its complete result has been created; success and failure both release the slot. Queued inputs and complete batch results still occupy memory. Keep batches bounded for your workload. Input copying and result creation take place on the JavaScript thread; large batches can still delay it. This API does not stream results or support cancellation.

`configureExecution({maxActive, maxQueued})` sets both limits while inference is idle. `maxActive` must be a positive safe integer; `maxQueued` must be a nonnegative safe integer, and zero disables waiting. Invalid settings or changes while jobs are pending throw synchronously. Limits are shared by models loaded through this package instance within one JavaScript isolate (the separate JavaScript environment of the main thread or a Node worker), not across Node worker threads or processes. They cover inference only: load models and download assets during startup. The libuv pool is still shared with other Node operations; these limits do not reserve worker threads or guarantee latency. Calls rejected with `SPARS_BUSY` need application-level backpressure, such as rejecting an HTTP request or retrying later with a bounded policy. Import the package entry point; directly importing a native `.node` file bypasses admission control.

`vector(word)` is a synchronous, case-sensitive static-vector lookup. It returns a copied `Float32Array`, or `null` when the lexical key has no row. Modifying the array is safe. Document/span vectors, similarities, traversal helpers, and matchers are not yet exposed through this binding; their Rust APIs remain available separately.

## Errors and verification

Model and processing failures reject their promises with an `Error` containing a `code` and message. Codes are `SPARS_IO`, `SPARS_INVALID_MODEL`, `SPARS_UNSUPPORTED`, `SPARS_INVALID_TEXT`, `SPARS_BOUNDS`, `SPARS_INFERENCE`, and `SPARS_BUSY`. Invalid JavaScript argument types or unknown stages throw immediately; when the queue is full, `SPARS_BUSY` takes precedence over checking individual batch elements. `vector` also throws immediately for malformed text. Use `loadModel`; constructing `Model` directly is unsupported and its TypeScript constructor is private.

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

Use `long` for the two long documents, `batch` or `concurrent` instead of `single`, and `Tokenizer` instead of `Ner` to measure tokenization alone. The argument `96` limits each batch to 96 documents; a smaller corpus produces a smaller group. For `concurrent`, replace `96` with `8` to stay within the default admission limits. Output separates model loading, awaited processing, peak process memory, submission time, and maximum timer gap. Awaited processing includes scheduling and result conversion; it is not a measurement of Rust inference alone. Repeat runs without competing workloads and record hardware, build mode, thread limits, and corpus identity as described in [performance measurements](PERFORMANCE.md).
