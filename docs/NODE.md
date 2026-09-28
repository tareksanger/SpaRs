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

Installation requires no Python. See [model installation](MODEL_INSTALLATION.md) for local archives and checksums. The Node package includes the `spars` CLI and an asynchronous `downloadModel` API backed by the native Rust installer. No separate installer executable is needed. `loadModel("en_core_web_lg")` resolves the installation under `SPARS_MODEL_DIR` and never downloads assets during loading or inference. Build the addon locally for your operating system and CPU. The [npm release workflow](RELEASING.md#publish-the-node-package) packages Linux x64/ARM64 glibc and musl, macOS x64/ARM64, and Windows x64/ARM64 builds and tests the resulting tarballs before optional publication. The workflow alone does not establish registry availability. Node dependencies are confined to `bindings/node/` and do not enter ordinary Rust consumer builds.

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
import { loadModel } from './index.js';
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
assert.equal(model.maxLength, 1_000_000);
assert.equal(model.batchSize, 256);
model.maxLength = 100_000;
model.batchSize = 64;
const streamed: string[] = [];
for await (const doc of model.pipe(['😀 café', ''], { batchSize: 2, stage: 'Tokenizer' })) {
  streamed.push(doc.text);
}
assert.deepEqual(streamed, ['😀 café', '']);
const docs = await model.processBatch(streamed, 'Tokenizer');
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

`model.maxLength` defaults to 1,000,000 Unicode code points, following spaCy 3.8.14's `Language.max_length`. An emoji such as `😀` counts as one code point; `e` followed by a combining accent counts as two. Equality is allowed. The native worker checks the limit before tokenization; exceeding it rejects with `SPARS_TEXT_TOO_LONG` and an `[E088]` message. Set `model.maxLength` to a nonnegative unsigned 32-bit integer; zero permits only empty text. This Node setting does not change Rust's direct `Model::process` API. Invalid UTF-16 remains unsupported.

`model.batchSize` reads the exported `[nlp] batch_size`: 256 for the supported official sm/md/lg releases. When that setting is absent, it uses spaCy's `Language` constructor default of 1,000. Set a positive unsigned 32-bit integer to override it per model. This is a processing chunk size, not a maximum number of input documents. Both model settings throw for invalid values or changes while any inference is pending in this package instance and JavaScript isolate. This stricter validation and idle-only mutation are Node contracts; Python permits direct attribute assignment.

`processBatch` snapshots the entire input array, processes consecutive chunks of at most `model.batchSize`, and resolves to all documents in input order. Each chunk uses a background task that performs sequential document inference. A failure rejects the whole call. The call retains a single admission slot across chunks, including result creation, and returns that slot on success or failure. All input strings and completed results remain in memory until the call finishes. Concurrent calls share immutable model weights and own their processing state.

`model.pipe(texts, {batchSize?, stage?})` returns an async generator over a synchronous or asynchronous iterable of strings. No input is consumed until iteration starts. It resolves the buffering size from its option or `model.batchSize` on the first iteration, processes one buffer, yields documents in order, then reads the next buffer. The final buffer may be smaller. Breaking iteration closes the source; a source or processing failure rejects iteration and stops it. Earlier buffers may already have been yielded. The initial buffering size stays fixed for that iterator; changes to model settings while idle apply when the next buffer is submitted. Avoid retaining yielded documents if bounded memory is required.

These buffers are not spaCy's component-level neural batching, and there is no `n_process` multiprocessing, `as_tuples`, per-component pipe configuration, or cancellation support. `pipe` accepts only `batchSize` and `stage` options; unknown options reject during iteration. Buffer failure timing need not match spaCy for every pipeline, especially tokenizer-only processing. Both input copying and result creation run on the JavaScript thread, so large documents or buffers can still delay the event loop. The default maximum length follows spaCy; it does not promise that all accepted text fits in memory or avoids native regex/inference limits.

## Optional server resource limits

Additional queue and input limits are disabled by default. Applications relying on earlier default limits must now configure both policies explicitly at startup. Services can opt in during startup, before submitting inference. Choose values from measurements of their workload; these example values are application policy, not spaCy defaults:

```typescript
import assert from 'node:assert/strict';
import { configureExecution, configureInputLimits, loadModel } from './index.js';

const model = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');
configureExecution({ maxActive: 2, maxQueued: 32 });
configureInputLimits({ maxTextLength: 32_768, maxBatchSize: 128, maxBatchTextLength: 65_536 });
// Keep pipe buffers within the explicit maxBatchSize policy.
model.batchSize = 128;
assert.equal((await model.process('Hello.')).text, 'Hello.');
// Demonstration only: omit these resets in a server that should retain its limits.
configureExecution(null);
configureInputLimits(null);
```

Import `configureExecution` and `configureInputLimits` from the package entry point. `configureExecution({maxActive, maxQueued})` sets a maximum number of submitted inference jobs and a first-in, first-out waiting queue. `maxActive` must be a positive safe integer; `maxQueued` must be a nonnegative safe integer, and zero disables waiting. Excess calls reject with `SPARS_BUSY` without submitting native work. Calls rejected with `SPARS_BUSY` need application backpressure, such as rejecting an HTTP request or retrying later with a bounded policy. `configureExecution(null)` removes these additional admission limits while idle; it does not cancel work or change the native pool's thread count.

`configureInputLimits({maxTextLength, maxBatchSize, maxBatchTextLength})` sets all three additional input limits. All must be positive safe integers. Text lengths count JavaScript UTF-16 units (`string.length`), so `😀` counts as two units for these server policies. `maxBatchSize` limits the whole array passed to `processBatch` or one `pipe` buffer, before any internal chunking. Empty text and empty batches remain supported. `configureInputLimits(null)` removes these extra limits while idle; the per-model `maxLength` still applies.

Both configuration functions copy their settings and throw synchronously for invalid values or changes while inference is pending. Policies are shared by models loaded through this package instance within one JavaScript isolate (the separate JavaScript environment of the main thread or a Node worker), not across Node workers or processes. They cover inference only: load models and download assets during startup. Node's libuv worker pool is shared with other Node operations; these policies do not reserve threads or guarantee latency. Import the package entry point; importing the native `.node` file directly bypasses the JavaScript policies and streaming wrapper.

Oversized inputs reject with `SPARS_INPUT_LIMIT` before copying into Rust or submitting inference. Batch count is checked before accessing elements; text and aggregate length are checked while capturing the batch, and the first violation rejects the entire call. A `pipe` buffer is checked when submitted. Size rejection does not consume an inference slot. A full queue can return `SPARS_BUSY` before inspecting batch elements. These policies do not limit model loading, downloads, `vector` lookups, other isolates, or memory already allocated by the caller. Large allowed results still require synchronous JavaScript object creation. Increasing limits increases potential CPU, memory, and event-loop cost.

`vector(word)` is a synchronous, case-sensitive static-vector lookup. It returns a copied `Float32Array`, or `null` when the lexical key has no row. Modifying the array is safe. Document/span vectors, similarities, traversal helpers, and matchers are not yet exposed through this binding; their Rust APIs remain available separately.

## Errors and verification

Model and processing failures reject their promises with an `Error` containing a `code` and message. Codes are `SPARS_IO`, `SPARS_INVALID_MODEL`, `SPARS_UNSUPPORTED`, `SPARS_INVALID_TEXT`, `SPARS_BOUNDS`, `SPARS_INFERENCE`, `SPARS_BUSY`, `SPARS_INPUT_LIMIT`, and `SPARS_TEXT_TOO_LONG`. For `pipe`, errors reject iteration; `process` and `processBatch` throw immediately for invalid JavaScript argument types or unknown stages; when the queue is full, `SPARS_BUSY` takes precedence over checking individual batch elements. `vector` also throws immediately for malformed text. Use `loadModel`; constructing `Model` directly is unsupported and its TypeScript constructor is private.

After the [reference setup](DEVELOPMENT.md#set-up-the-project), run:

```sh
npm --prefix bindings/node run build
npm --prefix bindings/node run typecheck
npm --prefix bindings/node test
.venv/bin/python tools/check_docs.py
```

Tests execute against the required model assets; missing assets fail. The binding compares all discrete outputs on the frozen 190-document / 7,029-token suite, checks Unicode offsets, eight static-vector lookup cases, stage availability, error paths, mutation isolation, batches, concurrent jobs, and model lifetime during garbage collection. `tests/processing-parity.test.mts` covers model defaults, code-point length boundaries, opt-in policy reset, lazy iteration, ordering, source cleanup, and failure propagation. Type checking checks generated declarations and distinct offset types. The full [acceptance command](QUALITY.md#run-the-acceptance-checks) includes these checks. Generated mismatch reports belong under `target/reports/` and CI retains them as artifacts.

Processing defaults follow the pinned [spaCy 3.8.14 Language implementation](https://github.com/explosion/spaCy/blob/v3.8.14/spacy/language.py) and the official model configs preserved in exported manifests. The implementation follows [napi-rs worker tasks](https://napi.rs/docs/concepts/async-task) and [object conversion](https://napi.rs/docs/concepts/object). Rust and npm lockfiles pin the binding dependencies. The build corrects napi-rs's missing private-constructor declaration and fails if that generated class changes unexpectedly.

For repeatable local measurements, this command processes the short-document portion of the frozen evaluation corpus with one warmup pass and three measured passes:

```sh
UV_THREADPOOL_SIZE=1 node bindings/node/scripts/measure.mts assets/en_core_web_md-3.8.0 fixtures/evaluation-v1.json short single 3 1 96 Ner
```

Use `long` for the two long documents, `batch` or `concurrent` instead of `single`, and `Tokenizer` instead of `Ner` to measure tokenization alone. The argument `96` limits each batch to 96 documents; a smaller corpus produces a smaller group. For `concurrent`, replace `96` with `8` to measure groups of eight concurrent calls; enable explicit admission limits when measuring a server policy. Output separates model loading, awaited processing, peak process memory, submission time, and maximum timer gap. Awaited processing includes scheduling and result conversion; it is not a measurement of Rust inference alone. Repeat runs without competing workloads and record hardware, build mode, thread limits, and corpus identity as described in [performance measurements](PERFORMANCE.md).
