# Use SpaRs from Node.js

The separate `spars-node` crate exposes native Rust inference to Node.js through Node-API. Load a model once, then process text or batches with a typed TypeScript API. The binding runs model loading and inference on Node's background worker threads. It uses the same Rust library and official model assets as Rust applications.

## Build and install a model

Use Node.js 22 or newer, npm, and Rust 1.88 or newer. Run these commands from the repository root:

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
import { downloadModel, loadModel } from '@spars/node';

// Install the small English model into the model folder. A model is the set of files
// SpaRs needs to understand a language. The `archive` option installs it from a file
// already on disk, so this step needs no internet connection.
await downloadModel('en_core_web_sm', {
  archive: 'assets/en_core_web_sm-3.8.0-py3-none-any.whl',
});

// Load the installed model by its name. Loading reads files only; it never downloads.
const nlp = await loadModel('en_core_web_sm');

// Process a sentence and check its first token. A token is one piece of the text,
// such as a word or a punctuation mark. Here the first token is the word "Alice".
assert.equal((await nlp.process('Alice visits London.')).tokens[0]?.text, 'Alice');
```

`downloadModel(name, {path?, version?, archive?})` resolves to the installed directory. Existing installations are verified and reused. `loadModel(name, {path?})` loads by name; an explicit directory such as `./models/export` remains supported. Bare names select installed models even if a same-named directory exists in the working directory. Missing models reject with a download hint. Async calls capture environment and relative paths when called, before work is queued. Downloads use Node's worker pool and share the Rust installer's checksum verification, archive limits, staging, and locking; cancellation and progress callbacks are not implemented.

## Process text

Save this example as `bindings/node/analyze.mts`, then run `node bindings/node/analyze.mts` from the repository root. `SPARS_MODEL` optionally selects another model name or an explicit export directory. Reference verification supplies the medium-model export through this variable. Verification executes this code directly from this guide.

```typescript
import assert from 'node:assert/strict';
import { loadModel } from '@spars/node';

// Load the model once. It uses the name in the SPARS_MODEL environment variable,
// or the large English model when that variable is not set.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// Process a sentence. The result holds the text split into tokens (words and
// punctuation marks) plus what the model worked out about them.
const doc = await nlp.process('Alice works in London.');

// The tokens are: Alice(0) works(1) in(2) London(3) .(4)
assert.equal(doc.tokens[0]?.text, 'Alice');

// An entity is a real-world thing named in the text, such as a person or a place.
// Check that the model produced a list of them before reading it.
assert.ok(doc.entities);

// Keep only the entities labeled GPE, the label for countries, cities and similar
// places, and turn each one back into text.
const places = doc.entities.filter(entity => entity.label === 'GPE').map(entity => {
  // An entity says which tokens it covers. `end` is exclusive, so the last token
  // of the entity is at `end - 1`.
  const first = doc.tokens[entity.start];
  const last = doc.tokens[entity.end - 1];
  assert.ok(first && last);

  // Cut the entity out of the original text. UTF-16 offsets are positions counted
  // the way JavaScript strings count them, so they work directly with `slice`.
  return doc.text.slice(first.utf16Start, last.utf16End);
});

// The only place named in the sentence is London.
assert.deepEqual(places, ['London']);
console.log(places);
```

This prints `[ 'London' ]`. `GPE` labels a country, city, or similar political area. Entity, sentence, and noun-chunk spans use token indices: `start` is included and `end` is excluded. Tokens include text, trailing whitespace, normalization, grammatical tags, part-of-speech labels, morphology, base forms (lemmas), dependency heads and labels, sentence starts, and entity annotations. Returned objects own their data; editing them cannot change the model or later results.

Use `utf16Start` and `utf16End` with JavaScript's `slice`. `byteStart` and `byteEnd` count UTF-8 bytes; `codePointStart` and `codePointEnd` count Unicode code points. TypeScript treats these as distinct types to prevent accidental interchange. Offsets and token indices are checked unsigned 32-bit values. Text containing an unpaired UTF-16 surrogate is rejected because it cannot be preserved as valid Rust text.

## Types and autocomplete

The TypeScript declarations type every fixed vocabulary as a set of literal values, so editors offer them as completions and the compiler rejects typos. These include part-of-speech tags (`UniversalPos`), entity IOB tags (`EntityIob`: `B` begins an entity, `I` continues it, `O` is outside any entity), `PhraseMatcher` attributes (`PhraseAttribute`), token-condition attributes, predicate kinds and comparison operators (`TokenConstraint`), repetition kinds (`TokenRepetition`), DependencyMatcher relation operators such as `>` and `<<` (`DependencyRelation`), official model names (`OfficialModelName`) and error codes (`SparsErrorCode`). `TokenConstraint` also ties each attribute to the predicates it accepts: a lexical flag takes only `{ kind: 'flag', value }`, and `length` takes only numeric predicates. Token fields that are unavailable are `null`, while an empty string `''` is spaCy's unset or empty value; for example, tokens outside any entity have `entityType: ''`. Entity spans are `Span<EntityLabel>`, sentences `Span<'', ''>` and noun chunks `Span<'NP', ''>`; the second parameter types the span's `id`, which entities and span group members can carry. Span group members are `Span<SpanLabel | ''>`.

Entity labels (`EntityLabel`), fine-grained tags (`FineGrainedTag`), dependency labels (`DependencyLabel`), span labels (`SpanLabel`), span group keys (`SpanGroupKey`) and model names (`ModelName`) are open sets: they offer the labels of the supported official models for autocomplete, but also accept any other string, because custom models and edited documents can use other labels. Span labels offer the entity labels, and span group keys offer spaCy's default keys `ruler` (`span_ruler`) and `sc` (`spancat`). Register your own labels by extending the matching interface (`EntityLabels`, `FineGrainedTags`, `DependencyLabels`, `SpanLabels`, `SpanGroupKeys` or `ModelNames`); `SpanLabels` extends `EntityLabels`, so a registered entity label is also offered as a span label; TypeScript's declaration merging adds the members of a redeclared interface to the original. In your own project, write the declaration in a file that your `tsconfig` includes and that has at least one `import` or `export`, and name the package: `declare module '@spars/node' { interface EntityLabels { PRODUCT_CODE: true } }`. Without an `import` or `export`, TypeScript treats the block as a replacement for the package's types rather than an extension. To accept only registered values in a set, declare `interface StrictLabelSets { entity: true }` the same way, using `entity`, `tag`, `dependency` or `span`; strict `tag` and `dependency` also narrow the values allowed in token conditions, and strict `entity` does not narrow span labels. Model names and span group keys always accept any string, because `loadModel` also takes directory paths and any key can name a group. Strict mode changes only compile-time types: `entityType`, span labels, `tag` and `dep` still hold whatever the loaded model or snapshot contains, so enable it only when your registered labels cover every model you load.

```typescript
import assert from 'node:assert/strict';
import { isSparsError, NativeDocument, PhraseMatcher } from '@spars/node';
import type { EntityLabel, TokenConstraint } from '@spars/node';

// Register a custom entity label. An entity label names the kind of thing a piece
// of text refers to, such as PERSON. Adding PRODUCT_CODE to this interface makes
// the editor offer it as a completion alongside the labels of the official models.
declare module '@spars/node' {
  interface EntityLabels { PRODUCT_CODE: true }
}

// The registered label is accepted as an EntityLabel.
const label: EntityLabel = 'PRODUCT_CODE';

// A token condition: "this token looks like a number". `like_num` is a lexical
// flag, a yes-or-no fact about a word, so the types allow only a `flag` predicate.
const number: TokenConstraint = { attribute: 'like_num', predicate: { kind: 'flag', value: true } };

// 'LOWER' is one of the attributes a PhraseMatcher can compare, so this compiles
// and the matcher reports the attribute it was given.
assert.equal(new PhraseMatcher('LOWER').attribute, 'LOWER');

// A misspelled attribute is a compile error. The line below tells TypeScript to
// expect that error, and the assertion shows the mistake also fails when run.
// @ts-expect-error 'LOWERR' is not a PhraseMatcher attribute
assert.throws(() => new PhraseMatcher('LOWERR'));

// Cause an error on purpose: '{}' is not a valid saved document.
try {
  NativeDocument.fromSnapshot('{}');
  assert.fail('expected an error');
} catch (error) {
  // A caught value could be anything. `isSparsError` confirms it is a SpaRs error,
  // so TypeScript knows `error.code` is one of the SPARS_* codes.
  assert.ok(isSparsError(error));
  console.log(label, number.attribute, error.code);
}
```

This prints `PRODUCT_CODE like_num SPARS_INVALID_MODEL`. `isSparsError(error)` narrows a caught value to `SparsError`, whose `code` is a `SparsErrorCode`, and `isOfficialModelName(name)` narrows a string to `OfficialModelName` before calling `downloadModel`. A POS condition such as `{ attribute: 'pos', predicate: { kind: 'equals', value: 'noun' } }` is a compile error: spaCy and SpaRs accept it at runtime, but POS values are compared exactly and every POS tag is uppercase (`NOUN`), so `'noun'` never equals a token's POS. In `in` it matches nothing, and in `not_in` it excludes nothing. These types add no runtime checks: JavaScript callers, and values cast past the types, still receive the same native errors. A matcher stores what it was given, so `get()` returns such a value unchanged even though its declared return type excludes it.

## Batches, stages, and vectors

This example can also be saved in `bindings/node` and run from the repository root:

```typescript
import assert from 'node:assert/strict';
import { loadModel } from '@spars/node';

// Load the model once and reuse it for every text.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// Each model starts with two settings. `maxLength` is the longest text it accepts,
// counted in characters (Unicode code points). `batchSize` is how many texts are
// handed to the background worker in one group; such a group is called a batch.
assert.equal(nlp.maxLength, 1_000_000);
assert.equal(nlp.batchSize, 256);

// Both settings can be changed while the model is not processing anything.
nlp.maxLength = 100_000;
nlp.batchSize = 64;

// `pipe` reads texts a few at a time and hands back each result as it is ready.
// A stage is one step of the model's pipeline. Stopping at 'Tokenizer' only splits
// the text into tokens (words and punctuation marks) and skips the later steps.
const streamed: string[] = [];
for await (const doc of nlp.pipe(['😀 café', ''], { batchSize: 2, stage: 'Tokenizer' })) {
  streamed.push(doc.text);
}

// The results arrive in the same order as the inputs. Empty text is allowed.
assert.deepEqual(streamed, ['😀 café', '']);

// `processBatch` takes a whole list of texts and returns all the results together.
const docs = await nlp.processBatch(streamed, 'Tokenizer');

