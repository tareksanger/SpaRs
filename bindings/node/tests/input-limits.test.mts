import assert from 'node:assert/strict';
import { test } from 'node:test';
import { configureInputLimits, configureExecution, loadModel } from '../index.js';
import { modelPath } from './fixtures.mts';

const defaults = { maxTextLength: 32_768, maxBatchSize: 128, maxBatchTextLength: 65_536 };

test('input limits use UTF-16 units and enforce exact text, count and aggregate boundaries', async () => {
  const model = await loadModel(modelPath);
  configureInputLimits({ maxTextLength: 4, maxBatchSize: 2, maxBatchTextLength: 6 });
  try {
    assert.equal((await model.process('😀😀', 'Tokenizer')).text, '😀😀');
    await assert.rejects(model.process('😀😀a'), { code: 'SPARS_INPUT_LIMIT' });
    assert.equal((await model.processBatch(['abcd', 'ef'], 'Tokenizer')).length, 2);
    assert.equal((await model.processBatch(['😀😀', '😀'], 'Tokenizer')).length, 2);
    await assert.rejects(model.processBatch(['😀😀', '😀a']), { code: 'SPARS_INPUT_LIMIT' });
    await assert.rejects(model.processBatch(['😀😀a']), { code: 'SPARS_INPUT_LIMIT' });
    await assert.rejects(model.processBatch(['abcd', 'efg']), { code: 'SPARS_INPUT_LIMIT' });
    await assert.rejects(model.processBatch(['abcde']), { code: 'SPARS_INPUT_LIMIT' });
    await assert.rejects(model.processBatch(['', '', '']), { code: 'SPARS_INPUT_LIMIT' });
    assert.deepEqual(await model.processBatch([]), []);
    assert.equal((await model.process('')).text, '');
    await assert.rejects(model.process('\ud800'), { code: 'SPARS_INVALID_TEXT' });
    assert.throws(() => Reflect.apply(model.process, model, [42]));
    assert.throws(() => Reflect.apply(model.processBatch, model, [[null]]));
  } finally { configureInputLimits(defaults); }
});

test('oversize batches are rejected before elements are read or an admission slot is taken', async () => {
  const model = await loadModel(modelPath);
  configureInputLimits({ maxTextLength: 4, maxBatchSize: 2, maxBatchTextLength: 6 });
  configureExecution({ maxActive: 1, maxQueued: 0 });
  try {
    const oversized = ['', '', ''];
    Object.defineProperty(oversized, 0, { get() { throw new Error('must not read an oversized batch'); } });
    await assert.rejects(model.processBatch(oversized), { code: 'SPARS_INPUT_LIMIT' });
    const rejected = model.process('12345');
    const accepted = model.process('ok');
    await assert.rejects(rejected, { code: 'SPARS_INPUT_LIMIT' });
    assert.throws(() => configureInputLimits(defaults), /idle/);
    assert.equal((await accepted).text, 'ok');
    assert.equal((await model.processBatch(['ok']))[0]?.text, 'ok');
  } finally {
    configureExecution({ maxActive: 2, maxQueued: 32 });
    configureInputLimits(defaults);
  }
});

test('invalid input-limit configuration is rejected atomically and caller mutation is isolated', async () => {
  const model = await loadModel(modelPath);
  const options = { maxTextLength: 4, maxBatchSize: 2, maxBatchTextLength: 6 };
  configureInputLimits(options);
  options.maxTextLength = 100;
  try {
    for (const bad of [null, [], {}, { maxTextLength: 0, maxBatchSize: 2, maxBatchTextLength: 6 },
      { maxTextLength: 4, maxBatchSize: -1, maxBatchTextLength: 6 },
      { maxTextLength: 4, maxBatchSize: 1.5, maxBatchTextLength: 6 },
      { maxTextLength: 4, maxBatchSize: 2, maxBatchTextLength: Infinity },
      { maxTextLength: '4', maxBatchSize: 2, maxBatchTextLength: 6 },
      { maxTextLength: 4, maxBatchSize: 2, maxBatchTextLength: Number.MAX_SAFE_INTEGER + 1 }]) {
      assert.throws(() => Reflect.apply(configureInputLimits, undefined, [bad]));
      await assert.rejects(model.process('12345'), { code: 'SPARS_INPUT_LIMIT' });
      assert.equal((await model.process('1234', 'Tokenizer')).text, '1234');
    }
  } finally { configureInputLimits(defaults); }
});

test('default limits reject extreme workloads', async () => {
  const model = await loadModel(modelPath);
  await assert.rejects(model.process('x'.repeat(defaults.maxTextLength + 1)), { code: 'SPARS_INPUT_LIMIT' });
  await assert.rejects(model.processBatch(Array(defaults.maxBatchSize + 1).fill('')), { code: 'SPARS_INPUT_LIMIT' });
  await assert.rejects(model.processBatch(Array(3).fill('x'.repeat(defaults.maxTextLength))), { code: 'SPARS_INPUT_LIMIT' });
  assert.equal((await model.process('still usable')).text, 'still usable');
});
