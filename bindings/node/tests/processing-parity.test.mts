import assert from 'node:assert/strict';
import { test } from 'node:test';
import { loadModel, configureExecution, configureInputLimits } from '../index.js';
import { modelPath } from './fixtures.mts';

test('loaded pipeline defaults and Python character counting match the pinned reference', async () => {
  const model = await loadModel(modelPath);
  assert.equal(model.maxLength, 1_000_000);
  assert.equal(model.batchSize, 256);
  model.maxLength = 2;
  assert.equal((await model.process('😀😀', 'Tokenizer')).text, '😀😀');
  await assert.rejects(model.process('😀😀a', 'Tokenizer'), { code: 'SPARS_TEXT_TOO_LONG' });
});

test('default processing accepts more than the former server caps', async () => {
  const model = await loadModel(modelPath);
  assert.equal((await model.process('word '.repeat(13_108), 'Tokenizer')).text.length, 65_540);
  assert.equal((await model.processBatch(Array(257).fill(''), 'Tokenizer')).length, 257);
  assert.equal((await Promise.all(Array.from({length: 40}, () => model.process('', 'Tokenizer')))).length, 40);
});


test('length limits count code points, allow equality and zero, and remain model-local', async () => {
  const model = await loadModel(modelPath);
  const other = await loadModel(modelPath);
  model.maxLength = 2;
  for (const text of ['😀😀', 'e\u0301', 'a\0', '', 'ab']) {
    assert.equal((await model.process(text, 'Tokenizer')).text, text);
  }
  for (const text of ['😀😀a', 'e\u0301a', 'a\0b', 'abc']) {
    await assert.rejects(model.process(text, 'Tokenizer'), { code: 'SPARS_TEXT_TOO_LONG', message: /\[E088\]/ });
    await assert.rejects(model.processBatch(['ok', text], 'Tokenizer'), { code: 'SPARS_TEXT_TOO_LONG' });
  }
  model.maxLength = 0;
  assert.equal((await model.process('', 'Tokenizer')).text, '');
  await assert.rejects(model.process('a', 'Tokenizer'), { code: 'SPARS_TEXT_TOO_LONG' });
  assert.equal(other.maxLength, 1_000_000);
  // The default limit rejects before tokenization (which would be expensive here).
  await assert.rejects(other.process('a'.repeat(1_000_001), 'Tokenizer'), { code: 'SPARS_TEXT_TOO_LONG' });
});

test('pipeline settings validate atomically and cannot change during pending work', async () => {
  const model = await loadModel(modelPath);
  for (const name of ['maxLength', 'batchSize']) {
    const original = Reflect.get(model, name);
    for (const bad of [-1, 0.5, NaN, Infinity, 2 ** 32, null, undefined, '2', true, false]) {
      assert.throws(() => Reflect.set(model, name, bad));
      assert.equal(Reflect.get(model, name), original);
    }
  }
  assert.throws(() => { model.batchSize = 0; });
  model.batchSize = 1;
  model.maxLength = 3;
  configureExecution({ maxActive: 1, maxQueued: 1 });
  const first = model.processBatch(['abc', 'def'], 'Tokenizer');
  const second = model.process('ghi', 'Tokenizer');
  try {
    assert.throws(() => { model.maxLength = 100; }, /idle/);
    assert.throws(() => { model.batchSize = 100; }, /idle/);
    assert.throws(() => configureExecution(null), /idle/);
    assert.throws(() => configureInputLimits(null), /idle/);
    assert.deepEqual((await first).map(doc => doc.text), ['abc', 'def']);
    assert.equal((await second).text, 'ghi');
  } finally {
    await Promise.allSettled([first, second]);
    configureExecution(null);
  }
  model.maxLength = 4;
  assert.equal((await model.process('abcd', 'Tokenizer')).text, 'abcd');
});

test('pipe is lazy, buffers only a batch, preserves ordering, and closes sources on return', async () => {
  const model = await loadModel(modelPath);
  model.batchSize = 2;
  let consumed = 0;
  let closed = false;
  async function* source(): AsyncGenerator<string, void, unknown> {
    try {
      for (const text of ['a', 'b', 'c', 'd', 'e']) { consumed++; yield text; }
    } finally { closed = true; }
  }
  const docs = model.pipe(source(), { stage: 'Tokenizer' });
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
  const piped: Awaited<ReturnType<typeof model.process>>[] = [];
  for await (const doc of model.pipe(texts, { batchSize: 3 })) piped.push(doc);
  assert.deepEqual(piped, await model.processBatch(texts));
  const empty = model.pipe([]);
  assert.equal((await empty.next()).done, true);
});

test('pipe resolves its initial batch size lazily, and reports native and source errors', async () => {
  const model = await loadModel(modelPath);
  let consumed = 0;
  function* source(): Generator<string, void, unknown> {
    for (const text of ['a', 'b', 'c']) { consumed++; yield text; }
  }
  const docs = model.pipe(source(), { stage: 'Tokenizer' });
  model.batchSize = 1;
  assert.equal((await docs.next()).value?.text, 'a');
  assert.equal(consumed, 1);
  model.batchSize = 3;
  assert.equal((await docs.next()).value?.text, 'b');
  assert.equal(consumed, 2, 'pipe retains its initial buffering size');
  model.maxLength = 0;
  await assert.rejects(docs.next(), { code: 'SPARS_TEXT_TOO_LONG' });
  model.maxLength = 1;
  function* broken(): Generator<string, void, unknown> {
    yield 'a';
    throw new Error('source failed');
  }
  const brokenDocs = model.pipe(broken(), { batchSize: 1, stage: 'Tokenizer' });
  assert.equal((await brokenDocs.next()).value?.text, 'a');
  await assert.rejects(brokenDocs.next(), /source failed/);
  const invalid = model.pipe(['a', '\ud800'], { batchSize: 2, stage: 'Tokenizer' });
  await assert.rejects(invalid.next(), { code: 'SPARS_INVALID_TEXT' });
  assert.equal((await model.process('a', 'Tokenizer')).text, 'a');
});

test('pipe rejects malformed options and input, and honors opt-in server limits', async () => {
  const model = await loadModel(modelPath);
  for (const options of [null, [], { batchSize: 0 }, { batchSize: 1.5 }, { batchSize: 2 ** 32 }, { stage: 'bad' }, { nProcess: 2 }]) {
    const iterator: AsyncGenerator<unknown, void, unknown> = Reflect.apply(model.pipe, model, [[], options]);
    await assert.rejects(iterator.next());
  }
  for (const input of [42, null, ['ok', 3]]) {
    const iterator: AsyncGenerator<unknown, void, unknown> = Reflect.apply(model.pipe, model, [input]);
    await assert.rejects(iterator.next());
  }
  configureInputLimits({ maxTextLength: 4, maxBatchSize: 1, maxBatchTextLength: 4 });
  try {
    await assert.rejects(model.pipe(['a', 'b'], { batchSize: 2 }).next(), { code: 'SPARS_INPUT_LIMIT' });
    await assert.rejects(model.process('😀😀a', 'Tokenizer'), { code: 'SPARS_INPUT_LIMIT' });
  } finally { configureInputLimits(null); }
  model.maxLength = 2;
  assert.equal((await model.process('😀😀', 'Tokenizer')).text, '😀😀');
  await assert.rejects(model.process('abc', 'Tokenizer'), { code: 'SPARS_TEXT_TOO_LONG' });
});