// The first token is the emoji. Its end position depends on the unit used to count:
// JavaScript strings store this emoji as 2 UTF-16 units, and UTF-8 stores it as 4 bytes.
assert.equal(docs[0]?.tokens[0]?.utf16End, 2);
assert.equal(docs[0]?.tokens[0]?.byteEnd, 4);

// Entities are the named things in a text, such as people and places. The tokenizer
// stage does not look for them, so the list is `null`, meaning "not computed".
assert.equal(docs[0]?.entities, null);

// The full pipeline does look for entities. Empty text has none, so the list is
// empty: computed, with nothing found.
assert.deepEqual((await nlp.process('')).entities, []);

// A vector is a list of numbers that stands for a word's meaning. Words with
// similar meanings have similar numbers.
const vector = nlp.vector('dog');
assert.ok(vector instanceof Float32Array);

// The medium and large English models store 300 numbers for each word.
assert.equal(vector.length, 300);

// A word the model has no stored vector for gives `null`.
assert.equal(nlp.vector('spars_unknown_🙂_lexeme'), null);
```

`process(text, stage?)` and `processBatch(texts, stage?)` return promises. The default runs the full pipeline. A stage selects an ordered prefix: `Tokenizer`, `Tagger`, `Parser`, `AttributeRuler`, `Lemmatizer`, or `Ner`. Later annotations remain `null`; computed empty lists remain `[]`. The early-stage unavailable-value contract follows SpaRs and does not reproduce every default value in a partially processed Python Doc.

`nlp.maxLength` defaults to 1,000,000 Unicode code points, following spaCy 3.8.14's `Language.max_length`. An emoji such as `😀` counts as one code point; `e` followed by a combining accent counts as two. Equality is allowed. The native worker checks the limit before tokenization; exceeding it rejects with `SPARS_TEXT_TOO_LONG` and an `[E088]` message. Set `nlp.maxLength` to a nonnegative unsigned 32-bit integer; zero permits only empty text. This Node setting does not change Rust's direct `Model::process` API. Invalid UTF-16 remains unsupported.

`nlp.batchSize` reads the exported `[nlp] batch_size`: 256 for the supported official sm/md/lg releases. When that setting is absent, it uses spaCy's `Language` constructor default of 1,000. Set a positive unsigned 32-bit integer to override it per model. This is a processing chunk size, not a maximum number of input documents. Both model settings throw for invalid values or changes while any inference is pending in this package instance and JavaScript isolate. This stricter validation and idle-only mutation are Node contracts; Python permits direct attribute assignment.

`processBatch` snapshots the entire input array, processes consecutive chunks of at most `nlp.batchSize`, and resolves to all documents in input order. Each chunk uses a background task that performs sequential document inference. A failure rejects the whole call. The call retains a single admission slot across chunks, including result creation, and returns that slot on success or failure. All input strings and completed results remain in memory until the call finishes. Concurrent calls share immutable model weights and own their processing state.

`nlp.pipe(texts, {batchSize?, stage?})` returns an async generator over a synchronous or asynchronous iterable of strings. No input is consumed until iteration starts. It resolves the buffering size from its option or `nlp.batchSize` on the first iteration, processes one buffer, yields documents in order, then reads the next buffer. The final buffer may be smaller. Breaking iteration closes the source; a source or processing failure rejects iteration and stops it. Earlier buffers may already have been yielded. The initial buffering size stays fixed for that iterator; changes to model settings while idle apply when the next buffer is submitted. Avoid retaining yielded documents if bounded memory is required.

These buffers are not spaCy's component-level neural batching, and there is no `n_process` multiprocessing, `as_tuples`, per-component pipe configuration, or cancellation support. `pipe` accepts only `batchSize` and `stage` options; unknown options reject during iteration. Buffer failure timing need not match spaCy for every pipeline, especially tokenizer-only processing. Both input copying and result creation run on the JavaScript thread, so large documents or buffers can still delay the event loop. The default maximum length follows spaCy; it does not promise that all accepted text fits in memory or avoids native regex/inference limits.

## Upgrade from npm 0.2.0

The published `@spars/node@0.2.0` package has no package-level admission queue, configurable input caps, `maxLength`, or `pipe` wrapper. Its npm tarball is the upgrade baseline; the later source-tree defaults are not the defaults that npm consumers received. The published package can be inspected without running lifecycle scripts with `npm pack @spars/node@0.2.0 --ignore-scripts`. The changes below apply to an upgrade that includes the processing API described in this guide.

| Affected usage | Change and observable impact | Migration |
| --- | --- | --- |
| More than 34 unfinished inference calls across models in one package instance and JavaScript isolate | Two calls can be active and 32 wait; further calls reject with `SPARS_BUSY`. A synchronous burst of 35 calls previously reached the native pool without package-level rejection. | Bound submissions, consume `pipe` incrementally, or use `processBatch` when retaining all results is acceptable. Configure larger finite `maxActive`/`maxQueued` values during startup only when the service has capacity. |
| A service relying on more than two concurrent native inference tasks | Admission now limits active jobs to two, regardless of a larger libuv pool. Requests can wait longer and application deadlines may expire. | Measure the workload and tune `configureExecution`, keeping application backpressure. Increasing `UV_THREADPOOL_SIZE` alone does not change package admission. |
| Any document longer than 1,000,000 Unicode code points | The worker rejects with the new `SPARS_TEXT_TOO_LONG` code before tokenization, including within `processBatch`. The released package had no separate document-length policy. | Set `nlp.maxLength` to an intentional larger bound on each loaded model while inference is idle, or split input when losing cross-boundary NLP context is acceptable. Handle the new error; raising the limit does not guarantee enough memory or successful inference. |
| JavaScript callers matching native argument-error codes or messages | Wrapper validation of invalid `process`/`processBatch` arguments and unknown stages throws `TypeError`. Examples that previously reported native `InvalidArg` or `StringExpected` no longer carry those codes; a full queue may reject a batch with `SPARS_BUSY` before checking its elements. | Validate inputs and handle `TypeError` for argument mistakes. Catch documented `SPARS_*` processing errors separately; do not rely on native conversion messages. |
| Hand-edited or third-party model manifests with previously ignored `config` data | The loader now reads `[nlp] batch_size`; a null/non-string config, malformed or duplicate processing fields, or interpolation can reject loading with `SPARS_INVALID_MODEL`. This loader change also affects direct Rust consumers. | Use an unmodified official export or regenerate compatible model assets. For deliberately maintained manifests, supply the actual literal positive integer batch size in the preserved config; see [model format](MODEL_FORMAT.md). Normal supported official exports need no re-export. |
| Deployments with `NAPI_RS_NATIVE_LIBRARY_PATH` pointing to an older addon, or manually mixed wrapper and native versions | Package import throws `SPARS_NATIVE_INCOMPATIBLE` when required native settings accessors are absent. This check prevents silently returning empty batches with an older addon. | Remove the override to use the matching packaged addon, or update it to a compatible build. Reinstall the root and platform packages together; do not combine an old binary with a new wrapper. |

A rejected `Promise.all` does not cancel calls already admitted. Do not retry the entire original collection blindly: account for completed and still-running work. The limit is shared across models, so creating another model in the same isolate does not create a separate queue. `configureExecution(null)` restores two active jobs and 32 queued; it does not restore the published package's unbounded submission behavior.

For a collection that previously used `Promise.all(texts.map(text => nlp.process(text)))`, this example submits one buffer at a time and verifies that all 100 documents are consumed. It assumes no competing producers saturate the shared queue. Configure settings during startup before starting other inference:

```typescript
import assert from 'node:assert/strict';
import { loadModel } from '@spars/node';

// Load the model once, during startup.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// `maxLength` is the longest text the model accepts, counted in characters
// (Unicode code points). The default is 1,000,000.
// Only raise this if the application intentionally accepts larger documents.
nlp.maxLength = 2_000_000;

// Build a list of 100 texts to process.
const texts = Array.from({ length: 100 }, () => 'Alice visits London.');

// `pipe` sends the texts to the model one buffer at a time: here 64 texts, then
// the remaining 36. Only one buffer waits for the model at once, so the 100 texts
// do not all ask for a turn at the same moment.
let completed = 0;
for await (const doc of nlp.pipe(texts, { batchSize: 64 })) {
  // Each result comes back in input order and keeps its original text.
  assert.equal(doc.text, 'Alice visits London.');
  completed++;
}

// Every one of the 100 texts was processed.
assert.equal(completed, texts.length);
```

Valid `process`, `processBatch`, loading, vector lookup, result shapes, and within-batch ordering retain their APIs. Small sequential documents need no call-site rewrite. Batch chunking changes scheduling and result-conversion timing, not the ordered result contract: `processBatch` still collects all results and rejects the whole call on failure. `pipe` is optional; choosing it introduces incremental delivery, so earlier buffers may already have been consumed when a later error occurs.

The former 32,768-unit text, 128-document batch, and 65,536-unit aggregate caps existed in later unreleased source builds, not npm 0.2.0. Users of those builds who relied on the caps should enable `configureInputLimits` as shown below. Existing explicit input policies retain their UTF-16 units and values; a policy allowing text over 1,000,000 code points now also needs an appropriate per-model `maxLength`.

## Upgrade TypeScript code to the literal types

The declarations now use literal types for fixed vocabularies, as described in [types and autocomplete](#types-and-autocomplete). JavaScript behavior is unchanged except for snapshot validation, but TypeScript code that passed plain strings can stop compiling:

| Affected usage | Change and observable impact | Migration |
| --- | --- | --- |
| Passing a `string` variable to `downloadModel`, `new PhraseMatcher(...)`, a token condition's `attribute`, `kind` or `operator`, a repetition `kind` or a dependency `relation` | These parameters take literal types, so a value typed as `string` is a compile error. | Narrow external input first: use `isOfficialModelName` for model names, or check the value against your own list of the literal values. |
| Code importing the generated `TokenPredicate` interface or building conditions as `{ attribute: string; predicate: TokenPredicate }` | `TokenPredicate` is removed; `TokenConstraint` is a union whose predicate shape depends on the attribute. | Write conditions as `TokenConstraint` values, or use `Extract<TokenConstraint, { attribute: 'length' }>['predicate']` for one attribute's predicates. |
| Fields typed as `string` in your own code, such as `token.pos` or `span.label` | Output fields have literal or open label types. Assigning them to `string` still works, but comparing a closed field with an impossible value, such as `token.pos === 'noun'`, is a compile error. | Compare with the declared values; register custom labels as described above. |
| Hand-edited or third-party snapshots with an IOB tag outside `B`, `I`, `O` and empty, an `I` tag with no entity to continue, entities that differ from those the token IOB tags describe, a POS tag outside the universal tags, an unlabeled entity, a labeled sentence, or a noun chunk not labeled `NP` | `NativeDocument.fromSnapshot` and Rust `Doc::from_json` now reject them with `SPARS_INVALID_MODEL` (`Error::Model` in Rust). Snapshots this version writes from model output or entity updates are unaffected. | Correct the values, or regenerate the snapshot by processing the text again. |
| Code comparing whole token or span objects, or building `Span` objects | Every token now has `entityId` and every span has `id`, `''` when absent; sentences are `Span<'', ''>` and noun chunks `Span<'NP', ''>`. | Include the new fields in expected objects, or compare the fields you need. |

## Server resource limits

Inference admission defaults to two active jobs and 32 queued jobs per package instance and JavaScript isolate. Additional input limits are disabled by default. Users of unreleased source builds relying on earlier text and batch caps must configure `configureInputLimits` explicitly at startup. Published npm 0.2.0 consumers should follow the upgrade guidance above for the new queue protection. Services can customize either policy while inference is idle. These are Node server policies, not spaCy processing defaults:

```typescript
import assert from 'node:assert/strict';
import { configureExecution, configureInputLimits, loadModel } from '@spars/node';

