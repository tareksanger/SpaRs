import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';
import { setTimeout as delay } from 'node:timers/promises';
import { loadModel } from '../index.js';
import type { Document, Stage } from '../index.js';
import { array, record, readJson } from '../tests/fixtures.mts';

const [path, corpusPath, profile, mode, roundsArg, warmupArg, batchArg, stageArg] = process.argv.slice(2);
assert.ok(path && corpusPath && (profile === 'short' || profile === 'long'));
assert.ok(mode === 'single' || mode === 'batch' || mode === 'concurrent');
const rounds = Number(roundsArg ?? 3);
const warmup = Number(warmupArg ?? 1);
const batchSize = Number(batchArg ?? 96);
assert.ok(Number.isSafeInteger(rounds) && rounds > 0);
assert.ok(Number.isSafeInteger(warmup) && warmup >= 0);
assert.ok(Number.isSafeInteger(batchSize) && batchSize > 0);
assert.ok(stageArg === undefined || stageArg === 'Tokenizer' || stageArg === 'Ner');
const stage: Stage = stageArg ?? 'Ner';
const start = performance.now();
const model = await loadModel(path);
const loadSeconds = (performance.now() - start) / 1000;
const cases = array(record(readJson(corpusPath)).cases).map(record);
const texts = cases.filter(c => (c.category === 'long') === (profile === 'long')).map(c => {
  assert.ok(typeof c.text === 'string');
  return c.text;
});
assert.ok(texts.length > 0);
const groups: string[][] = [];
for (let i = 0; i < texts.length; i += batchSize) groups.push(texts.slice(i, i + batchSize));
let maximumSubmissionMs = 0;
let maximumTimerGapMs = 0;
let previousTick = performance.now();
let tokenCount = 0;
let documentCount = 0;
let elapsed = 0;

async function run(): Promise<void> {
  const consume = (docs: Document[]): void => {
    documentCount += docs.length;
    tokenCount += docs.reduce((sum, doc) => sum + doc.tokens.length, 0);
  };
  for (const group of groups) {
    if (mode === 'single') {
      for (const text of group) {
        const submitted = performance.now();
        const task = model.process(text, stage);
        maximumSubmissionMs = Math.max(maximumSubmissionMs, performance.now() - submitted);
        const doc = await task;
        elapsed += (performance.now() - submitted) / 1000;
        consume([doc]);
      }
    } else {
      const submitted = performance.now();
      const task = mode === 'batch' ? model.processBatch(group, stage) : Promise.all(group.map(text => model.process(text, stage)));
      maximumSubmissionMs = Math.max(maximumSubmissionMs, performance.now() - submitted);
      const docs = await task;
      elapsed += (performance.now() - submitted) / 1000;
      consume(docs);
    }
  }
}
for (let i = 0; i < warmup; i++) await run();
tokenCount = 0;
documentCount = 0;
elapsed = 0;
maximumSubmissionMs = 0;
previousTick = performance.now();
const timer = setInterval(() => {
  const now = performance.now();
  maximumTimerGapMs = Math.max(maximumTimerGapMs, now - previousTick);
  previousTick = now;
}, 1);
for (let i = 0; i < rounds; i++) await run();
await delay(5); // Observe a timer turn after the final result has been converted.
clearInterval(timer);
console.log(JSON.stringify({ mode, stage, batch_size: batchSize, worker_pool: process.env.UV_THREADPOOL_SIZE ?? 'default',
  load_seconds: loadSeconds, corpus_documents: texts.length, rounds, warmup_passes: warmup,
  measured_documents: documentCount, measured_tokens: tokenCount, inference_seconds: elapsed,
  documents_per_second: documentCount / elapsed, tokens_per_second: tokenCount / elapsed,
  maximum_submission_ms: maximumSubmissionMs, maximum_timer_gap_ms: maximumTimerGapMs,
  peak_process_rss_bytes: process.resourceUsage().maxRSS * 1024,
  note: 'Awaited latency includes scheduling, inference, Rust conversion and JS object creation. Loading excludes Node startup; peak RSS includes it. Timer gaps include normal timer resolution.' }));
