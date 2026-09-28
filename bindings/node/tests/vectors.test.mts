import assert from 'node:assert/strict';
import { before, test } from 'node:test';
import { loadModel, Model, NativeDocument } from '../index.js';
import { array, modelPath, readJson, record, root } from './fixtures.mts';

let model: Model;
before(async () => { model = await loadModel(modelPath); });

test('contextual similarity checks dimensions after identity and zero-vector shortcuts', async () => {
  const small = await loadModel(`${root}assets/en_core_web_sm-3.8.0`);
  const make = (text: string, row: number[]): NativeDocument => NativeDocument.fromSnapshot(JSON.stringify({
    format_version: 2, document: { text, tokens: [{ start: 0, end: text.length, idx: 0, whitespace: false, norm: text }], tensor: [row] },
  }));
  const a = make('a', [1, 0]);
  const b = make('b', [1, 0, 1]);
  assert.throws(() => small.similarity(a, b), { code: 'SPARS_UNSUPPORTED' });
  assert.throws(() => small.spanSimilarity(a, 0, 1, b, 0, 1), { code: 'SPARS_UNSUPPORTED' });
  assert.equal(small.similarity(a, make('a', [1, 0, 1])), 1);
  const zero = make('b', [0, 0, 0]);
  assert.equal(small.similarity(a, zero), 0);
  assert.equal(small.spanSimilarity(a, 0, 1, zero, 0, 1), 0);
  const empty = await small.processDocument('', 'Tokenizer');
  assert.equal(small.similarity(empty, a), 0);
  assert.equal(small.similarity(empty, empty), 1);
});
function string(value: unknown): string { assert.ok(typeof value === 'string'); return value; }
function number(value: unknown): number { assert.ok(typeof value === 'number' && Number.isFinite(value)); return value; }
function vector(value: unknown): number[] { return array(value).map(number); }
function close(actual: Float32Array, expected: number[]): void {
  assert.ok(actual instanceof Float32Array);
  assert.equal(actual.length, expected.length);
  for (const [i, want] of expected.entries()) {
    const got = actual[i];
    assert.ok(got !== undefined && Number.isFinite(got));
    assert.ok(Math.abs(got - want) <= 2e-6 + 2e-6 * Math.abs(want), `vector ${i}: ${got} != ${want}`);
  }
}

test('document and span similarities match frozen official reference', async () => {
  const fixture = record(readJson(`${root}fixtures/vectors.expected.json`));
  for (const value of array(fixture.similarities)) {
    const entry = record(value);
    const a = await model.processDocument(string(entry.a), 'Tokenizer');
    const b = await model.processDocument(string(entry.b), 'Tokenizer');
    const expected = number(entry.expected);
    assert.ok(Math.abs(model.similarity(a, b) - expected) < 2e-6);
    assert.ok(Math.abs(model.spanSimilarity(a, 0, a.toObject().tokens.length,
      b, 0, b.toObject().tokens.length) - expected) < 2e-6);
  }
});

test('static token and singleton document/span vectors agree with frozen word vectors', () => {
  const fixture = record(readJson(`${root}fixtures/vectors.expected.json`));
  for (const value of array(fixture.words)) {
    const entry = record(value);
    const text = string(entry.text);
    if (text.length === 0) continue;
    const doc = NativeDocument.fromSnapshot(JSON.stringify({ format_version: 2, document: {
      text, tokens: [{ start: 0, end: Buffer.byteLength(text), idx: 0, whitespace: false, norm: text }],
      entities: null, sentences: null, noun_chunks: null, tensor: [],
    } }));
    const actual = model.tokenVector(doc, 0);
    if (entry.has_vector === false) { assert.equal(actual, null); continue; }
    assert.equal(entry.has_vector, true);
    assert.ok(actual);
    close(actual, vector(entry.vector));
    close(model.documentVector(doc), vector(entry.vector));
    close(model.spanVector(doc, 0, 1), vector(entry.vector));
    actual.fill(42);
    const repeated = model.tokenVector(doc, 0);
    assert.ok(repeated);
    close(repeated, vector(entry.vector));
  }
});