// Load the model during startup, before any text is processed.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// Limit how much work runs at once. Processing happens on worker threads, which are
// background helpers that keep the main program responsive. At most 2 jobs run at
// a time, and up to 32 more wait in a queue (a line). Any call beyond that is
// rejected with the SPARS_BUSY error instead of piling up.
configureExecution({ maxActive: 2, maxQueued: 32 });

// Limit how big the input may be. Lengths are counted in UTF-16 units, the same
// count as JavaScript's `string.length`. One text may have at most 32,768 units,
// one batch (a list of texts sent together) at most 128 texts, and all the texts
// of one batch together at most 65,536 units.
configureInputLimits({ maxTextLength: 32_768, maxBatchSize: 128, maxBatchTextLength: 65_536 });

// Keep pipe buffers within the explicit maxBatchSize policy.
nlp.batchSize = 128;

// A short text fits inside every limit, so it is processed as usual.
assert.equal((await nlp.process('Hello.')).text, 'Hello.');

// Demonstration only: restore default admission and remove additional input caps.
// Omit these resets in a server that should retain its custom policies.
configureExecution(null);
configureInputLimits(null);
```

Import `configureExecution` and `configureInputLimits` from the package entry point. `configureExecution({maxActive, maxQueued})` sets a maximum number of submitted inference jobs and a first-in, first-out waiting queue. `maxActive` must be a positive safe integer; `maxQueued` must be a nonnegative safe integer, and zero disables waiting. Excess calls reject with `SPARS_BUSY` without submitting native work. Calls rejected with `SPARS_BUSY` need application backpressure, such as rejecting an HTTP request or retrying later with a bounded policy. `configureExecution(null)` restores two active jobs and 32 queued jobs while idle; it does not cancel work or change the native pool's thread count.

`configureInputLimits({maxTextLength, maxBatchSize, maxBatchTextLength})` sets all three additional input limits. All must be positive safe integers. Text lengths count JavaScript UTF-16 units (`string.length`), so `😀` counts as two units for these server policies. `maxBatchSize` limits the whole array passed to `processBatch` or one `pipe` buffer, before any internal chunking. Empty text and empty batches remain supported. `configureInputLimits(null)` removes these extra limits while idle; the per-model `maxLength` still applies.

Both configuration functions copy their settings and throw synchronously for invalid values or changes while inference is pending. Policies are shared by models loaded through this package instance within one JavaScript isolate (the separate JavaScript environment of the main thread or a Node worker), not across Node workers or processes. They cover inference and matcher searches; model loading and downloads remain outside these policies. Load models and download assets during startup. Node's libuv worker pool is shared with other Node operations; these policies do not reserve threads or guarantee latency. Import the package entry point; importing the native `.node` file directly bypasses the JavaScript policies and streaming wrapper.

Oversized inputs reject with `SPARS_INPUT_LIMIT` before copying into Rust or submitting inference. Batch count is checked before accessing elements; text and aggregate length are checked while capturing the batch, and the first violation rejects the entire call. A `pipe` buffer is checked when submitted. Size rejection does not consume an inference slot. A full queue can return `SPARS_BUSY` before inspecting batch elements. These policies do not limit model loading, downloads, `vector` lookups, other isolates, or memory already allocated by the caller. Large allowed results still require synchronous JavaScript object creation. Increasing limits increases potential CPU, memory, and event-loop cost.

`vector(word)` is a synchronous, case-sensitive static-vector lookup. It returns a copied `Float32Array`, or `null` when the lexical key has no row. Modifying the array is safe. Native documents support the additional vector, traversal, and matcher APIs below.

## Handle processing errors

Processing failures reach a caller in two ways. Invalid configuration values and invalid JavaScript arguments throw synchronously when the function is called, before any promise exists: `configureExecution` and `configureInputLimits` throw `RangeError` or `TypeError` for invalid values, and a plain `Error` without a `code` when called while inference is pending, so configure them during startup. `process` and `processBatch` throw `TypeError` for non-string text or an unknown stage when the call is admitted (see below). Admission and processing failures reject the returned promise with an `Error` whose `code` is one of the `SPARS_*` values listed under [errors and verification](#errors-and-verification). The example below does not depend on a web framework and exercises each path deterministically:

```typescript
import assert from 'node:assert/strict';
import { configureExecution, configureInputLimits, loadModel } from '@spars/node';
import type { Document, InputLimits, Model } from '@spars/node';

// Read the `code` of an error, such as 'SPARS_BUSY'. It returns `undefined` when
// the value is not an error or has no text code.
// Caught values are unknown
function errorCode(error: unknown): string | undefined {
  return error instanceof Error && 'code' in error && typeof error.code === 'string' ? error.code : undefined;
}

// Process one text, trying again a few times when the package is too busy.
async function processWithRetry(nlp: Model, text: string, attempts = 3): Promise<Document> {
  for (let attempt = 1; ; attempt++) {
    try {
      return await nlp.process(text);
    } catch (error) {
      // SPARS_BUSY means the text was fine but there was no free place to run it.
      // Any other error, or running out of attempts, is passed on to the caller.
      if (errorCode(error) !== 'SPARS_BUSY' || attempt >= attempts) throw error;

      // Wait before the next attempt, a little longer each time: 50 ms, then 100 ms.
      await new Promise(resolve => setTimeout(resolve, 50 * attempt));
    }
  }
}

// Split a list of separate texts into batches (groups sent together) that each fit
// the input limits. It never cuts a single text into pieces.
function planBatches(texts: readonly string[], limits: InputLimits): string[][] {
  const batches: string[][] = [];
  let current: string[] = [];
  let length = 0;
  for (const text of texts) {
    // A text that is too long by itself cannot fit in any batch, so report it.
    if (text.length > Math.min(limits.maxTextLength, limits.maxBatchTextLength)) throw new RangeError('Document exceeds the input policy');

    // Close the current batch when it is full, or when adding this text would make
    // its total length too large. Then start an empty one.
    if (current.length > 0 && (current.length === limits.maxBatchSize || length + text.length > limits.maxBatchTextLength)) {
      batches.push(current);
      current = [];
      length = 0;
    }
    current.push(text);
    length += text.length;
  }

  // Keep the last batch if it holds any texts.
  if (current.length > 0) batches.push(current);
  return batches;
}

// Load the model once, during startup.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// Error case 1: a wrong setting or a wrong argument throws straight away, on the
// line that made the call. Zero active jobs is not allowed, and 42 is not a string.
assert.throws(() => configureExecution({ maxActive: 0, maxQueued: 0 }), RangeError);
assert.throws(() => nlp.processBatch(['ok', 42 as unknown as string]), TypeError);

// Error case 2: too much work at once. Allow one running job and no queue (the
// line where extra jobs wait), then start a job without waiting for it to finish.
configureExecution({ maxActive: 1, maxQueued: 0 });
const running = nlp.process('Alice visits London.', 'Tokenizer');

// While that job holds the only place, another call is rejected with SPARS_BUSY.
await assert.rejects(nlp.process('Overflow.'), { code: 'SPARS_BUSY' });

// The busy check comes before the arguments are looked at, so the same bad batch
// that threw a TypeError above is rejected with SPARS_BUSY here.
await assert.rejects(nlp.processBatch(['ok', 42 as unknown as string]), { code: 'SPARS_BUSY' });

// Let the first job finish, then restore the default limits: 2 running, 32 waiting.
await running;
configureExecution(null);

// With a free place again, the same kind of call succeeds.
assert.equal((await processWithRetry(nlp, 'Recovered.')).text, 'Recovered.');

// Error case 3: input that is too big. Set small limits: 20 UTF-16 units (the
// count `string.length` gives) per text, 2 texts per batch, 30 units per batch.
// Oversize rejection the big batch
const limits: InputLimits = { maxTextLength: 20, maxBatchSize: 2, maxBatchTextLength: 30 };
configureInputLimits(limits);

// One text of 21 units is over the limit, so the whole batch is rejected.
await assert.rejects(nlp.processBatch(['Short.', 'x'.repeat(21)]), { code: 'SPARS_INPUT_LIMIT' });

// The planning helper refuses the same oversized text before anything is sent.
assert.throws(() => planBatches(['x'.repeat(21)], limits), RangeError);

// Three short texts are 10, 11 and 10 units long. The first two fit in one batch
// (21 units). That batch then holds 2 texts, the most allowed, so the helper puts
// the third text in a second batch.
const texts = ['Dogs bark.', 'Cats sleep.', 'Birds fly.'];
const docs: Document[] = [];
for (const batch of planBatches(texts, limits)) docs.push(...await nlp.processBatch(batch));

// Every text was processed, and the results are in the original order.
assert.deepEqual(docs.map(doc => doc.text), texts);

// Remove the extra input limits.
configureInputLimits(null);

// Error case 4: text the model cannot read. '\ud800' is half of a two-part UTF-16
// character with its other half missing. The error is not SPARS_BUSY, so the retry
// helper passes it on at once.
await assert.rejects(processWithRetry(nlp, '\ud800'), { code: 'SPARS_INVALID_TEXT' });

