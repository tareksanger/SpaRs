import assert from 'node:assert/strict';
import { test } from 'node:test';
import { configureExecution, loadModel } from '../index.js';
import { Scheduler, install } from '../execution.cjs';
import type { Document } from '../index.js';
import { Model } from '../index.js';
import { modelPath } from './fixtures.mts';

test('admission is shared across models, rejects overflow, and recovers after errors', async () => {
  const first = await loadModel(modelPath);
  const second = await loadModel(modelPath);
  configureExecution({ maxActive: 1, maxQueued: 1 });
  try {
    const active = first.process('Alice visits London.');
    const textsWaiting = ['Dogs run.'];
    const queued = second.processBatch(textsWaiting);
    textsWaiting[0] = 'changed';
    await assert.rejects(first.process('overflow'), { code: 'SPARS_BUSY' });
    assert.throws(() => configureExecution({ maxActive: 2, maxQueued: 2 }), /idle/);
    assert.equal((await active).text, 'Alice visits London.');
    assert.equal((await queued)[0]?.text, 'Dogs run.');
    const invalid = first.process('\ud800');
    const next = second.process('Still works.');
    await assert.rejects(invalid, { code: 'SPARS_INVALID_TEXT' });
    assert.equal((await next).text, 'Still works.');
    install({ Model }); // Reinstalling must not wrap an already verified loader twice.
    configureExecution({ maxActive: 1, maxQueued: 0 });
    const texts = ['original'];
    const batch = first.processBatch(texts);
    texts[0] = 'changed';
    await assert.rejects(second.processBatch(['overflow']), { code: 'SPARS_BUSY' });
    assert.equal((await batch)[0]?.text, 'original');
  } finally { configureExecution({ maxActive: 2, maxQueued: 32 }); }
});

function deferred(): { promise: Promise<number>; resolve: (value: number) => void; reject: (error: Error) => void } {
  let resolve: (value: number) => void = () => { throw new Error('not initialized'); };
  let reject: (error: Error) => void = () => { throw new Error('not initialized'); };
  const promise = new Promise<number>((accept, fail) => { resolve = accept; reject = fail; });
  return { promise, resolve, reject };
}

test('scheduler caps native submissions, preserves FIFO, and releases every slot', async () => {
  const scheduler = new Scheduler();
  scheduler.configure({ maxActive: 2, maxQueued: 2 });
  const work = [deferred(), deferred(), deferred(), deferred()];
  const started: number[] = [];
  const jobs = work.map((item, index) => scheduler.submit(() => { started.push(index); return item.promise; }));
  const settled = Promise.allSettled(jobs);
  assert.deepEqual(started, [0, 1]);
  await assert.rejects(scheduler.submit(() => { throw new Error('must not start'); }), { code: 'SPARS_BUSY' });
  work[1]?.resolve(1);
  assert.equal(await jobs[1], 1);
  assert.deepEqual(started, [0, 1, 2]);
  work[0]?.reject(new Error('native failure'));
  await assert.rejects(jobs[0]!, /native failure/);
  assert.deepEqual(started, [0, 1, 2, 3]);
  work[2]?.resolve(2); work[3]?.resolve(3);
  await settled;
  assert.equal(scheduler.active, 0);
  assert.equal(scheduler.queued, 0);
  await assert.rejects(scheduler.submit(() => { throw new Error('sync failure'); }), /sync failure/);
  assert.equal(await scheduler.submit(async () => 42), 42);
  scheduler.configure({ maxActive: 1, maxQueued: 0 });
});

test('invalid configuration leaves defaults unchanged', () => {
  for (const options of [null, [], {}, {maxActive: 0,maxQueued: 1}, {maxActive: 1.5,maxQueued: 1},
    {maxActive: 1,maxQueued: -1}, {maxActive: 1,maxQueued: Infinity}, {maxActive: '1',maxQueued: 1},
    {maxActive: Number.MAX_SAFE_INTEGER + 1,maxQueued: 0}]) {
    const scheduler = new Scheduler();
    assert.throws(() => Reflect.apply(scheduler.configure, scheduler, [options]));
    assert.equal(scheduler.maxActive, 2);
    assert.equal(scheduler.maxQueued, 32);
  }
});

test('wrappers defer native invocation and reject saturation without reading batch elements', async () => {
  const started: string[] = [];
  const gates = new Map<string, { promise: Promise<Document>; resolve: (value: Document) => void }>();
  for (const key of ['first', 'second', 'third']) {
    let resolve: (value: Document) => void = () => { throw new Error('not initialized'); };
    const promise = new Promise<Document>(accept => { resolve = accept; });
    gates.set(key, { promise, resolve });
  }
  class FakeModel {
    process(text: string): Promise<Document> {
      started.push(text);
      const gate = gates.get(text);
      assert.ok(gate);
      return gate.promise;
    }
    async processBatch(texts: string[]): Promise<Document[]> {
      const text = texts[0];
      assert.ok(text);
      return [await FakeModelNative.call(this, text)];
    }
    vector(): null { return null; }
  }
  const FakeModelNative = FakeModel.prototype.process;
  install({ Model: FakeModel });
  configureExecution({ maxActive: 1, maxQueued: 2 });
  const model = new FakeModel();
  const first = model.process('first');
  const second = model.processBatch(['second']);
  const third = model.process('third');
  assert.deepEqual(started, ['first']);
  const inaccessible = ['never'];
  Object.defineProperty(inaccessible, 0, { get() { throw new Error('must not copy a rejected batch'); } });
  await assert.rejects(model.processBatch(inaccessible), { code: 'SPARS_BUSY' });
  assert.deepEqual(started, ['first']);
  const doc = (text: string): Document => ({ text, tokens: [], entities: null, sentences: null, nounChunks: null });
  gates.get('first')?.resolve(doc('first'));
  await first;
  assert.deepEqual(started, ['first', 'second']);
  gates.get('second')?.resolve(doc('second'));
  await second;
  assert.deepEqual(started, ['first', 'second', 'third']);
  gates.get('third')?.resolve(doc('third'));
  await third;
  configureExecution({ maxActive: 2, maxQueued: 32 });
});