test('small-model contextual vectors and restored snapshots match independent reference', async () => {
  const small = await loadModel(`${root}assets/en_core_web_sm-3.8.0`);
  const fixture = record(readJson(`${root}fixtures/model-capabilities-v1.expected.json`));
  assert.equal(fixture.vector_model, 'en_core_web_sm 3.8.0');
  const cases = array(fixture.vector_cases);
  assert.equal(cases.length, 4);
  for (const value of cases) {
    const entry = record(value);
    const doc = await small.processDocument(string(entry.text));
    const tokens = array(entry.tokens);
    for (const [i, expected] of tokens.entries()) {
      const actual = small.tokenVector(doc, i);
      assert.ok(actual);
      close(actual, vector(expected));
      actual.fill(42);
      const repeated = small.tokenVector(doc, i);
      assert.ok(repeated);
      close(repeated, vector(expected));
    }
    close(small.documentVector(doc), vector(entry.document));
    close(small.spanVector(doc, Math.min(1, tokens.length), Math.min(3, tokens.length)), vector(entry.span));
    close(small.spanVector(doc, 0, 0), vector(entry.empty_span));
    const snapshot = doc.toSnapshot();
    const restored = NativeDocument.fromSnapshot(snapshot);
    assert.equal(restored.toSnapshot(), snapshot);
    close(small.documentVector(restored), vector(entry.document));
    assert.equal(small.similarity(doc, restored), 1);
    const copy = small.documentVector(doc);
    copy.fill(42);
    close(small.documentVector(doc), vector(entry.document));
  }
  const raw = await small.processDocument('hello world', 'Tokenizer');
  assert.equal(small.tokenVector(raw, 0), null);
  close(small.documentVector(raw), []);
  close(small.spanVector(raw, 0, 2), []);
  assert.equal(small.similarity(raw, await small.processDocument('different', 'Tokenizer')), 0);
});

test('indices reject coercion, unsafe integers and invalid intervals', async () => {
  const doc = await model.processDocument('dog cat', 'Tokenizer');
  for (const index of [-1, 0.5, NaN, Infinity, 2 ** 32, Number.MAX_SAFE_INTEGER + 1]) {
    assert.throws(() => model.tokenVector(doc, index));
    assert.throws(() => model.spanVector(doc, index, 2));
    assert.throws(() => model.spanVector(doc, 0, index));
    assert.throws(() => model.spanSimilarity(doc, index, 2, doc, 0, 2));
    assert.throws(() => model.spanSimilarity(doc, 0, 2, doc, 0, index));
  }
  assert.throws(() => model.tokenVector(doc, 2), { code: 'SPARS_BOUNDS' });
  assert.throws(() => model.spanVector(doc, 2, 1), { code: 'SPARS_BOUNDS' });
  for (const index of ['0', null, undefined, true]) {
    assert.throws(() => Reflect.apply(model.tokenVector, model, [doc, index]));
  }
  assert.equal(model.spanSimilarity(doc, 0, 0, doc, 1, 1), 1);
});

test('receiving model selects static vectors, while small models consume document tensors', async () => {
  const small = await loadModel(`${root}assets/en_core_web_sm-3.8.0`);
  const doc = await small.processDocument('dog');
  const staticVector = model.vector('dog');
  assert.ok(staticVector);
  close(model.documentVector(doc), Array.from(staticVector));
  const synthetic = NativeDocument.fromSnapshot(JSON.stringify({ format_version: 2, document: {
    text: 'dog', tokens: [{ start: 0, end: 3, idx: 0, whitespace: false, norm: 'dog' }],
    entities: null, sentences: null, noun_chunks: null, tensor: [[3, 4]],
  } }));
  close(small.documentVector(synthetic), [3, 4]);
  close(model.documentVector(synthetic), Array.from(staticVector));
});

test('large-model document vectors match the frozen reference', async () => {
  const large = await loadModel(`${root}assets/en_core_web_lg-3.8.0`);
  const fixture = record(readJson(`${root}fixtures/model-lg-v1.expected.json`));
  assert.equal(fixture.model, 'en_core_web_lg 3.8.0');
  const cases = array(fixture.cases);
  assert.equal(cases.length, 98);
  for (const value of cases) {
    const entry = record(value);
    const doc = await large.processDocument(string(entry.text), 'Tokenizer');
    close(large.documentVector(doc), vector(entry.vector));
  }
});