// A failed call does not break anything: the next valid call succeeds.
assert.equal((await nlp.process('Still works.')).text, 'Still works.');
```

For `process`, `processBatch`, `pipe`, and `findMatches`, `SPARS_BUSY` means every active slot is in use and the waiting queue is full; it does not indicate a problem with the input. (Matcher `add` and `remove` also throw `SPARS_BUSY` synchronously while a search on that matcher is pending; see [token matching](#token-matching).) Retrying immediately and without a limit adds calls to a queue that is already full and prolongs the overload. Use a bounded policy instead: a few delayed attempts, as `processWithRetry` shows, or reject the request (for example with HTTP 503) and let the client retry later. The same call can fail differently depending on load. While the queue has room, `processBatch(['ok', 42])` throws `TypeError`; while the queue is full, it rejects with `SPARS_BUSY`, because admission is checked before batch elements are inspected. A retried call that is admitted can therefore still throw `TypeError`, so validate arguments before retrying.

`SPARS_INPUT_LIMIT` means an input exceeded the policy set with `configureInputLimits`. A batch is rejected as a whole when its element count, total length, or any single element exceeds the policy; no documents from that call are returned. Either reject the input, or group separate, independent documents into smaller batches, as `planBatches` does. Do not cut one document into arbitrary pieces to fit a limit: each piece is processed without the surrounding text, so tokens, sentences, entities, and dependencies near a cut can change, and offsets start again at zero in each piece. The per-model `nlp.maxLength` limit is separate: it counts Unicode code points and rejects with `SPARS_TEXT_TOO_LONG` after admission, also failing the whole batch. With `pipe`, a rejection stops iteration after earlier buffers may already have been yielded, so account for consumed documents instead of restarting the source.

Handle only the codes that the application's policy understands, and rethrow the rest, as `processWithRetry` does with `SPARS_INVALID_TEXT`. A rejected call does not leave the package unusable: later valid calls succeed after busy, input-limit, and processing rejections. The defaults remain two active jobs, 32 queued jobs, and no additional input caps. Both policies apply per package instance and JavaScript isolate. They bound admission and input size, not the JavaScript-thread cost of converting large accepted results into objects.

## Retain and restore native documents

`nlp.processDocument(text, stage?)` runs the same pipeline as `process` and retains an immutable native document. It shares the inference queue, optional text input limit, and per-model `maxLength`. Existing `process` and `processBatch` results remain plain JavaScript objects. Call `toObject()` when you need that output shape; each call returns an independent copy.

```typescript
import assert from 'node:assert/strict';
import { loadModel, NativeDocument } from '@spars/node';

// Load the model once.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// Process a sentence and keep the result as a native document. A native document
// stays in the fast Rust part of the library instead of being copied into plain
// JavaScript objects, and it cannot be changed.
const doc = await nlp.processDocument('Alice visits London.');

// A snapshot is the whole document saved as a JSON string. Save the document,
// then build a second document from that string.
const restored = NativeDocument.fromSnapshot(doc.toSnapshot());

// `toObject` copies a native document into plain JavaScript objects. The restored
// document gives exactly the same output as the original, so nothing was lost.
assert.deepEqual(restored.toObject(), doc.toObject());

// Each `toObject` call returns its own separate copy. Emptying the token list of
// one copy changes only that copy.
const output = restored.toObject();
output.tokens.length = 0;

// The native document still has all its tokens (words and punctuation marks).
assert.ok(restored.toObject().tokens.length > 0);
```

`toSnapshot()` preserves native annotations, contextual vectors and entity IDs when available. A snapshot with entity IDs uses format version 3, which older SpaRs versions reject instead of dropping the IDs. Documents with [span groups](#store-span-groups), from Node or Rust, use format version 4, which older SpaRs versions reject instead of dropping the groups. Span groups in an earlier version throw `SPARS_UNSUPPORTED`; repeated group keys, repeated attr keys, empty span IDs or unknown group fields throw `SPARS_INVALID_MODEL`. Entity IDs must be omitted rather than empty, may appear only on tokens and entities, must match each entity's first token, and require format version 3 or later; otherwise import throws `SPARS_INVALID_MODEL`, or `SPARS_UNSUPPORTED` for IDs in an earlier version. `NativeDocument.fromSnapshot(json)` validates the complete snapshot, including text boundaries, annotation indices, tensor shapes and closed values: entity IOB tags must be `B`, `I`, `O` or empty, POS tags must be spaCy universal POS tags or empty, entities need a label, sentences have none and noun chunks are `NP`. Malformed JSON, a missing field, an invalid tensor or a value outside these closed sets throws `SPARS_INVALID_MODEL`; offsets, token indices or spans that do not fit the text throw `SPARS_BOUNDS`; an unknown format version throws `SPARS_UNSUPPORTED`; and input with an unpaired UTF-16 surrogate throws `SPARS_INVALID_TEXT`. Import also rejects, with `SPARS_INVALID_MODEL`, an `I` tag with no entity to continue and an entity list other than the one the token IOB tags, types and first-token IDs describe; see [entity annotation in snapshots](ENTITIES.md#entity-annotation-in-snapshots). This versioned native format is separate from spaCy JSON and DocBin. Native documents remain usable after their originating model is released. Snapshot import/export and plain-object conversion run synchronously on the JavaScript thread and allocate memory proportional to the document, including any contextual tensor; inference admission limits do not bound these operations.

### Native memory and garbage collection

A native document's text, tokens, spans and contextual tensor live outside the JavaScript heap. Each `NativeDocument` reports an estimate of that memory to V8, Node's JavaScript engine and garbage collector, when it is created, and withdraws it when the object is collected, so V8 can collect unused documents sooner when many large ones are created. `reportedNativeMemory()` returns the total external memory reported through Node-API to the current thread's V8 isolate (one independent JavaScript heap; each worker thread has its own), in bytes. The total also includes memory reported by other native code, so compare values taken before and after an operation rather than reading it as SpaRs' own use. On Node 24.17, the version tested, `process.memoryUsage().external` does not include it. This example builds a document from a small snapshot, so it needs only the built addon, not a model.

```typescript
import assert from 'node:assert/strict';
import { NativeDocument, reportedNativeMemory } from '@spars/node';

// Note how much native memory has been reported so far, in bytes. Native memory is
// memory used outside JavaScript's own storage, here by the Rust part of the library.
const before = reportedNativeMemory();

// Build a document from a snapshot, a document saved as a JSON string. This one is
// written by hand, so no model is needed. It holds the text and two tokens (pieces
// of the text): "Hello" at bytes 0 to 5 and "world" at bytes 6 to 11. `idx` is where
// each token starts counted in characters, `whitespace` says whether a space
// follows it, and `norm` is its normalized (here lowercase) form.
const doc = NativeDocument.fromSnapshot(JSON.stringify({ format_version: 1, document: { text: 'Hello world', tokens: [
  { start: 0, end: 5, idx: 0, whitespace: true, norm: 'hello' },
  { start: 6, end: 11, idx: 6, whitespace: false, norm: 'world' },
] } }));

// Creating the document reported its memory, so the total went up.
assert.ok(reportedNativeMemory() > before);

// "Hello world" is 11 UTF-16 units long, the same count as JavaScript's
// `string.length` gives.
assert.equal(doc.utf16Length, 11);
```

`utf16Length` is the text length in UTF-16 code units, the same as JavaScript's `string.length`. For documents from `processDocument` and `NativeDocument.fromSnapshot`, the estimate counts allocated capacity and is a lower bound: it leaves out the allocator's per-allocation overhead and the dependency index that the first tree traversal builds and caches. A document from `withEntities` reports its source's estimate without recounting; the copy usually owns somewhat less, because copying drops unused capacity, but slightly more when the update adds entity tags to a document that had none. Documents from `withSpanGroup` and `withoutSpanGroup` are recounted, including their span groups. Token and span views share their document's memory without reporting it, so a view that outlives its document object keeps unreported memory alive until the view is collected. Node frees a document's native memory in a finalizer, a cleanup callback that runs on a later event-loop turn after garbage collection finds the object unused, so a synchronous loop that creates many documents without yielding cannot free any of them until it ends. When creating many large documents in one task, yield between documents (or every few documents), for example with `await setImmediate()` from `node:timers/promises`. `bindings/node/tests/memory.test.mts` checks that reports are made, released on collection, reused by copies and skipped for rejected updates; `npm --prefix bindings/node test` runs it.

## Lexical attributes

`nlp.lexeme(text)` returns an owned record of the existing native lexical attributes, including normalization, shape, prefix/suffix, lexical flags, and static-vector availability. It runs synchronously and does not change the vocabulary.

```typescript
import assert from 'node:assert/strict';
import { loadModel } from '@spars/node';

// Load the model once.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// Look up a lexeme. A lexeme is a word seen by itself, outside any sentence. It
// holds the facts that are true wherever the word appears, such as its spelling.
const lexeme = nlp.lexeme('London');

// "London" is made of letters only.
assert.equal(lexeme.isAlpha, true);

// The suffix is the last three characters of the word.
assert.equal(lexeme.suffix, 'don');

// `orth` is a number that identifies the exact spelling. It can be too large for
// an ordinary JavaScript number, so it is a `bigint`, which stays exact.
assert.equal(typeof lexeme.orth, 'bigint');
```

`orth` is a JavaScript `bigint` so the unsigned 64-bit string identity stays exact. Convert it explicitly to a decimal string when serializing a lexical record to JSON; JavaScript's default JSON serializer rejects bigints.

## Model-store management

`ModelStore` exposes offline selection of installed model directories. Construction and `discover()` capture the directory at call time; `resolve` and `register` perform filesystem work asynchronously. Registration selects an existing direct child directory and does not inspect model weights. Loading by name still validates the selected model identity and contents.

```typescript
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, realpathSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { ModelStore } from '@spars/node';

// Create an empty temporary folder to act as the model store. A model store is
// the folder where installed models are kept.
const directory = mkdtempSync(join(tmpdir(), 'spars-store-'));
try {
  // Open the store on that folder.
  const store = new ModelStore(directory);

  // Make a folder directly inside the store to stand in for an installed model.
  const installed = join(directory, 'example-installation');
  mkdirSync(installed);

  // Tell the store that this folder is the installation for the name
  // 'en_core_web_sm'. Registering does not check that the folder holds a real model.
  await store.register('en_core_web_sm', installed);

  // Asking the store for that name gives back the full path of the same folder.
  assert.equal(await store.resolve('en_core_web_sm'), realpathSync(installed));

// Delete the temporary folder at the end, even if a step above failed.
} finally { rmSync(directory, { recursive: true, force: true }); }
```

Use `ModelStore.discover()` for `SPARS_MODEL_DIR` or the platform cache default. `store.root` supplies the `{path}` option for `loadModel(name, {path: store.root})`. Store operations neither acquire models nor implement update/removal commands.

## Token and span views

Native documents expose checked token and span views that retain the document. Indices must be nonnegative integers within the document; spans use an exclusive end. `doc.tokens()` and `span.tokens()` return arrays of token views. Traversal returns arrays in the same order as the Rust API; unavailable dependency annotations raise an error, while unavailable sentence lists return `null`.

```typescript
import assert from 'node:assert/strict';
import { loadModel } from '@spars/node';

// Load the model once. It is reused for every step below.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// Process a sentence and keep it as a native document, which can be explored with
// views. The tokens (words and punctuation marks) are: Alice(0) runs(1) .(2)
const doc = await nlp.processDocument('Alice runs.');

