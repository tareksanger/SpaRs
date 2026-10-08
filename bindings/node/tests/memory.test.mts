import assert from 'node:assert/strict';
import { setImmediate } from 'node:timers/promises';
import { test } from 'node:test';
import { loadModel, NativeDocument, reportedNativeMemory } from '../index.js';
import { modelPath } from './fixtures.mts';

const TOKENS = 20_000;
// Each native token record is more than 64 and, with its short strings and list growth, far less
// than 1024 bytes, so these bound one document's report without depending on exact layout.
const PER_TOKEN = { min: 64, max: 1024 };

function snapshot(): string {
  const tokens = Array.from({ length: TOKENS }, (_, i) => ({
    start: i * 5, end: i * 5 + 4, idx: i * 5, whitespace: i + 1 < TOKENS, norm: 'word', entity_iob: 'O', entity_type: '',
  }));
  return JSON.stringify({ format_version: 1, document: { text: Array(TOKENS).fill('word').join(' '), tokens, entities: [] } });
}

async function collect(): Promise<void> {
  const gc: unknown = Reflect.get(globalThis, 'gc');
  assert.ok(typeof gc === 'function', 'run with --expose-gc');
  // Node-API finalizers may run after the collection that found the object, so give them turns.
  for (let i = 0; i < 5; i += 1) {
    Reflect.apply(gc, undefined, []);
    await setImmediate();
  }
}

/** The change in reported memory while `work` runs. */
function reportedBy<T>(work: () => T): [T, number] {
  const before = reportedNativeMemory();
  const result = work();
  return [result, reportedNativeMemory() - before];
}

/** Create documents and a copy, check what they report, and drop them; returns one document's report. */
function createAndDrop(json: string): [number, number] {
  const documents: NativeDocument[] = [];
  const before = reportedNativeMemory();
  const [first, one] = reportedBy(() => NativeDocument.fromSnapshot(json));
  documents.push(first);
  assert.ok(one >= TOKENS * PER_TOKEN.min && one <= TOKENS * PER_TOKEN.max, `one document reported ${one} bytes`);
  for (let i = 1; i < 10; i += 1) documents.push(NativeDocument.fromSnapshot(json));
  // A copy reports its source's estimate.
  const [copy, copied] = reportedBy(() => first.withEntities({ entities: [{ start: 0, end: 1, label: 'X' }] }));
  documents.push(copy);
  assert.equal(copied, one);
  const reported = reportedNativeMemory() - before;
  assert.equal(reported, 11 * one);
  // An invalid update creates no document and reports nothing.
  assert.throws(() => first.withEntities({ entities: [{ start: 0, end: TOKENS + 1, label: 'X' }] }), { code: 'SPARS_BOUNDS' });
  assert.equal(reportedNativeMemory() - before, reported);
  return [one, reported];
}

test('native document memory is reported to V8 and released on collection', async () => {
  const json = snapshot();
  await collect();
  const before = reportedNativeMemory();
  const [one, reported] = createAndDrop(json);
  await collect();
  // Collection withdraws what was reported, no more and no less.
  const remaining = reportedNativeMemory() - before;
  assert.ok(Math.abs(remaining) < one, `${remaining} of ${reported} bytes still reported`);
});

test('documents from inference are reported and released', async () => {
  const model = await loadModel(modelPath);
  await collect();
  const before = reportedNativeMemory();
  let doc: NativeDocument | null = await model.processDocument('Alice visits London. '.repeat(500));
  const reported = reportedNativeMemory() - before;
  assert.ok(reported >= 2000 * PER_TOKEN.min && reported <= 2000 * PER_TOKEN.max * 4, `reported ${reported} bytes`);
  assert.ok(doc.utf16Length > 0);
  doc = null;
  await collect();
  assert.ok(Math.abs(reportedNativeMemory() - before) < reported / 10);
});
