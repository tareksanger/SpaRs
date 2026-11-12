import assert from 'node:assert/strict';
import { test } from 'node:test';
import { configureInputLimits, configureExecution, loadModel } from '../index.js';
import { modelPath } from './fixtures.mts';

const serverLimits = { maxTextLength: 32_768, maxBatchSize: 128, maxBatchTextLength: 65_536 };

test('input limits use UTF-16 units and enforce exact text, count and aggregate boundaries', async () => {
  const nlp = await loadModel(modelPath);
  configureInputLimits({ maxTextLength: 4, maxBatchSize: 2, maxBatchTextLength: 6 });
  try {
    assert.equal((await nlp.process('😀😀', 'Tokenizer')).text, '😀😀');
    await assert.rejects(nlp.process('😀😀a'), { code: 'SPARS_INPUT_LIMIT' });
    assert.equal((await nlp.processBatch(['abcd', 'ef'], 'Tokenizer')).length, 2);
    assert.equal((await nlp.processBatch(['😀😀', '😀'], 'Tokenizer')).length, 2);
    await assert.rejects(nlp.processBatch(['😀😀', '😀a']), { code: 'SPARS_INPUT_LIMIT' });
    await assert.rejects(nlp.processBatch(['😀😀a']), { code: 'SPARS_INPUT_LIMIT' });
    await assert.rejects(nlp.processBatch(['abcd', 'efg']), { code: 'SPARS_INPUT_LIMIT' });
    await assert.rejects(nlp.processBatch(['abcde']), { code: 'SPARS_INPUT_LIMIT' });
    await assert.rejects(nlp.processBatch(['', '', '']), { code: 'SPARS_INPUT_LIMIT' });
    assert.deepEqual(await nlp.processBatch([]), []);
    assert.equal((await nlp.process('')).text, '');
    await assert.rejects(nlp.process('\ud800'), { code: 'SPARS_INVALID_TEXT' });
    assert.throws(() => Reflect.apply(nlp.process, nlp, [42]));
    assert.throws(() => Reflect.apply(nlp.processBatch, nlp, [[null]]));
  } finally { configureInputLimits(null); }
});

test('oversize batches are rejected before elements are read or an admission slot is taken', async () => {
  const nlp = await loadModel(modelPath);
  configureInputLimits({ maxTextLength: 4, maxBatchSize: 2, maxBatchTextLength: 6 });
  configureExecution({ maxActive: 1, maxQueued: 0 });
  try {
    const oversized = ['', '', ''];
    Object.defineProperty(oversized, 0, { get() { throw new Error('must not read an oversized batch'); } });
    await assert.rejects(nlp.processBatch(oversized), { code: 'SPARS_INPUT_LIMIT' });
    const rejected = nlp.process('12345');
    const accepted = nlp.process('ok');
    await assert.rejects(rejected, { code: 'SPARS_INPUT_LIMIT' });
    assert.throws(() => configureInputLimits(null), /idle/);
    assert.equal((await accepted).text, 'ok');
    assert.equal((await nlp.processBatch(['ok']))[0]?.text, 'ok');
  } finally {
    configureExecution(null);
    configureInputLimits(null);
  }
});

test('invalid input-limit configuration is rejected atomically and caller mutation is isolated', async () => {
  const nlp = await loadModel(modelPath);
  const options = { maxTextLength: 4, maxBatchSize: 2, maxBatchTextLength: 6 };
  configureInputLimits(options);
  options.maxTextLength = 100;
  try {
    for (const bad of [[], {}, { maxTextLength: 0, maxBatchSize: 2, maxBatchTextLength: 6 },
      { maxTextLength: 4, maxBatchSize: -1, maxBatchTextLength: 6 },
      { maxTextLength: 4, maxBatchSize: 1.5, maxBatchTextLength: 6 },
      { maxTextLength: 4, maxBatchSize: 2, maxBatchTextLength: Infinity },
      { maxTextLength: '4', maxBatchSize: 2, maxBatchTextLength: 6 },
      { maxTextLength: 4, maxBatchSize: 2, maxBatchTextLength: Number.MAX_SAFE_INTEGER + 1 }]) {
      assert.throws(() => Reflect.apply(configureInputLimits, undefined, [bad]));
      await assert.rejects(nlp.process('12345'), { code: 'SPARS_INPUT_LIMIT' });
      assert.equal((await nlp.process('1234', 'Tokenizer')).text, '1234');
    }
  } finally { configureInputLimits(null); }
});

test('explicit server limits reject extreme workloads', async () => {
  configureInputLimits(serverLimits);
  const nlp = await loadModel(modelPath);
  await assert.rejects(nlp.process('x'.repeat(serverLimits.maxTextLength + 1)), { code: 'SPARS_INPUT_LIMIT' });
  await assert.rejects(nlp.processBatch(Array(serverLimits.maxBatchSize + 1).fill('')), { code: 'SPARS_INPUT_LIMIT' });
  await assert.rejects(nlp.processBatch(Array(3).fill('x'.repeat(serverLimits.maxTextLength))), { code: 'SPARS_INPUT_LIMIT' });
  assert.equal((await nlp.process('still usable')).text, 'still usable');
  configureInputLimits(null);
});


test('reset removes each server cap while preserving the pipeline length limit', async () => {
  const nlp = await loadModel(modelPath);
  nlp.maxLength = 8;
  configureInputLimits({ maxTextLength: 4, maxBatchSize: 2, maxBatchTextLength: 6 });
  try {
    await assert.rejects(nlp.process('12345'), { code: 'SPARS_INPUT_LIMIT' });
    await assert.rejects(nlp.processBatch(['a', 'b', 'c']), { code: 'SPARS_INPUT_LIMIT' });
    await assert.rejects(nlp.processBatch(['1234', '5678']), { code: 'SPARS_INPUT_LIMIT' });
    configureInputLimits(null);
    assert.equal((await nlp.process('12345', 'Tokenizer')).text, '12345');
    assert.equal((await nlp.processBatch(['a', 'b', 'c'], 'Tokenizer')).length, 3);
    assert.equal((await nlp.processBatch(['1234', '5678'], 'Tokenizer')).length, 2);
    await assert.rejects(nlp.process('123456789'), { code: 'SPARS_TEXT_TOO_LONG' });
  } finally { configureInputLimits(null); }
});