// A token view is a small object that points at one token of the document.
const alice = doc.token(0);
assert.equal(alice.text, 'Alice');

// The model links every word to the word it depends on, called its head. "Alice"
// is the one doing the running, so the head of "Alice" is "runs".
assert.equal(alice.head()?.text, 'runs');

// Ancestors are the head, then the head's head, and so on. "runs" is the main word
// of the sentence, so the chain stops there.
assert.deepEqual(alice.ancestors().map(token => token.text), ['runs']);

// A token can report the sentence it belongs to.
assert.equal(alice.sentence().text, 'Alice runs.');

// A span is a run of tokens in a row. The end index is exclusive, so span(0, 2)
// covers tokens 0 and 1.
assert.equal(doc.span(0, 2).text, 'Alice runs');

// The document contains one sentence.
assert.equal(doc.sentenceViews()?.length, 1);
```

`token.annotations()` returns a copied record of the native annotations and byte/code-point start offsets. `token.children()`, `ancestors()`, `subtree()`, `head()`, `span()`, and `sentence()` provide graph and sentence access. Use `doc.toObject()` for the full JavaScript output with UTF-16 offsets. Views are immutable; retaining even one view retains its full document and contextual tensor. Traversal and view creation run synchronously and allocate in proportion to the returned results.

## Edit entity annotation

`doc.withEntities(update)` replaces entity annotation like spaCy's `Doc.set_ents` and returns a new native document. The original document and every token and span view of it keep their earlier annotation, so code holding them is unaffected. Intervals use token indices with an exclusive end. `entities` lists labeled intervals, each with an optional `id`; `outside`, `blocked` and `missing` list unlabeled intervals for tokens outside any entity, tokens that can never be part of one, and tokens whose annotation is unknown. `default` sets every other token: `'outside'` (the default), `'missing'`, `'blocked'` or `'unmodified'`. An entity's `id` is written to its tokens, which report it as `entityId`, and each entity in `toObject().entities` reports its first token's ID as `id`. In output, `''` means no ID; in an update, an `id` of `''` has the same effect as leaving it out. As in spaCy, an entity without an `id` keeps the IDs its tokens already have, and tokens that stop being part of an entity keep theirs; the [entity editing guide](ENTITIES.md#entity-ids) explains these rules.

```typescript
import assert from 'node:assert/strict';
import { loadModel } from '@spars/node';

// Load the model once. It is reused for every step below.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// Process a sentence. The tokens (words and punctuation marks) are:
// Tim(0) Cook(1) visited(2) London(3) .(4)
// The model finds two entities, which are named real-world things: "Tim Cook" with
// the label PERSON and "London" with the label GPE (a country, city or similar place).
const doc = await nlp.processDocument('Tim Cook visited London.');

// Relabel "London" as CITY. The entity covers token 3; `end` is exclusive.
// `default: 'unmodified'` leaves every other token as the model labeled it.
const edited = doc.withEntities({ entities: [{ start: 3, end: 4, label: 'CITY' }], default: 'unmodified' });

// The edited copy keeps the PERSON entity and has CITY in place of GPE.
assert.deepEqual(edited.toObject().entities?.map(entity => entity.label), ['PERSON', 'CITY']);

// An entity can also carry an ID, a name of your choice for the thing it refers to.
// Set the "Tim Cook" entity again, this time with the ID 'tim-cook'.
const named = doc.withEntities({ entities: [{ start: 0, end: 2, label: 'PERSON', id: 'tim-cook' }], default: 'unmodified' });

// The ID is written to each token of the entity, and the entity reports it too.
assert.equal(named.token(1).annotations().entityId, 'tim-cook');
assert.equal(named.toObject().entities?.[0]?.id, 'tim-cook');

// Editing always makes a copy. The original document still labels "London" GPE.
assert.equal(doc.token(3).annotations().entityType, 'GPE');

// Error case: the document has only 5 tokens, so an entity ending at token 9
// does not fit and is refused with the SPARS_BOUNDS error.
assert.throws(() => doc.withEntities({ entities: [{ start: 3, end: 9, label: 'CITY' }] }), { code: 'SPARS_BOUNDS' });
```

The original `doc` still reports London as `GPE`. An interval outside the document, a reversed interval, a non-integer or negative index, or two intervals sharing a token (entities with an empty label are ignored) throws `SPARS_BOUNDS`. A label with an unpaired surrogate throws `SPARS_INVALID_TEXT`, and an update that leaves an `I` tag with no entity to continue, for example by blocking the token before an `I` that has no entity type, throws `SPARS_UNSUPPORTED`; spaCy writes such an update and fails when the entities are next read. An object of the wrong shape, an unknown `default`, or an undeclared property such as a misspelled `ents` or an unsupported `kbId` throws a native argument error (`InvalidArg`, `NumberExpected` or `StringExpected`), so a typo cannot silently become the default update. The update runs synchronously on the JavaScript thread, outside inference admission and input limits. It is checked before anything is copied, so an invalid update throws without copying; a valid one copies the whole document, including any contextual tensor. Token states, the repair of inside tags and the differences from spaCy are described in the [entity editing guide](ENTITIES.md).

## Store span groups

Span groups are named lists of spans, like spaCy's `Doc.spans`: the spans of a group may overlap, repeat or be empty, and groups never change entities. `doc.withSpanGroup(key, group)` returns a new native document with `group` stored under `key`, and `doc.withoutSpanGroup(key)` returns one without it; like `withEntities`, they leave the original document and its views unchanged. `toObject().spanGroups` lists every group, in the order its key was added, with its `key`, `name`, `attrs` and `spans`. A new key goes last, replacing a key keeps its position, and a key that is removed and set again goes last. A group's `name` defaults to its key, as when spaCy is given a plain list of spans, and a span's `label` defaults to `''`. Span IDs are reported as `''` when absent. Node has no equivalent of spaCy's `has_overlap`, `extend` or `+`: to change a group, read it from `toObject().spanGroups`, edit the plain object and store it again with `withSpanGroup`; reading a group converts the whole document. The [span groups guide](SPAN_GROUPS.md) describes the behavior and the [differences from spaCy](SPAN_GROUPS.md#differences-from-spacy).

```typescript
import assert from 'node:assert/strict';
import { loadModel } from '@spars/node';

// Load the model once. It is reused for every step below.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// Split a sentence into tokens (words and punctuation marks). A stage is one step
// of the model's pipeline; 'Tokenizer' runs only the splitting step, which is all
// span groups need. The tokens are:
// Tim(0) Cook(1) visited(2) New(3) York(4) City(5) .(6)
const doc = await nlp.processDocument('Tim Cook visited New York City.', 'Tokenizer');

// A span is a run of tokens in a row, and a span group is a named list of spans.
// Store a group called 'places' with two spans that overlap: "New York City"
// (tokens 3 to 5) and "New York" (tokens 3 and 4). `end` is exclusive. The second
// span has the ID 'nyc'. `attrs` holds extra information of your choice about the group.
const edited = doc.withSpanGroup('places', {
  attrs: { source: 'gazetteer', confidence: [0.9, 0.7] },
  spans: [{ start: 3, end: 6, label: 'GPE' }, { start: 3, end: 5, label: 'GPE', id: 'nyc' }],
});

// Read the groups back as plain JavaScript objects and take the first one.
const [places] = edited.toObject().spanGroups;

// No name was given, so the group's name is the same as its key.
assert.equal(places?.name, 'places');

// The extra information comes back unchanged.
assert.deepEqual(places?.attrs, { source: 'gazetteer', confidence: [0.9, 0.7] });

// Both spans were kept, in the order given. A span without an ID reports ''.
assert.deepEqual(places?.spans.map(span => [span.start, span.end, span.id]), [[3, 6, ''], [3, 5, 'nyc']]);

// Storing a group makes a copy, so the original document still has no groups.
assert.deepEqual(doc.toObject().spanGroups, []);

// Removing the group also returns a copy, this one without the group.
assert.deepEqual(edited.withoutSpanGroup('places').toObject().spanGroups, []);

// Error case: the document has only 7 tokens, so a span ending at token 9 does not
// fit and is refused with the SPARS_BOUNDS error.
assert.throws(() => doc.withSpanGroup('places', { spans: [{ start: 3, end: 9 }] }), { code: 'SPARS_BOUNDS' });

// Error case: `attrs` may hold only plain values such as text, numbers and lists.
// TypeScript rejects a Date in attrs at compile time; untyped JavaScript gets a TypeError when called.
assert.throws(() => Reflect.apply(doc.withSpanGroup, doc, ['places', { attrs: { when: new Date() }, spans: [] }]), TypeError);
```

The example uses the `'Tokenizer'` stage because span groups need no other annotation. Its group holds "New York City" and "New York", the second with ID `nyc`, and the original `doc` has no groups.

`attrs` is optional; leaving it out or passing `null` means no attrs. Otherwise it must be an object whose values are plain JSON, the values a native snapshot can save: `null`, booleans, finite numbers, strings, arrays without holes, and objects with the default prototype or none. An `attrs` that is not an object, or a nested `undefined`, `NaN`, infinite number, `Date`, `Map`, class instance, getter, symbol key or circular reference, throws `TypeError` rather than being changed or dropped silently, and a string or key with an unpaired surrogate throws `SPARS_INVALID_TEXT`. Attr keys keep their order, except that JavaScript objects list integer-like keys such as `'1'` first, and numbers are JavaScript numbers, so an integer beyond `Number.MAX_SAFE_INTEGER` in a snapshot written from Rust loses precision when read. A span with `start > end`, past the last token, or with a negative or non-integer index throws `SPARS_BOUNDS`; a key, name, label or ID with an unpaired surrogate throws `SPARS_INVALID_TEXT`; `withoutSpanGroup` with a key that has no group throws a native argument error with code `InvalidArg`, as spaCy raises `KeyError`; and an undeclared property such as `kbId`, or a value of the wrong type, throws a native argument error. Both methods run synchronously on the JavaScript thread, outside inference admission and input limits; an invalid group throws without copying, and a valid edit copies the whole document, including any contextual tensor.

Group keys are typed `SpanGroupKey`, which offers spaCy's default keys `ruler` and `sc` for autocomplete and accepts any string, and span labels are `SpanLabel`, which offers the entity labels. Register your own with `interface SpanGroupKeys` and `interface SpanLabels`, as described in [types and autocomplete](#types-and-autocomplete); `StrictLabelSets` uses `span` for span labels.

## Vectors and similarity

```typescript
import assert from 'node:assert/strict';
import { loadModel } from '@spars/node';

// Load the model once. It is reused for every step below.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// Process two words. The tokens (pieces of the text) are: dog(0) cat(1)
const doc = await nlp.processDocument('dog cat');

