import assert from 'node:assert/strict';
import { test } from 'node:test';
import { loadModel, configureExecution, configureInputLimits } from '../index.js';
import { modelPath } from './fixtures.mts';

test('loaded pipeline defaults and Python character counting match the pinned reference', async () => {
  const nlp = await loadModel(modelPath);
  assert.equal(nlp.maxLength, 1_000_000);
  assert.equal(nlp.batchSize, 256);
  nlp.maxLength = 2;
  assert.equal((await nlp.process('😀😀', 'Tokenizer')).text, '😀😀');
  await assert.rejects(nlp.process('😀😀a', 'Tokenizer'), { code: 'SPARS_TEXT_TOO_LONG' });
});

test('default processing accepts more than the former input caps', async () => {
  const nlp = await loadModel(modelPath);
  assert.equal((await nlp.process('word '.repeat(13_108), 'Tokenizer')).text.length, 65_540);
  assert.equal((await nlp.processBatch(Array(257).fill(''), 'Tokenizer')).length, 257);
  const accepted = Array.from({ length: 34 }, () => nlp.process('', 'Tokenizer'));
  const settled = Promise.allSettled(accepted);
  await assert.rejects(nlp.process('', 'Tokenizer'), { code: 'SPARS_BUSY' });
  assert.ok((await settled).every(result => result.status === 'fulfilled'));
});


test('length limits count code points, allow equality and zero, and remain model-local', async () => {
  const nlp = await loadModel(modelPath);
  const other = await loadModel(modelPath);
  nlp.maxLength = 2;
  for (const text of ['😀😀', 'e\u0301', 'a\0', '', 'ab']) {
    assert.equal((await nlp.process(text, 'Tokenizer')).text, text);
  }
  for (const text of ['😀😀a', 'e\u0301a', 'a\0b', 'abc']) {
    await assert.rejects(nlp.process(text, 'Tokenizer'), { code: 'SPARS_TEXT_TOO_LONG', message: /\[E088\]/ });
    await assert.rejects(nlp.processBatch(['ok', text], 'Tokenizer'), { code: 'SPARS_TEXT_TOO_LONG' });
  }
  nlp.maxLength = 0;
  assert.equal((await nlp.process('', 'Tokenizer')).text, '');
  await assert.rejects(nlp.process('a', 'Tokenizer'), { code: 'SPARS_TEXT_TOO_LONG' });
  assert.equal(other.maxLength, 1_000_000);
  // The default limit rejects before tokenization (which would be expensive here).
  await assert.rejects(other.process('a'.repeat(1_000_001), 'Tokenizer'), { code: 'SPARS_TEXT_TOO_LONG' });
});

test('pipeline settings validate atomically and cannot change during pending work', async () => {
  const nlp = await loadModel(modelPath);
  for (const name of ['maxLength', 'batchSize']) {
    const original = Reflect.get(nlp, name);
    for (const bad of [-1, 0.5, NaN, Infinity, 2 ** 32, null, undefined, '2', true, false]) {
      assert.throws(() => Reflect.set(nlp, name, bad));
      assert.equal(Reflect.get(nlp, name), original);
    }
  }
  assert.throws(() => { nlp.batchSize = 0; });
  nlp.batchSize = 1;
  nlp.maxLength = 3;
  configureExecution({ maxActive: 1, maxQueued: 1 });
  const first = nlp.processBatch(['abc', 'def'], 'Tokenizer');
  const second = nlp.process('ghi', 'Tokenizer');
  try {
    assert.throws(() => { nlp.maxLength = 100; }, /idle/);
    assert.throws(() => { nlp.batchSize = 100; }, /idle/);
    assert.throws(() => configureExecution(null), /idle/);
    assert.throws(() => configureInputLimits(null), /idle/);
    assert.deepEqual((await first).map(doc => doc.text), ['abc', 'def']);
    assert.equal((await second).text, 'ghi');
  } finally {
    await Promise.allSettled([first, second]);
    configureExecution(null);
  }
  nlp.maxLength = 4;
  assert.equal((await nlp.process('abcd', 'Tokenizer')).text, 'abcd');
});

