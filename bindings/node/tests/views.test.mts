import assert from 'node:assert/strict';
import { before, test } from 'node:test';
import { loadModel, Model, NativeDocument } from '../index.js';
import { array, modelPath, readJson, record, root } from './fixtures.mts';

let model: Model;
before(async () => { model = await loadModel(modelPath); });

function number(value: unknown): number { assert.equal(typeof value, 'number'); assert.ok(typeof value === 'number'); return value; }
function string(value: unknown): string { assert.ok(typeof value === 'string'); return value; }

test('all frozen dependency traversals match pinned spaCy order and sentence bounds', async () => {
  const fixture = record(readJson(`${root}fixtures/traversal-v1.expected.json`));
  assert.deepEqual(fixture.versions, { spacy: '3.8.14', thinc: '8.3.13' });
  assert.equal(fixture.model, 'en_core_web_md 3.8.0');
  const cases = array(fixture.cases);
  assert.equal(cases.length, 98);
  let count = 0;
  for (const value of cases) {
    const expected = record(value);
    const doc = await model.processDocument(string(expected.text));
    const saved = doc.toSnapshot();
    const tokens = array(expected.tokens);
    assert.equal(doc.length, tokens.length);
    for (const value of tokens) {
      const expectedToken = record(value);
      const token = doc.token(number(expectedToken.index));
      assert.deepEqual(token.children().map(t => t.index), expectedToken.children);
      assert.deepEqual(token.ancestors().map(t => t.index), expectedToken.ancestors);
      assert.deepEqual(token.subtree().map(t => t.index), expectedToken.subtree);
      const sentence = token.sentence();
      assert.deepEqual([sentence.start, sentence.end], expectedToken.sentence);
      count++;
    }
    assert.equal(doc.toSnapshot(), saved);
    assert.deepEqual(doc.sentenceViews()?.map(s => [s.start, s.end]), doc.toObject().sentences?.map(s => [s.start, s.end]));
  }
  assert.equal(count, 5568);
});

test('views preserve text, annotations and lifetime without permitting mutation', async () => {
  let doc: NativeDocument | undefined = await model.processDocument('😀 Café e\u0301.  ');
  const output = doc.toObject();
  assert.equal(doc.text, output.text);
  assert.deepEqual(doc.tokens().map(t => t.text), output.tokens.map(t => t.text));
  const token = doc.token(1);
  const span = doc.span(0, doc.length);
  const expected = output.tokens[1];
  assert.ok(expected);
  const { index: ignoredIndex, text: ignoredText, codePointEnd: ignoredEnd, utf16Start: ignoredStart16, utf16End: ignoredEnd16, ...annotations } = expected;
  assert.deepEqual(token.annotations(), annotations);
  const changed = token.annotations();
  changed.norm = 'changed';
  assert.deepEqual(token.annotations(), annotations);
  assert.equal(token.head()?.index, expected.head);
  assert.equal(token.span().text, token.text);
  assert.equal(token.span().start, token.index);
  assert.equal(token.span().end, token.index + 1);
  doc = undefined;
  global.gc?.();
  assert.equal(token.text, expected.text);
  assert.deepEqual(span.tokens().map(t => t.text), output.tokens.map(t => t.text));
  assert.equal(span.text, output.text.slice(0, output.tokens.at(-1)?.utf16End));
});

test('bounds reject fractions, negative numbers, wrapping, infinities and NaN', async () => {
  const doc = await model.processDocument('A B', 'Tokenizer');
  for (const invalid of [-1, 0.5, Number.NaN, Infinity, -Infinity, 2 ** 32, Number.MAX_SAFE_INTEGER]) {
    assert.throws(() => doc.token(invalid), { code: 'SPARS_BOUNDS' });
    assert.throws(() => doc.span(invalid, 1), { code: 'SPARS_BOUNDS' });
    assert.throws(() => doc.span(0, invalid), { code: 'SPARS_BOUNDS' });
  }
  assert.throws(() => doc.token(doc.length), { code: 'SPARS_BOUNDS' });
  assert.throws(() => doc.span(1, 0), { code: 'SPARS_BOUNDS' });
  assert.throws(() => doc.span(0, doc.length + 1), { code: 'SPARS_BOUNDS' });
  assert.equal(doc.span(doc.length, doc.length).text, '');
  assert.deepEqual(doc.span(1, 1).tokens(), []);
});

test('missing annotations remain distinct from empty results', async () => {
  const doc = await model.processDocument('A B', 'Tokenizer');
  assert.equal(doc.sentenceViews(), null);
  assert.equal(doc.token(0).head(), null);
  assert.throws(() => doc.token(0).sentence());
  for (const traversal of ['children', 'ancestors', 'subtree'] as const) {
    assert.throws(() => doc.token(0)[traversal]());
  }
  const empty = await model.processDocument('');
  assert.deepEqual(empty.sentenceViews(), []);
  assert.deepEqual(empty.tokens(), []);
  assert.equal(empty.span(0, 0).text, '');
  assert.throws(() => empty.token(0));
});

test('restored nonprojective and cyclic graphs preserve native traversal contracts', async () => {
  const original = await model.processDocument('A B C D', 'Tokenizer');
  function withHeads(heads: number[]): NativeDocument {
    const saved = record(JSON.parse(original.toSnapshot()));
    const tokens = array(record(saved.document).tokens);
    assert.equal(tokens.length, heads.length);
    tokens.forEach((token, i) => { record(token).head = heads[i]; });
    return NativeDocument.fromSnapshot(JSON.stringify(saved));
  }
  const crossing = withHeads([2, 2, 2, 0]);
  assert.deepEqual(crossing.token(2).subtree().map(t => t.index), [0, 3, 1, 2]);
  assert.deepEqual(crossing.token(2).ancestors(), []);
  assert.equal(crossing.token(2).head()?.index, 2);
  const cycle = withHeads([1, 0, 2, 2]);
  for (const traversal of ['children', 'ancestors', 'subtree'] as const) {
    assert.throws(() => cycle.token(2)[traversal](), /cycle/);
    assert.throws(() => cycle.token(2)[traversal](), /cycle/);
  }
});