// A vector is a list of numbers that stands for meaning. Texts with similar
// meanings have similar numbers. The model can give one vector for the whole document.
assert.ok(nlp.documentVector(doc) instanceof Float32Array);

// It can also give the vector of one token, here token 0, "dog".
assert.ok(nlp.tokenVector(doc, 0) instanceof Float32Array);

// And the vector of a span, a run of tokens in a row. The end index is exclusive,
// so the span from 0 to 1 holds only "dog".
assert.ok(nlp.spanVector(doc, 0, 1) instanceof Float32Array);

// Similarity is a score for how alike two vectors are. A document compared with
// itself scores 1, the score for identical meaning.
assert.equal(nlp.similarity(doc, doc), 1);

// The same holds for a span compared with itself.
assert.equal(nlp.spanSimilarity(doc, 0, 1, doc, 0, 1), 1);
```

The receiving model selects vector behavior: md/lg use their static vocabulary, while sm uses contextual rows retained in the supplied document. Documents do not record model identity; when comparing contextual documents, callers should use the same model and processing stage. Token vectors return `null` when unavailable; unavailable document/span vectors are empty arrays. All returned vectors are independent copies. Similarity preserves identical-token and zero-vector shortcuts, then rejects incompatible nonzero vector dimensions with `SPARS_UNSUPPORTED`; this check is stricter than the current Rust similarity helpers, which truncate unequal dimensions during their dot product.

These methods run synchronously. Mean vectors and similarities scan the selected tokens and vector dimensions; they do not run inference. Large repeated comparisons can delay the JavaScript thread.

## Token matching

`TokenMatcher` compiles the existing native token patterns and returns ordered, overlapping matches over a `NativeDocument`. Patterns use the typed SpaRs schema, not spaCy's Python dictionary syntax. Registration validates the complete addition before changing rules; unsupported fields and predicate/operator values reject.

```typescript
import assert from 'node:assert/strict';
import { loadModel, TokenMatcher } from '@spars/node';

// Load the model once. It is reused for every step below.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// Split a sentence into tokens (words and punctuation marks). The 'Tokenizer' stage
// runs only that first step of the model, which is enough to match on text.
// The tokens are: Alice(0) visits(1) London(2) .(3)
const doc = await nlp.processDocument('Alice visits London.', 'Tokenizer');

// A matcher holds named rules. Each rule is a list of patterns to look for.
const matcher = new TokenMatcher();

// The rule 'place' has one pattern, and the pattern describes one token: its text
// must equal "London", and it must appear exactly once.
matcher.add('place', [{ tokens: [{
  constraints: [{ attribute: 'text', predicate: { kind: 'equals', value: 'London' } }],
  repetition: { kind: 'once' },
}] }]);

// Search the document. One match is found: it starts at token 2 and stops before
// token 3. The end is exclusive, so the match covers only "London".
assert.deepEqual(await matcher.findMatches(doc), [{ rule: 'place', start: 2, end: 3 }]);
```

Supported token attributes are `text`, `lower`, `norm`, `lemma`, `pos`, `tag`, `dep`, `morphology`, the lexical flags `is_alpha`, `is_digit`, `is_space`, `is_punct`, `like_num`, `is_lower`, `is_upper`, `is_title`, `is_ascii`, `is_currency`, `is_stop`, `is_bracket`, `is_quote`, `is_left_punct`, `is_right_punct`, `like_url` and `like_email`, and `length`. `lower` compares the token's pinned Unicode 15.0.0 lowercase text, as in spaCy's `LOWER`, and needs no model annotations; pattern values are compared as written, so write them in lowercase. Predicates are `equals` with a string `value`, `flag` with a boolean `value`, `compare` with an `operator` (`==`, `!=`, `>=`, `<=`, `>` or `<`) and a finite number `value`, `in`, `not_in`, `is_subset`, `is_superset`, `intersects`, `morph_superset`, and `morph_intersects` with string `values`, or `in_integers`, `not_in_integers`, `is_subset_integers`, `is_superset_integers` and `intersects_integers` with integer `values`. `is_subset`, `is_superset` and `intersects` follow spaCy's set operators on any string attribute (see [Compare with a set of values](TOKEN_MATCHER.md#compare-with-a-set-of-values)); `morph_superset` and `morph_intersects` require morphology; `flag` is the only predicate for lexical flags; `length` accepts only `compare` and the integer predicates and counts Unicode code points. Integer `values` must be safe JavaScript integers (whole numbers up to 2^53−1 in magnitude); other numbers are rejected rather than rounded. A `compare` value may be any finite number, including a fraction such as `2.5`; NaN and infinities are rejected. For example, `{ attribute: 'length', predicate: { kind: 'compare', operator: '>=', value: 7 } }` matches tokens of at least seven code points. Repetition kinds are `once`, `optional`, `zero_or_more`, `one_or_more`, `negated`, and `range`; a range requires `min` and accepts an optional inclusive `max`. An `equals` condition on `lemma`, `pos`, `tag`, `dep` or `morphology` rejects with `SPARS_INFERENCE` when no token of a nonempty document has that annotation; a token without the annotation compares as the empty string, with special rules for missing morphology, as in spaCy (see [missing annotations](TOKEN_MATCHER.md#missing-annotations)). See the [token matcher guide](TOKEN_MATCHER.md) for native semantics and unsupported options.

`TokenMatcher.add` takes an optional third argument, an options object that stands for the keyword arguments of spaCy's `Matcher.add`. The only option is `greedy`, typed `Greedy` (spaCy's `on_match` is not available): `matcher.add('place', patterns, { greedy: 'LONGEST' })` keeps, for that rule, only matches that share no token with each other, preferring the longest; `'FIRST'` prefers the earliest start. The values are spaCy's, in upper case. Omitting the argument or the option, or passing `null`, reports every match, like Python's `None`. As in spaCy, each `add` call sets the options for the whole rule, so a later `add` without `greedy` makes the rule report every match again, and an `add` with an empty pattern list adds no pattern but still sets the options, registering the rule if its name is new. Any other well-formed string, such as `'longest'`, throws `SPARS_INVALID_PATTERN`, and it is reported before errors in well-typed pattern values, such as an empty pattern, as spaCy reports its error E947 first; a string with an unpaired surrogate throws `SPARS_INVALID_TEXT`. A `greedy` that is not a string, or an unknown option such as spaCy's `on_match`, throws `InvalidArg`. The matches of rules with `greedy` come after all other matches in the result. See [Keep matches that do not overlap](TOKEN_MATCHER.md#keep-matches-that-do-not-overlap) for an example and the result order, and the [accepted difference](SPACY_DIFFERENCES.md#node-checks-pattern-shapes-before-greedy) in the order of the checks for a pattern of the wrong shape or field type.

Lexical flag conditions use the model's language rules, so create the matcher with a model: `new TokenMatcher(nlp)` or `new DependencyMatcher(nlp)`. A matcher created without a model rejects flag conditions with `SPARS_INVALID_PATTERN`. This example finds a number followed by a word:

```typescript
import assert from 'node:assert/strict';
import { loadModel, TokenMatcher } from '@spars/node';
import type { FlagAttribute, TokenPatternItem } from '@spars/node';

// Load the model once. It is reused for every step below.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// A lexical flag is a yes-or-no fact about a word, such as "looks like a number".
// The answers depend on the language, so the matcher is given the model.
const matcher = new TokenMatcher(nlp);

// A small helper that builds one pattern item: a single token for which the given
// flag is true.
const flag = (attribute: FlagAttribute): TokenPatternItem => ({
  constraints: [{ attribute, predicate: { kind: 'flag', value: true } }],
  repetition: { kind: 'once' },
});

// The rule 'quantity' describes two tokens in a row: one that looks like a number,
// then one made of letters only.
matcher.add('quantity', [{ tokens: [flag('like_num'), flag('is_alpha')] }]);

// Split a sentence into tokens (words and punctuation marks). The 'Tokenizer' stage
// runs only that first step of the model. The tokens are:
// Ten(0) people(1) paid(2) 1,000(3) dollars(4) .(5)
const doc = await nlp.processDocument('Ten people paid 1,000 dollars.', 'Tokenizer');

// Two matches are found: "Ten people" and "1,000 dollars". A number written as a
// word counts as number-like too. Each `end` is exclusive.
assert.deepEqual(await matcher.findMatches(doc), [
  { rule: 'quantity', start: 0, end: 2 },
  { rule: 'quantity', start: 3, end: 5 },
]);
```

Matcher instances support `size`, `contains(rule)`, `get(rule)`, `add(rule, patterns)`, and `remove(rule)`. Repeated additions append to an existing rule; `get` returns independent pattern copies or `null`. Matching runs on the worker pool using the retained native document, without rerunning inference or copying compiled rules. Searches share the inference admission queue and optional `maxTextLength` policy. Rule mutation is rejected with `SPARS_BUSY` while any search on that matcher is queued or active; after the returned promise settles, mutation is available again. Registration and inspection are synchronous and are not limited by inference input policies. Large match arrays still require JavaScript-thread allocation.

## Dependency matching

`DependencyMatcher` uses the same token constraints and matcher lifecycle as `TokenMatcher`. Each node after the first links to an earlier named node. Matches contain token indices in pattern-node order. All 20 native relationship operators are supported; see the [dependency matcher guide](DEPENDENCY_MATCHER.md) for their meanings and remaining scope.

```typescript
import assert from 'node:assert/strict';
import { loadModel, DependencyMatcher } from '@spars/node';

// Load the model once. It is reused for every step below.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// A dependency matcher looks for words by how they are linked in the sentence's
// grammar, not by where they sit in the text.
const matcher = new DependencyMatcher();

// The rule 'subject' describes two linked tokens. Each one is a node with a name.
matcher.add('subject', [{ nodes: [
  // First node, named 'verb': no conditions, so any token can fill it.
  { id: 'verb', constraints: [] },
  // Second node, named 'subject': its dependency label must be `nsubj`, the label
  // for the subject of a sentence. The link says how it relates to 'verb': the
  // relation '>' means the 'verb' token is the direct head of this token, the word
  // this token depends on.
  { id: 'subject', constraints: [{ attribute: 'dep', predicate: { kind: 'equals', value: 'nsubj' } }], link: { left: 'verb', relation: '>' } },
] }]);

// Process a sentence with the full pipeline, because the matcher needs the links
// between words. The tokens (words and punctuation marks) are: Alice(0) runs(1) .(2)
const doc = await nlp.processDocument('Alice runs.');