test('pipe is lazy, buffers only a batch, preserves ordering, and closes sources on return', async () => {
  const nlp = await loadModel(modelPath);
  nlp.batchSize = 2;
  let consumed = 0;
  let closed = false;
  async function* source(): AsyncGenerator<string, void, unknown> {
    try {
      for (const text of ['a', 'b', 'c', 'd', 'e']) { consumed++; yield text; }
    } finally { closed = true; }
  }
  const docs = nlp.pipe(source(), { stage: 'Tokenizer' });
  assert.equal(consumed, 0);
  assert.equal((await docs.next()).value?.text, 'a');
  assert.equal(consumed, 2);
  assert.equal((await docs.next()).value?.text, 'b');
  assert.equal(consumed, 2);
  assert.equal((await docs.next()).value?.text, 'c');
  assert.equal(consumed, 4);
  await docs.return();
  assert.equal(closed, true);
  assert.equal(consumed, 4);
  const texts = ['Alice works in London.', '', '😀 Café.', 'Dogs run.', 'Done.'];
  const piped: Awaited<ReturnType<typeof nlp.process>>[] = [];
  for await (const doc of nlp.pipe(texts, { batchSize: 3 })) piped.push(doc);
  assert.deepEqual(piped, await nlp.processBatch(texts));
  const empty = nlp.pipe([]);
  assert.equal((await empty.next()).done, true);
});

test('pipe resolves its initial batch size lazily, and reports native and source errors', async () => {
  const nlp = await loadModel(modelPath);
  let consumed = 0;
  function* source(): Generator<string, void, unknown> {
    for (const text of ['a', 'b', 'c']) { consumed++; yield text; }
  }
  const docs = nlp.pipe(source(), { stage: 'Tokenizer' });
  nlp.batchSize = 1;
  assert.equal((await docs.next()).value?.text, 'a');
  assert.equal(consumed, 1);
  nlp.batchSize = 3;
  assert.equal((await docs.next()).value?.text, 'b');
  assert.equal(consumed, 2, 'pipe retains its initial buffering size');
  nlp.maxLength = 0;
  await assert.rejects(docs.next(), { code: 'SPARS_TEXT_TOO_LONG' });
  nlp.maxLength = 1;
  function* broken(): Generator<string, void, unknown> {
    yield 'a';
    throw new Error('source failed');
  }
  const brokenDocs = nlp.pipe(broken(), { batchSize: 1, stage: 'Tokenizer' });
  assert.equal((await brokenDocs.next()).value?.text, 'a');
  await assert.rejects(brokenDocs.next(), /source failed/);
  const invalid = nlp.pipe(['a', '\ud800'], { batchSize: 2, stage: 'Tokenizer' });
  await assert.rejects(invalid.next(), { code: 'SPARS_INVALID_TEXT' });
  assert.equal((await nlp.process('a', 'Tokenizer')).text, 'a');
});

test('pipe rejects malformed options and input, and honors opt-in server limits', async () => {
  const nlp = await loadModel(modelPath);
  for (const options of [null, [], { batchSize: 0 }, { batchSize: 1.5 }, { batchSize: 2 ** 32 }, { stage: 'bad' }, { nProcess: 2 }]) {
    const iterator: AsyncGenerator<unknown, void, unknown> = Reflect.apply(nlp.pipe, nlp, [[], options]);
    await assert.rejects(iterator.next());
  }
  for (const input of [42, null, ['ok', 3]]) {
    const iterator: AsyncGenerator<unknown, void, unknown> = Reflect.apply(nlp.pipe, nlp, [input]);
    await assert.rejects(iterator.next());
  }
  configureInputLimits({ maxTextLength: 4, maxBatchSize: 1, maxBatchTextLength: 4 });
  try {
    await assert.rejects(nlp.pipe(['a', 'b'], { batchSize: 2 }).next(), { code: 'SPARS_INPUT_LIMIT' });
    await assert.rejects(nlp.process('😀😀a', 'Tokenizer'), { code: 'SPARS_INPUT_LIMIT' });
  } finally { configureInputLimits(null); }
  nlp.maxLength = 2;
  assert.equal((await nlp.process('😀😀', 'Tokenizer')).text, '😀😀');
  await assert.rejects(nlp.process('abc', 'Tokenizer'), { code: 'SPARS_TEXT_TOO_LONG' });
});