// One match is found. Its tokens are listed in the order of the pattern's nodes:
// token 1, "runs", fills 'verb' and token 0, "Alice", fills 'subject'.
assert.deepEqual(await matcher.findMatches(doc), [{ rule: 'subject', tokens: [1, 0] }]);
```

## Phrase matching on text, lowercase, annotations, lexical flags or length

`PhraseMatcher` accepts native documents as patterns. Choose `LOWER` for Python-compatible lowercase matching, `ORTH`/`TEXT` for exact token text, `NORM` for token norms, `LEMMA`, `POS`, `TAG`, `DEP` or `MORPH` for linguistic annotations, a lexical flag such as `IS_ALPHA` or `LIKE_NUM`, or `LENGTH`. Lexical flags need the model as a second argument, `new PhraseMatcher('LIKE_NUM', nlp)`, for its language rules; adding a rule to a flag matcher without one throws `SPARS_INVALID_PATTERN`. `LENGTH` needs no model, and a model passed for any other attribute has no effect. It follows the same asynchronous matching, rule mutation, and admission contracts as the other matchers.

```typescript
import assert from 'node:assert/strict';
import { loadModel, PhraseMatcher } from '@spars/node';

// Load the model once. It is reused for every step below.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// A phrase matcher finds whole phrases. 'LOWER' tells it to compare the lowercase
// form of each token (word or punctuation mark), so capital letters do not matter.
const matcher = new PhraseMatcher('LOWER');

// Add the rule 'hotel'. A phrase pattern is itself a document, so the phrase is
// processed first. The 'Tokenizer' stage, which only splits text into tokens, is
// enough for comparing text.
matcher.add('hotel', [await nlp.processDocument('the ritz', 'Tokenizer')]);

// Split the text to search in the same way. The tokens are: The(0) Ritz(1)
const doc = await nlp.processDocument('The Ritz', 'Tokenizer');

// "The Ritz" matches "the ritz" despite the capitals. The match starts at token 0
// and stops before token 2; the end is exclusive.
assert.deepEqual(await matcher.findMatches(doc), [{ rule: 'hotel', start: 0, end: 2 }]);

// `get` shows the values the rule compares: the lowercase text of each pattern token.
assert.deepEqual(matcher.get('hotel'), [['the', 'ritz']]);
```

Lowercase mappings are pinned to Unicode 15.0.0 and include contextual Greek sigma. This is lowercase matching, not case folding or Unicode normalization: `ß` does not become `ss`, and composed/decomposed accents remain distinct. Phrase boundaries still follow tokenization. `get(rule)` returns copies of the unique compared token values in first-registration order: lowercase text for `LOWER`, the annotation values, such as `[['the', 'dog', 'run']]`, for `LEMMA`, `'true'` and `'false'` for flags, and decimal numbers such as `'3'` for `LENGTH`. Matching preserves overlaps and native result order. Attribute names must be uppercase. spaCy also accepts lowercase names, but here lowercase names such as `'lemma'` and the unsupported attributes `SHAPE`, the entity attributes, `SENT_START` and `SPACY` throw `SPARS_UNSUPPORTED`. Span input, callbacks, and Python pattern JSON are unsupported; see the [phrase matcher guide](PHRASE_MATCHER.md).

Annotation attributes need pattern documents processed by the model rather than only tokenized:

```typescript
import assert from 'node:assert/strict';
import { loadModel, PhraseMatcher } from '@spars/node';

// Load the model once. It is reused for every step below.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// 'LEMMA' tells the matcher to compare lemmas. The lemma is the dictionary form of
// a word, so "dogs" has the lemma "dog" and "ran" has the lemma "run".
const matcher = new PhraseMatcher('LEMMA');

// Add the rule 'dog-runs'. The phrase goes through the full pipeline, because the
// model has to work out the lemma of each token (word or punctuation mark).
matcher.add('dog-runs', [await nlp.processDocument('the dog runs')]);

// Process the text to search. The tokens are: The(0) dogs(1) ran(2) home(3) .(4)
const doc = await nlp.processDocument('The dogs ran home.');

// "The dogs ran" has the same lemmas as "the dog runs", so it matches even though
// the words differ. The match covers tokens 0 to 2; the end is exclusive.
assert.deepEqual(await matcher.findMatches(doc), [{ rule: 'dog-runs', start: 0, end: 3 }]);

// Error case: a phrase that was only split into tokens has no lemmas to compare.
// A stage is one step of the pipeline, and 'Tokenizer' stops after the first one.
const tokensOnly = await nlp.processDocument('the dog', 'Tokenizer');
assert.throws(() => matcher.add('tokens-only', [tokensOnly]), { code: 'SPARS_INVALID_PATTERN' });

// The failed call added nothing, so the matcher has no rule with that name.
assert.equal(matcher.contains('tokens-only'), false);
```

A nonempty pattern document with no token carrying the selected annotation throws `SPARS_INVALID_PATTERN`, and the matcher is unchanged. Input documents need no annotations: a token without the annotation has the value `""` and matches only a pattern token that also lacks it. An empty morphological analysis has the value `_`. Morphology must be in spaCy's canonical order, as the model writes it; other morphology makes `MORPH` matching reject with `SPARS_UNSUPPORTED`.

Lexical flags compare whether each token has a property, so a pattern matches any tokens with the same flags. This matcher finds a number-like token followed by one that is not:

```typescript
import assert from 'node:assert/strict';
import { loadModel, PhraseMatcher } from '@spars/node';

// Load the model once. It is reused for every step below.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// 'LIKE_NUM' is a lexical flag, a yes-or-no fact about a word: does it look like a
// number? The answer depends on the language, so the matcher is given the model.
const matcher = new PhraseMatcher('LIKE_NUM', nlp);

// Add the rule 'count' from the phrase "3 apples". The 'Tokenizer' stage, the step
// that splits text into tokens (words and punctuation marks), is enough for flags.
matcher.add('count', [await nlp.processDocument('3 apples', 'Tokenizer')]);

// The rule stores only the flag of each pattern token: "3" looks like a number
// ('true') and "apples" does not ('false'). The words themselves are not compared.
assert.deepEqual(matcher.get('count'), [['true', 'false']]);

// Split the text to search. The tokens are:
// I(0) ate(1) 3(2) apples(3) and(4) ten(5) pears(6) .(7)
const doc = await nlp.processDocument('I ate 3 apples and ten pears.', 'Tokenizer');

// Any number-like token followed by one that is not matches: "3 apples" and also
// "ten pears", because "ten" counts as number-like. Each `end` is exclusive.
assert.deepEqual(await matcher.findMatches(doc), [{ rule: 'count', start: 2, end: 4 }, { rule: 'count', start: 5, end: 7 }]);
```

## Add entities with rules

`EntityRuler` adds named entities from token patterns and phrases, like spaCy's `entity_ruler` placed after the recognizer. Each pattern has a `label`, an optional `id`, and either `tokens`, a [token pattern](#token-matching), or `phrase`, a native document. Process phrase text with the same model stages as the documents the ruler annotates, as spaCy runs phrase patterns through the components before the ruler. `annotate(doc)` resolves to a new native document with the ruler's entities and leaves `doc` and its views unchanged; `findMatches(doc)` resolves to every candidate match before filtering, longest first, then earliest, with `id` `''` for a pattern without an ID. Both run on Node's worker pool under the same admission limits as the matchers; every other member is synchronous. The assertions below show `en_core_web_md` predictions.

```typescript
import assert from 'node:assert/strict';
import { EntityRuler, loadModel } from '@spars/node';
import type { EntityPattern } from '@spars/node';

// Load the model once. It is reused for every step below.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// Process a sentence with the full pipeline. The tokens (words and punctuation
// marks) are: Tim(0) Cook(1) opened(2) Apple(3) Park(4) in(5) Cupertino(6) .(7)
// The model's recognizer finds entities, which are named real-world things. Each
// has a label for its kind: PERSON, FAC (a building or facility), GPE (a country,
// city or similar place).
const doc = await nlp.processDocument('Tim Cook opened Apple Park in Cupertino.');

// Two patterns for the ruler, each with the label to give its matches.
const patterns: EntityPattern[] = [
  // A phrase pattern: the exact words "Apple Park" get the label FAC and the ID
  // 'apple-park'. The phrase is processed the same way as the document above.
  { label: 'FAC', id: 'apple-park', phrase: await nlp.processDocument('Apple Park') },
  // A token pattern: one token whose lowercase text is "cupertino" gets the label CITY.
  { label: 'CITY', tokens: { tokens: [{ constraints: [{ attribute: 'lower', predicate: { kind: 'equals', value: 'cupertino' } }], repetition: { kind: 'once' } }] } },
];

// A small helper that reduces each entity to its label and ID, to keep the
// checks below short.
const labelsAndIds = (entities: { label: string; id: string }[] | null) => entities?.map(({ label, id }) => [label, id]);

// The recognizer already labeled Apple Park FAC and Cupertino GPE. By default the ruler skips
// matches on tokens that already belong to an entity, so both matches are skipped.
const ruler = new EntityRuler();
ruler.add(patterns);

// The entities are still the model's own three, and none has an ID ('').
assert.deepEqual(labelsAndIds((await ruler.annotate(doc)).toObject().entities), [['PERSON', ''], ['FAC', ''], ['GPE', '']]);

// With overwriting, matches replace the entities they overlap.
const overwriting = new EntityRuler({ overwriteEntities: true });
overwriting.add(patterns);

// `annotate` returns a copy of the document with the ruler's entities added.
const annotated = await overwriting.annotate(doc);

// Apple Park carries the ID from its pattern, and Cupertino is labeled CITY
// instead of GPE. The PERSON entity matched no pattern, so it is unchanged.
assert.deepEqual(labelsAndIds(annotated.toObject().entities), [['PERSON', ''], ['FAC', 'apple-park'], ['CITY', '']]);

// The ID is also written to the entity's tokens. Token 3 is "Apple".
assert.equal(annotated.token(3).annotations().entityId, 'apple-park');

// The ruler can list the labels its patterns use.
assert.deepEqual(overwriting.labels, ['CITY', 'FAC']);

// Remove every pattern that has the ID 'apple-park'. No IDs are left afterwards.
overwriting.remove('apple-park');
assert.deepEqual(overwriting.ids, []);
```

The options are spaCy's settings: `overwriteEntities` (`overwrite_ents`, default `false`), `phraseAttribute` (`phrase_matcher_attr`, the [phrase attribute](#phrase-matching-on-text-lowercase-annotations-lexical-flags-or-length) phrases compare, default `'ORTH'`) and `idSeparator` (`ent_id_sep`, default `'||'`). Pass a model as the second argument, `new EntityRuler(options, nlp)`, to use lexical flag conditions or a lexical flag phrase attribute. The read-only properties `size`, `labels`, `ids` and `patterns` report the added patterns like spaCy's `len(ruler)`, `labels`, `ent_ids` and `patterns`, and `overwriteEntities`, `phraseAttribute` and `idSeparator` report the options. As with `PhraseMatcher`, attribute names must be uppercase, where spaCy also accepts lowercase. `patterns` lists phrases by their text and leaves `id` out for a pattern without one or with an empty one; as in spaCy, a token pattern added with `id: ''` is still listed by `ids` as `''` and removed by `remove('')`. `remove(id)` removes every pattern with that ID, and `clear()` removes every pattern and keeps the options and model. How matches become entities, how rule names combine labels and IDs, and the accepted differences from spaCy are described in the [EntityRuler guide](ENTITY_RULER.md).

`add` checks every pattern before changing the ruler. A pattern with an unknown field, with a `label` or `id` that is not a string, with both or neither of `tokens` and `phrase`, with a `phrase` that is not a native document, with an invalid token pattern, with a lexical flag condition or flag phrase attribute on a ruler created without a model, or with a phrase lacking the annotation the phrase attribute compares throws `SPARS_INVALID_PATTERN` and adds nothing. `remove` of an ID that was never added or whose patterns were already removed throws `SPARS_INVALID_PATTERN`. When the ruler is created, an empty `idSeparator` throws `SPARS_INVALID_PATTERN`, an unknown `phraseAttribute` `SPARS_UNSUPPORTED`, and an undeclared option, such as spaCy's own `overwrite_ents`, or a value of the wrong type throws `InvalidArg`. A search over a nonempty document in which no token has the annotation an `equals` token condition reads, such as a lemma condition on a tokenizer-only document, rejects with `SPARS_INFERENCE`. `add`, `remove` and `clear` throw `SPARS_BUSY` while a `findMatches` or `annotate` call on the same ruler is queued or running, and passing anything other than a native document to `findMatches` or `annotate` throws a `TypeError`. Loading or saving patterns, spaCy's pattern dictionaries, fuzzy matching and running the ruler before the recognizer are not provided.

## Add spans with rules

`SpanRuler` stores matches of token patterns and phrases as a [span group](#store-span-groups) and, optionally, as entities, like spaCy's `span_ruler`. Patterns have the same shape as for the [EntityRuler](#add-entities-with-rules): a `label`, an optional `id`, and either `tokens` or `phrase`. `annotate(doc)` resolves to a new native document and leaves `doc` and its views unchanged. `findMatches(doc)` resolves to every nonempty match before filtering, ordered by start and then end, with `id` `''` for a pattern without an ID; matches of different rules over the same tokens follow spaCy's string-hash order, which is not alphabetical. Both run on Node's worker pool under the same admission limits as the matchers; every other member is synchronous.

```typescript
import assert from 'node:assert/strict';
import { loadModel, SpanRuler } from '@spars/node';
import type { Document, SpanPattern } from '@spars/node';

// Load the model once. It is reused for every step below.
const nlp = await loadModel(process.env.SPARS_MODEL ?? 'en_core_web_lg');

// Split a sentence into tokens (words and punctuation marks). The 'Tokenizer' stage
// runs only that first step of the model. The tokens are:
// New(0) York(1) City(2) is(3) in(4) New(5) York(6) State(7) .(8)
const doc = await nlp.processDocument('New York City is in New York State.', 'Tokenizer');

// Two phrase patterns, both with the label GPE (a country, city or similar place).
// The first also has the ID 'nyc'. Each phrase is split the same way as the document.
const patterns: SpanPattern[] = [
  { label: 'GPE', id: 'nyc', phrase: await nlp.processDocument('New York City', 'Tokenizer') },
  { label: 'GPE', phrase: await nlp.processDocument('New York', 'Tokenizer') },
];

// Two small helpers to keep the checks below short. A span is a run of tokens in a
// row, and a span group is a named list of spans. `bounds` reduces each span to its
// start, its exclusive end and its ID. `group` finds the spans of one group by key.
const bounds = (spans: { start: number; end: number; id: string }[] | null | undefined) => spans?.map(({ start, end, id }) => [start, end, id]);
const group = (document: Document, key: string) => document.spanGroups.find(entry => entry.key === key)?.spans;

// By default every match goes into the 'ruler' group, including overlapping ones.
const ruler = new SpanRuler();
ruler.add(patterns);

// Three spans: "New York" (tokens 0 and 1), "New York City" (tokens 0 to 2, with
// the ID 'nyc') and the second "New York" (tokens 5 and 6). The first two overlap.
assert.deepEqual(bounds(group((await ruler.annotate(doc)).toObject(), 'ruler')), [[0, 2, ''], [0, 3, 'nyc'], [5, 7, '']]);

// An entity is a named real-world thing in the text. Unlike spans in a group,
// entities cannot overlap.
// With spansKey null the ruler writes only entities; the default entity filter keeps the longest match.
const entities = new SpanRuler({ spansKey: null, annotateEntities: true });
entities.add(patterns);
const annotated = (await entities.annotate(doc)).toObject();

// No span group was written.
assert.deepEqual(annotated.spanGroups, []);

// Where the two matches overlapped, only the longer "New York City" became an entity.
assert.deepEqual(bounds(annotated.entities), [[0, 3, 'nyc'], [5, 7, '']]);

// A span filter resolves overlaps in the group too.
const filtered = new SpanRuler({ spansKey: 'places', spansFilter: 'first_longest' });
filtered.add(patterns);

// This ruler stores its group under the key 'places', and the shorter overlapping
// "New York" span is gone.
assert.deepEqual(bounds(group((await filtered.annotate(doc)).toObject(), 'places')), [[0, 3, 'nyc'], [5, 7, '']]);

// Remove the pattern that was added with the ID 'nyc'. One pattern is left, it has
// no ID, and the ruler still knows the label GPE.
filtered.removeById('nyc');
assert.deepEqual([filtered.size, filtered.ids, filtered.contains('GPE')], [1, [], true]);
```

The options are spaCy's `span_ruler` settings with its factory defaults: `spansKey` (`spans_key`, default `'ruler'`; `null` or `''` writes no span group), `spansFilter` (`spans_filter`, default `null`, which keeps every match), `annotateEntities` (`annotate_ents`, default `false`), `entitiesFilter` (`ents_filter`, default `'first_longest'`), `phraseAttribute` (`phrase_matcher_attr`, default `'ORTH'`) and `overwrite` (default `true`). An omitted or `undefined` `spansKey` keeps the default; only `null` turns the group off. Filters, listed by the `SpanFilter` type, are short forms of spaCy's registered names, such as `'first_longest'` for `spacy.first_longest_spans_filter.v1`: `'first_longest'` keeps the longest non-overlapping spans, treating existing spans and matches alike; `'prioritize_new'` lets matches replace the existing spans they overlap; and `'prioritize_existing'` keeps every existing span and drops matches that overlap one. With the default `overwrite: true`, `annotate` replaces the group under `spansKey` and, with `annotateEntities`, discards the document's existing entities, including the recognizer's, before filtering; the EntityRuler keeps them by default. With `overwrite: false`, an existing group is extended and keeps its name and attrs, and existing entities compete with the matches. Pattern labels are typed `SpanLabel` and the entities written are `EntityLabel`, so under `StrictLabelSets` register a custom label in both `SpanLabels` and `EntityLabels` (see [types and autocomplete](#types-and-autocomplete)). Pass a model as the second argument, `new SpanRuler(options, nlp)`, to use lexical flag conditions or a lexical flag phrase attribute. The read-only properties report the options and the patterns: `size`, `labels`, `ids` and `patterns` behave like spaCy's `len(ruler)`, `labels`, `ids` and `patterns`, and `patterns` keeps each `id` exactly as added, including `''`. `contains(label)` reports whether a label was ever added, which stays true after its patterns are removed, as in spaCy. `remove(label)` removes a label's patterns, `removeById(id)` removes the patterns added with an ID, and `clear()` removes every pattern and keeps the options and model. The [SpanRuler guide](SPAN_RULER.md) describes the filters, overwriting, match order and the [differences from spaCy](SPAN_RULER.md#differences-from-spacy).

`add` checks every pattern before changing the ruler and throws `SPARS_INVALID_PATTERN` for a non-array argument and for the same malformed patterns as the EntityRuler's `add`; a failed `add` adds nothing. A label, ID or option string with an unpaired surrogate throws `SPARS_INVALID_TEXT`, and a `MORPH` phrase with noncanonical morphology `SPARS_UNSUPPORTED`. `remove` of a label that was never added, and `removeById` of an ID no current pattern has, throw `SPARS_INVALID_PATTERN`. When the ruler is created, an unknown filter or `phraseAttribute` throws `SPARS_UNSUPPORTED`, and an undeclared option, such as spaCy's own `spans_key`, or a value of the wrong type throws `InvalidArg`. Searches reject with `SPARS_INFERENCE` for an `equals` token condition on annotation the document lacks, as for the EntityRuler. `add`, `remove`, `removeById` and `clear` throw `SPARS_BUSY` while a `findMatches` or `annotate` call on the same ruler is queued or running, and passing anything other than a native document to `findMatches` or `annotate` throws a `TypeError`. Custom filter functions, loading or saving patterns, spaCy's pattern dictionaries and fuzzy matching are not provided.

## Errors and verification

An incompatible native addon can throw `SPARS_NATIVE_INCOMPATIBLE` during package import; remove stale native-library overrides or install matching package versions. Invalid matcher, EntityRuler or SpanRuler registration throws synchronously. Model and processing failures reject their promises with an `Error` containing a `code` and message. Codes are `SPARS_IO`, `SPARS_INVALID_MODEL`, `SPARS_UNSUPPORTED`, `SPARS_INVALID_TEXT`, `SPARS_BOUNDS`, `SPARS_INFERENCE`, `SPARS_INVALID_PATTERN`, `SPARS_BUSY`, `SPARS_INPUT_LIMIT`, and `SPARS_TEXT_TOO_LONG`. For `pipe`, errors reject iteration; `process` and `processBatch` throw immediately for invalid JavaScript argument types or unknown stages; when the queue is full, `SPARS_BUSY` takes precedence over checking individual batch elements. `vector` also throws immediately for malformed text. Use `loadModel`; constructing `Model` directly is unsupported and its TypeScript constructor is private.

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

Use `long` for the two long documents, `batch` or `concurrent` instead of `single`, and `Tokenizer` instead of `Ner` to measure tokenization alone. The argument `96` limits each batch to 96 documents; a smaller corpus produces a smaller group. For `concurrent`, replace `96` with `8` to measure groups of eight concurrent calls; configure different admission limits when measuring another server policy. Output separates model loading, awaited processing, peak process memory, submission time, and maximum timer gap. Awaited processing includes scheduling and result conversion; it is not a measurement of Rust inference alone. Repeat runs without competing workloads and record hardware, build mode, thread limits, and corpus identity as described in [performance measurements](PERFORMANCE.md).
